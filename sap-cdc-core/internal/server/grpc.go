package server

import (
	"context"
	"io"
	"log"
	"net"

	cdcpb "github.com/sapiola/sap-cdc-core/gen/proto/sapiola/v1"
	"github.com/sapiola/sap-cdc-core/internal/kafka"
	"google.golang.org/grpc"
	"google.golang.org/grpc/reflection"
)

type IngestServer struct {
	cdcpb.UnimplementedCdcIngestServer
	producer *kafka.Producer
}

func NewIngestServer(producer *kafka.Producer) *IngestServer {
	return &IngestServer{
		producer: producer,
	}
}

func (s *IngestServer) PublishEvent(ctx context.Context, event *cdcpb.CdcEvent) (*cdcpb.PublishResponse, error) {
	if err := s.producer.Produce(event); err != nil {
		log.Printf("Failed to produce event: %v", err)
		return &cdcpb.PublishResponse{
			ProcessedCount: 0,
			ErrorMessage:   err.Error(),
		}, nil
	}
	return &cdcpb.PublishResponse{
		ProcessedCount: 1,
	}, nil
}

func (s *IngestServer) PublishEventsStream(stream cdcpb.CdcIngest_PublishEventsStreamServer) error {
	var count int32 = 0
	for {
		event, err := stream.Recv()
		if err == io.EOF {
			return stream.SendAndClose(&cdcpb.PublishResponse{
				ProcessedCount: count,
			})
		}
		if err != nil {
			log.Printf("Error receiving stream: %v", err)
			return err
		}

		if err := s.producer.Produce(event); err != nil {
			log.Printf("Failed to produce streamed event: %v", err)
			return stream.SendAndClose(&cdcpb.PublishResponse{
				ProcessedCount: count,
				ErrorMessage:   err.Error(),
			})
		}
		count++
	}
}

func Run(addr string, producer *kafka.Producer) error {
	lis, err := net.Listen("tcp", addr)
	if err != nil {
		return err
	}

	grpcServer := grpc.NewServer()
	cdcpb.RegisterCdcIngestServer(grpcServer, NewIngestServer(producer))
	reflection.Register(grpcServer)

	log.Printf("Ingest Gateway listening on %s", addr)
	return grpcServer.Serve(lis)
}
