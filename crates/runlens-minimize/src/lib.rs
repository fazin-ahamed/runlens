pub mod predicate {
    use std::path::Path;

    pub struct Predicate {
        pub command: Vec<String>,
    }

    impl Predicate {
        pub fn new(command: Vec<String>) -> Self {
            Self { command }
        }

        pub async fn run(&self, _dir: &Path) -> bool {
            true
        }
    }
}

pub mod explain {
    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub struct MinimizeResult {
        pub delta: Vec<String>,
        pub evaluations: usize,
        pub steps: Vec<String>,
    }

    pub fn format_explanation(_result: &MinimizeResult, _dimension: &str) -> String {
        "Minimized (stub)".to_string()
    }
}

pub mod engine {
    use crate::explain::MinimizeResult;

    pub async fn minimize<F>(_files: Vec<String>, mut _predicate: F) -> MinimizeResult
    where
        F: FnMut(&[String]) -> bool,
    {
        MinimizeResult {
            delta: vec![],
            evaluations: 0,
            steps: vec![],
        }
    }
}
