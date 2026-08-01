#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SnapshotFile {
    pub path: String,
    pub size: u64,
    pub chunk_hashes: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SnapshotManifest {
    pub root_hash: String,
    pub files: Vec<SnapshotFile>,
    pub total_size: u64,
    pub message: Option<String>,
    pub git_head: Option<String>,
}

impl SnapshotManifest {
    pub fn root_hash(&self) -> &str {
        &self.root_hash
    }
}
