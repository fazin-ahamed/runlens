use std::path::PathBuf;

use clap::{Parser, Subcommand};
use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;

mod commands;
mod output;
mod paths;

use commands::bisect::BisectArgs;
use commands::checkpoint::CheckpointArgs;
use commands::ci::CiArgs;
use commands::dap::DapArgs;
use commands::db::DbArgs;
use commands::diagnose::DiagnoseArgs;
use commands::doctor::DoctorArgs;
use commands::graph::GraphArgs;
use commands::hypothesis::HypothesisArgs;
use commands::matrix::MatrixArgs;
use commands::minimize::MinimizeArgs;
use commands::proxy::ProxyArgs;
use commands::query::QueryArgs;
use commands::regression::RegressionArgs;
use commands::replay::ReplayArgs;

fn run_async<F, Fut>(f: F) -> anyhow::Result<()>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = anyhow::Result<()>>,
{
    tokio::runtime::Runtime::new()?.block_on(f())
}

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
    #[command(about = "Export a session to a bundle")]
    Export {
        session_id: String,
        #[arg()]
        out: PathBuf,
    },
    #[command(about = "Import a session bundle")]
    Import {
        #[arg()]
        path: PathBuf,
        #[arg(long)]
        extract_root: Option<PathBuf>,
        #[arg(long)]
        overwrite: bool,
        #[arg(long)]
        json: bool,
    },
    #[command(about = "Run git bisect over a commit range")]
    Bisect {
        #[arg(long, default_value = "HEAD~10")]
        good: String,
        #[arg(long, default_value = "HEAD")]
        bad: String,
        #[arg(last = true)]
        command: Vec<String>,
        #[arg(long)]
        repo: Option<PathBuf>,
        #[arg(long)]
        resume: bool,
    },
    #[command(about = "Compare two sessions")]
    Compare {
        baseline: String,
        candidate: String,
        #[arg(long)]
        json: bool,
    },
    #[command(about = "Inspect session graphs")]
    Graph(GraphArgs),
    #[command(about = "Run a session query")]
    Query(QueryArgs),
    #[command(about = "Manage session checkpoints")]
    Checkpoint(CheckpointArgs),
    #[command(about = "Roll and archive old sessions")]
    Roll {
        #[arg()]
        keep: u32,
        #[arg(long)]
        archive: Option<PathBuf>,
        #[arg(long)]
        dry_run: bool,
    },
    #[command(about = "Diagnose a failing bug report")]
    Diagnose(DiagnoseArgs),
    #[command(about = "Investigate over repeated runs")]
    Investigate {
        #[arg(short, long, default_value_t = 3)]
        runs: u32,
        #[arg(long)]
        label: Option<String>,
        #[arg(last = true)]
        command: Vec<String>,
    },
    #[command(about = "Run a doctor scan")]
    Doctor(DoctorArgs),
    #[command(about = "Gather a bug report")]
    BugReport {
        session_id: String,
        #[arg()]
        dest: PathBuf,
    },
    #[command(about = "Run a hypothesis matrix")]
    Matrix(MatrixArgs),
    #[command(about = "Manage the proxy")]
    Proxy(ProxyArgs),
    #[command(about = "Manage a bug hypothesis")]
    Hypothesis(HypothesisArgs),
    #[command(about = "Manage the memory database")]
    Db(DbArgs),
    #[command(about = "Run a debugging session")]
    Dap(DapArgs),
    #[command(about = "Run a CI command")]
    Ci(CiArgs),
    #[command(about = "Minimize a failing input")]
    Minimize(MinimizeArgs),
    #[command(about = "Detect regression between sessions")]
    Regression(RegressionArgs),
    #[command(about = "Replay captures")]
    Replay(ReplayArgs),
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
        } => run_async(|| {
            commands::record::run(
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
            )
        }),
        Commands::List { limit, project, json } => commands::list::run(&ws, limit, project, json),
        Commands::Show {
            session_id,
            json,
            find,
            severity,
        } => commands::show::run(&ws, &session_id, json, find.as_deref(), severity.as_deref()),
        Commands::Verify { session_id, json } => commands::verify::run(&ws, &session_id, json),
        Commands::Redactions { session_id, json } => commands::redactions::run(&ws, &session_id, json),
        Commands::Mcp => commands::mcp::run(),
        Commands::Export { session_id, out } => commands::export::run(&ws, &session_id, &out),
        Commands::Import {
            path,
            extract_root,
            overwrite,
            json,
        } => commands::import::run(&ws, &path, &extract_root.unwrap_or_default(), overwrite, json),
        Commands::Bisect {
            good,
            bad,
            command,
            repo,
            resume,
        } => {
            let args = BisectArgs {
                good,
                bad,
                command,
                repo,
                resume,
            };
            run_async(|| commands::bisect::run(&args, &ws))
        },
        Commands::Compare {
            baseline,
            candidate,
            json,
        } => run_async(|| commands::compare::run(&ws, &baseline, &candidate, json)),
        Commands::Graph(args) => run_async(|| commands::graph::run(&args, &ws)),
        Commands::Query(args) => run_async(|| commands::query::run(&args, &ws)),
        Commands::Checkpoint(args) => run_async(|| commands::checkpoint::run(&args, &ws)),
        Commands::Roll { keep, archive, dry_run } => run_async(|| commands::roll::run(&ws, keep, archive, dry_run)),
        Commands::Diagnose(args) => run_async(|| commands::diagnose::run(&ws, &args)),
        Commands::Investigate { runs, label, command } => {
            run_async(|| commands::investigate::run(&ws, runs, label, command))
        },
        Commands::Doctor(args) => run_async(|| commands::doctor::run(&ws, &args)),
        Commands::BugReport { session_id, dest } => {
            run_async(|| commands::bug_report::run(&ws, &session_id, &dest, None))
        },
        Commands::Matrix(args) => run_async(|| commands::matrix::run(&ws, &args)),
        Commands::Proxy(args) => run_async(|| commands::proxy::run(&args, &ws)),
        Commands::Hypothesis(args) => run_async(|| commands::hypothesis::run(&ws, &args)),
        Commands::Db(args) => run_async(|| commands::db::run(&args, &ws)),
        Commands::Dap(args) => run_async(|| commands::dap::run(&ws, &args)),
        Commands::Ci(args) => run_async(|| commands::ci::run(&ws, &args)),
        Commands::Minimize(args) => run_async(|| commands::minimize::run(&args, &ws)),
        Commands::Regression(args) => run_async(|| commands::regression::run(&ws, &args)),
        Commands::Replay(args) => run_async(|| commands::replay::run(&ws, &args)),
    }
}
