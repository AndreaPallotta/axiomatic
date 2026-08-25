# Changelog

All notable changes to the **Axiomatic** project will be documented in this file.

## [v0.2.0] - 2026-08-25

### Unified Mathematical Knowledge Graph & Sparse GNN Routing

#### Unified Master Graph (`src/nn/graph.rs`)
- Introduced `MasterMathGraph`: a single unified heterogeneous graph connecting mathematical operators ($+, *, -, D, \land, \lor, \neg, \cap, \cup, S$), constants ($0, 1, U$), universal axiom property hubs (Commutativity, Associativity, Distributivity, Identity, Annihilation, Inversion, DeMorgan, Linearity), and tactic primitives.
- Connected isomorphic algebraic, boolean, and set-theoretic operations to shared property hubs via typed edges (`EdgeType::Isomorphism`, `EdgeType::Dual`, `EdgeType::SubOperation`).
- Added `active_subgraph_nodes` to dynamically extract active seed nodes and expand via $k$-hop BFS over only relevant mathematical pathways.

#### Dynamic Sparse Graph Attention Network (`src/nn/gnn.rs`)
- Implemented `SparseGraphAttentionNetwork`: runs message passing strictly over active mathematical subgraphs, leaving dormant domains at zero compute.
- Added attention-gated node scoring with softmax normalization over the active cluster.
- Implemented attentive graph pooling for invariant embedding generation.

#### Graph Embedding Pipeline (`src/nn/embedding.rs`)
- Added `graph_vectorize_proof_state` combining GNN graph pooling with structural invariant metrics.

#### Agent-DevKit Integration
- Integrated `agent-devkit` standard guidelines (`CLAUDE.md`, `AGENT.md`, `GEMINI.md`, `DECISIONS.md`, `DEPS.md`, `CODEBASE.md`).

## [v0.1.0] - 2026-08-19

### Initial Release
- Multi-domain formal logic kernel (Abstract Algebra, Boolean Propositional Logic, Symbolic Calculus, Set Theory, Peano Structural Induction).
- Deep policy-value Monte Carlo Tree Search (MCTS) with self-play reinforcement learning.
- Vector premise retrieval database with 128-dim embedding.
- Formal proof certification exporters (Lean 4, Coq, LaTeX).
- Real-time HTML5 Canvas interactive command center and WebSocket visualizer.
