# SAPiola Zero-ETL Graph & AI Platform

**SAPiola** is a state-of-the-art Zero-ETL platform designed to seamlessly transform raw Change Data Capture (CDC) events from SAP HANA into a high-performance Knowledge Graph, layered with a Generative AI (RAG) Orchestrator, all accessible via the Model Context Protocol (MCP).

Unlike traditional batch ETL pipelines, SAPiola instantly decouples complex relational SAP attributes from structural hierarchy. It uses **Fjall** (Log-Structured Merge trees) for high-concurrency relational state storage and **PuppyGraph** for instantaneous Cypher-based structural querying. 

## Key Features

- **Zero-ETL Streaming:** Consume SAP HANA (or mock SALT) data live via `sap-streaming-gateway` without heavy batch transformations.
- **Decoupled Architecture:** 
  - **Structural Data:** Handled by a Cypher-parsing Rust engine (`sap-graph-layer`) pointing to PuppyGraph.
  - **Relational Data:** Handled by a fast `poly-lsm-core` index, keeping the graph memory footprint light.
- **RAG AI Orchestrator:** A Python-based `sap-ai-gateway` that uses `litellm` (supporting Gemini, OpenAI, etc.) to orchestrate domain classification, RBAC checks, graph queries, and LSM pointer resolution before generating hallucinations-free answers.
- **Universal Agent Access (MCP):** Connect your preferred AI Assistant (Cursor, Claude Desktop) to the `sap-mcp-server` to allow it to natively query the graph or ask SAP natural language questions.
- **Deep RBAC Security:** Tenant IDs (`principal`) are passed through the entire stack. From graph projection to embedding retrieval, if an entity lacks access to a domain (e.g., `finance`), the system stops it cold (`403 Forbidden`).

## Project Structure

The SAPiola monorepo consists of polyglot microservices tailored for performance:

| Component | Language | Description |
|-----------|----------|-------------|
| `sap-cdc-core` | Go | The original CDC Agent / Ingest Gateway. |
| `sap-streaming-gateway` | Rust | High-throughput Kafka/Redpanda consumer and dispatcher. |
| `sap-graph-layer` | Rust | Graph persistence, Cypher-parsing (`pest`), and `RelationalBackend` executor. |
| `poly-lsm-core` | Rust | Fjall DB abstraction for rapid relational pointer storage. |
| `sap-ai-gateway` | Python | RAG Orchestrator (FastAPI), Litellm bindings, and Domain Classification. |
| `sap-mcp-server` | Rust | Native MCP Protocol server defining `run_graph_query` and `ask_sap` tools. |
| `sapiola-mcp` | Node.js | Wrapper for the MCP Server for easy distribution and `npm` installation. |
| `tools/salt_importer` | Python | Testing utility to stream mock SAP (SALT) data into the ingest layer. |

## Quickstart

### 1. Prerequisites
- `cargo` (Rust toolchain)
- `go` (1.21+)
- `python` 3.12+ (with `uv` installed)
- `npm`

### 2. Stream Mock Data
To get started without a live SAP HANA instance, you can use the SALT dataset importer.
```bash
cd tools/salt_importer
python importer.py
```
This will parse the `salt_mapping.dsl` and stream mock `CdcEvent` protobufs into the ingest gateway.

### 3. Start the AI Gateway
```bash
cd sap-ai-gateway
uv sync
uvicorn sapiola_ai.api:app --host 0.0.0.0 --port 8000
```

### 4. Connect via MCP
If you are developing locally, you can use the `sapiola-mcp` wrapper, which will safely fall back to a local Cargo build if a release binary is not found.
```bash
cd sapiola-mcp
npm install
node bin/run.js
```

## Further Reading
- [Architecture Deep Dive](architecture.md)
- [Developer Guide](developer_guide.md)
- [Command Reference](commands.md)
