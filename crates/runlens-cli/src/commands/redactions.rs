pub async fn run(
    workspace: &crate::paths::WorkspacePaths,
    session_id: &str,
    json: bool,
) -> anyhow::Result<()> {
    let repo = runlens_storage::Repository::open(&workspace.db_path)?;
    let findings = repo.list_redactions(session_id).await?;
    if json {
        crate::output::render_json(&findings)?;
    } else {
        let headers = ["finding_id", "session_id", "kind", "preview", "created_at"];
        let rows = findings
            .into_iter()
            .map(|f| {
                vec![
                    f.finding_id.to_string(),
                    f.session_id,
                    format!("{:?}", f.kind),
                    f.preview,
                    f.created_at,
                ]
            })
            .collect();
        crate::output::render_table(&headers, rows);
    }
    Ok(())
}
