pub fn run(
    workspace: &crate::paths::WorkspacePaths,
    limit: u32,
    _project: Option<String>,
    json: bool,
) -> anyhow::Result<()> {
    let repo = runlens_storage::Repository::open(&workspace.db_path)?;
    let entries = repo.list_recent_sessions(limit as usize)?;
    if json {
        crate::output::render_json(&entries)?;
        return Ok(());
    }
    let headers = ["session_id", "project_id", "state", "started_at", "events", "command"];
    let rows = entries
        .into_iter()
        .map(|s| {
            vec![
                s.session_id,
                s.project_id,
                s.state.to_string(),
                s.started_at.to_rfc3339(),
                s.source_event_count.to_string(),
                s.command.unwrap_or_default(),
            ]
        })
        .collect();
    crate::output::render_table(&headers, rows);
    Ok(())
}
