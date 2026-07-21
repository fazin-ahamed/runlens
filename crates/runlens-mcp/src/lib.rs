//! RunLens MCP server.
//!
//! Two transport modes:
//!  - **stdio** for direct integration with Claude Code / Continue.dev
//!  - **HTTP** (loopback-only) for browser-based tools and manual probes
//!
//! The protocol implemented is the Model Context Protocol JSON-RPC over
//! the transport. Tools exposed are intentionally read-only / safe —
//! they only walk stored sessions and return structured information.
//! No tool here deletes data or invokes the recorder.

#![forbid(unsafe_code)]
#![warn(rust_2018_idioms)]

#![allow(
    clippy::doc_markdown,
    clippy::str_to_string,
    clippy::missing_const_for_fn,
)]

use serde::{Deserialize, Serialize};

pub mod http;
pub mod stdio_mode;
pub mod tools;

pub mod run {
    pub use crate::stdio_mode::run as stdio;
    pub use crate::http::run as http;
}

pub use crate::run::{stdio, http};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: &'static str,
    pub description: &'static str,
    pub input_schema: serde_json::Value,
}
