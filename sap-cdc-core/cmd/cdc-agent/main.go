package main

import (
	"context"
	"encoding/json"
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

func loadManifest(path string) (map[string]string, map[string][]string, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return nil, nil, err
	}

	var m struct {
		Tables []struct {
			Name        string   `json:"name"`
			LogTable    string   `json:"log_table"`
			PrimaryKeys []string `json:"primary_keys"`
		} `json:"tables"`
	}

	if err := json.Unmarshal(data, &m); err == nil {
		tables := make(map[string]string)
		pks := make(map[string][]string)
		for _, t := range m.Tables {
			tables[t.Name] = t.LogTable
			pks[t.Name] = t.PrimaryKeys
		}
		return tables, pks, nil
	}

	// Fallback to map style
	var mapStyle struct {
		Tables map[string]struct {
			LogTable    string   `json:"log_table"`
			PrimaryKeys []string `json:"primary_keys"`
		} `json:"tables"`
	}
	if err := json.Unmarshal(data, &mapStyle); err != nil {
		return nil, nil, err
	}
	tables := make(map[string]string)
	pks := make(map[string][]string)
	for name, t := range mapStyle.Tables {
		tables[name] = t.LogTable
		pks[name] = t.PrimaryKeys
	}
	return tables, pks, nil
}


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

	manifestPath := os.Getenv("SAPIOLA_SCHEMA_MANIFEST")
	if manifestPath == "" {
		manifestPath = "./schema_manifest.json"
	}
	
	tables, pks, err := loadManifest(manifestPath)
	if err != nil {
		logger.Fatal("Failed to load schema manifest", zap.Error(err))
	}

	cfg := connector.SLTReaderConfig{
		Tables:            tables,
		PrimaryKeyColumns: pks,
		MinPollInterval:   1 * time.Second,
	}
	
	// Open a mock DB connection (or real one depending on DSN)
	// For Phase 0, we just pass nil to bypass actual DB connection logic since we are just mocking events
	sltReader := connector.NewSLTReader(nil, cfg, logger)
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

	// Start reading events
	go func() {
		ticker := time.NewTicker(cfg.MinPollInterval)
		defer ticker.Stop()
		lsns := make(map[string]int64)
		for {
			select {
			case <-ctx.Done():
				return
			case <-ticker.C:
				for tableName := range cfg.Tables {
					events, err := sltReader.ReadChanges(ctx, tableName, lsns[tableName])
					if err != nil {
						logger.Error("Failed to read changes", zap.String("table", tableName), zap.Error(err))
						continue
					}
					for _, ev := range events {
						e := ev
						eventChan <- &e
						if e.LSN > lsns[tableName] {
							lsns[tableName] = e.LSN
						}
					}
				}
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
