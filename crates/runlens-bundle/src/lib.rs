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

pub use export::{export_session, ExportError, ExportOptions};
pub use import::{import_session, ImportError, ImportOptions, ImportReport};
pub use manifest::{
    BundleManifest, COMPATIBLE_VERSIONS, FORMAT_VERSION, ManifestProject, ManifestSession,
};
