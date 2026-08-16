//! Hexagonal persistence port (Phase 1 M1 exit).
//!
//! `sddk_engine` depends only on this trait; `sddk_storage::Storage` is the
//! concrete SQLite implementation. The trait is object-safe (no associated
//! `Self` fns, no `Self` in generics) so `&dyn Ledger` is usable from the
//! engine's accessor.

use crate::models::*;
use crate::StorageError;

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
    fn get_project_optional(&self, project_id: &str) -> Result<Option<ProjectRecord>, StorageError>;
    /// Loads a workspace when present.
    fn get_workspace_optional(&self, workspace_id: &str) -> Result<Option<WorkspaceRecord>, StorageError>;
    /// Reports whether the database contains any project registration.
    fn has_projects(&self) -> Result<bool, StorageError>;
    /// Registers a project and workspace in one SQLite transaction.
    fn register_project_workspace(
        &mut self,
        project: &ProjectRecord,
        workspace: &WorkspaceRecord,
    ) -> Result<(), StorageError>;
}
