use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
pub struct CheckpointArgs {
    #[command(subcommand)]
    pub action: CheckpointAction,
}

#[derive(Debug, Subcommand)]
pub enum CheckpointAction {
    Create {
        #[arg(long, default_value = ".")]
        root: std::path::PathBuf,
        #[arg(long)]
        message: Option<String>,
        #[arg(long)]
        include_secrets: bool,
    },
    Restore {
        checkpoint_id: String,
        #[arg(long, default_value = "./runlens-restore")]
        output: std::path::PathBuf,
    },
    List {
        #[arg(default_value = "latest")]
        session_id: String,
    },
    Gc {
        #[arg(long)]
        dry_run: bool,
    },
}

pub async fn run(
    args: &CheckpointArgs,
    workspace: &crate::paths::WorkspacePaths,
) -> anyhow::Result<()> {
    match &args.action {
        CheckpointAction::Create { root, message, include_secrets } => {
            let store = runlens_workspace::object_store::ObjectStore::open(&workspace.blobs_dir)?;
            let mut snapshot = runlens_workspace::snapshot::Snapshot::new(store);
            let manifest = snapshot.create(root, message.clone(), *include_secrets)?;
            let root_hash = manifest.root_hash();
            let db_path = workspace.db_path.to_string_lossy().to_string();
            persist_checkpoint(&db_path, &manifest, &root_hash)?;
            let size: u64 = manifest.files.iter().map(|f| f.size).sum();
            println!("Created checkpoint {} ({} files, {} bytes)", root_hash, manifest.files.len(), size);
            Ok(())
        }
        CheckpointAction::Restore { checkpoint_id, output } => {
            let db_path = workspace.db_path.to_string_lossy().to_string();
            let (manifest_json, root_hash) = load_checkpoint(&db_path, checkpoint_id)?
                .ok_or_else(|| anyhow::anyhow!("checkpoint not found: {checkpoint_id}"))?;
            let manifest: runlens_workspace::manifest::SnapshotManifest = serde_json::from_str(&manifest_json)?;
            let restorer = runlens_workspace::restore::Restorer::new();
            restorer.restore(&manifest, &workspace.blobs_dir, output)?;
            println!("Restored checkpoint {} to {}", root_hash, output.display());
            Ok(())
        }
        CheckpointAction::List { session_id } => {
            let db_path = workspace.db_path.to_string_lossy().to_string();
            let conn = rusqlite::Connection::open(&db_path)?;
            let mut stmt = conn.prepare(
                "SELECT checkpoint_id, message, created_at, file_count, total_size, git_head
                 FROM checkpoints
                 WHERE session_id = ?1
                 ORDER BY created_at DESC
                 LIMIT 50"
            )?;
            let rows = stmt.query_map(rusqlite::params![session_id], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, i64>(3)?,
                    r.get::<_, i64>(4)?,
                    r.get::<_, Option<String>>(5)?,
                ))
            })?;
            for row in rows {
                let (id, msg, created, files, size, git) = row?;
                let git_info = git.as_deref().unwrap_or("");
                println!("{id:20} {created}  {files:>5} files  {size:>10} bytes  {git_info:20}  {msg}");
            }
            Ok(())
        }
        CheckpointAction::Gc { dry_run } => {
            let store = runlens_workspace::object_store::ObjectStore::open(&workspace.blobs_dir)?;
            let db_path = workspace.db_path.to_string_lossy().to_string();
            let conn = rusqlite::Connection::open(&db_path)?;
            let mut stmt = conn.prepare("SELECT manifest_hash FROM checkpoints")?;
            let live_hashes: Vec<String> = stmt.query_map([], |r| r.get::<_, String>(0))?
                .filter_map(|r| r.ok())
                .collect();
            let mut referenced = std::collections::HashSet::new();
            for hash in &live_hashes {
                let manifest_path = workspace.blobs_dir.join("manifests").join(format!("{hash}.json"));
                if let Ok(chunks_json) = std::fs::read_to_string(&manifest_path) {
                    if let Ok(manifest) = serde_json::from_str::<runlens_workspace::manifest::SnapshotManifest>(&chunks_json) {
                        for f in &manifest.files {
                            for ch in &f.chunk_hashes {
                                referenced.insert(ch.clone());
                            }
                        }
                    }
                }
            }
            let deleted = runlens_workspace::gc::run_gc(&store, &referenced)?;
            if *dry_run {
                println!("Would delete {} orphan chunks", deleted.len());
            } else {
                println!("Deleted {} orphan chunks", deleted.len());
            }
            Ok(())
        }
    }
}

fn persist_checkpoint(db_path: &str, manifest: &runlens_workspace::manifest::SnapshotManifest, root_hash: &str) -> anyhow::Result<()> {
    let conn = rusqlite::Connection::open(db_path)?;
    runlens_storage::migrations::run(&conn)?;
    let total_size: i64 = manifest.files.iter().map(|f| f.size as i64).sum();
    conn.execute(
        "INSERT INTO checkpoints(checkpoint_id, session_id, manifest_hash, root_hash, message, file_count, total_size, git_head, has_uncommitted)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        rusqlite::params![
            root_hash,
            "workspace",
            root_hash,
            root_hash,
            manifest.message.as_deref().unwrap_or(""),
            manifest.files.len() as i64,
            total_size,
            manifest.git_head.as_deref(),
            0i64,
        ],
    )?;
    Ok(())
}

fn load_checkpoint(db_path: &str, checkpoint_id: &str) -> anyhow::Result<Option<(String, String)>> {
    let conn = rusqlite::Connection::open(db_path)?;
    let mut stmt = conn.prepare(
        "SELECT manifest_hash, root_hash FROM checkpoints WHERE checkpoint_id = ?1"
    )?;
    let mut rows = stmt.query(rusqlite::params![checkpoint_id])?;
    match rows.next()? {
        Some(r) => Ok(Some((r.get::<_, String>(0)?, r.get::<_, String>(1)?))),
        None => Ok(None),
    }
}
