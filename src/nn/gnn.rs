use super::graph::MasterMathGraph;
use super::model::Matrix;
use crate::verifier::kernel::ProofState;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Sparse Graph Attention & Message Passing Network over MasterMathGraph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SparseGraphAttentionNetwork {
    pub master_graph: MasterMathGraph,
    pub node_dim: usize,
    pub node_embeddings: Vec<Vec<f64>>,
    pub w_self: Matrix,
    pub w_edge: Matrix,
    pub w_gate: Matrix,
    pub w_pool: Matrix,
}

impl SparseGraphAttentionNetwork {
    pub fn new(node_dim: usize) -> Self {
        let master_graph = MasterMathGraph::default_universe();
        let num_nodes = master_graph.nodes.len();

        let mut node_embeddings = Vec::with_capacity(num_nodes);
        for i in 0..num_nodes {
            let mut emb = vec![0.0; node_dim];
            // Positional domain seed
            emb[i % node_dim] = 1.0;
            node_embeddings.push(emb);
        }

        Self {
            node_embeddings,
            w_self: Matrix::init_random(node_dim, node_dim),
            w_edge: Matrix::init_random(node_dim, node_dim),
            w_gate: Matrix::init_random(1, node_dim),
            w_pool: Matrix::init_random(32, node_dim),
            master_graph,
            node_dim,
        }
    }

    /// GELU activation helper
    fn gelu(x: f64) -> f64 {
        0.5 * x * (1.0 + (0.79788456 * (x + 0.044715 * x.powi(3))).tanh())
    }

    /// Executes message passing and graph attention strictly over the active subgraph
    /// Returns (pooled_embedding, sparse_activation_weights)
    pub fn forward_active_subgraph(&self, state: &ProofState) -> (Vec<f64>, HashMap<usize, f64>) {
        let active_nodes = self.master_graph.active_subgraph_nodes(state, 1);
        let mut updated_embeddings: HashMap<usize, Vec<f64>> = HashMap::new();
        let mut activation_scores: HashMap<usize, f64> = HashMap::new();

        // 1. Sparse Message Passing for active nodes
        for &node_id in &active_nodes {
            let h_self = &self.node_embeddings[node_id];
            let self_contrib = self.w_self.dot_vec(h_self);

            let mut edge_contrib = vec![0.0; self.node_dim];
            let mut neighbor_count = 0.0;

            if let Some(neighbors) = self.master_graph.adjacency.get(&node_id) {
                for &(neighbor_id, _, weight) in neighbors {
                    if active_nodes.contains(&neighbor_id) {
                        let h_neighbor = &self.node_embeddings[neighbor_id];
                        let transformed = self.w_edge.dot_vec(h_neighbor);
                        for d in 0..self.node_dim {
                            edge_contrib[d] += transformed[d] * weight;
                        }
                        neighbor_count += 1.0;
                    }
                }
            }

            let mut h_new = vec![0.0; self.node_dim];
            for d in 0..self.node_dim {
                let neighbor_avg = if neighbor_count > 0.0 {
                    edge_contrib[d] / neighbor_count
                } else {
                    0.0
                };
                h_new[d] = Self::gelu(self_contrib[d] + neighbor_avg);
            }

            // Gating score (soft attention weight)
            let gate_logit = self.w_gate.dot_vec(&h_new)[0];
            let gate_score = 1.0 / (1.0 + (-gate_logit).exp()); // Sigmoid

            activation_scores.insert(node_id, gate_score);
            updated_embeddings.insert(node_id, h_new);
        }

        // 2. Softmax Normalization over active nodes
        let sum_gates: f64 = activation_scores.values().sum();
        let normalized_weights: HashMap<usize, f64> = activation_scores
            .into_iter()
            .map(|(k, v)| {
                let norm_v = if sum_gates > 0.0 { v / sum_gates } else { 0.0 };
                (k, norm_v)
            })
            .collect();

        // 3. Attentive Graph Pooling
        let mut aggregated_graph_vector = vec![0.0; self.node_dim];
        for (&node_id, &weight) in &normalized_weights {
            if let Some(h) = updated_embeddings.get(&node_id) {
                for d in 0..self.node_dim {
                    aggregated_graph_vector[d] += h[d] * weight;
                }
            }
        }

        // 4. Projection to output embedding dimension (32-dim)
        let pooled_embedding = self.w_pool.dot_vec(&aggregated_graph_vector);

        (pooled_embedding, normalized_weights)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nn::NodeType;
    use crate::verifier::fol::{Equality, Term};

    #[test]
    fn test_sparse_gnn_forward_pass() {
        let gnn = SparseGraphAttentionNetwork::new(16);
        let x = Term::var("x");
        let zero = Term::constant("0");
        let state = ProofState::new(Equality::new(
            Term::func("+", vec![x.clone(), zero.clone()]),
            x,
        ));

        let (pooled, activations) = gnn.forward_active_subgraph(&state);

        assert_eq!(pooled.len(), 32);
        assert!(!activations.is_empty());

        let total_weight: f64 = activations.values().sum();
        assert!((total_weight - 1.0).abs() < 1e-5);

        // Verify that boolean operators were NOT activated
        let not_id = gnn.master_graph.find_node(&NodeType::Operator("!".into())).unwrap();
        assert!(!activations.contains_key(&not_id));
    }
}
