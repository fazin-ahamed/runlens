use std::path::Path;

pub async fn run(
    workspace: &crate::paths::WorkspacePaths,
    session_id: &str,
    output: &Path,
    title: Option<&str>,
) -> anyhow::Result<()> {
    let repo = runlens_storage::Repository::open(&workspace.db_path)?;
    let events = repo.list_events(session_id).await?;
    let session = repo.get_session(session_id).await?.unwrap_or_else(|| {
        runlens_core::model::SessionInfo {
            session_id: session_id.into(),
            project_id: String::new(),
            state: runlens_core::model::SessionState::Complete,
            started_at: chrono::Utc::now(),
            stopped_at: None,
            command: None,
            args: vec![],
            labels: vec![],
            source_event_count: 0,
            imported: false,
            bundle_origin: None,
        }
    });

    let mut md = String::new();
    md.push_str(&format!(
        "# {}\n\n",
        title.unwrap_or("RunLens bug report")
    ));
    md.push_str(&format!(
        "- session: `{}`\n- project: `{}`\n- state: `{}`\n- events: `{}`\n",
        session.session_id,
        session.project_id,
        session.state.as_str(),
        session.source_event_count
    ));
    md.push_str(&format!(
        "- started: {}\n",
        session.started_at.to_rfc3339()
    ));
    if let Some(cmd) = &session.command {
        md.push_str(&format!("- command: `{}`\n", cmd));
    }
    if !session.labels.is_empty() {
        md.push_str(&format!("- labels: {:?}\n", session.labels));
    }
    md.push_str("\n## First flagged event\n\n");
    let error = events.iter().find(|e| e.is_error_like());
    match error {
        Some(e) => {
            md.push_str(&format!(
                "- sequence: `{}`\n- kind: `{}`\n- severity: `{}`\n- ts: {}\n\n",
                e.sequence,
                e.kind,
                e.severity.as_str(),
                e.utc_timestamp.to_rfc3339()
            ));
            md.push_str("```json\n");
            md.push_str(&serde_json::to_string_pretty(&e.payload).unwrap_or_default());
            md.push_str("\n```\n");
        }
        None => md.push_str("_no error-shaped events in this session_\n"),
    }
    md.push_str("\n## Last 20 events\n\n");
    let tail: Vec<_> = events.iter().rev().take(20).collect();
    md.push_str("| seq | kind | sev | hash (16) |\n|---|---|---|---|\n");
    for e in tail.iter().rev() {
        md.push_str(&format!(
            "| {} | {} | {} | {} |\n",
            e.sequence,
            e.kind,
            e.severity.as_str(),
            e.current_hash.clone().unwrap_or_default().chars().take(16).collect::<String>()
        ));
    }
    if output == std::path::Path::new("stdout") {
        println!("{md}");
    } else {
        std::fs::write(output, &md)?;
        println!("wrote bug report to {}", output.display());
    }
    Ok(())
}
