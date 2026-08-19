use super::kernel::ProofState;
use serde::{Deserialize, Serialize};
use std::process::Command;
use std::time::Instant;

/// Result of running Lean 4 compiler on generated code
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LeanValidationResult {
    Certified {
        elapsed_ms: f64,
        lean_version: String,
    },
    CompilerError {
        stderr: String,
        stdout: String,
    },
    LeanNotInstalled {
        message: String,
    },
}

/// Runs official Lean 4 executable to formally check exported proofs
pub struct Lean4Validator;

impl Lean4Validator {
    /// Detects if Lean 4 is installed
    pub fn get_lean_version() -> Option<String> {
        let output = Command::new("lean").arg("--version").output().ok()?;
        if output.status.success() {
            Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            None
        }
    }

    /// Validates a ProofState with official Lean 4 compiler
    pub fn validate_proof(name: &str, state: &ProofState) -> LeanValidationResult {
        let lean_version = match Self::get_lean_version() {
            Some(v) => v,
            None => {
                return LeanValidationResult::LeanNotInstalled {
                    message: "Lean 4 compiler (`lean`) is not found in system PATH. Install Lean 4 from https://leanprover.github.io/".to_string(),
                };
            }
        };

        let lean_code = super::lean::export_to_lean4(name, state);
        let temp_dir = std::env::temp_dir();
        let temp_path = temp_dir.join(format!("{}_axiomatic.lean", name));

        if let Err(e) = std::fs::write(&temp_path, &lean_code) {
            return LeanValidationResult::CompilerError {
                stderr: format!("Failed to write temporary file: {}", e),
                stdout: String::new(),
            };
        }

        let start = Instant::now();
        let output = Command::new("lean")
            .arg(temp_path.to_str().unwrap_or("proof.lean"))
            .output();

        let elapsed = start.elapsed();
        let _ = std::fs::remove_file(&temp_path);

        match output {
            Ok(out) => {
                if out.status.success() {
                    LeanValidationResult::Certified {
                        elapsed_ms: elapsed.as_secs_f64() * 1000.0,
                        lean_version,
                    }
                } else {
                    LeanValidationResult::CompilerError {
                        stderr: String::from_utf8_lossy(&out.stderr).to_string(),
                        stdout: String::from_utf8_lossy(&out.stdout).to_string(),
                    }
                }
            }
            Err(e) => LeanValidationResult::CompilerError {
                stderr: format!("Process execution error: {}", e),
                stdout: String::new(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::verifier::fol::Equality;

    #[test]
    fn test_lean_validator_fallback() {
        let state = ProofState::new(Equality::new(
            crate::verifier::fol::Term::constant("x"),
            crate::verifier::fol::Term::constant("x"),
        ));
        let res = Lean4Validator::validate_proof("test_thm", &state);
        match res {
            LeanValidationResult::Certified { .. }
            | LeanValidationResult::CompilerError { .. }
            | LeanValidationResult::LeanNotInstalled { .. } => {}
        }
    }
}
