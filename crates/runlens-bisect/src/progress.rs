use std::path::{Path, PathBuf};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BisectProgress {
    pub good: String,
    pub bad: String,
    pub evaluations: usize,
    pub cache: crate::cache::BisectCache,
}

fn progress_path(project_root: &Path) -> PathBuf {
    project_root.join(".runlens").join("bisect.json")
}

pub fn load_progress(project_root: &Path) -> anyhow::Result<Option<BisectProgress>> {
    let path = progress_path(project_root);
    if !path.exists() {
        return Ok(None);
    }
    let bytes = std::fs::read(&path)?;
    let progress = serde_json::from_slice(&bytes)?;
    Ok(Some(progress))
}

pub fn save_progress(project_root: &Path, progress: &BisectProgress) -> anyhow::Result<()> {
    let dir = progress_path(project_root);
    if let Some(parent) = dir.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let bytes = serde_json::to_vec_pretty(progress)?;
    std::fs::write(dir, bytes)?;
    Ok(())
}
