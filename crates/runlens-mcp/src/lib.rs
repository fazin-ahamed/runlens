#![forbid(unsafe_code)]
#![warn(rust_2018_idioms)]
#![allow(clippy::doc_markdown, clippy::str_to_string, clippy::missing_const_for_fn)]

use serde::{Deserialize, Serialize};

pub mod http;
pub mod stdio_mode;
pub mod tools;

pub mod run {
    pub use crate::http::run as http;
    pub use crate::stdio_mode::run as stdio;
}

pub use crate::run::{http, stdio};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: &'static str,
    pub description: &'static str,
    pub input_schema: serde_json::Value,
}
