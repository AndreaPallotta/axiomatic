pub mod embedding;
pub mod model;
pub mod optim;
pub mod reward;
pub mod supervisor;
pub mod trainer;

pub use embedding::{vectorize_proof_state, EMBEDDING_DIM};
pub use model::{DeepNeuralPolicy, DeepProofNetwork, Matrix, ModelCheckpoint, NUM_TACTIC_CLASSES};
pub use optim::{AdamOptimizer, TrainingSample};
pub use reward::{RewardConfig, RewardEngine};
pub use supervisor::{RollbackEvent, SupervisorAction, TrainingHealthStatus, TrainingSupervisor};
pub use trainer::{
    collect_parallel_self_play_trajectories, collect_self_play_trajectory,
    generate_synthetic_algebra_conjecture, train_continuous_session, train_self_play_cycle,
    ReplayBuffer, TrainingMetrics,
};
