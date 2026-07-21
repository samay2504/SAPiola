package event

import (
    "go.etcd.io/bbolt"
)

type IdempotencyFilter struct {
    db *bbolt.DB
}

func NewIdempotencyFilter(path string) (*IdempotencyFilter, error) {
    db, err := bbolt.Open(path, 0600, nil)
    if err != nil {
        return nil, err
    }
    
    err = db.Update(func(tx *bbolt.Tx) error {
        _, err := tx.CreateBucketIfNotExists([]byte("lsn_store"))
        return err
    })
    if err != nil {
        return nil, err
    }
    
    return &IdempotencyFilter{db: db}, nil
}

// IsDuplicate checks if the event is a duplicate based on its deduplication key.
// Returns true if the event has been seen before (or if an event with a higher LSN for this PK has been seen).
// Actually, to be strictly correct with CDC, we only care if we've seen this exact event.
// For now, we will store the exact dedup key.
func (f *IdempotencyFilter) IsDuplicate(e *CdcEvent) (bool, error) {
    var isDup bool
    err := f.db.Update(func(tx *bbolt.Tx) error {
        b := tx.Bucket([]byte("lsn_store"))
        key := []byte(e.DeduplicationKey())
        
        if v := b.Get(key); v != nil {
            isDup = true
            return nil
        }
        
        return b.Put(key, []byte("1"))
    })
    return isDup, err
}

func (f *IdempotencyFilter) Close() error {
    return f.db.Close()
}
