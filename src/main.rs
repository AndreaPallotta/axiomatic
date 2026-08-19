#![allow(unused_variables, dead_code, clippy::all)]

use axiomatic::{
    export_to_lean4, AxiomLibrary, Equality, LemmaDatabase, MctsEngine, ProofState,
    SymbolicNeuralPolicy, Term,
};
use std::env;
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
        "  prove <THEOREMS...>   Run autonomous MCTS proof discovery using trained Neural Net"
    );
    println!("  serve [PORT]          Launch live Web Graphical Dashboard (default: 3000)");
    println!("  demo                  Run autonomous theorem discovery and memory compounding");
    println!("  lean                  Generate and export certified Lean 4 formal proofs\n");
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
            run_autonomous_proof_cli();
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

/// Runs CLI proof discovery on algebraic goals
fn run_autonomous_proof_cli() {
    print_banner();
    println!("[INFO] Initiating Neurosymbolic MCTS Proof Search");
    println!("================================================================================");

    let axioms = AxiomLibrary::standard_algebra();
    let (model, epochs, _) = axiomatic::ModelCheckpoint::try_load_or_init("models");
    let policy = axiomatic::DeepNeuralPolicy::new(model);
    if epochs > 0 {
        println!(
            "[INFO] Using Trained Neural Network (Trained for {} Epochs)",
            epochs
        );
    } else {
        println!("[INFO] Using Initialized Neural Network");
    }

    // Goal: (x + 0) + (y * 1) = (1 * y) + (0 + x)
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

    let target_eq = Equality::new(lhs, rhs);
    println!("[TARGET] Conjecture: {}\n", target_eq);

    let initial_state = ProofState::new(target_eq);
    let mut mcts = MctsEngine::new(initial_state, 8);

    let start = std::time::Instant::now();
    let proof = mcts.run_search(&policy, &axioms, 250);
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

    // Record in database
    database.record_theorem("lemma_add_zero_comm", lemma_eq, p1);
    database.augment_axioms(&mut axioms);
    println!(
        "  [OK] Registered in Knowledge Base. Total active rules: {}\n",
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

/// Exports a discovered proof to Lean 4 syntax
fn run_lean_export_demo() {
    print_banner();
    println!("[INFO] Exporting Discovered Proof to Lean 4 Formal Kernel");
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

    let lean_code = export_to_lean4("add_zero_symmetric", &proof);
    println!("{}", lean_code);
    println!("================================================================================\n");
}
