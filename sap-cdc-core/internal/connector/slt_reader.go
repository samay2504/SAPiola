// ==============================================================================
// COPYRIGHT & PATENT NOTICE
// Copyright (c) 2026 Samay Mehar (ID: 487266569521). All Rights Reserved.
//
// PATENTS PENDING & INTELLECTUAL PROPERTY NOTICE:
// This source code, algorithm, architecture, and underlying inventions are the
// proprietary intellectual property of Samay Mehar (ID: 487266569521).
// Protected under national and international copyright, patent, and trade secret laws.
//
// COMMERCIAL LICENSE NOTICE:
// Unauthorized copying, modification, distribution, reverse engineering, or commercial
// exploitation of this software in whole or in part without express written authorization
// from Samay Mehar is strictly prohibited.
// ==============================================================================

package connector

import (
	"context"
	"database/sql"
	"fmt"
	"regexp"
	"strings"
	"time"

	_ "github.com/SAP/go-hdb/driver"
	"github.com/sapiola/sap-cdc-core/internal/event"
	"go.uber.org/zap"
)

var (
	pkExactPattern  = regexp.MustCompile(`(?i)^(id|primary_key|pk|key|guid|uuid)$`)
	pkSuffixPattern = regexp.MustCompile(`(?i)(_id|_key|_nr|id|nr|key)$`)
)

type SLTReaderConfig struct {
	Tables            map[string]string
	PrimaryKeyColumns map[string][]string
	MinPollInterval   time.Duration
	MaxPollInterval   time.Duration
	BatchSize         int
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
		return nil, fmt.Errorf("table %s not configured for CDC monitoring", tableName)
	}

	query := fmt.Sprintf("SELECT * FROM %s WHERE LSN > ? ORDER BY LSN ASC LIMIT ?", loggingTable)

	rows, err := r.db.QueryContext(ctx, query, lastLSN, r.config.BatchSize)
	if err != nil {
		return nil, fmt.Errorf("failed to query logging table %s: %w", loggingTable, err)
	}
	defer rows.Close()

	cols, err := rows.Columns()
	if err != nil {
		return nil, fmt.Errorf("failed to get columns for %s: %w", loggingTable, err)
	}

	var events []event.CdcEvent
	for rows.Next() {
		columnPointers := make([]interface{}, len(cols))
		columnData := make([]interface{}, len(cols))
		for i := range columnData {
			columnPointers[i] = &columnData[i]
		}

		if err := rows.Scan(columnPointers...); err != nil {
			return nil, fmt.Errorf("failed to scan row: %w", err)
		}

		colMap := make(map[string]interface{})
		for i, colName := range cols {
			colMap[colName] = columnData[i]
		}

		var lsn int64
		if v, ok := colMap["LSN"]; ok && v != nil {
			fmt.Sscanf(fmt.Sprintf("%v", v), "%d", &lsn)
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
		pkCols := r.config.PrimaryKeyColumns[tableName]
		pk = buildCompositeKey(colMap, pkCols)
		if pk == "" {
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

func buildCompositeKey(colMap map[string]interface{}, pkCols []string) string {
	// 1. Explicit manifest configuration (Primary path)
	if len(pkCols) > 0 {
		var parts []string
		for _, col := range pkCols {
			if v, ok := colMap[col]; ok && v != nil {
				parts = append(parts, stringify(v))
			}
		}
		if len(parts) > 0 {
			return strings.Join(parts, "|")
		}
	}

	// 2. Intelligent pattern discovery (Fallback path)
	// Phase A: Check exact PK pattern matches (e.g., ID, PRIMARY_KEY, KEY, GUID, UUID)
	for col, v := range colMap {
		if v != nil && pkExactPattern.MatchString(col) {
			return stringify(v)
		}
	}

	// Phase B: Check suffix pattern matches (e.g., MATNR, VBELN, EBELN, KUNNR, ORDER_ID, ITEM_NR)
	for col, v := range colMap {
		if v != nil && pkSuffixPattern.MatchString(col) {
			return stringify(v)
		}
	}

	return ""
}
