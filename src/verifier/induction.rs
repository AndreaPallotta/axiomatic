use super::fol::{Equality, Term};
use super::kernel::{Goal, ProofState, Tactic};

/// Peano Arithmetic and Structural Induction Engine
pub struct InductionEngine;

impl InductionEngine {
    /// Applies Peano induction on a variable in the current proof goal
    pub fn apply_induction(state: &ProofState, var_name: &str) -> Result<ProofState, String> {
        if state.open_goals.is_empty() {
            return Err("No open goals to apply induction on".to_string());
        }

        let current_goal = &state.open_goals[0];
        let eq = &current_goal.equality;

        // Check that the variable occurs in the goal
        if !eq.lhs.contains_symbol(var_name) && !eq.rhs.contains_symbol(var_name) {
            return Err(format!("Variable '{}' does not occur in goal", var_name));
        }

        let zero = Term::constant("0");
        let k_var = Term::constant(&format!("{}_k", var_name));
        let succ_k = Term::func("succ", vec![k_var.clone()]);

        // 1. Base Case: P(0)
        let base_lhs = eq.lhs.replace_variable(var_name, &zero);
        let base_rhs = eq.rhs.replace_variable(var_name, &zero);
        let base_goal = Goal {
            id: current_goal.id * 10 + 1,
            equality: Equality::new(base_lhs, base_rhs),
        };

        // 2. Inductive Step: P(succ(k))
        let step_lhs = eq.lhs.replace_variable(var_name, &succ_k);
        let step_rhs = eq.rhs.replace_variable(var_name, &succ_k);
        let step_goal = Goal {
            id: current_goal.id * 10 + 2,
            equality: Equality::new(step_lhs, step_rhs),
        };

        let mut new_open_goals = vec![base_goal, step_goal];
        new_open_goals.extend_from_slice(&state.open_goals[1..]);

        let mut new_history = state.proof_history.clone();
        new_history.push((
            Tactic::ApplyAxiom(format!("induction on {}", var_name)),
            format!(
                "Split into Base Case (n=0) and Inductive Step (n=succ({}))",
                var_name
            ),
        ));

        let mut next_state = ProofState {
            open_goals: new_open_goals,
            proof_history: new_history,
            is_solved: false,
            depth: state.depth + 1,
        };

        next_state.check_solved();
        Ok(next_state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_peano_induction_split() {
        let n = Term::constant("n");
        let zero = Term::constant("0");
        let goal_eq = Equality::new(Term::func("+", vec![n.clone(), zero.clone()]), n.clone());

        let initial_state = ProofState::new(goal_eq);
        let inductive_state = InductionEngine::apply_induction(&initial_state, "n")
            .expect("Induction should succeed");

        assert_eq!(inductive_state.open_goals.len(), 2);
        // Base case: 0 + 0 = 0
        assert_eq!(
            inductive_state.open_goals[0].equality.to_string(),
            "(0 + 0) = 0"
        );
        // Inductive step: succ(n_k) + 0 = succ(n_k)
        assert_eq!(
            inductive_state.open_goals[1].equality.to_string(),
            "(succ(n_k) + 0) = succ(n_k)"
        );
    }
}
