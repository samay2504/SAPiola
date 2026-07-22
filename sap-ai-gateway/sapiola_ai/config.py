from pydantic_settings import BaseSettings, SettingsConfigDict

class SapiolaConfig(BaseSettings):
    # LLM
    llm_model: str                       # e.g. "openrouter/anthropic/claude-3.5-sonnet"
    llm_api_key: str | None = None       # optional
    llm_api_base: str | None = None      # only needed for custom/self-hosted endpoints

    # Embeddings (optional — omit entirely to run structural-only RAG)
    embedding_model: str | None = None
    embedding_api_key: str | None = None
    embedding_api_base: str | None = None

    # SAP connection
    sap_dsn: str                         # e.g. "hana://user:pass@host:port"

    # Local infra
    lancedb_path: str = "./sapiola_data/embeddings"
    graph_grpc_target: str = "localhost:50053"

    model_config = SettingsConfigDict(env_prefix="SAPIOLA_", env_file=".env", extra="ignore")
