use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PredicateResult {
    Good,
    Bad,
    Inconclusive,
}

pub struct BisectPredicate {
    command: Vec<String>,
}

impl BisectPredicate {
    pub fn new(command: Vec<String>) -> Self {
        Self { command }
    }

    pub async fn run_async(&self, worktree: &Path) -> anyhow::Result<PredicateResult> {
        if self.command.is_empty() {
            return Ok(PredicateResult::Inconclusive);
        }
        let mut cmd = Command::new(&self.command[0]);
        cmd.args(&self.command[1..]);
        cmd.current_dir(worktree);
        let status = cmd.status()?;
        Ok(if status.success() {
            PredicateResult::Good
        } else {
            PredicateResult::Bad
        })
    }
}
