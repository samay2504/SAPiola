import litellm

class LiteLlmClient:
    """Concrete LlmClient — works for OpenAI, Anthropic, OpenRouter, Bedrock,
    Gemini, local Ollama, or any of litellm's 100+ supported providers, purely
    by changing a model string. No provider-specific code lives here.
    """
    def __init__(self, model: str, api_key: str | None = None, api_base: str | None = None):
        self._model = model
        self._api_key = api_key
        self._api_base = api_base

    async def complete(self, prompt: str) -> str:
        # For simplicity, we pass the raw prompt as the user message.
        # In a more complex setup, you'd separate system instructions from the query,
        # but the prompt built by RagOrchestrator includes all context.
        messages = [{"role": "user", "content": prompt}]
        
        response = await litellm.acompletion(
            model=self._model,
            messages=messages,
            api_key=self._api_key,
            api_base=self._api_base,
            timeout=30,
        )
        return response.choices[0].message.content
