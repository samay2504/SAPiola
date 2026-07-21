package main

import (
	"log"
	"os"

	"github.com/sapiola/sap-cdc-core/internal/kafka"
	"github.com/sapiola/sap-cdc-core/internal/server"
)

func main() {
	brokers := os.Getenv("KAFKA_BROKERS")
	if brokers == "" {
		brokers = "localhost:9092"
	}
	topic := os.Getenv("CDC_TOPIC")
	if topic == "" {
		topic = "sap-cdc-events"
	}
	addr := os.Getenv("GRPC_ADDR")
	if addr == "" {
		addr = ":50051"
	}

	log.Println("Starting Ingest Gateway...")

	producer, err := kafka.NewProducer(brokers, topic)
	if err != nil {
		log.Fatalf("Failed to create Kafka producer: %v", err)
	}
	defer producer.Close()

	if err := server.Run(addr, producer); err != nil {
		log.Fatalf("Server failed: %v", err)
	}
}
