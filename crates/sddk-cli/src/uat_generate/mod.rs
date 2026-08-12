//! E14.5 — Generate pipeline: requirements → UAT plan.
//!
//! Orchestrates: optional discover → plan → enrich → quality → approval → validate.
//! All inputs validated BEFORE any file write (atomic write rule).
//!
//! # Module structure
//!
//! - `parsing.rs` — text parsing utilities (criteria extraction, changelog parsing)
//! - `validator.rs` — input validation (requirements, changelog, discover)
//! - `planner/` — pure planner with merge and build submodules
//! - `runner/` — pipeline orchestration with injectable ApprovalIo
//! - `tests.rs` — integration and unit tests for the generate pipeline

pub mod parsing;
pub mod planner;
pub mod runner;
pub mod tests;
pub mod validator;

// Re-exports for convenience
#[allow(unused)]
pub use planner::{PlanError, PlanOutput, build_plan};
#[allow(unused)]
pub use runner::{PipelineConfig, PipelineError, StageOutput, run_pipeline};
#[allow(unused)]
pub use validator::{ValidateError, validate_inputs};
