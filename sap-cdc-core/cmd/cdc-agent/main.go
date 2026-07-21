package main

import (
	"context"
	"flag"
	"os"
	"os/signal"
	"syscall"
	"time"

	"github.com/sapiola/sap-cdc-core/internal/connector"
	"github.com/sapiola/sap-cdc-core/internal/event"
	"github.com/sapiola/sap-cdc-core/internal/kafka"
	"go.uber.org/zap"
)

func main() {
	var (
		kafkaBrokers = flag.String("kafka-brokers", "localhost:9092", "Kafka bootstrap servers")
		kafkaTopic   = flag.String("kafka-topic", "sap.cdc.events", "Kafka topic to produce to")
		dbPath       = flag.String("db-path", "idempotency.db", "Path to idempotency KV store")
	)
	flag.Parse()

	logger, _ := zap.NewProduction()
	defer logger.Sync()

	logger.Info("Starting SAP CDC Agent")

	// Initialize Kafka Producer
	producer, err := kafka.NewProducer(*kafkaBrokers, *kafkaTopic)
	if err != nil {
		logger.Fatal("Failed to create Kafka producer", zap.Error(err))
	}
	defer producer.Close()

	// Initialize Idempotency Filter
	filter, err := event.NewIdempotencyFilter(*dbPath)
	if err != nil {
		logger.Fatal("Failed to create idempotency filter", zap.Error(err))
	}
	defer filter.Close()

	cfg := connector.SLTReaderConfig{
		Tables:          map[string]string{"MARA": "Z_MARA_LOG"},
		MinPollInterval: 1 * time.Second,
	}
	
	// Open a mock DB connection (or real one depending on DSN)
	// For Phase 0, we just pass nil to bypass actual DB connection logic since we are just mocking events
	_ = connector.NewSLTReader(nil, cfg, logger)
	eventChan := make(chan *event.CdcEvent, 100)

	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	// Handle SIGTERM/SIGINT for graceful shutdown
	sigChan := make(chan os.Signal, 1)
	signal.Notify(sigChan, syscall.SIGINT, syscall.SIGTERM)
	go func() {
		<-sigChan
		logger.Info("Shutting down CDC Agent...")
		cancel()
	}()

	// Start reading events (mocking the loop since Start() isn't implemented natively yet)
	go func() {
		ticker := time.NewTicker(2 * time.Second)
		defer ticker.Stop()
		var lsn int64 = 1
		for {
			select {
			case <-ctx.Done():
				return
			case <-ticker.C:
				// Mock reading an event
				ev := &event.CdcEvent{
					Table:      "MARA",
					PrimaryKey: "MAT-123",
					Operation:  event.OpInsert,
					LSN:        lsn,
				}
				lsn++
				eventChan <- ev
			}
		}
	}()

	// Consume and produce events
	for {
		select {
		case <-ctx.Done():
			logger.Info("Main loop exiting")
			return
		case e, ok := <-eventChan:
			if !ok {
				logger.Info("Event channel closed")
				return
			}
			
			// Idempotency check per INV-5
			isDup, err := filter.IsDuplicate(e)
			if err != nil {
				logger.Error("Idempotency check failed", zap.Error(err))
				continue
			}
			if isDup {
				// Already processed this LSN for this primary key
				continue
			}

			err = producer.Produce(e.ToProto())
			if err != nil {
				logger.Error("Failed to produce event", zap.Error(err))
				// In a real system, you'd backoff and retry here to avoid losing events.
			} else {
				// Successfully produced to Kafka
				// IsDuplicate already persisted the event LSN to bbolt
				logger.Debug("Produced event", zap.String("pk", e.PrimaryKey), zap.Int64("lsn", e.LSN))
			}
		}
	}
}
