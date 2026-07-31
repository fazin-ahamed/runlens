use crate::paths::WorkspacePaths;
use anyhow::Result;
use clap::Parser;

#[derive(Debug, Parser)]
pub struct DapArgs {
    #[command(subcommand)]
    pub cmd: DapCommand,
}

#[derive(Debug, Parser)]
pub enum DapCommand {
    Start {
        adapter: String,
        program: String,
        #[arg(long)]
        attach: bool,
        #[arg(long)]
        port: Option<u16>,
    },
    Snapshot {
        session_id: String,
        #[arg(long)]
        label: Option<String>,
    },
    Compare {
        session_id: String,
        snapshot_a: String,
        snapshot_b: String,
    },
}

pub async fn run(_workspace: &WorkspacePaths, args: &DapArgs) -> Result<()> {
    match &args.cmd {
        DapCommand::Start { adapter, program, attach, port } => {
            let mode = if *attach { "attach" } else { "launch" };
            println!("DAP {} {} on {} (port={})", mode, adapter, program, port.unwrap_or(0));
        }
        DapCommand::Snapshot { session_id, label } => {
            println!("Snapshot taken for session {} ({})", session_id, label.as_deref().unwrap_or("unlabeled"));
        }
        DapCommand::Compare { session_id, snapshot_a, snapshot_b } => {
            println!("Comparing snapshots {} vs {} in session {}", snapshot_a, snapshot_b, session_id);
        }
    }
    Ok(())
}
