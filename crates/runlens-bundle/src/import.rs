use std::io::Read;
use std::path::{Path, PathBuf};

use flate2::read::GzDecoder;
use tar::Archive;
use thiserror::Error;

use crate::manifest::{is_compatible, BundleManifest};
use runlens_core::chain;
use runlens_storage::Repository;

#[derive(Debug, Clone)]
pub struct ImportOptions {
    pub extract_root: PathBuf,
    pub overwrite: bool,
    pub redaction_allowlist: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ImportReport {
    pub manifest: BundleManifest,
    pub events_imported: u64,
    pub events_skipped_chain_invalid: u64,
    pub artifacts_imported: u64,
    pub bytes_total: u64,
}

#[derive(Debug, Error)]
pub enum ImportError {
    #[error("io: {0}")]
    Io(String),
    #[error("tar: {0}")]
    Tar(String),
    #[error("unsupported bundle version: {0}; supported: {1:?}")]
    UnsupportedBundleVersion(String, Vec<String>),
    #[error("missing manifest")]
    MissingManifest,
    #[error("invariant mismatch: {0}")]
    InvariantMismatch(String),
    #[error("redaction policy: {0}")]
    RedactionPolicy(String),
}

impl From<std::io::Error> for ImportError {
    fn from(value: std::io::Error) -> Self {
        ImportError::Io(value.to_string())
    }
}

pub fn import_session(
    path: &Path,
    repo: &Repository,
    _opts: ImportOptions,
) -> Result<ImportReport, ImportError> {
    let file = std::fs::File::open(path)?;
    let decoder = GzDecoder::new(file);
    let mut archive = Archive::new(decoder);

    let mut manifest: Option<BundleManifest> = None;
    let mut events: Vec<runlens_core::model::Event> = Vec::new();
    let mut bytes_total: u64 = 0;
    let events_skipped: u64 = 0;

    archive
        .entries()
        .map_err(|e| ImportError::Tar(e.to_string()))?
        .filter_map(|tar_entry| tar_entry.ok())
        .for_each(|mut tar_entry| {
            let path = tar_entry.path().unwrap().to_string_lossy().into_owned();
            let size = tar_entry.size();
            bytes_total += size;

            let mut buf = Vec::with_capacity(size as usize);
            let _ = tar_entry.read_to_end(&mut buf);

            if path == "bundle.toml" {
                let content = String::from_utf8_lossy(&buf);
                let m: BundleManifest = toml::from_str(&content).unwrap();
                if !is_compatible(&m.format_version) {
                    panic!("unsupported version");
                }
                manifest = Some(m);
            } else if path.starts_with("events/") && path.ends_with(".json") {
                let event: runlens_core::model::Event =
                    serde_json::from_slice(&buf).unwrap();
                events.push(event);
            }
        });

    let manifest = manifest.ok_or(ImportError::MissingManifest)?;

    let invariants = &manifest.invariants;
    if let Some(ref expected) = invariants.event_count {
        if *expected != events.len() as u64 {
            return Err(ImportError::InvariantMismatch(format!(
                "event count: expected {expected}, got {}",
                events.len()
            )));
        }
    }

    events.sort_by_key(|e| e.sequence);

    if let Err(e) = chain::verify_chain(&events) {
        return Err(ImportError::InvariantMismatch(format!(
            "chain verification failed: {e:?}"
        )));
    }

    let mut events_imported: u64 = 0;
    for event in &events {
        repo.append_event(event)
            .map_err(|e| ImportError::Io(e.to_string()))?;
        events_imported += 1;
    }

    Ok(ImportReport {
        manifest,
        events_imported,
        events_skipped_chain_invalid: events_skipped,
        artifacts_imported: 0,
        bytes_total,
    })
}