pub(crate) const LATEST_SCHEMA_VERSION: i32 = 1;

pub(crate) const MIGRATION_1: &str = r#"
CREATE TABLE projects (
    project_id TEXT PRIMARY KEY,
    display_name TEXT NOT NULL CHECK (display_name <> ''),
    remote_url TEXT,
    scope TEXT NOT NULL CHECK (scope <> ''),
    created_at TEXT NOT NULL,
    UNIQUE (remote_url, scope)
);

CREATE TABLE workspaces (
    workspace_id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(project_id) ON DELETE RESTRICT,
    canonical_path TEXT NOT NULL CHECK (canonical_path <> ''),
    created_at TEXT NOT NULL,
    UNIQUE (project_id, canonical_path),
    UNIQUE (project_id, workspace_id)
);

CREATE TABLE cycles (
    cycle_id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL,
    workspace_id TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN (
        'OPEN', 'BLOCKED', 'REMEDIATING', 'RELEASE_PENDING',
        'RELEASED', 'CLOSED', 'ABANDONED', 'RECOVERING'
    )),
    phase TEXT NOT NULL CHECK (phase IN (
        'explore', 'specify', 'design', 'plan', 'build',
        'verify', 'review', 'release', 'archive'
    )),
    manifest_json TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (project_id, workspace_id)
        REFERENCES workspaces(project_id, workspace_id) ON DELETE RESTRICT,
    UNIQUE (project_id, cycle_id)
);

CREATE INDEX cycles_project_status_idx ON cycles(project_id, status);

CREATE TABLE ledger_events (
    sequence INTEGER PRIMARY KEY CHECK (sequence > 0),
    event_id TEXT NOT NULL UNIQUE,
    project_id TEXT NOT NULL REFERENCES projects(project_id) ON DELETE RESTRICT,
    cycle_id TEXT,
    frame_id TEXT NOT NULL,
    command_id TEXT NOT NULL,
    actor TEXT NOT NULL,
    event_type TEXT NOT NULL,
    occurred_at TEXT NOT NULL,
    state_before_json TEXT,
    state_after_json TEXT,
    payload_json TEXT NOT NULL,
    previous_hash TEXT,
    event_hash TEXT NOT NULL UNIQUE,
    CHECK (
        (sequence = 1 AND previous_hash IS NULL)
        OR (sequence > 1 AND previous_hash IS NOT NULL)
    ),
    FOREIGN KEY (project_id, cycle_id)
        REFERENCES cycles(project_id, cycle_id) ON DELETE RESTRICT
);

CREATE INDEX ledger_events_cycle_sequence_idx
    ON ledger_events(cycle_id, sequence);

CREATE TRIGGER ledger_events_no_update
BEFORE UPDATE ON ledger_events
BEGIN
    SELECT RAISE(ABORT, 'ledger events are append-only');
END;

CREATE TRIGGER ledger_events_no_delete
BEFORE DELETE ON ledger_events
BEGIN
    SELECT RAISE(ABORT, 'ledger events are append-only');
END;

CREATE TABLE artifacts (
    artifact_id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(project_id) ON DELETE RESTRICT,
    cycle_id TEXT,
    kind TEXT NOT NULL CHECK (kind <> ''),
    path TEXT NOT NULL CHECK (path <> ''),
    sha256 TEXT,
    producer TEXT,
    created_at TEXT NOT NULL,
    metadata_json TEXT NOT NULL,
    FOREIGN KEY (project_id, cycle_id)
        REFERENCES cycles(project_id, cycle_id) ON DELETE RESTRICT
);

CREATE INDEX artifacts_project_hash_idx ON artifacts(project_id, sha256);
CREATE INDEX artifacts_cycle_idx ON artifacts(cycle_id);

CREATE TABLE capability_receipts (
    receipt_id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(project_id) ON DELETE RESTRICT,
    cycle_id TEXT,
    capability TEXT NOT NULL CHECK (capability <> ''),
    request_hash TEXT NOT NULL,
    request_json TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('started', 'succeeded', 'failed', 'unknown')),
    result_json TEXT,
    started_at TEXT NOT NULL,
    completed_at TEXT,
    FOREIGN KEY (project_id, cycle_id)
        REFERENCES cycles(project_id, cycle_id) ON DELETE RESTRICT
);

CREATE TABLE idempotency_records (
    project_id TEXT NOT NULL REFERENCES projects(project_id) ON DELETE RESTRICT,
    idempotency_key TEXT NOT NULL,
    request_hash TEXT NOT NULL,
    receipt_id TEXT NOT NULL UNIQUE
        REFERENCES capability_receipts(receipt_id) ON DELETE RESTRICT,
    created_at TEXT NOT NULL,
    PRIMARY KEY (project_id, idempotency_key)
);

CREATE TABLE cycle_leases (
    cycle_id TEXT PRIMARY KEY REFERENCES cycles(cycle_id) ON DELETE RESTRICT,
    owner TEXT NOT NULL CHECK (owner <> ''),
    acquired_at_ms INTEGER NOT NULL CHECK (acquired_at_ms >= 0),
    expires_at_ms INTEGER NOT NULL CHECK (expires_at_ms > acquired_at_ms),
    fencing_token INTEGER NOT NULL CHECK (fencing_token > 0)
);
"#;
