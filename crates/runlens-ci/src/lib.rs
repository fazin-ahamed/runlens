#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum CiJobStatus {
    Passed,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CiMetric {
    pub name: String,
    pub value: f64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CiJobSummary {
    pub title: String,
    pub status: CiJobStatus,
    pub metrics: Vec<CiMetric>,
    pub regressions: Vec<String>,
    pub artifacts: Vec<String>,
    pub duration_secs: u64,
}

impl CiJobSummary {
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    pub fn to_github_markdown(&self) -> String {
        format!("# {}\nStatus: {:?}", self.title, self.status)
    }
}

pub struct CiEnvironment {
    pub name: String,
}

impl CiEnvironment {
    pub fn detect() -> Self {
        Self {
            name: "GitHub Actions".to_string(),
        }
    }

    pub fn as_str(&self) -> &str {
        &self.name
    }
}
