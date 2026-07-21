use crate::paths::WorkspacePaths;

pub async fn run(workspace: &WorkspacePaths, port: Option<u16>) -> anyhow::Result<()> {
    let repo = runlens_storage::Repository::open(&workspace.db_path)?;
    match port {
        None => {
            runlens_mcp::run::stdio(repo).await?;
        }
        Some(p) => {
            runlens_mcp::run::http(repo, p).await?;
        }
    }
    Ok(())
}
