use crate::memory::database::LemmaDatabase;
use crate::memory::vectordb::{DistanceMetric, MathematicalVectorDB, TheoremPayload};
use crate::nn::embedding::vectorize_proof_state;
use crate::nn::model::{DeepNeuralPolicy, DeepProofNetwork, ModelCheckpoint};
use crate::nn::optim::AdamOptimizer;
use crate::nn::supervisor::{RollbackEvent, TrainingHealthStatus, TrainingSupervisor};
use crate::nn::trainer::{
    collect_parallel_self_play_trajectories, convert_mcts_tree_to_training_samples, ReplayBuffer,
    TrainingMetrics,
};
use crate::search::mcts::{MctsEngine, SearchEvent, SearchGraphSnapshot};
use crate::theory::curriculum::CurriculumController;
use crate::theory::decomposer::GoalDecomposer;
use crate::theory::inventor::{InventedTheorem, TheoryInventor};
use crate::verifier::exporter::MultiFormatExporter;
use crate::verifier::fol::Equality;
use crate::verifier::induction::InductionEngine;
use crate::verifier::kernel::{AxiomLibrary, MathDomain, ProofState};
use crate::verifier::lean_runner::Lean4Validator;
use crate::verifier::parser::parse_conjecture;
use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    extract::{Json, Path},
    response::Html,
    routing::{get, post},
    Extension, Router,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tower_http::cors::CorsLayer;

/// Target Goal Probe Result Telemetry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetProbeResult {
    pub epoch: usize,
    pub target: String,
    pub is_proven: bool,
    pub proof_steps: usize,
    pub search_nodes: usize,
    pub estimated_value: f64,
    pub timestamp: String,
}

/// Engine Controller managing live state, MCTS, Neural Network, Supervisor, and Axioms
pub struct EngineController {
    pub mcts: MctsEngine,
    pub model: DeepProofNetwork,
    pub best_model_snapshot: DeepProofNetwork,
    pub optimizer: AdamOptimizer,
    pub supervisor: TrainingSupervisor,
    pub curriculum: CurriculumController,
    pub replay_buffer: ReplayBuffer,
    pub active_domain: MathDomain,
    pub axioms: AxiomLibrary,
    pub database: LemmaDatabase,
    pub vectordb: MathematicalVectorDB,
    pub invented_theorems: Vec<InventedTheorem>,
    pub training_history: Vec<TrainingMetrics>,
    pub target_goal: Option<Equality>,
    pub target_probe_history: Vec<TargetProbeResult>,
    pub target_solved: bool,
    pub current_conjecture: Equality,
    pub event_sender: broadcast::Sender<SearchEvent>,
    pub is_training: Arc<AtomicBool>,
    pub is_continuous_discovery: Arc<AtomicBool>,
}

impl EngineController {
    pub fn new(event_sender: broadcast::Sender<SearchEvent>) -> Self {
        let active_domain = MathDomain::Unified;
        let axioms = AxiomLibrary::for_domain(active_domain);
        let (model, _, best_loss) = ModelCheckpoint::try_load_or_init("models");
        let best_model_snapshot = model.clone();
        let optimizer = AdamOptimizer::new(&model, 0.005);
        let mut supervisor = TrainingSupervisor::new(0.005);
        if best_loss.is_finite() && best_loss > 0.001 {
            supervisor.best_loss = best_loss;
        }

        let curriculum = CurriculumController::new();
        let replay_buffer = ReplayBuffer::new(5000);
        let database = LemmaDatabase::new();
        let mut vectordb = MathematicalVectorDB::new(32, DistanceMetric::Cosine);

        // Index standard axioms into Vector DB
        for (name, rule) in &axioms.rules {
            let state = ProofState::new(rule.clone());
            let vec = vectorize_proof_state(&state);
            vectordb.insert(
                vec,
                TheoremPayload {
                    name: name.clone(),
                    statement: rule.to_string(),
                    tactic_name: format!("rw_lhs [{}]", name),
                    proof_length: 1,
                    timestamp: chrono::Utc::now().to_rfc3339(),
                },
            );
        }

        let conjecture = curriculum.generate_conjecture(0);
        let mut mcts = MctsEngine::new(ProofState::new(conjecture.clone()), 8);
        mcts.set_event_sender(event_sender.clone());

        let mut ctrl = Self {
            mcts,
            model,
            best_model_snapshot,
            optimizer,
            supervisor,
            curriculum,
            replay_buffer,
            active_domain,
            axioms,
            database,
            vectordb,
            invented_theorems: Vec::new(),
            training_history: Vec::new(),
            target_goal: None,
            target_probe_history: Vec::new(),
            target_solved: false,
            current_conjecture: conjecture,
            event_sender,
            is_training: Arc::new(AtomicBool::new(false)),
            is_continuous_discovery: Arc::new(AtomicBool::new(false)),
        };

        ctrl.step_search(20);
        ctrl
    }

    pub fn set_domain(&mut self, domain: MathDomain) {
        self.active_domain = domain;
        self.axioms = AxiomLibrary::for_domain(domain);

        for (name, rule) in &self.axioms.rules {
            if !self
                .vectordb
                .records
                .iter()
                .any(|r| r.payload.name == *name)
            {
                let state = ProofState::new(rule.clone());
                let vec = vectorize_proof_state(&state);
                self.vectordb.insert(
                    vec,
                    TheoremPayload {
                        name: name.clone(),
                        statement: rule.to_string(),
                        tactic_name: format!("rw_lhs [{}]", name),
                        proof_length: 1,
                        timestamp: chrono::Utc::now().to_rfc3339(),
                    },
                );
            }
        }
    }

    pub fn set_conjecture(&mut self, conjecture: Equality) {
        self.current_conjecture = conjecture.clone();
        let mut new_mcts = MctsEngine::new(ProofState::new(conjecture), 8);
        new_mcts.set_event_sender(self.event_sender.clone());
        self.mcts = new_mcts;
        self.step_search(20);
    }

    pub fn step_search(&mut self, count: usize) -> Option<ProofState> {
        let policy = DeepNeuralPolicy::new(self.model.clone());
        let mut result = None;
        for _ in 0..count {
            if let Some(proven_id) = self.mcts.step(&policy, &self.axioms) {
                let state = self.mcts.nodes[proven_id].state.clone();
                result = Some(state.clone());
                let thm_name = format!("thm_{}", self.database.theorems.len() + 1);

                self.database.record_theorem(
                    &thm_name,
                    self.current_conjecture.clone(),
                    state.clone(),
                );

                let vec = vectorize_proof_state(&state);
                self.vectordb.insert(
                    vec,
                    TheoremPayload {
                        name: thm_name,
                        statement: self.current_conjecture.to_string(),
                        tactic_name: format!("lemma [{}]", self.current_conjecture),
                        proof_length: state.proof_history.len(),
                        timestamp: chrono::Utc::now().to_rfc3339(),
                    },
                );
            }
        }
        result
    }

    /// Probes the designated target goal with the latest neural weights
    pub fn probe_target_goal(
        &mut self,
        epoch_num: usize,
        budget: usize,
    ) -> Option<TargetProbeResult> {
        let target = self.target_goal.as_ref()?.clone();
        let policy = DeepNeuralPolicy::new(self.model.clone());

        let mut probe_mcts = MctsEngine::new(ProofState::new(target.clone()), 10);
        probe_mcts.set_event_sender(self.event_sender.clone());
        let proof = probe_mcts.run_search(&policy, &self.axioms, budget);
        let root_val = probe_mcts.nodes[0].mean_value;

        let is_proven = proof.is_some() || probe_mcts.proven_node_id.is_some();
        let proof_steps = proof.as_ref().map(|s| s.proof_history.len()).unwrap_or(0);
        let search_nodes = probe_mcts.nodes.len();

        if is_proven {
            self.target_solved = true;
            if let Some(ref solved_state) = proof {
                let thm_name = format!("target_solved_thm_{}", self.database.theorems.len() + 1);
                self.database
                    .record_theorem(&thm_name, target.clone(), solved_state.clone());
                let vec = vectorize_proof_state(solved_state);
                self.vectordb.insert(
                    vec,
                    TheoremPayload {
                        name: thm_name,
                        statement: target.to_string(),
                        tactic_name: format!("lemma [{}]", target),
                        proof_length: solved_state.proof_history.len(),
                        timestamp: chrono::Utc::now().to_rfc3339(),
                    },
                );
            }
            self.current_conjecture = target.clone();
            self.mcts = probe_mcts;
        }

        let result = TargetProbeResult {
            epoch: epoch_num,
            target: target.to_string(),
            is_proven,
            proof_steps,
            search_nodes,
            estimated_value: root_val,
            timestamp: chrono::Utc::now().to_rfc3339(),
        };

        self.target_probe_history.push(result.clone());
        Some(result)
    }

    /// Autonomously invents a new non-trivial conjecture, visualizes its live MCTS tree, and trains on it
    pub fn invent_and_prove_novel_theorem(&mut self) -> Option<InventedTheorem> {
        let policy = DeepNeuralPolicy::new(self.model.clone());
        let seed = self.invented_theorems.len() + rand::random::<usize>() % 100;

        let candidate = TheoryInventor::invent_for_domain(self.active_domain, seed);
        if !TheoryInventor::is_non_trivial(&candidate) {
            return None;
        }

        self.current_conjecture = candidate.clone();
        let mut live_mcts = MctsEngine::new(ProofState::new(candidate.clone()), 10);
        live_mcts.set_event_sender(self.event_sender.clone());
        let complexity = TheoryInventor::compute_complexity(&candidate);
        let budget = (90 + (complexity * 4.0) as usize).min(180);
        let proof = live_mcts.run_search(&policy, &self.axioms, budget);

        if let Some(solved_state) = proof {
            let thm_name = format!("invented_thm_{}", self.invented_theorems.len() + 1);
            let complexity = TheoryInventor::compute_complexity(&candidate);

            // Record in Lemma Database
            self.database
                .record_theorem(&thm_name, candidate.clone(), solved_state.clone());

            // Index in Vector DB
            let vec = vectorize_proof_state(&solved_state);
            self.vectordb.insert(
                vec,
                TheoremPayload {
                    name: thm_name.clone(),
                    statement: candidate.to_string(),
                    tactic_name: format!("lemma [{}]", candidate),
                    proof_length: solved_state.proof_history.len(),
                    timestamp: chrono::Utc::now().to_rfc3339(),
                },
            );

            let record = InventedTheorem {
                name: thm_name,
                conjecture: candidate.to_string(),
                domain: self.active_domain.name().to_string(),
                proof_steps: solved_state.proof_history.len(),
                complexity_score: complexity,
                timestamp: chrono::Utc::now().to_rfc3339(),
            };

            self.invented_theorems.push(record.clone());

            // Immediately reinforce neural network on this discovery trajectory!
            let samples = convert_mcts_tree_to_training_samples(&live_mcts, &self.axioms, true);
            let epoch_num = self.training_history.len() + 1;
            self.train_batch_step(samples, 1, epoch_num, 1);

            self.mcts = live_mcts;
            Some(record)
        } else {
            // Record challenging attempt so solve rate reflects actual search difficulty
            let samples = convert_mcts_tree_to_training_samples(&live_mcts, &self.axioms, false);
            let epoch_num = self.training_history.len() + 1;
            self.train_batch_step(samples, 0, epoch_num, 1);

            self.mcts = live_mcts;
            None
        }
    }

    pub fn train_batch_step(
        &mut self,
        samples: Vec<crate::nn::optim::TrainingSample>,
        solved_count: usize,
        epoch_num: usize,
        episodes: usize,
    ) -> TrainingMetrics {
        for is_proven in (0..episodes).map(|i| i < solved_count) {
            self.curriculum.record_attempt(is_proven);
        }

        for sample in samples {
            self.replay_buffer.push(sample);
        }

        let (total_loss, pol_loss, val_loss) = if !self.replay_buffer.samples.is_empty() {
            let samples = &self.replay_buffer.samples;
            self.optimizer.train_batch(&mut self.model, samples)
        } else {
            (0.0, 0.0, 0.0)
        };

        let rate = (solved_count as f64) / (episodes as f64) * 100.0;
        let metric = TrainingMetrics {
            epoch: epoch_num,
            total_loss,
            policy_cross_entropy: pol_loss,
            value_mean_squared_error: val_loss,
            self_play_theorems_solved: solved_count,
            self_play_total_theorems: episodes,
            solve_rate_percent: rate,
            total_samples_trained: self.replay_buffer.samples.len(),
        };

        self.training_history.push(metric.clone());

        let _ = self.supervisor.evaluate_step(
            &mut self.model,
            &mut self.optimizer,
            &self.best_model_snapshot,
            &metric,
        );

        if total_loss > 0.0001 && total_loss < self.supervisor.best_loss {
            self.best_model_snapshot = self.model.clone();
            let ckpt = ModelCheckpoint::new(
                self.model.clone(),
                epoch_num,
                total_loss,
                self.database.theorems.len(),
            );
            let _ = ckpt.save_to_file("models/checkpoint_best.json");
        }

        if epoch_num % 5 == 0 && total_loss > 0.0001 {
            let ckpt = ModelCheckpoint::new(
                self.model.clone(),
                epoch_num,
                total_loss,
                self.database.theorems.len(),
            );
            let _ = ckpt.save_to_file("models/checkpoint_latest.json");
        }

        metric
    }

    pub fn rollback_to_checkpoint(&mut self, filename: &str) -> Result<ModelCheckpoint, String> {
        let path = format!("models/{}", filename);
        let ckpt = ModelCheckpoint::load_from_file(&path)
            .map_err(|e| format!("Failed to load checkpoint {}: {}", path, e))?;

        self.model = ckpt.model.clone();
        self.best_model_snapshot = ckpt.model.clone();
        self.optimizer.reset_moments();
        if ckpt.best_loss > 0.0001 {
            self.supervisor.best_loss = ckpt.best_loss;
            self.supervisor.moving_avg_loss = ckpt.best_loss;
        }
        self.supervisor.status = TrainingHealthStatus::Optimal;

        self.supervisor.rollback_history.push(RollbackEvent {
            epoch: ckpt.total_epochs_trained,
            bad_loss: 0.0,
            restored_loss: ckpt.best_loss,
            new_lr: self.supervisor.current_lr,
            reason: format!("Manual user rollback to {}", filename),
            timestamp: chrono::Utc::now().to_rfc3339(),
        });

        Ok(ckpt)
    }
}

pub type SharedState = Arc<RwLock<EngineController>>;

pub async fn start_visualizer_server(port: u16, controller: EngineController) {
    let shared_state: SharedState = Arc::new(RwLock::new(controller));

    let app = Router::new()
        .route("/", get(serve_dashboard_html))
        .route("/ws", get(websocket_handler))
        .route("/api/status", get(get_status_handler))
        .route("/api/vectordb", get(get_vectordb_handler))
        .route("/api/vectordb/search", post(post_vectordb_search_handler))
        .route("/api/domain/select", post(post_domain_select_handler))
        .route("/api/step", post(post_step_handler))
        .route("/api/search", post(post_search_handler))
        .route("/api/induction", post(post_induction_handler))
        .route("/api/train/start", post(post_train_start_handler))
        .route("/api/train/stop", post(post_train_stop_handler))
        .route("/api/train/reset-lr", post(post_train_reset_lr_handler))
        .route("/api/train/telemetry", get(get_train_telemetry_handler))
        .route(
            "/api/train/target/start",
            post(post_train_target_start_handler),
        )
        .route(
            "/api/train/target/telemetry",
            get(get_train_target_telemetry_handler),
        )
        .route(
            "/api/discovery/continuous/start",
            post(post_continuous_discovery_start_handler),
        )
        .route(
            "/api/discovery/continuous/stop",
            post(post_continuous_discovery_stop_handler),
        )
        .route("/api/checkpoints", get(get_checkpoints_handler))
        .route("/api/checkpoints/rollback", post(post_rollback_handler))
        .route("/api/supervisor", get(get_supervisor_handler))
        .route("/api/invent", post(post_invent_handler))
        .route("/api/invented", get(get_invented_handler))
        .route("/api/conjecture", post(post_conjecture_handler))
        .route(
            "/api/conjecture/custom",
            post(post_custom_conjecture_handler),
        )
        .route("/api/reset", post(post_reset_handler))
        .route("/api/export/:format", get(get_export_handler))
        .route("/api/lean/validate", get(get_lean_validate_handler))
        .layer(CorsLayer::permissive())
        .layer(Extension(shared_state));

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    println!("\n[INFO] Axiomatic Unified Dashboard Active:");
    println!("       http://localhost:{}\n", port);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

#[derive(Serialize)]
struct StatusResponse {
    nodes_count: usize,
    iterations: usize,
    is_proven: bool,
    conjecture: String,
    training_epochs: usize,
    latest_loss: Option<f64>,
    theorems_in_db: usize,
    invented_theorems_count: usize,
    vector_premises_count: usize,
    is_training_active: bool,
    is_continuous_discovery_active: bool,
    active_domain: String,
    target_solved: bool,
    supervisor_status: String,
    current_learning_rate: f64,
    curriculum_level: String,
    solve_rate: f64,
    graph: SearchGraphSnapshot,
    loss_history: Vec<f64>,
}

async fn get_status_handler(Extension(state): Extension<SharedState>) -> Json<StatusResponse> {
    let ctrl = state.read().await;
    let snap = ctrl.mcts.snapshot();
    let is_proven = ctrl.mcts.proven_node_id.is_some();
    let latest_loss = ctrl.training_history.last().map(|m| m.total_loss);
    let loss_history = ctrl.training_history.iter().map(|m| m.total_loss).collect();
    let is_training_active = ctrl.is_training.load(Ordering::Relaxed);
    let is_continuous_discovery_active = ctrl.is_continuous_discovery.load(Ordering::Relaxed);

    let solve_rate = if !ctrl.training_history.is_empty() {
        let recent = ctrl.training_history.iter().rev().take(20);
        let total_theorems: usize = recent.clone().map(|m| m.self_play_total_theorems).sum();
        let total_solved: usize = recent.map(|m| m.self_play_theorems_solved).sum();
        if total_theorems > 0 {
            (total_solved as f64) / (total_theorems as f64) * 100.0
        } else {
            100.0
        }
    } else {
        100.0
    };

    Json(StatusResponse {
        nodes_count: snap.nodes.len(),
        iterations: ctrl.mcts.iterations,
        is_proven,
        conjecture: ctrl.current_conjecture.to_string(),
        training_epochs: ctrl.training_history.len(),
        latest_loss,
        theorems_in_db: ctrl.database.theorems.len(),
        invented_theorems_count: ctrl.invented_theorems.len(),
        vector_premises_count: ctrl.vectordb.records.len(),
        is_training_active,
        is_continuous_discovery_active,
        active_domain: ctrl.active_domain.name().to_string(),
        target_solved: ctrl.target_solved,
        supervisor_status: format!("{:?}", ctrl.supervisor.status),
        current_learning_rate: ctrl.supervisor.current_lr,
        curriculum_level: ctrl.curriculum.current_level.name().to_string(),
        solve_rate,
        graph: snap,
        loss_history,
    })
}

#[derive(Deserialize)]
struct DomainSelectRequest {
    domain: String,
}

async fn post_domain_select_handler(
    Extension(state): Extension<SharedState>,
    Json(req): Json<DomainSelectRequest>,
) -> Json<serde_json::Value> {
    let mut ctrl = state.write().await;
    let domain = match req.domain.to_lowercase().as_str() {
        "boolean" => MathDomain::BooleanLogic,
        "calculus" => MathDomain::SymbolicCalculus,
        "set_theory" => MathDomain::SetTheory,
        "unified" => MathDomain::Unified,
        _ => MathDomain::AbstractAlgebra,
    };
    ctrl.set_domain(domain);
    Json(serde_json::json!({
        "status": "ok",
        "domain": domain.name(),
        "total_axioms": ctrl.axioms.rules.len(),
    }))
}

async fn get_vectordb_handler(Extension(state): Extension<SharedState>) -> Json<serde_json::Value> {
    let ctrl = state.read().await;
    let premises: Vec<_> = ctrl.vectordb.records.iter().map(|r| &r.payload).collect();
    Json(serde_json::json!({
        "total_premises": ctrl.vectordb.records.len(),
        "dimension": ctrl.vectordb.dimension,
        "metric": "Cosine Similarity",
        "records": premises,
    }))
}

#[derive(Deserialize)]
struct VectorSearchRequest {
    query: String,
    top_k: Option<usize>,
}

async fn post_vectordb_search_handler(
    Extension(state): Extension<SharedState>,
    Json(req): Json<VectorSearchRequest>,
) -> Json<serde_json::Value> {
    let ctrl = state.read().await;
    let top_k = req.top_k.unwrap_or(5);

    match parse_conjecture(&req.query) {
        Ok(eq) => {
            let temp_state = ProofState::new(eq);
            let q_vec = vectorize_proof_state(&temp_state);
            let results = ctrl.vectordb.query(&q_vec, top_k);
            Json(serde_json::json!({
                "status": "ok",
                "query": req.query,
                "results": results.iter().map(|r| serde_json::json!({
                    "score": r.score,
                    "payload": r.record.payload,
                })).collect::<Vec<_>>(),
            }))
        }
        Err(e) => Json(serde_json::json!({
            "status": "error",
            "message": format!("Could not parse query: {}", e),
        })),
    }
}

#[derive(Deserialize)]
struct StepRequest {
    steps: Option<usize>,
}

async fn post_step_handler(
    Extension(state): Extension<SharedState>,
    Json(req): Json<StepRequest>,
) -> Json<serde_json::Value> {
    let mut ctrl = state.write().await;
    let steps = req.steps.unwrap_or(15);
    let solved = ctrl.step_search(steps);
    let snap = ctrl.mcts.snapshot();

    Json(serde_json::json!({
        "status": "stepped",
        "nodes_count": snap.nodes.len(),
        "is_proven": solved.is_some(),
        "graph": snap,
    }))
}

async fn post_search_handler(Extension(state): Extension<SharedState>) -> Json<serde_json::Value> {
    let mut ctrl = state.write().await;
    let solved = ctrl.step_search(40);
    let snap = ctrl.mcts.snapshot();

    Json(serde_json::json!({
        "status": "search_completed",
        "nodes_count": snap.nodes.len(),
        "is_proven": solved.is_some(),
        "graph": snap,
    }))
}

#[derive(Deserialize)]
struct InductionRequest {
    variable: Option<String>,
}

async fn post_induction_handler(
    Extension(state): Extension<SharedState>,
    Json(req): Json<InductionRequest>,
) -> Json<serde_json::Value> {
    let mut ctrl = state.write().await;
    let var_name = req.variable.unwrap_or_else(|| "n".to_string());
    let current_root = &ctrl.mcts.nodes[0].state;

    match InductionEngine::apply_induction(current_root, &var_name) {
        Ok(next_state) => {
            let sender = ctrl.event_sender.clone();
            ctrl.mcts = MctsEngine::new(next_state, 8);
            ctrl.mcts.set_event_sender(sender);
            ctrl.step_search(20);
            let snap = ctrl.mcts.snapshot();
            Json(serde_json::json!({
                "status": "ok",
                "message": format!("Applied Peano Induction on variable '{}'", var_name),
                "graph": snap,
            }))
        }
        Err(err) => Json(serde_json::json!({
            "status": "error",
            "message": err,
        })),
    }
}

#[derive(Deserialize)]
struct StartTrainRequest {
    epochs: Option<usize>,
    episodes_per_epoch: Option<usize>,
}

async fn post_train_start_handler(
    Extension(state): Extension<SharedState>,
    Json(req): Json<StartTrainRequest>,
) -> Json<serde_json::Value> {
    let is_already_training = {
        let mut ctrl = state.write().await;
        ctrl.supervisor.reset_health(Some(0.005));
        ctrl.optimizer.set_learning_rate(0.005);
        ctrl.is_training.swap(true, Ordering::SeqCst)
    };

    if is_already_training {
        return Json(serde_json::json!({ "status": "already_running" }));
    }

    let state_clone = state.clone();
    let total_epochs = req.epochs.unwrap_or(20);
    let episodes = req.episodes_per_epoch.unwrap_or(8);

    tokio::spawn(async move {
        let is_training_flag = { state_clone.read().await.is_training.clone() };

        for _e in 1..=total_epochs {
            if !is_training_flag.load(Ordering::Relaxed) {
                break;
            }

            let (model_snapshot, axioms_snapshot, conjectures, epoch_num) = {
                let ctrl = state_clone.read().await;
                let epoch_num = ctrl.training_history.len() + 1;
                let conjectures: Vec<_> = (0..episodes)
                    .map(|g| {
                        ctrl.curriculum
                            .generate_conjecture(epoch_num * episodes + g)
                    })
                    .collect();
                (
                    ctrl.model.clone(),
                    ctrl.axioms.clone(),
                    conjectures,
                    epoch_num,
                )
            };

            let (samples, solved_count) = tokio::task::spawn_blocking(move || {
                collect_parallel_self_play_trajectories(
                    &model_snapshot,
                    &axioms_snapshot,
                    conjectures,
                    45,
                )
            })
            .await
            .unwrap_or((Vec::new(), 0));

            {
                let mut ctrl = state_clone.write().await;
                ctrl.train_batch_step(samples, solved_count, epoch_num, episodes);
            }

            tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;
        }

        is_training_flag.store(false, Ordering::Relaxed);
    });

    Json(serde_json::json!({
        "status": "training_started",
        "target_epochs": total_epochs,
    }))
}

#[derive(Deserialize)]
struct StartTargetTrainRequest {
    target_equation: String,
    epochs: Option<usize>,
    episodes_per_epoch: Option<usize>,
    probe_budget: Option<usize>,
}

async fn post_train_target_start_handler(
    Extension(state): Extension<SharedState>,
    Json(req): Json<StartTargetTrainRequest>,
) -> Json<serde_json::Value> {
    let parsed_target = match parse_conjecture(&req.target_equation) {
        Ok(eq) => eq,
        Err(e) => {
            return Json(
                serde_json::json!({ "status": "error", "message": format!("Syntax Error: {}", e) }),
            )
        }
    };

    let is_already_training = {
        let mut ctrl = state.write().await;
        ctrl.target_goal = Some(parsed_target.clone());
        ctrl.target_solved = false;
        ctrl.supervisor.reset_health(Some(0.005));
        ctrl.optimizer.set_learning_rate(0.005);
        ctrl.is_training.swap(true, Ordering::SeqCst)
    };

    if is_already_training {
        return Json(serde_json::json!({ "status": "already_running" }));
    }

    let state_clone = state.clone();
    let total_epochs = req.epochs.unwrap_or(40);
    let episodes = req.episodes_per_epoch.unwrap_or(12);
    let probe_budget = req.probe_budget.unwrap_or(100);

    let target_for_spawn = parsed_target.clone();
    let target_str = parsed_target.to_string();

    tokio::spawn(async move {
        let is_training_flag = { state_clone.read().await.is_training.clone() };

        for _e in 1..=total_epochs {
            if !is_training_flag.load(Ordering::Relaxed) {
                break;
            }

            let (model_snapshot, axioms_snapshot, conjectures, epoch_num) = {
                let ctrl = state_clone.read().await;
                let epoch_num = ctrl.training_history.len() + 1;
                let subproblems = GoalDecomposer::generate_subproblems(&target_for_spawn, episodes);
                (
                    ctrl.model.clone(),
                    ctrl.axioms.clone(),
                    subproblems,
                    epoch_num,
                )
            };

            let (samples, solved_count) = tokio::task::spawn_blocking(move || {
                collect_parallel_self_play_trajectories(
                    &model_snapshot,
                    &axioms_snapshot,
                    conjectures,
                    50,
                )
            })
            .await
            .unwrap_or((Vec::new(), 0));

            let target_cracked = {
                let mut ctrl = state_clone.write().await;
                ctrl.train_batch_step(samples, solved_count, epoch_num, episodes);
                let probe = ctrl.probe_target_goal(epoch_num, probe_budget);
                probe.map(|p| p.is_proven).unwrap_or(false)
            };

            if target_cracked {
                break;
            }

            tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;
        }

        is_training_flag.store(false, Ordering::Relaxed);
    });

    Json(serde_json::json!({
        "status": "target_training_started",
        "target": target_str,
        "epochs": total_epochs,
    }))
}

async fn get_train_target_telemetry_handler(
    Extension(state): Extension<SharedState>,
) -> Json<serde_json::Value> {
    let ctrl = state.read().await;
    let is_active = ctrl.is_training.load(Ordering::Relaxed);
    let target_str = ctrl.target_goal.as_ref().map(|g| g.to_string());
    let latest_probe = ctrl.target_probe_history.last();

    Json(serde_json::json!({
        "is_active": is_active,
        "target_goal": target_str,
        "target_solved": ctrl.target_solved,
        "latest_probe": latest_probe,
        "probe_history": ctrl.target_probe_history.iter().rev().take(15).collect::<Vec<_>>(),
    }))
}

#[derive(Deserialize)]
struct ContinuousDiscoveryRequest {
    domain: Option<String>,
    interval_ms: Option<u64>,
}

async fn post_continuous_discovery_start_handler(
    Extension(state): Extension<SharedState>,
    Json(req): Json<ContinuousDiscoveryRequest>,
) -> Json<serde_json::Value> {
    let is_already_active = {
        let mut ctrl = state.write().await;
        if let Some(ref d_str) = req.domain {
            let domain = match d_str.to_lowercase().as_str() {
                "boolean" => MathDomain::BooleanLogic,
                "calculus" => MathDomain::SymbolicCalculus,
                "set_theory" => MathDomain::SetTheory,
                "unified" => MathDomain::Unified,
                _ => MathDomain::AbstractAlgebra,
            };
            ctrl.set_domain(domain);
        }
        ctrl.is_continuous_discovery.swap(true, Ordering::SeqCst)
    };

    if is_already_active {
        return Json(serde_json::json!({ "status": "already_active" }));
    }

    let state_clone = state.clone();
    let interval = req.interval_ms.unwrap_or(1000);

    tokio::spawn(async move {
        let flag = { state_clone.read().await.is_continuous_discovery.clone() };

        while flag.load(Ordering::Relaxed) {
            {
                let mut ctrl = state_clone.write().await;
                let _ = ctrl.invent_and_prove_novel_theorem();
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(interval)).await;
        }
    });

    Json(serde_json::json!({
        "status": "continuous_discovery_started",
    }))
}

async fn post_continuous_discovery_stop_handler(
    Extension(state): Extension<SharedState>,
) -> Json<serde_json::Value> {
    let ctrl = state.read().await;
    ctrl.is_continuous_discovery.store(false, Ordering::SeqCst);
    Json(serde_json::json!({ "status": "continuous_discovery_stopped" }))
}

async fn post_train_stop_handler(
    Extension(state): Extension<SharedState>,
) -> Json<serde_json::Value> {
    let ctrl = state.read().await;
    ctrl.is_training.store(false, Ordering::SeqCst);
    ctrl.is_continuous_discovery.store(false, Ordering::SeqCst);
    Json(serde_json::json!({ "status": "training_stopped" }))
}

async fn post_train_reset_lr_handler(
    Extension(state): Extension<SharedState>,
) -> Json<serde_json::Value> {
    let mut ctrl = state.write().await;
    ctrl.supervisor.reset_health(Some(0.005));
    ctrl.optimizer.set_learning_rate(0.005);
    Json(serde_json::json!({
        "status": "ok",
        "new_lr": 0.005,
    }))
}

async fn get_train_telemetry_handler(
    Extension(state): Extension<SharedState>,
) -> Json<serde_json::Value> {
    let ctrl = state.read().await;
    let is_active = ctrl.is_training.load(Ordering::Relaxed);
    let metrics = &ctrl.training_history;

    let total_loss_series: Vec<f64> = metrics.iter().map(|m| m.total_loss).collect();
    let policy_ce_series: Vec<f64> = metrics.iter().map(|m| m.policy_cross_entropy).collect();
    let value_mse_series: Vec<f64> = metrics.iter().map(|m| m.value_mean_squared_error).collect();
    let solve_rate_series: Vec<f64> = metrics.iter().map(|m| m.solve_rate_percent).collect();

    Json(serde_json::json!({
        "is_active": is_active,
        "total_epochs": metrics.len(),
        "curriculum_level": ctrl.curriculum.current_level.name(),
        "total_loss_series": total_loss_series,
        "policy_ce_series": policy_ce_series,
        "value_mse_series": value_mse_series,
        "solve_rate_series": solve_rate_series,
        "replay_buffer_size": ctrl.replay_buffer.samples.len(),
        "supervisor": {
            "status": format!("{:?}", ctrl.supervisor.status),
            "current_lr": ctrl.supervisor.current_lr,
            "plateau_counter": ctrl.supervisor.plateau_counter,
            "patience": ctrl.supervisor.patience,
            "best_loss": if ctrl.supervisor.best_loss.is_finite() { ctrl.supervisor.best_loss } else { 0.0 },
            "rollback_events": ctrl.supervisor.rollback_history,
        },
        "recent_metrics": metrics.iter().rev().take(15).collect::<Vec<_>>(),
    }))
}

#[derive(Deserialize)]
struct RollbackRequest {
    filename: String,
}

async fn post_rollback_handler(
    Extension(state): Extension<SharedState>,
    Json(req): Json<RollbackRequest>,
) -> Json<serde_json::Value> {
    let mut ctrl = state.write().await;
    match ctrl.rollback_to_checkpoint(&req.filename) {
        Ok(ckpt) => Json(serde_json::json!({
            "status": "rollback_successful",
            "filename": req.filename,
            "restored_epoch": ckpt.total_epochs_trained,
            "restored_loss": ckpt.best_loss,
        })),
        Err(err) => Json(serde_json::json!({
            "status": "error",
            "message": err,
        })),
    }
}

async fn get_supervisor_handler(
    Extension(state): Extension<SharedState>,
) -> Json<serde_json::Value> {
    let ctrl = state.read().await;
    Json(serde_json::json!({
        "supervisor": ctrl.supervisor,
    }))
}

async fn post_invent_handler(Extension(state): Extension<SharedState>) -> Json<serde_json::Value> {
    let mut ctrl = state.write().await;
    let mut discovered = None;
    for _ in 0..10 {
        if let Some(thm) = ctrl.invent_and_prove_novel_theorem() {
            discovered = Some(thm);
            break;
        }
    }

    if let Some(thm) = discovered {
        Json(serde_json::json!({
            "status": "discovered",
            "theorem": thm,
        }))
    } else {
        Json(serde_json::json!({
            "status": "attempted",
            "message": "Generated candidate conjectures were complex. Click again to test next theory candidate.",
        }))
    }
}

async fn get_invented_handler(Extension(state): Extension<SharedState>) -> Json<serde_json::Value> {
    let ctrl = state.read().await;
    let is_continuous_active = ctrl.is_continuous_discovery.load(Ordering::Relaxed);
    Json(serde_json::json!({
        "is_continuous_active": is_continuous_active,
        "active_domain": ctrl.active_domain.name(),
        "total_invented": ctrl.invented_theorems.len(),
        "theorems": ctrl.invented_theorems.iter().rev().take(30).collect::<Vec<_>>(),
    }))
}

async fn get_checkpoints_handler() -> Json<serde_json::Value> {
    let mut ckpts = Vec::new();
    if let Ok(entries) = std::fs::read_dir("models") {
        for entry in entries.flatten() {
            if let Ok(content) = std::fs::read_to_string(entry.path()) {
                if let Ok(parsed) = serde_json::from_str::<ModelCheckpoint>(&content) {
                    ckpts.push(serde_json::json!({
                        "filename": entry.file_name().to_string_lossy().to_string(),
                        "epochs": parsed.total_epochs_trained,
                        "loss": parsed.best_loss,
                        "theorems_solved": parsed.total_theorems_solved,
                        "timestamp": parsed.timestamp,
                    }));
                }
            }
        }
    }
    Json(serde_json::json!({ "checkpoints": ckpts }))
}

#[derive(Deserialize)]
struct ConjectureRequest {
    index: Option<usize>,
}

async fn post_conjecture_handler(
    Extension(state): Extension<SharedState>,
    Json(req): Json<ConjectureRequest>,
) -> Json<serde_json::Value> {
    let mut ctrl = state.write().await;
    let idx = req.index.unwrap_or_else(|| rand::random::<usize>() % 10);
    let conj = ctrl.curriculum.generate_conjecture(idx);
    ctrl.set_conjecture(conj.clone());
    let snap = ctrl.mcts.snapshot();

    Json(serde_json::json!({
        "status": "conjecture_updated",
        "conjecture": conj.to_string(),
        "graph": snap,
    }))
}

#[derive(Deserialize)]
struct CustomConjectureRequest {
    equation: String,
}

async fn post_custom_conjecture_handler(
    Extension(state): Extension<SharedState>,
    Json(req): Json<CustomConjectureRequest>,
) -> Json<serde_json::Value> {
    let mut ctrl = state.write().await;
    match parse_conjecture(&req.equation) {
        Ok(eq) => {
            ctrl.set_conjecture(eq.clone());
            let snap = ctrl.mcts.snapshot();
            Json(serde_json::json!({
                "status": "ok",
                "conjecture": eq.to_string(),
                "graph": snap,
            }))
        }
        Err(err) => Json(serde_json::json!({
            "status": "error",
            "message": format!("Syntax Error: {}", err),
        })),
    }
}

async fn post_reset_handler(Extension(state): Extension<SharedState>) -> Json<serde_json::Value> {
    let mut ctrl = state.write().await;
    let conj = ctrl.current_conjecture.clone();
    ctrl.set_conjecture(conj);
    let snap = ctrl.mcts.snapshot();

    Json(serde_json::json!({
        "status": "reset",
        "graph": snap,
    }))
}

async fn get_export_handler(
    Extension(state): Extension<SharedState>,
    Path(format): Path<String>,
) -> Html<String> {
    let ctrl = state.read().await;
    if let Some(proven_id) = ctrl.mcts.proven_node_id {
        let state_ref = &ctrl.mcts.nodes[proven_id].state;
        let code = match format.as_str() {
            "coq" => MultiFormatExporter::to_coq("discovered_theorem", state_ref),
            "latex" => MultiFormatExporter::to_latex("discovered_theorem", state_ref),
            _ => MultiFormatExporter::to_lean4("discovered_theorem", state_ref),
        };
        Html(format!("<pre style='color:#1a73e8; background:#ffffff; padding:24px; border-radius:8px; border:1px solid #dadce0; font-family:monospace; font-size:14px; line-height:1.6; white-space:pre-wrap;'>{}</pre>", code))
    } else {
        Html("<pre style='color:#d93025; background:#ffffff; padding:24px; border-radius:8px; border:1px solid #dadce0; font-family:monospace; font-size:14px;'>No theorem proven yet in active search tree. Run MCTS search first.</pre>".to_string())
    }
}

async fn get_lean_validate_handler(
    Extension(state): Extension<SharedState>,
) -> Json<serde_json::Value> {
    let ctrl = state.read().await;
    if let Some(proven_id) = ctrl.mcts.proven_node_id {
        let state_ref = &ctrl.mcts.nodes[proven_id].state;
        let res = Lean4Validator::validate_proof("discovered_theorem", state_ref);
        Json(serde_json::json!(res))
    } else {
        Json(serde_json::json!({
            "status": "error",
            "message": "No proven theorem in search tree to validate."
        }))
    }
}

async fn websocket_handler(
    ws: WebSocketUpgrade,
    Extension(state): Extension<SharedState>,
) -> axum::response::Response {
    ws.on_upgrade(|socket| handle_websocket(socket, state))
}

async fn handle_websocket(mut socket: WebSocket, state: SharedState) {
    let (rx_sender, snap) = {
        let ctrl = state.read().await;
        (ctrl.event_sender.clone(), ctrl.mcts.snapshot())
    };
    let mut rx = rx_sender.subscribe();

    if let Ok(initial_json) = serde_json::to_string(&snap) {
        let _ = socket.send(Message::Text(initial_json)).await;
    }

    while let Ok(event) = rx.recv().await {
        if let Ok(msg_text) = serde_json::to_string(&event) {
            if socket.send(Message::Text(msg_text)).await.is_err() {
                break;
            }
        }
    }
}

async fn serve_dashboard_html() -> Html<&'static str> {
    Html(DASHBOARD_HTML)
}

pub const DASHBOARD_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Axiomatic - Autonomous AI Mathematician</title>
    <link rel="preconnect" href="https://fonts.googleapis.com">
    <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
    <link href="https://fonts.googleapis.com/css2?family=Google+Sans:wght@400;500;700&family=Roboto+Mono:wght@400;500&family=Roboto:wght@400;500;700&display=swap" rel="stylesheet">
    <style>
        :root {
            --md-bg: #f8f9fa;
            --md-surface: #ffffff;
            --md-surface-variant: #f1f3f4;
            --md-border: #dadce0;
            --md-primary: #1a73e8;
            --md-primary-hover: #1557b0;
            --md-secondary: #5f6368;
            --md-success: #137333;
            --md-success-bg: #e6f4ea;
            --md-purple: #7627bb;
            --md-purple-bg: #f3e8fd;
            --md-warning: #e37400;
            --md-warning-bg: #fef7e0;
            --md-danger: #c5221f;
            --md-danger-bg: #fce8e6;
            --md-text-primary: #202124;
            --md-text-secondary: #5f6368;
            --md-shadow: 0 1px 2px 0 rgba(60,64,67,0.3), 0 1px 3px 1px rgba(60,64,67,0.15);
        }
        * { box-sizing: border-box; margin: 0; padding: 0; }
        body {
            background: var(--md-bg);
            color: var(--md-text-primary);
            font-family: 'Roboto', 'Google Sans', -apple-system, sans-serif;
            height: 100vh;
            overflow: hidden;
            display: flex;
            flex-direction: column;
        }

        /* Unified Header */
        header {
            background: var(--md-surface);
            border-bottom: 1px solid var(--md-border);
            padding: 8px 20px;
            display: flex;
            justify-content: space-between;
            align-items: center;
            box-shadow: 0 1px 2px 0 rgba(60,64,67,0.08);
            z-index: 10;
            gap: 16px;
        }
        .brand-section { display: flex; align-items: center; gap: 10px; }
        .brand-icon {
            background: var(--md-primary);
            color: #fff;
            width: 32px; height: 32px;
            border-radius: 8px;
            display: flex; align-items: center; justify-content: center;
            font-size: 1.1rem; font-weight: 700;
            font-family: 'Google Sans', sans-serif;
        }
        .brand-text h1 { font-family: 'Google Sans', sans-serif; font-size: 1.05rem; font-weight: 700; color: var(--md-text-primary); line-height: 1.2; }
        .brand-text p { font-size: 0.7rem; color: var(--md-text-secondary); }

        .master-controls { display: flex; align-items: center; gap: 8px; flex: 1; justify-content: center; }

        .header-stats { display: flex; align-items: center; gap: 8px; }
        .chip {
            display: inline-flex;
            align-items: center;
            gap: 6px;
            padding: 5px 10px;
            border-radius: 16px;
            font-size: 0.72rem;
            font-weight: 500;
        }
        .chip-success { background: var(--md-success-bg); color: var(--md-success); }
        .chip-primary { background: #e8f0fe; color: var(--md-primary); }
        .chip-pulse { width: 6px; height: 6px; border-radius: 50%; background: var(--md-success); animation: pulse 1.5s infinite; }
        @keyframes pulse { 0%, 100% { opacity: 1; transform: scale(1); } 50% { opacity: 0.4; transform: scale(0.85); } }

        /* Buttons */
        .btn {
            font-family: 'Google Sans', sans-serif;
            font-size: 0.78rem;
            font-weight: 500;
            padding: 6px 14px;
            border-radius: 16px;
            border: 1px solid transparent;
            cursor: pointer;
            transition: all 0.2s ease;
            display: inline-flex;
            align-items: center;
            gap: 6px;
            outline: none;
            white-space: nowrap;
        }
        .btn-sm { font-size: 0.72rem; padding: 4px 8px; border-radius: 12px; }
        .btn-filled { background: var(--md-primary); color: #fff; box-shadow: 0 1px 2px rgba(0,0,0,0.12); }
        .btn-filled:hover { background: var(--md-primary-hover); }
        .btn-tonal { background: var(--md-surface-variant); color: var(--md-text-primary); border: 1px solid var(--md-border); }
        .btn-tonal:hover { background: #e8eaed; }
        .btn-success { background: var(--md-success); color: #fff; }
        .btn-success:hover { background: #0d652d; }
        .btn-danger { background: var(--md-danger); color: #fff; }
        .btn-danger:hover { background: #a51d19; }

        /* Unified Cockpit Layout */
        .cockpit-grid {
            flex: 1;
            display: grid;
            grid-template-columns: 1fr 420px;
            overflow: hidden;
            padding: 10px 14px;
            gap: 12px;
        }

        /* Main Canvas Panel */
        .canvas-card {
            background: var(--md-surface);
            border: 1px solid var(--md-border);
            border-radius: 10px;
            display: flex;
            flex-direction: column;
            overflow: hidden;
            box-shadow: var(--md-shadow);
        }

        .sub-toolbar {
            display: flex;
            align-items: center;
            justify-content: space-between;
            padding: 8px 12px;
            background: #ffffff;
            border-bottom: 1px solid var(--md-border);
            gap: 8px;
            flex-wrap: wrap;
        }
        .toolbar-group { display: flex; align-items: center; gap: 6px; }

        #tree-canvas {
            flex: 1;
            width: 100%;
            height: 100%;
            cursor: grab;
            background-color: #fafafa;
            background-image: radial-gradient(#e0e0e0 1px, transparent 1px);
            background-size: 20px 20px;
        }

        /* Side Cockpit Dock */
        .side-dock {
            display: flex;
            flex-direction: column;
            gap: 10px;
            overflow-y: auto;
        }
        .dock-card {
            background: var(--md-surface);
            border: 1px solid var(--md-border);
            border-radius: 8px;
            padding: 10px 12px;
            box-shadow: 0 1px 2px rgba(60,64,67,0.1);
            display: flex;
            flex-direction: column;
            gap: 6px;
        }
        .dock-card-header {
            display: flex;
            justify-content: space-between;
            align-items: center;
        }
        .dock-card-title {
            font-family: 'Google Sans', sans-serif;
            font-size: 0.76rem;
            font-weight: 700;
            color: var(--md-text-secondary);
            text-transform: uppercase;
            letter-spacing: 0.5px;
        }

        /* Metrics */
        .metrics-grid { display: grid; grid-template-columns: 1fr 1fr 1fr; gap: 6px; }
        .metric-tile {
            background: var(--md-surface-variant);
            border-radius: 6px;
            padding: 6px 4px;
            text-align: center;
        }
        .metric-value { font-family: 'Google Sans', sans-serif; font-size: 0.98rem; font-weight: 700; color: var(--md-text-primary); }
        .metric-label { font-size: 0.65rem; color: var(--md-text-secondary); margin-top: 1px; }

        #loss-canvas { width: 100%; height: 50px; background: #ffffff; border: 1px solid var(--md-border); border-radius: 6px; }

        /* Tables & Lists */
        .material-table { width: 100%; border-collapse: collapse; font-size: 0.72rem; text-align: left; }
        .material-table th { background: var(--md-surface-variant); padding: 6px 8px; font-weight: 600; color: var(--md-text-secondary); border-bottom: 1px solid var(--md-border); }
        .material-table td { padding: 6px 8px; border-bottom: 1px solid #f1f3f4; font-family: 'Roboto Mono', monospace; }

        /* Domain Pill Badges */
        .domain-pill { display: inline-block; padding: 2px 6px; border-radius: 10px; font-size: 0.65rem; font-weight: 600; font-family: sans-serif; text-transform: uppercase; }
        .domain-algebra { background: #e8f0fe; color: #1a73e8; }
        .domain-boolean { background: #fef7e0; color: #b06000; }
        .domain-calculus { background: #f3e8fd; color: #7627bb; }
        .domain-set_theory { background: #e6f4ea; color: #137333; }

        /* Proof scroll */
        .proof-scroll { overflow-y: auto; max-height: 110px; display: flex; flex-direction: column; gap: 4px; }
        .proof-step-tile {
            background: var(--md-success-bg);
            border-left: 3px solid var(--md-success);
            padding: 4px 6px;
            border-radius: 4px;
            font-size: 0.68rem;
            font-family: 'Roboto Mono', monospace;
            color: #0d652d;
            line-height: 1.3;
        }

        /* Form Inputs */
        .form-input {
            padding: 5px 8px;
            border-radius: 6px;
            border: 1px solid var(--md-border);
            font-family: 'Roboto Mono', monospace;
            font-size: 0.75rem;
            outline: none;
        }
        .form-input:focus { border-color: var(--md-primary); box-shadow: 0 0 0 2px rgba(26,115,232,0.2); }
    </style>
</head>
<body>
    <header>
        <div class="brand-section">
            <div class="brand-icon">A</div>
            <div class="brand-text">
                <h1>Axiomatic</h1>
                <p>Autonomous AI Mathematician</p>
            </div>
        </div>

        <!-- Master Engine Controls -->
        <div class="master-controls">
            <button id="master-loop-btn" class="btn btn-success" onclick="toggleMasterAutonomousLoop()" style="padding:6px 18px; font-weight:700;">
                Start Autonomous Engine
            </button>
            <select id="domain-select" onchange="changeDomain(this.value)" class="form-input" style="height:32px; font-weight:600; border-radius:16px;">
                <option value="unified">Unified Multidomain (All Domains)</option>
                <option value="algebra">Abstract Algebra</option>
                <option value="boolean">Boolean Propositional Logic</option>
                <option value="calculus">Symbolic Calculus & Derivatives</option>
                <option value="set_theory">Set Theory & Relations</option>
            </select>
        </div>

        <!-- Header Stats -->
        <div class="header-stats">
            <div class="chip chip-primary">
                Discoveries: <b id="stat-discoveries" style="margin-left:4px;">0</b>
            </div>
            <div class="chip chip-primary">
                Premises: <b id="stat-premises" style="margin-left:4px;">0</b>
            </div>
            <div class="chip chip-success" id="supervisor-chip">
                <div class="chip-pulse"></div>
                <span id="stat-supervisor">Supervisor: Optimal</span>
            </div>
        </div>
    </header>

    <div class="cockpit-grid">
        <!-- Main Interactive Proof Canvas -->
        <div class="canvas-card">
            <!-- Canvas Controls Sub-Toolbar -->
            <div class="sub-toolbar">
                <div class="toolbar-group" style="flex:1; min-width:280px;">
                    <span style="font-size:0.72rem; font-weight:700; color:var(--md-text-secondary);">GOAL:</span>
                    <input type="text" id="goal-input" value="((x + -(x)) + (y * 1)) = (0 + y)" style="flex:1; padding:4px 8px; border-radius:12px; border:1px solid var(--md-border); font-family:'Roboto Mono', monospace; font-size:0.75rem; outline:none;">
                    <button class="btn btn-tonal btn-sm" onclick="setGoalFromInput()">Set Goal</button>
                    <button class="btn btn-filled btn-sm" onclick="stepMcts(15)">Step +15</button>
                    <button class="btn btn-tonal btn-sm" onclick="applyInduction()">Induction</button>
                    <button class="btn btn-tonal btn-sm" onclick="nextGoal()">Next</button>
                </div>

                <div class="toolbar-group" style="border-left:1px solid var(--md-border); padding-left:6px;">
                    <label style="display:flex; align-items:center; gap:4px; font-size:0.72rem; color:var(--md-text-secondary); cursor:pointer; margin-right:4px;">
                        <input type="checkbox" id="chk-autofollow" checked style="cursor:pointer;">
                        <span>Live Follow</span>
                    </label>
                    <button id="btn-filt-proven" class="btn btn-tonal btn-sm" onclick="setTreeFilter('proven')" style="font-weight:600; background:#e6f4ea; color:#137333;">Proven Path</button>
                    <button id="btn-filt-visited" class="btn btn-tonal btn-sm" onclick="setTreeFilter('visited')">Visited (N>0)</button>
                    <button id="btn-filt-all" class="btn btn-tonal btn-sm" onclick="setTreeFilter('all')">All Nodes</button>
                    <button class="btn btn-tonal btn-sm" onclick="fitTreeToCanvas()">Fit & Center</button>
                    <button class="btn btn-tonal btn-sm" onclick="zoomIn()">+</button>
                    <button class="btn btn-tonal btn-sm" onclick="zoomOut()">-</button>
                    <button class="btn btn-tonal btn-sm" onclick="exportProof('lean4')">Lean 4</button>
                    <button class="btn btn-tonal btn-sm" onclick="exportProof('coq')">Coq</button>
                    <button class="btn btn-tonal btn-sm" onclick="exportProof('latex')">LaTeX</button>
                </div>
            </div>

            <div style="padding:4px 12px; background:#fff; border-bottom:1px solid var(--md-border); display:flex; justify-content:space-between; align-items:center;">
                <div style="font-size:0.72rem; color:var(--md-text-secondary);">
                    Visible Nodes: <b id="nodes-count" style="color:var(--md-text-primary);">0</b> &nbsp;|&nbsp; Iterations: <b id="iter-count" style="color:var(--md-primary);">0</b>
                </div>
                <div style="display:flex; align-items:center; gap:6px;">
                    <span id="canvas-status-tag" style="font-size:0.72rem; font-weight:700; color:var(--md-primary);">Explored</span>
                </div>
            </div>

            <canvas id="tree-canvas"></canvas>
        </div>

        <!-- Right Side Unified Cockpit Dock -->
        <div class="side-dock">
            <!-- 1. Active Proof Trace Card -->
            <div class="dock-card">
                <div class="dock-card-header">
                    <span class="dock-card-title">Formal Proof Trace (Q.E.D.)</span>
                    <span id="proof-status-badge" style="font-size:0.7rem; font-weight:700; color:var(--md-success);">Certified</span>
                </div>
                <div class="proof-scroll" id="proof-container">
                    <div style="font-size:0.7rem; color:var(--md-text-secondary);">Watch search tree grow in real-time or click "Start Autonomous Engine"...</div>
                </div>
            </div>

            <!-- 2. Live Autonomous Discoveries Feed -->
            <div class="dock-card" style="flex:1; max-height:280px; overflow:hidden;">
                <div class="dock-card-header">
                    <span class="dock-card-title">Live Discoveries Feed</span>
                    <span id="feed-count-badge" class="domain-pill domain-algebra">0 theorems</span>
                </div>
                <div style="overflow-y:auto; flex:1;">
                    <table class="material-table">
                        <thead>
                            <tr>
                                <th>Theorem</th>
                                <th>Domain</th>
                                <th>Steps</th>
                                <th>Action</th>
                            </tr>
                        </thead>
                        <tbody id="discoveries-tbody">
                            <tr><td colspan="4" style="color:var(--md-text-secondary);">Discoveries will stream here live...</td></tr>
                        </tbody>
                    </table>
                </div>
            </div>

            <!-- 3. Neural Network Learning & Loss Progression -->
            <div class="dock-card">
                <div class="dock-card-header">
                    <span class="dock-card-title">Neural Policy & Loss Curve</span>
                    <span id="lr-badge" style="font-size:0.7rem; font-weight:700; color:var(--md-primary);">LR: 0.0050</span>
                </div>
                <div class="metrics-grid">
                    <div class="metric-tile">
                        <div class="metric-value" id="loss-val">--</div>
                        <div class="metric-label">Loss</div>
                    </div>
                    <div class="metric-tile">
                        <div class="metric-value" id="epochs-val">0</div>
                        <div class="metric-label">Epochs</div>
                    </div>
                    <div class="metric-tile">
                        <div class="metric-value" id="solve-val" style="color:var(--md-success);">100%</div>
                        <div class="metric-label">Solve Rate</div>
                    </div>
                </div>
                <canvas id="loss-canvas"></canvas>
            </div>

            <!-- 4. Vector Knowledge Base Quick-Search -->
            <div class="dock-card">
                <div class="dock-card-header">
                    <span class="dock-card-title">Vector Knowledge Base</span>
                </div>
                <div style="display:flex; gap:6px;">
                    <input type="text" id="vdb-search-inp" placeholder="Query premise (e.g. !(a & b))..." class="form-input" style="flex:1;">
                    <button class="btn btn-filled btn-sm" onclick="searchVectorDB()">Search</button>
                </div>
                <div id="vdb-results" style="display:none; font-size:0.7rem; font-family:monospace; background:var(--md-surface-variant); padding:6px; border-radius:4px;"></div>
            </div>
        </div>
    </div>

    <script>
        const canvas = document.getElementById('tree-canvas');
        const ctx = canvas.getContext('2d');
        const lossCanvas = document.getElementById('loss-canvas');
        const lossCtx = lossCanvas.getContext('2d');

        let nodes = [];
        let lossHistory = [];
        let zoom = 1.0;
        let panX = 120;
        let panY = 50;
        let isDragging = false;
        let startX, startY;
        let selectedNodeId = 0;
        let isAutonomousActive = false;
        let currentConjectureIdx = 0;
        let treeFilterMode = 'proven'; // 'proven' | 'visited' | 'all'
        let isRedrawScheduled = false;
        let autoFitNextRedraw = false;

        function setTreeFilter(mode) {
            treeFilterMode = mode;
            ['proven', 'visited', 'all'].forEach(m => {
                const btn = document.getElementById(`btn-filt-${m}`);
                if (btn) {
                    if (m === mode) {
                        btn.style.background = m === 'proven' ? '#e6f4ea' : '#e8eaed';
                        btn.style.color = m === 'proven' ? '#137333' : '#202124';
                        btn.style.fontWeight = '700';
                    } else {
                        btn.style.background = 'var(--md-surface-variant)';
                        btn.style.color = 'var(--md-text-primary)';
                        btn.style.fontWeight = '500';
                    }
                }
            });
            scheduleRedraw(true);
        }

        function scheduleRedraw(autoFit = false) {
            if (autoFit) autoFitNextRedraw = true;
            if (!isRedrawScheduled) {
                isRedrawScheduled = true;
                requestAnimationFrame(performRedraw);
            }
        }

        function performRedraw() {
            isRedrawScheduled = false;
            const visNodes = getVisibleNodes();
            document.getElementById('nodes-count').innerText = `${visNodes.length} / ${nodes.length}`;
            
            const provenNode = nodes.find(n => n.is_proven);
            if (provenNode) {
                document.getElementById('canvas-status-tag').innerText = 'PROVEN (Q.E.D.)';
                document.getElementById('canvas-status-tag').style.color = '#137333';
                highlightProof(provenNode.id);
            } else {
                document.getElementById('canvas-status-tag').innerText = 'Explored';
                document.getElementById('canvas-status-tag').style.color = '#1a73e8';
            }

            layoutTree(visNodes);
            if (autoFitNextRedraw) {
                autoFitNextRedraw = false;
                fitTreeToCanvas(visNodes);
            } else {
                renderTree(visNodes);
            }
        }

        function isAutoFollow() {
            const chk = document.getElementById('chk-autofollow');
            return chk ? chk.checked : true;
        }

        function resizeCanvases() {
            if (canvas && canvas.parentElement) {
                canvas.width = canvas.parentElement.clientWidth;
                canvas.height = canvas.parentElement.clientHeight - 80;
            }
            if (lossCanvas && lossCanvas.parentElement) {
                lossCanvas.width = lossCanvas.parentElement.clientWidth - 24;
                lossCanvas.height = 50;
            }
            scheduleRedraw(false);
            renderLossChart();
        }
        window.addEventListener('resize', resizeCanvases);
        setTimeout(resizeCanvases, 100);

        const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
        const ws = new WebSocket(`${protocol}//${window.location.host}/ws`);

        ws.onmessage = (event) => {
            try {
                const data = JSON.parse(event.data);
                if (!isAutoFollow()) return;

                if (data.TreeReset) {
                    nodes = data.TreeReset.nodes || [];
                    scheduleRedraw(true);
                } else if (data.nodes) {
                    nodes = data.nodes;
                    scheduleRedraw(false);
                } else if (data.NodeCreated) {
                    if (!nodes.some(n => n.id === data.NodeCreated.id)) {
                        nodes.push(data.NodeCreated);
                        scheduleRedraw(false);
                    }
                } else if (data.NodeVisited) {
                    const n = nodes.find(x => x.id === data.NodeVisited.id);
                    if (n) {
                        n.visit_count = data.NodeVisited.visits;
                        n.mean_value = data.NodeVisited.mean_value;
                        scheduleRedraw(false);
                    }
                } else if (data.ProofDiscovered) {
                    highlightProof(data.ProofDiscovered.node_id);
                    scheduleRedraw(true);
                }
            } catch (e) { console.error(e); }
        };

        function getVisibleNodes() {
            if (nodes.length === 0) return [];
            const provenNode = nodes.find(n => n.is_proven);

            if (treeFilterMode === 'proven' && provenNode) {
                const pathIds = new Set();
                let curr = provenNode;
                while (curr) {
                    pathIds.add(curr.id);
                    curr = nodes.find(n => n.id === curr.parent_id);
                }
                // Include root and siblings of first level
                nodes.forEach(n => {
                    if (n.parent_id === 0 && pathIds.has(0)) pathIds.add(n.id);
                });
                return nodes.filter(n => pathIds.has(n.id));
            } else if (treeFilterMode === 'visited' || (treeFilterMode === 'proven' && !provenNode)) {
                return nodes.filter(n => n.visit_count > 0 || n.id === 0 || n.depth <= 1 || n.is_proven);
            }
            return nodes;
        }

        async function fetchStatus() {
            try {
                const res = await fetch('/api/status');
                const data = await res.json();
                document.getElementById('goal-input').value = data.conjecture;
                document.getElementById('iter-count').innerText = data.iterations;
                document.getElementById('stat-discoveries').innerText = data.invented_theorems_count || 0;
                document.getElementById('stat-premises').innerText = data.vector_premises_count || 0;
                document.getElementById('epochs-val').innerText = data.training_epochs || 0;

                if (data.active_domain) {
                    const sel = document.getElementById('domain-select');
                    const domKey = data.active_domain.toLowerCase().includes('bool') ? 'boolean' : (data.active_domain.toLowerCase().includes('calc') ? 'calculus' : (data.active_domain.toLowerCase().includes('set') ? 'set_theory' : (data.active_domain.toLowerCase().includes('unified') ? 'unified' : 'algebra')));
                    if (sel && sel.value !== domKey) sel.value = domKey;
                }

                if (data.is_continuous_discovery_active !== undefined) {
                    isAutonomousActive = data.is_continuous_discovery_active;
                    const btn = document.getElementById('master-loop-btn');
                    if (isAutonomousActive) {
                        btn.className = 'btn btn-danger';
                        btn.innerText = 'Pause Autonomous Engine';
                    } else {
                        btn.className = 'btn btn-success';
                        btn.innerText = 'Start Autonomous Engine';
                    }
                }

                if (data.current_learning_rate !== undefined) {
                    document.getElementById('lr-badge').innerText = `LR: ${data.current_learning_rate.toFixed(4)}`;
                }
                if (data.supervisor_status) {
                    document.getElementById('stat-supervisor').innerText = `Supervisor: ${data.supervisor_status}`;
                }
                if (data.latest_loss !== null && data.latest_loss !== undefined) {
                    document.getElementById('loss-val').innerText = data.latest_loss.toFixed(3);
                }
                if (data.solve_rate !== undefined) {
                    const solveEl = document.getElementById('solve-val');
                    if (solveEl) {
                        solveEl.innerText = `${data.solve_rate.toFixed(0)}%`;
                        if (data.solve_rate >= 80) {
                            solveEl.style.color = '#137333';
                        } else if (data.solve_rate >= 50) {
                            solveEl.style.color = '#e37400';
                        } else {
                            solveEl.style.color = '#c5221f';
                        }
                    }
                }
                if (data.loss_history && data.loss_history.length > 0) {
                    lossHistory = data.loss_history;
                    renderLossChart();
                }
                if (data.graph && data.graph.nodes) {
                    nodes = data.graph.nodes;
                    scheduleRedraw();
                }
            } catch (e) { console.error(e); }
        }
        fetchStatus();
        setInterval(fetchStatus, 2000);

        async function fetchDiscoveriesFeed() {
            try {
                const res = await fetch('/api/invented');
                const data = await res.json();
                document.getElementById('feed-count-badge').innerText = `${data.total_invented || 0} theorems`;

                const tbody = document.getElementById('discoveries-tbody');
                if (data.theorems && data.theorems.length > 0) {
                    tbody.innerHTML = '';
                    data.theorems.slice(0, 15).forEach(t => {
                        const tr = document.createElement('tr');
                        const dom = t.domain || 'Algebra';
                        const domClass = dom.toLowerCase().includes('bool') ? 'domain-boolean' : (dom.toLowerCase().includes('calc') ? 'domain-calculus' : (dom.toLowerCase().includes('set') ? 'domain-set_theory' : 'domain-algebra'));
                        const safeConj = t.conjecture.replace(/"/g, '&quot;').replace(/'/g, "\\'");

                        tr.innerHTML = `<td><code>${t.conjecture}</code></td><td><span class="domain-pill ${domClass}">${dom}</span></td><td>${t.proof_steps}</td><td><button class="btn btn-tonal btn-sm" onclick="loadAndProveGoal('${safeConj}')" style="padding:1px 5px; font-size:0.65rem;">Load</button></td>`;
                        tbody.appendChild(tr);
                    });
                }
            } catch (e) { console.error(e); }
        }
        fetchDiscoveriesFeed();
        setInterval(fetchDiscoveriesFeed, 2500);

        async function toggleMasterAutonomousLoop() {
            if (isAutonomousActive) {
                await fetch('/api/discovery/continuous/stop', { method: 'POST' });
                isAutonomousActive = false;
                document.getElementById('master-loop-btn').className = 'btn btn-success';
                document.getElementById('master-loop-btn').innerText = 'Start Autonomous Engine';
            } else {
                const dom = document.getElementById('domain-select').value;
                await fetch('/api/discovery/continuous/start', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ domain: dom, interval_ms: 1200 })
                });
                isAutonomousActive = true;
                document.getElementById('master-loop-btn').className = 'btn btn-danger';
                document.getElementById('master-loop-btn').innerText = 'Pause Autonomous Engine';
            }
            fetchStatus();
        }

        async function changeDomain(domKey) {
            await fetch('/api/domain/select', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ domain: domKey })
            });
            fetchStatus();
        }

        async function setGoalFromInput() {
            const eq = document.getElementById('goal-input').value.trim();
            if (!eq) return;
            await loadAndProveGoal(eq);
        }

        async function loadAndProveGoal(eq) {
            document.getElementById('goal-input').value = eq;
            const res = await fetch('/api/conjecture/custom', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ equation: eq })
            });
            const data = await res.json();
            if (data.status === 'ok' && data.graph && data.graph.nodes) {
                nodes = data.graph.nodes;
                scheduleRedraw(true);
            }
            fetchStatus();
        }

        async function stepMcts(steps) {
            const res = await fetch('/api/step', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ steps: steps })
            });
            const data = await res.json();
            if (data.graph && data.graph.nodes) {
                nodes = data.graph.nodes;
                scheduleRedraw(true);
            }
            fetchStatus();
        }

        async function applyInduction() {
            const res = await fetch('/api/induction', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ variable: 'n' })
            });
            const data = await res.json();
            if (data.status === 'ok' && data.graph && data.graph.nodes) {
                nodes = data.graph.nodes;
                scheduleRedraw(true);
            }
        }

        async function nextGoal() {
            currentConjectureIdx++;
            const res = await fetch('/api/conjecture', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ index: currentConjectureIdx })
            });
            const data = await res.json();
            if (data.graph && data.graph.nodes) {
                nodes = data.graph.nodes;
                scheduleRedraw(true);
            }
            fetchStatus();
        }

        async function searchVectorDB() {
            const q = document.getElementById('vdb-search-inp').value.trim();
            if (!q) return;
            const res = await fetch('/api/vectordb/search', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ query: q, top_k: 3 })
            });
            const data = await res.json();
            const container = document.getElementById('vdb-results');
            if (data.status === 'ok' && data.results) {
                container.style.display = 'block';
                container.innerHTML = data.results.map(r => `<div><b>${r.payload.name}</b>: ${r.payload.statement} (${r.score.toFixed(3)})</div>`).join('');
            }
        }

        function exportProof(fmt) {
            window.open(`/api/export/${fmt}`, '_blank');
        }

        function highlightProof(provenId) {
            const list = document.getElementById('proof-container');
            list.innerHTML = '';
            let curr = nodes.find(n => n.id === provenId);
            if (!curr) return;

            const steps = [];
            while (curr) {
                if (curr.applied_tactic) steps.unshift(curr.applied_tactic);
                curr = nodes.find(n => n.id === curr.parent_id);
            }

            steps.forEach((step, idx) => {
                const el = document.createElement('div');
                el.className = 'proof-step-tile';
                el.innerText = `${idx + 1}. ${typeof step === 'string' ? step : (step.RewriteLhs ? `rw [${step.RewriteLhs}]` : (step.RewriteRhs ? `nth_rw [${step.RewriteRhs}]` : JSON.stringify(step)))}`;
                list.appendChild(el);
            });
        }

        function renderLossChart() {
            lossCtx.clearRect(0, 0, lossCanvas.width, lossCanvas.height);
            if (lossHistory.length < 2) return;

            const maxLoss = Math.max(...lossHistory) * 1.05;
            const minLoss = Math.min(...lossHistory) * 0.95;
            const stepX = (lossCanvas.width - 16) / (lossHistory.length - 1);

            lossCtx.beginPath();
            lossCtx.strokeStyle = '#7627bb';
            lossCtx.lineWidth = 2.0;

            lossHistory.forEach((val, i) => {
                const x = 8 + i * stepX;
                const y = lossCanvas.height - 6 - ((val - minLoss) / (maxLoss - minLoss + 1e-6)) * (lossCanvas.height - 12);
                if (i === 0) lossCtx.moveTo(x, y); else lossCtx.lineTo(x, y);
            });
            lossCtx.stroke();
        }

        function zoomIn() {
            zoom = Math.min(3.0, zoom * 1.25);
            renderTree(getVisibleNodes());
        }

        function zoomOut() {
            zoom = Math.max(0.15, zoom / 1.25);
            renderTree(getVisibleNodes());
        }

        function fitTreeToCanvas(visNodes = null) {
            const vNodes = visNodes || getVisibleNodes();
            if (vNodes.length === 0) return;

            let minX = Infinity, maxX = -Infinity, minY = Infinity, maxY = -Infinity;
            vNodes.forEach(n => {
                minX = Math.min(minX, n.x || 0);
                maxX = Math.max(maxX, n.x || 0);
                minY = Math.min(minY, n.y || 0);
                maxY = Math.max(maxY, n.y || 0);
            });

            const pad = 60;
            const w = Math.max(maxX - minX, 80);
            const h = Math.max(maxY - minY, 80);

            zoom = Math.min((canvas.width - pad * 2) / w, (canvas.height - pad * 2) / h, 1.2);
            zoom = Math.max(0.25, Math.min(1.4, zoom));

            panX = (canvas.width - w * zoom) / 2 - minX * zoom;
            panY = (canvas.height - h * zoom) / 2 - minY * zoom;

            renderTree(vNodes);
        }

        function layoutTree(visNodes) {
            if (visNodes.length === 0) return;

            const visIds = new Set(visNodes.map(n => n.id));
            const childMap = new Map();
            visNodes.forEach(n => {
                if (n.parent_id !== null && visIds.has(n.parent_id)) {
                    if (!childMap.has(n.parent_id)) childMap.set(n.parent_id, []);
                    childMap.get(n.parent_id).push(n.id);
                }
            });

            const nodeMap = new Map();
            visNodes.forEach(n => nodeMap.set(n.id, n));

            function computeLayout(nodeId, depth, startY) {
                const node = nodeMap.get(nodeId);
                if (!node) return 0;

                node.x = depth * 200 + 70;
                const children = childMap.get(nodeId) || [];

                if (children.length === 0) {
                    node.y = startY;
                    return 55;
                }

                let curY = startY;
                let totalH = 0;
                children.forEach(cid => {
                    const h = computeLayout(cid, depth + 1, curY);
                    curY += h;
                    totalH += h;
                });

                const firstChild = nodeMap.get(children[0]);
                const lastChild = nodeMap.get(children[children.length - 1]);
                node.y = (firstChild.y + lastChild.y) / 2;

                return Math.max(totalH, 55);
            }

            const root = visNodes.find(n => n.parent_id === null || !visIds.has(n.parent_id)) || visNodes[0];
            if (root) {
                computeLayout(root.id, 0, 0);
            }
        }

        function renderTree(visNodes) {
            ctx.clearRect(0, 0, canvas.width, canvas.height);
            ctx.save();
            ctx.translate(panX, panY);
            ctx.scale(zoom, zoom);

            if (!visNodes || visNodes.length === 0) { ctx.restore(); return; }

            const visIds = new Set(visNodes.map(n => n.id));
            const nodeMap = new Map();
            visNodes.forEach(n => nodeMap.set(n.id, n));

            visNodes.forEach(node => {
                if (node.parent_id !== null && visIds.has(node.parent_id)) {
                    const parent = nodeMap.get(node.parent_id);
                    ctx.beginPath();
                    ctx.strokeStyle = node.is_proven ? '#137333' : '#cbd5e1';
                    ctx.lineWidth = node.is_proven ? 3 : 1.5;

                    const midX = (parent.x + node.x) / 2;
                    ctx.moveTo(parent.x, parent.y);
                    ctx.bezierCurveTo(midX, parent.y, midX, node.y, node.x, node.y);
                    ctx.stroke();

                    if (node.applied_tactic) {
                        const tacticStr = typeof node.applied_tactic === 'string'
                            ? node.applied_tactic
                            : (node.applied_tactic.RewriteLhs ? `rw [${node.applied_tactic.RewriteLhs}]` : (node.applied_tactic.RewriteRhs ? `nth_rw [${node.applied_tactic.RewriteRhs}]` : JSON.stringify(node.applied_tactic)));

                        const labelX = (parent.x + node.x) / 2;
                        const labelY = (parent.y + node.y) / 2 - 6;

                        ctx.font = '500 9px "Roboto Mono", monospace';
                        const textWidth = ctx.measureText(tacticStr).width;

                        ctx.fillStyle = node.is_proven ? '#e6f4ea' : '#f1f3f4';
                        ctx.fillRect(labelX - textWidth / 2 - 4, labelY - 8, textWidth + 8, 14);
                        ctx.strokeStyle = node.is_proven ? '#137333' : '#dadce0';
                        ctx.lineWidth = 1;
                        ctx.strokeRect(labelX - textWidth / 2 - 4, labelY - 8, textWidth + 8, 14);

                        ctx.fillStyle = node.is_proven ? '#137333' : '#5f6368';
                        ctx.textAlign = 'center';
                        ctx.textBaseline = 'middle';
                        ctx.fillText(tacticStr, labelX, labelY - 1);
                    }
                }
            });

            visNodes.forEach(node => {
                ctx.beginPath();
                const radius = Math.min(20, Math.max(12, 12 + Math.log2(node.visit_count + 1) * 2.2));
                ctx.arc(node.x, node.y, radius, 0, 2 * Math.PI);

                if (node.is_proven) {
                    ctx.fillStyle = '#34a853';
                    ctx.fill();
                    ctx.strokeStyle = '#137333';
                    ctx.lineWidth = 2.5;
                    ctx.stroke();
                } else if (node.id === selectedNodeId) {
                    ctx.fillStyle = '#1a73e8';
                    ctx.fill();
                    ctx.strokeStyle = '#1557b0';
                    ctx.lineWidth = 2.5;
                    ctx.stroke();
                } else if (node.id === 0) {
                    ctx.fillStyle = '#f9ab00';
                    ctx.fill();
                    ctx.strokeStyle = '#e37400';
                    ctx.lineWidth = 2;
                    ctx.stroke();
                } else {
                    ctx.fillStyle = '#ffffff';
                    ctx.fill();
                    ctx.strokeStyle = '#9aa0a6';
                    ctx.lineWidth = 1.5;
                    ctx.stroke();
                }

                ctx.fillStyle = (node.is_proven || node.id === selectedNodeId || node.id === 0) ? '#ffffff' : '#202124';
                ctx.font = '500 9px "Roboto Mono", monospace';
                ctx.textAlign = 'center';
                ctx.textBaseline = 'middle';
                ctx.fillText(`N:${node.visit_count}`, node.x, node.y);
            });

            ctx.restore();
        }

        canvas.addEventListener('mousedown', e => {
            isDragging = true;
            startX = e.clientX - panX;
            startY = e.clientY - panY;
        });

        window.addEventListener('mouseup', () => isDragging = false);

        window.addEventListener('mousemove', e => {
            if (isDragging) {
                panX = e.clientX - startX;
                panY = e.clientY - startY;
                renderTree(getVisibleNodes());
            }
        });

        canvas.addEventListener('wheel', e => {
            e.preventDefault();
            const mouseX = e.clientX - canvas.getBoundingClientRect().left;
            const mouseY = e.clientY - canvas.getBoundingClientRect().top;
            const scaleFactor = e.deltaY < 0 ? 1.15 : 0.85;

            const newZoom = Math.max(0.15, Math.min(3.0, zoom * scaleFactor));
            panX = mouseX - (mouseX - panX) * (newZoom / zoom);
            panY = mouseY - (mouseY - panY) * (newZoom / zoom);
            zoom = newZoom;
            renderTree(getVisibleNodes());
        });

        canvas.addEventListener('click', e => {
            const rect = canvas.getBoundingClientRect();
            const mouseX = (e.clientX - rect.left - panX) / zoom;
            const mouseY = (e.clientY - rect.top - panY) / zoom;

            let closest = null;
            let minDist = 25;
            const visNodes = getVisibleNodes();

            visNodes.forEach(n => {
                const dist = Math.hypot(n.x - mouseX, n.y - mouseY);
                if (dist < minDist) {
                    minDist = dist;
                    closest = n;
                }
            });

            if (closest) {
                selectedNodeId = closest.id;
                renderTree(visNodes);
            }
        });
    </script>
</body>
</html>
"#;
