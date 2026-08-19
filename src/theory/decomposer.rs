use crate::verifier::fol::{Equality, Term};
use rand::Rng;

/// Target Goal Decomposer & Subproblem Synthesizer for Directed Neurosymbolic Training
pub struct GoalDecomposer;

impl GoalDecomposer {
    /// Extracts all non-trivial subterms present within a term
    pub fn extract_subterms(term: &Term) -> Vec<Term> {
        let mut subterms = Vec::new();
        Self::collect_subterms_recursive(term, &mut subterms);
        subterms.sort_by_key(|t| t.to_string());
        subterms.dedup();
        subterms
    }

    fn collect_subterms_recursive(term: &Term, acc: &mut Vec<Term>) {
        match term {
            Term::Var(_) | Term::Const(_) => {}
            Term::Func(_, args) => {
                acc.push(term.clone());
                for arg in args {
                    Self::collect_subterms_recursive(arg, acc);
                }
            }
        }
    }

    /// Synthesizes targeted training subproblems directed toward mastering the target goal
    pub fn generate_subproblems(target: &Equality, count: usize) -> Vec<Equality> {
        let mut subproblems = Vec::new();
        let lhs_subterms = Self::extract_subterms(&target.lhs);
        let rhs_subterms = Self::extract_subterms(&target.rhs);
        let mut all_subterms = lhs_subterms;
        all_subterms.extend(rhs_subterms);
        all_subterms.sort_by_key(|t| t.to_string());
        all_subterms.dedup();

        let zero = Term::constant("0");
        let one = Term::constant("1");
        let mut rng = rand::thread_rng();

        // 1. Direct identities on extracted subterms
        for sub in &all_subterms {
            // Identity: (sub + 0) = sub
            subproblems.push(Equality::new(
                Term::func("+", vec![sub.clone(), zero.clone()]),
                sub.clone(),
            ));
            // Identity: (0 + sub) = sub
            subproblems.push(Equality::new(
                Term::func("+", vec![zero.clone(), sub.clone()]),
                sub.clone(),
            ));
            // Identity: (sub * 1) = sub
            subproblems.push(Equality::new(
                Term::func("*", vec![sub.clone(), one.clone()]),
                sub.clone(),
            ));
            // Identity: (1 * sub) = sub
            subproblems.push(Equality::new(
                Term::func("*", vec![one.clone(), sub.clone()]),
                sub.clone(),
            ));
            // Inverse if addition: (sub + (-sub)) = 0
            subproblems.push(Equality::new(
                Term::func("+", vec![sub.clone(), Term::func("-", vec![sub.clone()])]),
                zero.clone(),
            ));
        }

        // 2. Subterm commutativity & associativity permutations
        if all_subterms.len() >= 2 {
            for i in 0..all_subterms.len() {
                for j in (i + 1)..all_subterms.len() {
                    let t1 = &all_subterms[i];
                    let t2 = &all_subterms[j];
                    subproblems.push(Equality::new(
                        Term::func("+", vec![t1.clone(), t2.clone()]),
                        Term::func("+", vec![t2.clone(), t1.clone()]),
                    ));
                    subproblems.push(Equality::new(
                        Term::func("*", vec![t1.clone(), t2.clone()]),
                        Term::func("*", vec![t2.clone(), t1.clone()]),
                    ));
                }
            }
        }

        // 3. Always include the target goal itself and symmetry variant
        subproblems.push(target.clone());
        subproblems.push(target.flip());

        // Fill up to count if needed by sampling / combining
        while subproblems.len() < count {
            let idx = rng.gen_range(0..subproblems.len());
            let base = subproblems[idx].clone();
            let perturb = if rng.gen_bool(0.5) {
                Equality::new(Term::func("+", vec![base.lhs, zero.clone()]), base.rhs)
            } else {
                Equality::new(base.lhs, Term::func("*", vec![base.rhs, one.clone()]))
            };
            subproblems.push(perturb);
        }

        subproblems.truncate(count);
        subproblems
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_goal_decomposer_extracts_subterms() {
        let x = Term::constant("x");
        let y = Term::constant("y");
        let zero = Term::constant("0");
        let one = Term::constant("1");

        // ((x + 0) + (y * 1)) = (y + x)
        let eq = Equality::new(
            Term::func(
                "+",
                vec![
                    Term::func("+", vec![x.clone(), zero.clone()]),
                    Term::func("*", vec![y.clone(), one.clone()]),
                ],
            ),
            Term::func("+", vec![y.clone(), x.clone()]),
        );

        let subterms = GoalDecomposer::extract_subterms(&eq.lhs);
        assert!(!subterms.is_empty());
        assert!(subterms.iter().any(|t| t.to_string().contains("+")));

        let subproblems = GoalDecomposer::generate_subproblems(&eq, 10);
        assert_eq!(subproblems.len(), 10);
    }
}
