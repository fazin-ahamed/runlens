//! SQLite storage for RunLens. Pure repository logic; no protocol layer.

#![forbid(unsafe_code)]
#![warn(rust_2018_idioms)]

#![allow(
    clippy::doc_markdown,
    clippy::cast_lossless,
    clippy::redundant_closure,
    clippy::str_to_string,
)]

pub mod migrations;
pub mod repo;
pub mod stores;

pub use repo::Repository;
pub use stores::DiskArtifacts;
