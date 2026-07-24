# SAPiola Developer & Operations Guide (Cross-Platform)

This guide provides onboarding, architecture, and operational instructions for developers building with or deploying SAPiola across **Windows**, **Linux**, and **macOS**.

---

## 1. Prerequisites & Toolchain Setup

### Polyglot Requirements:
1. **Rust (1.75+)**: Core storage (`poly-lsm-core`), gRPC server (`sap-graph-layer`), MCP server (`sap-mcp-server`), and streaming gateway (`sap-streaming-gateway`).
2. **Go (1.21+)**: Pure-Go (`CGO_ENABLED=0`) CDC event pipeline (`sap-cdc-core`).
3. **Python (3.12+)**: RAG orchestrator (`sap-ai-gateway`) and schema introspector (`tools/salt_importer`).
4. **Node.js (18+) & npm**: MCP client wrapper (`sapiola-mcp`).
5. **Docker & Docker Compose**: Streaming infrastructure (`Redpanda` / `Kafka`).

---

## 2. Environment Variables & PATH per Operating System

### Windows (PowerShell):
```powershell
# Add Rust and MinGW binaries to PATH for compilation
$env:Path += ";$env:USERPROFILE\.cargo\bin;C:\msys64\mingw64\bin"

# Set core service ports and configurations
$env:GRAPH_SERVER_LISTEN_ADDR="127.0.0.1:50053"
$env:SAPIOLA_GRAPH_URL="http://127.0.0.1:50053"
$env:SAPIOLA_MAPPING_DSL="tools\salt_importer\salt_mapping.dsl"
```

### Linux / macOS (Bash / Zsh):
```bash
# Add Rust binaries to PATH
export PATH="$HOME/.cargo/bin:$PATH"

# Set core service ports and configurations
export GRAPH_SERVER_LISTEN_ADDR="127.0.0.1:50053"
export SAPIOLA_GRAPH_URL="http://127.0.0.1:50053"
export SAPIOLA_SCHEMA_MANIFEST="tools/schema_discovery/schema_manifest.json"
export SAPIOLA_MAPPING_DSL="tools/schema_discovery/hana_mapping.dsl"
export SAPIOLA_HANA_USER="DBADMIN"
export SAPIOLA_HANA_PASSWORD="your_password"
```

---

## 3. Generic SAP HANA Data Ingestion (Non-SALT)

SAPiola is 100% schema-agnostic. While the SALT dataset is provided as a test fixture, you can ingest **any** live SAP ERP or SAP HANA database using the following workflow:

```
[SAP HANA Database]
   │ (SLT / DB Triggers on INSERT, UPDATE, DELETE)
   ▼
[sap-cdc-core (Go)]
   │ (Serializes row mutation into binary CdcEvent Protobuf)
   ▼
[Redpanda / Kafka Cluster]
   │ (Business-key partitioned stream with guaranteed idempotency)
   ▼
[sap-streaming-gateway (Rust)]
   │ (Consumes Kafka stream via rskafka)
   ▼
[poly-lsm-core (Fjall Storage Engine)]
```

### Step 1: Introspect your SAP HANA Database
Use `hana_introspector.py` with `credential_resolver` to query SAP HANA metadata tables (`SYS.TABLE_COLUMNS`):

```bash
# Provide SAP HANA Cloud credentials via environment variables
export SAPIOLA_HANA_HOST="your-hana-instance.hanacloud.ondemand.com"
export SAPIOLA_HANA_PORT="443"
export SAPIOLA_HANA_USER="SAPIOLA_TEST"
export SAPIOLA_HANA_PASSWORD="your_password"

# Run discovery against ANY SAP schema (e.g. ERP_PROD, DBADMIN, or custom tables)
python tools/schema_discovery/hana_introspector.py \
  --schema ERP_PROD \
  --table-filter "%" \
  --output-dir tools/schema_discovery/
```
This automatically discovers all tables in `ERP_PROD`, computes foreign key value-overlap confidence empirically (`score_foreign_key_confidence`), detects primary keys via uniqueness sampling, and generates `schema_manifest.json` + `hana_mapping.dsl` with a SHA-256 fingerprint.

### Step 2: Single Source of Truth (`schema_manifest.json`)
> **Fully Automated & Schema-Agnostic**: You do **NOT** need to write `.dsl` files by hand or update internal code for new SAP schemas.
> 1. `hana_introspector.py` generates `schema_manifest.json` containing table metadata, column types, primary keys, empirical FK confidence scores, and a SHA-256 fingerprint.
> 2. The Rust Graph Layer loads `schema_manifest.json` directly via `SAPIOLA_SCHEMA_MANIFEST`, verifying the fingerprint on startup.
> 3. The Go CDC Agent loads `schema_manifest.json` to configure monitored tables and primary key columns dynamically, falling back to intelligent regex pattern matching (`(?i)(_id|_key|_nr|id|nr|key)$`) if PK configuration is missing.

```json
{
  "source": { "schema": "DBADMIN", "table_filter": "%_RAG" },
  "tables": {
    "VBAK_RAG": { "primary_keys": ["VBELN"], "node_label": "VbakRag" }
  },
  "relationships": [
    { "from_table": "VBAP_RAG", "from_col": "VBELN", "to_table": "VBAK_RAG", "to_col": "VBELN", "confidence": 1.0 }
  ],
  "fingerprint": "sha256:f2490430b9b11e6d4c0ad15570f1dc1e9bcf31615b4aa82319ae4ef7958972f5"
}
```

### Step 3: Stream Live CDC Events into SAPiola (`sap-cdc-core`)
In production, deploy `sap-cdc-core` alongside SAP SLT or database triggers. It serializes SAP HANA table mutations into `CdcEvent` Protobuf streams and publishes to Redpanda/Kafka.

`sap-streaming-gateway` consumes these events and updates `poly-lsm-core` live without ETL downtime.

> **Note on SAP HANA Licensing**: No free public SAP HANA Cloud database exists open to the internet due to enterprise licensing. For local offline testing without an enterprise SAP license, `tools/salt_importer/importer.py` acts as a high-fidelity **CDC Stream Emulator**, converting dataset rows into `CdcEvent` Protobuf streams.

---

## 4. Garbage Collection, Compaction & Idle System Standards

1. **LSM Compaction & Garbage Collection**:
   - `poly-lsm-core` uses the **Fjall LSM-tree engine**.
   - Updated or deleted vertices/edges write tombstone records to memtables.
   - The background worker task (`PolyLsmEngine::open_with_worker`) runs **Leveled / Tiered Compaction** continuously in the background, merging SSTables and purging obsolete tombstones to free disk space automatically.

2. **Idle Resource Optimization**:
   - **Memtable Flush**: Uncommitted write buffers in RAM auto-flush to disk after an idle timeout.
   - **Zero-Copy Mmap**: Disk SSTables use memory mapping (`mmap`), reducing idle RAM consumption to minimal block caches (QuickCache).
   - **Idle TCP Keep-Alive**: gRPC channels in `sap-mcp-server` and `sap-ai-gateway` enter TCP keep-alive wait with 0% CPU consumption.

---

## 5. Multi-LLM Provider Support Configuration

`sap-ai-gateway` uses `LiteLlmClient` (`litellm`), supporting **100+ LLM providers** out of the box with zero code changes.

To swap LLMs, simply update `sap-ai-gateway/.env`:

| Provider | `SAPIOLA_LLM_MODEL` in `.env` | Required API Key / Base in `.env` |
|---|---|---|
| **Google Gemini** | `gemini/gemini-2.5-flash` | `SAPIOLA_LLM_API_KEY=AIzaSy...` |
| **OpenAI** | `gpt-4o` or `gpt-4o-mini` | `SAPIOLA_LLM_API_KEY=sk-...` |
| **Anthropic** | `claude-3-5-sonnet-20241022` | `SAPIOLA_LLM_API_KEY=sk-ant-...` |
| **Groq (Fast Llama 3)** | `groq/llama-3.3-70b-versatile` | `SAPIOLA_LLM_API_KEY=gsk_...` |
| **DeepSeek** | `deepseek/deepseek-chat` | `SAPIOLA_LLM_API_KEY=sk-...` |
| **Local Ollama** | `ollama/llama3` | `SAPIOLA_LLM_API_BASE=http://localhost:11434` |

---

## 6. MCP Server Deployment & Integration

To connect SAPiola to external AI agent frameworks (e.g. Claude Desktop, AGY, VS Code MCP extensions):

### Stdio Transport Configuration (`claude_desktop_config.json`):

#### Windows:
```json
{
  "mcpServers": {
    "sapiola": {
      "command": "node",
      "args": ["C:/path/to/SAPiola/sapiola-mcp/bin/run.js"],
      "env": {
        "SAPIOLA_GRAPH_URL": "http://127.0.0.1:50053",
        "SAPIOLA_PRINCIPAL": "alice"
      }
    }
  }
}
```

#### Linux / macOS:
```json
{
  "mcpServers": {
    "sapiola": {
      "command": "node",
      "args": ["/path/to/SAPiola/sapiola-mcp/bin/run.js"],
      "env": {
        "SAPIOLA_GRAPH_URL": "http://127.0.0.1:50053",
        "SAPIOLA_PRINCIPAL": "alice"
      }
    }
  }
}
```

---

## 7. Understanding Manifest & DSL Path Resolution

If you observe the following warning at startup:
```text
WARN sap_graph_layer: No SAP schema .dsl file found in SAPIOLA_SCHEMA_MANIFEST, SAPIOLA_MAPPING_DSL, or search directories.
```

### Resolution:
`sap-graph-layer` automatically checks candidate paths in order of priority:
1. Environment variable `SAPIOLA_SCHEMA_MANIFEST` (loads `schema_manifest.json`, logs SHA-256 fingerprint)
2. Environment variable `SAPIOLA_MAPPING_DSL` (direct path to `.dsl` file)
3. Auto-search for any `.dsl` file in `tools/`, `config/`, `.`, `..`, or `../tools`

---

## 8. Docker Stack Specification & Necessity Breakdown

### Service Inventory (`docker-compose.yml`):

| Container Service Name | Dockerfile / Image Source | Exposed Ports | Primary Purpose |
|---|---|---|---|
| **`redpanda`** | `docker.redpanda.com/redpandadata/redpanda:v24.1.2` | `19092`, `18081`, `18082`, `9644` | High-throughput, Kafka-compatible event stream broker. |
| **`redpanda-init`** | `redpanda:v24.1.2` (One-shot runner) | Internal | Automatically creates the `sap.cdc.events` streaming topic at startup. |
| **`sap_cdc_agent`** | `./sap-cdc-core/Dockerfile` | Internal | Pure-Go CDC event producer capturing live SAP HANA mutations. |
| **`sap_ingest_gateway`** | `./sap-cdc-core/Dockerfile.ingest` | `50051:50051` | Ingestion gRPC endpoint receiving streaming batch mutations. |
| **`sap_streaming_gateway`** | `./sap-streaming-gateway/Dockerfile` | Internal | Rust streaming consumer forwarding Kafka events to the graph layer. |
| **`sap_graph_server`** | `./sap-graph-layer/Dockerfile` | `50052:50052` / `50053:50053` | Rust gRPC graph storage and query engine (`poly-lsm-core`). |

---

### Is Docker Absolutely Required to Run this MCP?

#### 1. Local Desktop MCP Use (Claude Desktop / Cursor / AGY): **NOT REQUIRED** ❌
- You do **NOT** need Docker or Docker Compose running to build or use `sapiola-mcp`.
- **Host Native Run**: You can run `sap-graph-layer` directly on your host machine (`cargo run -p sap-graph-layer`) and run `sapiola-mcp` natively (`node bin/run.js`). The storage engine reads and writes directly to local disk without needing Redpanda or container overhead.

#### 2. Distributed Production Deployment (Cloud / SAP Enterprise CDC): **REQUIRED** ✅
- Docker Compose / Kubernetes is **REQUIRED** when deploying the full distributed infrastructure across production server clusters.
- It orchestrates the asynchronous CDC pipeline (`Redpanda` broker + `sap-cdc-core` + `sap-streaming-gateway` + `sap-graph-layer`), enabling zero-downtime streaming updates directly from enterprise SAP landscapes.

