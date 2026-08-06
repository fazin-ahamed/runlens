use std::path::{Path, PathBuf};
use std::process::Command;

pub struct BisectWorkspace {
    pub worktree_path: PathBuf,
    repo_path: PathBuf,
}

impl BisectWorkspace {
    pub fn new(repo_path: &Path) -> anyhow::Result<Self> {
        let base = std::env::temp_dir().join(format!("runlens-bisect-{}", std::process::id()));
        let _ = Command::new("git")
            .args(["worktree", "remove", "--force"])
            .arg(&base)
            .current_dir(repo_path)
            .output();
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base)?;

        let out = Command::new("git")
            .args(["worktree", "add", "--detach"])
            .arg(&base)
            .arg("HEAD")
            .current_dir(repo_path)
            .output()?;
        if !out.status.success() {
            anyhow::bail!(
                "git worktree add failed: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            );
        }
        Ok(Self {
            worktree_path: base,
            repo_path: repo_path.to_path_buf(),
        })
    }

    pub fn checkout(&self, commit: &str) -> anyhow::Result<()> {
        let out = Command::new("git")
            .args(["checkout", "--detach", commit])
            .current_dir(&self.worktree_path)
            .output()?;
        if !out.status.success() {
            anyhow::bail!(
                "git checkout {commit} failed: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            );
        }
        Ok(())
    }
}

impl Drop for BisectWorkspace {
    fn drop(&mut self) {
        let _ = Command::new("git")
            .args(["worktree", "remove", "--force"])
            .arg(&self.worktree_path)
            .current_dir(&self.repo_path)
            .output();
        let _ = std::fs::remove_dir_all(&self.worktree_path);
    }
}
