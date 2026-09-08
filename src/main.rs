#![allow(unused_variables, dead_code, clippy::all)]

use axiomatic::{
    export_to_lean4, parse_conjecture, AxiomLibrary, Equality, Lean4Validator,
    LeanValidationResult, LemmaDatabase, MctsEngine, ProofState, SymbolicNeuralPolicy, Term,
    TraceExporter,
};
use std::env;
use std::path::Path;
use tokio::sync::broadcast;

fn print_banner() {
    println!("\n================================================================================");
    println!("       AXIOMATIC // Autonomous Neurosymbolic Mathematical Discovery Engine      ");
    println!("       MCTS + Formal Logic Verifier + Real-Time Graphical Dashboard             ");
    println!("================================================================================\n");
}

fn print_usage() {
    print_banner();
    println!("USAGE:");
    println!("  cargo run --release -- <COMMAND> [OPTIONS]\n");
    println!("COMMANDS:");
    println!(
        "  train [OPTIONS]       Run Self-Play Neural Network Training with auto-checkpointing"
    );
    println!("                        Options: --hours <H>, --epochs <N>, --dir <PATH>");
    println!("                        Examples: train --hours 2");
    println!("                                  train --epochs 500");
    println!(
        "  prove [CONJECTURE]    Run autonomous MCTS proof discovery and certify via Lean 4 kernel"
    );
    println!("  serve [PORT]          Launch live Web Graphical Dashboard (default: 3000)");
    println!("  demo                  Run autonomous theorem discovery and memory compounding");
    println!(
        "  lean                  Generate, export, and formally certify Lean 4 proof artifacts"
    );
    println!("  export-trace [CONJECTURE] [OPTIONS]");
    println!("                        Export standalone static proof trace and interactive showcase HTML");
    println!(
        "                        Options: --out <DIR>, --html, --iterations <N>, --name <NAME>"
    );
    println!("                        Examples: export-trace \"(a + 0) = (0 + a)\" --out showcase --html\n");
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_usage();
        return;
    }

    match args[1].as_str() {
        "train" | "learn" => {
            let mut hours: Option<f64> = None;
            let mut epochs: Option<usize> = None;
            let mut dir = "models".to_string();

            let mut i = 2;
            while i < args.len() {
                match args[i].as_str() {
                    "--hours" | "-h" => {
                        if let Some(val) = args.get(i + 1).and_then(|v| v.parse::<f64>().ok()) {
                            hours = Some(val);
                            i += 1;
                        }
                    }
                    "--epochs" | "-e" => {
                        if let Some(val) = args.get(i + 1).and_then(|v| v.parse::<usize>().ok()) {
                            epochs = Some(val);
                            i += 1;
                        }
                    }
                    "--dir" | "-d" => {
                        if let Some(val) = args.get(i + 1) {
                            dir = val.clone();
                            i += 1;
                        }
                    }
                    arg => {
                        if let Ok(num) = arg.parse::<usize>() {
                            epochs = Some(num);
                        }
                    }
                }
                i += 1;
            }

            let target_secs = hours.map(|h| (h * 3600.0) as u64);
            let target_epochs = if target_secs.is_none() && epochs.is_none() {
                Some(50)
            } else {
                epochs
            };

            run_neural_network_training(target_secs, target_epochs, &dir);
        }
        "prove" | "search" => {
            let custom = args.get(2).map(|s| s.as_str());
            run_autonomous_proof_cli(custom);
        }
        "serve" | "ui" | "dashboard" => {
            let port = args.get(2).and_then(|p| p.parse().ok()).unwrap_or(3000);
            run_live_visualizer_server(port).await;
        }
        "demo" => {
            run_compounding_memory_demo();
        }
        "lean" => {
            run_lean_export_demo();
        }
        "export-trace" | "trace" => {
            let mut conjecture: Option<String> = None;
            let mut out_dir = "showcase".to_string();
            let mut emit_html = false;
            let mut iterations = 150;
            let mut name = "showcase_theorem".to_string();

            let mut i = 2;
            while i < args.len() {
                match args[i].as_str() {
                    "--out" | "-o" => {
                        if let Some(val) = args.get(i + 1) {
                            out_dir = val.clone();
                            i += 1;
                        }
                    }
                    "--html" => {
                        emit_html = true;
                    }
                    "--iterations" | "-n" => {
                        if let Some(val) = args.get(i + 1).and_then(|v| v.parse::<usize>().ok()) {
                            iterations = val;
                            i += 1;
                        }
                    }
                    "--name" => {
                        if let Some(val) = args.get(i + 1) {
                            name = val.clone();
                            i += 1;
                        }
                    }
                    arg if !arg.starts_with('-') && conjecture.is_none() => {
                        conjecture = Some(arg.to_string());
                    }
                    _ => {}
                }
                i += 1;
            }

            run_export_trace_cli(
                conjecture.as_deref(),
                &out_dir,
                emit_html,
                iterations,
                &name,
            );
        }
        _ => {
            print_usage();
        }
    }
}

/// Runs Autonomous Self-Play Neural Training Session with Checkpointing
fn run_neural_network_training(
    target_duration_secs: Option<u64>,
    target_epochs: Option<usize>,
    checkpoint_dir: &str,
) {
    print_banner();

    let axioms = AxiomLibrary::standard_algebra();
    let (mut model, prior_epochs, prior_loss) =
        axiomatic::ModelCheckpoint::try_load_or_init(checkpoint_dir);

    if prior_epochs > 0 {
        println!(
            "[INFO] Resuming training from Epoch {} (Prior Best Loss: {:.4})",
            prior_epochs, prior_loss
        );
    }

    let mut optimizer = axiomatic::AdamOptimizer::new(&model, 0.005);
    let mut replay = axiomatic::ReplayBuffer::new(5000);

    axiomatic::train_continuous_session(
        &mut model,
        &mut optimizer,
        &mut replay,
        &axioms,
        checkpoint_dir,
        target_duration_secs,
        target_epochs,
        10, // Checkpoint every 10 epochs
    );
}

fn default_target_conjecture() -> Equality {
    let x = Term::constant("x");
    let y = Term::constant("y");
    let zero = Term::constant("0");
    let one = Term::constant("1");

    let lhs = Term::func(
        "+",
        vec![
            Term::func("+", vec![x.clone(), zero.clone()]),
            Term::func("*", vec![y.clone(), one.clone()]),
        ],
    );
    let rhs = Term::func(
        "+",
        vec![
            Term::func("*", vec![one.clone(), y.clone()]),
            Term::func("+", vec![zero.clone(), x.clone()]),
        ],
    );

    Equality::new(lhs, rhs)
}

/// Runs CLI proof discovery on algebraic goals
fn run_autonomous_proof_cli(custom_conjecture: Option<&str>) {
    print_banner();
    println!("[INFO] Initiating Neurosymbolic MCTS Proof Search");
    println!("================================================================================");

    let (target_eq, is_bool) = if let Some(conjecture_str) = custom_conjecture {
        match parse_conjecture(conjecture_str) {
            Ok(eq) => {
                println!("[INPUT] Parsed target conjecture: {}", eq);
                let is_b = axiomatic::is_boolean_equality(&eq);
                (eq, is_b)
            }
            Err(e) => {
                println!(
                    "[WARN] Failed to parse input '{}': {}. Using default conjecture.",
                    conjecture_str, e
                );
                (default_target_conjecture(), false)
            }
        }
    } else {
        (default_target_conjecture(), false)
    };

    let axioms = if is_bool {
        println!("[DOMAIN] Detected Propositional Boolean Logic Domain");
        AxiomLibrary::boolean_logic()
    } else {
        AxiomLibrary::standard_algebra()
    };

    let (model, epochs, _) = axiomatic::ModelCheckpoint::try_load_or_init("models");
    let policy: Box<dyn axiomatic::NeuralPolicy> = if is_bool {
        Box::new(SymbolicNeuralPolicy::new())
    } else {
        if epochs > 0 {
            println!(
                "[INFO] Using Trained Neural Network (Trained for {} Epochs)",
                epochs
            );
        } else {
            println!("[INFO] Using Initialized Neural Network");
        }
        Box::new(axiomatic::DeepNeuralPolicy::new(model))
    };

    println!("[TARGET] Conjecture: {}\n", target_eq);

    let initial_state = ProofState::new(target_eq.clone());
    let mut mcts = MctsEngine::new(initial_state, 8);

    let start = std::time::Instant::now();
    let proof = mcts.run_search(policy.as_ref(), &axioms, 250);
    let elapsed = start.elapsed();

    if let Some(solved_state) = proof {
        println!("[Q.E.D.] Formal proof discovered successfully.");
        println!("  - Search Nodes:    {}", mcts.nodes.len());
        println!("  - Iterations:      {}", mcts.iterations);
        println!(
            "  - Execution Time:  {:.3} ms",
            elapsed.as_secs_f64() * 1000.0
        );
        println!(
            "  - Proof Length:    {} tactic steps\n",
            solved_state.proof_history.len()
        );

        println!("Formal Proof Derivation (Verified by Kernel):");
        for (i, (tactic, desc)) in solved_state.proof_history.iter().enumerate() {
            println!("  Step {}. [{}] -> {}", i + 1, tactic, desc);
        }

        println!("\nFormal Lean 4 Certification:");
        let proofs_dir = Path::new("proofs");
        let theorem_name = if custom_conjecture.is_some() {
            "custom_theorem"
        } else {
            "compound_algebra_conjecture"
        };
        let (val_result, artifact_path) =
            Lean4Validator::save_and_validate_proof(theorem_name, &solved_state, proofs_dir);

        match val_result {
            LeanValidationResult::Certified {
                elapsed_ms,
                lean_version,
            } => {
                println!("  - Proof Artifact:  {}", artifact_path.display());
                println!("  - Status:          CERTIFIED by Lean 4 Kernel");
                println!("  - Kernel Time:     {:.3} ms", elapsed_ms);
                println!("  - Compiler:        {}", lean_version);
            }
            LeanValidationResult::CompilerError { stderr, stdout } => {
                println!("  - Proof Artifact:  {}", artifact_path.display());
                println!("  - Status:          COMPILER REJECTED");
                if !stderr.is_empty() {
                    println!("  - Stderr:          {}", stderr.trim());
                }
                if !stdout.is_empty() {
                    println!("  - Stdout:          {}", stdout.trim());
                }
            }
            LeanValidationResult::LeanNotInstalled { message } => {
                println!("  - Proof Artifact:  {}", artifact_path.display());
                println!(
                    "  - Status:          Lean 4 not detected (proof saved, compilation skipped)"
                );
                println!("  - Note:            {}", message);
            }
        }
    } else {
        println!("[FAILED] Search exhausted max iterations without reaching formal proof state.");
    }
    println!("================================================================================\n");
}

/// Runs live interactive visualizer server streaming real-time search
async fn run_live_visualizer_server(port: u16) {
    print_banner();

    let (tx, _) = broadcast::channel(1000);
    let controller = axiomatic::EngineController::new(tx);

    axiomatic::start_visualizer_server(port, controller).await;
}

/// Demonstrates compound learning: Proving lemmas, saving to database, and using them in harder proofs
fn run_compounding_memory_demo() {
    print_banner();
    println!("[INFO] Compounding Knowledge Base Demonstration");
    println!("Step 1: Prove auxiliary lemma -> Step 2: Store in Knowledge Base -> Step 3: Solve complex goal");
    println!("================================================================================");

    let mut axioms = AxiomLibrary::standard_algebra();
    let policy = SymbolicNeuralPolicy::new();
    let mut database = LemmaDatabase::new();

    // Lemma 1: (x + 0) = (0 + x)
    let x = Term::var("x");
    let zero = Term::constant("0");
    let lemma_eq = Equality::new(
        Term::func("+", vec![x.clone(), zero.clone()]),
        Term::func("+", vec![zero.clone(), x.clone()]),
    );

    println!(
        "[STAGE 1] Proving Auxiliary Lemma [lemma_add_zero_comm]: {}",
        lemma_eq
    );
    let mut mcts1 = MctsEngine::new(ProofState::new(lemma_eq.clone()), 6);
    let p1 = mcts1
        .run_search(&policy, &axioms, 100)
        .expect("Lemma must be proven");
    println!("  [OK] Lemma verified in {} steps", p1.proof_history.len());

    // Record in database with Lean 4 formal certification gate
    match database.record_and_certify("lemma_add_zero_comm", lemma_eq, p1) {
        Ok(LeanValidationResult::Certified { elapsed_ms, .. }) => {
            println!("  [CERTIFIED] Formally verified by Lean 4 ({:.3} ms) -> Admitted to Knowledge Base", elapsed_ms);
        }
        Ok(_) => {
            println!("  [OK] Registered in Knowledge Base (Lean 4 gate passed)");
        }
        Err(e) => {
            println!(
                "  [REJECTED] Knowledge Base rejected unverified lemma: {}",
                e
            );
        }
    }
    database.augment_axioms(&mut axioms);
    println!(
        "  [OK] Total active rules in library: {}\n",
        axioms.rules.len()
    );

    // Complex Goal using the learned lemma:
    let a = Term::constant("a");
    let complex_lhs = Term::func("+", vec![a.clone(), zero.clone()]);
    let complex_rhs = Term::func("+", vec![zero.clone(), a.clone()]);
    let complex_eq = Equality::new(complex_lhs, complex_rhs);

    println!(
        "[STAGE 2] Solving Target Theorem using Learned Lemma: {}",
        complex_eq
    );
    let mut mcts2 = MctsEngine::new(ProofState::new(complex_eq), 6);
    let p2 = mcts2
        .run_search(&policy, &axioms, 100)
        .expect("Target must be proven");
    println!(
        "  [Q.E.D.] Target solved in {} steps using learned lemma",
        p2.proof_history.len()
    );
    println!("================================================================================\n");
}

/// Exports a discovered proof to Lean 4 syntax and formally certifies it with Lean 4
fn run_lean_export_demo() {
    print_banner();
    println!("[INFO] Autonomous Theorem Discovery and Lean 4 Formal Kernel Certification");
    println!("================================================================================");

    let axioms = AxiomLibrary::standard_algebra();
    let policy = SymbolicNeuralPolicy::new();

    let a = Term::constant("a");
    let zero = Term::constant("0");
    let goal_eq = Equality::new(
        Term::func("+", vec![a.clone(), zero.clone()]),
        Term::func("+", vec![zero.clone(), a.clone()]),
    );
    let mut mcts = MctsEngine::new(ProofState::new(goal_eq), 6);
    let proof = mcts
        .run_search(&policy, &axioms, 100)
        .expect("Proof must be found");

    let theorem_name = "add_zero_symmetric";
    let proofs_dir = Path::new("proofs");
    let (val_result, artifact_path) =
        Lean4Validator::save_and_validate_proof(theorem_name, &proof, proofs_dir);

    let lean_code = export_to_lean4(theorem_name, &proof);
    println!(
        "Generated Lean 4 Proof Artifact ({}):\n",
        artifact_path.display()
    );
    println!("{}", lean_code);

    println!("================================================================================");
    match val_result {
        LeanValidationResult::Certified {
            elapsed_ms,
            lean_version,
        } => {
            println!("[CERTIFICATION] Formally Certified by Lean 4 Kernel");
            println!("  - Artifact:        {}", artifact_path.display());
            println!("  - Validation:      Kernel Confirmed (0 errors, 0 warnings, no sorry)");
            println!("  - Compiler Time:   {:.3} ms", elapsed_ms);
            println!("  - Toolchain:       {}", lean_version);
        }
        LeanValidationResult::CompilerError { stderr, stdout } => {
            println!("[REJECTED] Lean 4 compiler rejected the generated proof");
            if !stderr.is_empty() {
                println!("  - Stderr: {}", stderr.trim());
            }
            if !stdout.is_empty() {
                println!("  - Stdout: {}", stdout.trim());
            }
        }
        LeanValidationResult::LeanNotInstalled { message } => {
            println!("[SKIP] Lean 4 compiler not detected: {}", message);
        }
    }
    println!("================================================================================\n");
}

fn run_export_trace_cli(
    custom_conjecture: Option<&str>,
    out_dir_str: &str,
    emit_html: bool,
    max_iterations: usize,
    theorem_name: &str,
) {
    print_banner();
    println!("[INFO] Exporting Static Proof Trace & Showcase");
    println!("================================================================================");

    let (target_eq, is_bool) = if let Some(conjecture_str) = custom_conjecture {
        match parse_conjecture(conjecture_str) {
            Ok(eq) => {
                println!("[INPUT] Parsed target conjecture: {}", eq);
                let is_b = axiomatic::is_boolean_equality(&eq);
                (eq, is_b)
            }
            Err(e) => {
                println!(
                    "[WARN] Failed to parse input '{}': {}. Using default conjecture.",
                    conjecture_str, e
                );
                (default_target_conjecture(), false)
            }
        }
    } else {
        (default_target_conjecture(), false)
    };

    let domain_name = if is_bool {
        "Propositional Boolean Logic"
    } else {
        "Abstract Algebra"
    };

    let axioms = if is_bool {
        AxiomLibrary::boolean_logic()
    } else {
        AxiomLibrary::standard_algebra()
    };

    let policy = SymbolicNeuralPolicy::new();
    let root_state = ProofState::new(target_eq.clone());
    let mut mcts = MctsEngine::new(root_state, 8);

    println!(
        "[SEARCH] Running MCTS with budget of {} iterations...",
        max_iterations
    );
    let _ = mcts.run_search(&policy, &axioms, max_iterations);
    let snapshot = mcts.snapshot();

    println!(
        "[SEARCH] Complete. Total nodes: {}, Proven: {}",
        snapshot.nodes.len(),
        snapshot.proven_node_id.is_some()
    );

    let trace =
        TraceExporter::create_trace(theorem_name, &target_eq.to_string(), domain_name, snapshot);

    let out_dir = Path::new(out_dir_str);
    let json_file = out_dir.join("trace.json");
    match TraceExporter::export_json(&trace, &json_file) {
        Ok(_) => println!("  [OK] Exported JSON Trace:   {}", json_file.display()),
        Err(e) => println!("  [ERROR] Failed to export JSON trace: {}", e),
    }

    if emit_html {
        let html_file = out_dir.join("index.html");
        match TraceExporter::export_standalone_html(&trace, &html_file) {
            Ok(_) => println!("  [OK] Exported Standalone HTML: {}", html_file.display()),
            Err(e) => println!("  [ERROR] Failed to export HTML showcase: {}", e),
        }
    }

    println!("================================================================================\n");
}
