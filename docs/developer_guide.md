# SAPiola Developer Guide

This guide provides instructions for onboarding developers to the SAPiola Zero-ETL monorepo. It covers environment setup, running the local stack, and contributing to the polyglot services.

## Environment Setup

SAPiola is a polyglot system requiring several runtimes. Ensure you have the following installed:
1. **Rust (Cargo):** Latest stable release (1.75+). Required for `sap-graph-layer`, `sap-streaming-gateway`, `poly-lsm-core`, and `sap-mcp-server`.
2. **Go:** 1.21+. Required for `sap-cdc-core`.
3. **Python:** 3.12+. We recommend using `uv` for lightning-fast environment management. Required for `sap-ai-gateway` and `tools/salt_importer`.
4. **Node.js & npm:** 18+. Required for the `sapiola-mcp` wrapper.
5. **Docker & Docker Compose:** Required to run the Kafka/Redpanda broker locally for streaming.

## Running the Local Stack

### 1. Start Core Infrastructure
You will need a Kafka or Redpanda broker running to handle CDC event routing. (Assumes a standard `docker-compose.yml` at the root).
```bash
docker-compose up -d
```

### 2. Start the SAP Graph Layer (Rust)
The graph layer must be running for both ingestion and querying to work.
```bash
cd sap-graph-layer
GRAPH_SERVER_LISTEN_ADDR="127.0.0.1:50053" cargo run --release
```
*(This will bind the GraphServiceServer to port 50053).*

### 3. Start the Ingestion Gateway (Rust/Go)
If you are testing against mock data, run the streaming gateway.
```bash
cd sap-streaming-gateway
cargo run
```

### 4. Run the SALT Data Importer (Python)
Instead of a live HANA instance, stream the mock dataset:
```bash
cd tools/salt_importer
python importer.py
```

### 5. Start the AI Gateway (Python)
Ensure you have set the `SAPIOLA_GEMINI_API_KEY` (or your preferred Litellm key) in `sap-ai-gateway/.env`.
```bash
cd sap-ai-gateway
uv sync
# Activate virtual environment if needed
uvicorn sapiola_ai.api:app --host 0.0.0.0 --port 8000 --reload
```

## Adding a New SAP Domain

SAP is massive. To add support for a new SAP Domain (e.g., `HR` or `Logistics`):

1. **Update `salt_mapping.dsl`:**
   Navigate to `tools/salt_importer/salt_mapping.dsl` and define the extraction rules for your new table. Ensure the primary key maps correctly to the 7th block (`parts[7]`) per the DSL parser.

2. **Update the AI Domain Classifier:**
   Open `sap-ai-gateway/sapiola_ai/orchestrator.py` and add your keyword mapping to the `DomainClassifier` rules.
   ```python
   _rules: tuple[tuple[str, str], ...] = (
       # ... existing rules ...
       ("employee", "hr"),
       ("payroll", "hr"),
   )
   ```

3. **Verify RBAC Policies:**
   Ensure that the `SimpleRbacPolicy` or your external IAM is aware of the new `hr` domain so it doesn't default to a `403 Forbidden`.

## Testing

### End-to-End Verification
We have provided an E2E testing script that simulates LLM queries against the RAG orchestrator, testing both happy paths and RBAC rejection paths.
```bash
python scratch/verify_e2e.py
```

### Rust Unit Tests
To run tests across all Rust crates:
```bash
cargo test --workspace
```
