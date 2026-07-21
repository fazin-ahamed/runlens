//! RunLens bundle format.
//!
//! A `.runlens` file is a deterministic tar archive with a strict layout:
//!
//! ```text
//! bundle.toml              # manifest, version+1
//! invariants.json          # the canonical hash chain hashes (NEW + OLD)
//! events-0.jsonl           # one JSON Event {"kind":"..."} per line
//! events-1.jsonl           # optional chunked continuation
//! artifacts/<hash>.bin     # content-addressed blobs (zero-padded)
//! ```
//!
//! Bundles are gzipped (`.runlens` ⇒ gzip-compressed tar). On import,
//! RunLens refuses to consume any bundle missing `bundle.toml` or whose
//! declared version is not in a known-compatible set, refuses any
//! extracted file whose absolute path escapes the import destination,
//! and refuses any event whose hash doesn't match invariants.json.

#![forbid(unsafe_code)]
#![warn(rust_2018_idioms)]

#![allow(
    clippy::doc_markdown,
    clippy::str_to_string,
    clippy::missing_const_for_fn,
    clippy::inefficient_to_string,
    clippy::unused_async,
    clippy::manual_contains,
)]

pub mod export;
pub mod import;
pub mod manifest;

pub use export::{ExportError, ExportOptions, export_session};
pub use import::{ImportError, ImportOptions, ImportReport, import_bundle};
pub use manifest::{
    ArtifactRef, BundleManifest, COMPATIBLE_VERSIONS, FORMAT_VERSION, ManifestProject,
    ManifestSession,
};
