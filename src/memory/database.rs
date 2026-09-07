use crate::verifier::fol::Equality;
use crate::verifier::kernel::{AxiomLibrary, ProofState};
use crate::verifier::lean_runner::{Lean4Validator, LeanValidationResult};
use serde::{Deserialize, Serialize};

/// A machine-proven theorem stored in the persistent Knowledge Base
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifiedTheorem {
    pub name: String,
    pub statement: Equality,
    pub proof_length: usize,
    pub proof_state: ProofState,
    pub timestamp: String,
    #[serde(default)]
    pub lean_certified: bool,
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
    pub fn record_theorem(&mut self, name: &str, statement: Equality, proof_state: ProofState) {
        let entry = VerifiedTheorem {
            name: name.to_string(),
            statement,
            proof_length: proof_state.proof_history.len(),
            proof_state,
            timestamp: chrono::Utc::now().to_rfc3339(),
            lean_certified: false,
        };
        self.theorems.push(entry);
    }

    /// Validates with Lean 4 kernel before admitting into the Knowledge Base
    pub fn record_and_certify(
        &mut self,
        name: &str,
        statement: Equality,
        proof_state: ProofState,
    ) -> Result<LeanValidationResult, String> {
        let val_result = Lean4Validator::validate_proof(name, &proof_state);
        let certified = matches!(val_result, LeanValidationResult::Certified { .. });

        if Lean4Validator::get_lean_version().is_some() && !certified {
            return Err(format!(
                "Theorem '{}' rejected: formal Lean 4 verification failed: {:?}",
                name, val_result
            ));
        }

        let entry = VerifiedTheorem {
            name: name.to_string(),
            statement,
            proof_length: proof_state.proof_history.len(),
            proof_state,
            timestamp: chrono::Utc::now().to_rfc3339(),
            lean_certified: certified,
        };
        self.theorems.push(entry);
        Ok(val_result)
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
    use crate::verifier::kernel::Tactic;

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

    #[test]
    fn test_record_and_certify_success() -> Result<(), Box<dyn std::error::Error>> {
        let mut db = LemmaDatabase::new();
        let x = Term::var("x");
        let zero = Term::constant("0");
        let eq = Equality::new(Term::func("+", vec![x.clone(), zero.clone()]), x.clone());
        let mut state = ProofState::new(eq.clone());
        state.proof_history.push((
            Tactic::RewriteLhs("add_zero".to_string()),
            "Rewrote LHS via [add_zero]: x = x".to_string(),
        ));
        state
            .proof_history
            .push((Tactic::Reflexivity, "Solved #1: x = x via rfl".to_string()));

        let res = db.record_and_certify("cert_lemma_add_zero", eq, state);
        assert!(res.is_ok());
        assert_eq!(db.theorems.len(), 1);
        if Lean4Validator::get_lean_version().is_some() {
            assert!(db.theorems[0].lean_certified);
        }
        Ok(())
    }
}
