use super::fol::{apply_rewrite, Equality, Term};
use serde::{Deserialize, Serialize};
use std::fmt;

/// An applied tactic / transformation step in the proof
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Tactic {
    RewriteLhs(String),      // Rule name
    RewriteRhs(String),      // Rule name
    Symmetry,                // A = B -> B = A
    Reflexivity,             // A = A is trivially true (Q.E.D.)
    Transitivity(Term),      // A = B via intermediate term C
    ApplyAxiom(String),      // Apply a known axiom
}

impl fmt::Display for Tactic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Tactic::RewriteLhs(r) => write!(f, "rw_lhs [{}]", r),
            Tactic::RewriteRhs(r) => write!(f, "rw_rhs [{}]", r),
            Tactic::Symmetry => write!(f, "symm"),
            Tactic::Reflexivity => write!(f, "rfl"),
            Tactic::Transitivity(c) => write!(f, "trans ({})", c),
            Tactic::ApplyAxiom(a) => write!(f, "apply {}", a),
        }
    }
}

/// A formal mathematical goal
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Goal {
    pub id: usize,
    pub equality: Equality,
}

impl fmt::Display for Goal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Goal #{}: {}", self.id, self.equality)
    }
}

/// The state of a mathematical proof at any point in the search tree
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProofState {
    pub open_goals: Vec<Goal>,
    pub proof_history: Vec<(Tactic, String)>, // (tactic, result_description)
    pub is_solved: bool,
    pub depth: usize,
}

impl ProofState {
    pub fn new(initial_equality: Equality) -> Self {
        Self {
            open_goals: vec![Goal {
                id: 1,
                equality: initial_equality,
            }],
            proof_history: Vec::new(),
            is_solved: false,
            depth: 0,
        }
    }

    /// Checks if all goals are solved
    pub fn check_solved(&mut self) -> bool {
        if self.open_goals.is_empty() {
            self.is_solved = true;
            true
        } else {
            // Check if any open goal is trivially reflexive (A = A)
            self.open_goals.retain(|g| g.equality.lhs != g.equality.rhs);
            self.is_solved = self.open_goals.is_empty();
            self.is_solved
        }
    }
}

/// Mathematical Domains supported by Axiomatic Engine
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MathDomain {
    AbstractAlgebra,
    BooleanLogic,
    SymbolicCalculus,
    SetTheory,
    Unified,
}

impl MathDomain {
    pub fn name(&self) -> &'static str {
        match self {
            MathDomain::AbstractAlgebra => "Abstract Algebra",
            MathDomain::BooleanLogic => "Boolean Logic",
            MathDomain::SymbolicCalculus => "Symbolic Calculus",
            MathDomain::SetTheory => "Set Theory",
            MathDomain::Unified => "Unified Multidomain",
        }
    }
}

/// A collection of formal axioms and established lemmas
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AxiomLibrary {
    pub rules: Vec<(String, Equality)>, // (Name, Rule)
}

impl AxiomLibrary {
    pub fn empty() -> Self {
        Self { rules: Vec::new() }
    }

    pub fn for_domain(domain: MathDomain) -> Self {
        match domain {
            MathDomain::AbstractAlgebra => Self::standard_algebra(),
            MathDomain::BooleanLogic => Self::boolean_logic(),
            MathDomain::SymbolicCalculus => Self::symbolic_calculus(),
            MathDomain::SetTheory => Self::set_theory(),
            MathDomain::Unified => Self::unified_multidomain(),
        }
    }

    /// Standard Abstract Algebra (Group / Monoid / Ring) Axioms
    pub fn standard_algebra() -> Self {
        let x = Term::var("x");
        let y = Term::var("y");
        let z = Term::var("z");
        let zero = Term::constant("0");
        let one = Term::constant("1");

        let mut lib = Self::empty();

        // Additive Monoid / Group
        lib.add_rule(
            "add_zero",
            Equality::new(Term::func("+", vec![x.clone(), zero.clone()]), x.clone()),
        );
        lib.add_rule(
            "zero_add",
            Equality::new(Term::func("+", vec![zero.clone(), x.clone()]), x.clone()),
        );
        lib.add_rule(
            "add_comm",
            Equality::new(
                Term::func("+", vec![x.clone(), y.clone()]),
                Term::func("+", vec![y.clone(), x.clone()]),
            ),
        );
        lib.add_rule(
            "add_assoc",
            Equality::new(
                Term::func("+", vec![Term::func("+", vec![x.clone(), y.clone()]), z.clone()]),
                Term::func("+", vec![x.clone(), Term::func("+", vec![y.clone(), z.clone()])]),
            ),
        );
        lib.add_rule(
            "add_inv",
            Equality::new(
                Term::func("+", vec![x.clone(), Term::func("-", vec![x.clone()])]),
                zero.clone(),
            ),
        );

        // Multiplicative Semigroup / Ring
        lib.add_rule(
            "mul_one",
            Equality::new(Term::func("*", vec![x.clone(), one.clone()]), x.clone()),
        );
        lib.add_rule(
            "one_mul",
            Equality::new(Term::func("*", vec![one.clone(), x.clone()]), x.clone()),
        );
        lib.add_rule(
            "mul_zero",
            Equality::new(Term::func("*", vec![x.clone(), zero.clone()]), zero.clone()),
        );
        lib.add_rule(
            "mul_comm",
            Equality::new(
                Term::func("*", vec![x.clone(), y.clone()]),
                Term::func("*", vec![y.clone(), x.clone()]),
            ),
        );
        lib.add_rule(
            "mul_assoc",
            Equality::new(
                Term::func("*", vec![Term::func("*", vec![x.clone(), y.clone()]), z.clone()]),
                Term::func("*", vec![x.clone(), Term::func("*", vec![y.clone(), z.clone()])]),
            ),
        );
        lib.add_rule(
            "distrib_left",
            Equality::new(
                Term::func("*", vec![x.clone(), Term::func("+", vec![y.clone(), z.clone()])]),
                Term::func(
                    "+",
                    vec![
                        Term::func("*", vec![x.clone(), y.clone()]),
                        Term::func("*", vec![x.clone(), z.clone()]),
                    ],
                ),
            ),
        );

        lib
    }

    /// Propositional Calculus & Boolean Algebra Axioms
    pub fn boolean_logic() -> Self {
        let a = Term::var("a");
        let b = Term::var("b");
        let c = Term::var("c");
        let zero = Term::constant("0");
        let one = Term::constant("1");

        let mut lib = Self::empty();

        lib.add_rule("and_true", Equality::new(Term::func("&", vec![a.clone(), one.clone()]), a.clone()));
        lib.add_rule("true_and", Equality::new(Term::func("&", vec![one.clone(), a.clone()]), a.clone()));
        lib.add_rule("or_false", Equality::new(Term::func("|", vec![a.clone(), zero.clone()]), a.clone()));
        lib.add_rule("false_or", Equality::new(Term::func("|", vec![zero.clone(), a.clone()]), a.clone()));
        lib.add_rule("and_false", Equality::new(Term::func("&", vec![a.clone(), zero.clone()]), zero.clone()));
        lib.add_rule("or_true", Equality::new(Term::func("|", vec![a.clone(), one.clone()]), one.clone()));
        lib.add_rule("and_comm", Equality::new(Term::func("&", vec![a.clone(), b.clone()]), Term::func("&", vec![b.clone(), a.clone()])));
        lib.add_rule("or_comm", Equality::new(Term::func("|", vec![a.clone(), b.clone()]), Term::func("|", vec![b.clone(), a.clone()])));
        lib.add_rule("and_assoc", Equality::new(
            Term::func("&", vec![Term::func("&", vec![a.clone(), b.clone()]), c.clone()]),
            Term::func("&", vec![a.clone(), Term::func("&", vec![b.clone(), c.clone()])]),
        ));
        lib.add_rule("or_assoc", Equality::new(
            Term::func("|", vec![Term::func("|", vec![a.clone(), b.clone()]), c.clone()]),
            Term::func("|", vec![a.clone(), Term::func("|", vec![b.clone(), c.clone()])]),
        ));
        lib.add_rule("not_not", Equality::new(Term::func("!", vec![Term::func("!", vec![a.clone()])]), a.clone()));
        lib.add_rule("and_not", Equality::new(Term::func("&", vec![a.clone(), Term::func("!", vec![a.clone()])]), zero.clone()));
        lib.add_rule("or_not", Equality::new(Term::func("|", vec![a.clone(), Term::func("!", vec![a.clone()])]), one.clone()));
        lib.add_rule("de_morgan_and", Equality::new(
            Term::func("!", vec![Term::func("&", vec![a.clone(), b.clone()])]),
            Term::func("|", vec![Term::func("!", vec![a.clone()]), Term::func("!", vec![b.clone()])]),
        ));
        lib.add_rule("de_morgan_or", Equality::new(
            Term::func("!", vec![Term::func("|", vec![a.clone(), b.clone()])]),
            Term::func("&", vec![Term::func("!", vec![a.clone()]), Term::func("!", vec![b.clone()])]),
        ));

        lib
    }

    /// Elementary Calculus & Symbolic Differentiation Axioms
    pub fn symbolic_calculus() -> Self {
        let u = Term::var("u");
        let v = Term::var("v");
        let zero = Term::constant("0");
        let one = Term::constant("1");

        let mut lib = Self::empty();

        lib.add_rule("d_const_0", Equality::new(Term::func("D", vec![zero.clone()]), zero.clone()));
        lib.add_rule("d_const_1", Equality::new(Term::func("D", vec![one.clone()]), zero.clone()));
        lib.add_rule("d_var", Equality::new(Term::func("D", vec![Term::constant("x")]), one.clone()));
        lib.add_rule("d_sum", Equality::new(
            Term::func("D", vec![Term::func("+", vec![u.clone(), v.clone()])]),
            Term::func("+", vec![Term::func("D", vec![u.clone()]), Term::func("D", vec![v.clone()])]),
        ));
        lib.add_rule("d_prod", Equality::new(
            Term::func("D", vec![Term::func("*", vec![u.clone(), v.clone()])]),
            Term::func("+", vec![
                Term::func("*", vec![Term::func("D", vec![u.clone()]), v.clone()]),
                Term::func("*", vec![u.clone(), Term::func("D", vec![v.clone()])]),
            ]),
        ));
        lib.add_rule("add_zero", Equality::new(Term::func("+", vec![u.clone(), zero.clone()]), u.clone()));
        lib.add_rule("zero_add", Equality::new(Term::func("+", vec![zero.clone(), u.clone()]), u.clone()));
        lib.add_rule("mul_one", Equality::new(Term::func("*", vec![u.clone(), one.clone()]), u.clone()));
        lib.add_rule("one_mul", Equality::new(Term::func("*", vec![one.clone(), u.clone()]), u.clone()));
        lib.add_rule("mul_zero", Equality::new(Term::func("*", vec![u.clone(), zero.clone()]), zero.clone()));
        lib.add_rule("zero_mul", Equality::new(Term::func("*", vec![zero.clone(), u.clone()]), zero.clone()));

        lib
    }

    /// Set Theory Axioms
    pub fn set_theory() -> Self {
        let a = Term::var("a");
        let b = Term::var("b");
        let c = Term::var("c");
        let empty = Term::constant("0");
        let univ = Term::constant("U");

        let mut lib = Self::empty();

        lib.add_rule("inter_univ", Equality::new(Term::func("inter", vec![a.clone(), univ.clone()]), a.clone()));
        lib.add_rule("union_empty", Equality::new(Term::func("union", vec![a.clone(), empty.clone()]), a.clone()));
        lib.add_rule("inter_empty", Equality::new(Term::func("inter", vec![a.clone(), empty.clone()]), empty.clone()));
        lib.add_rule("union_univ", Equality::new(Term::func("union", vec![a.clone(), univ.clone()]), univ.clone()));
        lib.add_rule("inter_comm", Equality::new(Term::func("inter", vec![a.clone(), b.clone()]), Term::func("inter", vec![b.clone(), a.clone()])));
        lib.add_rule("union_comm", Equality::new(Term::func("union", vec![a.clone(), b.clone()]), Term::func("union", vec![b.clone(), a.clone()])));
        lib.add_rule("comp_comp", Equality::new(Term::func("comp", vec![Term::func("comp", vec![a.clone()])]), a.clone()));
        lib.add_rule("inter_comp", Equality::new(Term::func("inter", vec![a.clone(), Term::func("comp", vec![a.clone()])]), empty.clone()));
        lib.add_rule("union_comp", Equality::new(Term::func("union", vec![a.clone(), Term::func("comp", vec![a.clone()])]), univ.clone()));
        lib.add_rule("de_morgan_inter", Equality::new(
            Term::func("comp", vec![Term::func("inter", vec![a.clone(), b.clone()])]),
            Term::func("union", vec![Term::func("comp", vec![a.clone()]), Term::func("comp", vec![b.clone()])]),
        ));
        lib.add_rule("de_morgan_union", Equality::new(
            Term::func("comp", vec![Term::func("union", vec![a.clone(), b.clone()])]),
            Term::func("inter", vec![Term::func("comp", vec![a.clone()]), Term::func("comp", vec![b.clone()])]),
        ));

        lib
    }

    /// Unified Multi-Domain Library
    pub fn unified_multidomain() -> Self {
        let mut lib = Self::standard_algebra();
        for (name, rule) in Self::boolean_logic().rules {
            if !lib.rules.iter().any(|(n, _)| n == &name) {
                lib.add_rule(&name, rule);
            }
        }
        for (name, rule) in Self::symbolic_calculus().rules {
            if !lib.rules.iter().any(|(n, _)| n == &name) {
                lib.add_rule(&name, rule);
            }
        }
        for (name, rule) in Self::set_theory().rules {
            if !lib.rules.iter().any(|(n, _)| n == &name) {
                lib.add_rule(&name, rule);
            }
        }
        lib
    }

    pub fn add_rule(&mut self, name: &str, rule: Equality) {
        self.rules.push((name.to_string(), rule));
    }
}

/// The Infallible Formal Verifier Kernel
pub struct FormalVerifier;

impl FormalVerifier {
    /// Applies a tactic to a proof state and returns valid successor states
    pub fn apply_tactic(
        state: &ProofState,
        tactic: &Tactic,
        axioms: &AxiomLibrary,
    ) -> Result<ProofState, &'static str> {
        if state.open_goals.is_empty() {
            return Ok(state.clone());
        }

        let mut next_state = state.clone();
        let target_goal = next_state.open_goals.remove(0);

        match tactic {
            Tactic::Reflexivity => {
                if target_goal.equality.lhs == target_goal.equality.rhs {
                    next_state.proof_history.push((
                        Tactic::Reflexivity,
                        format!("Solved #{}: {} via rfl", target_goal.id, target_goal.equality),
                    ));
                    next_state.check_solved();
                    Ok(next_state)
                } else {
                    Err("Reflexivity failed: LHS does not match RHS")
                }
            }
            Tactic::Symmetry => {
                let flipped = target_goal.equality.flip();
                next_state.open_goals.insert(
                    0,
                    Goal {
                        id: target_goal.id,
                        equality: flipped.clone(),
                    },
                );
                next_state.proof_history.push((
                    Tactic::Symmetry,
                    format!("Applied symm: {}", flipped),
                ));
                next_state.depth += 1;
                Ok(next_state)
            }
            Tactic::RewriteLhs(rule_name) => {
                if let Some((_, rule)) = axioms.rules.iter().find(|(n, _)| n == rule_name) {
                    let rewrites = apply_rewrite(&target_goal.equality.lhs, rule);
                    if let Some(new_lhs) = rewrites.into_iter().next() {
                        let new_eq = Equality::new(new_lhs, target_goal.equality.rhs.clone());
                        next_state.open_goals.insert(
                            0,
                            Goal {
                                id: target_goal.id,
                                equality: new_eq.clone(),
                            },
                        );
                        next_state.proof_history.push((
                            Tactic::RewriteLhs(rule_name.clone()),
                            format!("Rewrote LHS via [{}]: {}", rule_name, new_eq),
                        ));
                        next_state.depth += 1;
                        next_state.check_solved();
                        Ok(next_state)
                    } else {
                        Err("Rewrite rule did not match LHS")
                    }
                } else {
                    Err("Unknown rewrite rule")
                }
            }
            Tactic::RewriteRhs(rule_name) => {
                if let Some((_, rule)) = axioms.rules.iter().find(|(n, _)| n == rule_name) {
                    let rewrites = apply_rewrite(&target_goal.equality.rhs, rule);
                    if let Some(new_rhs) = rewrites.into_iter().next() {
                        let new_eq = Equality::new(target_goal.equality.lhs.clone(), new_rhs);
                        next_state.open_goals.insert(
                            0,
                            Goal {
                                id: target_goal.id,
                                equality: new_eq.clone(),
                            },
                        );
                        next_state.proof_history.push((
                            Tactic::RewriteRhs(rule_name.clone()),
                            format!("Rewrote RHS via [{}]: {}", rule_name, new_eq),
                        ));
                        next_state.depth += 1;
                        next_state.check_solved();
                        Ok(next_state)
                    } else {
                        Err("Rewrite rule did not match RHS")
                    }
                } else {
                    Err("Unknown rewrite rule")
                }
            }
            Tactic::Transitivity(_) => Err("Transitivity unsupported in minimal verifier"),
            Tactic::ApplyAxiom(_) => Err("Direct axiom application unsupported in minimal verifier"),
        }
    }

    /// Generates all formally valid successor states from the current proof state
    pub fn expand_valid_transitions(
        state: &ProofState,
        axioms: &AxiomLibrary,
    ) -> Vec<(Tactic, ProofState)> {
        let mut successors = Vec::new();

        // 1. Try Reflexivity
        if let Ok(next) = Self::apply_tactic(state, &Tactic::Reflexivity, axioms) {
            successors.push((Tactic::Reflexivity, next));
            return successors; // Immediate Q.E.D.
        }

        // 2. Try Symmetry (if not applied immediately before)
        if !state.proof_history.iter().rev().take(1).any(|(t, _)| matches!(t, Tactic::Symmetry)) {
            if let Ok(next) = Self::apply_tactic(state, &Tactic::Symmetry, axioms) {
                successors.push((Tactic::Symmetry, next));
            }
        }

        // 3. Try Rewriting with all available axioms across all matching subterms
        if let Some(target_goal) = state.open_goals.first() {
            for (name, rule) in &axioms.rules {
                for new_lhs in apply_rewrite(&target_goal.equality.lhs, rule) {
                    let mut next_state = state.clone();
                    next_state.open_goals.remove(0);
                    let new_eq = Equality::new(new_lhs, target_goal.equality.rhs.clone());
                    next_state.open_goals.insert(
                        0,
                        Goal {
                            id: target_goal.id,
                            equality: new_eq.clone(),
                        },
                    );
                    next_state.proof_history.push((
                        Tactic::RewriteLhs(name.clone()),
                        format!("Rewrote LHS via [{}]: {}", name, new_eq),
                    ));
                    next_state.depth += 1;
                    next_state.check_solved();
                    successors.push((Tactic::RewriteLhs(name.clone()), next_state));
                }

                for new_rhs in apply_rewrite(&target_goal.equality.rhs, rule) {
                    let mut next_state = state.clone();
                    next_state.open_goals.remove(0);
                    let new_eq = Equality::new(target_goal.equality.lhs.clone(), new_rhs);
                    next_state.open_goals.insert(
                        0,
                        Goal {
                            id: target_goal.id,
                            equality: new_eq.clone(),
                        },
                    );
                    next_state.proof_history.push((
                        Tactic::RewriteRhs(name.clone()),
                        format!("Rewrote RHS via [{}]: {}", name, new_eq),
                    ));
                    next_state.depth += 1;
                    next_state.check_solved();
                    successors.push((Tactic::RewriteRhs(name.clone()), next_state));
                }
            }
        }

        successors
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prove_commutativity_identity() {
        let axioms = AxiomLibrary::standard_algebra();

        // Goal: a + 0 = 0 + a
        let a = Term::constant("a");
        let zero = Term::constant("0");
        let goal_eq = Equality::new(
            Term::func("+", vec![a.clone(), zero.clone()]),
            Term::func("+", vec![zero.clone(), a.clone()]),
        );

        let initial_state = ProofState::new(goal_eq);
        assert!(!initial_state.is_solved);

        // Step 1: rw_lhs [add_zero] -> a = 0 + a
        let step1 = FormalVerifier::apply_tactic(
            &initial_state,
            &Tactic::RewriteLhs("add_zero".to_string()),
            &axioms,
        )
        .expect("Step 1 valid");

        // Step 2: rw_rhs [zero_add] -> a = a (automatically recognized as reflexive & solved)
        let step2 = FormalVerifier::apply_tactic(
            &step1,
            &Tactic::RewriteRhs("zero_add".to_string()),
            &axioms,
        )
        .expect("Step 2 valid");

        assert!(step2.is_solved, "Proof must be certified complete upon reaching reflexivity");
        assert_eq!(step2.proof_history.len(), 2);
    }
}
