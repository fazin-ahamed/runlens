use std::path::PathBuf;

use anyhow::Result;
use indexmap::IndexMap;
use runlens_recorder::session::{RecordingOptions, Session, TestAdapterKind};
use runlens_storage::Repository;

use crate::paths::WorkspacePaths;

pub async fn run(
    workspace: &WorkspacePaths,
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
        test_adapter: adapter,
        fail_on_findings,
    };
    let summary = Session::record(repo, opts).await?;
    println!(
        "{}",
        serde_json::to_string_pretty(&summary)
            .unwrap_or_else(|_| format!("{:?}", summary))
    );
    let code = match summary.state {
        runlens_core::model::SessionState::Complete => 0,
        _ => 1,
    };
    std::process::exit(code);
}
