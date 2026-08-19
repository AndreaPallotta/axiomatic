use crate::verifier::fol::Equality;
use crate::verifier::kernel::{AxiomLibrary, ProofState};
use serde::{Deserialize, Serialize};

/// A machine-proven theorem stored in the persistent Knowledge Base
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifiedTheorem {
    pub name: String,
    pub statement: Equality,
    pub proof_length: usize,
    pub proof_state: ProofState,
    pub timestamp: String,
}

/// The Knowledge Base / Lemma Hall-of-Fame
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LemmaDatabase {
    pub theorems: Vec<VerifiedTheorem>,
}

impl LemmaDatabase {
    pub fn new() -> Self {
        Self {
            theorems: Vec::new(),
        }
    }

    /// Registers a newly discovered and verified theorem
    pub fn record_theorem(
        &mut self,
        name: &str,
        statement: Equality,
        proof_state: ProofState,
    ) {
        let entry = VerifiedTheorem {
            name: name.to_string(),
            statement,
            proof_length: proof_state.proof_history.len(),
            proof_state,
            timestamp: chrono::Utc::now().to_rfc3339(),
        };
        self.theorems.push(entry);
    }

    /// Exports all proven theorems back into the AxiomLibrary as active rewrite rules
    pub fn augment_axioms(&self, lib: &mut AxiomLibrary) {
        for thm in &self.theorems {
            lib.add_rule(&thm.name, thm.statement.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::verifier::fol::Term;

    #[test]
    fn test_record_and_augment_axioms() {
        let mut db = LemmaDatabase::new();
        let eq = Equality::new(
            Term::func("+", vec![Term::var("x"), Term::constant("0")]),
            Term::var("x"),
        );
        db.record_theorem("lemma_add_zero", eq.clone(), ProofState::new(eq));

        let mut lib = AxiomLibrary::empty();
        db.augment_axioms(&mut lib);
        assert_eq!(lib.rules.len(), 1);
        assert_eq!(lib.rules[0].0, "lemma_add_zero");
    }
}
