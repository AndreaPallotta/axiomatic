use super::node::MctsNode;
use crate::generator::policy::NeuralPolicy;
use crate::verifier::kernel::{AxiomLibrary, FormalVerifier, ProofState};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

/// Real-time search event emitted to the graphical visualizer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SearchEvent {
    TreeReset(SearchGraphSnapshot),
    NodeCreated(MctsNode),
    NodeVisited {
        id: usize,
        visits: usize,
        mean_value: f64,
    },
    ProofDiscovered {
        node_id: usize,
        depth: usize,
        tactics_count: usize,
    },
    SearchStepCompleted {
        iteration: usize,
        total_nodes: usize,
        best_value: f64,
    },
}

/// Snapshot of the complete search graph for live D3 / Canvas rendering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchGraphSnapshot {
    pub nodes: Vec<MctsNode>,
    pub total_iterations: usize,
    pub proven_node_id: Option<usize>,
}

/// The Neurosymbolic Monte Carlo Tree Search Engine
pub struct MctsEngine {
    pub nodes: Vec<MctsNode>,
    pub c_puct: f64,
    pub max_depth: usize,
    pub proven_node_id: Option<usize>,
    pub iterations: usize,
    pub event_sender: Option<broadcast::Sender<SearchEvent>>,
}

impl MctsEngine {
    pub fn new(initial_state: ProofState, max_depth: usize) -> Self {
        let root = MctsNode::new_root(initial_state);
        let proven_id = if root.is_proven { Some(0) } else { None };
        Self {
            nodes: vec![root],
            c_puct: 3.5,
            max_depth,
            proven_node_id: proven_id,
            iterations: 0,
            event_sender: None,
        }
    }

    pub fn reset_tree(&mut self, state: ProofState) {
        let root = MctsNode::new_root(state);
        self.proven_node_id = if root.is_proven { Some(0) } else { None };
        self.nodes = vec![root];
        self.iterations = 0;
        let snap = self.snapshot();
        self.emit(SearchEvent::TreeReset(snap));
    }

    pub fn set_event_sender(&mut self, sender: broadcast::Sender<SearchEvent>) {
        self.event_sender = Some(sender);
        let snap = self.snapshot();
        self.emit(SearchEvent::TreeReset(snap));
    }

    fn emit(&self, event: SearchEvent) {
        if let Some(ref sender) = self.event_sender {
            let _ = sender.send(event);
        }
    }

    /// Selects the most promising leaf node using PUCT
    fn select_leaf(&self) -> usize {
        let mut current_id = 0;

        loop {
            let node = &self.nodes[current_id];
            if !node.is_expanded || node.children_ids.is_empty() || node.is_terminal {
                return current_id;
            }

            // Find best child by PUCT
            let mut best_score = f64::NEG_INFINITY;
            let mut best_child_id = node.children_ids[0];

            for &child_id in &node.children_ids {
                let child = &self.nodes[child_id];
                let score = child.puct_score(node.visit_count, self.c_puct);
                if score > best_score {
                    best_score = score;
                    best_child_id = child_id;
                }
            }

            current_id = best_child_id;
        }
    }

    /// Expands a leaf node using the formal verifier & neural policy
    fn expand_and_evaluate(
        &mut self,
        leaf_id: usize,
        policy: &dyn NeuralPolicy,
        axioms: &AxiomLibrary,
    ) -> f64 {
        let leaf_state = self.nodes[leaf_id].state.clone();
        let leaf_depth = self.nodes[leaf_id].depth;

        // If leaf is already proven or depth exceeded, return value directly
        if self.nodes[leaf_id].is_proven {
            return 1.0;
        }

        if leaf_depth >= self.max_depth {
            self.nodes[leaf_id].is_terminal = true;
            return 0.0;
        }

        // Evaluate via Neural Policy
        let policy_output = policy.evaluate(&leaf_state, axioms);
        let value = policy_output.state_value;

        // Generate formally verified successor transitions
        let valid_transitions = FormalVerifier::expand_valid_transitions(&leaf_state, axioms);

        if valid_transitions.is_empty() {
            self.nodes[leaf_id].is_terminal = true;
            self.nodes[leaf_id].is_expanded = true;
            return 0.0;
        }

        // Build child nodes
        let mut child_ids = Vec::new();
        for (tactic, next_state) in valid_transitions {
            // Find neural prior for this tactic
            let prior = policy_output
                .prior_probabilities
                .iter()
                .find(|(t, _)| t == &tactic)
                .map(|(_, p)| *p)
                .unwrap_or(0.01);

            let new_node_id = self.nodes.len();
            let child_node = MctsNode::new_child(
                new_node_id,
                leaf_id,
                next_state,
                tactic,
                prior,
                leaf_depth + 1,
            );

            if child_node.is_proven && self.proven_node_id.is_none() {
                self.proven_node_id = Some(new_node_id);
                self.emit(SearchEvent::ProofDiscovered {
                    node_id: new_node_id,
                    depth: leaf_depth + 1,
                    tactics_count: child_node.state.proof_history.len(),
                });
            }

            self.emit(SearchEvent::NodeCreated(child_node.clone()));
            self.nodes.push(child_node);
            child_ids.push(new_node_id);
        }

        self.nodes[leaf_id].children_ids = child_ids;
        self.nodes[leaf_id].is_expanded = true;

        value
    }

    /// Backpropagates the neural value evaluation up the tree to the root
    fn backpropagate(&mut self, leaf_id: usize, value: f64) {
        let mut curr: Option<usize> = Some(leaf_id);

        while let Some(node_id) = curr {
            let (visits, mean_val, parent) = {
                let node = &mut self.nodes[node_id];
                node.update(value);
                (node.visit_count, node.mean_value, node.parent_id)
            };

            self.emit(SearchEvent::NodeVisited {
                id: node_id,
                visits,
                mean_value: mean_val,
            });

            curr = parent;
        }
    }

    /// Executes one full MCTS iteration (Select -> Expand/Eval -> Backpropagate)
    pub fn step(&mut self, policy: &dyn NeuralPolicy, axioms: &AxiomLibrary) -> Option<usize> {
        self.iterations += 1;
        let leaf_id = self.select_leaf();
        let value = self.expand_and_evaluate(leaf_id, policy, axioms);
        self.backpropagate(leaf_id, value);

        self.emit(SearchEvent::SearchStepCompleted {
            iteration: self.iterations,
            total_nodes: self.nodes.len(),
            best_value: self.nodes[0].mean_value,
        });

        self.proven_node_id
    }

    /// Runs MCTS until a proof is discovered or max_iterations is reached
    pub fn run_search(
        &mut self,
        policy: &dyn NeuralPolicy,
        axioms: &AxiomLibrary,
        max_iterations: usize,
    ) -> Option<ProofState> {
        for _ in 0..max_iterations {
            if let Some(proven_id) = self.step(policy, axioms) {
                let mut state = self.nodes[proven_id].state.clone();
                state.minimize(axioms);
                return Some(state);
            }
        }

        if let Some(proven_id) = self.proven_node_id {
            let mut state = self.nodes[proven_id].state.clone();
            state.minimize(axioms);
            Some(state)
        } else {
            None
        }
    }

    /// Runs MCTS with dynamic premise retrieval from the Vector DB and Lemma Database
    pub fn run_search_with_premises(
        &mut self,
        policy: &dyn NeuralPolicy,
        base_axioms: &AxiomLibrary,
        vector_db: &crate::memory::vectordb::MathematicalVectorDB,
        lemma_db: &crate::memory::database::LemmaDatabase,
        top_k: usize,
        max_iterations: usize,
    ) -> Option<ProofState> {
        let mut active_axioms = base_axioms.clone();
        if !vector_db.records.is_empty() && top_k > 0 && !self.nodes.is_empty() {
            let query_vec = crate::nn::embedding::vectorize_proof_state(&self.nodes[0].state);
            let results = vector_db.query(&query_vec, top_k);
            for res in results {
                if let Some(thm) = lemma_db.theorems.iter().find(|t| t.name == res.record.payload.name) {
                    if !active_axioms.rules.iter().any(|(n, _)| n == &thm.name) {
                        active_axioms.add_rule(&thm.name, thm.statement.clone());
                    }
                }
            }
        }
        self.run_search(policy, &active_axioms, max_iterations)
    }

    /// Returns a full snapshot of the search graph
    pub fn snapshot(&self) -> SearchGraphSnapshot {
        SearchGraphSnapshot {
            nodes: self.nodes.clone(),
            total_iterations: self.iterations,
            proven_node_id: self.proven_node_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generator::policy::SymbolicNeuralPolicy;
    use crate::verifier::fol::{Equality, Term};

    #[test]
    fn test_mcts_proves_commutativity_autonomously() {
        let axioms = AxiomLibrary::standard_algebra();
        let policy = SymbolicNeuralPolicy::new();

        // Goal: a + 0 = 0 + a
        let a = Term::constant("a");
        let zero = Term::constant("0");
        let goal_eq = Equality::new(
            Term::func("+", vec![a.clone(), zero.clone()]),
            Term::func("+", vec![zero.clone(), a.clone()]),
        );

        let initial_state = ProofState::new(goal_eq);
        let mut mcts = MctsEngine::new(initial_state, 6);

        let proof = mcts.run_search(&policy, &axioms, 100);
        assert!(proof.is_some(), "MCTS must autonomously discover the proof");
        let solved = proof.unwrap();
        assert!(solved.is_solved, "Proof must be verified complete");
    }

    #[test]
    fn test_mcts_proves_multi_step_algebraic_goal() {
        let axioms = AxiomLibrary::standard_algebra();
        let policy = SymbolicNeuralPolicy::new();

        // Goal: ((x + 0) * 1) = (1 * x)
        let x = Term::constant("x");
        let zero = Term::constant("0");
        let one = Term::constant("1");
        let goal_eq = Equality::new(
            Term::func(
                "*",
                vec![Term::func("+", vec![x.clone(), zero.clone()]), one.clone()],
            ),
            Term::func("*", vec![one.clone(), x.clone()]),
        );

        let initial_state = ProofState::new(goal_eq);
        let mut mcts = MctsEngine::new(initial_state, 6);

        let proof = mcts.run_search(&policy, &axioms, 150);
        assert!(
            proof.is_some(),
            "MCTS must autonomously prove multi-step goal"
        );
        let solved = proof.unwrap();
        assert!(solved.is_solved, "Proof must be verified complete");
    }

    #[test]
    fn test_mcts_proves_real_root_polynomial_evaluation() {
        let axioms = AxiomLibrary::complex_numbers();
        let policy = SymbolicNeuralPolicy::new();

        let two = Term::constant("2");
        let ten = Term::constant("10");
        let goal_eq = Equality::new(
            Term::func(
                "+",
                vec![
                    Term::func(
                        "*",
                        vec![
                            Term::func("*", vec![two.clone(), two.clone()]),
                            two.clone(),
                        ],
                    ),
                    two,
                ],
            ),
            ten,
        );

        let initial_state = ProofState::new(goal_eq);
        let mut mcts = MctsEngine::new(initial_state, 6);

        let proof = mcts.run_search(&policy, &axioms, 50);
        let Some(solved) = proof else {
            panic!("MCTS must autonomously prove real root arithmetic goal");
        };
        assert!(solved.is_solved, "Proof must be verified complete");
    }

    #[test]
    fn test_mcts_proves_complex_unit_identity() {
        let axioms = AxiomLibrary::complex_numbers();
        let policy = SymbolicNeuralPolicy::new();

        let i = Term::constant("i");
        let neg_one = Term::from_i64(-1);
        let goal_eq = Equality::new(
            Term::func("*", vec![i.clone(), i]),
            neg_one,
        );

        let initial_state = ProofState::new(goal_eq);
        let mut mcts = MctsEngine::new(initial_state, 6);

        let proof = mcts.run_search(&policy, &axioms, 50);
        let Some(solved) = proof else {
            panic!("MCTS must autonomously prove complex unit identity");
        };
        assert!(solved.is_solved, "Proof must be verified complete");
    }

    #[test]
    fn test_mcts_proves_complex_imaginary_rotation() {
        let axioms = AxiomLibrary::complex_numbers();
        let policy = SymbolicNeuralPolicy::new();

        let x = Term::constant("x");
        let i = Term::constant("i");
        let goal_eq = Equality::new(
            Term::func(
                "*",
                vec![Term::func("*", vec![x.clone(), i.clone()]), i],
            ),
            Term::func("-", vec![x]),
        );

        let initial_state = ProofState::new(goal_eq);
        let mut mcts = MctsEngine::new(initial_state, 6);

        let proof = mcts.run_search(&policy, &axioms, 50);
        let Some(solved) = proof else {
            panic!("MCTS must autonomously prove complex imaginary rotation");
        };
        assert!(solved.is_solved, "Proof must be verified complete");
    }

    #[test]
    fn test_mcts_proves_all_roots_of_cubic() {
        let axioms = AxiomLibrary::complex_numbers();
        let policy = SymbolicNeuralPolicy::new();
        let Ok(eq) = crate::verifier::parser::parse_conjecture("(((X * X) * X) + X) = 10") else {
            panic!("Failed to parse cubic equation");
        };
        let roots = crate::verifier::eval::find_polynomial_roots(&eq, "X", 4);
        assert_eq!(roots.len(), 3);
        for root in roots {
            let subst_eq = eq.replace_variable("X", &root);
            let initial_state = ProofState::new(subst_eq);
            let mut mcts = MctsEngine::new(initial_state, 8);
            let proof = mcts.run_search(&policy, &axioms, 100);
            let Some(solved) = proof else {
                panic!("Must prove satisfaction for root");
            };
            assert!(solved.is_solved);
        }
    }

    #[test]
    fn test_mcts_proves_rational_root_arithmetic() {
        let axioms = AxiomLibrary::standard_algebra();
        let policy = SymbolicNeuralPolicy::new();
        let Ok(eq) = crate::verifier::parser::parse_conjecture("(2 * X) - 3 = 0") else {
            panic!("Failed to parse linear equation");
        };
        let roots = crate::verifier::eval::find_polynomial_roots(&eq, "X", 4);
        assert!(!roots.is_empty());
        let subst_eq = eq.replace_variable("X", &roots[0]);
        let initial_state = ProofState::new(subst_eq);
        let mut mcts = MctsEngine::new(initial_state, 8);
        let proof = mcts.run_search(&policy, &axioms, 50);
        let Some(solved) = proof else {
            panic!("MCTS must prove rational root substitution");
        };
        assert!(solved.is_solved);
    }

    #[test]
    fn test_mcts_proves_calculus_sum_and_exp() {
        let axioms = AxiomLibrary::symbolic_calculus();
        let policy = SymbolicNeuralPolicy::new();

        let Ok(eq) = crate::verifier::parser::parse_conjecture("D(x + 1) = (1 + 0)") else {
            panic!("Failed to parse calculus sum");
        };
        let initial_state = ProofState::new(eq);
        let mut mcts = MctsEngine::new(initial_state, 6);
        let proof = mcts.run_search(&policy, &axioms, 50);
        let Some(solved) = proof else {
            panic!("MCTS must prove calculus derivative of sum");
        };
        assert!(solved.is_solved);
    }

    #[test]
    fn test_mcts_proves_first_order_ode() {
        let axioms = AxiomLibrary::symbolic_calculus();
        let policy = SymbolicNeuralPolicy::new();

        let Ok(eq) = crate::verifier::parser::parse_conjecture("(D(exp(x)) - exp(x)) = 0") else {
            panic!("Failed to parse ODE conjecture");
        };
        let initial_state = ProofState::new(eq);
        let mut mcts = MctsEngine::new(initial_state, 6);
        let proof = mcts.run_search(&policy, &axioms, 50);
        let Some(solved) = proof else {
            panic!("MCTS must prove exponential satisfies ODE");
        };
        assert!(solved.is_solved);
    }

    #[test]
    fn test_mcts_proves_difference_of_squares() {
        let axioms = AxiomLibrary::standard_algebra();
        let policy = SymbolicNeuralPolicy::new();

        let Ok(eq) = crate::verifier::parser::parse_conjecture("((x - y) * (x + y)) = ((x * x) - (y * y))") else {
            panic!("Failed to parse difference of squares");
        };
        let initial_state = ProofState::new(eq);
        let mut mcts = MctsEngine::new(initial_state, 4);
        let proof = mcts.run_search(&policy, &axioms, 20);
        let Some(solved) = proof else {
            panic!("MCTS must prove difference of squares");
        };
        assert!(solved.is_solved);
    }

    #[test]
    fn test_negative_conjectures_exhaust_without_hallucination() {
        let axioms = AxiomLibrary::standard_algebra();
        let policy = SymbolicNeuralPolicy::new();

        let false_conjectures = [
            "1 = 0",
            "(2 + 2) = 5",
            "(x + 1) = x",
            "((x + y) * (x + y)) = ((x * x) + (y * y))",
        ];

        for conj in false_conjectures {
            let Ok(eq) = crate::verifier::parser::parse_conjecture(conj) else {
                panic!("Failed to parse conjecture: {}", conj);
            };
            let initial_state = ProofState::new(eq);
            let mut mcts = MctsEngine::new(initial_state, 6);
            let proof = mcts.run_search(&policy, &axioms, 30);
            assert!(proof.is_none(), "Engine must reject invalid conjecture without hallucinating proof: {}", conj);
        }
    }

    #[test]
    fn test_mcts_proves_fundamental_theorem_of_calculus() {
        let axioms = AxiomLibrary::symbolic_calculus();
        let policy = SymbolicNeuralPolicy::new();

        let Ok(eq) = crate::verifier::parser::parse_conjecture("D(Int(x)) = x") else {
            panic!("Failed to parse FTC conjecture");
        };
        let initial_state = ProofState::new(eq);
        let mut mcts = MctsEngine::new(initial_state, 4);
        let proof = mcts.run_search(&policy, &axioms, 20);
        let Some(solved) = proof else {
            panic!("MCTS must prove Fundamental Theorem of Calculus");
        };
        assert!(solved.is_solved);
    }

    #[test]
    fn test_mcts_proves_harmonic_oscillator_ode() {
        let axioms = AxiomLibrary::symbolic_calculus();
        let policy = SymbolicNeuralPolicy::new();

        let Ok(eq) = crate::verifier::parser::parse_conjecture("(D(D(sin(x))) + sin(x)) = 0") else {
            panic!("Failed to parse harmonic oscillator conjecture");
        };
        let initial_state = ProofState::new(eq);
        let mut mcts = MctsEngine::new(initial_state, 10);
        let proof = mcts.run_search(&policy, &axioms, 100);
        let Some(solved) = proof else {
            panic!("MCTS must prove harmonic oscillator ODE");
        };
        assert!(solved.is_solved);
    }

    #[test]
    fn test_mcts_proves_matrix_transpose_product() {
        let axioms = AxiomLibrary::linear_algebra();
        let policy = SymbolicNeuralPolicy::new();

        let Ok(eq) = crate::verifier::parser::parse_conjecture("T(A * B) = (T(B) * T(A))") else {
            panic!("Failed to parse transpose product conjecture");
        };
        let initial_state = ProofState::new(eq);
        let mut mcts = MctsEngine::new(initial_state, 4);
        let proof = mcts.run_search(&policy, &axioms, 20);
        let Some(solved) = proof else {
            panic!("MCTS must prove matrix transpose product identity");
        };
        assert!(solved.is_solved);
    }

    #[test]
    fn test_mcts_proves_matrix_trace_cyclic() {
        let axioms = AxiomLibrary::linear_algebra();
        let policy = SymbolicNeuralPolicy::new();

        let Ok(eq) = crate::verifier::parser::parse_conjecture("tr(A * B) = tr(B * A)") else {
            panic!("Failed to parse trace cyclic conjecture");
        };
        let initial_state = ProofState::new(eq);
        let mut mcts = MctsEngine::new(initial_state, 4);
        let proof = mcts.run_search(&policy, &axioms, 20);
        let Some(solved) = proof else {
            panic!("MCTS must prove cyclic trace property");
        };
        assert!(solved.is_solved);
    }

    #[test]
    fn test_mcts_proves_inequality_square_nonneg() {
        let axioms = AxiomLibrary::order_theory();
        let policy = SymbolicNeuralPolicy::new();

        let Ok(eq) = crate::verifier::parser::parse_conjecture("0 <= (x * x)") else {
            panic!("Failed to parse inequality conjecture");
        };
        let initial_state = ProofState::new(eq);
        let mut mcts = MctsEngine::new(initial_state, 4);
        let proof = mcts.run_search(&policy, &axioms, 20);
        let Some(solved) = proof else {
            panic!("MCTS must prove square non-negativity inequality");
        };
        assert!(solved.is_solved);
    }

    #[test]
    fn test_mcts_proves_dynamic_premise_selection() {
        let policy = SymbolicNeuralPolicy::new();
        let base_axioms = AxiomLibrary::empty();

        let Ok(lemma_eq) = crate::verifier::parser::parse_conjecture("((x * x) - (y * y)) = ((x - y) * (x + y))") else {
            panic!("Failed to parse lemma");
        };
        let mut lemma_db = crate::memory::database::LemmaDatabase::new();
        lemma_db.record_theorem("factor_diff_squares", lemma_eq.clone(), ProofState::new(lemma_eq.clone()));

        let mut vector_db = crate::memory::vectordb::MathematicalVectorDB::new(
            crate::nn::embedding::EMBEDDING_DIM,
            crate::memory::vectordb::DistanceMetric::Cosine,
        );
        let dummy_state = ProofState::new(lemma_eq);
        let emb = crate::nn::embedding::vectorize_proof_state(&dummy_state);
        vector_db.insert(
            emb,
            crate::memory::vectordb::TheoremPayload {
                name: "factor_diff_squares".to_string(),
                statement: "((x * x) - (y * y)) = ((x - y) * (x + y))".to_string(),
                tactic_name: "rw_lhs [factor_diff_squares]".to_string(),
                proof_length: 1,
                timestamp: "2026-09-12".to_string(),
            },
        );

        let Ok(target_eq) = crate::verifier::parser::parse_conjecture("((x * x) - (y * y)) = ((x - y) * (x + y))") else {
            panic!("Failed to parse target");
        };
        let mut mcts = MctsEngine::new(ProofState::new(target_eq), 4);
        let proof = mcts.run_search_with_premises(&policy, &base_axioms, &vector_db, &lemma_db, 1, 20);
        let Some(solved) = proof else {
            panic!("MCTS must retrieve premise and prove conjecture");
        };
        assert!(solved.is_solved);
    }

    #[test]
    fn test_mcts_proves_clairaut_mixed_partials() {
        let axioms = AxiomLibrary::multivariable_calculus();
        let policy = SymbolicNeuralPolicy::new();

        let Ok(eq) = crate::verifier::parser::parse_conjecture("Dx(Dy(f)) = Dy(Dx(f))") else {
            panic!("Failed to parse Clairaut mixed partials conjecture");
        };
        let mut mcts = MctsEngine::new(ProofState::new(eq), 4);
        let proof = mcts.run_search(&policy, &axioms, 10);
        let Some(solved) = proof else {
            panic!("MCTS must prove Clairaut's theorem on mixed partials");
        };
        assert!(solved.is_solved);
    }

    #[test]
    fn test_mcts_proves_curl_grad_zero() {
        let axioms = AxiomLibrary::multivariable_calculus();
        let policy = SymbolicNeuralPolicy::new();

        let Ok(eq) = crate::verifier::parser::parse_conjecture("curl(grad(f)) = 0") else {
            panic!("Failed to parse curl grad conjecture");
        };
        let mut mcts = MctsEngine::new(ProofState::new(eq), 4);
        let proof = mcts.run_search(&policy, &axioms, 10);
        let Some(solved) = proof else {
            panic!("MCTS must prove curl of grad is zero");
        };
        assert!(solved.is_solved);
    }

    #[test]
    fn test_mcts_proves_hyperbolic_fundamental_identity() {
        let axioms = AxiomLibrary::symbolic_calculus();
        let policy = SymbolicNeuralPolicy::new();

        let Ok(eq) = crate::verifier::parser::parse_conjecture("((cosh(x) * cosh(x)) + -(sinh(x) * sinh(x))) = 1") else {
            panic!("Failed to parse hyperbolic conjecture");
        };
        let mut mcts = MctsEngine::new(ProofState::new(eq), 4);
        let proof = mcts.run_search(&policy, &axioms, 10);
        let Some(solved) = proof else {
            panic!("MCTS must prove hyperbolic fundamental identity");
        };
        assert!(solved.is_solved);
    }

    #[test]
    fn test_mcts_proves_probability_bayes_symmetric() {
        let axioms = AxiomLibrary::probability();
        let policy = SymbolicNeuralPolicy::new();

        let Ok(eq) = crate::verifier::parser::parse_conjecture("(cond(A, B) * P(B)) = (cond(B, A) * P(A))") else {
            panic!("Failed to parse Bayes symmetric conjecture");
        };
        let mut mcts = MctsEngine::new(ProofState::new(eq), 4);
        let proof = mcts.run_search(&policy, &axioms, 10);
        let Some(solved) = proof else {
            panic!("MCTS must prove symmetric Bayes formulation");
        };
        assert!(solved.is_solved);
    }

    #[test]
    fn test_mcts_bidirectional_confluence_shortcut() {
        let axioms = AxiomLibrary::standard_algebra();
        let policy = SymbolicNeuralPolicy::new();

        let Ok(eq) = crate::verifier::parser::parse_conjecture("(x + 0) = (0 + x)") else {
            panic!("Failed to parse commutativity identity");
        };
        let mut mcts = MctsEngine::new(ProofState::new(eq), 4);
        let proof = mcts.run_search(&policy, &axioms, 5);
        let Some(solved) = proof else {
            panic!("MCTS must prove via confluence");
        };
        assert!(solved.is_solved);
    }
}
