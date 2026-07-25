# PATENT NOTICE & INTELLECTUAL PROPERTY CLAIMS

**Inventor & Patent Owner:** Samay Mehar (Identifier: 487266569521)  
**Platform:** SAPiola Zero-ETL Graph & AI Platform  
**Notice Type:** Patent Pending / Proprietary Trade Secret Notice  

---

## Abstract of Inventions

This patent document establishes the technical claims and intellectual property rights owned by **Samay Mehar (487266569521)** over the key software architectures, mathematical methods, and hardware-efficient data structures comprising SAPiola.

---

## Core Patentable Claims & Innovations

### Claim 1: Zero-Code-Change Dynamic Schema Discovery Engine
*A computer-implemented method for automatically reconstructing structural knowledge graphs from relational enterprise databases without predefined schemas or hardcoded software alterations.*
- **Key Innovation:** System queries low-level data dictionary metadata (`SYS.TABLE_COLUMNS`), executes dynamic uniqueness sampling to discover primary key candidate combinations, and performs set-overlap intersection scoring across sample arrays to calculate an empirical foreign key confidence coefficient:
$$\text{Confidence}(S.c \to T.pk) = \frac{|V_{sample}(S.c) \cap V_{sample}(T.pk)|}{|V_{sample}(S.c)|}$$
- **Technical Advantage:** Operates seamlessly on enterprise systems (such as SAP ERP) where referential constraints are enforced strictly in the application layer and absent from system tables (`SYS.REFERENTIAL_CONSTRAINTS`).

### Claim 2: Embedded Dual-Index LSM Graph Architecture
*A hardware-optimized database engine providing concurrent topological graph traversal and relational key-value storage within a unified Log-Structured Merge (LSM) tree.*
- **Key Innovation:** Structural adjacency lists (`source_id -> target_id`) and high-cardinality relational column dictionaries are stored in distinct keyspaces of an embedded Fjall LSM tree (`poly-lsm-core`). Structural queries execute via zero-copy slice reads, while heavy attribute hydration occurs lazily via pointer indices.
- **Technical Advantage:** Reduces RAM footprint by over 80% compared to traditional property graphs while preserving $O(1)$ memory-mapped disk pointer resolution.

### Claim 3: Agentic Model Context Protocol (MCP) RAG Orchestrator with Circuit-Breaker
*An AI integration framework providing secure, real-time context retrieval for Large Language Models via MCP tools with dynamic RBAC tenant isolation.*
- **Key Innovation:** The orchestrator translates natural language queries into candidate graph traversals, validates tenant principal security bounds before storage hydration, and executes tool trace verification to eliminate LLM hallucinations.

---

## Patent Ownership & Legal Notice

All rights, titles, and interests in and to these inventions, patents, patent applications, trade secrets, software designs, and associated documentation are strictly reserved by **Samay Mehar (487266569521)**. 

Any unauthorized implementation, derivation, commercial sale, or sub-licensing of these patent claims constitutes willful infringement punishable under international patent treaties and laws.
