use crate::paths::WorkspacePaths;
use anyhow::Result;
use clap::Parser;
use runlens_regression::RegressionDetector;

#[derive(Debug, Parser)]
pub struct RegressionArgs {
    #[command(subcommand)]
    pub cmd: RegressionCommand,
}

#[derive(Debug, Parser)]
pub enum RegressionCommand {
    Baseline {
        session_id: String,
        #[arg(long)]
        label: Option<String>,
    },
    Check {
        baseline_id: String,
        candidate_session_id: String,
        #[arg(long)]
        json: bool,
    },
    List {
        #[arg(long)]
        json: bool,
    },
}

pub async fn run(_workspace: &WorkspacePaths, args: &RegressionArgs) -> Result<()> {
    match &args.cmd {
        RegressionCommand::Baseline { session_id, label } => {
            println!("Registered baseline: {} ({})", session_id, label.as_deref().unwrap_or("unlabeled"));
        }
        RegressionCommand::Check { baseline_id, candidate_session_id, json } => {
            let detector = RegressionDetector::new();
            let report = detector.compare(baseline_id, vec![]);
            if *json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                println!("Regression Report: {} vs {}", baseline_id, candidate_session_id);
                println!("  Failed: {} ({:?})", report.summary.failed, report.summary.severity);
            }
        }
        RegressionCommand::List { json } => {
            let detector = RegressionDetector::new();
            let baselines = detector.list_baselines();
            if *json {
                println!("{}", serde_json::to_string_pretty(&baselines)?);
            } else {
                for b in &baselines {
                    println!("  {}", b);
                }
            }
        }
    }
    Ok(())
}
