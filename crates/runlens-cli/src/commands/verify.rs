pub async fn run(
    workspace: &crate::paths::WorkspacePaths,
    session_id: &str,
    json: bool,
) -> anyhow::Result<()> {
    let repo = runlens_storage::Repository::open(&workspace.db_path)?;
    let events = repo.list_events(session_id).await?;
    let report = match runlens_core::chain::verify_chain(&events) {
        Ok(()) => serde_json::json!({
            "session_id": session_id,
            "status": "ok",
            "events": events.len(),
        }),
        Err(e) => serde_json::json!({
            "session_id": session_id,
            "status": "fail",
            "errors": format!("{:?}", e),
        }),
    };
    if json { crate::output::render_json(&report)?; }
    else {
        let s = report.get("status").and_then(|v| v.as_str()).unwrap_or("");
        let n = report.get("events").and_then(|v| v.as_u64()).unwrap_or(0);
        println!("verify {session_id}\n  status: {s}\n  events: {n}");
        if s == "fail" {
            if let Some(errs) = report.get("errors") {
                eprintln!("  failure: {errs}");
            }
            std::process::exit(2);
        }
    }
    Ok(())
}
