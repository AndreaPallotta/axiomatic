use super::embedding::vectorize_proof_state;
use super::model::{DeepNeuralPolicy, DeepProofNetwork, NUM_TACTIC_CLASSES};
use super::optim::{AdamOptimizer, TrainingSample};
use crate::search::mcts::MctsEngine;
use crate::verifier::fol::{Equality, Term};
use crate::verifier::kernel::{AxiomLibrary, ProofState};
use rand::Rng;
use serde::{Deserialize, Serialize};

/// Training progress metrics per epoch
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingMetrics {
    pub epoch: usize,
    pub total_loss: f64,
    pub policy_cross_entropy: f64,
    pub value_mean_squared_error: f64,
    pub self_play_theorems_solved: usize,
    pub self_play_total_theorems: usize,
    pub solve_rate_percent: f64,
    pub total_samples_trained: usize,
}

/// Experience Replay Buffer
pub struct ReplayBuffer {
    pub samples: Vec<TrainingSample>,
    pub max_size: usize,
}

impl ReplayBuffer {
    pub fn new(max_size: usize) -> Self {
        Self {
            samples: Vec::new(),
            max_size,
        }
    }

    pub fn push(&mut self, sample: TrainingSample) {
        if self.samples.len() >= self.max_size {
            self.samples.remove(0);
        }
        self.samples.push(sample);
    }
}

/// Generates diverse synthetic algebraic conjectures for Self-Play
pub fn generate_synthetic_algebra_conjecture(difficulty: usize) -> Equality {
    let mut rng = rand::thread_rng();
    let vars = ["x", "y", "z", "a", "b"];
    let v1 = Term::constant(vars[rng.gen_range(0..vars.len())]);
    let v2 = Term::constant(vars[rng.gen_range(0..vars.len())]);
    let zero = Term::constant("0");
    let one = Term::constant("1");

    match difficulty % 4 {
        0 => {
            // Identity: (v + 0) = (0 + v)
            Equality::new(
                Term::func("+", vec![v1.clone(), zero.clone()]),
                Term::func("+", vec![zero.clone(), v1.clone()]),
            )
        }
        1 => {
            // Multiplication Identity: (v * 1) = (1 * v)
            Equality::new(
                Term::func("*", vec![v1.clone(), one.clone()]),
                Term::func("*", vec![one.clone(), v1.clone()]),
            )
        }
        2 => {
            // Compound Commutativity: (v1 + 0) + (v2 * 1) = (1 * v2) + (0 + v1)
            let lhs = Term::func(
                "+",
                vec![
                    Term::func("+", vec![v1.clone(), zero.clone()]),
                    Term::func("*", vec![v2.clone(), one.clone()]),
                ],
            );
            let rhs = Term::func(
                "+",
                vec![
                    Term::func("*", vec![one.clone(), v2.clone()]),
                    Term::func("+", vec![zero.clone(), v1.clone()]),
                ],
            );
            Equality::new(lhs, rhs)
        }
        _ => {
            // Commutative sum: (v1 + v2) = (v2 + v1)
            Equality::new(
                Term::func("+", vec![v1.clone(), v2.clone()]),
                Term::func("+", vec![v2.clone(), v1.clone()]),
            )
        }
    }
}

/// Runs self-play MCTS to collect training samples from search tree trajectories
pub fn collect_self_play_trajectory(
    model: &DeepProofNetwork,
    axioms: &AxiomLibrary,
    conjecture: Equality,
    max_mcts_steps: usize,
) -> (Vec<TrainingSample>, bool) {
    let initial_state = ProofState::new(conjecture);
    let mut mcts = MctsEngine::new(initial_state, 6);
    let policy = DeepNeuralPolicy::new(model.clone());

    let solved = mcts.run_search(&policy, axioms, max_mcts_steps);
    let is_proven = solved.is_some();

    let reward_engine = super::reward::RewardEngine::new(super::reward::RewardConfig::default());
    let mut samples = Vec::new();

    // Extract training pairs from explored MCTS nodes with RL discounted returns
    for node in &mcts.nodes {
        if node.visit_count > 0 && node.is_expanded && !node.children_ids.is_empty() {
            let x = vectorize_proof_state(&node.state);

            // Compute MCTS improved policy target: \pi_i = N(child_i) / \sum N(child_j)
            let total_visits: usize = node
                .children_ids
                .iter()
                .map(|&cid| mcts.nodes[cid].visit_count)
                .sum();

            if total_visits > 0 {
                let mut target_pi = vec![0.0; NUM_TACTIC_CLASSES];
                for &cid in &node.children_ids {
                    let child = &mcts.nodes[cid];
                    if let Some(ref tactic) = child.applied_tactic {
                        let class_idx = DeepProofNetwork::tactic_to_index(tactic, axioms);
                        let p = (child.visit_count as f64) / (total_visits as f64);
                        target_pi[class_idx.min(NUM_TACTIC_CLASSES - 1)] += p;
                    }
                }

                // Compute RL Return via Bellman equation
                let trajectory_states = vec![node.state.clone()];
                let computed_returns =
                    reward_engine.compute_trajectory_returns(&trajectory_states, is_proven);
                let target_return =
                    computed_returns
                        .first()
                        .copied()
                        .unwrap_or(if is_proven { 1.0 } else { -0.5 });

                samples.push(TrainingSample {
                    x,
                    target_policy: target_pi,
                    target_value: target_return,
                });
            }
        }
    }

    (samples, is_proven)
}

/// Converts an existing MCTS tree into training samples for immediate reinforcement learning
pub fn convert_mcts_tree_to_training_samples(
    mcts: &MctsEngine,
    axioms: &AxiomLibrary,
    is_proven: bool,
) -> Vec<TrainingSample> {
    let reward_engine = super::reward::RewardEngine::new(super::reward::RewardConfig::default());
    let mut samples = Vec::new();

    for node in &mcts.nodes {
        if node.visit_count > 0 && node.is_expanded && !node.children_ids.is_empty() {
            let x = vectorize_proof_state(&node.state);
            let total_visits: usize = node
                .children_ids
                .iter()
                .map(|&cid| mcts.nodes[cid].visit_count)
                .sum();

            if total_visits > 0 {
                let mut target_pi = vec![0.0; NUM_TACTIC_CLASSES];
                for &cid in &node.children_ids {
                    let child = &mcts.nodes[cid];
                    if let Some(ref tactic) = child.applied_tactic {
                        let class_idx = DeepProofNetwork::tactic_to_index(tactic, axioms);
                        let p = (child.visit_count as f64) / (total_visits as f64);
                        target_pi[class_idx.min(NUM_TACTIC_CLASSES - 1)] += p;
                    }
                }

                let trajectory_states = vec![node.state.clone()];
                let computed_returns =
                    reward_engine.compute_trajectory_returns(&trajectory_states, is_proven);
                let target_return =
                    computed_returns
                        .first()
                        .copied()
                        .unwrap_or(if is_proven { 1.0 } else { -0.5 });

                samples.push(TrainingSample {
                    x,
                    target_policy: target_pi,
                    target_value: target_return,
                });
            }
        }
    }

    samples
}

/// Runs parallel self-play MCTS across all available CPU threads using Rayon
pub fn collect_parallel_self_play_trajectories(
    model: &DeepProofNetwork,
    axioms: &AxiomLibrary,
    conjectures: Vec<Equality>,
    max_steps: usize,
) -> (Vec<TrainingSample>, usize) {
    use rayon::prelude::*;

    let results: Vec<(Vec<TrainingSample>, bool)> = conjectures
        .into_par_iter()
        .map(|conj| collect_self_play_trajectory(model, axioms, conj, max_steps))
        .collect();

    let mut all_samples = Vec::new();
    let mut total_solved = 0;

    for (samples, is_proven) in results {
        if is_proven {
            total_solved += 1;
        }
        all_samples.extend(samples);
    }

    (all_samples, total_solved)
}

/// Runs the Master Self-Improvement Training Pipeline
pub fn train_self_play_cycle(
    model: &mut DeepProofNetwork,
    optimizer: &mut AdamOptimizer,
    replay_buffer: &mut ReplayBuffer,
    axioms: &AxiomLibrary,
    epochs: usize,
    games_per_epoch: usize,
) -> Vec<TrainingMetrics> {
    let mut history = Vec::new();

    for epoch in 1..=epochs {
        let conjectures: Vec<_> = (0..games_per_epoch)
            .map(|g| generate_synthetic_algebra_conjecture(epoch + g))
            .collect();

        let (samples, solved_count) =
            collect_parallel_self_play_trajectories(model, axioms, conjectures, 40);

        for sample in samples {
            replay_buffer.push(sample);
        }

        // 2. Train Network on Replay Buffer
        let (total_loss, pol_loss, val_loss) = if !replay_buffer.samples.is_empty() {
            optimizer.train_batch(model, &replay_buffer.samples)
        } else {
            (0.0, 0.0, 0.0)
        };

        let rate = (solved_count as f64) / (games_per_epoch as f64) * 100.0;

        let metric = TrainingMetrics {
            epoch,
            total_loss,
            policy_cross_entropy: pol_loss,
            value_mean_squared_error: val_loss,
            self_play_theorems_solved: solved_count,
            self_play_total_theorems: games_per_epoch,
            solve_rate_percent: rate,
            total_samples_trained: replay_buffer.samples.len(),
        };

        history.push(metric);
    }

    history
}

/// Runs a continuous or duration-bounded training session with automatic checkpointing
pub fn train_continuous_session(
    model: &mut DeepProofNetwork,
    optimizer: &mut AdamOptimizer,
    replay_buffer: &mut ReplayBuffer,
    axioms: &AxiomLibrary,
    checkpoint_dir: &str,
    target_duration_secs: Option<u64>,
    target_epochs: Option<usize>,
    save_every_epochs: usize,
) -> (DeepProofNetwork, usize, f64) {
    use super::model::ModelCheckpoint;
    use std::time::Instant;

    let start_time = Instant::now();
    let mut epoch = 0;
    let mut best_loss = f64::INFINITY;
    let mut total_solved = 0;
    let games_per_epoch = 8;

    println!("[INFO] Starting Continuous Neural Training Session");
    println!("  - Checkpoint Dir: {}", checkpoint_dir);
    if let Some(secs) = target_duration_secs {
        println!(
            "  - Target Duration: {:.1} hours ({} seconds)",
            secs as f64 / 3600.0,
            secs
        );
    }
    if let Some(ep) = target_epochs {
        println!("  - Target Epochs:   {}", ep);
    }
    println!("================================================================================================================");
    println!(
        " {:>6} | {:>10} | {:>12} | {:>12} | {:>12} | {:>12} | {:>16}",
        "Epoch", "Elapsed", "Total Loss", "Policy CE", "Value MSE", "Solve Rate", "Best Checkpoint"
    );
    println!("----------------------------------------------------------------------------------------------------------------");

    loop {
        epoch += 1;
        let conjectures: Vec<_> = (0..games_per_epoch)
            .map(|g| generate_synthetic_algebra_conjecture(epoch * games_per_epoch + g))
            .collect();

        let (samples, solved_this_epoch) =
            collect_parallel_self_play_trajectories(model, axioms, conjectures, 50);

        total_solved += solved_this_epoch;

        for sample in samples {
            replay_buffer.push(sample);
        }

        let (total_loss, pol_loss, val_loss) = if !replay_buffer.samples.is_empty() {
            optimizer.train_batch(model, &replay_buffer.samples)
        } else {
            (0.0, 0.0, 0.0)
        };

        let solve_rate = (solved_this_epoch as f64 / games_per_epoch as f64) * 100.0;
        let elapsed_secs = start_time.elapsed().as_secs();
        let elapsed_str = format!(
            "{:02}:{:02}:{:02}",
            elapsed_secs / 3600,
            (elapsed_secs % 3600) / 60,
            elapsed_secs % 60
        );

        let mut saved_label = "-";

        // Save Best Checkpoint if loss improved
        if total_loss > 0.0 && total_loss < best_loss {
            best_loss = total_loss;
            let ckpt = ModelCheckpoint::new(model.clone(), epoch, best_loss, total_solved);
            let _ = ckpt.save_to_file(&format!("{}/checkpoint_best.json", checkpoint_dir));
            saved_label = "saved best";
        }

        // Periodic checkpoint
        if epoch % save_every_epochs == 0 {
            let ckpt = ModelCheckpoint::new(model.clone(), epoch, total_loss, total_solved);
            let _ = ckpt.save_to_file(&format!("{}/checkpoint_latest.json", checkpoint_dir));
            if saved_label == "-" {
                saved_label = "saved periodic";
            }
        }

        // Print progress every epoch or periodically
        if epoch % 5 == 0 || epoch == 1 || saved_label == "saved best" {
            println!(
                " {:>6} | {:>10} | {:>12.4} | {:>12.4} | {:>12.4} | {:>11.1}% | {:>16}",
                epoch, elapsed_str, total_loss, pol_loss, val_loss, solve_rate, saved_label
            );
        }

        // Check stopping criteria
        if let Some(max_s) = target_duration_secs {
            if elapsed_secs >= max_s {
                println!(
                    "[INFO] Target duration reached ({:.1}s). Ending session.",
                    elapsed_secs
                );
                break;
            }
        }
        if let Some(max_e) = target_epochs {
            if epoch >= max_e {
                println!("[INFO] Target epochs reached ({}). Ending session.", epoch);
                break;
            }
        }
    }

    // Save final state
    let final_ckpt = ModelCheckpoint::new(model.clone(), epoch, best_loss, total_solved);
    let _ = final_ckpt.save_to_file(&format!("{}/checkpoint_latest.json", checkpoint_dir));
    println!("================================================================================================================");
    println!(
        "[OK] Continuous Training Complete. Total Epochs: {}, Total Theorems Solved: {}",
        epoch, total_solved
    );
    println!(
        "  - Best Model saved to:   {}/checkpoint_best.json (Loss: {:.4})",
        checkpoint_dir, best_loss
    );
    println!(
        "  - Latest Model saved to: {}/checkpoint_latest.json",
        checkpoint_dir
    );
    println!("================================================================================================================\n");

    (model.clone(), epoch, best_loss)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_self_play_training_cycle() {
        let mut model = DeepProofNetwork::new_random();
        let mut optimizer = AdamOptimizer::new(&model, 0.01);
        let mut replay = ReplayBuffer::new(500);
        let axioms = AxiomLibrary::standard_algebra();

        let history = train_self_play_cycle(&mut model, &mut optimizer, &mut replay, &axioms, 2, 3);
        assert_eq!(history.len(), 2);
        assert!(history[0].total_samples_trained > 0);
    }
}
