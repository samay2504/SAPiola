# Phase 5: Production Baselines

This directory contains the benchmarking harness for comparing **SAPiola Poly-Lsm Engine** against industry standards **Neo4j** and **Memgraph**.

## Architectural Caveats: Embedded vs Network

When interpreting the benchmark numbers, it is **critical** to understand that this is an apples-to-oranges architectural comparison:

| Metric | SAPiola Poly-Lsm (Our Engine) | Neo4j / Memgraph |
|--------|-------------------------------|------------------|
| **Deployment Model** | Embedded / In-Process | Networked Daemon |
| **Protocol** | Zero-copy direct memory access | Bolt Protocol over TCP |
| **Parsing Overhead** | None (Raw API calls) | Cypher AST parsing & Planning |
| **Disk Engine** | Pure Rust LSM (Fjall) | Custom B-Tree / In-Memory with WAL |

### Why This Matters

1. **Network Overhead**: Neo4j and Memgraph incur TCP stack latency and serialization/deserialization costs for every batch of writes and reads. SAPiola's engine is embedded directly into the streaming gateway (or routed via a lightweight internal gRPC), meaning writes hit the disk engine almost immediately.
2. **Query Parsing**: `bench.py` sends raw Cypher strings to Neo4j/Memgraph. Even with query caching, parameterizing strings takes CPU cycles. Our benchmark for Poly-LSM writes directly to the `put_edge()` Rust API.

## Baseline Results (Skewed Workload)

These are the real results executed against single-node Docker containers via the official Python driver.

*   **Neo4j (Network)**: ~11,428 edge writes/sec | Reads: 13.13 ms/query
*   **Memgraph (Network)**: ~40,970 edge writes/sec | Reads: 1.90 ms/query
*   **SAPiola Poly-Lsm (Embedded)**: **~52,157 edge writes/sec** | Reads: 0.221 ms/query (p99)

## Running the Baselines

To run the baselines locally:

1. Ensure Docker is running.
2. Spin up the databases:
   ```bash
   docker-compose up -d
   ```
3. Install the python driver:
   ```bash
   pip install neo4j
   ```
4. Run the benchmark script:
   ```bash
   python bench.py
   ```
