package event

import (
    "fmt"
    "time"
    
    cdcpb "github.com/sapiola/sap-cdc-core/gen/proto/sapiola/v1"
    "google.golang.org/protobuf/types/known/timestamppb"
)

type CdcEvent struct {
    Table          string
    PrimaryKey     string
    Operation      Operation
    Before         map[string]any
    After          map[string]any
    Timestamp      time.Time
    TxID           string
    LSN            int64
}

type Operation int

const (
    OpInsert Operation = 1
    OpUpdate Operation = 2
    OpDelete Operation = 3
)

func (e *CdcEvent) ToProto() *cdcpb.CdcEvent {
    pb := &cdcpb.CdcEvent{
        Table:             e.Table,
        PrimaryKey:        e.PrimaryKey,
        Operation:         cdcpb.Operation(e.Operation),
        Timestamp:         timestamppb.New(e.Timestamp),
        Txid:              e.TxID,
        LogSequenceNumber: e.LSN,
    }
    pb.Before = convertMapToProto(e.Before)
    pb.After = convertMapToProto(e.After)
    return pb
}

func (e *CdcEvent) DeduplicationKey() string {
    return fmt.Sprintf("%s|%s|%d", e.Table, e.PrimaryKey, e.LSN)
}

func (e *CdcEvent) PartitionKey() string {
    return fmt.Sprintf("%s|%s", e.Table, e.PrimaryKey)
}

func convertMapToProto(m map[string]any) map[string]*cdcpb.Value {
    if m == nil {
        return nil
    }
    result := make(map[string]*cdcpb.Value, len(m))
    for k, v := range m {
        result[k] = anyToProtoValue(v)
    }
    return result
}

func anyToProtoValue(v any) *cdcpb.Value {
    switch val := v.(type) {
    case string:
        return &cdcpb.Value{Kind: &cdcpb.Value_StringValue{StringValue: val}}
    case int64:
        return &cdcpb.Value{Kind: &cdcpb.Value_IntValue{IntValue: val}}
    case float64:
        return &cdcpb.Value{Kind: &cdcpb.Value_DoubleValue{DoubleValue: val}}
    case bool:
        return &cdcpb.Value{Kind: &cdcpb.Value_BoolValue{BoolValue: val}}
    case []byte:
        return &cdcpb.Value{Kind: &cdcpb.Value_BytesValue{BytesValue: val}}
    default:
        return &cdcpb.Value{Kind: &cdcpb.Value_StringValue{StringValue: fmt.Sprintf("%v", val)}}
    }
}
