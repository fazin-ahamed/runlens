use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct WorkspacePaths {
    pub root: PathBuf,
    pub db_path: PathBuf,
    pub blobs_dir: PathBuf,
}

impl WorkspacePaths {
    pub fn from_opts(db: Option<&Path>) -> anyhow::Result<Self> {
        let cwd = std::env::current_dir()?;
        let root = cwd.join(".runlens");
        let db_path = db.map(PathBuf::from).unwrap_or_else(|| root.join("runlens.sqlite"));
        let blobs_dir = root.join("blobs");
        Ok(WorkspacePaths {
            root,
            db_path,
            blobs_dir,
        })
    }

    pub fn ensure_root(&self) -> anyhow::Result<()> {
        std::fs::create_dir_all(&self.root)?;
        std::fs::create_dir_all(&self.blobs_dir)?;
        Ok(())
    }
}
