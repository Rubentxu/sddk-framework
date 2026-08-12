//! E14.5 — Generate pipeline: requirements → UAT plan.
//!
//! Orchestrates: optional discover → plan → enrich → quality → approval → validate.
//! All inputs validated BEFORE any file write (atomic write rule).
//!
//! # Module structure
//!
//! - `validator.rs` — input validation (requirements, changelog, discover)
//! - `planner.rs` — pure planner (consumes requirements + changelog + AAM)
//! - `runner.rs` — pipeline orchestration with injectable ApprovalIo
//! - `tests.rs` — integration tests

pub mod planner;
pub mod runner;
pub mod tests;
pub mod validator;

// Re-export for convenience
#[allow(unused)]
pub use planner::{PlanError, PlanOutput, build_plan};
#[allow(unused)]
pub use runner::{PipelineConfig, PipelineError, StageOutput, run_pipeline};
#[allow(unused)]
pub use validator::{ValidateError, validate_inputs};
