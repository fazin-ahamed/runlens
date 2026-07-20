pub mod error;
pub mod migrations;
pub mod repo;

pub use error::StorageError;
pub use repo::Repository;

#[derive(Debug, Clone)]
pub struct DiskArtifacts;

impl DiskArtifacts {
    pub fn open<P: AsRef<std::path::Path>>(_path: P) -> Result<Self, StorageError> {
        Ok(Self)
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EventRecord {
    pub session_id: String,
    pub event_id: String,
    pub sequence: u64,
    pub kind: String,
    pub payload_json: String,
    pub timestamp_ns: i64,
    pub hash: String,
}
