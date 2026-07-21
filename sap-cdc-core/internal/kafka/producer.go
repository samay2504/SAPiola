package kafka

import (
	"fmt"
	"github.com/confluentinc/confluent-kafka-go/v2/kafka"
    cdcpb "github.com/sapiola/sap-cdc-core/gen/proto/sapiola/v1"
	"google.golang.org/protobuf/proto"
)

type Producer struct {
	p     *kafka.Producer
	topic string
}

func NewProducer(brokers string, topic string) (*Producer, error) {
	p, err := kafka.NewProducer(&kafka.ConfigMap{
		"bootstrap.servers": brokers,
		"acks":              "all",
		"enable.idempotence": true,
	})
	if err != nil {
		return nil, err
	}

	return &Producer{
		p:     p,
		topic: topic,
	}, nil
}

// Produce sends the protobuf event to Kafka, partitioned by PartitionKey (Business Key)
func (p *Producer) Produce(pbEvent *cdcpb.CdcEvent) error {
	data, err := proto.Marshal(pbEvent)
	if err != nil {
		return fmt.Errorf("failed to marshal proto: %w", err)
	}

	deliveryChan := make(chan kafka.Event)
	defer close(deliveryChan)

    partitionKey := fmt.Sprintf("%s|%s", pbEvent.Table, pbEvent.PrimaryKey)

	msg := &kafka.Message{
		TopicPartition: kafka.TopicPartition{Topic: &p.topic, Partition: kafka.PartitionAny},
		Key:            []byte(partitionKey),
		Value:          data,
	}

	err = p.p.Produce(msg, deliveryChan)
	if err != nil {
		return fmt.Errorf("failed to produce message: %w", err)
	}

	ev := <-deliveryChan
	switch evResult := ev.(type) {
	case *kafka.Message:
		if evResult.TopicPartition.Error != nil {
			return fmt.Errorf("delivery failed: %v", evResult.TopicPartition.Error)
		}
	default:
		return fmt.Errorf("ignored event: %v", evResult)
	}

	return nil
}

func (p *Producer) Close() {
	p.p.Flush(15 * 1000)
	p.p.Close()
}
