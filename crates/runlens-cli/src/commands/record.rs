use std::path::PathBuf;

use anyhow::Result;
use indexmap::IndexMap;
use runlens_recorder::session::{RecordingOptions, Session, TestAdapterKind};
use runlens_storage::Repository;

#[allow(clippy::too_many_arguments)]
pub async fn run(
    workspace: &crate::paths::WorkspacePaths,
    cwd: PathBuf,
    label: Option<String>,
    enable_git: bool,
    enable_env: bool,
    enable_profiler: bool,
    profiler_interval_ms: u64,
    fail_on_findings: bool,
    test_adapter: Option<String>,
    command: Vec<String>,
) -> Result<()> {
    if command.is_empty() {
        anyhow::bail!("no command supplied");
    }
    std::fs::create_dir_all(&workspace.root)?;
    let repo = Repository::open(&workspace.db_path)?;

    let mut iter = command.into_iter();
    let cmd = iter.next().expect("checked above");
    let args: Vec<String> = iter.collect();
    let adapter = match test_adapter.as_deref() {
        None | Some("auto") | Some("") => None,
        Some("pytest") => Some(TestAdapterKind::Pytest),
        Some("junit") => Some(TestAdapterKind::Junit),
        Some("vitest") => Some(TestAdapterKind::Vitest),
        Some("gotest") => Some(TestAdapterKind::Gotest),
        Some(other) => anyhow::bail!("unknown test-adapter {:?}", other),
    };

    let opts = RecordingOptions {
        command: cmd.clone(),
        args,
        working_dir: cwd,
        env: IndexMap::new(),
        label,
        watch_paths: vec![],
        enable_profiler,
        enable_git,
        enable_env,
        profiler_interval_ms,
        test_adapter_hint: adapter,
        fail_on_findings,
        max_events: None,
    };
    let session = Session::new(repo, opts);
    let summary = session.record().await?;
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "session_id": summary.session_id,
            "project_id": summary.project_id,
            "state": summary.state.to_string(),
            "exit_code": summary.exit_code,
            "events": summary.source_event_count,
            "redactions": summary.redaction_findings_total,
        }))?
    );
    Ok(())
}
