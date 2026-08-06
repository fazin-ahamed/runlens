use std::fmt;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RqlError {
    #[error("parse error at {position}: {message}")]
    Parse { position: usize, message: String },
    #[error("lex error at {position}: {message}")]
    Lex { position: usize, message: String },
    #[error("execution error: {0}")]
    Execution(String),
    #[error("unknown source table: {0}")]
    UnknownSource(String),
    #[error("unsupported feature: {0}")]
    Unsupported(String),
    #[error(transparent)]
    Db(#[from] rusqlite::Error),
}

impl RqlError {
    pub fn parse(pos: usize, msg: impl fmt::Display) -> Self {
        Self::Parse {
            position: pos,
            message: msg.to_string(),
        }
    }
    pub fn lex(pos: usize, msg: impl fmt::Display) -> Self {
        Self::Lex {
            position: pos,
            message: msg.to_string(),
        }
    }
    pub fn exec(msg: impl fmt::Display) -> Self {
        Self::Execution(msg.to_string())
    }
}
