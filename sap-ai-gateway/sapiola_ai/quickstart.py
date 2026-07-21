from sapiola_ai.config import SapiolaConfig
from sapiola_ai.orchestrator import RagOrchestrator, SimpleRbacPolicy
from sapiola_ai.llm_binding import LiteLlmClient
from sapiola_ai.embedding_binding import LiteLlmEmbeddingFn
from sapiola_ai.embedding_lancedb import LanceDbEmbeddingClient
from sapiola_ai.graph_client_grpc import GrpcGraphClient
from sapiola_ai.resiliency import ResilientLlmClient

class Sapiola:
    """Single entrypoint. This is the entire public onboarding surface."""

    @classmethod
    async def connect(cls, config: SapiolaConfig | None = None) -> RagOrchestrator:
        cfg = config or SapiolaConfig()  # auto-loads from .env / environment

        llm_client = ResilientLlmClient(LiteLlmClient(cfg.llm_model, cfg.llm_api_key, cfg.llm_api_base))
        graph_client = GrpcGraphClient(cfg.graph_grpc_target)
        
        await graph_client.check_health()
        
        # We pass None for pointer_client right now as it is expected to be part of the cold-path.
        # But wait, orchestrator requires pointer_client. We will stub it or let it fail if used, 
        # but orchestrator __init__ requires it.
        # Wait, the prompt shows `RagOrchestrator(graph_client, llm_client, embedding_client, rbac)`.
        # I need to handle `pointer_client`. I'll use a dummy stub for now since Phase 1 isn't done.
        
        class DummyPointerClient:
            def resolve(self, candidates, principal):
                return []

        embedding_client = None
        if cfg.embedding_model:
            embedding_fn = LiteLlmEmbeddingFn(cfg.embedding_model, cfg.embedding_api_key, cfg.embedding_api_base)
            embedding_client = LanceDbEmbeddingClient(cfg.lancedb_path, embedding_fn)
            await embedding_client.verify_dimensions()

        return RagOrchestrator(
            graph_client=graph_client,
            pointer_client=DummyPointerClient(),
            llm_client=llm_client,
            embedding_client=embedding_client,
            rbac=SimpleRbacPolicy.from_env(),
        )
