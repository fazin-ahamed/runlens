use std::path::Path;

pub fn run(
    workspace: &crate::paths::WorkspacePaths,
    path: &Path,
    extract_root: &Path,
    overwrite: bool,
    json: bool,
) -> anyhow::Result<()> {
    let repo = runlens_storage::Repository::open(&workspace.db_path)?;
    let report = runlens_bundle::import_session(
        path,
        &repo,
        runlens_bundle::ImportOptions {
            extract_root: extract_root.to_path_buf(),
            overwrite,
            redaction_allowlist: vec![],
        },
    )?;
    if json {
        crate::output::render_json(&report)?;
    } else {
        println!(
            "imported {} events from {}\n  format: {}\n  events: {}\n  artifacts: {}\n",
            report.events_imported,
            path.display(),
            report.manifest.format_version,
            report.events_imported,
            report.artifacts_imported
        );
    }
    Ok(())
}
