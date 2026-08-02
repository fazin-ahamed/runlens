use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct BisectState {
    pub worktree_path: std::path::PathBuf,
    pub commit: String,
}

pub fn run_git_rev_parse(repo_path: &Path, rev: &str) -> anyhow::Result<String> {
    let out = Command::new("git")
        .args(["rev-parse", rev])
        .current_dir(repo_path)
        .output()?;
    if !out.status.success() {
        anyhow::bail!(
            "git rev-parse {rev} failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

pub fn list_commits(repo_path: &Path, good: &str, bad: &str) -> anyhow::Result<Vec<String>> {
    let out = Command::new("git")
        .args(["rev-list", "--reverse", &format!("{good}..{bad}")])
        .current_dir(repo_path)
        .output()?;
    if !out.status.success() {
        anyhow::bail!(
            "git rev-list failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect())
}

#[derive(Debug, Clone)]
pub struct BisectResult {
    pub evaluations: usize,
    pub found: Option<String>,
}

pub async fn bisect<F, Fut>(
    repo_path: &Path,
    good: &str,
    bad: &str,
    mut predicate: F,
    ws: &crate::workspace::BisectWorkspace,
) -> anyhow::Result<BisectResult>
where
    F: FnMut(BisectState) -> Fut,
    Fut: std::future::Future<Output = crate::predicate::PredicateResult>,
{
    let commits = list_commits(repo_path, good, bad)?;
    if commits.is_empty() {
        return Ok(BisectResult {
            evaluations: 0,
            found: None,
        });
    }

    let mut low = 0usize;
    let mut high = commits.len();
    let mut evaluations = 0usize;
    let mut found = None;

    while low < high {
        let mid = low + (high - low) / 2;
        let commit = commits[mid].clone();
        evaluations += 1;
        ws.checkout(&commit)?;
        let result = predicate(BisectState {
            worktree_path: ws.worktree_path.clone(),
            commit: commit.clone(),
        })
        .await;
        match result {
            crate::predicate::PredicateResult::Good => low = mid + 1,
            crate::predicate::PredicateResult::Bad => {
                high = mid;
                found = Some(commit);
            }
            crate::predicate::PredicateResult::Inconclusive => low = mid + 1,
        }
    }

    Ok(BisectResult {
        evaluations,
        found,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::predicate::PredicateResult;
    use crate::workspace::BisectWorkspace;

    fn make_repo(dir: &Path, flags: &[&str]) -> anyhow::Result<(String, String)> {
        Command::new("git").args(["init"]).current_dir(dir).output()?;
        let mut shas = Vec::new();
        for (i, flag) in flags.iter().enumerate() {
            std::fs::write(dir.join("flag.txt"), format!("{i}:{flag}\n"))?;
            Command::new("git")
                .args(["add", "flag.txt"])
                .current_dir(dir)
                .output()?;
            let out = Command::new("git")
                .args([
                    "-c", "user.name=test", "-c", "user.email=test@example.com",
                    "commit", "-m", &format!("commit {i}"),
                ])
                .current_dir(dir)
                .output()?;
            if !out.status.success() {
                panic!(
                    "commit {i} failed exit={:?} out={:?} err={:?}",
                    out.status.code(),
                    String::from_utf8_lossy(&out.stdout),
                    String::from_utf8_lossy(&out.stderr),
                );
            }
            let sha = Command::new("git")
                .args(["rev-parse", "HEAD"])
                .current_dir(dir)
                .output()?;
            shas.push(String::from_utf8_lossy(&sha.stdout).trim().to_string());
        }
        Ok((shas[0].clone(), shas[shas.len() - 1].clone()))
    }

    // regression appears at the third of five commits; binary search must
    // land on it, not on a later sibling.
    #[tokio::test]
    async fn finds_first_bad_commit() -> anyhow::Result<()> {
        let dir = std::env::temp_dir().join(format!("runlens-bisect-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir)?;

        let (good, bad) = make_repo(&dir, &["ok", "ok", "regressed", "regressed", "regressed"])?;
        let ws = BisectWorkspace::new(&dir)?;

        let predicate = |_state: BisectState| async move {
            let flag = std::fs::read_to_string(&_state.worktree_path.join("flag.txt")).unwrap_or_default();
            if flag.contains("regressed") {
                PredicateResult::Bad
            } else {
                PredicateResult::Good
            }
        };

        let result = bisect(&dir, &good, &bad, predicate, &ws).await?;
        let expected = {
            let out = Command::new("git")
                .args(["rev-list", "--reverse", &format!("{good}..{bad}")])
                .current_dir(&dir)
                .output()?;
            String::from_utf8_lossy(&out.stdout)
                .lines()
                .map(|l| l.trim().to_string())
                .collect::<Vec<_>>()
        };
        assert_eq!(result.found.as_deref(), expected.get(1).map(|s| s.as_str()));
        assert!(result.evaluations <= 3, "expected a short search, got {}", result.evaluations);
        drop(ws);
        let _ = std::fs::remove_dir_all(&dir);
        Ok(())
    }
}
