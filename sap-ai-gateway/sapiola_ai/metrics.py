from prometheus_client import Histogram, Counter

RAG_LLM_LATENCY = Histogram("rag_llm_latency_ms", "LLM call latency", buckets=[50, 100, 250, 500, 1000, 2500, 5000])
RAG_DEGRADED_RESPONSES = Counter("rag_degraded_responses_total", "Responses served without LLM due to breaker/failure")
RAG_RBAC_DENIALS = Counter("rag_rbac_denials_total", "Candidates stripped by RBAC filter", ["domain"])
RAG_EMBEDDING_FALLBACK_RATE = Histogram("rag_embedding_enabled_ratio", "Fraction of requests using embedding widening")
