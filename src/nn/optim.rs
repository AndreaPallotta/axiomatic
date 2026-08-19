use super::model::{DeepProofNetwork, NUM_TACTIC_CLASSES};

/// Training Sample Tuple: (input_state_features, target_mcts_policy, target_value_outcome)
#[derive(Debug, Clone)]
pub struct TrainingSample {
    pub x: Vec<f64>,
    pub target_policy: Vec<f64>, // Length: NUM_TACTIC_CLASSES
    pub target_value: f64,       // in [-1.0, 1.0]
}

/// Adam Optimizer with Momentum and RMSprop Velocity for all Network Weights
#[derive(Debug, Clone)]
pub struct AdamOptimizer {
    pub lr: f64,
    pub beta1: f64,
    pub beta2: f64,
    pub eps: f64,
    pub step_t: usize,

    // Moments for w1, b1, w2, b2, w_p, b_p, w_v, b_v
    pub m_w1: Vec<f64>,
    pub v_w1: Vec<f64>,
    pub m_b1: Vec<f64>,
    pub v_b1: Vec<f64>,

    pub m_w2: Vec<f64>,
    pub v_w2: Vec<f64>,
    pub m_b2: Vec<f64>,
    pub v_b2: Vec<f64>,

    pub m_wp: Vec<f64>,
    pub v_wp: Vec<f64>,
    pub m_bp: Vec<f64>,
    pub v_bp: Vec<f64>,

    pub m_wv: Vec<f64>,
    pub v_wv: Vec<f64>,
    pub m_bv: Vec<f64>,
    pub v_bv: Vec<f64>,
}

impl AdamOptimizer {
    pub fn new(model: &DeepProofNetwork, lr: f64) -> Self {
        Self {
            lr,
            beta1: 0.9,
            beta2: 0.999,
            eps: 1e-8,
            step_t: 0,

            m_w1: vec![0.0; model.w1.data.len()],
            v_w1: vec![0.0; model.w1.data.len()],
            m_b1: vec![0.0; model.b1.len()],
            v_b1: vec![0.0; model.b1.len()],

            m_w2: vec![0.0; model.w2.data.len()],
            v_w2: vec![0.0; model.w2.data.len()],
            m_b2: vec![0.0; model.b2.len()],
            v_b2: vec![0.0; model.b2.len()],

            m_wp: vec![0.0; model.w_policy.data.len()],
            v_wp: vec![0.0; model.w_policy.data.len()],
            m_bp: vec![0.0; model.b_policy.len()],
            v_bp: vec![0.0; model.b_policy.len()],

            m_wv: vec![0.0; model.w_value.data.len()],
            v_wv: vec![0.0; model.w_value.data.len()],
            m_bv: vec![0.0; model.b_value.len()],
            v_bv: vec![0.0; model.b_value.len()],
        }
    }

    /// Trains the neural network on a batch of samples and returns (total_loss, policy_loss, value_loss)
    pub fn train_batch(
        &mut self,
        model: &mut DeepProofNetwork,
        batch: &[TrainingSample],
    ) -> (f64, f64, f64) {
        if batch.is_empty() {
            return (0.0, 0.0, 0.0);
        }

        self.step_t += 1;
        let batch_size = batch.len() as f64;

        // Accumulated Gradients
        let mut grad_w1 = vec![0.0; model.w1.data.len()];
        let mut grad_b1 = vec![0.0; model.b1.len()];
        let mut grad_w2 = vec![0.0; model.w2.data.len()];
        let mut grad_b2 = vec![0.0; model.b2.len()];
        let mut grad_wp = vec![0.0; model.w_policy.data.len()];
        let mut grad_bp = vec![0.0; model.b_policy.len()];
        let mut grad_wv = vec![0.0; model.w_value.data.len()];
        let mut grad_bv = vec![0.0; model.b_value.len()];

        let mut total_pol_loss = 0.0;
        let mut total_val_loss = 0.0;

        for sample in batch {
            let (h1, h2, logits, value) = model.forward(&sample.x);
            let probs = DeepProofNetwork::softmax(&logits);

            // 1. Policy Cross-Entropy Loss: L_p = - \sum \pi_i * ln(p_i)
            let mut d_logits = vec![0.0; NUM_TACTIC_CLASSES];
            for i in 0..NUM_TACTIC_CLASSES {
                let target_p = if i < sample.target_policy.len() {
                    sample.target_policy[i]
                } else {
                    0.0
                };
                if target_p > 0.0 {
                    total_pol_loss -= target_p * (probs[i].max(1e-12)).ln();
                }
                d_logits[i] = probs[i] - target_p;
            }

            // 2. Value Mean Squared Error: L_v = (value - target_value)²
            let val_err = value - sample.target_value;
            total_val_loss += val_err * val_err;
            let d_zv = 2.0 * val_err * (1.0 - value * value);

            // Backprop to Policy Weights & Biases: W_p (NUM_CLASSES x 32)
            for r in 0..NUM_TACTIC_CLASSES {
                grad_bp[r] += d_logits[r];
                for c in 0..32 {
                    grad_wp[r * 32 + c] += d_logits[r] * h2[c];
                }
            }

            // Backprop to Value Weights & Biases: W_v (1 x 32)
            grad_bv[0] += d_zv;
            for c in 0..32 {
                grad_wv[c] += d_zv * h2[c];
            }

            // Gradient arriving at Hidden Layer 2: h2 (size 32)
            let mut d_h2 = vec![0.0; 32];
            for c in 0..32 {
                let mut sum = d_zv * model.w_value.data[c];
                for r in 0..NUM_TACTIC_CLASSES {
                    sum += d_logits[r] * model.w_policy.data[r * 32 + c];
                }
                d_h2[c] = sum;
            }

            let mut d_z2 = vec![0.0; 32];
            for c in 0..32 {
                let act_deriv = if h2[c] > 0.0 { 1.0 } else { 0.1 };
                d_z2[c] = d_h2[c] * act_deriv;
            }

            // Backprop to W2 (32 x 64) and b2 (32)
            for r in 0..32 {
                grad_b2[r] += d_z2[r];
                for c in 0..64 {
                    grad_w2[r * 64 + c] += d_z2[r] * h1[c];
                }
            }

            // Gradient arriving at Hidden Layer 1: h1 (size 64)
            let mut d_h1 = vec![0.0; 64];
            for c in 0..64 {
                let mut sum = 0.0;
                for r in 0..32 {
                    sum += d_z2[r] * model.w2.data[r * 64 + c];
                }
                d_h1[c] = sum;
            }

            let mut d_z1 = vec![0.0; 64];
            for c in 0..64 {
                let act_deriv = if h1[c] > 0.0 { 1.0 } else { 0.1 };
                d_z1[c] = d_h1[c] * act_deriv;
            }

            // Backprop to W1 (64 x 32) and b1 (64)
            for r in 0..64 {
                grad_b1[r] += d_z1[r];
                for c in 0..sample.x.len().min(32) {
                    grad_w1[r * 32 + c] += d_z1[r] * sample.x[c];
                }
            }
        }

        // Apply Adam Updates
        let lr = self.lr;
        let b1 = self.beta1;
        let b2 = self.beta2;
        let eps = self.eps;
        let t = self.step_t;

        apply_adam(
            lr,
            b1,
            b2,
            eps,
            t,
            &mut model.w1.data,
            &grad_w1,
            &mut self.m_w1,
            &mut self.v_w1,
            batch_size,
        );
        apply_adam(
            lr,
            b1,
            b2,
            eps,
            t,
            &mut model.b1,
            &grad_b1,
            &mut self.m_b1,
            &mut self.v_b1,
            batch_size,
        );

        apply_adam(
            lr,
            b1,
            b2,
            eps,
            t,
            &mut model.w2.data,
            &grad_w2,
            &mut self.m_w2,
            &mut self.v_w2,
            batch_size,
        );
        apply_adam(
            lr,
            b1,
            b2,
            eps,
            t,
            &mut model.b2,
            &grad_b2,
            &mut self.m_b2,
            &mut self.v_b2,
            batch_size,
        );

        apply_adam(
            lr,
            b1,
            b2,
            eps,
            t,
            &mut model.w_policy.data,
            &grad_wp,
            &mut self.m_wp,
            &mut self.v_wp,
            batch_size,
        );
        apply_adam(
            lr,
            b1,
            b2,
            eps,
            t,
            &mut model.b_policy,
            &grad_bp,
            &mut self.m_bp,
            &mut self.v_bp,
            batch_size,
        );

        apply_adam(
            lr,
            b1,
            b2,
            eps,
            t,
            &mut model.w_value.data,
            &grad_wv,
            &mut self.m_wv,
            &mut self.v_wv,
            batch_size,
        );
        apply_adam(
            lr,
            b1,
            b2,
            eps,
            t,
            &mut model.b_value,
            &grad_bv,
            &mut self.m_bv,
            &mut self.v_bv,
            batch_size,
        );

        let avg_pol_loss = total_pol_loss / batch_size;
        let avg_val_loss = total_val_loss / batch_size;
        (avg_pol_loss + avg_val_loss, avg_pol_loss, avg_val_loss)
    }

    pub fn set_learning_rate(&mut self, new_lr: f64) {
        self.lr = new_lr;
    }

    pub fn reset_moments(&mut self) {
        self.m_w1.fill(0.0);
        self.v_w1.fill(0.0);
        self.m_b1.fill(0.0);
        self.v_b1.fill(0.0);
        self.m_w2.fill(0.0);
        self.v_w2.fill(0.0);
        self.m_b2.fill(0.0);
        self.v_b2.fill(0.0);
        self.m_wp.fill(0.0);
        self.v_wp.fill(0.0);
        self.m_bp.fill(0.0);
        self.v_bp.fill(0.0);
        self.m_wv.fill(0.0);
        self.v_wv.fill(0.0);
        self.m_bv.fill(0.0);
        self.v_bv.fill(0.0);
        self.step_t = 0;
    }
}

fn apply_adam(
    lr: f64,
    beta1: f64,
    beta2: f64,
    eps: f64,
    step_t: usize,
    params: &mut [f64],
    grads: &[f64],
    m: &mut [f64],
    v: &mut [f64],
    batch_size: f64,
) {
    let b1_correction = 1.0 - beta1.powi(step_t as i32);
    let b2_correction = 1.0 - beta2.powi(step_t as i32);

    for i in 0..params.len() {
        let g = grads[i] / batch_size;
        m[i] = beta1 * m[i] + (1.0 - beta1) * g;
        v[i] = beta2 * v[i] + (1.0 - beta2) * g * g;

        let m_hat = m[i] / b1_correction.max(1e-8);
        let v_hat = v[i] / b2_correction.max(1e-8);

        params[i] -= lr * m_hat / (v_hat.sqrt() + eps);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adam_training_step_reduces_loss() {
        let mut model = DeepProofNetwork::new_random();
        let mut optim = AdamOptimizer::new(&model, 0.01);

        let mut target_policy = vec![0.0; NUM_TACTIC_CLASSES];
        target_policy[0] = 1.0; // Target: Reflexivity

        let sample = TrainingSample {
            x: vec![0.5; 32],
            target_policy,
            target_value: 1.0,
        };

        let batch = vec![sample.clone()];
        let (initial_loss, _, _) = optim.train_batch(&mut model, &batch);

        for _ in 0..10 {
            optim.train_batch(&mut model, &batch);
        }

        let (final_loss, _, _) = optim.train_batch(&mut model, &batch);
        assert!(
            final_loss < initial_loss,
            "Loss must decrease: initial={}, final={}",
            initial_loss,
            final_loss
        );
    }
}
