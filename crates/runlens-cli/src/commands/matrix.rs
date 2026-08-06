use crate::paths::WorkspacePaths;
use anyhow::Result;
use clap::Parser;

#[derive(Debug, Parser)]
pub struct MatrixArgs {
    #[command(subcommand)]
    pub cmd: MatrixCommand,
}

#[derive(Debug, Parser)]
pub enum MatrixCommand {
    Define {
        name: String,
        #[arg(long)]
        file: Option<String>,
    },
    Run {
        matrix_id: String,
        #[arg(long, default_value_t = 2)]
        parallel: u32,
    },
    Show {
        matrix_id: String,
        #[arg(long)]
        json: bool,
    },
    List {
        #[arg(long)]
        json: bool,
    },
}

pub async fn run(_workspace: &WorkspacePaths, args: &MatrixArgs) -> Result<()> {
    match &args.cmd {
        MatrixCommand::Define { name, file } => {
            println!("Defined matrix '{}' from {}", name, file.as_deref().unwrap_or("inline"));
        },
        MatrixCommand::Run { matrix_id, parallel } => {
            println!("Running matrix {} with {} parallel workers", matrix_id, parallel);
        },
        MatrixCommand::Show { matrix_id, json } => {
            if *json {
                println!("{{\"matrix_id\": \"{}\", \"combinations\": []}}", matrix_id);
            } else {
                println!("Matrix: {}", matrix_id);
                println!("  Status: Not run");
            }
        },
        MatrixCommand::List { json } => {
            if *json {
                println!("[]");
            } else {
                println!("No matrices defined");
            }
        },
    }
    Ok(())
}
