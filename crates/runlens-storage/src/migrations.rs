//! Embedded migration runner. Each migration applies once in order.

use anyhow::{Context, Result};
use rusqlite::Connection;
use std::sync::atomic::{AtomicU64, Ordering};

static SCHEMA_VERSION: AtomicU64 = AtomicU64::new(0);

const MIGRATIONS: &[(&str, &str)] = &[
    (
        "0001_initial",
        include_str!("migrations/0001_initial.sql"),
    ),
    (
        "0002_trace_causal_spans",
        include_str!("migrations/0002_trace_causal_spans.sql"),
    ),
    (
        "0003_proxy",
        include_str!("migrations/0003_proxy.sql"),
    ),
    (
        "0004_browser_tabs",
        include_str!("migrations/0004_browser_tabs.sql"),
    ),
    (
        "0005_checkpoints",
        include_str!("migrations/0005_checkpoints.sql"),
    ),
];

/// Run all pending migrations. Records the resulting version in
/// `schema_version`.
pub fn run(conn: &Connection) -> Result<u64> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_version (
            version INTEGER PRIMARY KEY,
            applied_at TEXT NOT NULL DEFAULT (datetime('now'))
        );",
    )
    .context("creating schema_version table")?;
    let current: u64 = conn
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_version",
            [],
            |r| Ok(r.get::<_, i64>(0)? as u64),
        )
        .context("reading schema_version")?;
    for (name, sql) in MIGRATIONS {
        let n = migration_number(name).context("parsing migration name")?;
        if n <= current {
            continue;
        }
        let tx = conn.unchecked_transaction()?;
        tx.execute_batch(sql)
            .with_context(|| format!("applying migration {name}"))?;
        tx.execute(
            "INSERT INTO schema_version(version) VALUES (?1)",
            rusqlite::params![n as i64],
        )?;
        tx.commit()?;
    }
    let v = conn
        .query_row(
            "SELECT MAX(version) FROM schema_version",
            [],
            |r| Ok(r.get::<_, i64>(0)? as u64),
        )?;
    SCHEMA_VERSION.store(v, Ordering::SeqCst);
    Ok(v)
}

pub fn current_version() -> u64 {
    SCHEMA_VERSION.load(Ordering::SeqCst)
}

fn migration_number(name: &str) -> Result<u64> {
    name.split('_')
        .next()
        .and_then(|s| s.parse::<u64>().ok())
        .with_context(|| format!("invalid migration name: {name}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn migration_runs_idempotently() {
        let conn = Connection::open_in_memory().unwrap();
        run(&conn).unwrap();
        let v1 = run(&conn).unwrap();
        assert!(v1 >= 1);
        let v2 = run(&conn).unwrap();
        assert_eq!(v1, v2);
    }
}
