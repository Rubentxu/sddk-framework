//! Shared types and traits for UAT subcommands.
//! Re-exports from domain and defines injectable interfaces.

pub mod io;
pub mod plan_io;
pub mod time;

// Used by uat_quality::run
pub use plan_io::read_plan;
