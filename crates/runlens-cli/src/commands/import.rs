use std::path::PathBuf;

pub async fn run(
    workspace: &crate::paths::WorkspacePaths,
    path: &PathBuf,
    extract_root: &PathBuf,
    overwrite: bool,
    json: bool,
) -> anyhow::Result<()> {
    let repo = runlens_storage::Repository::open(&workspace.db_path)?;
    let report = runlens_bundle::import_bundle(
        &repo,
        path,
        runlens_bundle::ImportOptions {
            extract_root: extract_root.clone(),
            overwrite,
            redaction_allowlist: vec![],
        },
    )
    .await?;
    if json {
        crate::output::render_json(&report)?;
    } else {
        println!(
            "imported {} events from {}\n  bundle_format: {}\n  events: {}\n  artifacts: {}\n",
            report.events_imported,
            path.display(),
            report.manifest.bundle_format_version,
            report.events_imported,
            report.artifacts_imported
        );
    }
    Ok(())
}
