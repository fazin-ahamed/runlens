use clap::Parser;

#[derive(Debug, Parser)]
pub struct DbArgs {
    #[command(subcommand)]
    pub action: DbAction,
}

#[derive(Debug, clap::Subcommand)]
pub enum DbAction {
    Analyze {
        session_id: String,
        #[arg(long, default_value = "3")]
        n_plus_one_threshold: usize,
        #[arg(long, default_value = "100")]
        slow_ms: i64,
        #[arg(long)]
        json: bool,
    },
    Report {
        session_id: String,
        #[arg(long, default_value = "3")]
        n_plus_one_threshold: usize,
        #[arg(long, default_value = "100")]
        slow_ms: i64,
        #[arg(long)]
        json: bool,
    },
}

pub async fn run(args: &DbArgs, workspace: &crate::paths::WorkspacePaths) -> anyhow::Result<()> {
    let repo = runlens_storage::Repository::open(&workspace.db_path)?;

    match &args.action {
        DbAction::Analyze {
            session_id,
            n_plus_one_threshold,
            slow_ms,
            json,
        }
        | DbAction::Report {
            session_id,
            n_plus_one_threshold,
            slow_ms,
            json,
        } => {
            let analysis = runlens_db::analyze_session(&repo, session_id, *n_plus_one_threshold, *slow_ms * 1_000_000)
                .map_err(|e| anyhow::anyhow!("analysis failed: {:?}", e))?;
            if *json {
                println!("{}", runlens_db::report::to_json(&analysis));
            } else {
                print!("{}", runlens_db::report::to_text(&analysis));
            }
        },
    }
    Ok(())
}
