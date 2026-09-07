use super::kernel::ProofState;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

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

pub struct Lean4Validator;

impl Lean4Validator {
    pub fn lean_command() -> Command {
        if let Ok(out) = Command::new("lean").arg("--version").output() {
            if out.status.success() {
                return Command::new("lean");
            }
        }

        if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
            let p = PathBuf::from(local_app_data).join(r"Microsoft\WinGet\Links\lean.exe");
            if p.exists() {
                return Command::new(p);
            }
        }

        Command::new("lean")
    }

    pub fn get_lean_version() -> Option<String> {
        let output = Self::lean_command().arg("--version").output().ok()?;
        if output.status.success() {
            Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            None
        }
    }

    pub fn validate_file(file_path: &Path) -> LeanValidationResult {
        let lean_version = match Self::get_lean_version() {
            Some(v) => v,
            None => {
                return LeanValidationResult::LeanNotInstalled {
                    message: "Lean 4 compiler (`lean`) is not found in system PATH. Install Lean 4 from https://leanprover.github.io/".to_string(),
                };
            }
        };

        let start = Instant::now();
        let output = Self::lean_command().arg(file_path).output();
        let elapsed = start.elapsed();

        match output {
            Ok(out) => {
                let stderr = String::from_utf8_lossy(&out.stderr).to_string();
                let stdout = String::from_utf8_lossy(&out.stdout).to_string();

                if stdout.contains("warning: declaration uses `sorry`")
                    || stderr.contains("warning: declaration uses `sorry`")
                {
                    return LeanValidationResult::CompilerError {
                        stderr: "Proof contains unproven hypothesis (`sorry`)".to_string(),
                        stdout,
                    };
                }

                if out.status.success() && !stderr.contains("error:") && !stdout.contains("error:")
                {
                    LeanValidationResult::Certified {
                        elapsed_ms: elapsed.as_secs_f64() * 1000.0,
                        lean_version,
                    }
                } else {
                    LeanValidationResult::CompilerError { stderr, stdout }
                }
            }
            Err(e) => LeanValidationResult::CompilerError {
                stderr: format!("Process execution error: {}", e),
                stdout: String::new(),
            },
        }
    }

    pub fn validate_proof(name: &str, state: &ProofState) -> LeanValidationResult {
        let temp_dir = std::env::temp_dir();
        let temp_path = temp_dir.join(format!("{}_axiomatic.lean", name));

        let lean_code = super::lean::export_to_lean4(name, state);
        if let Err(e) = std::fs::write(&temp_path, &lean_code) {
            return LeanValidationResult::CompilerError {
                stderr: format!("Failed to write temporary file: {}", e),
                stdout: String::new(),
            };
        }

        let result = Self::validate_file(&temp_path);
        let _ = std::fs::remove_file(&temp_path);
        result
    }

    pub fn ensure_lean_workspace(dir: &Path) -> std::io::Result<()> {
        if !dir.exists() {
            std::fs::create_dir_all(dir)?;
        }

        let toolchain_path = dir.join("lean-toolchain");
        if !toolchain_path.exists() {
            std::fs::write(&toolchain_path, "leanprover/lean4:v4.33.1\n")?;
        }

        let lakefile_path = dir.join("lakefile.lean");
        if !lakefile_path.exists() {
            let lakefile_content = "import Lake\nopen Lake DSL\n\npackage «axiomatic_proofs» where\n\n@[default_target]\nlean_lib «AxiomaticProofs» where\n  srcDir := \".\"\n";
            std::fs::write(&lakefile_path, lakefile_content)?;
        }

        Ok(())
    }

    pub fn save_and_validate_proof(
        name: &str,
        state: &ProofState,
        output_dir: &Path,
    ) -> (LeanValidationResult, PathBuf) {
        let _ = Self::ensure_lean_workspace(output_dir);
        let file_path = output_dir.join(format!("{}.lean", name));
        if let Err(e) = std::fs::create_dir_all(output_dir) {
            return (
                LeanValidationResult::CompilerError {
                    stderr: format!("Failed to create output directory: {}", e),
                    stdout: String::new(),
                },
                file_path,
            );
        }

        let lean_code = super::lean::export_to_lean4(name, state);
        if let Err(e) = std::fs::write(&file_path, &lean_code) {
            return (
                LeanValidationResult::CompilerError {
                    stderr: format!("Failed to write proof file: {}", e),
                    stdout: String::new(),
                },
                file_path,
            );
        }

        let result = Self::validate_file(&file_path);
        (result, file_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::verifier::fol::{Equality, Term};
    use crate::verifier::kernel::Tactic;

    #[test]
    fn test_lean_validator_with_real_proof() {
        let x = Term::var("x");
        let zero = Term::constant("0");
        let goal = Equality::new(Term::func("+", vec![x.clone(), zero.clone()]), x.clone());
        let mut state = ProofState::new(goal);
        state.proof_history.push((
            Tactic::RewriteLhs("add_zero".to_string()),
            "Rewrote LHS via [add_zero]: x = x".to_string(),
        ));
        state
            .proof_history
            .push((Tactic::Reflexivity, "Solved #1: x = x via rfl".to_string()));

        let res = Lean4Validator::validate_proof("test_lean_verified_thm", &state);
        if Lean4Validator::get_lean_version().is_some() {
            match res {
                LeanValidationResult::Certified {
                    elapsed_ms,
                    ref lean_version,
                } => {
                    assert!(elapsed_ms >= 0.0);
                    assert!(!lean_version.is_empty());
                }
                LeanValidationResult::CompilerError {
                    ref stderr,
                    ref stdout,
                } => {
                    panic!(
                        "Expected Certified, got error: stderr={}, stdout={}",
                        stderr, stdout
                    );
                }
                LeanValidationResult::LeanNotInstalled { .. } => {
                    panic!("Lean should be detected");
                }
            }
        }
    }

    #[test]
    fn test_save_and_validate_proof_artifact() -> Result<(), Box<dyn std::error::Error>> {
        let x = Term::var("x");
        let state = ProofState::new(Equality::new(x.clone(), x.clone()));
        let temp_dir = std::env::temp_dir().join("axiomatic_test_proofs");

        let (res, path) =
            Lean4Validator::save_and_validate_proof("test_save_thm", &state, &temp_dir);
        assert!(path.exists());
        let content = std::fs::read_to_string(&path)?;
        assert!(content.contains("theorem test_save_thm"));

        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir(&temp_dir);

        if Lean4Validator::get_lean_version().is_some() {
            assert!(matches!(res, LeanValidationResult::Certified { .. }));
        }
        Ok(())
    }

    #[test]
    fn test_lean_rejects_bad_proof() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = std::env::temp_dir();
        let bad_path = temp_dir.join("test_bad_manual.lean");
        std::fs::write(
            &bad_path,
            "theorem bad_thm (x y : Nat) : x = y := by\n  rfl\n",
        )?;

        let res = Lean4Validator::validate_file(&bad_path);
        let _ = std::fs::remove_file(&bad_path);

        if Lean4Validator::get_lean_version().is_some() {
            assert!(matches!(res, LeanValidationResult::CompilerError { .. }));
        }
        Ok(())
    }

    #[test]
    fn test_ensure_lean_workspace() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = std::env::temp_dir().join("axiomatic_ws_test");
        Lean4Validator::ensure_lean_workspace(&temp_dir)?;

        assert!(temp_dir.join("lean-toolchain").exists());
        assert!(temp_dir.join("lakefile.lean").exists());

        let _ = std::fs::remove_dir_all(&temp_dir);
        Ok(())
    }
}
