import pytest
import asyncio
import time

from sapiola_ai.orchestrator import (
    GraphCandidate,
    PointerFragment,
    Principal,
    RagOrchestrator,
)


class FakeGraphClient:
    def __init__(self, candidates):
        self._candidates = candidates
        self.calls = []

    async def search(self, domain: str, query: str):
        self.calls.append((domain, query))
        return list(self._candidates)

    async def put_vertex(self, tenant_id: str, node_id: int, properties: dict[str, str]) -> bool:
        self.calls.append(("put_vertex", tenant_id, node_id, properties))
        return True

    async def put_edge(self, tenant_id: str, source_id: int, target_id: int, properties: dict[str, str]) -> bool:
        self.calls.append(("put_edge", tenant_id, source_id, target_id, properties))
        return True


class FakePointerClient:
    def __init__(self, fragments):
        self._fragments = fragments
        self.calls = []

    def resolve(self, candidates, principal):
        self.calls.append((tuple(candidates), principal))
        return list(self._fragments)


class FakeEmbeddingClient:
    def __init__(self, candidates):
        self._candidates = candidates
        self.calls = []

    async def recall(self, domain: str, query: str):
        self.calls.append((domain, query))
        return list(self._candidates)


class FakeLlmClient:
    def __init__(self):
        self.prompts = []

    async def complete(self, prompt: str) -> str:
        self.prompts.append(prompt)
        return "ok"


def _graph_candidate_for_domain(domain: str) -> GraphCandidate:
    return GraphCandidate(100, domain.title(), domain, f"{domain[:3].upper()}-100", ("Company", domain.title(), domain.title()))


def _pointer_fragment_for_domain(domain: str) -> PointerFragment:
    path = ("Company", domain.title(), domain.title())
    return PointerFragment(100, path, f"{domain[:3].upper()}-100", f"{domain} context", domain)


@pytest.mark.asyncio
async def test_rag_orchestrator_handles_twenty_queries_without_embeddings():
    cases = [
        ("show material allocation", "inventory"),
        ("find stock shortages", "inventory"),
        ("material and plant lookup", "inventory"),
        ("vendor delivery summary", "procurement"),
        ("supplier scorecard", "procurement"),
        ("payment status by invoice", "finance"),
        ("invoice aging report", "finance"),
        ("plant capacity review", "manufacturing"),
        ("stock and material exceptions", "inventory"),
        ("supplier contract reference", "procurement"),
        ("vendor master lookup", "procurement"),
        ("invoice reconciliation", "finance"),
        ("payment run validation", "finance"),
        ("plant maintenance plan", "manufacturing"),
        ("material lead time details", "inventory"),
        ("supplier payment terms", "procurement"),
        ("vendor onboarding status", "procurement"),
        ("invoice dispute history", "finance"),
        ("plant output forecast", "manufacturing"),
        ("material availability by plant", "inventory"),
    ]

    graph_client = FakeGraphClient([])
    pointer_client = FakePointerClient([])
    llm_client = FakeLlmClient()
    orchestrator = RagOrchestrator(graph_client, pointer_client, llm_client)
    principal = Principal("alice", frozenset({"*"}))

    for query, expected_domain in cases:
        graph_client._candidates = [_graph_candidate_for_domain(expected_domain)]
        pointer_client._fragments = [_pointer_fragment_for_domain(expected_domain)]

        result = await orchestrator.answer(query, principal)

        assert result.domain == expected_domain
        assert result.embedding_calls == 0
        assert len(result.candidates) == 1
        assert len(result.fragments) == 1
        assert query in result.prompt
        assert llm_client.prompts[-1] == result.prompt


@pytest.mark.asyncio
async def test_rag_orchestrator_blocks_embeddings_and_filters_rbac():
    candidates = [
        GraphCandidate(1, "Material", "inventory", "MAT-1", ("Company", "BU", "Plant", "Material")),
        GraphCandidate(2, "Invoice", "finance", "INV-9", ("Company", "Finance", "Invoice")),
    ]
    fragments = [
        PointerFragment(1, ("Company", "BU", "Plant", "Material"), "MAT-1", "Material fragment", "inventory"),
        PointerFragment(2, ("Company", "Finance", "Invoice"), "INV-9", "Invoice fragment", "finance"),
    ]

    graph_client = FakeGraphClient(candidates)
    pointer_client = FakePointerClient(fragments)
    llm_client = FakeLlmClient()
    orchestrator = RagOrchestrator(graph_client, pointer_client, llm_client)
    principal = Principal("alice", frozenset({"inventory"}))

    result = await orchestrator.answer("show material details", principal)

    assert result.domain == "inventory"
    assert result.embedding_calls == 0
    assert len(result.candidates) == 1
    assert len(result.fragments) == 1
    assert result.candidates[0].sap_key == "MAT-1"
    assert "Candidates:" in result.prompt
    assert llm_client.prompts == [result.prompt]


@pytest.mark.asyncio
async def test_rag_orchestrator_denies_cross_domain_access():
    orchestrator = RagOrchestrator(FakeGraphClient([]), FakePointerClient([]), FakeLlmClient())

    try:
        await orchestrator.answer("invoice lookup", Principal("bob", frozenset({"inventory"})))
    except PermissionError as exc:
        assert "invoice" in str(exc) or "finance" in str(exc)
    else:
        raise AssertionError("cross-domain query should be rejected")


@pytest.mark.asyncio
async def test_rag_orchestrator_hybrid_widening():
    graph_candidates = [
        GraphCandidate(1, "Material", "inventory", "MAT-1", ("Company", "BU", "Plant", "Material")),
    ]
    embed_candidates = [
        GraphCandidate(1, "Material", "inventory", "MAT-1", ("Company", "BU", "Plant", "Material")), # Duplicate
        GraphCandidate(2, "Material", "inventory", "MAT-2", ("Company", "BU", "Plant", "Material")), # New
    ]
    fragments = [
        PointerFragment(1, ("Company", "BU", "Plant", "Material"), "MAT-1", "Material 1", "inventory"),
        PointerFragment(2, ("Company", "BU", "Plant", "Material"), "MAT-2", "Material 2", "inventory"),
    ]

    graph_client = FakeGraphClient(graph_candidates)
    pointer_client = FakePointerClient(fragments)
    llm_client = FakeLlmClient()
    embed_client = FakeEmbeddingClient(embed_candidates)
    
    orchestrator = RagOrchestrator(graph_client, pointer_client, llm_client, embedding_client=embed_client)
    principal = Principal("alice", frozenset({"inventory"}))

    result = await orchestrator.answer("find materials", principal, enable_embeddings=True)

    assert result.domain == "inventory"
    assert result.embedding_calls == 1
    assert len(result.candidates) == 2  # Deduplicated
    assert len(result.fragments) == 2
    assert {"MAT-1", "MAT-2"} == {c.sap_key for c in result.candidates}


@pytest.mark.asyncio
async def test_rag_orchestrator_hybrid_rbac():
    # Simulate an EmbeddingClient that accidentally or maliciously returns out-of-domain results
    graph_candidates = [
        GraphCandidate(1, "Material", "inventory", "MAT-1", ("Company", "BU", "Plant", "Material")),
    ]
    embed_candidates = [
        GraphCandidate(2, "Material", "inventory", "MAT-2", ("Company", "BU", "Plant", "Material")), 
        GraphCandidate(3, "Invoice", "finance", "INV-9", ("Company", "Finance", "Invoice")), # Cross-domain!
    ]
    fragments = [
        PointerFragment(1, ("Company", "BU", "Plant", "Material"), "MAT-1", "Material 1", "inventory"),
        PointerFragment(2, ("Company", "BU", "Plant", "Material"), "MAT-2", "Material 2", "inventory"),
        PointerFragment(3, ("Company", "Finance", "Invoice"), "INV-9", "Invoice 9", "finance"),
    ]

    graph_client = FakeGraphClient(graph_candidates)
    pointer_client = FakePointerClient(fragments)
    llm_client = FakeLlmClient()
    embed_client = FakeEmbeddingClient(embed_candidates)
    
    orchestrator = RagOrchestrator(graph_client, pointer_client, llm_client, embedding_client=embed_client)
    principal = Principal("alice", frozenset({"inventory"})) # Only inventory allowed

    result = await orchestrator.answer("find materials", principal, enable_embeddings=True)

    assert result.embedding_calls == 1
    assert len(result.candidates) == 2
    assert {"MAT-1", "MAT-2"} == {c.sap_key for c in result.candidates}
    assert "INV-9" not in {c.sap_key for c in result.candidates}


class SlowGraphClient(FakeGraphClient):
    async def search(self, domain: str, query: str):
        await asyncio.sleep(0.1)
        return await super().search(domain, query)

class SlowEmbeddingClient(FakeEmbeddingClient):
    async def recall(self, domain: str, query: str):
        await asyncio.sleep(0.1)
        return await super().recall(domain, query)

@pytest.mark.asyncio
async def test_rag_orchestrator_concurrency():
    graph_client = SlowGraphClient([])
    pointer_client = FakePointerClient([])
    llm_client = FakeLlmClient()
    embed_client = SlowEmbeddingClient([])
    
    orchestrator = RagOrchestrator(graph_client, pointer_client, llm_client, embedding_client=embed_client)
    principal = Principal("alice", frozenset({"inventory"}))

    start = time.monotonic()
    await orchestrator.answer("find materials", principal, enable_embeddings=True)
    duration = time.monotonic() - start

    # If they were sequential, it would be >= 0.2s. 
    # With gather(), it should be close to max(0.1, 0.1) = 0.1s.
    assert duration < 0.15, f"Expected concurrent execution taking ~0.1s, took {duration}s"

from sapiola_ai.resiliency import resilient_call, default_breaker
from sapiola_ai.orchestrator import LlmUnavailableError
from prometheus_client import REGISTRY

class FailingLlmClient(FakeLlmClient):
    def __init__(self):
        super().__init__()
        self.attempts = 0

    @resilient_call()
    async def complete(self, prompt: str) -> str:
        self.attempts += 1
        raise TimeoutError("LLM timed out")

@pytest.mark.asyncio
async def test_rag_orchestrator_degraded_mode_and_circuit_breaker():
    # Reset breaker for this test
    default_breaker.close()
    
    graph_client = FakeGraphClient([_graph_candidate_for_domain("inventory")])
    pointer_client = FakePointerClient([_pointer_fragment_for_domain("inventory")])
    failing_llm = FailingLlmClient()
    
    orchestrator = RagOrchestrator(graph_client, pointer_client, failing_llm)
    principal = Principal("alice", frozenset({"inventory"}))

    # First call will exhaust retries (4 attempts) and count as 1 failure for the breaker
    start_time = time.monotonic()
    result1 = await orchestrator.answer("find stock shortages", principal)
    assert failing_llm.attempts == 4  # stop_after_attempt(4)
    assert result1.answer == ""
    assert len(result1.candidates) == 1
    
    # Let's force the breaker open by simulating 4 more orchestrator-level failures
    # (Each call exhausts 4 retries, so attempts goes up by 4 each time)
    for _ in range(4):
        await orchestrator.answer("find stock shortages", principal)
        
    assert failing_llm.attempts == 20  # 5 calls * 4 attempts
    assert default_breaker.current_state.name == "OPEN"
    
    # 6th call: Breaker is OPEN. It should fail fast without any retries.
    start_time = time.monotonic()
    result6 = await orchestrator.answer("find stock shortages", principal)
    duration = time.monotonic() - start_time
    
    # Attempts shouldn't have increased because the breaker intercepted it before calling the function
    assert failing_llm.attempts == 20
    assert result6.answer == ""
    assert duration < 0.1  # Fail fast!

@pytest.mark.asyncio
async def test_rag_orchestrator_observability():
    import structlog
    from structlog.testing import LogCapture
    
    log_capture = LogCapture()
    old_processors = structlog.get_config()["processors"]
    structlog.configure(processors=[log_capture])
    
    graph_candidates = [
        GraphCandidate(1, "Material", "inventory", "MAT-1", ("Company", "BU", "Plant", "Material")),
        GraphCandidate(2, "Invoice", "finance", "INV-9", ("Company", "Finance", "Invoice")),
    ]
    fragments = [
        PointerFragment(1, ("Company", "BU", "Plant", "Material"), "MAT-1", "Material 1", "inventory"),
        PointerFragment(2, ("Company", "Finance", "Invoice"), "INV-9", "Invoice 9", "finance"),
    ]
    
    graph_client = FakeGraphClient(graph_candidates)
    pointer_client = FakePointerClient(fragments)
    llm_client = FakeLlmClient()
    
    orchestrator = RagOrchestrator(graph_client, pointer_client, llm_client)
    principal = Principal("alice", frozenset({"inventory"}))
    
    # This query will retrieve both candidates, but the finance one will be stripped by RBAC defense-in-depth
    before_denials = REGISTRY.get_sample_value('rag_rbac_denials_total', labels={'domain': 'inventory'}) or 0.0
    
    await orchestrator.answer("find materials", principal)
    
    after_denials = REGISTRY.get_sample_value('rag_rbac_denials_total', labels={'domain': 'inventory'})
    assert after_denials == before_denials + 1.0
    
    # Verify the structured log
    denial_logs = [log for log in log_capture.entries if log.get("event") == "rbac_access_denied"]
    assert len(denial_logs) == 1
    assert denial_logs[0]["denied_entity_ids"] == ["INV-9"]
    
    structlog.configure(processors=old_processors)

@pytest.mark.asyncio
async def test_rag_orchestrator_write_authorized():
    from sapiola_ai.orchestrator import SimpleRbacPolicy
    
    graph_client = FakeGraphClient([])
    pointer_client = FakePointerClient([])
    llm_client = FakeLlmClient()
    
    rbac = SimpleRbacPolicy(write_principals=frozenset({"alice"}))
    orchestrator = RagOrchestrator(graph_client, pointer_client, llm_client, rbac=rbac)
    
    # Alice is authorized
    principal = Principal("alice", frozenset({"inventory"}))
    success = await orchestrator.put_vertex(principal, "inventory", 1, {"name": "Test"})
    assert success is True
    assert graph_client.calls[0] == ("put_vertex", "inventory", 1, {"name": "Test"})

    success = await orchestrator.put_edge(principal, "inventory", 1, 2, {"type": "REL"})
    assert success is True
    assert graph_client.calls[1] == ("put_edge", "inventory", 1, 2, {"type": "REL"})

@pytest.mark.asyncio
async def test_rag_orchestrator_write_unauthorized():
    from sapiola_ai.orchestrator import SimpleRbacPolicy
    
    graph_client = FakeGraphClient([])
    pointer_client = FakePointerClient([])
    llm_client = FakeLlmClient()
    
    # Only alice can write
    rbac = SimpleRbacPolicy(write_principals=frozenset({"alice"}))
    orchestrator = RagOrchestrator(graph_client, pointer_client, llm_client, rbac=rbac)
    
    # Bob is unauthorized
    principal = Principal("bob", frozenset({"inventory"}))
    
    with pytest.raises(PermissionError, match="Principal bob is not authorized to write"):
        await orchestrator.put_vertex(principal, "inventory", 1, {"name": "Test"})

    with pytest.raises(PermissionError, match="Principal bob is not authorized to write"):
        await orchestrator.put_edge(principal, "inventory", 1, 2, {"type": "REL"})