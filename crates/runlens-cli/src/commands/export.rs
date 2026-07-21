use std::path::Path;

pub async fn run(
    workspace: &crate::paths::WorkspacePaths,
    session_id: &str,
    out: &Path,
) -> anyhow::Result<()> {
    let repo = runlens_storage::Repository::open(&workspace.db_path)?;
    let blobs = runlens_storage::DiskArtifacts::open(&workspace.blobs_dir)?;
    let _manifest = runlens_bundle::export_session(
        &repo,
        session_id,
        &blobs,
        runlens_bundle::ExportOptions {
            out_path: out.to_path_buf(),
            mask_root: Some("~".into()),
        },
    )
    .await?;
    println!("exported {session_id} -> {}", out.display());
    Ok(())
}
