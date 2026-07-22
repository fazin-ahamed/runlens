//! RunLens command-line interface.
//!
//! Subcommands:
//!  - `init` - create a fresh local database under .runlens
//!  - `record` - run a command under recording; the public launch path
//!  - `list` - print sessions for the current project
//!  - `show` - print details + event timeline for a session
//!  - `investigate` - run a command multiple times, gather divergence
//!  - `bug-report` - render a Markdown bug report from a session
//!  - `verify` - recompute and report the chain for a session
//!  - `compare` - explain divergences between two session_ids
//!  - `export` - write a `.runlens` archive for a session
//!  - `import` - read a `.runlens` archive into the local store
//!  - `redact` - interactive review / override of redaction findings
//!  - `roll` - rotate the local store to keep a bounded window
//!  - `mcp` - launch the MCP stdio or HTTP server
//!
//! The CLI is intentionally printable by humans and machines: the
//! default textual form of `list`, `show`, and `compare` is plain
//! tables; `--json` switches all subcommands to JSON for scripts.

#![forbid(unsafe_code)]
#![warn(rust_2018_idioms)]

#![allow(
    clippy::doc_markdown,
    clippy::str_to_string,
    clippy::option_if_let_else,
    clippy::used_underscore_binding,
    clippy::too_many_arguments,
    clippy::ptr_arg,
    clippy::needless_borrow,
    clippy::unused_async,
)]

use std::path::PathBuf;

use anyhow::Context;
use clap::{Parser, Subcommand};
use runlens_storage::Repository;
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

    /// Path to the local RunLens database. Defaults to ./runlens.sqlite
    #[arg(long, global = true)]
    db: Option<PathBuf>,

    /// Where the artifact store lives. Defaults to ./runlens-blobs
    #[arg(long, global = true)]
    blobs: Option<PathBuf>,

    /// Emit logs to stderr (RUST_LOG: `info` is default)
    #[arg(long, global = true)]
    log_filter: Option<String>,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Create a fresh .runlens store in the current working directory
    Init {
        #[arg(long, default_value = "false")]
        force: bool,
    },
    /// Run `<command> [args...]` under the recorder
    Record {
        /// Path to the working directory of the recorded command
        #[arg(long, default_value = ".")]
        cwd: PathBuf,
        /// Optional label applied to the session
        #[arg(long)]
        label: Option<String>,
        /// Disable git snapshot collection
        #[arg(long, default_value_t = false)]
        no_git: bool,
        /// Disable env fingerprint
        #[arg(long, default_value_t = false)]
        no_env: bool,
        /// Enable resource profiler
        #[arg(long, default_value_t = false)]
        profiler: bool,
        /// Profiler interval in milliseconds
        #[arg(long, default_value_t = 250)]
        profiler_interval_ms: u64,
        /// Force-fail on any redaction finding
        #[arg(long, default_value_t = false)]
        fail_on_findings: bool,
        /// Test adapter hint: auto|junit|pytest|vitest|gotest
        #[arg(long)]
        test_adapter: Option<String>,
        #[arg(required = true)]
        command: Vec<String>,
    },
    /// List recent sessions for the active project
    List {
        #[arg(long, default_value_t = 20)]
        limit: u32,
        #[arg(long)]
        project: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Pretty-print events for one session
    Show {
        session_id: String,
        #[arg(long, default_value_t = false)]
        json: bool,
        #[arg(long)]
        find: Option<String>,
        #[arg(long)]
        severity: Option<String>,
    },
    /// Verify the recorded hash chain against the database
    Verify {
        session_id: String,
        #[arg(long)]
        json: bool,
    },
    /// Diff two sessions, surfacing explainable divergences
    Compare {
        baseline: String,
        candidate: String,
        #[arg(long)]
        json: bool,
    },
    /// Generate a Markdown bug report for a session
    BugReport {
        session_id: String,
        #[arg(long, default_value = "stdout")]
        output: PathBuf,
        #[arg(long)]
        title: Option<String>,
    },
    /// Export a session to a `.runlens` archive
    Export {
        session_id: String,
        #[arg(long, default_value = "session.runlens")]
        out: PathBuf,
    },
    /// Import a `.runlens` archive into the local store
    Import {
        path: PathBuf,
        #[arg(long, default_value = "./runlens-extract")]
        extract_root: PathBuf,
        #[arg(long)]
        overwrite: bool,
        #[arg(long)]
        json: bool,
    },
    /// Re-run a command N times to gather. Used for flaky-test
    /// investigation.
    Investigate {
        #[arg(long, default_value_t = 5)]
        runs: u32,
        #[arg(long)]
        label: Option<String>,
        #[arg(required = true)]
        command: Vec<String>,
    },
    /// Rotate the local store: keep last N sessions, archive older ones
    Roll {
        #[arg(long, default_value_t = 50)]
        keep: u32,
        #[arg(long)]
        archive_dir: Option<PathBuf>,
        #[arg(long)]
        dry_run: bool,
    },
    /// Run the local MCP server (stdio by default)
    Mcp {
        /// Bind a loopback HTTP server on this port instead of stdio
        #[arg(long)]
        http_port: Option<u16>,
    },
    /// Print path of the active database
    Where,
    /// Show known redaction findings
    Redactions {
        session_id: String,
        #[arg(long)]
        json: bool,
    },
    /// Start the HTTP/WS record-and-replay proxy
    Proxy(commands::proxy::ProxyArgs),
    /// Create, restore, and manage workspace checkpoints
    Checkpoint(commands::checkpoint::CheckpointArgs),
}

fn main() {
    let cli = Cli::parse();
    init_tracing(cli.log_filter.as_deref());
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .worker_threads(2)
        .build()
        .expect("tokio runtime");
    if let Err(e) = runtime.block_on(run(cli)) {
        eprintln!("runlens: {e:?}");
        std::process::exit(1);
    }
}

fn init_tracing(filter: Option<&str>) {
    let user_filter = filter.unwrap_or("info,runlens=info");
    let filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::WARN.into())
        .parse_lossy(user_filter);
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .try_init();
}

async fn run(cli: Cli) -> anyhow::Result<()> {
    let workspace = paths::WorkspacePaths::resolve(cli.db.as_deref(), cli.blobs.as_deref());
    let res: anyhow::Result<()> = match cli.cmd {
        Commands::Init { force } => commands::init::run(&workspace, force).await,
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
            commands::record::run(
                &workspace,
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
            .await
        }
        Commands::List { limit, project, json } => {
            commands::list::run(&workspace, limit, project, json).await
        }
        Commands::Show { session_id, json, find, severity } => {
            commands::show::run(&workspace, &session_id, find.as_deref(), severity.as_deref(), json).await
        }
        Commands::Verify { session_id, json } => commands::verify::run(&workspace, &session_id, json).await,
        Commands::Compare { baseline, candidate, json } => {
            commands::compare::run(&workspace, &baseline, &candidate, json).await
        }
        Commands::BugReport { session_id, output, title } => {
            commands::bug_report::run(&workspace, &session_id, &output, title.as_deref()).await
        }
        Commands::Export { session_id, out } => commands::export::run(&workspace, &session_id, &out).await,
        Commands::Import { path, extract_root, overwrite, json } => {
            commands::import::run(&workspace, &path, &extract_root, overwrite, json).await
        }
        Commands::Investigate { runs, label, command } => {
            commands::investigate::run(&workspace, runs, label, command).await
        }
        Commands::Roll { keep, archive_dir, dry_run } => {
            commands::roll::run(&workspace, keep, archive_dir, dry_run).await
        }
        Commands::Mcp { http_port } => commands::mcp::run(&workspace, http_port).await,
        Commands::Where => {
            println!("{}", workspace.db_path.display());
            Ok(())
        }
        Commands::Redactions { session_id, json } => {
            commands::redactions::run(&workspace, &session_id, json).await
        }
        Commands::Proxy(args) => commands::proxy::run(&args, &workspace).await,
        Commands::Checkpoint(args) => commands::checkpoint::run(&args, &workspace).await,
    };
    res.context("command failed")
}

#[allow(dead_code)]
fn _repo_for(cli: &Cli) -> anyhow::Result<Repository> {
    let workspace = paths::WorkspacePaths::resolve(cli.db.as_deref(), cli.blobs.as_deref());
    Repository::open(&workspace.db_path).context("open db")
}
