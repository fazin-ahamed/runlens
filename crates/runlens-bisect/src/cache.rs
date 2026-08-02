use std::collections::HashMap;

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct BisectCache {
    pub good: Option<String>,
    pub bad: Option<String>,
    entries: HashMap<String, String>,
}

impl BisectCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn known_good(&self) -> Option<&str> {
        self.good.as_deref()
    }

    pub fn known_bad(&self) -> Option<&str> {
        self.bad.as_deref()
    }

    pub fn record(&mut self, commit: &str, result: crate::predicate::PredicateResult) {
        match result {
            crate::predicate::PredicateResult::Good => {
                self.good = Some(commit.to_string());
                self.entries.insert(commit.to_string(), "good".to_string());
            }
            crate::predicate::PredicateResult::Bad => {
                self.bad = Some(commit.to_string());
                self.entries.insert(commit.to_string(), "bad".to_string());
            }
            crate::predicate::PredicateResult::Inconclusive => {
                self.entries.insert(commit.to_string(), "inconclusive".to_string());
            }
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }
}
