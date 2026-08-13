//! Architecture-rules registry: domain types + YAML loader.

pub mod registry;
pub mod types;

pub use registry::{RegistryError, RuleRegistry};
pub use types::{ArchitectureRule, BaselineRef, EvaluatorKind, RuleEvaluation, RuleSeverity, RuleStatus, RuleTarget, Waiver};
pub const ARCHITECTURE_RULES_SCHEMA_VERSION: &str = "1.0.0";
