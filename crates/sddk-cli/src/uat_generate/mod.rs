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
pub use planner::{build_plan, PlanError, PlanOutput};
pub use runner::{run_pipeline, PipelineConfig, PipelineError, StageOutput};
pub use validator::{validate_inputs, ValidateError};
