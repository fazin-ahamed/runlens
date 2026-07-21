use crate::paths::WorkspacePaths;
use anyhow::Result;
use runlens_storage::Repository;

pub async fn run(workspace: &WorkspacePaths, force: bool) -> Result<()> {
    std::fs::create_dir_all(&workspace.root)?;
    std::fs::create_dir_all(&workspace.blobs_dir)?;
    if workspace.db_path.exists() && !force {
        anyhow::bail!(
            "database already exists at {} (use --force to reset)",
            workspace.db_path.display()
        );
    }
    let _repo = Repository::open(&workspace.db_path)?;
    let count = match _repo.list_recent_sessions(5).await {
        Ok(v) => v.len(),
        Err(_) => 0,
    };
    println!(
        "Initialised RunLens at {}\n  db:     {}\n  blobs:  {}\n  sessions so far: {count}",
        workspace.root.display(),
        workspace.db_path.display(),
        workspace.blobs_dir.display()
    );
    Ok(())
}
