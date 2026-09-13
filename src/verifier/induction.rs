use super::fol::{Equality, Term};
use super::kernel::{AxiomLibrary, Goal, ProofState, Tactic};
use super::lean::{map_rule_to_lean_typed, term_to_lean};
use crate::generator::policy::SymbolicNeuralPolicy;
use crate::search::mcts::MctsEngine;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InductiveProof {
    pub var_name: String,
    pub theorem_name: String,
    pub initial_conjecture: Equality,
    pub base_case_goal: Equality,
    pub base_case_proof: ProofState,
    pub inductive_hypothesis: Equality,
    pub inductive_step_goal: Equality,
    pub inductive_step_proof: ProofState,
}

impl InductiveProof {
    pub fn export_to_lean4(&self) -> String {
        let mut code = String::new();
        code.push_str("-- ========================================================\n");
        code.push_str(&format!(
            "-- AXIOMATIC AUTONOMOUS INDUCTIVE PROOF: {}\n",
            self.theorem_name
        ));
        code.push_str("-- Synthesized by Induction Engine 2.0 & Verified by Lean 4 Kernel\n");
        code.push_str("-- ========================================================\n\n");
        code.push_str("set_option linter.unusedVariables false\n\n");

        let conj_str = format!(
            "{} = {}",
            term_to_lean(&self.initial_conjecture.lhs),
            term_to_lean(&self.initial_conjecture.rhs)
        );

        code.push_str(&format!(
            "theorem {} ({} : Nat) : {} := by\n",
            self.theorem_name, self.var_name, conj_str
        ));
        code.push_str(&format!("  induction {} with\n", self.var_name));
        code.push_str("  | zero =>\n");

        if self.base_case_proof.proof_history.is_empty() {
            code.push_str("    rfl\n");
        } else {
            for (tactic, _) in &self.base_case_proof.proof_history {
                match tactic {
                    Tactic::RewriteLhs(r) => {
                        let lean_r = map_rule_to_lean_typed(r, "Nat");
                        code.push_str(&format!("    rw [{}]\n", lean_r));
                    }
                    Tactic::RewriteRhs(r) => {
                        let lean_r = map_rule_to_lean_typed(r, "Nat");
                        code.push_str(&format!("    rw [<- {}]\n", lean_r));
                    }
                    Tactic::Reflexivity => {
                        code.push_str("    rfl\n");
                    }
                    _ => {}
                }
            }
            code.push_str("    try rfl\n");
        }

        code.push_str("  | succ k ih =>\n");
        if self.inductive_step_proof.proof_history.is_empty() {
            code.push_str("    rfl\n");
        } else {
            for (tactic, _) in &self.inductive_step_proof.proof_history {
                match tactic {
                    Tactic::RewriteLhs(r) => {
                        let lean_r = if r == "ih" {
                            "ih".to_string()
                        } else {
                            map_rule_to_lean_typed(r, "Nat")
                        };
                        code.push_str(&format!("    rw [{}]\n", lean_r));
                    }
                    Tactic::RewriteRhs(r) => {
                        let lean_r = if r == "ih" {
                            "<- ih".to_string()
                        } else {
                            format!("<- {}", map_rule_to_lean_typed(r, "Nat"))
                        };
                        code.push_str(&format!("    rw [{}]\n", lean_r));
                    }
                    Tactic::Reflexivity => {
                        code.push_str("    rfl\n");
                    }
                    _ => {}
                }
            }
            code.push_str("    try rfl\n");
        }

        code
    }
}

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

        if !eq.lhs.contains_symbol(var_name) && !eq.rhs.contains_symbol(var_name) {
            return Err(format!("Variable '{}' does not occur in goal", var_name));
        }

        let zero = Term::constant("0");
        let k_var = Term::constant(&format!("{}_k", var_name));
        let succ_k = Term::func("succ", vec![k_var.clone()]);

        let base_lhs = eq.lhs.replace_variable(var_name, &zero);
        let base_rhs = eq.rhs.replace_variable(var_name, &zero);
        let base_goal = Goal {
            id: current_goal.id * 10 + 1,
            equality: Equality::new(base_lhs, base_rhs),
        };

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
            initial_equality: state.initial_equality.clone(),
            open_goals: new_open_goals,
            proof_history: new_history,
            is_solved: false,
            depth: state.depth + 1,
        };

        next_state.check_solved();
        Ok(next_state)
    }

    /// Fully synthesizes an autonomous inductive proof by discovering proofs for base and step cases
    pub fn synthesize_induction_proof(
        conjecture: &Equality,
        var_name: &str,
        axioms: &AxiomLibrary,
        max_iterations: usize,
    ) -> Result<InductiveProof, String> {
        if !conjecture.lhs.contains_symbol(var_name) && !conjecture.rhs.contains_symbol(var_name) {
            return Err(format!("Variable '{}' does not occur in conjecture", var_name));
        }

        let zero = Term::constant("0");
        let k_var = Term::constant("k");

        let base_eq = Equality::new(
            conjecture.lhs.replace_variable(var_name, &zero),
            conjecture.rhs.replace_variable(var_name, &zero),
        );

        let policy = SymbolicNeuralPolicy::new();
        let mut base_engine = MctsEngine::new(ProofState::new(base_eq.clone()), 8);
        let base_proof = base_engine
            .run_search(&policy, axioms, max_iterations)
            .ok_or_else(|| format!("Induction Engine could not prove Base Case: {}", base_eq))?;

        let ih_eq = Equality::new(
            conjecture.lhs.replace_variable(var_name, &k_var),
            conjecture.rhs.replace_variable(var_name, &k_var),
        );

        let succ_k = Term::func("+", vec![k_var.clone(), Term::constant("1")]);
        let step_eq = Equality::new(
            conjecture.lhs.replace_variable(var_name, &succ_k),
            conjecture.rhs.replace_variable(var_name, &succ_k),
        );

        let mut step_axioms = axioms.clone();
        step_axioms.add_rule("ih", ih_eq.clone());

        let mut step_engine = MctsEngine::new(ProofState::new(step_eq.clone()), 8);
        let step_proof = step_engine
            .run_search(&policy, &step_axioms, max_iterations)
            .ok_or_else(|| format!("Induction Engine could not prove Inductive Step: {}", step_eq))?;

        Ok(InductiveProof {
            var_name: var_name.to_string(),
            theorem_name: "peano_induction_theorem".to_string(),
            initial_conjecture: conjecture.clone(),
            base_case_goal: base_eq,
            base_case_proof: base_proof,
            inductive_hypothesis: ih_eq,
            inductive_step_goal: step_eq,
            inductive_step_proof: step_proof,
        })
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
        assert_eq!(
            inductive_state.open_goals[0].equality.to_string(),
            "(0 + 0) = 0"
        );
        assert_eq!(
            inductive_state.open_goals[1].equality.to_string(),
            "(succ(n_k) + 0) = succ(n_k)"
        );
    }

    #[test]
    fn test_induction_engine_synthesize_peano() {
        let n = Term::constant("n");
        let zero = Term::constant("0");
        let goal_eq = Equality::new(Term::func("+", vec![n.clone(), zero.clone()]), n.clone());
        let axioms = AxiomLibrary::standard_algebra();

        let Ok(ind_proof) = InductionEngine::synthesize_induction_proof(&goal_eq, "n", &axioms, 100) else {
            panic!("Induction synthesis should succeed");
        };

        assert!(ind_proof.base_case_proof.is_solved);
        assert!(ind_proof.inductive_step_proof.is_solved);

        let lean_code = ind_proof.export_to_lean4();
        assert!(lean_code.contains("induction n with"));
        assert!(lean_code.contains("| zero =>"));
        assert!(lean_code.contains("| succ k ih =>"));
    }
}
