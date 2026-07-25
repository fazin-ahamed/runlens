use std::io::Write;
use std::path::PathBuf;

use flate2::write::GzEncoder;
use flate2::Compression;
use tar::Builder;
use thiserror::Error;

use crate::manifest::{
    BundleManifest, ExporterInfo, FORMAT_VERSION, InvariantSection, ManifestProject,
    ManifestSession,
};
use runlens_core::chain;
use runlens_storage::Repository;

#[derive(Debug, Clone)]
pub struct ExportOptions {
    pub out_path: PathBuf,
    pub mask_root: Option<String>,
}

#[derive(Debug, Error)]
pub enum ExportError {
    #[error("session not found")]
    SessionNotFound,
    #[error("verify-after-export failed: {0}")]
    VerifyFailed(String),
    #[error("io: {0}")]
    Io(String),
    #[error("tar: {0}")]
    Tar(String),
}

impl From<std::io::Error> for ExportError {
    fn from(value: std::io::Error) -> Self {
        ExportError::Io(value.to_string())
    }
}

fn append_tar_file<W: Write>(
    tar: &mut Builder<W>,
    name: &str,
    contents: &[u8],
) -> std::io::Result<()> {
    let mut header = tar::Header::new_gnu();
    header.set_size(contents.len() as u64);
    header.set_entry_type(tar::EntryType::Regular);
    header.set_mode(0o644);
    header.set_cksum();
    tar.append_data(&mut header, name, contents)
}

pub fn export_session(
    repo: &Repository,
    session_id: &str,
    opts: ExportOptions,
) -> Result<BundleManifest, ExportError> {
    let session = repo
        .get_session(session_id)
        .map_err(|e| ExportError::Io(e.to_string()))?;

    let events = repo
        .list_events(session_id)
        .map_err(|e| ExportError::Io(e.to_string()))?;

    let project = repo
        .get_project(&session.project_id)
        .map_err(|e| ExportError::Io(e.to_string()))?
        .ok_or(ExportError::SessionNotFound)?;

    if let Err(e) = chain::verify_chain(&events) {
        return Err(ExportError::VerifyFailed(format!("{e:?}")));
    }

    let head_hash = events
        .last()
        .and_then(|e| e.current_hash.clone())
        .unwrap_or_default();

    let manifest = BundleManifest {
        format_version: FORMAT_VERSION.to_string(),
        exporter: ExporterInfo {
            name: "runlens".into(),
            version: env!("CARGO_PKG_VERSION").into(),
        },
        project: ManifestProject {
            project_id: session.project_id.clone(),
            name: project.name,
            root: project.root,
            language_hints: project.language_hints,
        },
        session: ManifestSession {
            session_id: session.session_id.clone(),
            state: session.state.to_string(),
            started_at: session.started_at.to_rfc3339(),
            stopped_at: session.stopped_at.map(|t| t.to_rfc3339()),
            command: session.command.clone(),
            args: session.args.clone(),
            labels: session.labels.clone(),
            source_event_count: session.source_event_count,
            imported: session.imported,
            bundle_origin: session.bundle_origin.clone(),
        },
        event_count: events.len() as u64,
        head_hash: Some(head_hash),
        invariants: InvariantSection {
            head_hash: None,
            event_count: Some(events.len() as u64),
            payload_hash: None,
        },
        redacted_event_count: 0,
    };

    let out_dir = opts
        .out_path
        .parent()
        .unwrap_or(&PathBuf::from("."))
        .to_path_buf();
    std::fs::create_dir_all(&out_dir).map_err(|e| ExportError::Io(e.to_string()))?;

    let file = std::fs::File::create(&opts.out_path)?;
    let encoder = GzEncoder::new(file, Compression::default());
    let mut tar = Builder::new(encoder);

    let manifest_bytes = toml::to_string_pretty(&manifest)
        .map_err(|e| ExportError::Io(e.to_string()))?;
    append_tar_file(&mut tar, "bundle.toml", manifest_bytes.as_bytes())?;

    let invariants = serde_json::to_string_pretty(&manifest.invariants)
        .map_err(|e| ExportError::Io(e.to_string()))?;
    append_tar_file(&mut tar, "invariants.json", invariants.as_bytes())?;

    for event in &events {
        let path = format!("events/{}.json", event.event_id);
        let body = serde_json::to_string(event).map_err(|e| ExportError::Io(e.to_string()))?;
        append_tar_file(&mut tar, &path, body.as_bytes())?;
    }

    tar.finish()?;
    Ok(manifest)
}