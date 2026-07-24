# Schema Discovery

## Overview

SAPiola uses a **dynamic schema discovery engine** to automatically introspect SAP HANA databases and generate the graph mapping configuration. This eliminates the need for manual code changes when onboarding new SAP data.

## How It Works

```
sapiola-dev-key.json + --schema DBADMIN
         │
         ▼
   HanaIntrospector
   ├── SYS.TABLE_COLUMNS     → table/column discovery
   ├── Uniqueness sampling    → primary key detection
   ├── Value-overlap scoring  → foreign key detection (PRIMARY)
   └── SYS.REFERENTIAL_CONSTRAINTS → FK confidence boost only
         │
         ▼
   schema_manifest.json + hana_mapping.dsl
```

## Foreign Key Detection: Empirical-First

> **Critical**: Production SAP ERP tables do NOT have database-level foreign key constraints. SAP enforces referential integrity in the ABAP application layer, not the database schema. `SYS.REFERENTIAL_CONSTRAINTS` will return zero rows against real SAP application tables (MARA, VBAK, EKKO, etc.).

The empirical value-overlap method (`score_foreign_key_confidence()`) is the **mandatory, primary FK detection signal**. It works by:

1. For each candidate pair (source_table.column, target_table.pk_column) where column names match
2. Sample up to 100,000 non-null values from the source column
3. Sample up to 100,000 non-null values from the target column
4. Calculate: `confidence = count(source_values ∩ target_values) / count(source_values)`
5. If confidence ≥ threshold (default 0.8), the relationship is included

Declared constraints (`SYS.REFERENTIAL_CONSTRAINTS`) add a `+0.1` confidence boost when they corroborate an empirical match. They are never used as a standalone signal.

## Tuning Parameters

| Parameter | Default | Status | Notes |
|:---|:---|:---|:---|
| `min_fk_confidence` | 0.8 | Uncalibrated placeholder | Not validated against SAP data with imperfect overlap (nullable FKs, soft-deletes, orphaned rows). Adjust if false negatives observed. |
| `DECLARED_FK_CONFIDENCE_BOOST` | +0.1 | Arbitrary placeholder | Will almost never fire against real SAP data. Exists for the rare case of custom tables with explicit FKs. |
| `sample_cap` | 100,000 | Reasonable default | Higher values improve accuracy but increase discovery time. |

## Known Limitation: Auto-Generated Labels

Auto-generated node and edge labels use `PascalCase(table_name)` and `RefersTo_{target_table}` respectively.

**Example**: Table `VBAK` → node label `Vbak`, not `SalesOrderHeader`.

This is **schema-agnostic by design** — it avoids hardcoding SAP field-name semantics, which was the entire purpose of dynamic schema adaptation. However, the labels are semantically opaque to LLM agents trying to reason about business concepts from a `list_schema` call.

**Future extension point**: The `relationships_override` section in `schema_manifest.json` provides a manual override mechanism for human-readable aliases. A future phase could add LLM-powered label suggestion (e.g., using SAP Data Dictionary knowledge to suggest `VBAK → SalesOrderHeader`).

## Usage

```bash
# Set credentials
export SAPIOLA_HANA_USER="SAPIOLA_TEST"
export SAPIOLA_HANA_PASSWORD="your_password"

# Run discovery
python tools/schema_discovery/hana_introspector.py \
  --schema DBADMIN \
  --table-filter "%_RAG" \
  --min-fk-confidence 0.8 \
  --output-dir tools/schema_discovery/

# Output: schema_manifest.json + hana_mapping.dsl
```

## Two FK Detection Paths (Different Data Sources)

| Function | File | Input | Used By |
|:---|:---|:---|:---|
| `HanaIntrospector.score_foreign_key_confidence()` | `tools/schema_discovery/hana_introspector.py` | Live HANA via SQL | Schema manifest generator |
| `discover_foreign_keys()` | `tools/salt_importer/introspector.py` | In-memory Pandas DataFrames | Offline HuggingFace dataset import |

These are **not duplicates** — they operate on fundamentally different data sources (live SQL vs. offline DataFrames) and never compete on the same input.
