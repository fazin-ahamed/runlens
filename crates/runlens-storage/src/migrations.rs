use rusqlite::Connection;

use crate::error::{StorageError, StorageResult};

const MIGRATION_SQL: &str = include_str!("migration.sql");

pub fn run(conn: &Connection) -> StorageResult<()> {
    let current: i64 = conn
        .query_row("SELECT COALESCE(MAX(version), 0) FROM schema_version", [], |r| r.get(0))
        .unwrap_or(0);

    if current < 1 {
        conn.execute_batch(MIGRATION_SQL)
            .map_err(|e| StorageError::Migration(e.to_string()))?;
    }

    Ok(())
}
