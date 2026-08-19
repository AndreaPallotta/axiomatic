use super::model::DeepProofNetwork;
use super::optim::AdamOptimizer;
use super::trainer::TrainingMetrics;
use serde::{Deserialize, Serialize};

/// Supervisor Health Status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrainingHealthStatus {
    Optimal,
    PlateauAdjusting,
    RollbackTriggered,
    EarlyStopped,
    Diverged,
}

/// Rollback Event record for auditability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackEvent {
    pub epoch: usize,
    pub bad_loss: f64,
    pub restored_loss: f64,
    pub new_lr: f64,
    pub reason: String,
    pub timestamp: String,
}

/// Action decided by the Training Supervisor
#[derive(Debug, Clone)]
pub enum SupervisorAction {
    Continue,
    AdjustLearningRate { old_lr: f64, new_lr: f64 },
    Rollback { bad_loss: f64, restored_loss: f64, new_lr: f64, reason: String },
    EarlyStop { reason: String },
}

/// Production Supervisor monitoring convergence, plateau, and auto-rollback
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingSupervisor {
    pub initial_lr: f64,
    pub current_lr: f64,
    pub min_lr: f64,
    pub patience: usize,
    pub decay_factor: f64,
    pub divergence_multiplier: f64,
    pub plateau_counter: usize,
    pub best_loss: f64,
    pub moving_avg_loss: f64,
    pub status: TrainingHealthStatus,
    pub rollback_history: Vec<RollbackEvent>,
}

impl TrainingSupervisor {
    pub fn new(initial_lr: f64) -> Self {
        Self {
            initial_lr,
            current_lr: initial_lr,
            min_lr: 0.0002,
            patience: 4,
            decay_factor: 0.5,
            divergence_multiplier: 2.2,
            plateau_counter: 0,
            best_loss: f64::INFINITY,
            moving_avg_loss: 0.0,
            status: TrainingHealthStatus::Optimal,
            rollback_history: Vec::new(),
        }
    }

    /// Resets supervisor status and learning rate when a new training session starts
    pub fn reset_health(&mut self, lr: Option<f64>) {
        let new_lr = lr.unwrap_or(self.initial_lr);
        self.current_lr = new_lr;
        self.plateau_counter = 0;
        self.status = TrainingHealthStatus::Optimal;
    }

    /// Evaluates latest training metrics and takes auto-adjustment or rollback actions
    pub fn evaluate_step(
        &mut self,
        model: &mut DeepProofNetwork,
        optimizer: &mut AdamOptimizer,
        best_model_snapshot: &DeepProofNetwork,
        metrics: &TrainingMetrics,
    ) -> SupervisorAction {
        let loss = metrics.total_loss;

        // Skip evaluation if loss is 0.0 (e.g. uninitialized empty batch)
        if loss <= 0.0001 {
            return SupervisorAction::Continue;
        }

        // 1. Check for NaN / Inf / Catastrophic Divergence
        let is_diverged = loss.is_nan() || loss.is_infinite() || (self.moving_avg_loss > 0.0 && loss > self.moving_avg_loss * self.divergence_multiplier && loss > 2.0);

        if is_diverged {
            let reason = if loss.is_nan() {
                "Loss exploded to NaN".to_string()
            } else if loss.is_infinite() {
                "Loss reached Infinity".to_string()
            } else {
                format!("Loss spiked from {:.4} (EMA) to {:.4}", self.moving_avg_loss, loss)
            };

            // Execute Self-Healing Rollback
            *model = best_model_snapshot.clone();
            optimizer.reset_moments();
            self.current_lr = (self.current_lr * self.decay_factor).max(self.min_lr);
            optimizer.set_learning_rate(self.current_lr);

            let event = RollbackEvent {
                epoch: metrics.epoch,
                bad_loss: loss,
                restored_loss: self.best_loss,
                new_lr: self.current_lr,
                reason: reason.clone(),
                timestamp: chrono::Utc::now().to_rfc3339(),
            };
            self.rollback_history.push(event);
            self.status = TrainingHealthStatus::RollbackTriggered;
            self.plateau_counter = 0;

            return SupervisorAction::Rollback {
                bad_loss: loss,
                restored_loss: self.best_loss,
                new_lr: self.current_lr,
                reason,
            };
        }

        // Update Moving Average Loss
        if self.moving_avg_loss == 0.0 {
            self.moving_avg_loss = loss;
        } else {
            self.moving_avg_loss = 0.8 * self.moving_avg_loss + 0.2 * loss;
        }

        // 2. Check for Improvement
        if loss < self.best_loss {
            self.best_loss = loss;
            self.plateau_counter = 0;
            self.status = TrainingHealthStatus::Optimal;
            return SupervisorAction::Continue;
        }

        // 3. Plateau Detection (Adaptive Learning Rate Scheduler)
        self.plateau_counter += 1;
        if self.plateau_counter >= self.patience {
            let old_lr = self.current_lr;
            let new_lr = (self.current_lr * self.decay_factor).max(self.min_lr);

            if new_lr < old_lr - 1e-6 {
                self.current_lr = new_lr;
                optimizer.set_learning_rate(new_lr);
                self.plateau_counter = 0;
                self.status = TrainingHealthStatus::PlateauAdjusting;

                return SupervisorAction::AdjustLearningRate { old_lr, new_lr };
            } else {
                // Minimum LR reached and still plateaued
                self.status = TrainingHealthStatus::EarlyStopped;
                return SupervisorAction::EarlyStop {
                    reason: format!("Convergence reached with minimum learning rate ({:.6})", self.min_lr),
                };
            }
        }

        SupervisorAction::Continue
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_supervisor_plateau_decay() {
        let mut supervisor = TrainingSupervisor::new(0.01);
        let mut model = DeepProofNetwork::new_random();
        let mut optim = AdamOptimizer::new(&model, 0.01);
        let best_snapshot = model.clone();

        // Initial improvement
        let m1 = TrainingMetrics {
            epoch: 1,
            total_loss: 2.0,
            policy_cross_entropy: 1.5,
            value_mean_squared_error: 0.5,
            self_play_theorems_solved: 5,
            self_play_total_theorems: 6,
            solve_rate_percent: 83.3,
            total_samples_trained: 100,
        };
        supervisor.evaluate_step(&mut model, &mut optim, &best_snapshot, &m1);
        assert_eq!(supervisor.best_loss, 2.0);

        // Simulate 4 plateau epochs
        for ep in 2..=5 {
            let m = TrainingMetrics {
                epoch: ep,
                total_loss: 2.05,
                policy_cross_entropy: 1.55,
                value_mean_squared_error: 0.5,
                self_play_theorems_solved: 5,
                self_play_total_theorems: 6,
                solve_rate_percent: 83.3,
                total_samples_trained: 100,
            };
            let action = supervisor.evaluate_step(&mut model, &mut optim, &best_snapshot, &m);
            if ep == 5 {
                match action {
                    SupervisorAction::AdjustLearningRate { new_lr, .. } => {
                        assert_eq!(new_lr, 0.005);
                    }
                    _ => panic!("Expected learning rate adjustment on plateau"),
                }
            }
        }
    }

    #[test]
    fn test_supervisor_divergence_rollback() {
        let mut supervisor = TrainingSupervisor::new(0.01);
        let mut model = DeepProofNetwork::new_random();
        let mut optim = AdamOptimizer::new(&model, 0.01);
        let best_snapshot = model.clone();

        // Initial good epoch
        let m1 = TrainingMetrics {
            epoch: 1,
            total_loss: 1.0,
            policy_cross_entropy: 0.8,
            value_mean_squared_error: 0.2,
            self_play_theorems_solved: 6,
            self_play_total_theorems: 6,
            solve_rate_percent: 100.0,
            total_samples_trained: 100,
        };
        supervisor.evaluate_step(&mut model, &mut optim, &best_snapshot, &m1);

        // Catastrophic divergence
        let m2 = TrainingMetrics {
            epoch: 2,
            total_loss: 8.5,
            policy_cross_entropy: 6.0,
            value_mean_squared_error: 2.5,
            self_play_theorems_solved: 1,
            self_play_total_theorems: 6,
            solve_rate_percent: 16.6,
            total_samples_trained: 150,
        };
        let action = supervisor.evaluate_step(&mut model, &mut optim, &best_snapshot, &m2);
        match action {
            SupervisorAction::Rollback { bad_loss, restored_loss, .. } => {
                assert_eq!(bad_loss, 8.5);
                assert_eq!(restored_loss, 1.0);
            }
            _ => panic!("Expected rollback action on divergence"),
        }
    }
}
