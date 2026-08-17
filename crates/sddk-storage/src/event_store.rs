//! SQLite-backed [`sddk_domain::EventStore`] adapter for the Common Event Protocol v1.
//!
//! Persists [`sddk_domain::EventEnvelopeV1`] to the `events_v1` table. Co-exists
//! with `ledger_events` (legacy ledger bookkeeping) — they are independent tables
//! within the same `ledger.sqlite` file.
//!
//! ## Stable error prefix contract (R2)
//!
//! All error responses use [`sddk_domain::StorageError::Other`] with a stable
//! `event_store:<code>` prefix:
//!
//! | Code | Meaning |
//! |------|---------|
//! | `event_store:content_hash_mismatch` | `content_hash` != recomputed hash |
//! | `event_store:invalid_content_hash` | missing `sha256:` prefix or wrong length |
//! | `event_store:invalid_event_type` | event_type failed validation |
//! | `event_store:hash_drift:<seq>` | stored hash differs from recomputed at sequence |
//!
//! ## Connection model (R6)
//!
//! Each `SqliteEventStore` instance owns its own `rusqlite::Connection` to
//! `ledger.sqlite`. This is a second connection separate from `Storage`'s
//! connection — both connect to the same file. They serialize writers via
//! `busy_timeout=5s` + WAL mode. A future cycle (SDDK2-203/204) that needs
//! cross-table atomic transactions between `ledger_events` and `events_v1`
//! will need to merge the two connections into one.

#![allow(dead_code, unused_imports)]

use std::path::Path;
use std::time::Duration;

use rusqlite::Connection;
use sddk_domain::{EventAppended, EventEnvelopeV1, EventStore, StorageError as DomainStorageError};

/// SQLite-backed [`EventStore`] implementation.
pub struct SqliteEventStore {
    conn: Connection,
}

impl SqliteEventStore {
    /// Opens (or creates) a `ledger.sqlite` file at `$dir/ledger.sqlite` and
    /// applies all pending migrations.
    ///
    /// Same WAL + busy-timeout + FK pragma policy as [`Storage::open`].
    pub fn open(dir: &Path) -> Result<Self, DomainStorageError> {
        let path = dir.join("ledger.sqlite");
        let conn = Connection::open(&path)
            .map_err(|e| DomainStorageError::Database(format!("open: {e}")))?;
        conn.busy_timeout(Duration::from_secs(5))
            .map_err(|e| DomainStorageError::Database(format!("busy_timeout: {e}")))?;
        conn.pragma_update(None, "foreign_keys", true)
            .map_err(|e| DomainStorageError::Database(format!("foreign_keys: {e}")))?;
        conn.pragma_update(None, "journal_mode", "WAL")
            .map_err(|e| DomainStorageError::Database(format!("journal_mode: {e}")))?;
        let mut conn = conn;
        Self::run_migrations(&mut conn)?;
        Ok(Self { conn })
    }

    /// Opens an isolated in-memory database with all migrations applied.
    /// Useful for tests.
    pub fn open_in_memory() -> Result<Self, DomainStorageError> {
        let conn = Connection::open_in_memory()
            .map_err(|e| DomainStorageError::Database(format!("open_in_memory: {e}")))?;
        let mut conn = conn;
        Self::run_migrations(&mut conn)?;
        Ok(Self { conn })
    }

    fn run_migrations(conn: &mut Connection) -> Result<(), DomainStorageError> {
        let version: i32 = conn
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .map_err(|e| DomainStorageError::Database(e.to_string()))?;
        if version < 5 {
            let tx = conn
                .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
                .map_err(|e| DomainStorageError::Database(e.to_string()))?;
            tx.execute_batch(crate::migrations::MIGRATION_5)
                .map_err(|e| DomainStorageError::Database(e.to_string()))?;
            tx.pragma_update(None, "user_version", 5)
                .map_err(|e| DomainStorageError::Database(e.to_string()))?;
            tx.commit()
                .map_err(|e| DomainStorageError::Database(e.to_string()))?;
        }
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn connection(&self) -> &Connection {
        &self.conn
    }
}
