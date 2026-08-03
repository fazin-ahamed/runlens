use crate::engine::BisectResult;
use std::fmt;

#[derive(Debug)]
pub struct BisectReport {
    pub result: String,
}

impl fmt::Display for BisectReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Bisect Result: {}", self.result)
    }
}

pub fn generate_report(result: &BisectResult) -> BisectReport {
    let message = match &result.found {
        Some(commit) => format!(
            "first bad commit {commit} found after {} evaluations",
            result.evaluations
        ),
        None => format!(
            "no regression found in range ({} evaluations)",
            result.evaluations
        ),
    };
    BisectReport { result: message }
}