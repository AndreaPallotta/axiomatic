use crate::verifier::kernel::{AxiomLibrary, FormalVerifier, ProofState, Tactic};
use serde::{Deserialize, Serialize};

/// The output of the Neural Policy Network for a given proof state:
/// - Prior Probabilities P(s, a) over valid tactics
/// - Value Evaluation V(s) estimating the probability of finding a proof
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyOutput {
    pub prior_probabilities: Vec<(Tactic, f64)>, // (Tactic, Softmax Probability)
    pub state_value: f64,                        // V(s) in [0.0, 1.0] (1.0 = Solved)
    pub reasoning_trace: String,
}

/// Abstract Trait for any Neural Model or Policy Network
pub trait NeuralPolicy: Send + Sync {
    fn evaluate(&self, state: &ProofState, axioms: &AxiomLibrary) -> PolicyOutput;
}

/// A High-Speed Neurosymbolic Policy Model with Tree-Complexity Heuristics & Softmax
#[derive(Debug, Clone, Default)]
pub struct SymbolicNeuralPolicy;

impl SymbolicNeuralPolicy {
    pub fn new() -> Self {
        Self
    }

    /// Computes the syntactic complexity / tree-size of an expression
    fn term_complexity(term: &crate::verifier::fol::Term) -> usize {
        match term {
            crate::verifier::fol::Term::Var(_) => 1,
            crate::verifier::fol::Term::Const(_) => 1,
            crate::verifier::fol::Term::Func(_, args) => {
                1 + args.iter().map(Self::term_complexity).sum::<usize>()
            }
        }
    }
}

impl NeuralPolicy for SymbolicNeuralPolicy {
    fn evaluate(&self, state: &ProofState, axioms: &AxiomLibrary) -> PolicyOutput {
        if state.is_solved {
            return PolicyOutput {
                prior_probabilities: vec![(Tactic::Reflexivity, 1.0)],
                state_value: 1.0,
                reasoning_trace: "State is fully solved (Q.E.D.)".to_string(),
            };
        }

        if state.open_goals.is_empty() {
            return PolicyOutput {
                prior_probabilities: vec![],
                state_value: 1.0,
                reasoning_trace: "No open goals remaining".to_string(),
            };
        }

        let current_goal = &state.open_goals[0];
        let lhs_len = Self::term_complexity(&current_goal.equality.lhs);
        let rhs_len = Self::term_complexity(&current_goal.equality.rhs);
        let total_complexity = lhs_len + rhs_len;

        // Base value estimate: decays exponentially with unsimplified tree complexity
        let base_value = (1.0 / (1.0 + 0.1 * total_complexity as f64 + 0.05 * state.depth as f64))
            .clamp(0.01, 0.99);

        // Find all formally valid transitions
        let valid_transitions = FormalVerifier::expand_valid_transitions(state, axioms);
        if valid_transitions.is_empty() {
            return PolicyOutput {
                prior_probabilities: vec![],
                state_value: 0.0,
                reasoning_trace: "Dead-end state: no valid transitions".to_string(),
            };
        }

        // Score each candidate tactic
        let mut scores = Vec::new();
        for (tactic, next_state) in &valid_transitions {
            let mut score: f64 = 1.0;

            // 1. Reflexivity gets astronomical priority
            if matches!(tactic, Tactic::Reflexivity) || next_state.is_solved {
                score += 100.0;
            }

            // 2. Simplification reward (reward tactics that reduce term complexity)
            if let Some(next_goal) = next_state.open_goals.first() {
                let next_complexity = Self::term_complexity(&next_goal.equality.lhs)
                    + Self::term_complexity(&next_goal.equality.rhs);
                if next_complexity < total_complexity {
                    score += 5.0 * (total_complexity - next_complexity) as f64;
                } else if next_complexity > total_complexity + 4 {
                    score *= 0.3; // Penalize excessive expansion
                }
            }

            // 3. Identity and Zero rules get natural boost
            if let Tactic::RewriteLhs(r) | Tactic::RewriteRhs(r) = tactic {
                if r.contains("zero") || r.contains("one") || r.contains("inv") {
                    score += 3.0;
                }
            }

            scores.push((*tactic == Tactic::Reflexivity, score));
        }

        // Apply Softmax normalization over candidate scores
        let max_score = scores
            .iter()
            .map(|(_, s)| *s)
            .fold(f64::NEG_INFINITY, f64::max);
        let exp_scores: Vec<f64> = scores.iter().map(|(_, s)| (s - max_score).exp()).collect();
        let sum_exp: f64 = exp_scores.iter().sum();

        let mut priors = Vec::new();
        for (i, (tactic, _)) in valid_transitions.into_iter().enumerate() {
            let prob = exp_scores[i] / sum_exp;
            priors.push((tactic, prob));
        }

        PolicyOutput {
            prior_probabilities: priors,
            state_value: base_value,
            reasoning_trace: format!(
                "Evaluated {} candidate tactics. Goal complexity: {}",
                scores.len(),
                total_complexity
            ),
        }
    }
}
