pub mod predicate {
    use std::process::Stdio;

    pub struct Predicate {
        pub command: Vec<String>,
    }

    impl Predicate {
        pub const fn new(command: Vec<String>) -> Self {
            Self { command }
        }

        /// Run the predicate command in `dir`. Returns `true` when the command
        /// exits successfully, `false` when it fails, and falls back to `true`
        /// if the command cannot be spawned (treated as a non reproducing run).
        pub async fn run(&self, dir: &std::path::Path) -> bool {
            if self.command.is_empty() {
                return true;
            }
            let status = tokio::process::Command::new(&self.command[0])
                .args(&self.command[1..])
                .current_dir(dir)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .await;
            match status {
                Ok(s) => s.success(),
                Err(e) => {
                    tracing::warn!("predicate error: {e}");
                    true
                },
            }
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

    pub fn format_explanation(result: &MinimizeResult, dimension: &str) -> String {
        if result.delta.is_empty() {
            return format!(
                "No reduction found; kept all {dimension} items after {} evaluation{}",
                result.evaluations,
                if result.evaluations == 1 { "" } else { "s" }
            );
        }
        format!(
            "minimized {dimension} from {} items to {} ({} evaluation{}, {} steps)",
            result.steps.len() + result.delta.len(),
            result.delta.len(),
            result.evaluations,
            if result.evaluations == 1 { "" } else { "s" },
            result.steps.len()
        )
    }
}

pub mod engine {
    use crate::explain::MinimizeResult;

    /// Remove files one at a time while the predicate still returns false
    /// (that is, the failure still reproduces without the file). Each removed
    /// file is recorded as a step; the predicate is evaluated per candidate.
    pub async fn minimize<F>(files: Vec<String>, mut predicate: F) -> MinimizeResult
    where
        F: FnMut(&[String]) -> bool,
    {
        let mut delta = files;
        delta.sort();
        let mut steps: Vec<String> = Vec::new();
        let mut evaluations = 0usize;

        let mut idx = 0usize;
        while idx < delta.len() {
            let mut candidate = delta.clone();
            candidate.swap_remove(idx);
            evaluations += 1;
            // Predicate returns true when the failure still reproduces. If
            // the reproduction survives without a file, that file is not
            // needed and can be dropped from the minimized set.
            if predicate(&candidate) {
                let dropped = delta.swap_remove(idx);
                steps.push(format!("drop {dropped}"));
            } else {
                idx += 1;
            }
        }

        MinimizeResult {
            delta,
            evaluations,
            steps,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::engine::{self, minimize};

    #[tokio::test]
    async fn drops_files_that_do_not_reproduce() {
        let files = vec!["a".into(), "b".into(), "c".into()];
        // Reproduction depends only on "a"; removing b or c keeps the failure.
        let result = engine::minimize(files, |subset| subset.contains(&"a".to_string())).await;
        assert_eq!(result.delta, vec!["a".to_string()]);
        assert!(result.evaluations >= 2);
        assert!(!result.steps.is_empty());
    }

    #[tokio::test]
    async fn empty_input_yields_empty_delta() {
        let result = minimize(Vec::<String>::new(), |_| true).await;
        assert!(result.delta.is_empty());
        assert_eq!(result.evaluations, 0);
    }
}
