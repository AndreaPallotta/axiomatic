use super::fol::{apply_rewrite, Equality, Term};
use serde::{Deserialize, Serialize};
use std::fmt;

/// An applied tactic / transformation step in the proof
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Tactic {
    RewriteLhs(String), // Rule name
    RewriteRhs(String), // Rule name
    EvalArithmeticLhs,
    EvalArithmeticRhs,
    Symmetry,           // A = B -> B = A
    Reflexivity,        // A = A is trivially true (Q.E.D.)
    Transitivity(Term), // A = B via intermediate term C
    ApplyAxiom(String), // Apply a known axiom
    TransposeToZero,
    ZeroProductSplit,
}

impl fmt::Display for Tactic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Tactic::RewriteLhs(r) => write!(f, "rw_lhs [{}]", r),
            Tactic::RewriteRhs(r) => write!(f, "rw_rhs [{}]", r),
            Tactic::EvalArithmeticLhs => write!(f, "eval_arith_lhs"),
            Tactic::EvalArithmeticRhs => write!(f, "eval_arith_rhs"),
            Tactic::Symmetry => write!(f, "symm"),
            Tactic::Reflexivity => write!(f, "rfl"),
            Tactic::Transitivity(c) => write!(f, "trans ({})", c),
            Tactic::ApplyAxiom(a) => write!(f, "apply {}", a),
            Tactic::TransposeToZero => write!(f, "transpose_zero"),
            Tactic::ZeroProductSplit => write!(f, "zero_product_split"),
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
    #[serde(default)]
    pub initial_equality: Option<Equality>,
    pub open_goals: Vec<Goal>,
    pub proof_history: Vec<(Tactic, String)>,
    pub is_solved: bool,
    pub depth: usize,
}

impl ProofState {
    pub fn new(initial_equality: Equality) -> Self {
        Self {
            initial_equality: Some(initial_equality.clone()),
            open_goals: vec![Goal {
                id: 1,
                equality: initial_equality,
            }],
            proof_history: Vec::new(),
            is_solved: false,
            depth: 0,
        }
    }

    pub fn check_solved(&mut self) -> bool {
        if self.open_goals.is_empty() {
            self.is_solved = true;
            true
        } else {
            self.open_goals.retain(|g| g.equality.lhs != g.equality.rhs);
            self.is_solved = self.open_goals.is_empty();
            self.is_solved
        }
    }

    pub fn minimize(&mut self, axioms: &AxiomLibrary) {
        if let Some(ref initial) = self.initial_equality {
            if self.is_solved && !self.proof_history.is_empty() {
                self.proof_history =
                    FormalVerifier::prune_proof_history(initial, &self.proof_history, axioms);
                self.depth = self.proof_history.len();
            }
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
    ComplexNumbers,
    LinearAlgebra,
    OrderTheory,
    MultivariableCalculus,
    Probability,
    Unified,
}

impl MathDomain {
    pub fn name(&self) -> &'static str {
        match self {
            MathDomain::AbstractAlgebra => "Abstract Algebra",
            MathDomain::BooleanLogic => "Boolean Logic",
            MathDomain::SymbolicCalculus => "Symbolic Calculus",
            MathDomain::SetTheory => "Set Theory",
            MathDomain::ComplexNumbers => "Complex Numbers",
            MathDomain::LinearAlgebra => "Linear Algebra",
            MathDomain::OrderTheory => "Order Theory",
            MathDomain::MultivariableCalculus => "Multivariable Calculus",
            MathDomain::Probability => "Probability Theory",
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
            MathDomain::ComplexNumbers => Self::complex_numbers(),
            MathDomain::LinearAlgebra => Self::linear_algebra(),
            MathDomain::OrderTheory => Self::order_theory(),
            MathDomain::MultivariableCalculus => Self::multivariable_calculus(),
            MathDomain::Probability => Self::probability(),
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
                Term::func(
                    "+",
                    vec![Term::func("+", vec![x.clone(), y.clone()]), z.clone()],
                ),
                Term::func(
                    "+",
                    vec![x.clone(), Term::func("+", vec![y.clone(), z.clone()])],
                ),
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
                Term::func(
                    "*",
                    vec![Term::func("*", vec![x.clone(), y.clone()]), z.clone()],
                ),
                Term::func(
                    "*",
                    vec![x.clone(), Term::func("*", vec![y.clone(), z.clone()])],
                ),
            ),
        );
        lib.add_rule(
            "distrib_left",
            Equality::new(
                Term::func(
                    "*",
                    vec![x.clone(), Term::func("+", vec![y.clone(), z.clone()])],
                ),
                Term::func(
                    "+",
                    vec![
                        Term::func("*", vec![x.clone(), y.clone()]),
                        Term::func("*", vec![x.clone(), z.clone()]),
                    ],
                ),
            ),
        );
        lib.add_rule(
            "distrib_right",
            Equality::new(
                Term::func(
                    "*",
                    vec![Term::func("+", vec![x.clone(), y.clone()]), z.clone()],
                ),
                Term::func(
                    "+",
                    vec![
                        Term::func("*", vec![x.clone(), z.clone()]),
                        Term::func("*", vec![y.clone(), z.clone()]),
                    ],
                ),
            ),
        );
        lib.add_rule(
            "neg_neg",
            Equality::new(
                Term::func("-", vec![Term::func("-", vec![x.clone()])]),
                x.clone(),
            ),
        );
        lib.add_rule(
            "mul_neg_left",
            Equality::new(
                Term::func("*", vec![Term::func("-", vec![x.clone()]), y.clone()]),
                Term::func("-", vec![Term::func("*", vec![x.clone(), y.clone()])]),
            ),
        );
        lib.add_rule(
            "sub_self",
            Equality::new(
                Term::func("-", vec![x.clone(), x.clone()]),
                zero.clone(),
            ),
        );
        lib.add_rule(
            "diff_squares",
            Equality::new(
                Term::func(
                    "*",
                    vec![
                        Term::func("+", vec![x.clone(), Term::func("-", vec![y.clone()])]),
                        Term::func("+", vec![x.clone(), y.clone()]),
                    ],
                ),
                Term::func(
                    "+",
                    vec![
                        Term::func("*", vec![x.clone(), x.clone()]),
                        Term::func("-", vec![Term::func("*", vec![y.clone(), y.clone()])]),
                    ],
                ),
            ),
        );
        lib.add_rule(
            "factor_diff_squares",
            Equality::new(
                Term::func(
                    "+",
                    vec![
                        Term::func("*", vec![x.clone(), x.clone()]),
                        Term::func("-", vec![Term::func("*", vec![y.clone(), y.clone()])]),
                    ],
                ),
                Term::func(
                    "*",
                    vec![
                        Term::func("+", vec![x.clone(), Term::func("-", vec![y.clone()])]),
                        Term::func("+", vec![x.clone(), y.clone()]),
                    ],
                ),
            ),
        );
        lib.add_rule(
            "diff_cubes",
            Equality::new(
                Term::func(
                    "*",
                    vec![
                        Term::func("+", vec![x.clone(), Term::func("-", vec![y.clone()])]),
                        Term::func(
                            "+",
                            vec![
                                Term::func(
                                    "+",
                                    vec![
                                        Term::func("*", vec![x.clone(), x.clone()]),
                                        Term::func("*", vec![x.clone(), y.clone()]),
                                    ],
                                ),
                                Term::func("*", vec![y.clone(), y.clone()]),
                            ],
                        ),
                    ],
                ),
                Term::func(
                    "+",
                    vec![
                        Term::func(
                            "*",
                            vec![Term::func("*", vec![x.clone(), x.clone()]), x.clone()],
                        ),
                        Term::func(
                            "-",
                            vec![Term::func(
                                "*",
                                vec![Term::func("*", vec![y.clone(), y.clone()]), y.clone()],
                            )],
                        ),
                    ],
                ),
            ),
        );
        lib.add_rule(
            "factor_common_right",
            Equality::new(
                Term::func(
                    "+",
                    vec![
                        Term::func("*", vec![x.clone(), z.clone()]),
                        Term::func("*", vec![y.clone(), z.clone()]),
                    ],
                ),
                Term::func(
                    "*",
                    vec![Term::func("+", vec![x.clone(), y.clone()]), z.clone()],
                ),
            ),
        );
        lib.add_rule(
            "factor_common_left",
            Equality::new(
                Term::func(
                    "+",
                    vec![
                        Term::func("*", vec![z.clone(), x.clone()]),
                        Term::func("*", vec![z.clone(), y.clone()]),
                    ],
                ),
                Term::func(
                    "*",
                    vec![z.clone(), Term::func("+", vec![x.clone(), y.clone()])],
                ),
            ),
        );

        lib
    }

    /// Complex Numbers Field Axioms
    pub fn complex_numbers() -> Self {
        let mut lib = Self::standard_algebra();
        let i = Term::constant("i");
        let x = Term::var("x");
        let y = Term::var("y");
        let neg_one = Term::from_i64(-1);

        lib.add_rule(
            "i_squared",
            Equality::new(Term::func("*", vec![i.clone(), i.clone()]), neg_one.clone()),
        );
        lib.add_rule(
            "mul_i_i",
            Equality::new(
                Term::func(
                    "*",
                    vec![Term::func("*", vec![x.clone(), i.clone()]), i.clone()],
                ),
                Term::func("-", vec![x.clone()]),
            ),
        );
        lib.add_rule(
            "mul_imaginary",
            Equality::new(
                Term::func(
                    "*",
                    vec![
                        Term::func("*", vec![x.clone(), i.clone()]),
                        Term::func("*", vec![y.clone(), i.clone()]),
                    ],
                ),
                Term::func("-", vec![Term::func("*", vec![x.clone(), y.clone()])]),
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

        lib.add_rule(
            "and_true",
            Equality::new(Term::func("&", vec![a.clone(), one.clone()]), a.clone()),
        );
        lib.add_rule(
            "true_and",
            Equality::new(Term::func("&", vec![one.clone(), a.clone()]), a.clone()),
        );
        lib.add_rule(
            "or_false",
            Equality::new(Term::func("|", vec![a.clone(), zero.clone()]), a.clone()),
        );
        lib.add_rule(
            "false_or",
            Equality::new(Term::func("|", vec![zero.clone(), a.clone()]), a.clone()),
        );
        lib.add_rule(
            "and_false",
            Equality::new(Term::func("&", vec![a.clone(), zero.clone()]), zero.clone()),
        );
        lib.add_rule(
            "or_true",
            Equality::new(Term::func("|", vec![a.clone(), one.clone()]), one.clone()),
        );
        lib.add_rule(
            "and_comm",
            Equality::new(
                Term::func("&", vec![a.clone(), b.clone()]),
                Term::func("&", vec![b.clone(), a.clone()]),
            ),
        );
        lib.add_rule(
            "or_comm",
            Equality::new(
                Term::func("|", vec![a.clone(), b.clone()]),
                Term::func("|", vec![b.clone(), a.clone()]),
            ),
        );
        lib.add_rule(
            "and_assoc",
            Equality::new(
                Term::func(
                    "&",
                    vec![Term::func("&", vec![a.clone(), b.clone()]), c.clone()],
                ),
                Term::func(
                    "&",
                    vec![a.clone(), Term::func("&", vec![b.clone(), c.clone()])],
                ),
            ),
        );
        lib.add_rule(
            "or_assoc",
            Equality::new(
                Term::func(
                    "|",
                    vec![Term::func("|", vec![a.clone(), b.clone()]), c.clone()],
                ),
                Term::func(
                    "|",
                    vec![a.clone(), Term::func("|", vec![b.clone(), c.clone()])],
                ),
            ),
        );
        lib.add_rule(
            "not_not",
            Equality::new(
                Term::func("!", vec![Term::func("!", vec![a.clone()])]),
                a.clone(),
            ),
        );
        lib.add_rule(
            "and_not",
            Equality::new(
                Term::func("&", vec![a.clone(), Term::func("!", vec![a.clone()])]),
                zero.clone(),
            ),
        );
        lib.add_rule(
            "or_not",
            Equality::new(
                Term::func("|", vec![a.clone(), Term::func("!", vec![a.clone()])]),
                one.clone(),
            ),
        );
        lib.add_rule(
            "de_morgan_and",
            Equality::new(
                Term::func("!", vec![Term::func("&", vec![a.clone(), b.clone()])]),
                Term::func(
                    "|",
                    vec![
                        Term::func("!", vec![a.clone()]),
                        Term::func("!", vec![b.clone()]),
                    ],
                ),
            ),
        );
        lib.add_rule(
            "de_morgan_or",
            Equality::new(
                Term::func("!", vec![Term::func("|", vec![a.clone(), b.clone()])]),
                Term::func(
                    "&",
                    vec![
                        Term::func("!", vec![a.clone()]),
                        Term::func("!", vec![b.clone()]),
                    ],
                ),
            ),
        );

        lib
    }

    /// Elementary Calculus & Symbolic Differentiation Axioms
    pub fn symbolic_calculus() -> Self {
        let u = Term::var("u");
        let v = Term::var("v");
        let zero = Term::constant("0");
        let one = Term::constant("1");

        let mut lib = Self::empty();

        lib.add_rule(
            "d_const_0",
            Equality::new(Term::func("D", vec![zero.clone()]), zero.clone()),
        );
        lib.add_rule(
            "d_const_1",
            Equality::new(Term::func("D", vec![one.clone()]), zero.clone()),
        );
        lib.add_rule(
            "d_var",
            Equality::new(Term::func("D", vec![Term::constant("x")]), one.clone()),
        );
        lib.add_rule(
            "d_sum",
            Equality::new(
                Term::func("D", vec![Term::func("+", vec![u.clone(), v.clone()])]),
                Term::func(
                    "+",
                    vec![
                        Term::func("D", vec![u.clone()]),
                        Term::func("D", vec![v.clone()]),
                    ],
                ),
            ),
        );
        lib.add_rule(
            "d_prod",
            Equality::new(
                Term::func("D", vec![Term::func("*", vec![u.clone(), v.clone()])]),
                Term::func(
                    "+",
                    vec![
                        Term::func("*", vec![Term::func("D", vec![u.clone()]), v.clone()]),
                        Term::func("*", vec![u.clone(), Term::func("D", vec![v.clone()])]),
                    ],
                ),
            ),
        );
        lib.add_rule(
            "d_exp",
            Equality::new(
                Term::func("D", vec![Term::func("exp", vec![u.clone()])]),
                Term::func(
                    "*",
                    vec![
                        Term::func("exp", vec![u.clone()]),
                        Term::func("D", vec![u.clone()]),
                    ],
                ),
            ),
        );
        lib.add_rule(
            "d_sin",
            Equality::new(
                Term::func("D", vec![Term::func("sin", vec![u.clone()])]),
                Term::func(
                    "*",
                    vec![
                        Term::func("cos", vec![u.clone()]),
                        Term::func("D", vec![u.clone()]),
                    ],
                ),
            ),
        );
        lib.add_rule(
            "d_cos",
            Equality::new(
                Term::func("D", vec![Term::func("cos", vec![u.clone()])]),
                Term::func(
                    "-",
                    vec![Term::func(
                        "*",
                        vec![
                            Term::func("sin", vec![u.clone()]),
                            Term::func("D", vec![u.clone()]),
                        ],
                    )],
                ),
            ),
        );
        lib.add_rule(
            "d_quotient",
            Equality::new(
                Term::func("D", vec![Term::func("/", vec![u.clone(), v.clone()])]),
                Term::func(
                    "/",
                    vec![
                        Term::func(
                            "-",
                            vec![
                                Term::func(
                                    "*",
                                    vec![Term::func("D", vec![u.clone()]), v.clone()],
                                ),
                                Term::func(
                                    "*",
                                    vec![u.clone(), Term::func("D", vec![v.clone()])],
                                ),
                            ],
                        ),
                        Term::func("*", vec![v.clone(), v.clone()]),
                    ],
                ),
            ),
        );
        lib.add_rule(
            "add_zero",
            Equality::new(Term::func("+", vec![u.clone(), zero.clone()]), u.clone()),
        );
        lib.add_rule(
            "zero_add",
            Equality::new(Term::func("+", vec![zero.clone(), u.clone()]), u.clone()),
        );
        lib.add_rule(
            "mul_one",
            Equality::new(Term::func("*", vec![u.clone(), one.clone()]), u.clone()),
        );
        lib.add_rule(
            "one_mul",
            Equality::new(Term::func("*", vec![one.clone(), u.clone()]), u.clone()),
        );
        lib.add_rule(
            "mul_zero",
            Equality::new(Term::func("*", vec![u.clone(), zero.clone()]), zero.clone()),
        );
        lib.add_rule(
            "zero_mul",
            Equality::new(Term::func("*", vec![zero.clone(), u.clone()]), zero.clone()),
        );
        lib.add_rule(
            "sub_self",
            Equality::new(Term::func("-", vec![u.clone(), u.clone()]), zero.clone()),
        );
        lib.add_rule(
            "add_inv",
            Equality::new(
                Term::func("+", vec![u.clone(), Term::func("-", vec![u.clone()])]),
                zero.clone(),
            ),
        );
        lib.add_rule(
            "add_inv_left",
            Equality::new(
                Term::func("+", vec![Term::func("-", vec![u.clone()]), u.clone()]),
                zero.clone(),
            ),
        );
        lib.add_rule(
            "add_comm",
            Equality::new(
                Term::func("+", vec![u.clone(), v.clone()]),
                Term::func("+", vec![v.clone(), u.clone()]),
            ),
        );
        lib.add_rule(
            "d_neg",
            Equality::new(
                Term::func("D", vec![Term::func("-", vec![u.clone()])]),
                Term::func("-", vec![Term::func("D", vec![u.clone()])]),
            ),
        );
        lib.add_rule(
            "mul_neg_left",
            Equality::new(
                Term::func("*", vec![Term::func("-", vec![u.clone()]), v.clone()]),
                Term::func("-", vec![Term::func("*", vec![u.clone(), v.clone()])]),
            ),
        );
        lib.add_rule(
            "mul_neg_right",
            Equality::new(
                Term::func("*", vec![u.clone(), Term::func("-", vec![v.clone()])]),
                Term::func("-", vec![Term::func("*", vec![u.clone(), v.clone()])]),
            ),
        );
        lib.add_rule(
            "neg_neg",
            Equality::new(
                Term::func("-", vec![Term::func("-", vec![u.clone()])]),
                u.clone(),
            ),
        );

        // Fundamental Theorem of Calculus & Indefinite Integration
        lib.add_rule(
            "ftc",
            Equality::new(
                Term::func("D", vec![Term::func("Int", vec![u.clone()])]),
                u.clone(),
            ),
        );
        lib.add_rule(
            "ftc_reverse",
            Equality::new(
                Term::func("Int", vec![Term::func("D", vec![u.clone()])]),
                u.clone(),
            ),
        );
        lib.add_rule(
            "int_sum",
            Equality::new(
                Term::func("Int", vec![Term::func("+", vec![u.clone(), v.clone()])]),
                Term::func(
                    "+",
                    vec![
                        Term::func("Int", vec![u.clone()]),
                        Term::func("Int", vec![v.clone()]),
                    ],
                ),
            ),
        );
        lib.add_rule(
            "int_zero",
            Equality::new(Term::func("Int", vec![zero.clone()]), zero.clone()),
        );
        lib.add_rule(
            "int_one",
            Equality::new(Term::func("Int", vec![one.clone()]), Term::constant("x")),
        );
        lib.add_rule(
            "int_exp",
            Equality::new(
                Term::func("Int", vec![Term::func("exp", vec![Term::constant("x")])]),
                Term::func("exp", vec![Term::constant("x")]),
            ),
        );
        lib.add_rule(
            "int_cos",
            Equality::new(
                Term::func("Int", vec![Term::func("cos", vec![Term::constant("x")])]),
                Term::func("sin", vec![Term::constant("x")]),
            ),
        );
        lib.add_rule(
            "int_sin",
            Equality::new(
                Term::func("Int", vec![Term::func("sin", vec![Term::constant("x")])]),
                Term::func("-", vec![Term::func("cos", vec![Term::constant("x")])]),
            ),
        );

        // Hyperbolic Functions & Complex Euler Bridges
        lib.add_rule(
            "cosh_sq_sub_sinh_sq",
            Equality::new(
                Term::func(
                    "+",
                    vec![
                        Term::func("*", vec![Term::func("cosh", vec![u.clone()]), Term::func("cosh", vec![u.clone()])]),
                        Term::func("-", vec![Term::func("*", vec![Term::func("sinh", vec![u.clone()]), Term::func("sinh", vec![u.clone()])])]),
                    ],
                ),
                one.clone(),
            ),
        );
        lib.add_rule(
            "d_sinh",
            Equality::new(
                Term::func("D", vec![Term::func("sinh", vec![u.clone()])]),
                Term::func("*", vec![Term::func("cosh", vec![u.clone()]), Term::func("D", vec![u.clone()])]),
            ),
        );
        lib.add_rule(
            "d_cosh",
            Equality::new(
                Term::func("D", vec![Term::func("cosh", vec![u.clone()])]),
                Term::func("*", vec![Term::func("sinh", vec![u.clone()]), Term::func("D", vec![u.clone()])]),
            ),
        );
        lib.add_rule(
            "euler_cosh_i",
            Equality::new(
                Term::func("cosh", vec![Term::func("*", vec![Term::constant("i"), u.clone()])]),
                Term::func("cos", vec![u.clone()]),
            ),
        );
        lib.add_rule(
            "euler_sinh_i",
            Equality::new(
                Term::func("sinh", vec![Term::func("*", vec![Term::constant("i"), u.clone()])]),
                Term::func("*", vec![Term::constant("i"), Term::func("sin", vec![u.clone()])]),
            ),
        );
        lib.add_rule(
            "sinh_zero",
            Equality::new(Term::func("sinh", vec![zero.clone()]), zero.clone()),
        );
        lib.add_rule(
            "cosh_zero",
            Equality::new(Term::func("cosh", vec![zero.clone()]), one.clone()),
        );

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

        lib.add_rule(
            "inter_univ",
            Equality::new(
                Term::func("inter", vec![a.clone(), univ.clone()]),
                a.clone(),
            ),
        );
        lib.add_rule(
            "union_empty",
            Equality::new(
                Term::func("union", vec![a.clone(), empty.clone()]),
                a.clone(),
            ),
        );
        lib.add_rule(
            "inter_empty",
            Equality::new(
                Term::func("inter", vec![a.clone(), empty.clone()]),
                empty.clone(),
            ),
        );
        lib.add_rule(
            "union_univ",
            Equality::new(
                Term::func("union", vec![a.clone(), univ.clone()]),
                univ.clone(),
            ),
        );
        lib.add_rule(
            "inter_comm",
            Equality::new(
                Term::func("inter", vec![a.clone(), b.clone()]),
                Term::func("inter", vec![b.clone(), a.clone()]),
            ),
        );
        lib.add_rule(
            "union_comm",
            Equality::new(
                Term::func("union", vec![a.clone(), b.clone()]),
                Term::func("union", vec![b.clone(), a.clone()]),
            ),
        );
        lib.add_rule(
            "comp_comp",
            Equality::new(
                Term::func("comp", vec![Term::func("comp", vec![a.clone()])]),
                a.clone(),
            ),
        );
        lib.add_rule(
            "inter_comp",
            Equality::new(
                Term::func(
                    "inter",
                    vec![a.clone(), Term::func("comp", vec![a.clone()])],
                ),
                empty.clone(),
            ),
        );
        lib.add_rule(
            "union_comp",
            Equality::new(
                Term::func(
                    "union",
                    vec![a.clone(), Term::func("comp", vec![a.clone()])],
                ),
                univ.clone(),
            ),
        );
        lib.add_rule(
            "de_morgan_inter",
            Equality::new(
                Term::func(
                    "comp",
                    vec![Term::func("inter", vec![a.clone(), b.clone()])],
                ),
                Term::func(
                    "union",
                    vec![
                        Term::func("comp", vec![a.clone()]),
                        Term::func("comp", vec![b.clone()]),
                    ],
                ),
            ),
        );
        lib.add_rule(
            "de_morgan_union",
            Equality::new(
                Term::func(
                    "comp",
                    vec![Term::func("union", vec![a.clone(), b.clone()])],
                ),
                Term::func(
                    "inter",
                    vec![
                        Term::func("comp", vec![a.clone()]),
                        Term::func("comp", vec![b.clone()]),
                    ],
                ),
            ),
        );

        lib
    }

    /// Linear Algebra & Matrix Invariant Axioms
    pub fn linear_algebra() -> Self {
        let a = Term::var("A");
        let b = Term::var("B");
        let c = Term::var("C");
        let i = Term::constant("I");
        let zero = Term::constant("0");

        let mut lib = Self::empty();

        lib.add_rule(
            "mat_add_zero",
            Equality::new(Term::func("+", vec![a.clone(), zero.clone()]), a.clone()),
        );
        lib.add_rule(
            "mat_add_comm",
            Equality::new(
                Term::func("+", vec![a.clone(), b.clone()]),
                Term::func("+", vec![b.clone(), a.clone()]),
            ),
        );
        lib.add_rule(
            "mat_add_assoc",
            Equality::new(
                Term::func(
                    "+",
                    vec![Term::func("+", vec![a.clone(), b.clone()]), c.clone()],
                ),
                Term::func(
                    "+",
                    vec![a.clone(), Term::func("+", vec![b.clone(), c.clone()])],
                ),
            ),
        );
        lib.add_rule(
            "mat_mul_ident_right",
            Equality::new(Term::func("*", vec![a.clone(), i.clone()]), a.clone()),
        );
        lib.add_rule(
            "mat_mul_ident_left",
            Equality::new(Term::func("*", vec![i.clone(), a.clone()]), a.clone()),
        );
        lib.add_rule(
            "mat_mul_assoc",
            Equality::new(
                Term::func(
                    "*",
                    vec![Term::func("*", vec![a.clone(), b.clone()]), c.clone()],
                ),
                Term::func(
                    "*",
                    vec![a.clone(), Term::func("*", vec![b.clone(), c.clone()])],
                ),
            ),
        );
        lib.add_rule(
            "transpose_add",
            Equality::new(
                Term::func("T", vec![Term::func("+", vec![a.clone(), b.clone()])]),
                Term::func(
                    "+",
                    vec![
                        Term::func("T", vec![a.clone()]),
                        Term::func("T", vec![b.clone()]),
                    ],
                ),
            ),
        );
        lib.add_rule(
            "transpose_mul",
            Equality::new(
                Term::func("T", vec![Term::func("*", vec![a.clone(), b.clone()])]),
                Term::func(
                    "*",
                    vec![
                        Term::func("T", vec![b.clone()]),
                        Term::func("T", vec![a.clone()]),
                    ],
                ),
            ),
        );
        lib.add_rule(
            "transpose_transpose",
            Equality::new(
                Term::func("T", vec![Term::func("T", vec![a.clone()])]),
                a.clone(),
            ),
        );
        lib.add_rule(
            "trace_add",
            Equality::new(
                Term::func("tr", vec![Term::func("+", vec![a.clone(), b.clone()])]),
                Term::func(
                    "+",
                    vec![
                        Term::func("tr", vec![a.clone()]),
                        Term::func("tr", vec![b.clone()]),
                    ],
                ),
            ),
        );
        lib.add_rule(
            "trace_cyclic",
            Equality::new(
                Term::func("tr", vec![Term::func("*", vec![a.clone(), b.clone()])]),
                Term::func("tr", vec![Term::func("*", vec![b.clone(), a.clone()])]),
            ),
        );
        lib.add_rule(
            "det_mul",
            Equality::new(
                Term::func("det", vec![Term::func("*", vec![a.clone(), b.clone()])]),
                Term::func(
                    "*",
                    vec![
                        Term::func("det", vec![a.clone()]),
                        Term::func("det", vec![b.clone()]),
                    ],
                ),
            ),
        );
        lib.add_rule(
            "det_ident",
            Equality::new(Term::func("det", vec![i.clone()]), Term::constant("1")),
        );
        lib.add_rule(
            "mat_mul_inv_right",
            Equality::new(
                Term::func("*", vec![a.clone(), Term::func("inv", vec![a.clone()])]),
                i.clone(),
            ),
        );
        lib.add_rule(
            "mat_mul_inv_left",
            Equality::new(
                Term::func("*", vec![Term::func("inv", vec![a.clone()]), a.clone()]),
                i.clone(),
            ),
        );
        lib.add_rule(
            "inv_mul",
            Equality::new(
                Term::func("inv", vec![Term::func("*", vec![a.clone(), b.clone()])]),
                Term::func(
                    "*",
                    vec![
                        Term::func("inv", vec![b.clone()]),
                        Term::func("inv", vec![a.clone()]),
                    ],
                ),
            ),
        );
        lib.add_rule(
            "inv_inv",
            Equality::new(
                Term::func("inv", vec![Term::func("inv", vec![a.clone()])]),
                a.clone(),
            ),
        );

        lib
    }

    /// Order Theory & Inequality Axioms
    pub fn order_theory() -> Self {
        let x = Term::var("x");
        let y = Term::var("y");
        let z = Term::var("z");
        let zero = Term::constant("0");
        let t = Term::constant("true");

        let mut lib = Self::standard_algebra();

        lib.add_rule(
            "le_refl",
            Equality::new(
                Term::func("<=", vec![x.clone(), x.clone()]),
                t.clone(),
            ),
        );
        lib.add_rule(
            "square_nonneg",
            Equality::new(
                Term::func("<=", vec![zero.clone(), Term::func("*", vec![x.clone(), x.clone()])]),
                t.clone(),
            ),
        );
        lib.add_rule(
            "le_add_right",
            Equality::new(
                Term::func("<=", vec![x.clone(), y.clone()]),
                Term::func(
                    "<=",
                    vec![
                        Term::func("+", vec![x.clone(), z.clone()]),
                        Term::func("+", vec![y.clone(), z.clone()]),
                    ],
                ),
            ),
        );
        lib.add_rule(
            "abs_triangle",
            Equality::new(
                Term::func(
                    "<=",
                    vec![
                        Term::func("abs", vec![Term::func("+", vec![x.clone(), y.clone()])]),
                        Term::func(
                            "+",
                            vec![
                                Term::func("abs", vec![x.clone()]),
                                Term::func("abs", vec![y.clone()]),
                            ],
                        ),
                    ],
                ),
                t.clone(),
            ),
        );

        lib
    }

    /// Multivariable Calculus, Vector Differential Operators & PDEs
    pub fn multivariable_calculus() -> Self {
        let mut lib = Self::empty();
        let f = Term::var("f");
        let u = Term::var("u");
        let v = Term::var("v");
        let zero = Term::constant("0");
        let one = Term::constant("1");

        // Schwarz's / Clairaut's Theorem (Symmetry of Mixed Partials)
        lib.add_rule(
            "clairaut_xy",
            Equality::new(
                Term::func("Dx", vec![Term::func("Dy", vec![f.clone()])]),
                Term::func("Dy", vec![Term::func("Dx", vec![f.clone()])]),
            ),
        );
        lib.add_rule(
            "clairaut_xt",
            Equality::new(
                Term::func("Dx", vec![Term::func("Dt", vec![f.clone()])]),
                Term::func("Dt", vec![Term::func("Dx", vec![f.clone()])]),
            ),
        );
        lib.add_rule(
            "clairaut_yt",
            Equality::new(
                Term::func("Dy", vec![Term::func("Dt", vec![f.clone()])]),
                Term::func("Dt", vec![Term::func("Dy", vec![f.clone()])]),
            ),
        );

        // Linearity of Partial Derivatives
        lib.add_rule(
            "dx_sum",
            Equality::new(
                Term::func("Dx", vec![Term::func("+", vec![u.clone(), v.clone()])]),
                Term::func("+", vec![Term::func("Dx", vec![u.clone()]), Term::func("Dx", vec![v.clone()])]),
            ),
        );
        lib.add_rule(
            "dy_sum",
            Equality::new(
                Term::func("Dy", vec![Term::func("+", vec![u.clone(), v.clone()])]),
                Term::func("+", vec![Term::func("Dy", vec![u.clone()]), Term::func("Dy", vec![v.clone()])]),
            ),
        );
        lib.add_rule(
            "dt_sum",
            Equality::new(
                Term::func("Dt", vec![Term::func("+", vec![u.clone(), v.clone()])]),
                Term::func("+", vec![Term::func("Dt", vec![u.clone()]), Term::func("Dt", vec![v.clone()])]),
            ),
        );

        // Coordinate Variables
        lib.add_rule(
            "dx_x",
            Equality::new(Term::func("Dx", vec![Term::constant("x")]), one.clone()),
        );
        lib.add_rule(
            "dx_y",
            Equality::new(Term::func("Dx", vec![Term::constant("y")]), zero.clone()),
        );
        lib.add_rule(
            "dx_t",
            Equality::new(Term::func("Dx", vec![Term::constant("t")]), zero.clone()),
        );
        lib.add_rule(
            "dy_y",
            Equality::new(Term::func("Dy", vec![Term::constant("y")]), one.clone()),
        );
        lib.add_rule(
            "dy_x",
            Equality::new(Term::func("Dy", vec![Term::constant("x")]), zero.clone()),
        );
        lib.add_rule(
            "dt_t",
            Equality::new(Term::func("Dt", vec![Term::constant("t")]), one.clone()),
        );
        lib.add_rule(
            "dt_x",
            Equality::new(Term::func("Dt", vec![Term::constant("x")]), zero.clone()),
        );

        // Vector Calculus Fundamental Identities
        lib.add_rule(
            "curl_grad",
            Equality::new(
                Term::func("curl", vec![Term::func("grad", vec![f.clone()])]),
                zero.clone(),
            ),
        );
        lib.add_rule(
            "div_curl",
            Equality::new(
                Term::func("div", vec![Term::func("curl", vec![f.clone()])]),
                zero.clone(),
            ),
        );
        lib.add_rule(
            "laplacian_def",
            Equality::new(
                Term::func("laplacian", vec![f.clone()]),
                Term::func("div", vec![Term::func("grad", vec![f.clone()])]),
            ),
        );
        lib.add_rule(
            "laplacian_cartesian",
            Equality::new(
                Term::func("laplacian", vec![f.clone()]),
                Term::func(
                    "+",
                    vec![
                        Term::func("Dx", vec![Term::func("Dx", vec![f.clone()])]),
                        Term::func("Dy", vec![Term::func("Dy", vec![f.clone()])]),
                    ],
                ),
            ),
        );

        // Basic arithmetic laws
        lib.add_rule(
            "add_comm",
            Equality::new(
                Term::func("+", vec![u.clone(), v.clone()]),
                Term::func("+", vec![v.clone(), u.clone()]),
            ),
        );
        lib.add_rule(
            "add_zero",
            Equality::new(Term::func("+", vec![u.clone(), zero.clone()]), u.clone()),
        );
        lib.add_rule(
            "zero_add",
            Equality::new(Term::func("+", vec![zero.clone(), u.clone()]), u.clone()),
        );

        lib
    }

    /// Axiomatic Probability Theory & Bayes' Rule
    pub fn probability() -> Self {
        let mut lib = Self::empty();
        let a = Term::var("A");
        let b = Term::var("B");
        let zero = Term::constant("0");
        let one = Term::constant("1");
        let u = Term::constant("U");

        // Kolmogorov Axioms & Event Set Algebra
        lib.add_rule(
            "prob_total",
            Equality::new(Term::func("P", vec![u.clone()]), one.clone()),
        );
        lib.add_rule(
            "prob_empty",
            Equality::new(Term::func("P", vec![zero.clone()]), zero.clone()),
        );
        lib.add_rule(
            "prob_union",
            Equality::new(
                Term::func("P", vec![Term::func("|", vec![a.clone(), b.clone()])]),
                Term::func(
                    "+",
                    vec![
                        Term::func("+", vec![Term::func("P", vec![a.clone()]), Term::func("P", vec![b.clone()])]),
                        Term::func("-", vec![Term::func("P", vec![Term::func("&", vec![a.clone(), b.clone()])])]),
                    ],
                ),
            ),
        );
        lib.add_rule(
            "prob_complement",
            Equality::new(
                Term::func("P", vec![Term::func("!", vec![a.clone()])]),
                Term::func("+", vec![one.clone(), Term::func("-", vec![Term::func("P", vec![a.clone()])])]),
            ),
        );
        lib.add_rule(
            "event_and_comm",
            Equality::new(
                Term::func("&", vec![a.clone(), b.clone()]),
                Term::func("&", vec![b.clone(), a.clone()]),
            ),
        );

        // Conditional Probability Definition: P(A & B) = cond(A, B) * P(B)
        lib.add_rule(
            "prob_cond_def",
            Equality::new(
                Term::func("P", vec![Term::func("&", vec![a.clone(), b.clone()])]),
                Term::func("*", vec![Term::func("cond", vec![a.clone(), b.clone()]), Term::func("P", vec![b.clone()])]),
            ),
        );
        lib.add_rule(
            "prob_cond_def_rev",
            Equality::new(
                Term::func("*", vec![Term::func("cond", vec![a.clone(), b.clone()]), Term::func("P", vec![b.clone()])]),
                Term::func("P", vec![Term::func("&", vec![a.clone(), b.clone()])]),
            ),
        );

        // Bayes' Theorem Symmetric Formulation: cond(A, B) * P(B) = cond(B, A) * P(A)
        lib.add_rule(
            "bayes_symmetric",
            Equality::new(
                Term::func("*", vec![Term::func("cond", vec![a.clone(), b.clone()]), Term::func("P", vec![b.clone()])]),
                Term::func("*", vec![Term::func("cond", vec![b.clone(), a.clone()]), Term::func("P", vec![a.clone()])]),
            ),
        );
        lib.add_rule(
            "bayes_isolated",
            Equality::new(
                Term::func("cond", vec![a.clone(), b.clone()]),
                Term::func(
                    "/",
                    vec![
                        Term::func("*", vec![Term::func("cond", vec![b.clone(), a.clone()]), Term::func("P", vec![a.clone()])]),
                        Term::func("P", vec![b.clone()]),
                    ],
                ),
            ),
        );
        lib.add_rule(
            "mul_comm",
            Equality::new(
                Term::func("*", vec![a.clone(), b.clone()]),
                Term::func("*", vec![b.clone(), a.clone()]),
            ),
        );

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
        for (name, rule) in Self::complex_numbers().rules {
            if !lib.rules.iter().any(|(n, _)| n == &name) {
                lib.add_rule(&name, rule);
            }
        }
        for (name, rule) in Self::linear_algebra().rules {
            if !lib.rules.iter().any(|(n, _)| n == &name) {
                lib.add_rule(&name, rule);
            }
        }
        for (name, rule) in Self::order_theory().rules {
            if !lib.rules.iter().any(|(n, _)| n == &name) {
                lib.add_rule(&name, rule);
            }
        }
        for (name, rule) in Self::multivariable_calculus().rules {
            if !lib.rules.iter().any(|(n, _)| n == &name) {
                lib.add_rule(&name, rule);
            }
        }
        for (name, rule) in Self::probability().rules {
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
                        format!(
                            "Solved #{}: {} via rfl",
                            target_goal.id, target_goal.equality
                        ),
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
                next_state
                    .proof_history
                    .push((Tactic::Symmetry, format!("Applied symm: {}", flipped)));
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
            Tactic::EvalArithmeticLhs => {
                let rewrites = crate::verifier::eval::eval_step_arithmetic(&target_goal.equality.lhs);
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
                        Tactic::EvalArithmeticLhs,
                        format!("Evaluated arithmetic on LHS: {}", new_eq),
                    ));
                    next_state.depth += 1;
                    next_state.check_solved();
                    Ok(next_state)
                } else {
                    Err("No arithmetic operations to evaluate on LHS")
                }
            }
            Tactic::EvalArithmeticRhs => {
                let rewrites = crate::verifier::eval::eval_step_arithmetic(&target_goal.equality.rhs);
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
                        Tactic::EvalArithmeticRhs,
                        format!("Evaluated arithmetic on RHS: {}", new_eq),
                    ));
                    next_state.depth += 1;
                    next_state.check_solved();
                    Ok(next_state)
                } else {
                    Err("No arithmetic operations to evaluate on RHS")
                }
            }
            Tactic::Transitivity(_) => Err("Transitivity unsupported in minimal verifier"),
            Tactic::ApplyAxiom(_) => {
                Err("Direct axiom application unsupported in minimal verifier")
            }
            Tactic::TransposeToZero => {
                let zero = Term::constant("0");
                if target_goal.equality.rhs != zero {
                    let new_lhs = Term::func(
                        "-",
                        vec![
                            target_goal.equality.lhs.clone(),
                            target_goal.equality.rhs.clone(),
                        ],
                    );
                    let new_eq = Equality::new(new_lhs, zero);
                    next_state.open_goals.insert(
                        0,
                        Goal {
                            id: target_goal.id,
                            equality: new_eq.clone(),
                        },
                    );
                    next_state.proof_history.push((
                        Tactic::TransposeToZero,
                        format!("Transposed equation to zero: {}", new_eq),
                    ));
                    next_state.depth += 1;
                    next_state.check_solved();
                    Ok(next_state)
                } else {
                    Err("Equation already has zero RHS")
                }
            }
            Tactic::ZeroProductSplit => {
                let zero = Term::constant("0");
                if target_goal.equality.rhs == zero {
                    if let Term::Func(ref op, ref args) = target_goal.equality.lhs {
                        if (op == "*" || op == "·") && args.len() == 2 {
                            let goal_a = Goal {
                                id: target_goal.id,
                                equality: Equality::new(args[0].clone(), zero.clone()),
                            };
                            let goal_b = Goal {
                                id: target_goal.id + 1000,
                                equality: Equality::new(args[1].clone(), zero.clone()),
                            };
                            next_state.open_goals.insert(0, goal_a);
                            next_state.open_goals.push(goal_b);
                            next_state.proof_history.push((
                                Tactic::ZeroProductSplit,
                                format!(
                                    "Split zero product: ({} = 0) and ({} = 0)",
                                    args[0], args[1]
                                ),
                            ));
                            next_state.depth += 1;
                            next_state.check_solved();
                            return Ok(next_state);
                        }
                    }
                }
                Err("ZeroProductSplit requires (A * B) = 0")
            }
        }
    }

    /// Generates all formally valid successor states from the current proof state
    pub fn expand_valid_transitions(
        state: &ProofState,
        axioms: &AxiomLibrary,
    ) -> Vec<(Tactic, ProofState)> {
        let mut successors = Vec::new();

        if let Some(target_goal) = state.open_goals.first() {
            let zero = Term::constant("0");
            if target_goal.equality.rhs != zero
                && !state
                    .proof_history
                    .iter()
                    .any(|(t, _)| matches!(t, Tactic::TransposeToZero))
            {
                if let Ok(next) = Self::apply_tactic(state, &Tactic::TransposeToZero, axioms) {
                    successors.push((Tactic::TransposeToZero, next));
                }
            }

            if target_goal.equality.rhs == zero {
                if let Term::Func(ref op, ref args) = target_goal.equality.lhs {
                    if (op == "*" || op == "·") && args.len() == 2 {
                        if let Ok(next) =
                            Self::apply_tactic(state, &Tactic::ZeroProductSplit, axioms)
                        {
                            successors.push((Tactic::ZeroProductSplit, next));
                        }
                    }
                }
            }
        }

        // 1. Try Reflexivity
        if let Ok(next) = Self::apply_tactic(state, &Tactic::Reflexivity, axioms) {
            successors.push((Tactic::Reflexivity, next));
            return successors; // Immediate Q.E.D.
        }

        // 2. Try Symmetry (if not applied immediately before)
        if !state
            .proof_history
            .iter()
            .rev()
            .take(1)
            .any(|(t, _)| matches!(t, Tactic::Symmetry))
        {
            if let Ok(next) = Self::apply_tactic(state, &Tactic::Symmetry, axioms) {
                successors.push((Tactic::Symmetry, next));
            }
        }

        // 3. Bidirectional / Meet-in-the-Middle Confluence Detection
        if let Some(target_goal) = state.open_goals.first() {
            let mut lhs_reducts: Vec<(String, Term)> = Vec::new();
            let mut rhs_reducts: Vec<(String, Term)> = Vec::new();
            for (name, rule) in &axioms.rules {
                for new_lhs in apply_rewrite(&target_goal.equality.lhs, rule) {
                    lhs_reducts.push((name.clone(), new_lhs));
                }
                for new_rhs in apply_rewrite(&target_goal.equality.rhs, rule) {
                    rhs_reducts.push((name.clone(), new_rhs));
                }
            }

            for (lhs_rule, m_lhs) in &lhs_reducts {
                for (rhs_rule, m_rhs) in &rhs_reducts {
                    if m_lhs == m_rhs {
                        let mut next_state = state.clone();
                        next_state.open_goals.remove(0);
                        let mid_eq = Equality::new(m_lhs.clone(), target_goal.equality.rhs.clone());
                        let final_eq = Equality::new(m_lhs.clone(), m_rhs.clone());
                        next_state.proof_history.push((
                            Tactic::RewriteLhs(lhs_rule.clone()),
                            format!("Rewrote LHS via [{}]: {}", lhs_rule, mid_eq),
                        ));
                        next_state.proof_history.push((
                            Tactic::RewriteRhs(rhs_rule.clone()),
                            format!("Rewrote RHS via [{}]: {}", rhs_rule, final_eq),
                        ));
                        next_state.depth += 2;
                        next_state.is_solved = true;
                        successors.insert(0, (Tactic::RewriteLhs(lhs_rule.clone()), next_state));
                        return successors;
                    }
                }
            }
        }

        // 4. Try Rewriting with all available axioms across all matching subterms
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

            // 4. Try Arithmetic Normalization on LHS & RHS
            for new_lhs in crate::verifier::eval::eval_step_arithmetic(&target_goal.equality.lhs) {
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
                    Tactic::EvalArithmeticLhs,
                    format!("Evaluated arithmetic on LHS: {}", new_eq),
                ));
                next_state.depth += 1;
                next_state.check_solved();
                successors.push((Tactic::EvalArithmeticLhs, next_state));
            }

            for new_rhs in crate::verifier::eval::eval_step_arithmetic(&target_goal.equality.rhs) {
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
                    Tactic::EvalArithmeticRhs,
                    format!("Evaluated arithmetic on RHS: {}", new_eq),
                ));
                next_state.depth += 1;
                next_state.check_solved();
                successors.push((Tactic::EvalArithmeticRhs, next_state));
            }
        }

        successors
    }

    pub fn prune_proof_history(
        initial: &Equality,
        history: &[(Tactic, String)],
        axioms: &AxiomLibrary,
    ) -> Vec<(Tactic, String)> {
        let mut pruned = history.to_vec();

        let mut changed = true;
        while changed {
            changed = false;
            let mut i = 0;
            while i + 1 < pruned.len() {
                if matches!(pruned[i].0, Tactic::Symmetry)
                    && matches!(pruned[i + 1].0, Tactic::Symmetry)
                {
                    pruned.remove(i + 1);
                    pruned.remove(i);
                    changed = true;
                    break;
                }
                i += 1;
            }
        }

        let mut i = pruned.len();
        while i > 0 {
            i -= 1;
            let mut candidate = pruned.clone();
            candidate.remove(i);

            let mut test_state = ProofState::new(initial.clone());
            let mut valid = true;
            for (tac, _) in &candidate {
                match Self::apply_tactic(&test_state, tac, axioms) {
                    Ok(next) => test_state = next,
                    Err(_) => {
                        valid = false;
                        break;
                    }
                }
            }
            if valid && test_state.is_solved {
                pruned = candidate;
            }
        }
        pruned
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prove_commutativity_identity() {
        let axioms = AxiomLibrary::standard_algebra();

        let a = Term::constant("a");
        let zero = Term::constant("0");
        let goal_eq = Equality::new(
            Term::func("+", vec![a.clone(), zero.clone()]),
            Term::func("+", vec![zero.clone(), a.clone()]),
        );

        let initial_state = ProofState::new(goal_eq);
        assert!(!initial_state.is_solved);

        let step1 = FormalVerifier::apply_tactic(
            &initial_state,
            &Tactic::RewriteLhs("add_zero".to_string()),
            &axioms,
        )
        .expect("Step 1 valid");

        let step2 = FormalVerifier::apply_tactic(
            &step1,
            &Tactic::RewriteRhs("zero_add".to_string()),
            &axioms,
        )
        .expect("Step 2 valid");

        assert!(
            step2.is_solved,
            "Proof must be certified complete upon reaching reflexivity"
        );
        assert_eq!(step2.proof_history.len(), 2);
    }

    #[test]
    fn test_proof_minimization_prunes_redundancy() -> Result<(), Box<dyn std::error::Error>> {
        let axioms = AxiomLibrary::standard_algebra();
        let a = Term::constant("a");
        let zero = Term::constant("0");
        let goal_eq = Equality::new(Term::func("+", vec![a.clone(), zero.clone()]), a.clone());

        let initial_state = ProofState::new(goal_eq);
        let step1 = FormalVerifier::apply_tactic(&initial_state, &Tactic::Symmetry, &axioms)?;
        let step2 = FormalVerifier::apply_tactic(&step1, &Tactic::Symmetry, &axioms)?;
        let mut step3 = FormalVerifier::apply_tactic(
            &step2,
            &Tactic::RewriteLhs("add_zero".to_string()),
            &axioms,
        )?;
        assert!(step3.is_solved);
        assert_eq!(step3.proof_history.len(), 3);

        step3.minimize(&axioms);
        assert!(step3.is_solved);
        assert_eq!(step3.proof_history.len(), 1);
        assert_eq!(
            step3.proof_history[0].0,
            Tactic::RewriteLhs("add_zero".to_string())
        );
        Ok(())
    }
}
