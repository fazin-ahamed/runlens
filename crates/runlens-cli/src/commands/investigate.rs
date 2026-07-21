use indexmap::IndexMap;
use runlens_core::compare::compare_sessions;
use runlens_recorder::session::{RecordingOptions, Session, TestAdapterKind};
use runlens_storage::Repository;

pub async fn run(
    workspace: &crate::paths::WorkspacePaths,
    runs: u32,
    label: Option<String>,
    command: Vec<String>,
) -> anyhow::Result<()> {
    if command.is_empty() {
        anyhow::bail!("no command supplied");
    }
    let repo = Repository::open(&workspace.db_path)?;
    let mut iter = command.into_iter();
    let cmd = iter.next().unwrap();
    let args: Vec<String> = iter.collect();

    let mut summaries = Vec::new();
    let mut all_first_events = Vec::new();
    let mut all_last_events = Vec::new();

    for n in 1..=runs {
        let opts = RecordingOptions {
            command: cmd.clone(),
            args: args.clone(),
            working_dir: std::env::current_dir().unwrap_or_default(),
            env: IndexMap::new(),
            label: Some(format!("{}-{}", label.clone().unwrap_or_else(|| "run".into()), n)),
            watch_paths: vec![],
            enable_profiler: false,
            enable_git: false,
            enable_env: false,
            profiler_interval_ms: 0,
            test_adapter: Some(TestAdapterKind::Pytest),
            fail_on_findings: false,
        };
        let summary = Session::record(repo.clone(), opts).await?;
        let evs = repo.list_events(&summary.session_id).await?;
        summaries.push(summary);
        all_first_events.push(evs.first().cloned().unwrap());
        all_last_events.push(evs.last().cloned().unwrap());
    }

    println!("flaky-test investigation: {} runs", runs);
    println!("successes: {}", summaries.iter().filter(|s| s.is_success()).count());
    println!("failures:  {}", summaries.iter().filter(|s| !s.is_success()).count());
    if all_last_events.len() >= 2 {
        let baseline = vec![all_last_events[0].clone()];
        let candidate = all_last_events[1..].to_vec();
        let cmp = compare_sessions(&baseline, &candidate);
        println!("explainable divergences (last event of run #1 vs others):");
        for d in &cmp.divergences {
            println!("  [{:?}] {}", d.severity, d.title);
            println!("    {}", d.summary);
        }
    }
    Ok(())
}
