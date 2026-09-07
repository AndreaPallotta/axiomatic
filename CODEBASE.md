# Codebase Map
Generated: 2026-09-07 19:42:57
Commit: 611c50c feat(verifier): add proof minimization, lean workspace scaffolding, boolean domain, and memory gate

This file is a compact index of the codebase for AI agents to understand project structure without full recursive file scans.

## External & Linked Dependencies
See [DEPS.md](file:///c:/Users/andre/OneDrive/Desktop/projects/agent-devkit/DEPS.md) for runtime environment and linked package maps.

## File Index
- **.github/workflows/ci.yml** (43 lines)
- **.github/workflows/release.yml** (125 lines)
- **AGENT.md** (33 lines)
- **CHANGELOG.md** (33 lines)
- **CLAUDE.md** (30 lines)
- **CODEBASE.md** (192 lines)
- **DECISIONS.md** (31 lines)
- **DEPS.md** (30 lines)
- **docker-compose.yml** (12 lines)
- **GEMINI.md** (34 lines)
- **models/checkpoint_baseline.json** (5123 lines)
- **models/checkpoint_latest.json** (5123 lines)
- **README.md** (124 lines)
- **scripts/capture_docs.js** (95 lines)
- **scripts/hf_sync.py** (60 lines)
- **src/generator/mod.rs** (6 lines)
- **src/generator/policy.rs** (133 lines)
- **src/generator/prompt.rs** (22 lines)
- **src/lib.rs** (30 lines)
- **src/main.rs** (410 lines)
- **src/memory/database.rs** (130 lines)
- **src/memory/mod.rs** (8 lines)
- **src/memory/vectordb.rs** (177 lines)
- **src/nn/embedding.rs** (154 lines)
- **src/nn/gnn.rs** (150 lines)
- **src/nn/graph.rs** (345 lines)
- **src/nn/mod.rs** (22 lines)
- **src/nn/model.rs** (327 lines)
- **src/nn/optim.rs** (383 lines)
- **src/nn/reward.rs** (121 lines)
- **src/nn/supervisor.rs** (285 lines)
- **src/nn/trainer.rs** (436 lines)
- **src/search/mcts.rs** (311 lines)
- **src/search/mod.rs** (6 lines)
- **src/search/node.rs** (83 lines)
- **src/theory/curriculum.rs** (255 lines)
- **src/theory/decomposer.rs** (142 lines)
- **src/theory/inventor.rs** (709 lines)
- **src/theory/mod.rs** (8 lines)
- **src/verifier/exporter.rs** (136 lines)
- **src/verifier/fol.rs** (277 lines)
- **src/verifier/induction.rs** (94 lines)
- **src/verifier/kernel.rs** (876 lines)
- **src/verifier/lean.rs** (339 lines)
- **src/verifier/lean_runner.rs** (249 lines)
- **src/verifier/mod.rs** (16 lines)
- **src/verifier/parser.rs** (287 lines)
- **src/visualizer/mod.rs** (4 lines)
- **src/visualizer/server.rs** (2198 lines)

## Key Symbol & Interface Index

### scripts/hf_sync.py
- def upload_model(repo_id: str, checkpoint_path: str = "models/checkpoint_latest.json"): [L17]
- def download_model(repo_id: str, dest_dir: str = "models"): [L28]
- def main(): [L39]

### src/generator/policy.rs
- pub struct PolicyOutput { [L8]
- pub trait NeuralPolicy: Send + Sync { [L15]
- pub struct SymbolicNeuralPolicy; [L21]

### src/generator/prompt.rs
- pub fn format_proof_prompt(state: &ProofState) -> String { [L4]

### src/memory/database.rs
- pub struct VerifiedTheorem { [L8]
- pub struct LemmaDatabase { [L20]

### src/memory/vectordb.rs
- pub enum DistanceMetric { [L5]
- pub struct TheoremPayload { [L13]
- pub struct VectorRecord { [L23]
- pub struct ScoredResult { [L31]
- pub struct MathematicalVectorDB { [L38]
- pub fn cosine_similarity(a: &[f64], b: &[f64]) -> f64 { [L116]
- pub fn dot_product(a: &[f64], b: &[f64]) -> f64 { [L127]
- pub fn euclidean_distance(a: &[f64], b: &[f64]) -> f64 { [L131]

### src/nn/embedding.rs
- pub fn graph_vectorize_proof_state(gnn: &SparseGraphAttentionNetwork, state: &ProofState) -> Vec<f64> { [L9]
- pub fn vectorize_proof_state(state: &ProofState) -> Vec<f64> { [L27]

### src/nn/gnn.rs
- pub struct SparseGraphAttentionNetwork { [L9]

### src/nn/graph.rs
- pub enum NodeType { [L8]
- pub enum EdgeType { [L18]
- pub struct MathGraphNode { [L28]
- pub struct MathGraphEdge { [L37]
- pub struct MasterMathGraph { [L46]

### src/nn/model.rs
- pub struct Matrix { [L11]
- pub struct DeepProofNetwork { [L53]
- pub struct DeepNeuralPolicy { [L159]
- pub struct ModelCheckpoint { [L234]

### src/nn/optim.rs
- pub struct TrainingSample { [L5]
- pub struct AdamOptimizer { [L13]

### src/nn/reward.rs
- pub struct RewardConfig { [L6]
- pub struct RewardEngine { [L27]

### src/nn/supervisor.rs
- pub enum TrainingHealthStatus { [L8]
- pub struct RollbackEvent { [L18]
- pub enum SupervisorAction { [L29]
- pub struct TrainingSupervisor { [L48]

### src/nn/trainer.rs
- pub struct TrainingMetrics { [L12]
- pub struct ReplayBuffer { [L24]
- pub fn generate_synthetic_algebra_conjecture(difficulty: usize) -> Equality { [L46]
- pub fn collect_self_play_trajectory( [L98]
- pub fn convert_mcts_tree_to_training_samples( [L160]
- pub fn collect_parallel_self_play_trajectories( [L210]
- pub fn train_self_play_cycle( [L237]
- pub fn train_continuous_session( [L286]

### src/search/mcts.rs
- pub enum SearchEvent { [L9]
- pub struct SearchGraphSnapshot { [L31]
- pub struct MctsEngine { [L38]

### src/search/node.rs
- pub struct MctsNode { [L6]

### src/theory/curriculum.rs
- pub enum DifficultyLevel { [L6]
- pub struct CurriculumController { [L28]

### src/theory/decomposer.rs
- pub struct GoalDecomposer; [L5]

### src/theory/inventor.rs
- pub struct InventedTheorem { [L8]
- pub struct TheoryInventor; [L18]

### src/verifier/exporter.rs
- pub struct MultiFormatExporter; [L5]

### src/verifier/fol.rs
- pub enum Term { [L7]
- pub struct Equality { [L133]
- pub fn unify(t1: &Term, t2: &Term) -> Option<HashMap<String, Term>> { [L165]
- pub fn apply_rewrite(target: &Term, rule: &Equality) -> Vec<Term> { [L213]

### src/verifier/induction.rs
- pub struct InductionEngine; [L5]

### src/verifier/kernel.rs
- pub enum Tactic { [L7]
- pub struct Goal { [L31]
- pub struct ProofState { [L44]
- pub enum MathDomain { [L91]
- pub struct AxiomLibrary { [L113]
- pub struct FormalVerifier; [L574]

### src/verifier/lean.rs
- pub fn term_to_lean(term: &Term) -> String { [L5]
- pub fn term_to_lean_with_domain(term: &Term, is_bool: bool) -> String { [L9]
- pub fn is_boolean_equality(eq: &Equality) -> bool { [L58]
- pub fn map_rule_to_lean(rule: &str) -> String { [L78]
- pub fn export_to_lean4(theorem_name: &str, final_state: &ProofState) -> String { [L140]
- pub fn export_equality_to_lean4( [L149]

### src/verifier/lean_runner.rs
- pub enum LeanValidationResult { [L8]
- pub struct Lean4Validator; [L22]

### src/verifier/parser.rs
- pub fn parse_conjecture(input: &str) -> Result<Equality, String> { [L244]

### src/visualizer/server.rs
- pub struct TargetProbeResult { [L37]
- pub struct EngineController { [L48]
- pub type SharedState = Arc<RwLock<EngineController>>; [L416]
