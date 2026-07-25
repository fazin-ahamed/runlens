use thiserror::Error;

#[derive(Debug, Error)]
pub enum DbError {
    #[error("SQL parse error: {0}")]
    Parse(String),
    #[error("Storage error: {0}")]
    Storage(#[from] anyhow::Error),
    #[error("Session not found: {0}")]
    SessionNotFound(String),
}
