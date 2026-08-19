use crate::verifier::kernel::ProofState;
use serde::{Deserialize, Serialize};

/// Reinforcement Learning Reward Engine Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardConfig {
    pub success_reward: f64,
    pub failure_penalty: f64,
    pub conciseness_penalty_rate: f64,
    pub simplification_weight: f64,
    pub gamma_discount: f64,
}

impl Default for RewardConfig {
    fn default() -> Self {
        Self {
            success_reward: 1.0,
            failure_penalty: -1.0,
            conciseness_penalty_rate: 0.02,
            simplification_weight: 0.05,
            gamma_discount: 0.99,
        }
    }
}

/// Computes RL rewards and discounted returns for MCTS proof search trajectories
pub struct RewardEngine {
    pub config: RewardConfig,
}

impl RewardEngine {
    pub fn new(config: RewardConfig) -> Self {
        Self { config }
    }

    /// Evaluates a trajectory of states and returns the target return z_t for each step
    pub fn compute_trajectory_returns(
        &self,
        states: &[ProofState],
        is_proven: bool,
    ) -> Vec<f64> {
        let n = states.len();
        if n == 0 {
            return Vec::new();
        }

        // 1. Compute Step-wise Immediate Rewards
        let mut step_rewards = vec![0.0; n];
        for i in 0..(n - 1) {
            let prev_size = Self::state_ast_size(&states[i]);
            let next_size = Self::state_ast_size(&states[i + 1]);

            // Reward AST reductions (simplifications)
            let simplification_delta = (prev_size as f64 - next_size as f64) * self.config.simplification_weight;
            step_rewards[i] = simplification_delta;
        }

        // 2. Terminal Reward
        let terminal_reward = if is_proven {
            let length_penalty = (n as f64) * self.config.conciseness_penalty_rate;
            self.config.success_reward - length_penalty
        } else {
            self.config.failure_penalty
        };

        step_rewards[n - 1] += terminal_reward;

        // 3. Backward Bellman Discounted Return: G_t = r_t + \gamma * G_{t+1}
        let mut returns = vec![0.0; n];
        let mut g = 0.0;
        for t in (0..n).rev() {
            g = step_rewards[t] + self.config.gamma_discount * g;
            returns[t] = g.clamp(-1.0, 1.0);
        }

        returns
    }

    /// Computes total node size of open goals in a ProofState
    pub fn state_ast_size(state: &ProofState) -> usize {
        state
            .open_goals
            .iter()
            .map(|g| Self::term_ast_size(&g.equality.lhs) + Self::term_ast_size(&g.equality.rhs))
            .sum()
    }

    fn term_ast_size(term: &crate::verifier::fol::Term) -> usize {
        match term {
            crate::verifier::fol::Term::Var(_) | crate::verifier::fol::Term::Const(_) => 1,
            crate::verifier::fol::Term::Func(_, args) => {
                1 + args.iter().map(Self::term_ast_size).sum::<usize>()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::verifier::fol::{Equality, Term};

    #[test]
    fn test_reward_engine_computation() {
        let engine = RewardEngine::new(RewardConfig::default());

        let x = Term::var("x");
        let zero = Term::constant("0");
        let s0 = ProofState::new(Equality::new(
            Term::func("+", vec![x.clone(), zero.clone()]),
            x.clone(),
        ));
        let mut s1 = ProofState::new(Equality::new(x.clone(), x.clone()));
        s1.is_solved = true;

        let returns = engine.compute_trajectory_returns(&[s0, s1], true);
        assert_eq!(returns.len(), 2);
        assert!(returns[0] > 0.5, "Return must be positive for successful proof");
    }
}
