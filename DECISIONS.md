# Architectural Decisions & Agent Context Handoffs

> Shared state and decision records for human developers and AI agents.

---

## 1. Architectural Decisions (ADR)

### ADR-001: Unified Master Mathematical Graph & Sparse GNN Subgraph Routing
- **Date**: 2026-08-25
- **Status**: Accepted
- **Context**: Previous embedding representation partitioned operations into rigid, disjoint slices of a 32-dim vector without structural weight sharing between isomorphic domains (e.g., algebraic distributivity vs boolean distributivity).
- **Decision**: Implemented `MasterMathGraph` (`src/nn/graph.rs`) and `SparseGraphAttentionNetwork` (`src/nn/gnn.rs`). All mathematical primitives and properties live in a unified global heterogeneous graph. A $k$-hop sparse extractor activates only relevant subgraphs during evaluation, keeping cold domains dormant at zero compute.
- **Consequences**: Enables cross-domain representation transfer (e.g., algebraic ring properties transfer to boolean logic and set theory).

---

## 2. Agent Session Handoffs

### Handoff: 2026-08-25 (v0.2.0 Release)
- **Goal**: Implement unified graph neural network architecture with dynamic sparse subgraph activation.
- **Work Completed**:
  - `src/nn/graph.rs`: MasterMathGraph definition with heterogeneous nodes and typed isomorphism edges.
  - `src/nn/gnn.rs`: SparseGraphAttentionNetwork with dynamic Top-k subgraph message passing.
  - `src/nn/embedding.rs`: Graph vectorization pipeline.
  - 30/30 unit tests passing.
- **Key Symbols**:
  - `MasterMathGraph` in `src/nn/graph.rs`
  - `SparseGraphAttentionNetwork` in `src/nn/gnn.rs`
  - `graph_vectorize_proof_state` in `src/nn/embedding.rs`
