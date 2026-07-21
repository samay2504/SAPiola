# SAPiola Command & CLI Reference

This document provides a quick reference for the various CLI commands and scripts used to operate the SAPiola platform.

## 1. Mock Data Importer (`tools/salt_importer`)

The Python importer streams mock SALT data into the SAPiola ingest gateway.

**Location:** `tools/salt_importer/`

**Basic Usage:**
```bash
python importer.py
```
*Note: This requires `sap-streaming-gateway` (or `sap-cdc-core`) to be actively listening on `localhost:50051`, otherwise the gRPC connection will refuse.*

## 2. SAP AI Gateway (`sap-ai-gateway`)

The FastAPI-based Generative AI orchestration layer.

**Location:** `sap-ai-gateway/`

**Environment Variables Required:**
- `SAPIOLA_GEMINI_API_KEY` (or other litellm provider keys)
- `SAPIOLA_GRAPH_URL` (defaults to `http://[::1]:50051`)

**Run Development Server:**
```bash
# Using uvicorn with hot-reload
uvicorn sapiola_ai.api:app --host 0.0.0.0 --port 8000 --reload
```

**Testing:**
```bash
pytest tests/
```

## 3. SAP MCP Server (`sap-mcp-server` & `sapiola-mcp`)

The Model Context Protocol integration allows external AI agents to query the graph.

### Rust Backend (`sap-mcp-server`)
**Location:** `sap-mcp-server/`

**Build Release Binary:**
```bash
cargo build --release -p sap-mcp-server
```

**Run Directly (Stdio Transport):**
```bash
cargo run --release -p sap-mcp-server
```

### Node.js Wrapper (`sapiola-mcp`)
**Location:** `sapiola-mcp/`

The Node.js wrapper acts as a bridge for MCP clients (like Claude Desktop) that expect standard `npm` modules.

**Install Dependencies:**
```bash
npm install
```
*Note: The `postinstall` script (`scripts/fetch-binary.js`) will attempt to download a pre-built binary. If it fails, it will loudly log a warning and fallback to executing `cargo build --release` from the Rust workspace.*

**Force Developer Fallback Build:**
```bash
SAPIOLA_MCP_DEV_BUILD=1 npm install
```

**Run the MCP Server:**
```bash
node bin/run.js
```

## 4. Rust Backend Services (`sap-graph-layer` & `sap-streaming-gateway`)

**Start Graph Layer:**
```bash
cd sap-graph-layer
cargo run --release
```

**Start Streaming Gateway:**
```bash
cd sap-streaming-gateway
cargo run --release
```
