pub fn run(workspace: &crate::paths::WorkspacePaths, session_id: &str, json: bool) -> anyhow::Result<()> {
    let repo = runlens_storage::Repository::open(&workspace.db_path)?;
    let findings = repo.list_redactions(session_id)?;
    if json {
        crate::output::render_json(&findings)?;
    } else {
        let headers = ["kind", "preview"];
        let rows = findings
            .iter()
            .map(|f| vec![f.kind.clone(), f.preview.clone()])
            .collect();
        crate::output::render_table(&headers, rows);
    }
    Ok(())
}
