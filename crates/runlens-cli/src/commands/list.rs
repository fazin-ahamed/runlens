pub async fn run(
    workspace: &crate::paths::WorkspacePaths,
    limit: u32,
    project: Option<String>,
    json: bool,
) -> anyhow::Result<()> {
    let repo = runlens_storage::Repository::open(&workspace.db_path)?;
    let entries = repo.list_recent_sessions(limit).await?;
    if json {
        crate::output::render_json(&entries)?;
        return Ok(());
    }
    let project_display = project.unwrap_or_else(|| {
        std::env::current_dir()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default()
    });
    let headers = ["session_id", "project_id", "state", "started_at", "events", "command"];
    let rows = entries
        .into_iter()
        .map(|s| {
            vec![
                s.session_id,
                s.project_id,
                s.state.as_str().to_string(),
                s.started_at.to_rfc3339(),
                s.source_event_count.to_string(),
                s.command.unwrap_or_else(|| project_display.clone()),
            ]
        })
        .collect();
    crate::output::render_table(&headers, rows);
    Ok(())
}
