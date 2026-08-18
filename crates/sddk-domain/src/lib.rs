//! sddk-domain — Core domain types for SDDK
//!
//! This crate contains the canonical domain model for SDDK workflows,
//! including project identity, cycle state machines, and artifact references.

#![forbid(unsafe_code)]
#![deny(clippy::all)]
#![warn(missing_docs)]

pub mod cycle;
pub mod error;
pub mod event_envelope;
pub mod evidence;
pub mod graph;
pub mod identity;
pub mod legacy;
pub mod metrics;
pub mod models;
pub mod pack;
pub mod ports;
pub mod projections;
pub mod proposal;
pub mod rules;
pub mod schema;
pub mod uat;
pub mod workflow;

pub use cycle::*;
pub use error::*;
pub use event_envelope::*;
pub use evidence::*;
pub use graph::*;
pub use identity::*;
pub use legacy::*;
pub use metrics::*;
pub use models::*;
pub use pack::*;
pub use ports::{ControlPlane, EventAppended, EventStore, GraphStore, Ledger};
pub use projections::{
    Checkpoint, CycleState, CycleStateProjection, Projection, ProjectionError, ProjectionVersion,
};
pub use rules::*;
pub use schema::*;
pub use uat::*;
pub use workflow::*;
