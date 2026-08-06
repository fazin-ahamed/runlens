use clap::Parser;

#[derive(Debug, Parser)]
pub struct MinimizeArgs {
    pub session_id: String,
    #[arg(long, default_value = "files")]
    pub dimension: String,
    #[arg(long)]
    pub resume: bool,
}

pub async fn run(args: &MinimizeArgs, workspace: &crate::paths::WorkspacePaths) -> anyhow::Result<()> {
    let session_id = &args.session_id;

    let repo = runlens_storage::repo::Repository::open(&workspace.db_path)?;
    let session = repo
        .get_session(session_id)
        .map_err(|e| anyhow::anyhow!("session lookup failed: {e}"))?;

    let cwd = std::env::current_dir()?;
    let files: Vec<String> = std::fs::read_dir(&cwd)?
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();

    if files.is_empty() {
        anyhow::bail!("no files found in current directory");
    }

    let command = session.command.clone().unwrap_or_default();
    let predicate = runlens_minimize::predicate::Predicate::new(if command.is_empty() {
        vec!["true".into()]
    } else {
        command.split_whitespace().map(String::from).collect()
    });

    println!("Minimizing {} items (dimension: {})...", files.len(), args.dimension);
    let outcome = runlens_minimize::engine::minimize(files, |_subset| {
        let rt = tokio::runtime::Handle::current();
        let delta_dir = std::path::Path::new(".");
        rt.block_on(async { predicate.run(delta_dir).await })
    })
    .await;

    let explain = runlens_minimize::explain::MinimizeResult {
        delta: outcome.delta.clone(),
        evaluations: outcome.evaluations,
        steps: outcome.steps.clone(),
    };
    println!(
        "{}",
        runlens_minimize::explain::format_explanation(&explain, &args.dimension)
    );

    Ok(())
}
