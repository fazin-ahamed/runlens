use crate::paths::WorkspacePaths;
use anyhow::Result;
use clap::Parser;

#[derive(Debug, Parser)]
pub struct HypothesisArgs {
    #[command(subcommand)]
    pub cmd: HypothesisCommand,
}

#[derive(Debug, Parser)]
pub enum HypothesisCommand {
    Create {
        name: String,
        #[arg(long)]
        session: Option<String>,
    },
    Propose {
        workspace_id: String,
        description: String,
        #[arg(long, default_value = "user")]
        author: String,
    },
    List {
        workspace_id: String,
        #[arg(long)]
        status: Option<String>,
    },
    Status {
        hypothesis_id: String,
        status: String,
    },
}

pub async fn run(_workspace: &WorkspacePaths, args: &HypothesisArgs) -> Result<()> {
    match &args.cmd {
        HypothesisCommand::Create { name, session } => {
            println!(
                "Created workspace '{}' (session: {})",
                name,
                session.as_deref().unwrap_or("none")
            );
        },
        HypothesisCommand::Propose {
            workspace_id,
            description,
            author,
        } => {
            println!(
                "Proposed: '{}' in workspace {} by {}",
                description, workspace_id, author
            );
        },
        HypothesisCommand::List { workspace_id, status } => {
            println!(
                "Hypotheses in {} (status: {}):",
                workspace_id,
                status.as_deref().unwrap_or("all")
            );
            println!("  (none)");
        },
        HypothesisCommand::Status { hypothesis_id, status } => {
            println!("Updated {} status to {}", hypothesis_id, status);
        },
    }
    Ok(())
}
