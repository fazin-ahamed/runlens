#![forbid(unsafe_code)]
#![warn(rust_2018_idioms)]
#![allow(
    clippy::doc_markdown,
    clippy::str_to_string,
    clippy::missing_const_for_fn,
    clippy::option_if_let_else,
    clippy::used_underscore_binding,
    clippy::inefficient_to_string,
    clippy::clone_on_copy,
    clippy::default_constructed_unit_structs,
    clippy::string_lit_as_bytes
)]

pub mod dispatch;
pub mod env_fingerprint;
pub mod file_watcher;
pub mod git;
pub mod profiler;
pub mod pty;
pub mod redaction;
pub mod session;
pub mod test_adapters;

pub use session::{RecordingOptions, SessionSummary};
