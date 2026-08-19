//! sddk-domain — Core domain types for SDDK
//!
//! This crate contains the canonical domain model for SDDK workflows,
//! including project identity, cycle state machines, and artifact references.

#![forbid(unsafe_code)]
#![deny(clippy::all)]
#![warn(missing_docs)]

pub mod channel;
pub mod context_read;
pub mod cycle;
pub mod error;
pub mod event_envelope;
pub mod evidence;
pub mod fork;
pub mod graph;
pub mod identity;
pub mod legacy;
pub mod metrics;
pub mod models;
pub mod pack;
pub mod ports;
pub mod projections;
pub mod proposal;
pub mod replay;
pub mod rules;
pub mod schema;
pub mod staleness;
pub mod uat;
pub mod view;
pub mod workflow;

pub use channel::*;
pub use context_read::*;
pub use cycle::*;
pub use error::*;
pub use event_envelope::*;
pub use evidence::*;
pub use fork::*;
pub use graph::*;
pub use identity::*;
pub use legacy::*;
pub use metrics::*;
pub use models::*;
pub use pack::*;
pub use ports::{
    ArtifactStore, ControlPlane, EventAppended, EventStore, GraphStore, Ledger, LedgerFactory,
};
pub use projections::{
    Checkpoint, CycleState, CycleStateProjection, Projection, ProjectionError, ProjectionVersion,
};
pub use replay::*;
pub use rules::*;
pub use schema::*;
pub use staleness::*;
pub use uat::*;
pub use view::*;
pub use workflow::*;
