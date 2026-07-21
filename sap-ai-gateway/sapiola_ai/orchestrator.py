from __future__ import annotations
import asyncio
import time
import uuid

import structlog
from sapiola_ai.metrics import (
    RAG_LLM_LATENCY,
    RAG_DEGRADED_RESPONSES,
    RAG_RBAC_DENIALS,
    RAG_EMBEDDING_FALLBACK_RATE,
)

log = structlog.get_logger()

from dataclasses import dataclass, field
from typing import Iterable, Protocol

class LlmUnavailableError(Exception):
    """Raised when the LLM client is unavailable (e.g. circuit breaker open)."""
    pass

@dataclass(frozen=True)
class Principal:
    name: str
    domains: frozenset[str]


@dataclass(frozen=True)
class GraphCandidate:
    node_id: int
    label: str
    domain: str
    sap_key: str
    pointer_path: tuple[str, ...]


@dataclass(frozen=True)
class PointerFragment:
    node_id: int
    path: tuple[str, ...]
    sap_key: str
    text: str
    domain: str


@dataclass(frozen=True)
class RagResult:
    domain: str
    answer: str
    prompt: str
    candidates: tuple[GraphCandidate, ...] = field(default_factory=tuple)
    fragments: tuple[PointerFragment, ...] = field(default_factory=tuple)
    embedding_calls: int = 0
    degraded: bool = False


class GraphClient(Protocol):
    async def search(self, domain: str, query: str) -> list[GraphCandidate]:
        raise NotImplementedError
    async def put_vertex(self, tenant_id: str, node_id: int, properties: dict[str, str]) -> bool:
        raise NotImplementedError
    async def put_edge(self, tenant_id: str, source_id: int, target_id: int, properties: dict[str, str]) -> bool:
        raise NotImplementedError


class PointerIndexClient(Protocol):
    def resolve(self, candidates: Iterable[GraphCandidate], principal: Principal) -> list[PointerFragment]:
        raise NotImplementedError


class LlmClient(Protocol):
    async def complete(self, prompt: str) -> str:
        raise NotImplementedError


class EmbeddingClient(Protocol):
    async def recall(self, domain: str, query: str) -> list[GraphCandidate]:
        raise NotImplementedError


class DomainClassifier:
    _rules: tuple[tuple[str, str], ...] = (
        ("supplier", "procurement"),
        ("vendor", "procurement"),
        ("invoice", "finance"),
        ("payment", "finance"),
        ("material", "inventory"),
        ("stock", "inventory"),
        ("plant", "manufacturing"),
    )

    def classify(self, query: str) -> str:
        haystack = query.lower()
        for needle, domain in self._rules:
            if needle in haystack:
                return domain
        return "general"


class SimpleRbacPolicy:
    def __init__(self, write_principals: frozenset[str] = frozenset()):
        self._write_principals = write_principals

    @classmethod
    def from_env(cls) -> "SimpleRbacPolicy":
        import os
        write_principals_str = os.environ.get("SAPIOLA_WRITE_PRINCIPALS", "")
        write_principals = frozenset(p.strip() for p in write_principals_str.split(",") if p.strip())
        return cls(write_principals=write_principals)

    def allowed(self, principal: Principal, domain: str) -> bool:
        return domain in principal.domains or "*" in principal.domains

    def filter_candidates(self, principal: Principal, candidates: Iterable[GraphCandidate]) -> list[GraphCandidate]:
        return [candidate for candidate in candidates if self.allowed(principal, candidate.domain)]

    def filter_fragments(self, principal: Principal, fragments: Iterable[PointerFragment]) -> list[PointerFragment]:
        return [fragment for fragment in fragments if self.allowed(principal, fragment.domain)]

    def can_write(self, principal: Principal) -> bool:
        return principal.name in self._write_principals or "*" in self._write_principals


class RagOrchestrator:
    def __init__(
        self,
        graph_client: GraphClient,
        pointer_client: PointerIndexClient,
        llm_client: LlmClient,
        classifier: DomainClassifier | None = None,
        rbac: SimpleRbacPolicy | None = None,
        embedding_client: EmbeddingClient | None = None,
    ) -> None:
        self._graph_client = graph_client
        self._pointer_client = pointer_client
        self._llm_client = llm_client
        self._classifier = classifier or DomainClassifier()
        self._rbac = rbac or SimpleRbacPolicy()
        self._embedding_client = embedding_client

    async def answer(self, query: str, principal: Principal, enable_embeddings: bool = False) -> RagResult:
        request_id = str(uuid.uuid4())
        structlog.contextvars.bind_contextvars(request_id=request_id, principal_name=principal.name)

        t0 = time.monotonic()
        domain = self._classifier.classify(query)
        
        # Upfront rejection if entirely unauthorized for domain
        if not self._rbac.allowed(principal, domain):
            raise PermissionError(f"principal {principal.name} is not allowed to query domain {domain}")

        candidates_dict: dict[int, GraphCandidate] = {}
        embedding_calls = 0

        # Run graph query and (optionally) embedding recall concurrently
        if enable_embeddings and self._embedding_client is not None:
            graph_cands, embedding_cands = await asyncio.gather(
                self._graph_client.search(domain, query),
                self._embedding_client.recall(domain, query),
            )
            embedding_calls = 1
        else:
            if enable_embeddings and self._embedding_client is None:
                raise ValueError("enable_embeddings=True but no EmbeddingClient provided")
            graph_cands = await self._graph_client.search(domain, query)
            embedding_cands = []

        for c in graph_cands:
            candidates_dict[c.node_id] = c
            
        for c in embedding_cands:
            if c.node_id not in candidates_dict:
                candidates_dict[c.node_id] = c

        all_candidates = list(candidates_dict.values())
        filtered_candidates = self._rbac.filter_candidates(principal, all_candidates)

        # SECURITY EVENT — defense in depth RBAC check dropping candidates
        denied = [c for c in all_candidates if c not in filtered_candidates]
        if denied:
            log.warning(
                "rbac_access_denied",
                domain=domain,
                denied_count=len(denied),
                denied_entity_ids=[c.sap_key for c in denied],
            )
            RAG_RBAC_DENIALS.labels(domain=domain).inc()

        fragments = self._rbac.filter_fragments(principal, self._pointer_client.resolve(filtered_candidates, principal))
        prompt = self._build_prompt(query, domain, filtered_candidates, fragments)
        
        llm_start = time.monotonic()
        try:
            answer_text = await self._llm_client.complete(prompt)
        except LlmUnavailableError:
            log.error("llm_unavailable_degraded_mode", domain=domain)
            RAG_DEGRADED_RESPONSES.inc()
            return RagResult(
                domain=domain,
                answer="",
                prompt=prompt,
                candidates=tuple(filtered_candidates),
                fragments=tuple(fragments),
                embedding_calls=embedding_calls,
            )
            
        llm_latency_ms = (time.monotonic() - llm_start) * 1000

        RAG_LLM_LATENCY.observe(llm_latency_ms)
        RAG_EMBEDDING_FALLBACK_RATE.observe(1 if enable_embeddings else 0)

        log.info(
            "rag_answer_completed",
            domain=domain,
            candidate_count=len(filtered_candidates),
            llm_latency_ms=llm_latency_ms,
            embedding_calls=embedding_calls,
            total_latency_ms=(time.monotonic() - t0) * 1000,
        )

        return RagResult(
            domain=domain,
            answer=answer_text,
            prompt=prompt,
            candidates=tuple(filtered_candidates),
            fragments=tuple(fragments),
            embedding_calls=embedding_calls,
        )

    def _build_prompt(
        self,
        query: str,
        domain: str,
        candidates: list[GraphCandidate],
        fragments: list[PointerFragment],
    ) -> str:
        candidate_lines = [
            f"- {candidate.label}#{candidate.node_id} key={candidate.sap_key} path={' > '.join(candidate.pointer_path)}"
            for candidate in candidates
        ]
        fragment_lines = [
            f"- {fragment.domain}: {' > '.join(fragment.path)} :: {fragment.text}"
            for fragment in fragments
        ]

        return "\n".join(
            [
                f"Domain: {domain}",
                f"Query: {query}",
                "Candidates:",
                *(candidate_lines or ["- none"]),
                "Pointer context:",
                *(fragment_lines or ["- none"]),
            ]
        )

    async def put_vertex(self, principal: Principal, tenant_id: str, node_id: int, properties: dict[str, str]) -> bool:
        if not self._rbac.can_write(principal):
            raise PermissionError(f"Principal {principal.name} is not authorized to write")
        return await self._graph_client.put_vertex(tenant_id, node_id, properties)

    async def put_edge(self, principal: Principal, tenant_id: str, source_id: int, target_id: int, properties: dict[str, str]) -> bool:
        if not self._rbac.can_write(principal):
            raise PermissionError(f"Principal {principal.name} is not authorized to write")
        return await self._graph_client.put_edge(tenant_id, source_id, target_id, properties)