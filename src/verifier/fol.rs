use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

/// A formal mathematical term in First-Order Logic
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Term {
    Var(String),
    Const(String),
    Func(String, Vec<Term>),
}

impl Term {
    pub fn var(name: &str) -> Self {
        Term::Var(name.to_string())
    }

    pub fn constant(name: &str) -> Self {
        Term::Const(name.to_string())
    }

    pub fn func(name: &str, args: Vec<Term>) -> Self {
        Term::Func(name.to_string(), args)
    }

    /// Substitutes variables according to the given mapping
    pub fn substitute(&self, subst: &HashMap<String, Term>) -> Term {
        match self {
            Term::Var(name) => {
                if let Some(replacement) = subst.get(name) {
                    replacement.substitute(subst)
                } else {
                    self.clone()
                }
            }
            Term::Const(_) => self.clone(),
            Term::Func(name, args) => {
                let new_args = args.iter().map(|arg| arg.substitute(subst)).collect();
                Term::Func(name.clone(), new_args)
            }
        }
    }

    /// Check if a variable occurs in this term (Occurs Check for Unification)
    pub fn occurs(&self, var_name: &str) -> bool {
        match self {
            Term::Var(name) => name == var_name,
            Term::Const(_) => false,
            Term::Func(_, args) => args.iter().any(|arg| arg.occurs(var_name)),
        }
    }

    /// Check if a symbol (var or const) exists anywhere in this term
    pub fn contains_symbol(&self, sym: &str) -> bool {
        match self {
            Term::Var(name) | Term::Const(name) => name == sym,
            Term::Func(_, args) => args.iter().any(|arg| arg.contains_symbol(sym)),
        }
    }

    /// Replaces occurrences of a variable or constant with a new term (used in Induction)
    pub fn replace_variable(&self, var_name: &str, replacement: &Term) -> Term {
        match self {
            Term::Var(name) | Term::Const(name) if name == var_name => replacement.clone(),
            Term::Var(_) | Term::Const(_) => self.clone(),
            Term::Func(f, args) => {
                let new_args = args
                    .iter()
                    .map(|a| a.replace_variable(var_name, replacement))
                    .collect();
                Term::Func(f.clone(), new_args)
            }
        }
    }

    /// Computes AST node count (size) of the term
    pub fn size(&self) -> usize {
        match self {
            Term::Var(_) | Term::Const(_) => 1,
            Term::Func(_, args) => 1 + args.iter().map(|a| a.size()).sum::<usize>(),
        }
    }

    /// Extracts all variable/constant symbols present in the term
    pub fn extract_symbols(&self) -> Vec<String> {
        let mut symbols = Vec::new();
        match self {
            Term::Var(name) | Term::Const(name) => {
                if name != "0" && name != "1" {
                    symbols.push(name.clone());
                }
            }
            Term::Func(_, args) => {
                for arg in args {
                    symbols.extend(arg.extract_symbols());
                }
            }
        }
        symbols.sort();
        symbols.dedup();
        symbols
    }
}

impl fmt::Display for Term {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Term::Var(name) => write!(f, "?{}", name),
            Term::Const(name) => write!(f, "{}", name),
            Term::Func(name, args) => {
                if args.len() == 2 && (name == "+" || name == "*" || name == "·" || name == "^" || name == "&" || name == "|") {
                    write!(f, "({} {} {})", args[0], name, args[1])
                } else if args.len() == 1 && (name == "-" || name == "!") {
                    write!(f, "{}{}", name, args[0])
                } else {
                    let args_str: Vec<String> = args.iter().map(|a| a.to_string()).collect();
                    write!(f, "{}({})", name, args_str.join(", "))
                }
            }
        }
    }
}

/// A formal proposition / equality relation: Left = Right
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Equality {
    pub lhs: Term,
    pub rhs: Term,
}

impl Equality {
    pub fn new(lhs: Term, rhs: Term) -> Self {
        Self { lhs, rhs }
    }

    pub fn substitute(&self, subst: &HashMap<String, Term>) -> Self {
        Self {
            lhs: self.lhs.substitute(subst),
            rhs: self.rhs.substitute(subst),
        }
    }

    pub fn flip(&self) -> Self {
        Self {
            lhs: self.rhs.clone(),
            rhs: self.lhs.clone(),
        }
    }
}

impl fmt::Display for Equality {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} = {}", self.lhs, self.rhs)
    }
}

/// Robinson's First-Order Unification Algorithm: Computes the Most General Unifier (MGU)
pub fn unify(t1: &Term, t2: &Term) -> Option<HashMap<String, Term>> {
    let mut subst = HashMap::new();
    if unify_internal(t1, t2, &mut subst) {
        Some(subst)
    } else {
        None
    }
}

fn unify_internal(t1: &Term, t2: &Term, subst: &mut HashMap<String, Term>) -> bool {
    let s1 = t1.substitute(subst);
    let s2 = t2.substitute(subst);

    if s1 == s2 {
        return true;
    }

    match (s1, s2) {
        (Term::Var(v), term) => {
            if term.occurs(&v) {
                return false; // Occurs check failure (avoids infinite loops)
            }
            subst.insert(v, term);
            true
        }
        (term, Term::Var(v)) => {
            if term.occurs(&v) {
                return false;
            }
            subst.insert(v, term);
            true
        }
        (Term::Func(f1, args1), Term::Func(f2, args2)) => {
            if f1 != f2 || args1.len() != args2.len() {
                return false;
            }
            for (a1, a2) in args1.iter().zip(args2.iter()) {
                if !unify_internal(a1, a2, subst) {
                    return false;
                }
            }
            true
        }
        _ => false,
    }
}

/// Pattern-matching rewrite: Replaces occurrences of rule.lhs with rule.rhs in target
pub fn apply_rewrite(target: &Term, rule: &Equality) -> Vec<Term> {
    let mut results = Vec::new();

    // Try rewriting at the root
    if let Some(subst) = unify(target, &rule.lhs) {
        results.push(rule.rhs.substitute(&subst));
    }

    // Try rewriting in sub-arguments
    if let Term::Func(name, args) = target {
        for (i, arg) in args.iter().enumerate() {
            let sub_rewrites = apply_rewrite(arg, rule);
            for new_arg in sub_rewrites {
                let mut new_args = args.clone();
                new_args[i] = new_arg;
                results.push(Term::Func(name.clone(), new_args));
            }
        }
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unification_and_occurs_check() {
        let x = Term::var("x");
        let y = Term::var("y");
        let a = Term::constant("a");

        let f_x_y = Term::func("f", vec![x.clone(), y.clone()]);
        let f_a_a = Term::func("f", vec![a.clone(), a.clone()]);

        let mgu = unify(&f_x_y, &f_a_a).expect("Must unify successfully");
        assert_eq!(mgu.get("x").unwrap(), &a);
        assert_eq!(mgu.get("y").unwrap(), &a);
    }

    #[test]
    fn test_algebraic_rewrite() {
        // Rule: x + 0 = x
        let x = Term::var("x");
        let zero = Term::constant("0");
        let rule = Equality::new(Term::func("+", vec![x.clone(), zero]), x.clone());

        // Target: (a + 0) + b
        let a = Term::constant("a");
        let b = Term::constant("b");
        let target = Term::func(
            "+",
            vec![
                Term::func("+", vec![a.clone(), Term::constant("0")]),
                b.clone(),
            ],
        );

        let rewritten = apply_rewrite(&target, &rule);
        assert!(!rewritten.is_empty());
        assert_eq!(rewritten[0], Term::func("+", vec![a, b]));
    }
}
