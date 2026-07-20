-- 0001_initial: baseline schema for RunLens.

CREATE TABLE IF NOT EXISTS projects (
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
    bundle_origin TEXT,
    final_head_hash TEXT,
    FOREIGN KEY (project_id) REFERENCES projects(project_id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS sessions_by_project ON sessions(project_id, started_at);
CREATE INDEX IF NOT EXISTS sessions_by_state ON sessions(state);

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
    FOREIGN KEY (session_id) REFERENCES sessions(session_id) ON DELETE CASCADE,
    FOREIGN KEY (project_id) REFERENCES projects(project_id) ON DELETE CASCADE,
    UNIQUE (session_id, sequence)
);

CREATE INDEX IF NOT EXISTS events_by_session_seq ON events(session_id, sequence);
CREATE INDEX IF NOT EXISTS events_by_session_kind ON events(session_id, kind);
CREATE INDEX IF NOT EXISTS events_by_project ON events(project_id);
CREATE INDEX IF NOT EXISTS events_by_severity ON events(session_id, severity);

CREATE TABLE IF NOT EXISTS artifacts (
    content_hash TEXT PRIMARY KEY,
    size_bytes INTEGER NOT NULL,
    media_kind TEXT NOT NULL,
    stored_at TEXT NOT NULL DEFAULT (datetime('now')),
    origin TEXT NOT NULL DEFAULT 'recorded'
);

CREATE TABLE IF NOT EXISTS event_artifacts (
    event_id TEXT NOT NULL,
    artifact_hash TEXT NOT NULL,
    role TEXT NOT NULL,
    PRIMARY KEY (event_id, artifact_hash, role),
    FOREIGN KEY (event_id) REFERENCES events(event_id) ON DELETE CASCADE,
    FOREIGN KEY (artifact_hash) REFERENCES artifacts(content_hash) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS file_states (
    session_id TEXT NOT NULL,
    path TEXT NOT NULL,
    last_event_kind TEXT NOT NULL,
    last_seen_sequence INTEGER NOT NULL,
    content_hash TEXT,
    PRIMARY KEY (session_id, path),
    FOREIGN KEY (session_id) REFERENCES sessions(session_id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS markers (
    marker_id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    sequence INTEGER NOT NULL,
    label TEXT NOT NULL,
    body TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (session_id) REFERENCES sessions(session_id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS markers_by_session ON markers(session_id, sequence);

CREATE TABLE IF NOT EXISTS redaction_findings (
    finding_id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL,
    event_id TEXT,
    kind TEXT NOT NULL,
    span_start INTEGER,
    span_end INTEGER,
    redaction TEXT NOT NULL,
    preview TEXT NOT NULL,
    reviewed INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (session_id) REFERENCES sessions(session_id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS redactions_by_session ON redaction_findings(session_id);

CREATE TABLE IF NOT EXISTS imports (
    import_id TEXT PRIMARY KEY,
    bundle_path TEXT NOT NULL,
    bundle_size INTEGER NOT NULL,
    imported_session_id TEXT NOT NULL,
    imported_at TEXT NOT NULL DEFAULT (datetime('now')),
    verified INTEGER NOT NULL,
    FOREIGN KEY (imported_session_id) REFERENCES sessions(session_id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS imports_by_session ON imports(imported_session_id);

CREATE TABLE IF NOT EXISTS comparisons (
    comparison_id TEXT PRIMARY KEY,
    baseline_session_id TEXT NOT NULL,
    candidate_session_id TEXT NOT NULL,
    generated_at TEXT NOT NULL DEFAULT (datetime('now')),
    summary_json TEXT NOT NULL,
    FOREIGN KEY (baseline_session_id) REFERENCES sessions(session_id) ON DELETE CASCADE,
    FOREIGN KEY (candidate_session_id) REFERENCES sessions(session_id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS comparisons_by_candidate ON comparisons(candidate_session_id);

CREATE TABLE IF NOT EXISTS test_investigations (
    investigation_id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    target_label TEXT NOT NULL,
    runs_requested INTEGER NOT NULL,
    runs_completed INTEGER NOT NULL,
    passes INTEGER NOT NULL,
    failures INTEGER NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (session_id) REFERENCES sessions(session_id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS investigations_by_session ON test_investigations(session_id);

CREATE TABLE IF NOT EXISTS integrations (
    integration_id TEXT PRIMARY KEY,
    kind TEXT NOT NULL,
    label TEXT NOT NULL,
    public_key TEXT NOT NULL,
    secret_hash TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    last_used_at TEXT,
    revoked INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS bundles (
    bundle_id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    format_version TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    size_bytes INTEGER NOT NULL,
    artifact_count INTEGER NOT NULL,
    FOREIGN KEY (session_id) REFERENCES sessions(session_id) ON DELETE CASCADE
);
