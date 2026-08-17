pub(crate) const LATEST_SCHEMA_VERSION: i32 = 6;

/// Runs all pending migrations on an open SQLite connection.
pub(crate) fn run_migrations(conn: &mut rusqlite::Connection) -> Result<(), super::StorageError> {
    use rusqlite::TransactionBehavior;
    let version: i32 = conn
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .map_err(super::StorageError::Database)?;
    if version < 1 {
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(super::StorageError::Database)?;
        tx.execute_batch(MIGRATION_1)
            .map_err(super::StorageError::Database)?;
        tx.pragma_update(None, "user_version", 1)
            .map_err(super::StorageError::Database)?;
        tx.commit()
            .map_err(super::StorageError::Database)?;
    }
    if version < 2 {
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(super::StorageError::Database)?;
        tx.execute_batch(MIGRATION_2)
            .map_err(super::StorageError::Database)?;
        tx.pragma_update(None, "user_version", 2)
            .map_err(super::StorageError::Database)?;
        tx.commit()
            .map_err(super::StorageError::Database)?;
    }
    if version < 3 {
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(super::StorageError::Database)?;
        tx.execute_batch(MIGRATION_3)
            .map_err(super::StorageError::Database)?;
        tx.pragma_update(None, "user_version", 3)
            .map_err(super::StorageError::Database)?;
        tx.commit()
            .map_err(super::StorageError::Database)?;
    }
    if version < 4 {
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(super::StorageError::Database)?;
        tx.execute_batch(MIGRATION_4)
            .map_err(super::StorageError::Database)?;
        tx.pragma_update(None, "user_version", 4)
            .map_err(super::StorageError::Database)?;
        tx.commit()
            .map_err(super::StorageError::Database)?;
    }
    if version < 5 {
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(super::StorageError::Database)?;
        tx.execute_batch(MIGRATION_5)
            .map_err(super::StorageError::Database)?;
        tx.pragma_update(None, "user_version", 5)
            .map_err(super::StorageError::Database)?;
        tx.commit()
            .map_err(super::StorageError::Database)?;
    }
    if version < 6 {
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(super::StorageError::Database)?;
        tx.execute_batch(MIGRATION_6)
            .map_err(super::StorageError::Database)?;
        tx.pragma_update(None, "user_version", 6)
            .map_err(super::StorageError::Database)?;
        tx.commit()
            .map_err(super::StorageError::Database)?;
    }
    Ok(())
}

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

pub(crate) const MIGRATION_2: &str = r#"
CREATE TABLE gate_receipts (
    receipt_id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(project_id) ON DELETE RESTRICT,
    cycle_id TEXT,
    gate TEXT NOT NULL CHECK (gate <> ''),
    evaluator TEXT NOT NULL CHECK (evaluator <> ''),
    transition_id TEXT NOT NULL CHECK (transition_id <> ''),
    plan_hash TEXT NOT NULL CHECK (plan_hash <> ''),
    outcome TEXT NOT NULL CHECK (outcome IN ('passed', 'failed')),
    evidence TEXT NOT NULL,
    actor TEXT NOT NULL CHECK (actor <> ''),
    command_id TEXT NOT NULL CHECK (command_id <> ''),
    frame_id TEXT NOT NULL CHECK (frame_id <> ''),
    evaluated_at TEXT NOT NULL,
    FOREIGN KEY (project_id, cycle_id)
        REFERENCES cycles(project_id, cycle_id) ON DELETE RESTRICT
);

CREATE INDEX gate_receipts_cycle_idx ON gate_receipts(cycle_id);
CREATE INDEX gate_receipts_plan_hash_idx ON gate_receipts(plan_hash);
"#;

pub(crate) const MIGRATION_3: &str = r#"
ALTER TABLE gate_receipts ADD COLUMN seq INTEGER NOT NULL DEFAULT 1;

CREATE UNIQUE INDEX gate_receipts_gate_plan_seq_uniq
    ON gate_receipts(gate, plan_hash, seq);
"#;

pub(crate) const MIGRATION_4: &str = r#"
-- GateOutcomeStatus gains `waived`; SQLite cannot ALTER a CHECK constraint,
-- so the table is recreated. Nothing references gate_receipts, so the rename
-- is safe; the old composite FK (project_id, cycle_id) -> cycles(project_id,
-- cycle_id) pointed at a non-existent composite key (cycles' PK is cycle_id)
-- and is corrected to cycle_id -> cycles(cycle_id) in the recreated table.
-- Runs with foreign_keys=ON: every copied row must reference an existing
-- cycle (NULL cycle_id rows are exempt from FK enforcement).
ALTER TABLE gate_receipts RENAME TO gate_receipts_old;

CREATE TABLE gate_receipts (
    receipt_id TEXT NOT NULL PRIMARY KEY,
    project_id TEXT NOT NULL,
    cycle_id TEXT,
    gate TEXT NOT NULL CHECK (gate <> ''),
    evaluator TEXT NOT NULL CHECK (evaluator <> ''),
    transition_id TEXT NOT NULL CHECK (transition_id <> ''),
    plan_hash TEXT NOT NULL CHECK (plan_hash <> ''),
    outcome TEXT NOT NULL CHECK (outcome IN ('passed', 'failed', 'waived')),
    evidence TEXT NOT NULL,
    actor TEXT NOT NULL CHECK (actor <> ''),
    command_id TEXT NOT NULL CHECK (command_id <> ''),
    frame_id TEXT NOT NULL,
    evaluated_at TEXT NOT NULL,
    seq INTEGER NOT NULL DEFAULT 1,
    FOREIGN KEY (cycle_id)
        REFERENCES cycles(cycle_id) ON DELETE RESTRICT
);

INSERT INTO gate_receipts SELECT * FROM gate_receipts_old;
DROP TABLE gate_receipts_old;

CREATE UNIQUE INDEX gate_receipts_gate_plan_seq_uniq
    ON gate_receipts(gate, plan_hash, seq);
CREATE INDEX gate_receipts_cycle_idx ON gate_receipts(cycle_id);
CREATE INDEX gate_receipts_plan_hash_idx ON gate_receipts(plan_hash);
"#;

pub(crate) const MIGRATION_5: &str = r#"
-- events_v1: append-only event-sourced store for EventEnvelopeV1 (SDDK2-202).
-- Mirrors the ledger_events immutability policy via SQL triggers.
--
-- Minimal projects stub so the events_v1 FK reference is satisfiable when
-- SqliteEventStore runs without the full Storage migrations (e.g. in tests).
-- IF NOT EXISTS avoids conflict when both Storage and SqliteEventStore share
-- the same ledger.sqlite file.
CREATE TABLE IF NOT EXISTS projects (
    project_id  TEXT NOT NULL PRIMARY KEY
);

CREATE TABLE events_v1 (
    event_id           TEXT NOT NULL PRIMARY KEY
                       CHECK (event_id <> ''),
    stream_id          TEXT NOT NULL
                       CHECK (stream_id <> ''),
    sequence           INTEGER NOT NULL
                       CHECK (sequence > 0),
    event_type         TEXT NOT NULL
                       CHECK (event_type <> ''),
    schema_version     INTEGER NOT NULL
                       CHECK (schema_version = 1),
    project_id         TEXT NOT NULL
                       REFERENCES projects(project_id) ON DELETE RESTRICT,
    occurred_at        TEXT NOT NULL
                       CHECK (occurred_at <> ''),
    recorded_at        TEXT NOT NULL
                       CHECK (recorded_at <> ''),
    actor_json         TEXT NOT NULL,
    causation_id       TEXT,
    correlation_id     TEXT,
    cycle_id           TEXT,
    frame_id           TEXT,
    fork_id            TEXT,
    subjects_json      TEXT NOT NULL,
    payload_json       TEXT NOT NULL,
    evidence_refs_json TEXT NOT NULL,
    content_hash       TEXT NOT NULL
                       CHECK (content_hash LIKE 'sha256:%')
                       UNIQUE,
    metadata_json      TEXT,
    UNIQUE (stream_id, sequence)
);

CREATE INDEX events_v1_project_idx      ON events_v1(project_id);
CREATE INDEX events_v1_stream_seq_idx   ON events_v1(stream_id, sequence);
CREATE INDEX events_v1_content_hash_idx ON events_v1(content_hash);

CREATE TRIGGER events_v1_no_update
BEFORE UPDATE ON events_v1
BEGIN
    SELECT RAISE(ABORT, 'events_v1 are append-only');
END;

CREATE TRIGGER events_v1_no_delete
BEFORE DELETE ON events_v1
BEGIN
    SELECT RAISE(ABORT, 'events_v1 are append-only');
END;
"#;

pub(crate) const MIGRATION_6: &str = r#"
-- projection_checkpoints_v1: durable progress markers for read-model projections.
-- The table is mutable (no append-only triggers) because checkpoints are
-- regenerable from the event ledger via the rebuild() algorithm.
CREATE TABLE projection_checkpoints_v1 (
    projection_name      TEXT    NOT NULL,
    version              INTEGER NOT NULL,
    last_event_sequence  INTEGER NOT NULL
                         CHECK (last_event_sequence >= 0),
    last_event_hash      TEXT    NOT NULL
                         CHECK (last_event_hash LIKE 'sha256:%'),
    state_json           TEXT    NOT NULL
                         CHECK (length(state_json) > 0),
    updated_at           TEXT    NOT NULL
                         CHECK (updated_at <> ''),
    PRIMARY KEY (projection_name, version)
);

CREATE INDEX projection_checkpoints_v1_name_idx
    ON projection_checkpoints_v1(projection_name);
"#;
