package kafka

import (
	"context"
	"fmt"
	"time"

	cdcpb "github.com/sapiola/sap-cdc-core/gen/proto/sapiola/v1"
	kafkago "github.com/segmentio/kafka-go"
	"google.golang.org/protobuf/proto"
)

type Producer struct {
	w     *kafkago.Writer
	topic string
}

func NewProducer(brokers string, topic string) (*Producer, error) {
	w := &kafkago.Writer{
		Addr:         kafkago.TCP(brokers),
		Topic:        topic,
		Balancer:     &kafkago.Hash{},
		RequiredAcks: kafkago.RequireAll,
		Async:        false,
		WriteTimeout: 10 * time.Second,
	}

	return &Producer{
		w:     w,
		topic: topic,
	}, nil
}

// Produce sends the protobuf event to Kafka, partitioned by PartitionKey (Business Key)
func (p *Producer) Produce(pbEvent *cdcpb.CdcEvent) error {
	data, err := proto.Marshal(pbEvent)
	if err != nil {
		return fmt.Errorf("failed to marshal proto: %w", err)
	}

	partitionKey := fmt.Sprintf("%s|%s", pbEvent.Table, pbEvent.PrimaryKey)

	msg := kafkago.Message{
		Key:   []byte(partitionKey),
		Value: data,
		Time:  time.Now(),
	}

	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	err = p.w.WriteMessages(ctx, msg)
	if err != nil {
		return fmt.Errorf("failed to produce message: %w", err)
	}

	return nil
}

func (p *Producer) Close() {
	if p.w != nil {
		_ = p.w.Close()
	}
}
