use clap::Parser;
use std::path::PathBuf;

#[derive(Debug, Parser)]
pub struct BisectArgs {
    #[arg(long, default_value = "HEAD~10")]
    pub good: String,
    #[arg(long, default_value = "HEAD")]
    pub bad: String,
    pub command: Vec<String>,
    #[arg(long)]
    pub repo: Option<PathBuf>,
    #[arg(long)]
    pub resume: bool,
}

pub async fn run(args: &BisectArgs, _workspace: &crate::paths::WorkspacePaths) -> anyhow::Result<()> {
    let repo_path = args.repo.clone().unwrap_or_else(|| std::env::current_dir().unwrap());
    let project_root = &repo_path;

    let ws = runlens_bisect::workspace::BisectWorkspace::new(&repo_path)?;

    let good = runlens_bisect::engine::run_git_rev_parse(&repo_path, &args.good)?;
    let bad = runlens_bisect::engine::run_git_rev_parse(&repo_path, &args.bad)?;
    println!("Bisecting: good={good} bad={bad}");

    let mut cache = runlens_bisect::cache::BisectCache::new();
    if args.resume {
        if let Some(progress) = runlens_bisect::progress::load_progress(project_root.as_path())? {
            cache = progress.cache;
            if let (Some(saved_good), Some(saved_bad)) = (cache.known_good(), cache.known_bad()) {
                if saved_good == good || saved_bad == bad {
                    println!("Resuming from saved progress ({} cached results)", cache.len());
                }
            }
        }
    }

    let bisect_predicate = |state: runlens_bisect::engine::BisectState| async move {
        let cmd_pred = runlens_bisect::predicate::BisectPredicate::new(args.command.clone());
        match cmd_pred.run_async(&state.worktree_path).await {
            Ok(r) => r,
            Err(e) => {
                eprintln!("warning: predicate error: {e}");
                runlens_bisect::predicate::PredicateResult::Inconclusive
            },
        }
    };

    let outcome = runlens_bisect::engine::bisect(&repo_path, &good, &bad, bisect_predicate, &ws).await?;

    let progress = runlens_bisect::progress::BisectProgress {
        good: good.clone(),
        bad: bad.clone(),
        evaluations: outcome.evaluations,
        cache,
    };
    runlens_bisect::progress::save_progress(project_root.as_path(), &progress)?;

    let report = runlens_bisect::report::generate_report(&outcome);
    println!("{report}");

    Ok(())
}
