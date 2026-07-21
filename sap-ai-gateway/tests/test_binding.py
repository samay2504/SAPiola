import pytest
from unittest.mock import AsyncMock, patch

from sapiola_ai.llm_binding import LiteLlmClient
from sapiola_ai.embedding_binding import LiteLlmEmbeddingFn

@pytest.mark.asyncio
async def test_litellm_completion_mock():
    client = LiteLlmClient(model="openai/gpt-4o", api_key="sk-test", api_base="http://test")
    
    with patch("litellm.acompletion", new_callable=AsyncMock) as mock_acompletion:
        mock_response = AsyncMock()
        mock_response.choices = [AsyncMock()]
        mock_response.choices[0].message.content = "Mocked answer"
        mock_acompletion.return_value = mock_response
        
        result = await client.complete("Test prompt")
        
        assert result == "Mocked answer"
        mock_acompletion.assert_called_once()
        kwargs = mock_acompletion.call_args.kwargs
        assert kwargs["model"] == "openai/gpt-4o"
        assert kwargs["api_key"] == "sk-test"
        assert kwargs["api_base"] == "http://test"
        assert kwargs["messages"] == [{"role": "user", "content": "Test prompt"}]


@pytest.mark.asyncio
async def test_litellm_embedding_mock():
    embed_fn = LiteLlmEmbeddingFn(model="openai/text-embedding-3-small", api_key="sk-test", api_base="http://test")
    
    with patch("litellm.aembedding", new_callable=AsyncMock) as mock_aembedding:
        mock_response = AsyncMock()
        mock_response.data = [{"embedding": [0.1, 0.2, 0.3]}]
        mock_aembedding.return_value = mock_response
        
        result = await embed_fn.embed("Test text")
        
        assert result == [0.1, 0.2, 0.3]
        mock_aembedding.assert_called_once()
        kwargs = mock_aembedding.call_args.kwargs
        assert kwargs["model"] == "openai/text-embedding-3-small"
        assert kwargs["api_key"] == "sk-test"
        assert kwargs["api_base"] == "http://test"
        assert kwargs["input"] == ["Test text"]
        
    with patch("litellm.aembedding", new_callable=AsyncMock) as mock_aembedding:
        mock_response = AsyncMock()
        mock_response.data = [{"embedding": [0.1, 0.2, 0.3, 0.4]}]
        mock_aembedding.return_value = mock_response
        
        dim = await embed_fn.get_dimensions()
        assert dim == 4
