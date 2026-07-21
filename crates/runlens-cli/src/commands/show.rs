pub async fn run(
    workspace: &crate::paths::WorkspacePaths,
    session_id: &str,
    find: Option<&str>,
    severity: Option<&str>,
    json: bool,
) -> anyhow::Result<()> {
    let repo = runlens_storage::Repository::open(&workspace.db_path)?;
    let events = repo.list_events(session_id).await?;
    let filtered: Vec<_> = events
        .into_iter()
        .filter(|e| find.map(|f| e.kind.contains(f) || e.payload.to_string().contains(f)).unwrap_or(true))
        .filter(|e| severity.map(|s| e.severity.as_str().eq_ignore_ascii_case(s)).unwrap_or(true))
        .collect();
    if json {
        crate::output::render_json(&filtered)?;
        return Ok(());
    }
    let headers = ["seq", "kind", "severity", "ts", "hash"];
    let rows = filtered
        .iter()
        .map(|e| {
            vec![
                e.sequence.to_string(),
                e.kind.clone(),
                e.severity.as_str().to_string(),
                e.utc_timestamp.to_rfc3339(),
                e.current_hash.clone().unwrap_or_default().chars().take(16).collect(),
            ]
        })
        .collect();
    crate::output::render_table(&headers, rows);
    Ok(())
}
