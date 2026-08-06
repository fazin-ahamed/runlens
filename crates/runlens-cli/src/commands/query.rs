use clap::Parser;

#[derive(Debug, Parser)]
pub struct QueryArgs {
    #[command(subcommand)]
    pub action: QueryAction,
}

#[derive(Debug, clap::Subcommand)]
pub enum QueryAction {
    Run {
        rql: String,
        #[arg(long)]
        json: bool,
    },
    Explain {
        rql: String,
    },
}

pub async fn run(args: &QueryArgs, workspace: &crate::paths::WorkspacePaths) -> anyhow::Result<()> {
    let repo = runlens_storage::Repository::open(&workspace.db_path)?;
    let conn_guard = repo.conn().lock().unwrap();
    let conn: &rusqlite::Connection = &conn_guard;

    match &args.action {
        QueryAction::Run { rql, json } => {
            let rows = runlens_query::run_query(conn, rql)?;
            if *json {
                println!("{}", serde_json::to_string_pretty(&rows)?);
            } else if rows.is_empty() {
                println!("(no results)");
            } else {
                println!("{} result(s):", rows.len());
                for row in &rows {
                    println!("  {row}");
                }
            }
        },
        QueryAction::Explain { rql } => {
            let plan = runlens_query::run_explain(conn, rql)?;
            for row in &plan {
                println!("{row}");
            }
        },
    }
    Ok(())
}
