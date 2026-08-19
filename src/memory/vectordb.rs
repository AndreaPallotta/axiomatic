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
}
