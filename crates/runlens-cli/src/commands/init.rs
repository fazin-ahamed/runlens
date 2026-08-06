use runlens_storage::Repository;

pub fn run(ws: &crate::paths::WorkspacePaths, force: bool) -> anyhow::Result<()> {
    if ws.db_path.exists() && !force {
        anyhow::bail!(
            "RunLens store already exists at {}. Use --force to re-create.",
            ws.db_path.display()
        );
    }
    ws.ensure_root()?;
    if force {
        reset_store(&ws.db_path)?;
    }
    Repository::open(&ws.db_path)?;
    println!("RunLens store initialized at {}", ws.db_path.display());
    Ok(())
}

fn reset_store(db_path: &std::path::Path) -> anyhow::Result<()> {
    for path in [
        db_path.to_path_buf(),
        db_path.with_extension("sqlite-wal"),
        db_path.with_extension("sqlite-shm"),
    ] {
        if path.exists() {
            std::fs::remove_file(path)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::reset_store;

    #[test]
    fn reset_store_removes_sqlite_and_sidecars() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("runlens.sqlite");
        std::fs::write(&db, b"stale").unwrap();
        std::fs::write(db.with_extension("sqlite-wal"), b"wal").unwrap();
        std::fs::write(db.with_extension("sqlite-shm"), b"shm").unwrap();

        reset_store(&db).unwrap();

        assert!(!db.exists());
        assert!(!db.with_extension("sqlite-wal").exists());
        assert!(!db.with_extension("sqlite-shm").exists());
    }
}
