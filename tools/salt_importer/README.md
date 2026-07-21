# SALT Importer Tool

This tool is responsible for programmatically downloading the `sap-ai-research/SALT` dataset from Hugging Face and streaming it into the SAPiola Ingest Gateway over gRPC.

## Architecture

1. **Introspector (`introspector.py`)**: Dynamically scans the dataset to discover foreign key relationships empirically (via value-overlap checks) and outputs a draft `salt_mapping.dsl`.
2. **Importer (`importer.py`)**: Consumes the confirmed `salt_mapping.dsl`, connects to the Gateway, and streams the dataset rows as `OPERATION_INSERT` CDC events.

## Prerequisites

- You must have a Hugging Face account and accept the terms of the dataset at [https://huggingface.co/datasets/sap-ai-research/SALT](https://huggingface.co/datasets/sap-ai-research/SALT).
- Export your token: `export HF_TOKEN="your_token"`
- Run `uv pip install -r requirements.txt` (or install `grpcio`, `protobuf`, `datasets`, `pandas`).

## License Warning

> [!WARNING]
> **CC-BY-NC-SA 4.0 (Non-Commercial)**
> The `sap-ai-research/SALT` dataset is licensed strictly for non-commercial research use.
> **DO NOT** bundle, distribute, or ship SALT data as part of any open-source release artifacts, Docker images, or commercial deployments. This importer tool is strictly a local dev/test fixture.
