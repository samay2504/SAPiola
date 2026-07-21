import litellm

class LiteLlmEmbeddingFn:
    """Concrete embedding function for LanceDbEmbeddingClient — replaces the
    dummy-vector stub. Works with OpenAI, Cohere, HuggingFace inference,
    or any litellm-supported embedding model via the same model-string pattern.
    """
    def __init__(self, model: str = "openai/text-embedding-3-small", api_key: str | None = None, api_base: str | None = None):
        self._model = model
        self._api_key = api_key
        self._api_base = api_base

    async def embed(self, text: str) -> list[float]:
        response = await litellm.aembedding(
            model=self._model, 
            input=[text], 
            api_key=self._api_key, 
            api_base=self._api_base
        )
        return response.data[0]["embedding"]

    async def get_dimensions(self) -> int:
        """Helper to fetch the embedding dimension by performing a fast test embed."""
        vec = await self.embed("test")
        return len(vec)
