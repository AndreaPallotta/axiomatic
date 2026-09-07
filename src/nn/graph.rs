use crate::verifier::fol::Term;
use crate::verifier::kernel::ProofState;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};

/// Semantic type of a node in the master mathematical graph
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NodeType {
    Operator(String),
    Constant(String),
    AxiomProperty(String),
    TacticPrimitive(String),
    Variable,
}

/// Typed relationship between mathematical entities
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EdgeType {
    Isomorphism,
    SubOperation,
    AppliesTactic,
    Dual,
    IdentityPair,
}

/// A node in the master mathematical graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MathGraphNode {
    pub id: usize,
    pub node_type: NodeType,
    pub name: String,
    pub domain_tags: Vec<String>,
}

/// A directed weighted edge in the master mathematical graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MathGraphEdge {
    pub from: usize,
    pub to: usize,
    pub edge_type: EdgeType,
    pub weight: f64,
}

/// Unified Master Mathematical Graph containing operators, constants, properties, and tactics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MasterMathGraph {
    pub nodes: Vec<MathGraphNode>,
    pub edges: Vec<MathGraphEdge>,
    pub adjacency: HashMap<usize, Vec<(usize, EdgeType, f64)>>,
}

impl MasterMathGraph {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
            adjacency: HashMap::new(),
        }
    }

    pub fn add_node(&mut self, node_type: NodeType, name: &str, domains: &[&str]) -> usize {
        let id = self.nodes.len();
        let node = MathGraphNode {
            id,
            node_type,
            name: name.to_string(),
            domain_tags: domains.iter().map(|s| s.to_string()).collect(),
        };
        self.nodes.push(node);
        self.adjacency.entry(id).or_default();
        id
    }

    pub fn add_edge(&mut self, from: usize, to: usize, edge_type: EdgeType, weight: f64) {
        assert!(from < self.nodes.len() && to < self.nodes.len());
        self.edges.push(MathGraphEdge {
            from,
            to,
            edge_type: edge_type.clone(),
            weight,
        });
        self.adjacency
            .entry(from)
            .or_default()
            .push((to, edge_type.clone(), weight));
        // Undirected message passing connectivity
        self.adjacency
            .entry(to)
            .or_default()
            .push((from, edge_type, weight));
    }

    /// Constructs the standard unified mathematical universe
    pub fn default_universe() -> Self {
        let mut g = Self::new();

        // 1. Shared Universal Isomorphism Hubs
        let prop_comm = g.add_node(
            NodeType::AxiomProperty("Commutativity".into()),
            "Commutativity",
            &["universal", "algebra", "boolean", "sets"],
        );
        let prop_assoc = g.add_node(
            NodeType::AxiomProperty("Associativity".into()),
            "Associativity",
            &["universal", "algebra", "boolean", "sets"],
        );
        let prop_dist = g.add_node(
            NodeType::AxiomProperty("Distributivity".into()),
            "Distributivity",
            &["universal", "algebra", "boolean", "sets"],
        );
        let prop_ident = g.add_node(
            NodeType::AxiomProperty("Identity".into()),
            "Identity",
            &["universal", "algebra", "boolean", "sets"],
        );
        let prop_absorb = g.add_node(
            NodeType::AxiomProperty("Annihilation".into()),
            "Annihilation",
            &["universal", "algebra", "boolean", "sets"],
        );
        let prop_inv = g.add_node(
            NodeType::AxiomProperty("Inversion".into()),
            "Inversion",
            &["universal", "algebra", "boolean", "sets"],
        );
        let prop_demorgan = g.add_node(
            NodeType::AxiomProperty("DeMorgan".into()),
            "DeMorgan",
            &["universal", "boolean", "sets"],
        );
        let prop_linear = g.add_node(
            NodeType::AxiomProperty("Linearity".into()),
            "Linearity",
            &["universal", "calculus"],
        );

        // 2. Constants
        let c_zero = g.add_node(
            NodeType::Constant("0".into()),
            "0 / Empty / False",
            &["algebra", "calculus", "sets", "peano"],
        );
        let c_one = g.add_node(
            NodeType::Constant("1".into()),
            "1 / Universe / True",
            &["algebra", "calculus", "boolean"],
        );
        let c_univ = g.add_node(NodeType::Constant("U".into()), "Universal Set", &["sets"]);
        let var_node = g.add_node(NodeType::Variable, "Variable", &["universal"]);

        // 3. Algebraic Operators
        let op_add = g.add_node(
            NodeType::Operator("+".into()),
            "Addition (+)",
            &["algebra", "calculus"],
        );
        let op_mul = g.add_node(
            NodeType::Operator("*".into()),
            "Multiplication (*)",
            &["algebra", "calculus"],
        );
        let op_neg = g.add_node(
            NodeType::Operator("-".into()),
            "Additive Negation (-)",
            &["algebra"],
        );

        // 4. Boolean Propositional Operators
        let op_and = g.add_node(
            NodeType::Operator("&".into()),
            "Conjunction (&)",
            &["boolean"],
        );
        let op_or = g.add_node(
            NodeType::Operator("|".into()),
            "Disjunction (|)",
            &["boolean"],
        );
        let op_not = g.add_node(NodeType::Operator("!".into()), "Negation (!)", &["boolean"]);

        // 5. Set Theoretic Operators
        let op_inter = g.add_node(
            NodeType::Operator("inter".into()),
            "Intersection",
            &["sets"],
        );
        let op_union = g.add_node(NodeType::Operator("union".into()), "Union", &["sets"]);
        let op_comp = g.add_node(NodeType::Operator("comp".into()), "Complement", &["sets"]);

        // 6. Calculus Operators
        let op_diff = g.add_node(
            NodeType::Operator("D".into()),
            "Derivative (D)",
            &["calculus"],
        );

        // 7. Peano Arithmetic Operator
        let op_succ = g.add_node(NodeType::Operator("S".into()), "Successor (S)", &["peano"]);

        // 8. Tactics
        let tac_rw_lhs = g.add_node(
            NodeType::TacticPrimitive("RewriteLhs".into()),
            "Rewrite LHS",
            &["universal"],
        );
        let tac_rw_rhs = g.add_node(
            NodeType::TacticPrimitive("RewriteRhs".into()),
            "Rewrite RHS",
            &["universal"],
        );
        let tac_symm = g.add_node(
            NodeType::TacticPrimitive("Symmetry".into()),
            "Symmetry",
            &["universal"],
        );
        let tac_rfl = g.add_node(
            NodeType::TacticPrimitive("Reflexivity".into()),
            "Reflexivity",
            &["universal"],
        );
        let tac_ind = g.add_node(
            NodeType::TacticPrimitive("Induction".into()),
            "Peano Induction",
            &["peano"],
        );

        // Connect Isomorphisms & Properties to Operations
        // Addition & Multiplication
        g.add_edge(op_add, prop_comm, EdgeType::Isomorphism, 1.0);
        g.add_edge(op_add, prop_assoc, EdgeType::Isomorphism, 1.0);
        g.add_edge(op_add, prop_ident, EdgeType::IdentityPair, 1.0);
        g.add_edge(op_add, c_zero, EdgeType::IdentityPair, 1.0);
        g.add_edge(op_add, op_neg, EdgeType::Dual, 1.0);
        g.add_edge(op_neg, prop_inv, EdgeType::Isomorphism, 1.0);

        g.add_edge(op_mul, prop_comm, EdgeType::Isomorphism, 1.0);
        g.add_edge(op_mul, prop_assoc, EdgeType::Isomorphism, 1.0);
        g.add_edge(op_mul, prop_ident, EdgeType::IdentityPair, 1.0);
        g.add_edge(op_mul, c_one, EdgeType::IdentityPair, 1.0);
        g.add_edge(op_mul, prop_absorb, EdgeType::Isomorphism, 1.0);
        g.add_edge(op_mul, c_zero, EdgeType::SubOperation, 1.0);

        // Distributivity bridge (Mul distributes over Add, And distributes over Or, Inter over Union)
        g.add_edge(op_mul, prop_dist, EdgeType::Isomorphism, 1.0);
        g.add_edge(op_add, prop_dist, EdgeType::SubOperation, 1.0);
        g.add_edge(op_and, prop_dist, EdgeType::Isomorphism, 1.0);
        g.add_edge(op_or, prop_dist, EdgeType::SubOperation, 1.0);
        g.add_edge(op_inter, prop_dist, EdgeType::Isomorphism, 1.0);
        g.add_edge(op_union, prop_dist, EdgeType::SubOperation, 1.0);

        // Boolean Operators
        g.add_edge(op_and, prop_comm, EdgeType::Isomorphism, 1.0);
        g.add_edge(op_and, prop_assoc, EdgeType::Isomorphism, 1.0);
        g.add_edge(op_or, prop_comm, EdgeType::Isomorphism, 1.0);
        g.add_edge(op_or, prop_assoc, EdgeType::Isomorphism, 1.0);
        g.add_edge(op_and, op_or, EdgeType::Dual, 1.0);
        g.add_edge(op_not, prop_demorgan, EdgeType::Isomorphism, 1.0);
        g.add_edge(op_and, prop_demorgan, EdgeType::SubOperation, 1.0);
        g.add_edge(op_or, prop_demorgan, EdgeType::SubOperation, 1.0);

        // Set Theory Operators
        g.add_edge(op_inter, prop_comm, EdgeType::Isomorphism, 1.0);
        g.add_edge(op_union, prop_comm, EdgeType::Isomorphism, 1.0);
        g.add_edge(op_inter, op_union, EdgeType::Dual, 1.0);
        g.add_edge(op_comp, prop_demorgan, EdgeType::Isomorphism, 1.0);
        g.add_edge(op_inter, c_zero, EdgeType::IdentityPair, 1.0);
        g.add_edge(op_union, c_univ, EdgeType::IdentityPair, 1.0);

        // Calculus Operators
        g.add_edge(op_diff, prop_linear, EdgeType::Isomorphism, 1.0);
        g.add_edge(op_diff, op_add, EdgeType::SubOperation, 1.0);
        g.add_edge(op_diff, op_mul, EdgeType::SubOperation, 1.0);
        g.add_edge(op_diff, c_zero, EdgeType::SubOperation, 1.0);
        g.add_edge(op_diff, c_one, EdgeType::SubOperation, 1.0);

        // Peano Operators
        g.add_edge(op_succ, c_zero, EdgeType::SubOperation, 1.0);
        g.add_edge(op_succ, tac_ind, EdgeType::AppliesTactic, 1.0);

        // Tactic connections to general operators
        g.add_edge(tac_rw_lhs, var_node, EdgeType::AppliesTactic, 1.0);
        g.add_edge(tac_rw_rhs, var_node, EdgeType::AppliesTactic, 1.0);
        g.add_edge(tac_symm, var_node, EdgeType::AppliesTactic, 1.0);
        g.add_edge(tac_rfl, var_node, EdgeType::AppliesTactic, 1.0);

        g
    }

    pub fn find_node(&self, node_type: &NodeType) -> Option<usize> {
        self.nodes.iter().position(|n| &n.node_type == node_type)
    }

    /// Extracts active seed nodes directly referenced by an expression term
    pub fn extract_term_node_types(&self, term: &Term, out: &mut HashSet<usize>) {
        match term {
            Term::Var(_) => {
                if let Some(id) = self.find_node(&NodeType::Variable) {
                    out.insert(id);
                }
            }
            Term::Const(c) => {
                if let Some(id) = self.find_node(&NodeType::Constant(c.clone())) {
                    out.insert(id);
                } else if let Some(id) = self.find_node(&NodeType::Constant("0".into())) {
                    out.insert(id);
                }
            }
            Term::Func(name, args) => {
                if let Some(id) = self.find_node(&NodeType::Operator(name.clone())) {
                    out.insert(id);
                }
                for arg in args {
                    self.extract_term_node_types(arg, out);
                }
            }
        }
    }

    /// Finds all active nodes within k-hops of the seed nodes extracted from a ProofState
    pub fn active_subgraph_nodes(&self, state: &ProofState, k_hops: usize) -> HashSet<usize> {
        let mut active = HashSet::new();

        // 1. Extract direct seed nodes from current goal
        if !state.open_goals.is_empty() {
            let goal = &state.open_goals[0];
            self.extract_term_node_types(&goal.equality.lhs, &mut active);
            self.extract_term_node_types(&goal.equality.rhs, &mut active);
        }

        // 2. Extract seed nodes from proof history tactics
        for (tactic, _) in &state.proof_history {
            match tactic {
                crate::verifier::kernel::Tactic::RewriteLhs(_) => {
                    if let Some(id) =
                        self.find_node(&NodeType::TacticPrimitive("RewriteLhs".into()))
                    {
                        active.insert(id);
                    }
                }
                crate::verifier::kernel::Tactic::RewriteRhs(_) => {
                    if let Some(id) =
                        self.find_node(&NodeType::TacticPrimitive("RewriteRhs".into()))
                    {
                        active.insert(id);
                    }
                }
                crate::verifier::kernel::Tactic::Symmetry => {
                    if let Some(id) = self.find_node(&NodeType::TacticPrimitive("Symmetry".into()))
                    {
                        active.insert(id);
                    }
                }
                crate::verifier::kernel::Tactic::Reflexivity => {
                    if let Some(id) =
                        self.find_node(&NodeType::TacticPrimitive("Reflexivity".into()))
                    {
                        active.insert(id);
                    }
                }
                _ => {}
            }
        }

        if active.is_empty() {
            // Default to variable and identity nodes
            if let Some(id) = self.find_node(&NodeType::Variable) {
                active.insert(id);
            }
        }

        // 3. BFS expansion up to k-hops
        let mut visited = active.clone();
        let mut queue: VecDeque<(usize, usize)> = active.iter().map(|&id| (id, 0)).collect();

        while let Some((curr, depth)) = queue.pop_front() {
            if depth >= k_hops {
                continue;
            }
            if let Some(neighbors) = self.adjacency.get(&curr) {
                for &(next, _, _) in neighbors {
                    if visited.insert(next) {
                        queue.push_back((next, depth + 1));
                    }
                }
            }
        }

        visited
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::verifier::fol::Equality;

    #[test]
    fn test_master_graph_universe_creation() -> Result<(), Box<dyn std::error::Error>> {
        let g = MasterMathGraph::default_universe();
        assert!(g.nodes.len() >= 18);
        assert!(g.edges.len() >= 20);

        let dist_id = g
            .find_node(&NodeType::AxiomProperty("Distributivity".into()))
            .ok_or("Distributivity node not found")?;
        let mul_id = g
            .find_node(&NodeType::Operator("*".into()))
            .ok_or("Mul node not found")?;
        let and_id = g
            .find_node(&NodeType::Operator("&".into()))
            .ok_or("And node not found")?;

        // Check isomorphism connections
        let mul_neighbors: Vec<usize> = g.adjacency[&mul_id].iter().map(|(id, _, _)| *id).collect();
        let and_neighbors: Vec<usize> = g.adjacency[&and_id].iter().map(|(id, _, _)| *id).collect();

        assert!(mul_neighbors.contains(&dist_id));
        assert!(and_neighbors.contains(&dist_id));
        Ok(())
    }

    #[test]
    fn test_sparse_subgraph_activation_calculus() -> Result<(), Box<dyn std::error::Error>> {
        let g = MasterMathGraph::default_universe();
        let u = Term::var("u");
        let v = Term::var("v");
        // D(u * v) = D(u)*v + u*D(v)
        let lhs = Term::func("D", vec![Term::func("*", vec![u.clone(), v.clone()])]);
        let rhs = Term::func(
            "+",
            vec![
                Term::func("*", vec![Term::func("D", vec![u]), v.clone()]),
                Term::func("*", vec![Term::var("u"), Term::func("D", vec![v])]),
            ],
        );
        let state = ProofState::new(Equality::new(lhs, rhs));

        let active = g.active_subgraph_nodes(&state, 1);

        let diff_id = g
            .find_node(&NodeType::Operator("D".into()))
            .ok_or("Diff node not found")?;
        let mul_id = g
            .find_node(&NodeType::Operator("*".into()))
            .ok_or("Mul node not found")?;
        let add_id = g
            .find_node(&NodeType::Operator("+".into()))
            .ok_or("Add node not found")?;
        let not_id = g
            .find_node(&NodeType::Operator("!".into()))
            .ok_or("Not node not found")?;

        // Calculus and algebraic operations must be active
        assert!(active.contains(&diff_id));
        assert!(active.contains(&mul_id));
        assert!(active.contains(&add_id));

        // Boolean negation must remain dormant
        assert!(!active.contains(&not_id));
        Ok(())
    }
}
