# SAPiola Command & CLI Reference (Cross-Platform)

This document provides a comprehensive reference for CLI commands across **Windows (PowerShell)** and **Linux / macOS (Bash / Zsh)**.

---

## 1. Environment & Prerequisites Quick Reference

| OS | Shell | Toolchain Setup / PATH Adjustments |
|---|---|---|
| **Windows** | PowerShell | `$env:Path += ";$env:USERPROFILE\.cargo\bin;C:\msys64\mingw64\bin"` |
| **Linux** | Bash/Zsh | `export PATH="$HOME/.cargo/bin:$PATH"` |
| **macOS** | Zsh | `export PATH="$HOME/.cargo/bin:$PATH"` |

---

## 2. SAP Graph Layer (`sap-graph-layer`)

The core high-performance gRPC graph storage and query engine.

### Linux / macOS (Bash):
```bash
export GRAPH_SERVER_LISTEN_ADDR="127.0.0.1:50053"
export SAPIOLA_MAPPING_DSL="tools/salt_importer/salt_mapping.dsl"
cargo run --release -p sap-graph-layer
```

### Windows (PowerShell):
```powershell
$env:Path += ";$env:USERPROFILE\.cargo\bin;C:\msys64\mingw64\bin"
$env:GRAPH_SERVER_LISTEN_ADDR="127.0.0.1:50053"
$env:SAPIOLA_MAPPING_DSL="tools\salt_importer\salt_mapping.dsl"
cargo run --release -p sap-graph-layer
```

---

## 3. SAP MCP Server (`sap-mcp-server` & `sapiola-mcp`)

Model Context Protocol server for AI agent integration.

### Rust Native Server (`sap-mcp-server`)

#### Linux / macOS:
```bash
export SAPIOLA_GRAPH_URL="http://127.0.0.1:50053"
cargo run --release -p sap-mcp-server
```

#### Windows (PowerShell):
```powershell
$env:Path += ";$env:USERPROFILE\.cargo\bin;C:\msys64\mingw64\bin"
$env:SAPIOLA_GRAPH_URL="http://127.0.0.1:50053"
cargo run --release -p sap-mcp-server
```

### Node.js Wrapper (`sapiola-mcp`)

#### Install & Build (All Platforms):
```bash
cd sapiola-mcp
npm install
```

#### Dev Fallback Build:
- **Linux/macOS:** `SAPIOLA_MCP_DEV_BUILD=1 npm install`
- **Windows (PowerShell):** `$env:SAPIOLA_MCP_DEV_BUILD="1"; npm install`

#### Run Server:
```bash
node bin/run.js
```

---

## 4. SAP AI Gateway (`sap-ai-gateway`)

FastAPI RAG orchestration gateway.

### Linux / macOS:
```bash
cd sap-ai-gateway
export SAPIOLA_GEMINI_API_KEY="your_api_key_here"
export SAPIOLA_GRAPH_URL="http://127.0.0.1:50053"
uv sync
uvicorn sapiola_ai.api:app --host 0.0.0.0 --port 8000 --reload
```

### Windows (PowerShell):
```powershell
cd sap-ai-gateway
$env:SAPIOLA_GEMINI_API_KEY="your_api_key_here"
$env:SAPIOLA_GRAPH_URL="http://127.0.0.1:50053"
uv sync
uvicorn sapiola_ai.api:app --host 0.0.0.0 --port 8000 --reload
```

---

## 5. Streaming Ingestion Gateway (`sap-streaming-gateway` & `sap-cdc-core`)

### Go CDC Producer (`sap-cdc-core`):
```bash
cd sap-cdc-core
CGO_ENABLED=0 go test ./...
CGO_ENABLED=0 go build -o cdc_producer ./cmd/producer
```

### Docker Container Setup:
```bash
docker compose up -d redpanda
docker build -t sapiola-cdc ./sap-cdc-core
```

---

## 6. Testing & Benchmarking Scripts

| Script | Purpose | Execution Command |
|---|---|---|
| `tools/test_generality_schema.py` | Synthetic non-SALT 5-hop FK test | `python tools/test_generality_schema.py` |
| `tools/salt_importer/test_hana_introspector.py` | SAP HANA catalog introspector test | `python tools/salt_importer/test_hana_introspector.py` |
| `tools/populate_hub.py` | Populate Hub-Spoke graph test nodes | `python tools/populate_hub.py` |
| `scratch/soak_test.py` | Telemetry & write soak test | `python scratch/soak_test.py --duration 60` |
| `scratch/mcp_client_test.mjs` | E2E MCP tool protocol test | `node scratch/mcp_client_test.mjs` |
