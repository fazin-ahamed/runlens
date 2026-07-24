use rusqlite::Connection;

fn seed_db() -> Connection {
    let mut conn = Connection::open_in_memory().unwrap();
    let tx = conn.transaction().unwrap();

    tx.execute_batch(
        "CREATE TABLE IF NOT EXISTS projects (
            project_id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            root TEXT NOT NULL,
            language_hints TEXT NOT NULL DEFAULT '[]',
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
        CREATE TABLE IF NOT EXISTS sessions (
            session_id TEXT PRIMARY KEY,
            project_id TEXT NOT NULL,
            state TEXT NOT NULL,
            started_at TEXT NOT NULL,
            stopped_at TEXT,
            command TEXT,
            args TEXT NOT NULL DEFAULT '[]',
            labels TEXT NOT NULL DEFAULT '[]',
            source_event_count INTEGER NOT NULL DEFAULT 0,
            imported INTEGER NOT NULL DEFAULT 0,
            bundle_origin TEXT
        );
        CREATE TABLE IF NOT EXISTS events (
            event_id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL,
            project_id TEXT NOT NULL,
            sequence INTEGER NOT NULL,
            source_kind TEXT NOT NULL,
            source_value TEXT NOT NULL,
            kind TEXT NOT NULL,
            severity TEXT NOT NULL,
            utc_timestamp TEXT NOT NULL,
            monotonic_ns INTEGER NOT NULL,
            duration_ns INTEGER,
            correlation_id TEXT,
            parent_event_id TEXT,
            payload_version INTEGER NOT NULL,
            payload_json TEXT NOT NULL,
            classification TEXT NOT NULL,
            previous_hash TEXT,
            current_hash TEXT,
            is_error_like INTEGER NOT NULL DEFAULT 0,
            FOREIGN KEY (session_id) REFERENCES sessions(session_id),
            UNIQUE (session_id, sequence)
        );",
    ).unwrap();

    tx.execute(
        "INSERT INTO sessions(session_id, project_id, state, started_at, args, source_event_count)
         VALUES ('s1', 'p1', 'complete', '2025-01-01T00:00:00Z', '[]', 5)",
        [],
    ).unwrap();

    let events = vec![
        ("e1", "network.response", "info", "2025-01-01T00:00:00Z", 0, 1, "OK"),
        ("e2", "network.response", "info", "2025-01-01T00:00:05Z", 0, 2, "OK"),
        ("e3", "error", "error", "2025-01-01T00:00:10Z", 1, 3, "timeout"),
        ("e4", "marker", "info", "2025-01-01T00:00:15Z", 0, 4, "checkpoint"),
        ("e5", "network.response", "error", "2025-01-01T00:00:20Z", 1, 5, "server_error"),
    ];

    for (id, kind, severity, ts, error, seq, payload) in &events {
        tx.execute(
            "INSERT INTO events(event_id, session_id, project_id, sequence, source_kind, source_value,
             kind, severity, utc_timestamp, monotonic_ns, payload_version, payload_json, classification, is_error_like)
             VALUES (?1, 's1', 'p1', ?6, 'test', 'test', ?2, ?3, ?4, 0, 1, ?7, 'unknown', ?5)",
            rusqlite::params![id, kind, severity, ts, error, seq, payload],
        ).unwrap();
    }

    tx.commit().unwrap();
    conn
}

#[test]
fn test_query_filter_kind() {
    let conn = seed_db();
    let results = runlens_query::run_query(&conn, r#"FROM events WHERE kind = "error""#).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0]["event_id"], "e3");
}

#[test]
fn test_query_filter_severity() {
    let conn = seed_db();
    let results = runlens_query::run_query(&conn, r#"FROM events WHERE severity = "error""#).unwrap();
    assert_eq!(results.len(), 2);
}

#[test]
fn test_query_filter_and() {
    let conn = seed_db();
    let results = runlens_query::run_query(
        &conn,
        r#"FROM events WHERE kind = "network.response" AND severity = "error""#,
    ).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0]["event_id"], "e5");
}

#[test]
fn test_query_order() {
    let conn = seed_db();
    let results = runlens_query::run_query(
        &conn,
        r#"FROM events WHERE kind = "network.response" ORDER BY sequence DESC"#,
    ).unwrap();
    assert_eq!(results.len(), 3);
}

#[test]
fn test_query_time_window() {
    let conn = seed_db();
    let results = runlens_query::run_query(
        &conn,
        r#"FROM events WITHIN 30s BEFORE "marker""#,
    ).unwrap();
    assert!(!results.is_empty(), "should find events before marker");
}

#[test]
fn test_query_empty() {
    let conn = seed_db();
    let results = runlens_query::run_query(
        &conn,
        r#"FROM events WHERE kind = "nonexistent""#,
    ).unwrap();
    assert!(results.is_empty());
}

#[test]
fn test_query_explain() {
    let conn = seed_db();
    let plan = runlens_query::run_explain(
        &conn,
        r#"FROM events WHERE kind = "error""#,
    ).unwrap();
    assert!(!plan.is_empty(), "explain should return rows");
}
