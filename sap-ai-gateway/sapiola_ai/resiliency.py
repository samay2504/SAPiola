from datetime import timedelta
import logging
import structlog
from tenacity import (
    AsyncRetrying,
    stop_after_attempt,
    wait_exponential,
    retry_if_exception_type,
    before_sleep_log,
)
from aiobreaker import CircuitBreaker

from sapiola_ai.orchestrator import LlmUnavailableError

log = structlog.get_logger()

# Shared circuit breaker for LLM/Network calls.
# 5 consecutive failures opens the breaker for 30 seconds.
default_breaker = CircuitBreaker(fail_max=5, timeout_duration=timedelta(seconds=30))

def resilient_call(breaker=default_breaker):
    """
    Decorator that applies both transient retries (Tenacity) and 
    sustained failure circuit-breaking (aiobreaker).
    Composition is Breaker(Retry(Function)), meaning retries happen inside
    the breaker. The breaker only counts a failure if ALL retries fail.
    """
    def decorator(func):
        # 1. Setup Retry (transient failures)
        # We retry on typical transient network exceptions (TimeoutError, ConnectionError)
        async def retried_func(*args, **kwargs):
            async for attempt in AsyncRetrying(
                stop=stop_after_attempt(4),
                wait=wait_exponential(multiplier=0.01, min=0.01, max=0.05),
                retry=retry_if_exception_type((TimeoutError, ConnectionError)),
                before_sleep=before_sleep_log(log, logging.WARNING),
                reraise=True,
            ):
                with attempt:
                    return await func(*args, **kwargs)
        
        # 2. Setup Breaker (sustained failures)
        # The breaker wraps the retried function.
        broken_func = breaker(retried_func)
        
        async def wrapper(*args, **kwargs):
            try:
                return await broken_func(*args, **kwargs)
            except Exception as e:
                # If the breaker is open, or if we exhausted retries, 
                # we surface it as LlmUnavailableError (or GraphUnavailableError)
                # to trigger degraded mode in the orchestrator.
                raise LlmUnavailableError("Service unavailable due to sustained or transient failures") from e
                
        return wrapper
    return decorator


class ResilientLlmClient:
    """Wraps an LlmClient with the resilient_call circuit breaker logic."""
    def __init__(self, llm_client):
        self._llm_client = llm_client

    @resilient_call(default_breaker)
    async def complete(self, prompt: str) -> str:
        return await self._llm_client.complete(prompt)
