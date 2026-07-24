# Dynamic SAP Schema Discovery Engine (`tools/schema_discovery`)

This directory contains the production-grade, zero-code-change **Schema Discovery & Adaptation Engine** for SAPiola.

It allows developers, system administrators, and AI agents to connect **any unseen SAP HANA database or SAP ERP schema** to SAPiola without writing custom code or modifying internal mappings.

---

## Component Inventory

| File | Purpose |
|:---|:---|
| `hana_introspector.py` | Core discovery CLI. Introspects `SYS.TABLE_COLUMNS`, detects PKs via uniqueness sampling, scores FKs empirically via value-overlap sampling, and generates `schema_manifest.json` + `hana_mapping.dsl`. |
| `credential_resolver.py` | Unified credential resolution chain (`env vars → service key file → fail-with-actionable-message`). Safely handles OAuth UAA keys. |
| `schema_manifest.json` | Auto-generated single source of truth manifest containing schema metadata, PKs, empirical FK confidence scores, and a SHA-256 fingerprint. |
| `hana_mapping.dsl` | Pest-compliant DSL generated alongside the manifest. |
| `test_credential_resolver.py` | Unit tests for credential resolution. |
| `test_fk_calibration.py` | Calibration test script for empirical FK confidence scoring on live SAP HANA. |

---

## How a Developer Connects Unseen SAP Data

Suppose a developer or customer wants to attach a brand-new SAP schema (e.g. `ERP_PROD` or custom tables `Z_PLANT`, `Z_STOCK`):

### Step 1: Provide Credentials

Set environment variables or place your `sapiola-dev-key.json` service key in the root directory:

```bash
export SAPIOLA_HANA_HOST="your-hana-host.hanacloud.ondemand.com"
export SAPIOLA_HANA_PORT="443"
export SAPIOLA_HANA_USER="SAPIOLA_TEST"
export SAPIOLA_HANA_PASSWORD="your_password"
```

### Step 2: Run Schema Discovery Engine

Run `hana_introspector.py` specifying the target schema name:

```bash
python tools/schema_discovery/hana_introspector.py \
  --schema ERP_PROD \
  --table-filter "%" \
  --output-dir tools/schema_discovery/
```

What this does automatically:
1. `credential_resolver.py` verifies database access.
2. `hana_introspector.py` discovers all tables and column types in `ERP_PROD`.
3. Detects primary keys via uniqueness sampling.
4. **Empirically discovers foreign keys** via value-overlap sampling (`score_foreign_key_confidence`).
5. Generates `schema_manifest.json` and computes a SHA-256 fingerprint.

### Step 3: Launch SAPiola Services

Set `SAPIOLA_SCHEMA_MANIFEST` and start the graph server:

```bash
export SAPIOLA_SCHEMA_MANIFEST="tools/schema_discovery/schema_manifest.json"
cargo run -p sap-graph-layer
```

- `sap-graph-layer` (Rust) reads `schema_manifest.json`, builds the graph topology, and logs the fingerprint verification on startup.
- `sap-cdc-core` (Go) reads `schema_manifest.json` to monitor table CDC streams dynamically with regex pattern matching fallback for primary keys.
- `sap-ai-gateway` (Python) and `sapiola-mcp` (Node.js) serve AI agent queries (`list_schema`, `query_sap_graph`, `ask_sap`) against the newly discovered SAP schema.

---

## Verification & Testing

```bash
# Unit tests
python -m pytest tools/schema_discovery/test_credential_resolver.py -v

# Empirical FK calibration test
python tools/schema_discovery/test_fk_calibration.py
```
