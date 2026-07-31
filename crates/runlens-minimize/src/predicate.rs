use std::process::Stdio;

#[derive(Debug, Clone, PartialEq)]
pub enum PredicateResult {
    Pass,
    Fail,
    Inconclusive,
}

pub struct Predicate {
    command: Vec<String>,
}

impl Predicate {
    pub const fn new(command: Vec<String>) -> Self {
        Self { command }
    }

    pub async fn run(&self, delta_dir: &std::path::Path) -> PredicateResult {
        let cmd = &self.command[0];
        let args = &self.command[1..];
        let status = tokio::process::Command::new(cmd)
            .args(args)
            .current_dir(delta_dir)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await;
        match status {
            Ok(s) if s.success() => PredicateResult::Pass,
            Ok(_) => PredicateResult::Fail,
            Err(e) => {
                tracing::warn!("predicate error: {e}");
                PredicateResult::Inconclusive
            }
        }
    }

    pub async fn shell(cmd: &str) -> std::io::Result<PredicateResult> {
        let status = tokio::process::Command::new(if cfg!(target_os = "windows") { "cmd" } else { "sh" })
            .args(if cfg!(target_os = "windows") { ["/C", cmd] } else { ["-c", cmd] })
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await?;
        Ok(if status.success() { PredicateResult::Pass } else { PredicateResult::Fail })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn predicate_executes_command() {
        let result = Predicate::shell("echo ok").await.unwrap();
        assert_eq!(result, PredicateResult::Pass);
    }

    #[tokio::test]
    async fn predicate_fails_on_error() {
        let result = Predicate::shell("exit 1).await.unwrap();
        assert_eq!(result, PredicateResult::Fail);
    }
}
