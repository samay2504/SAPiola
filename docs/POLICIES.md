# SAPiola Security & Data Governance Policies

This document outlines the operational security, multi-tenant isolation, audit logging, and data licensing posture enforced by the SAPiola Zero-ETL platform.

---

## 1. Multi-Tenant Data Isolation

SAPiola guarantees strict mathematical key-space isolation across all tenant workloads:

1. **Storage Layer Key Prefixes (`TenantScopedKey`)**:
   - In `poly-lsm-core`, all keys written to the Fjall LSM tree column families (`vprop_val`, `eprop_val`, `topology`, `pointer_idx`) are encoded with a mandatory `tenant_id` header:
     $$\text{EncodedKey} = \text{TenantID} \parallel \text{Delimiter} \parallel \text{EntityKey}$$
   - Direct key scans and range iterations are strictly bounded by tenant namespace boundaries. Cross-tenant data leakage is physically impossible at the LSM storage engine level.

2. **gRPC Context Metadata**:
   - All gRPC requests (`GraphService`, `StorageService`) require a `tenant-id` header passed in gRPC metadata. Missing or invalid metadata defaults to fail-closed isolation.

---

## 2. Write Authorization & RBAC Policy

1. **Write Role Enforcement (`SimpleRbacPolicy`)**:
   - All write operations (`put_vertex`, `put_edge`, write endpoints in `sap-ai-gateway`) require the requesting principal to be present in the `SAPIOLA_WRITE_PRINCIPALS` environment whitelist.
   - Unauthorized write requests trigger an immediate `403 Forbidden` error with structured audit logs.

2. **Domain Scoping**:
   - Principals are bound to specific business domains (e.g., `inventory`, `procurement`, `finance`).
   - The RAG Orchestrator performs upfront domain entitlement checks before dispatching queries to the structural index or embedding index.

---

## 3. Audit Logging & Telemetry

- **Structured JSON Logging**: Every request is tagged with a unique `request_id` and `principal_name` via `structlog.contextvars`.
- **Telemetry Metrics**: `PolyLsmEngine` tracks compaction frequency, write amplification, degree counter increments, and migration task queue depth.
- **Fail-Fast Observability**: Circuit breaker state changes (`aiobreaker`) and gRPC health status updates emit high-priority structured audit events.

---

## 4. Dataset Licensing Policy (SALT Dataset)

> [!IMPORTANT]
> The SALT dataset used in test fixtures (`salt/` directory and `tools/salt_importer`) is subject to the **Creative Commons Attribution-NonCommercial-ShareAlike 4.0 International License (CC-BY-NC-SA 4.0)**.

### License Requirements & Constraints:
- **Non-Commercial Use Only**: The SALT dataset and test fixtures derived from it MUST NOT be used for commercial training, commercial benchmarking, or revenue-generating services.
- **Attribution**: Any derivative test fixtures or evaluation benchmarks using SALT data must maintain attribution to the original SALT project creators.
- **Production Isolation**: Production deployments of SAPiola running against live SAP HANA systems MUST NOT include or bundle the `salt/` dataset files.
