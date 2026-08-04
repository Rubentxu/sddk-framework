//! SQLite-backed operational persistence for SDDK.
//!
//! The crate stores project identity, workspaces, cycle snapshots, immutable
//! hash-linked ledger events, artifact metadata, capability receipts, and cycle
//! leases. Callers supply all timestamps; this crate never reads the system clock.

#![forbid(unsafe_code)]
#![deny(clippy::all)]
#![warn(missing_docs)]

mod migrations;
mod models;

use std::path::Path;
use std::time::Duration;

use migrations::{LATEST_SCHEMA_VERSION, MIGRATION_1};
pub use models::*;
use rusqlite::{
    Connection, OpenFlags, OptionalExtension, Row, Transaction, TransactionBehavior, params,
};
use sddk_domain::CycleManifest;
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use thiserror::Error;

/// Result type returned by storage operations.
pub type Result<T> = std::result::Result<T, StorageError>;

/// Errors emitted by the SQLite storage boundary.
#[derive(Debug, Error)]
pub enum StorageError {
    /// SQLite rejected an operation.
    #[error("sqlite storage error: {0}")]
    Database(#[from] rusqlite::Error),
    /// A persisted JSON value could not be encoded or decoded.
    #[error("storage serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    /// A database parent directory could not be created.
    #[error("storage filesystem error: {0}")]
    Io(#[from] std::io::Error),
    /// A requested record does not exist.
    #[error("{entity} not found: {id}")]
    NotFound {
        /// Entity kind.
        entity: &'static str,
        /// Missing entity identifier.
        id: String,
    },
    /// An idempotency key was reused for a different request.
    #[error("idempotency key {key:?} was already used for a different request")]
    IdempotencyConflict {
        /// Conflicting idempotency key.
        key: String,
    },
    /// A non-expired lease is owned by another runtime.
    #[error("cycle {cycle_id:?} is leased by {owner:?} until {expires_at_ms}")]
    LeaseConflict {
        /// Contended cycle identifier.
        cycle_id: String,
        /// Current lease owner.
        owner: String,
        /// Current lease expiry in Unix milliseconds.
        expires_at_ms: i64,
    },
    /// Lease times do not define a positive interval.
    #[error("lease expiry must be greater than acquisition time")]
    InvalidLease,
    /// Cycle state and event input refer to different cycles or projects.
    #[error("cycle state and ledger event identifiers do not match")]
    EventScopeMismatch,
    /// Existing identity data disagrees with an idempotent registration request.
    #[error("adoption registration conflicts with existing {entity}: {id}")]
    RegistrationConflict {
        /// Conflicting entity kind.
        entity: &'static str,
        /// Conflicting entity identifier.
        id: String,
    },
    /// A read-only database does not use the expected schema version.
    #[error("unsupported storage schema version {actual}; expected {expected}")]
    SchemaVersion {
        /// Version found in SQLite.
        actual: i32,
        /// Version supported by this runtime.
        expected: i32,
    },
    /// The ledger sequence or hash chain is invalid.
    #[error("ledger integrity failure at sequence {sequence}: {reason}")]
    LedgerIntegrity {
        /// Sequence at which verification failed.
        sequence: i64,
        /// Human-readable integrity failure.
        reason: String,
    },
}

/// SQLite-backed SDDK persistence.
pub struct Storage {
    connection: Connection,
}

impl Storage {
    /// Opens or creates a database and applies all pending migrations.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            std::fs::create_dir_all(parent)?;
        }
        Self::from_connection(Connection::open(path)?, true)
    }

    /// Opens an existing database without creating files or applying migrations.
    pub fn open_read_only(path: impl AsRef<Path>) -> Result<Self> {
        let connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        Self::from_connection(connection, false)
    }

    /// Opens an isolated in-memory database and applies all migrations.
    pub fn open_in_memory() -> Result<Self> {
        Self::from_connection(Connection::open_in_memory()?, true)
    }

    fn from_connection(mut connection: Connection, writable: bool) -> Result<Self> {
        connection.busy_timeout(Duration::from_secs(5))?;
        connection.pragma_update(None, "foreign_keys", true)?;
        if writable {
            connection.pragma_update(None, "journal_mode", "WAL")?;
            migrate(&mut connection)?;
        } else {
            let actual: i32 =
                connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
            if actual != LATEST_SCHEMA_VERSION {
                return Err(StorageError::SchemaVersion {
                    actual,
                    expected: LATEST_SCHEMA_VERSION,
                });
            }
        }
        Ok(Self { connection })
    }

    /// Returns the currently applied storage schema version.
    pub fn schema_version(&self) -> Result<i32> {
        Ok(self
            .connection
            .pragma_query_value(None, "user_version", |row| row.get(0))?)
    }

    /// Inserts a logical project.
    pub fn insert_project(&self, project: &ProjectRecord) -> Result<()> {
        self.connection.execute(
            "INSERT INTO projects (
                project_id, display_name, remote_url, scope, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                project.project_id,
                project.display_name,
                project.remote_url,
                project.scope,
                project.created_at
            ],
        )?;
        Ok(())
    }

    /// Loads a logical project by identifier.
    pub fn get_project(&self, project_id: &str) -> Result<ProjectRecord> {
        self.get_project_optional(project_id)?
            .ok_or_else(|| not_found("project", project_id))
    }

    /// Loads a logical project when present.
    pub fn get_project_optional(&self, project_id: &str) -> Result<Option<ProjectRecord>> {
        Ok(self
            .connection
            .query_row(
                "SELECT project_id, display_name, remote_url, scope, created_at
                 FROM projects WHERE project_id = ?1",
                [project_id],
                |row| {
                    Ok(ProjectRecord {
                        project_id: row.get(0)?,
                        display_name: row.get(1)?,
                        remote_url: row.get(2)?,
                        scope: row.get(3)?,
                        created_at: row.get(4)?,
                    })
                },
            )
            .optional()?)
    }

    /// Inserts a workspace belonging to an existing project.
    pub fn insert_workspace(&self, workspace: &WorkspaceRecord) -> Result<()> {
        self.connection.execute(
            "INSERT INTO workspaces (
                workspace_id, project_id, canonical_path, created_at
             ) VALUES (?1, ?2, ?3, ?4)",
            params![
                workspace.workspace_id,
                workspace.project_id,
                workspace.canonical_path,
                workspace.created_at
            ],
        )?;
        Ok(())
    }

    /// Loads a workspace by identifier.
    pub fn get_workspace(&self, workspace_id: &str) -> Result<WorkspaceRecord> {
        self.get_workspace_optional(workspace_id)?
            .ok_or_else(|| not_found("workspace", workspace_id))
    }

    /// Loads a workspace when present.
    pub fn get_workspace_optional(&self, workspace_id: &str) -> Result<Option<WorkspaceRecord>> {
        Ok(self
            .connection
            .query_row(
                "SELECT workspace_id, project_id, canonical_path, created_at
                 FROM workspaces WHERE workspace_id = ?1",
                [workspace_id],
                |row| {
                    Ok(WorkspaceRecord {
                        workspace_id: row.get(0)?,
                        project_id: row.get(1)?,
                        canonical_path: row.get(2)?,
                        created_at: row.get(3)?,
                    })
                },
            )
            .optional()?)
    }

    /// Reports whether the database contains any project registration.
    pub fn has_projects(&self) -> Result<bool> {
        Ok(self
            .connection
            .query_row("SELECT EXISTS(SELECT 1 FROM projects)", [], |row| {
                row.get(0)
            })?)
    }

    /// Registers a project and workspace in one SQLite transaction.
    ///
    /// Replaying matching identity data is a no-op. Existing identity data that
    /// disagrees with the request is rejected rather than overwritten.
    pub fn register_project_workspace(
        &mut self,
        project: &ProjectRecord,
        workspace: &WorkspaceRecord,
    ) -> Result<()> {
        if workspace.project_id != project.project_id {
            return Err(StorageError::RegistrationConflict {
                entity: "workspace project",
                id: workspace.workspace_id.clone(),
            });
        }
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let existing_project = project_optional_on(&transaction, &project.project_id)?;
        match existing_project {
            Some(existing)
                if existing.remote_url != project.remote_url || existing.scope != project.scope =>
            {
                return Err(StorageError::RegistrationConflict {
                    entity: "project",
                    id: project.project_id.clone(),
                });
            }
            Some(_) => {}
            None => {
                let has_other: bool =
                    transaction.query_row("SELECT EXISTS(SELECT 1 FROM projects)", [], |row| {
                        row.get(0)
                    })?;
                if has_other {
                    return Err(StorageError::RegistrationConflict {
                        entity: "project",
                        id: project.project_id.clone(),
                    });
                }
                transaction.execute(
                    "INSERT INTO projects (
                        project_id, display_name, remote_url, scope, created_at
                     ) VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![
                        project.project_id,
                        project.display_name,
                        project.remote_url,
                        project.scope,
                        project.created_at
                    ],
                )?;
            }
        }
        let existing_workspace = workspace_optional_on(&transaction, &workspace.workspace_id)?;
        match existing_workspace {
            Some(existing)
                if existing.project_id != workspace.project_id
                    || existing.canonical_path != workspace.canonical_path =>
            {
                return Err(StorageError::RegistrationConflict {
                    entity: "workspace",
                    id: workspace.workspace_id.clone(),
                });
            }
            Some(_) => {}
            None => {
                transaction.execute(
                    "INSERT INTO workspaces (
                        workspace_id, project_id, canonical_path, created_at
                     ) VALUES (?1, ?2, ?3, ?4)",
                    params![
                        workspace.workspace_id,
                        workspace.project_id,
                        workspace.canonical_path,
                        workspace.created_at
                    ],
                )?;
            }
        }
        transaction.commit()?;
        Ok(())
    }

    /// Inserts a cycle snapshot without a ledger event.
    ///
    /// Runtime code should normally prefer [`Storage::insert_cycle_with_event`]
    /// so the initial state and causal event are committed atomically.
    pub fn insert_cycle(&self, cycle: &CycleRecord) -> Result<()> {
        insert_cycle_on(&self.connection, cycle)
    }

    /// Inserts a cycle snapshot and its initial event atomically.
    pub fn insert_cycle_with_event(
        &mut self,
        cycle: &CycleRecord,
        event: &LedgerEventInput,
    ) -> Result<LedgerEvent> {
        ensure_event_scope(&cycle.manifest, event)?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        insert_cycle_on(&transaction, cycle)?;
        let appended = append_event_on(&transaction, event)?;
        transaction.commit()?;
        Ok(appended)
    }

    /// Loads a cycle snapshot by identifier.
    pub fn get_cycle(&self, cycle_id: &str) -> Result<CycleRecord> {
        self.connection
            .query_row(
                "SELECT manifest_json, created_at, updated_at
                 FROM cycles WHERE cycle_id = ?1",
                [cycle_id],
                cycle_from_row,
            )
            .optional()?
            .ok_or_else(|| not_found("cycle", cycle_id))
    }

    /// Replaces a cycle snapshot and appends its causal event atomically.
    pub fn update_cycle_with_event(
        &mut self,
        manifest: &CycleManifest,
        updated_at: &str,
        event: &LedgerEventInput,
    ) -> Result<LedgerEvent> {
        ensure_event_scope(manifest, event)?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let changed = transaction.execute(
            "UPDATE cycles SET
                project_id = ?2,
                workspace_id = ?3,
                status = ?4,
                phase = ?5,
                manifest_json = ?6,
                updated_at = ?7
             WHERE cycle_id = ?1",
            params![
                manifest.cycle_id,
                manifest.project_id,
                manifest.workspace_id,
                enum_string(&manifest.status)?,
                enum_string(&manifest.phase)?,
                serde_json::to_string(manifest)?,
                updated_at
            ],
        )?;
        if changed == 0 {
            return Err(not_found("cycle", &manifest.cycle_id));
        }
        let appended = append_event_on(&transaction, event)?;
        transaction.commit()?;
        Ok(appended)
    }

    /// Appends one immutable event to the ledger.
    pub fn append_event(&mut self, event: &LedgerEventInput) -> Result<LedgerEvent> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let appended = append_event_on(&transaction, event)?;
        transaction.commit()?;
        Ok(appended)
    }

    /// Lists all ledger events in ascending sequence order.
    pub fn list_events(&self) -> Result<Vec<LedgerEvent>> {
        let mut statement = self.connection.prepare(
            "SELECT sequence, event_id, project_id, cycle_id, frame_id,
                    command_id, actor, event_type, occurred_at,
                    state_before_json, state_after_json, payload_json,
                    previous_hash, event_hash
             FROM ledger_events ORDER BY sequence ASC",
        )?;
        let rows = statement.query_map([], event_from_row)?;
        rows.map(|row| row.map_err(StorageError::from)).collect()
    }

    /// Lists ledger events for one cycle in ascending global sequence order.
    pub fn list_cycle_events(&self, cycle_id: &str) -> Result<Vec<LedgerEvent>> {
        let mut statement = self.connection.prepare(
            "SELECT sequence, event_id, project_id, cycle_id, frame_id,
                    command_id, actor, event_type, occurred_at,
                    state_before_json, state_after_json, payload_json,
                    previous_hash, event_hash
             FROM ledger_events WHERE cycle_id = ?1 ORDER BY sequence ASC",
        )?;
        let rows = statement.query_map([cycle_id], event_from_row)?;
        rows.map(|row| row.map_err(StorageError::from)).collect()
    }

    /// Verifies sequence continuity, predecessor links, and event hashes.
    pub fn verify_ledger(&self) -> Result<LedgerVerification> {
        let events = self.list_events()?;
        let mut previous_hash: Option<String> = None;
        for (expected_sequence, event) in (1_i64..).zip(&events) {
            if event.sequence != expected_sequence {
                return Err(integrity_error(event.sequence, "sequence gap"));
            }
            if event.previous_hash != previous_hash {
                return Err(integrity_error(event.sequence, "previous hash mismatch"));
            }
            let expected_hash = hash_event(event.sequence, &event.as_input(), &previous_hash)?;
            if event.event_hash != expected_hash {
                return Err(integrity_error(event.sequence, "event hash mismatch"));
            }
            previous_hash = Some(event.event_hash.clone());
        }
        Ok(LedgerVerification {
            event_count: events.len(),
            last_hash: previous_hash,
        })
    }

    /// Inserts artifact metadata. Artifact bytes remain in the external store.
    pub fn insert_artifact(&self, artifact: &ArtifactRecord) -> Result<()> {
        self.connection.execute(
            "INSERT INTO artifacts (
                artifact_id, project_id, cycle_id, kind, path, sha256,
                producer, created_at, metadata_json
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                artifact.artifact_id,
                artifact.project_id,
                artifact.cycle_id,
                artifact.kind,
                artifact.path,
                artifact.sha256,
                artifact.producer,
                artifact.created_at,
                serde_json::to_string(&artifact.metadata)?
            ],
        )?;
        Ok(())
    }

    /// Loads artifact metadata by identifier.
    pub fn get_artifact(&self, artifact_id: &str) -> Result<ArtifactRecord> {
        self.connection
            .query_row(
                "SELECT artifact_id, project_id, cycle_id, kind, path, sha256,
                        producer, created_at, metadata_json
                 FROM artifacts WHERE artifact_id = ?1",
                [artifact_id],
                artifact_from_row,
            )
            .optional()?
            .ok_or_else(|| not_found("artifact", artifact_id))
    }

    /// Records a capability receipt exactly once for an idempotency key.
    ///
    /// Reusing the key with the same request returns the original receipt.
    /// Reusing it with a different request returns
    /// [`StorageError::IdempotencyConflict`].
    pub fn record_capability_receipt(
        &mut self,
        input: &CapabilityReceiptInput,
    ) -> Result<IdempotencyOutcome> {
        let request_json = serde_json::to_string(&input.request)?;
        let request_hash = hash_capability_request(input)?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;

        let existing = transaction
            .query_row(
                "SELECT request_hash, receipt_id FROM idempotency_records
                 WHERE project_id = ?1 AND idempotency_key = ?2",
                params![input.project_id, input.idempotency_key],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()?;

        if let Some((existing_hash, receipt_id)) = existing {
            if existing_hash != request_hash {
                return Err(StorageError::IdempotencyConflict {
                    key: input.idempotency_key.clone(),
                });
            }
            let receipt = get_capability_receipt_on(&transaction, &receipt_id)?;
            transaction.commit()?;
            return Ok(IdempotencyOutcome::Replayed(receipt));
        }

        transaction.execute(
            "INSERT INTO capability_receipts (
                receipt_id, project_id, cycle_id, capability, request_hash,
                request_json, status, result_json, started_at, completed_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                input.receipt_id,
                input.project_id,
                input.cycle_id,
                input.capability,
                request_hash,
                request_json,
                enum_string(&input.status)?,
                optional_json(&input.result)?,
                input.started_at,
                input.completed_at
            ],
        )?;
        transaction.execute(
            "INSERT INTO idempotency_records (
                project_id, idempotency_key, request_hash, receipt_id, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                input.project_id,
                input.idempotency_key,
                request_hash,
                input.receipt_id,
                input.started_at
            ],
        )?;
        let receipt = get_capability_receipt_on(&transaction, &input.receipt_id)?;
        transaction.commit()?;
        Ok(IdempotencyOutcome::Inserted(receipt))
    }

    /// Loads a capability receipt by identifier.
    pub fn get_capability_receipt(&self, receipt_id: &str) -> Result<CapabilityReceipt> {
        get_capability_receipt_on(&self.connection, receipt_id)
    }

    /// Acquires an absent or expired cycle lease.
    ///
    /// `now_ms` and `expires_at_ms` are supplied by the caller. Replacing an
    /// expired lease increments its fencing token.
    pub fn acquire_cycle_lease(
        &mut self,
        cycle_id: &str,
        owner: &str,
        now_ms: i64,
        expires_at_ms: i64,
    ) -> Result<CycleLease> {
        if now_ms < 0 || expires_at_ms <= now_ms {
            return Err(StorageError::InvalidLease);
        }
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let existing = get_cycle_lease_on(&transaction, cycle_id).optional()?;
        let fencing_token = match existing {
            Some(lease) if lease.expires_at_ms > now_ms => {
                return Err(StorageError::LeaseConflict {
                    cycle_id: cycle_id.to_owned(),
                    owner: lease.owner,
                    expires_at_ms: lease.expires_at_ms,
                });
            }
            Some(lease) => lease.fencing_token + 1,
            None => 1,
        };
        transaction.execute(
            "INSERT INTO cycle_leases (
                cycle_id, owner, acquired_at_ms, expires_at_ms, fencing_token
             ) VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(cycle_id) DO UPDATE SET
                owner = excluded.owner,
                acquired_at_ms = excluded.acquired_at_ms,
                expires_at_ms = excluded.expires_at_ms,
                fencing_token = excluded.fencing_token",
            params![cycle_id, owner, now_ms, expires_at_ms, fencing_token],
        )?;
        transaction.commit()?;
        Ok(CycleLease {
            cycle_id: cycle_id.to_owned(),
            owner: owner.to_owned(),
            acquired_at_ms: now_ms,
            expires_at_ms,
            fencing_token,
        })
    }

    /// Loads the current cycle lease.
    pub fn get_cycle_lease(&self, cycle_id: &str) -> Result<CycleLease> {
        get_cycle_lease_on(&self.connection, cycle_id)
            .optional()?
            .ok_or_else(|| not_found("cycle lease", cycle_id))
    }

    /// Releases a cycle lease only when owner and fencing token still match.
    pub fn release_cycle_lease(
        &self,
        cycle_id: &str,
        owner: &str,
        fencing_token: i64,
    ) -> Result<bool> {
        Ok(self.connection.execute(
            "DELETE FROM cycle_leases
             WHERE cycle_id = ?1 AND owner = ?2 AND fencing_token = ?3",
            params![cycle_id, owner, fencing_token],
        )? == 1)
    }
}

impl LedgerEvent {
    fn as_input(&self) -> LedgerEventInput {
        LedgerEventInput {
            event_id: self.event_id.clone(),
            project_id: self.project_id.clone(),
            cycle_id: self.cycle_id.clone(),
            frame_id: self.frame_id.clone(),
            command_id: self.command_id.clone(),
            actor: self.actor.clone(),
            event_type: self.event_type.clone(),
            occurred_at: self.occurred_at.clone(),
            state_before: self.state_before.clone(),
            state_after: self.state_after.clone(),
            payload: self.payload.clone(),
        }
    }
}

fn migrate(connection: &mut Connection) -> Result<()> {
    let version: i32 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
    if version < 1 {
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        transaction.execute_batch(MIGRATION_1)?;
        transaction.pragma_update(None, "user_version", LATEST_SCHEMA_VERSION)?;
        transaction.commit()?;
    }
    Ok(())
}

fn insert_cycle_on(connection: &Connection, cycle: &CycleRecord) -> Result<()> {
    connection.execute(
        "INSERT INTO cycles (
            cycle_id, project_id, workspace_id, status, phase, manifest_json,
            created_at, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            cycle.manifest.cycle_id,
            cycle.manifest.project_id,
            cycle.manifest.workspace_id,
            enum_string(&cycle.manifest.status)?,
            enum_string(&cycle.manifest.phase)?,
            serde_json::to_string(&cycle.manifest)?,
            cycle.created_at,
            cycle.updated_at
        ],
    )?;
    Ok(())
}

fn project_optional_on(connection: &Connection, project_id: &str) -> Result<Option<ProjectRecord>> {
    Ok(connection
        .query_row(
            "SELECT project_id, display_name, remote_url, scope, created_at
             FROM projects WHERE project_id = ?1",
            [project_id],
            |row| {
                Ok(ProjectRecord {
                    project_id: row.get(0)?,
                    display_name: row.get(1)?,
                    remote_url: row.get(2)?,
                    scope: row.get(3)?,
                    created_at: row.get(4)?,
                })
            },
        )
        .optional()?)
}

fn workspace_optional_on(
    connection: &Connection,
    workspace_id: &str,
) -> Result<Option<WorkspaceRecord>> {
    Ok(connection
        .query_row(
            "SELECT workspace_id, project_id, canonical_path, created_at
             FROM workspaces WHERE workspace_id = ?1",
            [workspace_id],
            |row| {
                Ok(WorkspaceRecord {
                    workspace_id: row.get(0)?,
                    project_id: row.get(1)?,
                    canonical_path: row.get(2)?,
                    created_at: row.get(3)?,
                })
            },
        )
        .optional()?)
}

fn append_event_on(transaction: &Transaction<'_>, input: &LedgerEventInput) -> Result<LedgerEvent> {
    let previous = transaction
        .query_row(
            "SELECT sequence, event_hash FROM ledger_events ORDER BY sequence DESC LIMIT 1",
            [],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()?;
    let (sequence, previous_hash) = previous
        .map(|(sequence, hash)| (sequence + 1, Some(hash)))
        .unwrap_or((1, None));
    let event_hash = hash_event(sequence, input, &previous_hash)?;
    transaction.execute(
        "INSERT INTO ledger_events (
            sequence, event_id, project_id, cycle_id, frame_id, command_id,
            actor, event_type, occurred_at, state_before_json,
            state_after_json, payload_json, previous_hash, event_hash
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
        params![
            sequence,
            input.event_id,
            input.project_id,
            input.cycle_id,
            input.frame_id,
            input.command_id,
            input.actor,
            input.event_type,
            input.occurred_at,
            optional_json(&input.state_before)?,
            optional_json(&input.state_after)?,
            serde_json::to_string(&input.payload)?,
            previous_hash,
            event_hash
        ],
    )?;
    Ok(LedgerEvent {
        sequence,
        event_id: input.event_id.clone(),
        project_id: input.project_id.clone(),
        cycle_id: input.cycle_id.clone(),
        frame_id: input.frame_id.clone(),
        command_id: input.command_id.clone(),
        actor: input.actor.clone(),
        event_type: input.event_type.clone(),
        occurred_at: input.occurred_at.clone(),
        state_before: input.state_before.clone(),
        state_after: input.state_after.clone(),
        payload: input.payload.clone(),
        previous_hash,
        event_hash,
    })
}

#[derive(Serialize)]
struct EventHashMaterial<'a> {
    sequence: i64,
    event_id: &'a str,
    project_id: &'a str,
    cycle_id: &'a Option<String>,
    frame_id: &'a str,
    command_id: &'a str,
    actor: &'a str,
    event_type: &'a str,
    occurred_at: &'a str,
    state_before: &'a Option<Value>,
    state_after: &'a Option<Value>,
    payload: &'a Value,
    previous_hash: &'a Option<String>,
}

#[derive(Serialize)]
struct CapabilityRequestHashMaterial<'a> {
    cycle_id: &'a Option<String>,
    capability: &'a str,
    request: &'a Value,
}

fn hash_event(
    sequence: i64,
    input: &LedgerEventInput,
    previous_hash: &Option<String>,
) -> Result<String> {
    let material = EventHashMaterial {
        sequence,
        event_id: &input.event_id,
        project_id: &input.project_id,
        cycle_id: &input.cycle_id,
        frame_id: &input.frame_id,
        command_id: &input.command_id,
        actor: &input.actor,
        event_type: &input.event_type,
        occurred_at: &input.occurred_at,
        state_before: &input.state_before,
        state_after: &input.state_after,
        payload: &input.payload,
        previous_hash,
    };
    Ok(hash_bytes(&serde_json::to_vec(&material)?))
}

fn hash_bytes(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    format!("sha256:{digest:x}")
}

fn hash_capability_request(input: &CapabilityReceiptInput) -> Result<String> {
    let material = CapabilityRequestHashMaterial {
        cycle_id: &input.cycle_id,
        capability: &input.capability,
        request: &input.request,
    };
    Ok(hash_bytes(&serde_json::to_vec(&material)?))
}

fn ensure_event_scope(manifest: &CycleManifest, event: &LedgerEventInput) -> Result<()> {
    if event.project_id != manifest.project_id
        || event.cycle_id.as_deref() != Some(manifest.cycle_id.as_str())
    {
        return Err(StorageError::EventScopeMismatch);
    }
    Ok(())
}

fn enum_string<T: Serialize>(value: &T) -> Result<String> {
    match serde_json::to_value(value)? {
        Value::String(value) => Ok(value),
        _ => unreachable!("serialized enum must be a string"),
    }
}

fn optional_json(value: &Option<Value>) -> Result<Option<String>> {
    value
        .as_ref()
        .map(serde_json::to_string)
        .transpose()
        .map_err(StorageError::from)
}

fn parse_optional_json(value: Option<String>) -> rusqlite::Result<Option<Value>> {
    value
        .map(|json| serde_json::from_str(&json).map_err(json_from_sql_error))
        .transpose()
}

fn cycle_from_row(row: &Row<'_>) -> rusqlite::Result<CycleRecord> {
    let manifest_json: String = row.get(0)?;
    Ok(CycleRecord {
        manifest: serde_json::from_str(&manifest_json).map_err(json_from_sql_error)?,
        created_at: row.get(1)?,
        updated_at: row.get(2)?,
    })
}

fn event_from_row(row: &Row<'_>) -> rusqlite::Result<LedgerEvent> {
    let payload_json: String = row.get(11)?;
    Ok(LedgerEvent {
        sequence: row.get(0)?,
        event_id: row.get(1)?,
        project_id: row.get(2)?,
        cycle_id: row.get(3)?,
        frame_id: row.get(4)?,
        command_id: row.get(5)?,
        actor: row.get(6)?,
        event_type: row.get(7)?,
        occurred_at: row.get(8)?,
        state_before: parse_optional_json(row.get(9)?)?,
        state_after: parse_optional_json(row.get(10)?)?,
        payload: serde_json::from_str(&payload_json).map_err(json_from_sql_error)?,
        previous_hash: row.get(12)?,
        event_hash: row.get(13)?,
    })
}

fn artifact_from_row(row: &Row<'_>) -> rusqlite::Result<ArtifactRecord> {
    let metadata_json: String = row.get(8)?;
    Ok(ArtifactRecord {
        artifact_id: row.get(0)?,
        project_id: row.get(1)?,
        cycle_id: row.get(2)?,
        kind: row.get(3)?,
        path: row.get(4)?,
        sha256: row.get(5)?,
        producer: row.get(6)?,
        created_at: row.get(7)?,
        metadata: serde_json::from_str(&metadata_json).map_err(json_from_sql_error)?,
    })
}

fn get_capability_receipt_on(
    connection: &Connection,
    receipt_id: &str,
) -> Result<CapabilityReceipt> {
    connection
        .query_row(
            "SELECT receipt_id, project_id, cycle_id, capability, request_hash,
                    request_json, status, result_json, started_at, completed_at
             FROM capability_receipts WHERE receipt_id = ?1",
            [receipt_id],
            capability_receipt_from_row,
        )
        .optional()?
        .ok_or_else(|| not_found("capability receipt", receipt_id))
}

fn capability_receipt_from_row(row: &Row<'_>) -> rusqlite::Result<CapabilityReceipt> {
    let request_json: String = row.get(5)?;
    let status: String = row.get(6)?;
    Ok(CapabilityReceipt {
        receipt_id: row.get(0)?,
        project_id: row.get(1)?,
        cycle_id: row.get(2)?,
        capability: row.get(3)?,
        request_hash: row.get(4)?,
        request: serde_json::from_str(&request_json).map_err(json_from_sql_error)?,
        status: serde_json::from_value(Value::String(status)).map_err(json_from_sql_error)?,
        result: parse_optional_json(row.get(7)?)?,
        started_at: row.get(8)?,
        completed_at: row.get(9)?,
    })
}

fn get_cycle_lease_on(connection: &Connection, cycle_id: &str) -> rusqlite::Result<CycleLease> {
    connection.query_row(
        "SELECT cycle_id, owner, acquired_at_ms, expires_at_ms, fencing_token
         FROM cycle_leases WHERE cycle_id = ?1",
        [cycle_id],
        |row| {
            Ok(CycleLease {
                cycle_id: row.get(0)?,
                owner: row.get(1)?,
                acquired_at_ms: row.get(2)?,
                expires_at_ms: row.get(3)?,
                fencing_token: row.get(4)?,
            })
        },
    )
}

fn json_from_sql_error(error: serde_json::Error) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(error))
}

fn not_found(entity: &'static str, id: &str) -> StorageError {
    StorageError::NotFound {
        entity,
        id: id.to_owned(),
    }
}

fn integrity_error(sequence: i64, reason: &str) -> StorageError {
    StorageError::LedgerIntegrity {
        sequence,
        reason: reason.to_owned(),
    }
}
