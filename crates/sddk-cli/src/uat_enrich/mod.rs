//! E14.3 — UX Form Agent: semantic form enrichment with deterministic rules.
//!
//! Design: `design.md §Decision: enrich semantic transform`.
//! Spec: `REQ-E14-EnrichForms-Semantic-Transform`.
//!
//! Decision tree for scenarios without existing form:
//! - Machine check: HTTP/API/DOM/JSON criteria → `UatFormOracleKind::Http/Json/Dom`
//! - Rating: UX subjective criteria → `UatFormInputKind::Rating` with scale anchors
//! - Blind observation: expected textual observable → `UatFormVisibility::Blind`
//! - Human confirmation: fallback when no other rule applies
//! - Checkpoint: every 5 items when total > 5
//! - P0/P1: blocking checks require `[Screenshot]` evidence
//! - Provenance: `UatProvenance` with `generated_by: "uat-ux-form"`, additive fields

mod rules;

#[cfg(test)]
mod tests;

// Re-export domain types for tests
#[allow(unused_imports)]
pub use sddk_domain::{
    UatFormCheck, UatFormElementKind as FEK, UatFormEvidenceKind as FEVK, UatFormInputKind as FIK,
    UatFormItem, UatFormOracleKind as FOK, UatFormSpec, UatFormVisibility as FVIS, UatPriority,
};

/// Build a default form for a scenario using deterministic enrichment rules.
/// Returns the existing form if the scenario already has one (preservation rule).
pub fn build_default_form(scenario: &sddk_domain::UatScenario) -> UatFormSpec {
    if scenario.form.is_some() {
        // Preservation rule: don't overwrite existing forms
        return scenario.form.clone().unwrap();
    }

    rules::build_form_for_scenario(scenario)
}
