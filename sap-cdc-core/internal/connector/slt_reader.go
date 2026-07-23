package connector

import (
	"context"
	"database/sql"
	"fmt"
	"time"

	_ "github.com/SAP/go-hdb/driver"
	"github.com/sapiola/sap-cdc-core/internal/event"
	"go.uber.org/zap"
)

type SLTReaderConfig struct {
	Tables          map[string]string
	MinPollInterval time.Duration
	MaxPollInterval time.Duration
	BatchSize       int
}

type SLTReader struct {
	db              *sql.DB
	config          SLTReaderConfig
	logger          *zap.Logger
	currentInterval time.Duration
}

func NewSLTReader(db *sql.DB, config SLTReaderConfig, logger *zap.Logger) *SLTReader {
	if config.MinPollInterval == 0 {
		config.MinPollInterval = 100 * time.Millisecond
	}
	if config.MaxPollInterval == 0 {
		config.MaxPollInterval = 5 * time.Second
	}
	if config.BatchSize == 0 {
		config.BatchSize = 1000
	}
	return &SLTReader{
		db:              db,
		config:          config,
		logger:          logger,
		currentInterval: config.MinPollInterval,
	}
}

// ReadChanges polls the SLT logging table for new events since the given LSN.
func (r *SLTReader) ReadChanges(ctx context.Context, tableName string, lastLSN int64) ([]event.CdcEvent, error) {
	loggingTable, ok := r.config.Tables[tableName]
	if !ok {
		return nil, nil // No mapping found
	}

	query := fmt.Sprintf("SELECT * FROM %s WHERE LSN > ? ORDER BY LSN ASC LIMIT ?", loggingTable)

	rows, err := r.db.QueryContext(ctx, query, lastLSN, r.config.BatchSize)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	cols, err := rows.Columns()
	if err != nil {
		return nil, err
	}

	var events []event.CdcEvent

	for rows.Next() {
		values := make([]interface{}, len(cols))
		ptrs := make([]interface{}, len(cols))
		for i := range values {
			ptrs[i] = &values[i]
		}

		if err := rows.Scan(ptrs...); err != nil {
			return nil, err
		}

		colMap := make(map[string]interface{}, len(cols))
		for i, name := range cols {
			colMap[name] = values[i]
		}

		var lsn int64
		if v, ok := colMap["LSN"]; ok && v != nil {
			switch val := v.(type) {
			case int64:
				lsn = val
			case int32:
				lsn = int64(val)
			case int:
				lsn = int64(val)
			default:
				fmt.Sscanf(fmt.Sprintf("%v", val), "%d", &lsn)
			}
		}

		var transType int
		if v, ok := colMap["TRANS_TYPE"]; ok && v != nil {
			fmt.Sscanf(fmt.Sprintf("%v", v), "%d", &transType)
		}

		var op event.Operation
		switch transType {
		case 1:
			op = event.OpInsert
		case 2, 3:
			op = event.OpUpdate
		case 4:
			op = event.OpDelete
		default:
			op = event.OpInsert
		}

		var pk string
		if v, ok := colMap["MATNR"]; ok && v != nil {
			pk = stringify(v)
		} else if v, ok := colMap["EBELN"]; ok && v != nil {
			pk = stringify(v)
		} else if v, ok := colMap["PRIMARY_KEY"]; ok && v != nil {
			pk = stringify(v)
		} else {
			pk = fmt.Sprintf("%d", lsn)
		}

		afterData := make(map[string]interface{})
		for k, v := range colMap {
			if k != "LSN" && k != "TRANS_TYPE" {
				if b, ok := v.([]byte); ok {
					afterData[k] = string(b)
				} else {
					afterData[k] = v
				}
			}
		}

		events = append(events, event.CdcEvent{
			Table:      tableName,
			PrimaryKey: pk,
			Operation:  op,
			After:      afterData,
			Timestamp:  time.Now(),
			TxID:       fmt.Sprintf("TX-%d", lsn),
			LSN:        lsn,
		})
	}

	return events, nil
}

func stringify(val interface{}) string {
	switch v := val.(type) {
	case []byte:
		return string(v)
	case string:
		return v
	default:
		return fmt.Sprintf("%v", val)
	}
}


