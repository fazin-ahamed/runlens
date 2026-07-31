use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnosis {
    pub answer: String,
    pub confidence: f64,
}

pub struct DiagnosisEngine;

impl DiagnosisEngine {
    pub fn new() -> Self {
        Self
    }

    pub fn add_evidence(&mut self, _session_id: &str, _evidence: Vec<String>) {}

    pub fn diagnose(&self, _session_id: &str, _question: &str) -> Diagnosis {
        Diagnosis {
            answer: "No anomaly detected".to_string(),
            confidence: 1.0,
        }
    }
}
