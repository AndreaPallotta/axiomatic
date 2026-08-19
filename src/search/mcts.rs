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
                return Some(self.nodes[proven_id].state.clone());
            }
        }

        if let Some(proven_id) = self.proven_node_id {
            Some(self.nodes[proven_id].state.clone())
        } else {
            None
        }
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
}
