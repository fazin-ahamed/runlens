//! Workspace path resolution.

use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct WorkspacePaths {
    pub root: PathBuf,
    pub db_path: PathBuf,
    pub blobs_dir: PathBuf,
}

impl WorkspacePaths {
    /// Resolve the workspace layout for the current run.
    ///
    /// Root is `.runlens/` unless `--db` is provided in which case the
    /// parent of the db file is the workspace. We prefer a personal
    /// directory per project to keep cache and DB colocated with the
    /// actual code being recorded.
    pub fn resolve(db: Option<&Path>, blobs: Option<&Path>) -> Self {
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let default_root = cwd.join(".runlens");
        let root = default_root.clone();
        let db_path = db
            .map(PathBuf::from)
            .unwrap_or_else(|| root.join("runlens.sqlite"));
        let blobs_dir = blobs
            .map(PathBuf::from)
            .unwrap_or_else(|| root.join("blobs"));
        WorkspacePaths { root, db_path, blobs_dir }
    }
}
