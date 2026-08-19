//! Hexagonal persistence port (Phase 1 M1 exit).
//!
//! `sddk_engine` depends only on this trait; `sddk_storage::Storage` is the
//! concrete SQLite implementation. The trait is object-safe (no associated
//! `Self` fns, no `Self` in generics) so `&dyn Ledger` is usable from the
//! engine's accessor.

use crate::StorageError;
use crate::metrics::MetricsRecord;
use crate::models::*;

/// Hexagonal port over the SDDK ledger.
pub trait Ledger {
    // ── Cycle ops ─────────────────────────────────────────────────────────
    /// Loads a cycle snapshot by identifier.
    fn get_cycle(&self, cycle_id: &str) -> Result<CycleRecord, StorageError>;
    /// Lists all ledger events for one cycle in ascending global sequence order.
    fn list_cycle_events(&self, cycle_id: &str) -> Result<Vec<LedgerEvent>, StorageError>;
    /// Inserts a cycle snapshot and its initial event atomically.
    fn insert_cycle_with_event(
        &mut self,
        cycle: &CycleRecord,
        event: &LedgerEventInput,
    ) -> Result<LedgerEvent, StorageError>;
    /// Replaces a cycle snapshot and appends its causal event atomically.
    fn update_cycle_with_event(
        &mut self,
        manifest: &crate::CycleManifest,
        updated_at: &str,
        event: &LedgerEventInput,
        release_lease_on_phase_change: bool,
    ) -> Result<LedgerEvent, StorageError>;

    // ── Lease ops ──────────────────────────────────────────────────────────
    /// Acquires an absent or expired cycle lease.
    fn acquire_cycle_lease(
        &mut self,
        cycle_id: &str,
        owner: &str,
        now_ms: i64,
        expires_at_ms: i64,
    ) -> Result<CycleLease, StorageError>;
    /// Releases a cycle lease only when owner and fencing token still match.
    #[allow(clippy::too_many_arguments)]
    fn release_cycle_lease(
        &mut self,
        project_id: &str,
        cycle_id: &str,
        owner: &str,
        fencing_token: i64,
        actor: &str,
        command_id: &str,
        occurred_at: &str,
    ) -> Result<bool, StorageError>;
    /// Extends the expiry of the lease you already hold without changing the
    /// fencing token (reuse / renew semantics).
    fn renew_cycle_lease(
        &mut self,
        cycle_id: &str,
        owner: &str,
        fencing_token: i64,
        now_ms: i64,
        new_expires_at_ms: i64,
    ) -> Result<CycleLease, StorageError>;
    /// Loads the current cycle lease.
    fn get_cycle_lease(&self, cycle_id: &str) -> Result<CycleLease, StorageError>;
    /// Verifies that the current lease still matches the caller's fencing
    /// token and has not expired at `now_ms`.
    fn verify_cycle_lease(
        &self,
        cycle_id: &str,
        owner: &str,
        fencing_token: i64,
        now_ms: i64,
    ) -> Result<CycleLease, StorageError>;

    // ── Gate receipts ──────────────────────────────────────────────────────
    /// Loads one gate receipt by identifier.
    fn get_gate_receipt(&self, receipt_id: &str) -> Result<GateReceipt, StorageError>;
    /// Persists one authorized gate evaluation receipt with atomic seq allocation.
    fn insert_gate_receipt_next_seq(
        &mut self,
        input: &GateReceiptNextSeqInput,
    ) -> Result<GateReceipt, StorageError>;

    // ── Project / workspace ────────────────────────────────────────────────
    /// Loads a logical project when present.
    fn get_project_optional(&self, project_id: &str)
    -> Result<Option<ProjectRecord>, StorageError>;
    /// Loads a workspace when present.
    fn get_workspace_optional(
        &self,
        workspace_id: &str,
    ) -> Result<Option<WorkspaceRecord>, StorageError>;
    /// Reports whether the database contains any project registration.
    fn has_projects(&self) -> Result<bool, StorageError>;
    /// Registers a project and workspace in one SQLite transaction.
    fn register_project_workspace(
        &mut self,
        project: &ProjectRecord,
        workspace: &WorkspaceRecord,
    ) -> Result<(), StorageError>;

    /// Loads all ledger events from the database in ascending sequence order.
    ///
    /// Used by telemetry ingest to derive metrics for cycles that have no
    /// metrics.jsonl entry.
    fn load_all_ledger_events(&self) -> Result<Vec<LedgerEvent>, StorageError>;
}

// ── Control-plane port ────────────────────────────────────────────────────────

/// Hexagonal port over the SDDK control-plane SQLite store (SDDK2-103).
/// The concrete implementation is `sddk_storage::SqliteControlPlane`.
pub trait ControlPlane {
    /// Returns true if the control-plane store file exists and is readable.
    fn store_exists(&self) -> bool;

    /// Inserts a discovered project (idempotent via INSERT OR IGNORE).
    fn upsert_project(
        &mut self,
        project_id: &str,
        display_name: &str,
        scope: &str,
        remote_url: Option<&str>,
        now: &str,
    ) -> Result<(), StorageError>;

    /// Inserts or replaces a `MetricsRecord` by `cycle_id`.
    fn upsert_cycle(
        &mut self,
        project_id: &str,
        record: &MetricsRecord,
    ) -> Result<(), StorageError>;

    /// Inserts or replaces the aggregate for a rolling window.
    fn upsert_aggregate(
        &mut self,
        window_days: u16,
        computed_at: &str,
        payload_json: &str,
    ) -> Result<(), StorageError>;

    /// Inserts or replaces a `UatResultRow`.
    fn upsert_uat_result(&mut self, result: &UatResultRow) -> Result<(), StorageError>;

    /// Loads all persisted `MetricsRecord` rows.
    fn load_cycles(&self) -> Result<Vec<MetricsRecord>, StorageError>;

    /// Loads all persisted `UatResultRow` rows.
    fn load_uat_results(&self) -> Result<Vec<UatResultRow>, StorageError>;
}

// ── Event-store port ──────────────────────────────────────────────────────────

/// Proof-of-success receipt returned by [`EventStore::append`].
///
/// The `content_hash` mirrors the value already stored in the database; the
/// adapter does NOT recompute it. Callers are expected to have built it via
/// [`EventEnvelopeV1::compute_content_hash`](crate::event_envelope::EventEnvelopeV1::compute_content_hash)
/// before calling `append`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventAppended {
    /// Globally unique event identifier.
    pub event_id: String,
    /// Stream this event belongs to.
    pub stream_id: String,
    /// Monotonic sequence number assigned within the stream at append time.
    pub sequence: u64,
    /// SHA-256 content hash — identical to `EventEnvelopeV1::content_hash`.
    pub content_hash: String,
    /// Wall-clock time when the event was recorded (RFC 3339).
    pub recorded_at: String,
    /// SHA-256 chain hash linking this event to the previous one.
    /// `chain_hash[0] = SHA256(content_hash || "genesis")`
    /// `chain_hash[N] = SHA256(content_hash[N] || chain_hash[N-1])`
    pub chain_hash: String,
}

/// Append-only event store for [`EventEnvelopeV1`] envelopes.
///
/// This trait is intentionally separate from [`Ledger`]. The `Ledger` trait
/// covers cycle/lease/gate_receipt/project bookkeeping that lives in the
/// `ledger_events` table (legacy). This trait covers the Common Event Protocol
/// v1 substrate that lives in `events_v1`.
///
/// Implementations MUST:
/// - Validate `content_hash` format (`sha256:<64-hex>`).
/// - Allocate sequence numbers per-stream under a transaction.
/// - Reject updates/deletes (enforced via SQL triggers on the storage side).
///
/// Error responses use [`StorageError::Other`] with a stable `event_store:<code>`
/// prefix contract:
/// - `event_store:content_hash_mismatch` — content hash does not match recomputed value
/// - `event_store:invalid_content_hash` — hash missing `sha256:` prefix or wrong length
/// - `event_store:invalid_event_type` — event_type failed validation
/// - `event_store:hash_drift:<seq>` — stored hash differs from recomputed at given sequence
pub trait EventStore {
    /// Appends an event envelope to the store, assigning a per-stream sequence number.
    ///
    /// The caller's `envelope.content_hash` MUST match `envelope.compute_content_hash()`
    /// and MUST start with `sha256:` before this method is called.
    ///
    /// Idempotency: re-appending the same `event_id` returns the original
    /// `EventAppended` (same `sequence`, same `recorded_at`) without allocating
    /// a new sequence.
    fn append(
        &mut self,
        envelope: &crate::event_envelope::EventEnvelopeV1,
    ) -> Result<EventAppended, StorageError>;

    /// Loads a single event by its global `event_id`.
    fn load_by_event_id(
        &self,
        event_id: &str,
    ) -> Result<Option<crate::event_envelope::EventEnvelopeV1>, StorageError>;

    /// Loads a contiguous range of events from one stream.
    ///
    /// Events are returned in ascending `sequence` order. `after_sequence`
    /// filters out events `≤` the supplied value (`None` = start from sequence 1).
    /// `limit` caps the result set.
    fn load_stream(
        &self,
        stream_id: &str,
        after_sequence: Option<u64>,
        limit: u32,
    ) -> Result<Vec<crate::event_envelope::EventEnvelopeV1>, StorageError>;

    /// Returns the highest allocated sequence number for a stream, or `None`
    /// when the stream has never received an event.
    fn last_sequence(&self, stream_id: &str) -> Result<Option<u64>, StorageError>;

    /// Returns the total number of events across all streams.
    fn count(&self) -> Result<u64, StorageError>;

    /// Returns the `content_hash` of the most-recently recorded event in a stream,
    /// or `None` when the stream is empty.
    fn head_hash(&self, stream_id: &str) -> Result<Option<String>, StorageError>;

    /// Returns the `chain_hash` of the most-recently recorded event in a stream,
    /// or `None` when the stream is empty.
    fn head_chain_hash(&self, stream_id: &str) -> Result<Option<String>, StorageError>;

    /// Verifies the cryptographic chain integrity of a stream.
    ///
    /// Loads every event in the stream and recomputes each
    /// [`EventEnvelopeV1::compute_content_hash`], comparing it against the stored
    /// `content_hash` column. Returns `Ok(())` when all hashes match; returns
    /// `Err(StorageError::Other("event_store:hash_drift:<seq>"))` on first mismatch.
    fn verify_stream_chain(&self, stream_id: &str) -> Result<(), StorageError>;

    /// Verifies the stream hash chain integrity.
    ///
    /// Loads every event in the stream in sequence order and recomputes each
    /// `chain_hash`:
    /// - `chain_hash[0] = SHA256(content_hash[0] || "genesis")`
    /// - `chain_hash[N] = SHA256(content_hash[N] || chain_hash[N-1])`
    ///
    /// Returns `Ok(())` when all chain hashes match; returns
    /// `Err(StorageError::Other("event_store:chain_drift:<seq>"))` on first mismatch.
    fn verify_chain_integrity(&self, stream_id: &str) -> Result<(), StorageError>;

    /// Loads a single event by stream identifier and sequence number.
    fn load_by_sequence(
        &self,
        stream_id: &str,
        sequence: u64,
    ) -> Result<Option<crate::event_envelope::EventEnvelopeV1>, StorageError>;
}

/// Graph store port (SPEC-004 §2). The graph is a projection — the ledger is
/// the authority; this store only persists the derived snapshot.
pub trait GraphStore {
    /// Persists the full graph state snapshot (upsert).
    fn save_state(&mut self, state: &crate::graph::GraphState) -> Result<(), StorageError>;

    /// Loads the persisted graph state, or `None` when never saved.
    fn load_state(&self) -> Result<Option<crate::graph::GraphState>, StorageError>;

    /// Returns the persisted checkpoint for the graph projection.
    fn checkpoint(&self) -> Result<Option<crate::projections::Checkpoint>, StorageError>;
}
