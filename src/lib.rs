#![allow(unused_variables, dead_code, clippy::all)]

pub mod generator;
pub mod memory;
pub mod nn;
pub mod search;
pub mod theory;
pub mod verifier;
pub mod visualizer;

pub use generator::{NeuralPolicy, PolicyOutput, SymbolicNeuralPolicy};
pub use memory::{
    DistanceMetric, LemmaDatabase, MathematicalVectorDB, ScoredResult, TheoremPayload,
    VectorRecord, VerifiedTheorem,
};
pub use nn::{
    collect_parallel_self_play_trajectories, train_continuous_session, train_self_play_cycle,
    vectorize_proof_state, AdamOptimizer, DeepNeuralPolicy, DeepProofNetwork, ModelCheckpoint,
    ReplayBuffer, RewardConfig, RewardEngine, RollbackEvent, SupervisorAction,
    TrainingHealthStatus, TrainingMetrics, TrainingSupervisor,
};
pub use search::{MctsEngine, MctsNode, SearchEvent, SearchGraphSnapshot};
pub use theory::{CurriculumController, DifficultyLevel, InventedTheorem, TheoryInventor};
pub use verifier::{
    export_to_lean4, parse_conjecture, term_to_lean, AxiomLibrary, Equality, FormalVerifier, Goal,
    InductionEngine, Lean4Validator, LeanValidationResult, MultiFormatExporter, ProofState, Tactic,
    Term,
};
pub use visualizer::{start_visualizer_server, EngineController};
