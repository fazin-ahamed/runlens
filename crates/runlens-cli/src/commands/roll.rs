use std::path::PathBuf;

pub async fn run(
    workspace: &crate::paths::WorkspacePaths,
    keep: u32,
    archive_dir: Option<PathBuf>,
    dry_run: bool,
) -> anyhow::Result<()> {
    let repo = runlens_storage::Repository::open(&workspace.db_path)?;
    let recent = repo.list_recent_sessions(keep + 200).await?;
    let to_drop: Vec<_> = recent.iter().skip(keep as usize).cloned().collect();
    println!(
        "rotation plan (dry_run={dry_run})\n  keep last: {keep}\n  sessions to archive: {}\n  archive_dir: {}",
        to_drop.len(),
        archive_dir
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "(none)".to_string())
    );
    for s in &to_drop {
        println!("  would-archive  {} ({})", s.session_id, s.state.as_str());
    }
    if dry_run || archive_dir.is_none() {
        return Ok(());
    }
    let archive_dir = archive_dir.unwrap();
    std::fs::create_dir_all(&archive_dir)?;
    let blobs = runlens_storage::DiskArtifacts::open(&workspace.blobs_dir)?;
    let mut archived = 0u32;
    for s in &to_drop {
        let out = archive_dir.join(format!("{}.runlens", s.session_id));
        if out.exists() {
            continue;
        }
        if runlens_bundle::export_session(
            &repo,
            &s.session_id,
            &blobs,
            runlens_bundle::ExportOptions {
                out_path: out.clone(),
                mask_root: Some("~".into()),
            },
        )
        .await
        .is_ok()
        {
            archived += 1;
        }
    }
    println!("archived {archived} sessions to {}", archive_dir.display());
    Ok(())
}
