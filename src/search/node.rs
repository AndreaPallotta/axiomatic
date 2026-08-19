use crate::verifier::kernel::{ProofState, Tactic};
use serde::{Deserialize, Serialize};

/// A node in the Monte Carlo Proof Search Tree
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MctsNode {
    pub id: usize,
    pub parent_id: Option<usize>,
    pub state: ProofState,
    pub applied_tactic: Option<Tactic>,
    pub visit_count: usize,     // N(s)
    pub total_value: f64,       // W(s)
    pub mean_value: f64,        // Q(s) = W(s) / N(s)
    pub policy_prior: f64,      // P(s, a) from neural policy network
    pub children_ids: Vec<usize>,
    pub is_expanded: bool,
    pub is_terminal: bool,
    pub is_proven: bool,
    pub depth: usize,
}

impl MctsNode {
    pub fn new_root(state: ProofState) -> Self {
        let is_proven = state.is_solved;
        Self {
            id: 0,
            parent_id: None,
            state,
            applied_tactic: None,
            visit_count: 0,
            total_value: 0.0,
            mean_value: 0.0,
            policy_prior: 1.0,
            children_ids: Vec::new(),
            is_expanded: false,
            is_terminal: is_proven,
            is_proven,
            depth: 0,
        }
    }

    pub fn new_child(
        id: usize,
        parent_id: usize,
        state: ProofState,
        applied_tactic: Tactic,
        policy_prior: f64,
        depth: usize,
    ) -> Self {
        let is_proven = state.is_solved;
        Self {
            id,
            parent_id: Some(parent_id),
            state,
            applied_tactic: Some(applied_tactic),
            visit_count: 0,
            total_value: 0.0,
            mean_value: 0.0,
            policy_prior,
            children_ids: Vec::new(),
            is_expanded: false,
            is_terminal: is_proven,
            is_proven,
            depth,
        }
    }

    /// Computes PUCT (Predictor Upper Confidence bounds for Trees) score:
    /// Score = Q(s, a) + c_puct * P(s, a) * sqrt(N_parent) / (1 + N(child))
    pub fn puct_score(&self, parent_visits: usize, c_puct: f64) -> f64 {
        let exploration = c_puct * self.policy_prior * (parent_visits as f64).sqrt()
            / (1.0 + self.visit_count as f64);
        self.mean_value + exploration
    }

    /// Updates node statistics during backpropagation
    pub fn update(&mut self, value: f64) {
        self.visit_count += 1;
        self.total_value += value;
        self.mean_value = self.total_value / self.visit_count as f64;
    }
}
