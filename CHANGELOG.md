# Changelog

All notable changes to the **Axiomatic** project will be documented in this file.

## [v0.4.0] - 2026-09-18

### 3D Vector Knowledge Galaxy, Interactive Equation AST Inspector, & Advanced Mathematical Domains

#### 3D Vector Knowledge Galaxy (`src/memory/vectordb.rs`, `src/visualizer/server.rs`, `src/verifier/trace.rs`)
- Added 3D power iteration PCA projection with Gram-Schmidt deflation to project high-dimensional theorem and search node embeddings into 3D coordinates.
- Added domain categorization classifying theorems across Boolean Logic, Symbolic Calculus, Set Theory, Linear Algebra, and Abstract Algebra.
- Added live 3D visualizer canvas with orbital mouse physics (pitch, yaw, zoom, pan), celestial bloom glow, and depth-sorted Painter's algorithm.
- Added green glowing arcs tracing proof search trajectories in 3D.
- Integrated 3D Vector Galaxy into standalone static HTML proof showcases with client-side PCA projection.

#### Interactive Equation AST Inspector (`src/visualizer/server.rs`, `src/verifier/trace.rs`)
- Added recursive descent AST parser for arithmetic, logical, and calculus expressions with precedence and associativity.
- Rendered hierarchical SVG syntax tree with circular operator nodes and rounded rectangle leaf nodes.
- Added glowing rewrite highlights illuminating modified subtrees based on applied tactic rewrites.
- Synchronized interactive selection bidirectionally across 2D MCTS tree, 3D Vector Galaxy, and Equation AST Inspector.

#### Advanced Mathematical Domains & Search Engine 2.0
- Added curriculum domains: information theory, abstract group theory, integral transforms, category theory, topology, exterior calculus, combinatorics, and control theory.
- Added active counterexample pruning to filter unprovable branches during MCTS expansion.
- Extended Peano structural induction engine to support higher-order hypotheses and multi-variable recurrence.

## [v0.3.1] - 2026-09-08

### Static Proof Trace & GitHub Pages Deployment Pipeline

#### Static Trace Exporter (`src/verifier/trace.rs`, `src/main.rs`)
- Added `StaticProofTrace` capturing full MCTS exploration graphs, proven root-to-leaf paths, tactics, and multi-format proof scripts (Lean 4, Coq, AMS-LaTeX).
- Added `TraceExporter::export_json` and `TraceExporter::export_standalone_html` for zero-backend static hosting.
- Added `axiomatic export-trace` CLI command with configurable budgets, domain auto-detection, and artifact directory output.

#### GitHub Pages Deployment Workflow (`.github/workflows/pages.yml`)
- Automated CI showcase builder generating arithmetic and boolean interactive showcases.
- Integrated official Lean 4 toolchain via `elan` to certify emitted proof scripts in CI.
- Added automated `dist/CNAME` emission for custom subdomain `axiomatic.andreapallotta.dev`.

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
