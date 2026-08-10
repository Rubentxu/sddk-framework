//! sddk-domain — Core domain types for SDDK
//!
//! This crate contains the canonical domain model for SDDK workflows,
//! including project identity, cycle state machines, and artifact references.

#![forbid(unsafe_code)]
#![deny(clippy::all)]
#![warn(missing_docs)]

pub mod cycle;
pub mod error;
pub mod identity;
pub mod legacy;
pub mod metrics;
pub mod pack;
pub mod schema;
pub mod uat;
pub mod workflow;

pub use cycle::*;
pub use error::*;
pub use identity::*;
pub use legacy::*;
pub use metrics::*;
pub use pack::*;
pub use schema::*;
pub use uat::*;
pub use workflow::*;
