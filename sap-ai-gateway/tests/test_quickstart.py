import os
import pytest
from unittest.mock import patch, AsyncMock
import lancedb

from sapiola_ai.quickstart import Sapiola, SapiolaConfig
from sapiola_ai.orchestrator import RagOrchestrator
from sapiola_ai.embedding_lancedb import LanceDbEmbeddingClient
from sapiola_ai.embedding_indexer import EmbeddingIndexer
from sapiola_ai.embedding_binding import LiteLlmEmbeddingFn

@pytest.fixture
def mock_env(tmp_path):
    env_file = tmp_path / ".env"
    env_file.write_text(
        "SAPIOLA_LLM_MODEL=openrouter/anthropic/claude-3.5-sonnet\n"
        "SAPIOLA_LLM_API_KEY=sk-test-llm\n"
        "SAPIOLA_LLM_API_BASE=http://openrouter.test\n"
        "SAPIOLA_SAP_DSN=hana://test\n"
        f"SAPIOLA_LANCEDB_PATH={tmp_path}/lancedb\n"
        "SAPIOLA_EMBEDDING_MODEL=openai/text-embedding-3-small\n"
        "SAPIOLA_EMBEDDING_API_KEY=sk-test-emb\n"
    )
    # Temporarily change directory so pydantic-settings finds the .env file if we pass env_file param
    # Wait, we can just pass the path to SettingsConfigDict or mock os.environ
    return env_file, tmp_path

@pytest.mark.asyncio
async def test_quickstart_cold_start(mock_env):
    env_file, tmp_path = mock_env
    
    # Override config to use the test .env file
    config = SapiolaConfig(_env_file=str(env_file))
    
    # We must mock get_dimensions to avoid hitting LiteLLM during Sapiola.connect
    with patch("sapiola_ai.embedding_binding.LiteLlmEmbeddingFn.get_dimensions", new_callable=AsyncMock) as mock_dims:
        mock_dims.return_value = 384
        
        # Pre-create the table so LanceDbEmbeddingClient doesn't crash on open_table
        import pyarrow as pa
        import lancedb
        db = lancedb.connect(str(tmp_path / "lancedb"))
        schema = pa.schema([pa.field("vector", pa.list_(pa.float32(), 384))])
        db.create_table("entity_embeddings", schema=schema)
        
        orchestrator = await Sapiola.connect(config)
        
        assert isinstance(orchestrator, RagOrchestrator)
        assert isinstance(orchestrator._embedding_client, LanceDbEmbeddingClient)
        # Verify wiring
        assert orchestrator._embedding_client._embedding_fn._model == "openai/text-embedding-3-small"
        
        # Verify llm wiring
        llm = orchestrator._llm_client._llm_client # Unwrap ResilientLlmClient
        assert llm._model == "openrouter/anthropic/claude-3.5-sonnet"
        assert llm._api_key == "sk-test-llm"
        assert llm._api_base == "http://openrouter.test"


@pytest.mark.asyncio
async def test_quickstart_no_embedding(mock_env):
    env_file, tmp_path = mock_env
    # Overwrite .env without embedding_model
    env_file.write_text(
        "SAPIOLA_LLM_MODEL=openai/gpt-4\n"
        "SAPIOLA_SAP_DSN=hana://test\n"
        f"SAPIOLA_LANCEDB_PATH={tmp_path}/lancedb\n"
    )
    
    config = SapiolaConfig(_env_file=str(env_file))
    
    orchestrator = await Sapiola.connect(config)
    assert orchestrator._embedding_client is None
    

@pytest.mark.asyncio
async def test_dimension_mismatch_raises_error(tmp_path):
    db_path = str(tmp_path / "lancedb")
    # First, create a table with dimension 384
    embed_fn_384 = LiteLlmEmbeddingFn("test_384")
    with patch.object(embed_fn_384, "get_dimensions", return_value=384):
        indexer = EmbeddingIndexer(db_path, embed_fn_384)
        await indexer.index([]) # creates empty table
        
    # Now try to connect with a model that has dimension 1536
    embed_fn_1536 = LiteLlmEmbeddingFn("test_1536")
    with patch.object(embed_fn_1536, "get_dimensions", return_value=1536):
        client = LanceDbEmbeddingClient(db_path, embed_fn_1536)
        with pytest.raises(ValueError, match="Embedding dimension mismatch"):
            await client.verify_dimensions()

        # Same for indexer appending
        indexer2 = EmbeddingIndexer(db_path, embed_fn_1536)
        with pytest.raises(ValueError, match="Embedding dimension mismatch"):
            await indexer2.index([])
