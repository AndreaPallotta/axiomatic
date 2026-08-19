use super::embedding::{vectorize_proof_state, EMBEDDING_DIM};
use crate::generator::policy::{NeuralPolicy, PolicyOutput};
use crate::verifier::kernel::{AxiomLibrary, FormalVerifier, ProofState, Tactic};
use rand::Rng;
use serde::{Deserialize, Serialize};

pub const NUM_TACTIC_CLASSES: usize = 26; // 12 LHS rules + 12 RHS rules + Symm + Rfl

/// A 2D Matrix of trainable weights
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Matrix {
    pub rows: usize,
    pub cols: usize,
    pub data: Vec<f64>,
}

impl Matrix {
    pub fn new(rows: usize, cols: usize) -> Self {
        Self {
            rows,
            cols,
            data: vec![0.0; rows * cols],
        }
    }

    /// Xavier / He initialization
    pub fn init_random(rows: usize, cols: usize) -> Self {
        let mut rng = rand::thread_rng();
        let scale = (2.0 / (rows + cols) as f64).sqrt();
        let mut data = Vec::with_capacity(rows * cols);
        for _ in 0..(rows * cols) {
            data.push((rng.gen_range(-1.0..1.0)) * scale);
        }
        Self { rows, cols, data }
    }

    pub fn dot_vec(&self, x: &[f64]) -> Vec<f64> {
        assert_eq!(self.cols, x.len(), "Matrix cols must match vector len");
        let mut out = vec![0.0; self.rows];
        for r in 0..self.rows {
            let mut sum = 0.0;
            for c in 0..self.cols {
                sum += self.data[r * self.cols + c] * x[c];
            }
            out[r] = sum;
        }
        out
    }
}

/// A Two-Headed Deep Neural Network for Policy and Value Prediction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeepProofNetwork {
    // Hidden Layer 1: 32 -> 64
    pub w1: Matrix,
    pub b1: Vec<f64>,

    // Hidden Layer 2: 64 -> 32
    pub w2: Matrix,
    pub b2: Vec<f64>,

    // Policy Head: 32 -> NUM_TACTIC_CLASSES
    pub w_policy: Matrix,
    pub b_policy: Vec<f64>,

    // Value Head: 32 -> 1
    pub w_value: Matrix,
    pub b_value: Vec<f64>,
}

impl DeepProofNetwork {
    pub fn new_random() -> Self {
        Self {
            w1: Matrix::init_random(64, EMBEDDING_DIM),
            b1: vec![0.0; 64],
            w2: Matrix::init_random(32, 64),
            b2: vec![0.0; 32],
            w_policy: Matrix::init_random(NUM_TACTIC_CLASSES, 32),
            b_policy: vec![0.0; NUM_TACTIC_CLASSES],
            w_value: Matrix::init_random(1, 32),
            b_value: vec![0.0; 1],
        }
    }

    /// GELU Activation Function
    pub fn gelu(x: f64) -> f64 {
        0.5 * x * (1.0 + (0.79788456 * (x + 0.044715 * x.powi(3))).tanh())
    }

    /// Forward pass through the network:
    /// Returns (hidden1, hidden2, policy_logits, value)
    pub fn forward(&self, x: &[f64]) -> (Vec<f64>, Vec<f64>, Vec<f64>, f64) {
        // Layer 1: h1 = GELU(W1 * x + b1)
        let z1 = self.w1.dot_vec(x);
        let h1: Vec<f64> = z1
            .iter()
            .zip(&self.b1)
            .map(|(&z, &b)| Self::gelu(z + b))
            .collect();

        // Layer 2: h2 = GELU(W2 * h1 + b2)
        let z2 = self.w2.dot_vec(&h1);
        let h2: Vec<f64> = z2
            .iter()
            .zip(&self.b2)
            .map(|(&z, &b)| Self::gelu(z + b))
            .collect();

        // Policy Head: logits = W_p * h2 + b_p
        let z_p = self.w_policy.dot_vec(&h2);
        let policy_logits: Vec<f64> = z_p
            .iter()
            .zip(&self.b_policy)
            .map(|(&z, &b)| z + b)
            .collect();

        // Value Head: v = tanh(W_v * h2 + b_v)
        let z_v = self.w_value.dot_vec(&h2);
        let value = (z_v[0] + self.b_value[0]).tanh();

        (h1, h2, policy_logits, value)
    }

    /// Softmax over policy logits
    pub fn softmax(logits: &[f64]) -> Vec<f64> {
        let max_l = logits.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let exp_vals: Vec<f64> = logits.iter().map(|&l| (l - max_l).exp()).collect();
        let sum_exp: f64 = exp_vals.iter().sum();
        exp_vals.iter().map(|&e| e / sum_exp.max(1e-12)).collect()
    }

    /// Maps a Tactic to a discrete class index [0..NUM_TACTIC_CLASSES)
    pub fn tactic_to_index(tactic: &Tactic, axioms: &AxiomLibrary) -> usize {
        match tactic {
            Tactic::Reflexivity => 0,
            Tactic::Symmetry => 1,
            Tactic::RewriteLhs(rule_name) => {
                let idx = axioms
                    .rules
                    .iter()
                    .position(|(n, _)| n == rule_name)
                    .unwrap_or(0);
                2 + (idx % 12)
            }
            Tactic::RewriteRhs(rule_name) => {
                let idx = axioms
                    .rules
                    .iter()
                    .position(|(n, _)| n == rule_name)
                    .unwrap_or(0);
                14 + (idx % 12)
            }
            _ => 0,
        }
    }
}

/// Integrates the Deep Neural Network into the MCTS search pipeline
pub struct DeepNeuralPolicy {
    pub model: DeepProofNetwork,
}

impl DeepNeuralPolicy {
    pub fn new(model: DeepProofNetwork) -> Self {
        Self { model }
    }
}

impl NeuralPolicy for DeepNeuralPolicy {
    fn evaluate(&self, state: &ProofState, axioms: &AxiomLibrary) -> PolicyOutput {
        if state.is_solved {
            return PolicyOutput {
                prior_probabilities: vec![(Tactic::Reflexivity, 1.0)],
                state_value: 1.0,
                reasoning_trace: "State is fully solved (Q.E.D.)".to_string(),
            };
        }

        let x = vectorize_proof_state(state);
        let (_, _, logits, value) = self.model.forward(&x);
        let probs = DeepProofNetwork::softmax(&logits);

        let valid_transitions = FormalVerifier::expand_valid_transitions(state, axioms);
        if valid_transitions.is_empty() {
            return PolicyOutput {
                prior_probabilities: vec![],
                state_value: value,
                reasoning_trace: "Dead-end state".to_string(),
            };
        }

        let prev_size = state
            .open_goals
            .first()
            .map(|g| g.equality.lhs.size() + g.equality.rhs.size())
            .unwrap_or(10);

        // Map model probabilities to valid tactics with heuristic simplification guidance
        let mut unnorm_priors = Vec::new();
        for (tactic, next_st) in &valid_transitions {
            let class_idx = DeepProofNetwork::tactic_to_index(tactic, axioms);
            let mut p = probs[class_idx.min(probs.len() - 1)].max(0.01);

            let next_size = next_st
                .open_goals
                .first()
                .map(|g| g.equality.lhs.size() + g.equality.rhs.size())
                .unwrap_or(10);
            if next_st.is_solved {
                p *= 20.0;
            } else if next_size < prev_size {
                p *= 4.0;
            }

            unnorm_priors.push(p);
        }

        let sum_p: f64 = unnorm_priors.iter().sum::<f64>().max(1e-6);
        let mut priors = Vec::new();
        for (i, (tactic, _)) in valid_transitions.into_iter().enumerate() {
            priors.push((tactic, unnorm_priors[i] / sum_p));
        }

        PolicyOutput {
            prior_probabilities: priors,
            state_value: (value + 1.0) * 0.5, // Map [-1.0, 1.0] to [0.0, 1.0]
            reasoning_trace: format!("Deep Neural Net Predicted Value: {:.4}", value),
        }
    }
}

/// Metadata-rich Model Checkpoint container
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelCheckpoint {
    pub model: DeepProofNetwork,
    pub total_epochs_trained: usize,
    pub best_loss: f64,
    pub total_theorems_solved: usize,
    pub timestamp: String,
}

impl ModelCheckpoint {
    pub fn new(model: DeepProofNetwork, epochs: usize, loss: f64, solved: usize) -> Self {
        Self {
            model,
            total_epochs_trained: epochs,
            best_loss: loss,
            total_theorems_solved: solved,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn save_to_file(&self, path: &str) -> std::io::Result<()> {
        if let Some(parent) = std::path::Path::new(path).parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(path, json)
    }

    pub fn load_from_file(path: &str) -> std::io::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        serde_json::from_str(&content)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }

    /// Tries loading best or latest checkpoint from `models/` dir, falling back to embedded baseline or random init
    pub fn try_load_or_init(dir: &str) -> (DeepProofNetwork, usize, f64) {
        let best_path = format!("{}/checkpoint_best.json", dir);
        let latest_path = format!("{}/checkpoint_latest.json", dir);

        if let Ok(ckpt) = Self::load_from_file(&best_path) {
            println!("[INFO] Loaded existing model checkpoint from {}", best_path);
            (ckpt.model, ckpt.total_epochs_trained, ckpt.best_loss)
        } else if let Ok(ckpt) = Self::load_from_file(&latest_path) {
            println!(
                "[INFO] Loaded existing model checkpoint from {}",
                latest_path
            );
            (ckpt.model, ckpt.total_epochs_trained, ckpt.best_loss)
        } else if let Ok(ckpt) = serde_json::from_str::<ModelCheckpoint>(EMBEDDED_BASELINE_MODEL) {
            println!("[INFO] Initialized from embedded pretrained baseline weights");
            (ckpt.model, ckpt.total_epochs_trained, ckpt.best_loss)
        } else {
            (DeepProofNetwork::new_random(), 0, f64::INFINITY)
        }
    }
}

pub const EMBEDDED_BASELINE_MODEL: &str = include_str!("../../models/checkpoint_baseline.json");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_neural_network_forward_pass() {
        let net = DeepProofNetwork::new_random();
        let x = vec![0.5; EMBEDDING_DIM];
        let (h1, h2, logits, value) = net.forward(&x);

        assert_eq!(h1.len(), 64);
        assert_eq!(h2.len(), 32);
        assert_eq!(logits.len(), NUM_TACTIC_CLASSES);
        assert!(value >= -1.0 && value <= 1.0);
    }

    #[test]
    fn test_model_checkpoint_serialization() {
        let net = DeepProofNetwork::new_random();
        let ckpt = ModelCheckpoint::new(net, 10, 2.5, 40);
        let json = serde_json::to_string(&ckpt).expect("Must serialize");
        let deserialized: ModelCheckpoint = serde_json::from_str(&json).expect("Must deserialize");
        assert_eq!(deserialized.total_epochs_trained, 10);
    }

    #[test]
    fn test_embedded_baseline_model_deserialization() {
        let ckpt: Result<ModelCheckpoint, _> = serde_json::from_str(EMBEDDED_BASELINE_MODEL);
        assert!(
            ckpt.is_ok(),
            "Embedded baseline checkpoint must deserialize cleanly"
        );
    }
}
