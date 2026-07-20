CREATE TABLE IF NOT EXISTS schema_version (
    version INTEGER PRIMARY KEY,
    applied_at TEXT NOT NULL
);

INSERT INTO schema_version (version, applied_at) VALUES (1, datetime('now'));

CREATE TABLE project (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    root TEXT NOT NULL,
    language_hints_json TEXT NOT NULL DEFAULT '[]'
);

CREATE TABLE session (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES project(id),
    state TEXT NOT NULL,
    started_at TEXT NOT NULL,
    stopped_at TEXT,
    command TEXT,
    args_json TEXT NOT NULL DEFAULT '[]',
    labels_json TEXT NOT NULL DEFAULT '[]',
    source_event_count INTEGER NOT NULL DEFAULT 0,
    imported INTEGER NOT NULL DEFAULT 0,
    bundle_origin TEXT
);

CREATE INDEX idx_session_project ON session(project_id, started_at);
CREATE INDEX idx_session_state ON session(state, started_at);

CREATE TABLE event (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    project_id TEXT NOT NULL,
    sequence INTEGER NOT NULL,
    source TEXT NOT NULL,
    kind TEXT NOT NULL,
    severity TEXT NOT NULL,
    utc_timestamp TEXT NOT NULL,
    monotonic_ns INTEGER NOT NULL,
    duration_ns INTEGER,
    correlation_id TEXT,
    parent_event_id TEXT,
    payload_version INTEGER NOT NULL,
    payload_json TEXT NOT NULL DEFAULT 'null',
    classification TEXT NOT NULL,
    previous_hash TEXT,
    current_hash TEXT
);

CREATE INDEX idx_event_session ON event(session_id, utc_timestamp);
CREATE INDEX idx_event_kind ON event(kind, utc_timestamp);
CREATE INDEX idx_event_seq ON event(session_id, sequence);

CREATE TABLE redaction (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL,
    event_id TEXT,
    kind TEXT NOT NULL,
    span_start INTEGER,
    span_end INTEGER,
    redaction TEXT NOT NULL,
    preview TEXT NOT NULL
);

CREATE INDEX idx_redaction_session ON redaction(session_id);
