# SALT Importer Tool

This tool is responsible for programmatically downloading the `sap-ai-research/SALT` dataset from Hugging Face and streaming it into the SAPiola Ingest Gateway over gRPC.

## Architecture

1. **Credential Resolver (`credential_resolver.py`)**: Unified resolution chain (`env vars → service key file → fail-with-message`). Catches OAuth UAA-only service keys and prevents hardcoded fallbacks.
2. **HANA Schema Introspector (`hana_introspector.py`)**: Queries live SAP HANA metadata (`SYS.TABLE_COLUMNS`), detects PKs via uniqueness sampling, discovers foreign keys empirically (`score_foreign_key_confidence`), and outputs `schema_manifest.json` + `hana_mapping.dsl`.
3. **Dataset Introspector (`introspector.py`)**: Generic CLI for discovering FK relationships on offline Pandas DataFrames.
4. **Importer (`importer.py`)**: Streams HuggingFace datasets or CDC events to the Ingest Gateway using primary key configurations from `schema_manifest.json` or mapping DSL.

## Prerequisites

- You must have a Hugging Face account and accept the terms of the dataset at [https://huggingface.co/datasets/sap-ai-research/SALT](https://huggingface.co/datasets/sap-ai-research/SALT).
- Export your token: `export HF_TOKEN="your_token"`
- Run `uv pip install -r requirements.txt` (or install `grpcio`, `protobuf`, `datasets`, `pandas`).

## License Warning

> [!WARNING]
> **CC-BY-NC-SA 4.0 (Non-Commercial)**
> The `sap-ai-research/SALT` dataset is licensed strictly for non-commercial research use.
> **DO NOT** bundle, distribute, or ship SALT data as part of any open-source release artifacts, Docker images, or commercial deployments. This importer tool is strictly a local dev/test fixture.
