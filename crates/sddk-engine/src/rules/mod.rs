//! Baseline consumer + stub evaluator for the architecture-rule registry.

pub mod baseline;
pub mod evaluators;

pub use baseline::{Baseline, BaselineConsumer, BaselineError, CrossCrateImport};
pub use evaluators::evaluate_all;

pub const EVALUATOR_VERSION: &str = evaluators::EVALUATOR_VERSION;
