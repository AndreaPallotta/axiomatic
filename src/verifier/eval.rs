use super::fol::{Equality, Term};

pub fn gcd(mut a: i64, mut b: i64) -> i64 {
    a = a.abs();
    b = b.abs();
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    if a == 0 {
        1
    } else {
        a
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rational {
    pub num: i64,
    pub den: i64,
}

impl Rational {
    pub fn new(num: i64, den: i64) -> Option<Self> {
        if den == 0 {
            return None;
        }
        let g = gcd(num, den);
        let sign = if den < 0 { -1 } else { 1 };
        Some(Self {
            num: sign * (num / g),
            den: den.abs() / g,
        })
    }

    pub fn from_i64(n: i64) -> Self {
        Self { num: n, den: 1 }
    }

    pub fn is_integer(&self) -> bool {
        self.den == 1
    }

    pub fn add(self, other: Self) -> Option<Self> {
        let num = self
            .num
            .checked_mul(other.den)?
            .checked_add(other.num.checked_mul(self.den)?)?;
        let den = self.den.checked_mul(other.den)?;
        Self::new(num, den)
    }

    pub fn sub(self, other: Self) -> Option<Self> {
        let num = self
            .num
            .checked_mul(other.den)?
            .checked_sub(other.num.checked_mul(self.den)?)?;
        let den = self.den.checked_mul(other.den)?;
        Self::new(num, den)
    }

    pub fn mul(self, other: Self) -> Option<Self> {
        let num = self.num.checked_mul(other.num)?;
        let den = self.den.checked_mul(other.den)?;
        Self::new(num, den)
    }

    pub fn div(self, other: Self) -> Option<Self> {
        if other.num == 0 {
            return None;
        }
        let num = self.num.checked_mul(other.den)?;
        let den = self.den.checked_mul(other.num)?;
        Self::new(num, den)
    }

    pub fn neg(self) -> Self {
        Self {
            num: -self.num,
            den: self.den,
        }
    }

    pub fn to_term(&self) -> Term {
        if self.den == 1 {
            Term::from_i64(self.num)
        } else {
            Term::func("/", vec![Term::from_i64(self.num), Term::from_i64(self.den)])
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ComplexRational {
    pub re: Rational,
    pub im: Rational,
}

impl ComplexRational {
    pub fn new(re: Rational, im: Rational) -> Self {
        Self { re, im }
    }

    pub fn from_i64(n: i64) -> Self {
        Self {
            re: Rational::from_i64(n),
            im: Rational::from_i64(0),
        }
    }

    pub fn from_term(term: &Term) -> Option<Self> {
        match term {
            Term::Const(s) if s == "i" => Some(Self::new(Rational::from_i64(0), Rational::from_i64(1))),
            Term::Const(s) => s.parse::<i64>().ok().map(Self::from_i64),
            Term::Func(op, args) if op == "-" && args.len() == 1 => {
                let inner = Self::from_term(&args[0])?;
                Some(Self::new(inner.re.neg(), inner.im.neg()))
            }
            Term::Func(op, args) if args.len() == 2 => {
                let a = Self::from_term(&args[0])?;
                let b = Self::from_term(&args[1])?;
                match op.as_str() {
                    "+" => a.add(b),
                    "-" => a.sub(b),
                    "*" | "·" => a.mul(b),
                    "/" => a.div(b),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    pub fn add(self, other: Self) -> Option<Self> {
        Some(Self::new(self.re.add(other.re)?, self.im.add(other.im)?))
    }

    pub fn sub(self, other: Self) -> Option<Self> {
        Some(Self::new(self.re.sub(other.re)?, self.im.sub(other.im)?))
    }

    pub fn mul(self, other: Self) -> Option<Self> {
        let re = self.re.mul(other.re)?.sub(self.im.mul(other.im)?)?;
        let im = self.re.mul(other.im)?.add(self.im.mul(other.re)?)?;
        Some(Self::new(re, im))
    }

    pub fn div(self, other: Self) -> Option<Self> {
        let denom = other.re.mul(other.re)?.add(other.im.mul(other.im)?)?;
        if denom.num == 0 {
            return None;
        }
        let re_num = self.re.mul(other.re)?.add(self.im.mul(other.im)?)?;
        let im_num = self.im.mul(other.re)?.sub(self.re.mul(other.im)?)?;
        Some(Self::new(re_num.div(denom)?, im_num.div(denom)?))
    }

    pub fn to_term(&self) -> Term {
        if self.im.num == 0 {
            self.re.to_term()
        } else if self.re.num == 0 {
            if self.im == Rational::from_i64(1) {
                Term::constant("i")
            } else if self.im == Rational::from_i64(-1) {
                Term::func("-", vec![Term::constant("i")])
            } else {
                Term::func("*", vec![self.im.to_term(), Term::constant("i")])
            }
        } else {
            let im_part = if self.im == Rational::from_i64(1) {
                Term::constant("i")
            } else if self.im == Rational::from_i64(-1) {
                Term::func("-", vec![Term::constant("i")])
            } else {
                Term::func("*", vec![self.im.to_term(), Term::constant("i")])
            };
            Term::func("+", vec![self.re.to_term(), im_part])
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ComplexInt {
    pub re: i64,
    pub im: i64,
}

impl ComplexInt {
    pub fn new(re: i64, im: i64) -> Self {
        Self { re, im }
    }

    pub fn from_term(term: &Term) -> Option<Self> {
        let cr = ComplexRational::from_term(term)?;
        if cr.re.is_integer() && cr.im.is_integer() {
            Some(Self::new(cr.re.num, cr.im.num))
        } else {
            None
        }
    }

    pub fn to_term(&self) -> Term {
        ComplexRational::new(Rational::from_i64(self.re), Rational::from_i64(self.im)).to_term()
    }
}

pub fn eval_step_arithmetic(term: &Term) -> Vec<Term> {
    let mut results = Vec::new();

    match term {
        Term::Func(op, args)
            if args.len() == 2 && (op == "+" || op == "*" || op == "·" || op == "-" || op == "/") =>
        {
            if let (Some(a), Some(b)) = (
                ComplexRational::from_term(&args[0]),
                ComplexRational::from_term(&args[1]),
            ) {
                let val = match op.as_str() {
                    "+" => a.add(b),
                    "-" => a.sub(b),
                    "*" | "·" => a.mul(b),
                    "/" => a.div(b),
                    _ => None,
                };
                if let Some(res) = val {
                    let new_term = res.to_term();
                    if &new_term != term {
                        results.push(new_term);
                    }
                }
            }

            for rewritten in eval_step_arithmetic(&args[0]) {
                results.push(Term::func(op, vec![rewritten, args[1].clone()]));
            }
            for rewritten in eval_step_arithmetic(&args[1]) {
                results.push(Term::func(op, vec![args[0].clone(), rewritten]));
            }
        }
        Term::Func(op, args) if args.len() == 1 && op == "-" => {
            if let Some(a) = ComplexRational::from_term(&args[0]) {
                let neg = ComplexRational::new(a.re.neg(), a.im.neg());
                let new_term = neg.to_term();
                if &new_term != term {
                    results.push(new_term);
                }
            }
            for rewritten in eval_step_arithmetic(&args[0]) {
                results.push(Term::func(op, vec![rewritten]));
            }
        }
        Term::Func(op, args) => {
            for i in 0..args.len() {
                for rewritten in eval_step_arithmetic(&args[i]) {
                    let mut new_args = args.clone();
                    new_args[i] = rewritten;
                    results.push(Term::func(op, new_args));
                }
            }
        }
        Term::Const(s) => {
            if let Ok(val) = s.parse::<i64>() {
                if val < 0 {
                    let t = Term::from_i64(val);
                    if &t != term {
                        results.push(t);
                    }
                }
            }
        }
        _ => {}
    }

    results.dedup();
    results
}

pub fn eval_full_arithmetic(term: &Term) -> Term {
    if let Some(c) = ComplexRational::from_term(term) {
        return c.to_term();
    }
    match term {
        Term::Func(op, args) => {
            let new_args: Vec<Term> = args.iter().map(eval_full_arithmetic).collect();
            Term::func(op, new_args)
        }
        Term::Const(s) => {
            if let Ok(val) = s.parse::<i64>() {
                if val < 0 {
                    Term::from_i64(val)
                } else {
                    term.clone()
                }
            } else {
                term.clone()
            }
        }
        _ => term.clone(),
    }
}

pub fn find_polynomial_roots(eq: &Equality, var: &str, range: i64) -> Vec<Term> {
    let mut roots = Vec::new();

    for r in -range..=range {
        let subst_term = Term::from_i64(r);
        let subst_eq = eq.replace_variable(var, &subst_term);
        let l = eval_full_arithmetic(&subst_eq.lhs);
        let r_val = eval_full_arithmetic(&subst_eq.rhs);
        if l == r_val {
            roots.push(subst_term);
        }
    }

    for q in 2..=4 {
        for p in -(range * q)..=(range * q) {
            if gcd(p, q) == 1 {
                if let Some(rat) = Rational::new(p, q) {
                    let subst_term = rat.to_term();
                    let subst_eq = eq.replace_variable(var, &subst_term);
                    let l = eval_full_arithmetic(&subst_eq.lhs);
                    let r_val = eval_full_arithmetic(&subst_eq.rhs);
                    if l == r_val && !roots.contains(&subst_term) {
                        roots.push(subst_term);
                    }
                }
            }
        }
    }

    for a in -range..=range {
        for b in -range..=range {
            if b == 0 {
                continue;
            }
            let c = ComplexRational::new(Rational::from_i64(a), Rational::from_i64(b));
            let subst_term = c.to_term();
            let subst_eq = eq.replace_variable(var, &subst_term);
            let l = eval_full_arithmetic(&subst_eq.lhs);
            let r_val = eval_full_arithmetic(&subst_eq.rhs);
            if l == r_val && !roots.contains(&subst_term) {
                roots.push(subst_term);
            }
        }
    }

    roots
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eval_simple_addition() {
        let t = Term::func("+", vec![Term::from_i64(2), Term::from_i64(3)]);
        let res = eval_full_arithmetic(&t);
        assert_eq!(res.as_i64(), Some(5));
    }

    #[test]
    fn test_eval_nested_arithmetic() {
        let t = Term::func(
            "+",
            vec![
                Term::func(
                    "*",
                    vec![
                        Term::func("*", vec![Term::from_i64(2), Term::from_i64(2)]),
                        Term::from_i64(2),
                    ],
                ),
                Term::from_i64(2),
            ],
        );
        let res = eval_full_arithmetic(&t);
        assert_eq!(res.as_i64(), Some(10));
    }

    #[test]
    fn test_eval_partial_ground() {
        let x = Term::constant("x");
        let t = Term::func(
            "+",
            vec![
                x.clone(),
                Term::func("*", vec![Term::from_i64(3), Term::from_i64(4)]),
            ],
        );
        let res = eval_full_arithmetic(&t);
        assert_eq!(res, Term::func("+", vec![x, Term::from_i64(12)]));
    }

    #[test]
    fn test_eval_complex_i_squared() {
        let t = Term::func("*", vec![Term::constant("i"), Term::constant("i")]);
        let res = eval_full_arithmetic(&t);
        assert_eq!(res.as_i64(), Some(-1));
    }

    #[test]
    fn test_eval_complex_root_satisfaction() {
        let Ok(eq) = crate::verifier::parser::parse_conjecture(
            "((((-1 + (2 * i)) * (-1 + (2 * i))) * (-1 + (2 * i))) + (-1 + (2 * i))) = 10",
        ) else {
            panic!("Failed to parse equation");
        };
        let res_lhs = eval_full_arithmetic(&eq.lhs);
        let res_rhs = eval_full_arithmetic(&eq.rhs);
        assert_eq!(res_lhs, res_rhs);
        assert_eq!(res_lhs.as_i64(), Some(10));
    }

    #[test]
    fn test_eval_complex_conjugate_root() {
        let Ok(eq) = crate::verifier::parser::parse_conjecture(
            "((((-1 - (2 * i)) * (-1 - (2 * i))) * (-1 - (2 * i))) + (-1 - (2 * i))) = 10",
        ) else {
            panic!("Failed to parse equation");
        };
        let res_lhs = eval_full_arithmetic(&eq.lhs);
        let res_rhs = eval_full_arithmetic(&eq.rhs);
        assert_eq!(res_lhs, res_rhs);
        assert_eq!(res_lhs.as_i64(), Some(10));
    }

    #[test]
    fn test_find_polynomial_roots_cubic() {
        let Ok(eq) = crate::verifier::parser::parse_conjecture("(((X * X) * X) + X) = 10") else {
            panic!("Failed to parse equation");
        };
        let roots = find_polynomial_roots(&eq, "X", 4);
        assert_eq!(roots.len(), 3);
        assert!(roots.iter().any(|r| r.to_string() == "2"));
        assert!(roots.iter().any(|r| r.to_string().contains("i")));
    }

    #[test]
    fn test_eval_rational_addition() {
        let Ok(eq) = crate::verifier::parser::parse_conjecture("(1 / 2) + (1 / 3) = (5 / 6)") else {
            panic!("Failed to parse rational addition");
        };
        let l = eval_full_arithmetic(&eq.lhs);
        let r = eval_full_arithmetic(&eq.rhs);
        assert_eq!(l, r);
    }

    #[test]
    fn test_eval_rational_division() {
        let Ok(eq) = crate::verifier::parser::parse_conjecture("(3 / 4) / (1 / 2) = (3 / 2)") else {
            panic!("Failed to parse rational division");
        };
        let l = eval_full_arithmetic(&eq.lhs);
        let r = eval_full_arithmetic(&eq.rhs);
        assert_eq!(l, r);
    }

    #[test]
    fn test_find_polynomial_roots_rational() {
        let Ok(eq) = crate::verifier::parser::parse_conjecture("(2 * X) - 3 = 0") else {
            panic!("Failed to parse linear equation");
        };
        let roots = find_polynomial_roots(&eq, "X", 4);
        assert!(!roots.is_empty(), "Must discover rational root");
        assert!(roots.iter().any(|r| r.to_string() == "(3 / 2)"));
    }

    #[test]
    fn test_counterexample_detects_false_identities() {
        let Ok(false_eq1) = crate::verifier::parser::parse_conjecture("((x + y) * (x + y)) = ((x * x) + (y * y))") else {
            panic!("Failed to parse false equation");
        };
        let ce1 = find_counterexample(&false_eq1);
        if let Some(unwrapped1) = ce1 {
            assert_ne!(unwrapped1.lhs_val, unwrapped1.rhs_val);
        } else {
            panic!("Expected counterexample for freshman's dream");
        }

        let Ok(false_eq2) = crate::verifier::parser::parse_conjecture("(x + 1) = x") else {
            panic!("Failed to parse false equation");
        };
        let ce2 = find_counterexample(&false_eq2);
        assert!(ce2.is_some(), "Must refute x + 1 = x");

        let Ok(true_eq) = crate::verifier::parser::parse_conjecture("(x + 0) = (0 + x)") else {
            panic!("Failed to parse true equation");
        };
        let ce_true = find_counterexample(&true_eq);
        assert!(ce_true.is_none(), "True identity must have no counterexample");
    }

    #[test]
    fn test_counterexample_boolean_logic() {
        let Ok(false_bool) = crate::verifier::parser::parse_conjecture("(a & 0) = 1") else {
            panic!("Failed to parse boolean equation");
        };
        let ce = find_counterexample(&false_bool);
        assert!(ce.is_some(), "Must refute false boolean equation");
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Counterexample {
    pub assignments: Vec<(String, Term)>,
    pub lhs_val: String,
    pub rhs_val: String,
}

pub fn eval_bool_term(term: &Term) -> Option<bool> {
    match term {
        Term::Const(s) if s == "1" || s == "true" => Some(true),
        Term::Const(s) if s == "0" || s == "false" => Some(false),
        Term::Func(op, args) if (op == "!" || op == "¬" || op == "~") && args.len() == 1 => {
            Some(!eval_bool_term(&args[0])?)
        }
        Term::Func(op, args) if args.len() == 2 => {
            let a = eval_bool_term(&args[0])?;
            let b = eval_bool_term(&args[1])?;
            match op.as_str() {
                "&" | "∧" => Some(a && b),
                "|" | "∨" => Some(a || b),
                _ => None,
            }
        }
        _ => None,
    }
}

pub fn find_counterexample(eq: &Equality) -> Option<Counterexample> {
    let mut vars = eq.extract_variables();
    vars.sort();
    vars.dedup();

    if vars.is_empty() {
        let is_bool = crate::verifier::lean::is_boolean_equality(eq);
        if is_bool {
            if let (Some(l), Some(r)) = (eval_bool_term(&eq.lhs), eval_bool_term(&eq.rhs)) {
                if l != r {
                    return Some(Counterexample {
                        assignments: Vec::new(),
                        lhs_val: l.to_string(),
                        rhs_val: r.to_string(),
                    });
                }
            }
        } else if let (Some(a), Some(b)) = (
            ComplexRational::from_term(&eq.lhs),
            ComplexRational::from_term(&eq.rhs),
        ) {
            if a != b {
                return Some(Counterexample {
                    assignments: Vec::new(),
                    lhs_val: a.to_term().to_string(),
                    rhs_val: b.to_term().to_string(),
                });
            }
        }
        return None;
    }

    let is_bool = crate::verifier::lean::is_boolean_equality(eq);
    if is_bool {
        let test_vals = vec![Term::constant("0"), Term::constant("1")];
        if vars.len() <= 5 {
            let total_combos = test_vals.len().pow(vars.len() as u32);
            for combo in 0..total_combos {
                let mut substituted_eq = eq.clone();
                let mut assigns = Vec::new();
                let mut temp = combo;
                for var in &vars {
                    let val = &test_vals[temp % test_vals.len()];
                    temp /= test_vals.len();
                    substituted_eq = substituted_eq.replace_variable(var, val);
                    assigns.push((var.clone(), val.clone()));
                }
                if let (Some(l), Some(r)) = (
                    eval_bool_term(&substituted_eq.lhs),
                    eval_bool_term(&substituted_eq.rhs),
                ) {
                    if l != r {
                        return Some(Counterexample {
                            assignments: assigns,
                            lhs_val: if l { "true".to_string() } else { "false".to_string() },
                            rhs_val: if r { "true".to_string() } else { "false".to_string() },
                        });
                    }
                }
            }
        }
    } else {
        let sample_vals = vec![
            Rational::from_i64(0),
            Rational::from_i64(1),
            Rational::from_i64(2),
            Rational::from_i64(-1),
            Rational::from_i64(3),
            Rational { num: 1, den: 2 },
        ];
        if vars.len() <= 4 {
            let num_vals = sample_vals.len();
            let total_combos = num_vals.pow(vars.len() as u32);
            for combo in 0..total_combos {
                let mut substituted_eq = eq.clone();
                let mut assigns = Vec::new();
                let mut temp = combo;
                for var in &vars {
                    let r = &sample_vals[temp % num_vals];
                    temp /= num_vals;
                    let val_term = r.to_term();
                    substituted_eq = substituted_eq.replace_variable(var, &val_term);
                    assigns.push((var.clone(), val_term));
                }
                if let (Some(l), Some(r)) = (
                    ComplexRational::from_term(&substituted_eq.lhs),
                    ComplexRational::from_term(&substituted_eq.rhs),
                ) {
                    if l != r {
                        return Some(Counterexample {
                            assignments: assigns,
                            lhs_val: l.to_term().to_string(),
                            rhs_val: r.to_term().to_string(),
                        });
                    }
                }
            }
        }
    }

    None
}
