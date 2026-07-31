pub struct RegressionDetector;

impl RegressionDetector {
    pub fn new() -> Self {
        Self
    }

    pub fn compare(&self, _baseline_id: &str, _events: Vec<()>) -> RegressionReport {
        RegressionReport {
            summary: RegressionSummary {
                failed: 0,
                severity: RegressionSeverity::None,
            },
        }
    }

    pub fn list_baselines(&self) -> Vec<()> {
        vec![]
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct RegressionReport {
    pub summary: RegressionSummary,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct RegressionSummary {
    pub failed: usize,
    pub severity: RegressionSeverity,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub enum RegressionSeverity {
    None,
    Low,
    Medium,
    High,
    Critical,
}