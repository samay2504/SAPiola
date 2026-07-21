package test

import (
    "fmt"
    "os"
    "testing"
    
    "github.com/sapiola/sap-cdc-core/internal/event"
    "github.com/stretchr/testify/assert"
)

// MockConsumer simulates a downstream consumer (like the graph engine or Poly-LSM)
// that receives events from the stream bus.
type MockConsumer struct {
    State map[string]int64 // primary_key -> latest LSN seen
    ProcessCount int       // total number of side effects processed
}

func NewMockConsumer() *MockConsumer {
    return &MockConsumer{
        State: make(map[string]int64),
    }
}

// Apply simulates applying an event to downstream state
func (c *MockConsumer) Apply(e *event.CdcEvent) {
    // If it's a true replay (which shouldn't happen if idempotency works), 
    // we just increment ProcessCount to detect the failure.
    c.State[e.PrimaryKey] = e.LSN
    c.ProcessCount++
}

func TestIdempotentReplay(t *testing.T) {
    dbPath := "test_idempotency.db"
    defer os.Remove(dbPath)
    
    consumer := NewMockConsumer()
    
    // 1. Initialize idempotency filter
    filter, err := event.NewIdempotencyFilter(dbPath)
    assert.NoError(t, err)
    
    // 2. Generate 1000 ordered LSN events
    var events []*event.CdcEvent
    for i := 1; i <= 1000; i++ {
        events = append(events, &event.CdcEvent{
            Table:      "MARA",
            PrimaryKey: fmt.Sprintf("MAT-%d", i),
            Operation:  event.OpInsert,
            LSN:        int64(i),
        })
    }
    
    // 3. Process first 500 events
    for i := 0; i < 500; i++ {
        isDup, err := filter.IsDuplicate(events[i])
        assert.NoError(t, err)
        if !isDup {
            consumer.Apply(events[i])
        }
    }
    
    assert.Equal(t, 500, consumer.ProcessCount)
    assert.Equal(t, int64(500), consumer.State["MAT-500"])
    
    // 4. Simulate Crash: Close filter to persist state, but keep the consumer state intact 
    //    (simulating the downstream consumer has already materialized these side effects).
    filter.Close()
    
    // 5. Restart agent
    filter2, err := event.NewIdempotencyFilter(dbPath)
    assert.NoError(t, err)
    defer filter2.Close()
    
    // 6. Replay last 500 events (events 0 to 499) + process next 500 (events 500 to 999)
    // Replay 0-499
    for i := 0; i < 500; i++ {
        isDup, err := filter2.IsDuplicate(events[i])
        assert.NoError(t, err)
        if !isDup {
            consumer.Apply(events[i])
        }
    }
    
    // Process 500-999
    for i := 500; i < 1000; i++ {
        isDup, err := filter2.IsDuplicate(events[i])
        assert.NoError(t, err)
        if !isDup {
            consumer.Apply(events[i])
        }
    }
    
    // 7. Assert: The downstream consumer's state exactly matches a clean run of 1000 events.
    //    It must not contain duplicate processing side-effects.
    assert.Equal(t, 1000, consumer.ProcessCount, "Consumer should have exactly 1000 side effects, no duplicates")
    assert.Equal(t, int64(1000), consumer.State["MAT-1000"])
}
