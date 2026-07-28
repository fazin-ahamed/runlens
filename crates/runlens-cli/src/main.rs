use std::path::PathBuf;

use clap::{Parser, Subcommand};
use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;

mod commands;
mod output;
mod paths;

#[derive(Debug, Parser)]
#[command(
    name = "runlens",
    about = "Local-first developer flight recorder",
    version = env!("CARGO_PKG_VERSION"),
    long_about = None
)]
struct Cli {
    #[command(subcommand)]
    cmd: Commands,

    #[arg(long, global = true)]
    db: Option<PathBuf>,

    #[arg(long, global = true)]
    log_filter: Option<String>,
}

#[derive(Debug, Subcommand)]
enum Commands {
    #[command(about = "Initialize a RunLens store in the current project")]
    Init {
        #[arg(long)]
        force: bool,
    },
    #[command(about = "Record a new session")]
    Record {
        #[arg()]
        cwd: PathBuf,
        #[arg(long)]
        label: Option<String>,
        #[arg(long)]
        no_git: bool,
        #[arg(long)]
        no_env: bool,
        #[arg(long)]
        profiler: bool,
        #[arg(long, default_value_t = 1000)]
        profiler_interval_ms: u64,
        #[arg(long)]
        fail_on_findings: bool,
        #[arg(long)]
        test_adapter: Option<String>,
        #[arg(last = true)]
        command: Vec<String>,
    },
    #[command(about = "List recorded sessions")]
    List {
        #[arg(long, default_value_t = 10)]
        limit: u32,
        #[arg(long)]
        project: Option<String>,
        #[arg(long)]
        json: bool,
    },
    #[command(about = "Show events for a session")]
    Show {
        session_id: String,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        find: Option<String>,
        #[arg(long)]
        severity: Option<String>,
    },
    #[command(about = "Verify chain integrity for a session")]
    Verify {
        session_id: String,
        #[arg(long)]
        json: bool,
    },
    #[command(about = "Show redactions for a session")]
    Redactions {
        session_id: String,
        #[arg(long)]
        json: bool,
    },
    #[command(about = "Start MCP stdio server")]
    Mcp,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env()
        .map_err(|e| anyhow::anyhow!("invalid RUST_LOG filter: {e}"))?;
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let ws = paths::WorkspacePaths::from_opts(cli.db.as_deref())?;

    match cli.cmd {
        Commands::Init { force } => commands::init::run(&ws, force),
        Commands::Record {
            cwd,
            label,
            no_git,
            no_env,
            profiler,
            profiler_interval_ms,
            fail_on_findings,
            test_adapter,
            command,
        } => {
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(commands::record::run(
                &ws,
                cwd,
                label,
                !no_git,
                !no_env,
                profiler,
                profiler_interval_ms,
                fail_on_findings,
                test_adapter,
                command,
            ))
        }
        Commands::List {
            limit,
            project,
            json,
        } => commands::list::run(&ws, limit, project, json),
        Commands::Show {
            session_id,
            json,
            find,
            severity,
        } => commands::show::run(&ws, &session_id, json, find.as_deref(), severity.as_deref()),
        Commands::Verify { session_id, json } => commands::verify::run(&ws, &session_id, json),
        Commands::Redactions { session_id, json } => {
            commands::redactions::run(&ws, &session_id, json)
        }
        Commands::Mcp => commands::mcp::run(),
    }
}
