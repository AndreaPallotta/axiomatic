use serde::{Deserialize, Serialize};

/// Distance Metric for Vector Similarity Search
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DistanceMetric {
    Cosine,
    DotProduct,
    Euclidean,
}

/// Metadata payload stored alongside theorem vectors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TheoremPayload {
    pub name: String,
    pub statement: String,
    pub tactic_name: String,
    pub proof_length: usize,
    pub timestamp: String,
}

/// A Vector Database Record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorRecord {
    pub id: usize,
    pub vector: Vec<f64>,
    pub payload: TheoremPayload,
}

/// Search result with similarity score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoredResult {
    pub record: VectorRecord,
    pub score: f64, // Higher is more similar for Cosine/DotProduct
}

/// Point in 3D projected theorem embedding space
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorPoint3D {
    pub id: usize,
    pub name: String,
    pub statement: String,
    pub domain: String,
    pub tactic_name: String,
    pub proof_length: usize,
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

/// Point along an active search trajectory in 3D
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrajectoryPoint3D {
    pub step: usize,
    pub state_str: String,
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

/// Projected 3D vector space containing theorem points and active trajectory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorSpace3D {
    pub points: Vec<VectorPoint3D>,
    pub trajectory: Vec<TrajectoryPoint3D>,
    pub total_theorems: usize,
}

/// In-Memory Open-Source HNSW / Vector Database for Mathematical Premise Selection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MathematicalVectorDB {
    pub dimension: usize,
    pub metric: DistanceMetric,
    pub records: Vec<VectorRecord>,
}

impl MathematicalVectorDB {
    pub fn new(dimension: usize, metric: DistanceMetric) -> Self {
        Self {
            dimension,
            metric,
            records: Vec::new(),
        }
    }

    /// Inserts a theorem vector with its metadata payload
    pub fn insert(&mut self, vector: Vec<f64>, payload: TheoremPayload) -> usize {
        assert_eq!(vector.len(), self.dimension, "Vector dimension mismatch");
        let id = self.records.len();
        self.records.push(VectorRecord {
            id,
            vector,
            payload,
        });
        id
    }

    /// Queries the vector database for the top-k most semantically relevant theorems
    pub fn query(&self, query_vector: &[f64], top_k: usize) -> Vec<ScoredResult> {
        if self.records.is_empty() || top_k == 0 {
            return Vec::new();
        }

        let mut scored: Vec<ScoredResult> = self
            .records
            .iter()
            .map(|rec| {
                let score = match self.metric {
                    DistanceMetric::Cosine => cosine_similarity(query_vector, &rec.vector),
                    DistanceMetric::DotProduct => dot_product(query_vector, &rec.vector),
                    DistanceMetric::Euclidean => -euclidean_distance(query_vector, &rec.vector),
                };
                ScoredResult {
                    record: rec.clone(),
                    score,
                }
            })
            .collect();

        // Sort descending by similarity score
        scored.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        scored.truncate(top_k);
        scored
    }

    /// Saves the vector database index to disk
    pub fn save_to_file(&self, path: &str) -> std::io::Result<()> {
        if let Some(parent) = std::path::Path::new(path).parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(path, json)
    }

    /// Loads a vector database index from disk
    pub fn load_from_file(path: &str) -> std::io::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        serde_json::from_str(&content)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }

    /// Projects theorem vectors and an optional search trajectory into 3D space via PCA
    pub fn project_to_3d(&self, query_trajectory: &[(String, Vec<f64>)]) -> VectorSpace3D {
        let n = self.records.len();
        if n == 0 {
            return VectorSpace3D {
                points: Vec::new(),
                trajectory: Vec::new(),
                total_theorems: 0,
            };
        }

        let d = self.dimension;
        let mut mean = vec![0.0; d];
        for rec in &self.records {
            for j in 0..d.min(rec.vector.len()) {
                mean[j] += rec.vector[j];
            }
        }
        for j in 0..d {
            mean[j] /= n as f64;
        }

        let centered: Vec<Vec<f64>> = self
            .records
            .iter()
            .map(|rec| {
                let mut v = vec![0.0; d];
                for j in 0..d.min(rec.vector.len()) {
                    v[j] = rec.vector[j] - mean[j];
                }
                v
            })
            .collect();

        let mut basis: Vec<Vec<f64>> = Vec::new();
        for k in 0..3 {
            let mut w = vec![0.0; d];
            for j in 0..d {
                let seed = ((j + 1) * (k + 1) * 31 % 97) as f64 - 48.0;
                w[j] = seed;
            }
            if w.iter().all(|&x| x.abs() < 1e-9) && d > 0 {
                w[k % d] = 1.0;
            }

            for u in &basis {
                let proj = dot_product(&w, u);
                for j in 0..d {
                    w[j] -= proj * u[j];
                }
            }
            let initial_norm: f64 = w.iter().map(|x| x * x).sum::<f64>().sqrt();
            if initial_norm > 1e-9 {
                for j in 0..d {
                    w[j] /= initial_norm;
                }
            }

            for _ in 0..25 {
                let mut next_w = vec![0.0; d];
                for c in &centered {
                    let dot = dot_product(c, &w);
                    for j in 0..d {
                        next_w[j] += c[j] * dot;
                    }
                }
                for j in 0..d {
                    next_w[j] /= n as f64;
                }

                for u in &basis {
                    let proj = dot_product(&next_w, u);
                    for j in 0..d {
                        next_w[j] -= proj * u[j];
                    }
                }

                let norm: f64 = next_w.iter().map(|x| x * x).sum::<f64>().sqrt();
                if norm > 1e-12 {
                    for j in 0..d {
                        w[j] = next_w[j] / norm;
                    }
                } else {
                    break;
                }
            }

            let mut final_norm: f64 = w.iter().map(|x| x * x).sum::<f64>().sqrt();
            if final_norm <= 1e-9 {
                for cand in 0..d {
                    let mut fallback = vec![0.0; d];
                    fallback[cand] = 1.0;
                    for u in &basis {
                        let proj = dot_product(&fallback, u);
                        for j in 0..d {
                            fallback[j] -= proj * u[j];
                        }
                    }
                    let fnorm: f64 = fallback.iter().map(|x| x * x).sum::<f64>().sqrt();
                    if fnorm > 1e-6 {
                        w = fallback;
                        final_norm = fnorm;
                        break;
                    }
                }
            }
            if final_norm > 1e-9 {
                for j in 0..d {
                    w[j] /= final_norm;
                }
            }
            basis.push(w);
        }

        let mut raw_points: Vec<(usize, &TheoremPayload, f64, f64, f64)> = Vec::with_capacity(n);
        let mut max_extent = 0.0f64;

        for (idx, rec) in self.records.iter().enumerate() {
            let c = &centered[idx];
            let x = if !basis.is_empty() { dot_product(c, &basis[0]) } else { 0.0 };
            let y = if basis.len() > 1 { dot_product(c, &basis[1]) } else { 0.0 };
            let z = if basis.len() > 2 { dot_product(c, &basis[2]) } else { 0.0 };
            max_extent = max_extent.max(x.abs()).max(y.abs()).max(z.abs());
            raw_points.push((rec.id, &rec.payload, x, y, z));
        }

        let mut raw_trajectory: Vec<(usize, String, f64, f64, f64)> = Vec::new();
        for (step_idx, (state_str, t_vec)) in query_trajectory.iter().enumerate() {
            let mut c = vec![0.0; d];
            for j in 0..d.min(t_vec.len()) {
                c[j] = t_vec[j] - mean[j];
            }
            let x = if !basis.is_empty() { dot_product(&c, &basis[0]) } else { 0.0 };
            let y = if basis.len() > 1 { dot_product(&c, &basis[1]) } else { 0.0 };
            let z = if basis.len() > 2 { dot_product(&c, &basis[2]) } else { 0.0 };
            max_extent = max_extent.max(x.abs()).max(y.abs()).max(z.abs());
            raw_trajectory.push((step_idx, state_str.clone(), x, y, z));
        }

        let scale = if max_extent > 1e-6 {
            120.0 / max_extent
        } else {
            1.0
        };

        let points: Vec<VectorPoint3D> = raw_points
            .into_iter()
            .enumerate()
            .map(|(i, (id, payload, x, y, z))| {
                let (px, py, pz) = if max_extent <= 1e-6 && n > 1 {
                    let angle = (i as f64) * 2.0 * std::f64::consts::PI / (n as f64);
                    (30.0 * angle.cos(), 0.0, 30.0 * angle.sin())
                } else {
                    (x * scale, y * scale, z * scale)
                };
                VectorPoint3D {
                    id,
                    name: payload.name.clone(),
                    statement: payload.statement.clone(),
                    domain: detect_domain(&payload.name, &payload.statement),
                    tactic_name: payload.tactic_name.clone(),
                    proof_length: payload.proof_length,
                    x: (px * 1000.0).round() / 1000.0,
                    y: (py * 1000.0).round() / 1000.0,
                    z: (pz * 1000.0).round() / 1000.0,
                }
            })
            .collect();

        let trajectory: Vec<TrajectoryPoint3D> = raw_trajectory
            .into_iter()
            .map(|(step, state_str, x, y, z)| TrajectoryPoint3D {
                step,
                state_str,
                x: ((x * scale) * 1000.0).round() / 1000.0,
                y: ((y * scale) * 1000.0).round() / 1000.0,
                z: ((z * scale) * 1000.0).round() / 1000.0,
            })
            .collect();

        VectorSpace3D {
            points,
            trajectory,
            total_theorems: n,
        }
    }
}

/// Cosine similarity between two vectors
pub fn cosine_similarity(a: &[f64], b: &[f64]) -> f64 {
    let dot = dot_product(a, b);
    let norm_a: f64 = a.iter().map(|x| x * x).sum::<f64>().sqrt();
    let norm_b: f64 = b.iter().map(|x| x * x).sum::<f64>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot / (norm_a * norm_b)
    }
}

pub fn dot_product(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

pub fn euclidean_distance(a: &[f64], b: &[f64]) -> f64 {
    a.iter()
        .zip(b)
        .map(|(x, y)| (x - y).powi(2))
        .sum::<f64>()
        .sqrt()
}

/// Categorizes a theorem into a mathematical domain based on name and statement
pub fn detect_domain(name: &str, statement: &str) -> String {
    let lower = format!("{} {}", name.to_lowercase(), statement.to_lowercase());
    if lower.contains("bool")
        || lower.contains("de_morgan")
        || lower.contains('&')
        || lower.contains('|')
        || lower.contains('!')
        || lower.contains("and")
        || lower.contains("or")
        || lower.contains("not")
        || lower.contains("modus")
    {
        "Boolean Logic".to_string()
    } else if lower.contains("diff")
        || lower.contains("d/dx")
        || lower.contains("deriv")
        || lower.contains("int")
        || lower.contains("integral")
        || lower.contains("exp")
        || lower.contains("sin")
        || lower.contains("cos")
        || lower.contains("limit")
    {
        "Symbolic Calculus".to_string()
    } else if lower.contains("subset")
        || lower.contains("union")
        || lower.contains("inter")
        || lower.contains("empty")
        || lower.contains("univ")
        || lower.contains("succ")
        || lower.contains("induct")
        || lower.contains("peano")
    {
        "Set Theory & Induction".to_string()
    } else if lower.contains("matrix")
        || lower.contains("det")
        || lower.contains("trans")
        || lower.contains("trace")
        || lower.contains("vector")
        || lower.contains("dot")
    {
        "Linear Algebra".to_string()
    } else {
        "Abstract Algebra".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vector_db_insert_and_query() {
        let mut db = MathematicalVectorDB::new(4, DistanceMetric::Cosine);

        db.insert(
            vec![1.0, 0.0, 0.0, 0.0],
            TheoremPayload {
                name: "add_zero".to_string(),
                statement: "(x + 0) = x".to_string(),
                tactic_name: "rw_lhs [add_zero]".to_string(),
                proof_length: 1,
                timestamp: "2026-08-18".to_string(),
            },
        );

        db.insert(
            vec![0.0, 1.0, 0.0, 0.0],
            TheoremPayload {
                name: "mul_one".to_string(),
                statement: "(x * 1) = x".to_string(),
                tactic_name: "rw_lhs [mul_one]".to_string(),
                proof_length: 1,
                timestamp: "2026-08-18".to_string(),
            },
        );

        let query_vec = vec![0.9, 0.1, 0.0, 0.0];
        let results = db.query(&query_vec, 1);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].record.payload.name, "add_zero");
        assert!(results[0].score > 0.9);
    }

    #[test]
    fn test_vector_db_pca_3d_projection() {
        let mut db = MathematicalVectorDB::new(4, DistanceMetric::Cosine);

        db.insert(
            vec![10.0, 0.0, 0.0, 0.0],
            TheoremPayload {
                name: "add_zero".to_string(),
                statement: "(x + 0) = x".to_string(),
                tactic_name: "rw_lhs [add_zero]".to_string(),
                proof_length: 1,
                timestamp: "2026-08-18".to_string(),
            },
        );
        db.insert(
            vec![0.0, 10.0, 0.0, 0.0],
            TheoremPayload {
                name: "mul_one".to_string(),
                statement: "(x * 1) = x".to_string(),
                tactic_name: "rw_lhs [mul_one]".to_string(),
                proof_length: 1,
                timestamp: "2026-08-18".to_string(),
            },
        );
        db.insert(
            vec![0.0, 0.0, 10.0, 0.0],
            TheoremPayload {
                name: "de_morgan".to_string(),
                statement: "!(A & B) = (!A | !B)".to_string(),
                tactic_name: "rw_lhs [de_morgan]".to_string(),
                proof_length: 2,
                timestamp: "2026-08-18".to_string(),
            },
        );
        db.insert(
            vec![0.0, 0.0, 0.0, 10.0],
            TheoremPayload {
                name: "diff_exp".to_string(),
                statement: "d/dx exp(x) = exp(x)".to_string(),
                tactic_name: "rw_lhs [diff_exp]".to_string(),
                proof_length: 3,
                timestamp: "2026-08-18".to_string(),
            },
        );

        let trajectory = vec![
            ("Start".to_string(), vec![5.0, 0.0, 0.0, 0.0]),
            ("Step 1".to_string(), vec![0.0, 5.0, 0.0, 0.0]),
        ];

        let space = db.project_to_3d(&trajectory);
        assert_eq!(space.total_theorems, 4);
        assert_eq!(space.points.len(), 4);
        assert_eq!(space.trajectory.len(), 2);

        for pt in &space.points {
            assert!(pt.x.is_finite());
            assert!(pt.y.is_finite());
            assert!(pt.z.is_finite());
            assert!(pt.x.abs() <= 120.001);
            assert!(pt.y.abs() <= 120.001);
            assert!(pt.z.abs() <= 120.001);
        }

        for tr in &space.trajectory {
            assert!(tr.x.is_finite());
            assert!(tr.y.is_finite());
            assert!(tr.z.is_finite());
        }

        assert_eq!(space.points[0].domain, "Abstract Algebra");
        assert_eq!(space.points[2].domain, "Boolean Logic");
        assert_eq!(space.points[3].domain, "Symbolic Calculus");

        let empty_db = MathematicalVectorDB::new(4, DistanceMetric::Cosine);
        let empty_space = empty_db.project_to_3d(&[]);
        assert_eq!(empty_space.total_theorems, 0);
        assert!(empty_space.points.is_empty());
        assert!(empty_space.trajectory.is_empty());
    }
}
