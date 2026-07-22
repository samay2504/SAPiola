# SAPiola MCP Tools Guide

This guide provides a comprehensive technical reference for all Model Context Protocol (MCP) tools exposed by `sap-mcp-server`. It is designed for both human operators and autonomous LLM agents.

---

## Tool Overview

| Tool Name | Operation Type | Target Service | Typical Latency Bound | Latency / Scale Characteristic |
|---|---|---|---|---|
| `get_vertex` | Read (Point Lookup) | `StorageService` | < 5ms | $O(1)$ key lookup in Fjall LSM tree |
| `get_neighbors` | Read (Adjacency) | `StorageService` | < 10ms | $O(D)$ where $D$ is vertex degree |
| `expand_neighborhood` | Read (Bounded Traversal) | `StorageService` | < 30ms | Bounded by `max_nodes_visited` circuit breaker |
| `list_schema` | Read (Introspection) | `GraphService` | < 5ms | $O(S)$ where $S$ is schema label count |
| `put_vertex` | Write (Authorized) | `sap-ai-gateway` | < 20ms | RBAC-checked, forwards to Fjall LSM tree |
| `put_edge` | Write (Authorized) | `sap-ai-gateway` | < 20ms | RBAC-checked, writes topology delta |
| `ask_sap` | RAG Query (Natural Lang) | `sap-ai-gateway` | < 800ms | End-to-end RAG pipeline + LLM synthesis |
| `run_graph_query` | Structural Query (Cypher) | `GraphService` | < 50ms | Cypher-subset Pest parser + LSM traversal |

---

## 1. `get_vertex`

**Description:** Retrieves property attributes for a specific vertex node ID within a tenant.

### Parameters
```json
{
  "node_id": 1000,
  "tenant_id": "test_tenant"
}
```

### Return Schema
```json
{
  "node_id": 1000,
  "properties": {
    "name": "Hub Node",
    "status": "ACTIVE"
  }
}
```

---

## 2. `get_neighbors`

**Description:** Retrieves inbound and outbound neighbor node IDs for a specified vertex.

### Parameters
```json
{
  "node_id": 1000,
  "tenant_id": "test_tenant"
}
```

### Return Schema
```json
{
  "out_edges": [1001, 1002, 1003],
  "in_edges": []
}
```

---

## 3. `expand_neighborhood`

**Description:** Executes a bounded Breadth-First Search (BFS) graph traversal starting from an anchor node. Guarantees safety on high-degree hub vertices via `max_nodes_visited`.

### Parameters
```json
{
  "anchor": 1000,
  "tenant_id": "test_tenant",
  "max_depth": 2,
  "max_nodes_visited": 15,
  "direction": "outbound"
}
```
*Directions:* `"outbound"`, `"inbound"`, or `"both"`.

### Performance & Safety Notes
> [!IMPORTANT]
> The latency of `expand_neighborhood` scales directly with `max_nodes_visited`, not search depth. Keep `max_nodes_visited` $\le 50$ for interactive agent queries.

---

## 4. `list_schema`

**Description:** Lists all available node and edge labels declared in the active DSL mapping catalog.

### Parameters
```json
{
  "tenant_id": "test_tenant"
}
```

### Return Schema
```json
{
  "node_labels": ["salesdocument", "material", "customer"],
  "edge_labels": ["CONTAINS", "ORDERED_BY"]
}
```

---

## 5. `put_vertex`

**Description:** Writes or updates a vertex and its properties. Requires write authorization (`SAPIOLA_WRITE_PRINCIPALS`).

### Parameters
```json
{
  "tenant_id": "test_tenant",
  "principal": "alice",
  "node_id": 1000,
  "properties": {
    "name": "Updated Hub",
    "type": "PLANT"
  }
}
```

---

## 6. `put_edge`

**Description:** Creates a directed edge relationship between two vertices. Requires write authorization.

### Parameters
```json
{
  "tenant_id": "test_tenant",
  "principal": "alice",
  "source_id": 1000,
  "target_id": 1001,
  "properties": {
    "relationship": "SUPPLIES"
  }
}
```

---

## 7. `ask_sap`

**Description:** Submits a natural language query to the SAPiola RAG Orchestrator (`sap-ai-gateway`). Performs domain classification, RBAC filtering, graph candidate retrieval, relational pointer hydration, and LLM answer generation.

### Parameters
```json
{
  "question": "What is the status of material 1000?",
  "tenant_id": "inventory_user"
}
```

---

## 8. `run_graph_query`

**Description:** Executes a Cypher-subset query against the structural graph index.

### Parameters
```json
{
  "cypher": "MATCH (n) RETURN n",
  "tenant_id": "test_tenant"
}
```
