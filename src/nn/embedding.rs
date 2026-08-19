use crate::verifier::fol::Term;
use crate::verifier::kernel::ProofState;

/// Dimension of the mathematical feature embedding vector
pub const EMBEDDING_DIM: usize = 32;

/// Vectorizes a ProofState and its AST expressions into a fixed-size float vector
pub fn vectorize_proof_state(state: &ProofState) -> Vec<f64> {
    let mut vec = vec![0.0; EMBEDDING_DIM];

    if state.is_solved || state.open_goals.is_empty() {
        vec[0] = 1.0; // Is solved indicator
        return vec;
    }

    let goal = &state.open_goals[0];
    let lhs = &goal.equality.lhs;
    let rhs = &goal.equality.rhs;

    // Feature 0: Solved flag
    vec[0] = 0.0;

    // Feature 1: Current proof depth
    vec[1] = (state.depth as f64) / 10.0;

    // Feature 2: Open goals count
    vec[2] = (state.open_goals.len() as f64) / 5.0;

    // Feature 3-4: LHS & RHS Tree Depth
    let (lhs_depth, lhs_nodes) = term_stats(lhs);
    let (rhs_depth, rhs_nodes) = term_stats(rhs);
    vec[3] = (lhs_depth as f64) / 8.0;
    vec[4] = (rhs_depth as f64) / 8.0;

    // Feature 5-6: LHS & RHS Total Node Count
    vec[5] = (lhs_nodes as f64) / 20.0;
    vec[6] = (rhs_nodes as f64) / 20.0;

    // Feature 7: Structural asymmetry
    vec[7] = ((lhs_nodes as f64 - rhs_nodes as f64).abs()) / 10.0;

    // Feature 8-15: Operator & Constant Frequencies in LHS
    extract_symbol_frequencies(lhs, &mut vec[8..16]);

    // Feature 16-23: Operator & Constant Frequencies in RHS
    extract_symbol_frequencies(rhs, &mut vec[16..24]);

    // Feature 24-31: Applied tactic history counts
    for (tactic, _) in &state.proof_history {
        match tactic {
            crate::verifier::kernel::Tactic::RewriteLhs(_) => vec[24] += 0.2,
            crate::verifier::kernel::Tactic::RewriteRhs(_) => vec[25] += 0.2,
            crate::verifier::kernel::Tactic::Symmetry => vec[26] += 0.2,
            crate::verifier::kernel::Tactic::Reflexivity => vec[27] += 0.2,
            _ => vec[28] += 0.2,
        }
    }

    vec
}

fn term_stats(term: &Term) -> (usize, usize) {
    match term {
        Term::Var(_) | Term::Const(_) => (1, 1),
        Term::Func(_, args) => {
            let mut max_depth = 0;
            let mut total_nodes = 1;
            for arg in args {
                let (d, n) = term_stats(arg);
                max_depth = max_depth.max(d);
                total_nodes += n;
            }
            (max_depth + 1, total_nodes)
        }
    }
}

fn extract_symbol_frequencies(term: &Term, slice: &mut [f64]) {
    match term {
        Term::Var(_) => slice[0] += 0.2,
        Term::Const(c) => {
            if c == "0" {
                slice[1] += 0.3;
            } else if c == "1" {
                slice[2] += 0.3;
            } else {
                slice[3] += 0.2;
            }
        }
        Term::Func(name, args) => {
            if name == "+" {
                slice[4] += 0.3;
            } else if name == "*" {
                slice[5] += 0.3;
            } else if name == "-" {
                slice[6] += 0.3;
            } else {
                slice[7] += 0.2;
            }
            for arg in args {
                extract_symbol_frequencies(arg, slice);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::verifier::fol::Equality;

    #[test]
    fn test_vectorize_state_shape() {
        let x = Term::var("x");
        let zero = Term::constant("0");
        let state = ProofState::new(Equality::new(x, zero));

        let vec = vectorize_proof_state(&state);
        assert_eq!(vec.len(), EMBEDDING_DIM);
        assert!(vec.iter().any(|&v| v > 0.0));
    }
}
