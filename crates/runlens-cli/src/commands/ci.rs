use crate::paths::WorkspacePaths;
use anyhow::Result;
use clap::Parser;

#[derive(Debug, Parser)]
pub struct CiArgs {
    #[command(subcommand)]
    pub cmd: CiCommand,
}

#[derive(Debug, Parser)]
pub enum CiCommand {
    Run {
        #[arg(long)]
        command: Vec<String>,
        #[arg(long)]
        fail_on: Option<String>,
    },
    Summary {
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        json: bool,
    },
}

pub async fn run(_workspace: &WorkspacePaths, args: &CiArgs) -> Result<()> {
    match &args.cmd {
        CiCommand::Run { command, fail_on } => {
            let env = runlens_ci::CiEnvironment::detect();
            println!("Running in CI environment: {}", env.as_str());
            if !command.is_empty() {
                println!("  Command: {}", command.join(" "));
            }
            if let Some(f) = fail_on {
                println!("  Fail on: {}", f);
            }
        },
        CiCommand::Summary { title, json } => {
            let summary = runlens_ci::CiJobSummary {
                title: title.clone().unwrap_or_else(|| "RunLens Report".into()),
                status: runlens_ci::CiJobStatus::Passed,
                metrics: vec![],
                regressions: vec![],
                artifacts: vec![],
                duration_secs: 0,
            };
            if *json {
                println!("{}", summary.to_json());
            } else {
                println!("{}", summary.to_github_markdown());
            }
        },
    }
    Ok(())
}
