//! RunLens recorder crate.
//!
//! Collectors, session orchestration, and redaction pipeline glue connecting
//! the [`runlens_core`] domain types with the [`runlens_storage`] repositories.
//!
//! Sub-modules:
//!
//! - [`pty`] - cross-platform PTY child command execution that captures
//!   output, exit status, wall-clock duration, and well-known signal codes.
//! - [`file_watcher`] - debounced notify-based file-system watcher that
//!   produces diff-friendly events for the session log.
//! - [`git`] - read-only git fingerprint capture via shell-out: HEAD,
//!   branch, dirty paths, lockfile hashes. Avoids depending on libgit2
//!   while still giving rich provenance.
//! - [`env_fingerprint`] - privacy-safe environment snapshot, allow-listed
//!   keys, BLAKE3-hashed so it can detect drift without leaking values.
//! - [`profiler`] - lightweight wall-clock resource sampler (RSS, page-fault
//!   counters, process count) using /proc on Unix and a polling shell on
//!   Windows. No ties to libproc/libprocps.
//! - [`test_adapters`] - parses JUnit XML, pytest fixture output, and
//!   vitest stdout into a uniform list of test_results events.
//! - [`redaction`] - bridges [`runlens_core::privacy`] with event payloads,
//!   applies redactions in-place, and records findings.
//! - [`session`] - the public surface for starting/stopping sessions,
//!   driving multiple collectors concurrently.
//! - [`dispatch`] - the in-process pub/sub for collectors -> storage.

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
    clippy::string_lit_as_bytes,
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

pub use session::{RecordingOptions, SessionHandle, SessionSummary};
