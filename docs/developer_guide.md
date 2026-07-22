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
export SAPIOLA_MAPPING_DSL="tools/salt_importer/salt_mapping.dsl"
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
Use `HanaIntrospector` or `introspector.py` to query SAP HANA metadata tables (`SYS.TABLE_COLUMNS` & `SYS.REFERENTIAL_CONSTRAINTS`):

```bash
# Set your SAP HANA connection string or dump metadata to pandas
python tools/salt_importer/introspector.py --hana-host <HANA_IP> --hana-port 30015 --user SYSTEM
```
This automatically computes foreign key value-overlap confidence and generates a draft mapping DSL file (`custom_mapping.dsl`).

### Step 2: Automated DSL Generation & LLM Verification (`custom_mapping.dsl`)
> **Fully Automated & LLM-Verifiable**: You do **NOT** need to write `.dsl` files by hand. 
> 1. `introspector.py` automatically generates the complete `custom_mapping.dsl` based on empirical primary key and foreign key confidence scores.
> 2. The attached **LLM Agent** (via `sap-ai-gateway` or MCP server) can automatically inspect the auto-generated `.dsl` file, refine default relationship labels (e.g., renaming `RefersTo_T001W` to human-friendly `LocatedAtPlant`), verify Pest grammar rules, and update the file automatically.

```dsl
// AUTO-GENERATED & LLM-VERIFIED MAPPING DSL
NODE Plant FROM T001W WITH id = WERKS
NODE StorageLocation FROM T001L WITH id = LGORT

EDGE LocatedAtPlant FROM T001L USING WERKS -> WERKS
//   -> targets T001W(WERKS) [Confidence: 100.0%]
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

## 7. Understanding DSL Path Resolution

If you observe the following warning at startup:
```text
WARN sap_graph_layer: No SAP schema .dsl file found in SAPIOLA_MAPPING_DSL or search directories.
```

### Resolution:
`sap-graph-layer` automatically checks candidate paths:
1. Environment variable `SAPIOLA_MAPPING_DSL`
2. `tools/salt_importer/salt_mapping.dsl` or any `.dsl` file in `tools/`, `config/`, `.`, or `..`

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

