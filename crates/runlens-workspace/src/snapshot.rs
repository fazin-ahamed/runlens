use std::path::Path;

pub struct Snapshot;

impl Snapshot {
    pub fn new(_store: crate::object_store::ObjectStore) -> Self {
        Self
    }

    pub fn create(
        &self,
        _root: &Path,
        message: Option<String>,
        _include_secrets: bool,
    ) -> anyhow::Result<crate::manifest::SnapshotManifest> {
        Ok(crate::manifest::SnapshotManifest {
            root_hash: "stub".to_string(),
            files: vec![],
            total_size: 0,
            message,
            git_head: None,
        })
    }
}
