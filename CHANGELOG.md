# Changelog

All notable changes to the **Axiomatic** project will be documented in this file.

## [v0.3.0] - 2026-09-07

### Lean 4 Formal Verification & System Improvements

#### Lean 4 Formal Kernel Integration (`src/verifier/lean.rs`, `src/verifier/lean_runner.rs`)
- Connected neurosymbolic MCTS proof discovery to the official Lean 4 formal proof assistant kernel.
- Proofs discovered by MCTS are translated into Lean 4 source scripts, saved to `proofs/`, and verified by the `lean` compiler.
- Replaced stubs with core prelude theorem mappings (`Nat` and `Bool`) and single-divergence `conv` AST navigation.
- Preserved initial conjecture equality across proof states and structural induction branches.

#### Proof Minimization & Dead-Tactic Pruning (`src/verifier/kernel.rs`, `src/search/mcts.rs`)
- Added two-pass proof minimization (`ProofState::minimize`): cancels consecutive symmetry pairs and removes redundant tactic steps via verifier replay simulation.
- Automated proof minimization in `MctsEngine::run_search`.

#### Standalone Lean 4 Workspace Scaffolding (`src/verifier/lean_runner.rs`)
- Added `ensure_lean_workspace` generating `proofs/lean-toolchain` (pinned to Lean 4.33.1) and `proofs/lakefile.lean` for direct VS Code and Lake integration.

#### Multi-Domain Propositional Logic Translation (`src/verifier/lean.rs`, `src/main.rs`)
- Added AST domain detection (`is_boolean_equality`) mapping boolean expressions to Lean 4 `Bool` primitives, library lemmas, and `try decide` closure.

#### Lean 4 Formal Gate for Compounding Memory (`src/memory/database.rs`, `src/main.rs`)
- Added `LemmaDatabase::record_and_certify` requiring formal Lean 4 kernel certification before admitting discovered lemmas into the active axiom database.

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
