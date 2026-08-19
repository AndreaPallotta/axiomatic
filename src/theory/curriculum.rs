use crate::verifier::fol::{Equality, Term};
use serde::{Deserialize, Serialize};

/// Mathematical Difficulty Tier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DifficultyLevel {
    Level1Identities = 1,
    Level2CommutativeAssociative = 2,
    Level3Distributivity = 3,
    Level4GroupInverses = 4,
    Level5ComplexPolynomials = 5,
}

impl DifficultyLevel {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Level1Identities => "Level 1: Monoid Identities",
            Self::Level2CommutativeAssociative => "Level 2: Commutativity & Associativity",
            Self::Level3Distributivity => "Level 3: Multi-term Distributivity",
            Self::Level4GroupInverses => "Level 4: Inverses & Cancellations",
            Self::Level5ComplexPolynomials => "Level 5: Compound Ring Polynomials",
        }
    }
}

/// Curriculum Learning Controller managing automated difficulty scaling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurriculumController {
    pub current_level: DifficultyLevel,
    pub consecutive_successes: usize,
    pub window_attempts: usize,
    pub window_solved: usize,
    pub graduation_threshold: f64, // Default: 80%
    pub demotion_threshold: f64,    // Default: 30%
}

impl Default for CurriculumController {
    fn default() -> Self {
        Self {
            current_level: DifficultyLevel::Level1Identities,
            consecutive_successes: 0,
            window_attempts: 0,
            window_solved: 0,
            graduation_threshold: 80.0,
            demotion_threshold: 30.0,
        }
    }
}

impl CurriculumController {
    pub fn new() -> Self {
        Self::default()
    }

    /// Records outcome of a theorem proving attempt and adjusts difficulty dynamically
    pub fn record_attempt(&mut self, is_solved: bool) -> Option<DifficultyLevel> {
        self.window_attempts += 1;
        if is_solved {
            self.window_solved += 1;
            self.consecutive_successes += 1;
        } else {
            self.consecutive_successes = 0;
        }

        // Evaluate every 10 attempts
        if self.window_attempts >= 8 {
            let solve_rate = (self.window_solved as f64 / self.window_attempts as f64) * 100.0;
            self.window_attempts = 0;
            self.window_solved = 0;

            if solve_rate >= self.graduation_threshold {
                let next_level = match self.current_level {
                    DifficultyLevel::Level1Identities => DifficultyLevel::Level2CommutativeAssociative,
                    DifficultyLevel::Level2CommutativeAssociative => DifficultyLevel::Level3Distributivity,
                    DifficultyLevel::Level3Distributivity => DifficultyLevel::Level4GroupInverses,
                    DifficultyLevel::Level4GroupInverses => DifficultyLevel::Level5ComplexPolynomials,
                    DifficultyLevel::Level5ComplexPolynomials => DifficultyLevel::Level5ComplexPolynomials,
                };
                if next_level != self.current_level {
                    self.current_level = next_level;
                    return Some(self.current_level);
                }
            } else if solve_rate < self.demotion_threshold {
                let prev_level = match self.current_level {
                    DifficultyLevel::Level5ComplexPolynomials => DifficultyLevel::Level4GroupInverses,
                    DifficultyLevel::Level4GroupInverses => DifficultyLevel::Level3Distributivity,
                    DifficultyLevel::Level3Distributivity => DifficultyLevel::Level2CommutativeAssociative,
                    DifficultyLevel::Level2CommutativeAssociative => DifficultyLevel::Level1Identities,
                    DifficultyLevel::Level1Identities => DifficultyLevel::Level1Identities,
                };
                if prev_level != self.current_level {
                    self.current_level = prev_level;
                    return Some(self.current_level);
                }
            }
        }

        None
    }

    /// Synthesizes a new algebraic conjecture tailored to the current curriculum difficulty
    pub fn generate_conjecture(&self, seed: usize) -> Equality {
        let x = Term::constant("x");
        let y = Term::constant("y");
        let z = Term::constant("z");
        let zero = Term::constant("0");
        let one = Term::constant("1");

        match self.current_level {
            DifficultyLevel::Level1Identities => {
                match seed % 3 {
                    0 => Equality::new(
                        Term::func("+", vec![x.clone(), zero.clone()]),
                        Term::func("+", vec![zero.clone(), x.clone()]),
                    ),
                    1 => Equality::new(
                        Term::func("*", vec![x.clone(), one.clone()]),
                        Term::func("*", vec![one.clone(), x.clone()]),
                    ),
                    _ => Equality::new(
                        Term::func("+", vec![y.clone(), zero.clone()]),
                        y.clone(),
                    ),
                }
            }
            DifficultyLevel::Level2CommutativeAssociative => {
                match seed % 3 {
                    0 => Equality::new(
                        Term::func("+", vec![Term::func("+", vec![x.clone(), y.clone()]), zero.clone()]),
                        Term::func("+", vec![y.clone(), x.clone()]),
                    ),
                    1 => Equality::new(
                        Term::func("+", vec![x.clone(), Term::func("+", vec![y.clone(), z.clone()])]),
                        Term::func("+", vec![Term::func("+", vec![z.clone(), y.clone()]), x.clone()]),
                    ),
                    _ => Equality::new(
                        Term::func("*", vec![Term::func("*", vec![x.clone(), y.clone()]), one.clone()]),
                        Term::func("*", vec![y.clone(), x.clone()]),
                    ),
                }
            }
            DifficultyLevel::Level3Distributivity => {
                match seed % 3 {
                    0 => Equality::new(
                        Term::func("+", vec![Term::func("+", vec![x.clone(), zero.clone()]), Term::func("*", vec![y.clone(), one.clone()])]),
                        Term::func("+", vec![Term::func("*", vec![one.clone(), y.clone()]), Term::func("+", vec![zero.clone(), x.clone()])]),
                    ),
                    1 => Equality::new(
                        Term::func("*", vec![x.clone(), Term::func("+", vec![y.clone(), zero.clone()])]),
                        Term::func("*", vec![y.clone(), x.clone()]),
                    ),
                    _ => Equality::new(
                        Term::func("*", vec![one.clone(), Term::func("+", vec![x.clone(), y.clone()])]),
                        Term::func("+", vec![y.clone(), x.clone()]),
                    ),
                }
            }
            DifficultyLevel::Level4GroupInverses => {
                match seed % 2 {
                    0 => Equality::new(
                        Term::func("+", vec![Term::func("+", vec![x.clone(), Term::func("-", vec![x.clone()])]), y.clone()]),
                        Term::func("+", vec![zero.clone(), y.clone()]),
                    ),
                    _ => Equality::new(
                        Term::func("+", vec![x.clone(), Term::func("+", vec![y.clone(), Term::func("-", vec![y.clone()])])]),
                        Term::func("+", vec![x.clone(), zero.clone()]),
                    ),
                }
            }
            DifficultyLevel::Level5ComplexPolynomials => {
                Equality::new(
                    Term::func(
                        "+",
                        vec![
                            Term::func("*", vec![Term::func("+", vec![x.clone(), zero.clone()]), one.clone()]),
                            Term::func("*", vec![Term::func("+", vec![y.clone(), zero.clone()]), one.clone()]),
                        ],
                    ),
                    Term::func("+", vec![y.clone(), x.clone()]),
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_curriculum_graduation() {
        let mut curr = CurriculumController::new();
        assert_eq!(curr.current_level, DifficultyLevel::Level1Identities);

        // 8 consecutive successful proofs
        for _ in 0..8 {
            curr.record_attempt(true);
        }

        assert_eq!(curr.current_level, DifficultyLevel::Level2CommutativeAssociative);
    }
}
