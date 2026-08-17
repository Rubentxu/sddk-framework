//! Real evaluators for ARCH001..005 (Phase 1).

// `missing_docs` is allowed across this file because the Phase 1 ARCH
// evaluators were introduced before the workspace-wide
// `#![warn(missing_docs)]` activation. A future docs-pass cycle should
// restore the per-item `///` doc comments and remove this allow.
#![allow(missing_docs)]

use sddk_domain::{EvaluatorKind, RuleEvaluation, RuleRegistry, RuleStatus};
use serde_json::json;

use super::Baseline;

pub const EVALUATOR_VERSION: &str = "0.1.0";

/// Evaluates every registered rule against the baseline (Phase 1).
///
/// Waiver precedence: if a waiver exists and `baseline.ref_.head_anchor <=
/// w.granted_until_sha`, the evaluation is overridden to `Waived`.
/// Expired waivers (head_anchor > granted_until_sha) result in `NotApplicable`
/// to preserve Phase 0 backward compatibility with existing waivers in the registry.
pub fn evaluate_all(
    registry: &RuleRegistry,
    baseline: &Baseline,
    evaluated_at: &str,
) -> Vec<RuleEvaluation> {
    registry
        .iter()
        .map(|rule| {
            // ── Waiver pre-check ──────────────────────────────────────────────
            if let Some(w) = registry.waiver_for(&rule.id) {
                if baseline.ref_.head_anchor <= w.granted_until_sha {
                    return RuleEvaluation {
                        rule_id: rule.id.clone(),
                        status: RuleStatus::Waived,
                        observed: json!({
                            "waiver_id": w.id,
                            "reason": w.reason
                        }),
                        baseline_sha256: baseline.ref_.sha256.clone(),
                        evaluated_at: evaluated_at.to_owned(),
                        evaluated_by: format!("sddk-rules-cli@{EVALUATOR_VERSION}"),
                        waiver_id: Some(w.id.clone()),
                        evaluator_kind: EvaluatorKind::Schema,
                        evaluator_version: EVALUATOR_VERSION.to_owned(),
                        provenance: None,
                    };
                }
                // Waiver expired → NotApplicable (Phase 0 backward compat)
                return RuleEvaluation {
                    rule_id: rule.id.clone(),
                    status: RuleStatus::NotApplicable,
                    observed: json!({ "phase": "phase0", "rule_id": rule.id }),
                    baseline_sha256: baseline.ref_.sha256.clone(),
                    evaluated_at: evaluated_at.to_owned(),
                    evaluated_by: format!("sddk-rules-cli@{EVALUATOR_VERSION}"),
                    waiver_id: None,
                    evaluator_kind: EvaluatorKind::Schema,
                    evaluator_version: EVALUATOR_VERSION.to_owned(),
                    provenance: Some(format!(
                        "waiver {} expired at baseline {}",
                        w.id, baseline.ref_.head_anchor
                    )),
                };
            }

            // ── Rule-specific evaluation ─────────────────────────────────────
            match rule.id.as_str() {
                "ARCH001" => evaluate_arch001(rule, baseline, evaluated_at),
                "ARCH002" => evaluate_arch002(rule, baseline, evaluated_at),
                "ARCH003" => evaluate_arch003(rule, baseline, evaluated_at),
                "ARCH004" => evaluate_arch004(rule, baseline, evaluated_at),
                "ARCH005" => evaluate_arch005(rule, baseline, evaluated_at),
                _ => RuleEvaluation {
                    rule_id: rule.id.clone(),
                    status: RuleStatus::NotApplicable,
                    observed: json!({}),
                    baseline_sha256: baseline.ref_.sha256.clone(),
                    evaluated_at: evaluated_at.to_owned(),
                    evaluated_by: format!("sddk-rules-cli@{EVALUATOR_VERSION}"),
                    waiver_id: None,
                    evaluator_kind: EvaluatorKind::Schema,
                    evaluator_version: EVALUATOR_VERSION.to_owned(),
                    provenance: Some(format!("evaluator not implemented for {}", rule.id)),
                },
            }
        })
        .collect()
}

// ── ARCH001 ──────────────────────────────────────────────────────────────────

/// engine_must_not_depend_on_storage: Fail if any edge from sddk-engine to sddk-storage
/// exists in the baseline (Cargo dep or use statement).
fn evaluate_arch001(
    rule: &sddk_domain::ArchitectureRule,
    baseline: &Baseline,
    evaluated_at: &str,
) -> RuleEvaluation {
    let violating: Vec<_> = baseline
        .cross_crate_imports
        .iter()
        .filter(|e| e.from_crate == "sddk-engine" && e.to_crate == "sddk-storage")
        .map(|e| {
            json!({
                "from_file": e.from_file,
                "line": e.line,
                "kind": e.kind,
            })
        })
        .collect();

    let status = if violating.is_empty() {
        RuleStatus::Pass
    } else {
        RuleStatus::Fail
    };

    RuleEvaluation {
        rule_id: rule.id.clone(),
        status,
        observed: json!({
            "edges": violating,
            "count": violating.len(),
        }),
        baseline_sha256: baseline.ref_.sha256.clone(),
        evaluated_at: evaluated_at.to_owned(),
        evaluated_by: format!("sddk-rules-cli@{EVALUATOR_VERSION}"),
        waiver_id: None,
        evaluator_kind: EvaluatorKind::Schema,
        evaluator_version: EVALUATOR_VERSION.to_owned(),
        provenance: Some(
            "ARCH001 live evaluator: checks sddk-engine→sddk-storage edges in \
             cross_crate_imports (Cargo deps + use statements)"
                .to_owned(),
        ),
    }
}

// ── ARCH002 ──────────────────────────────────────────────────────────────────

/// domain_must_not_depend_on_adapters: Fail if any edge from sddk-domain to
/// sddk-storage, sddk-gateway, or sddk-cli exists.
fn evaluate_arch002(
    rule: &sddk_domain::ArchitectureRule,
    baseline: &Baseline,
    evaluated_at: &str,
) -> RuleEvaluation {
    let forbidden = ["sddk-storage", "sddk-gateway", "sddk-cli"];
    let violating: Vec<_> = baseline
        .cross_crate_imports
        .iter()
        .filter(|e| e.from_crate == "sddk-domain" && forbidden.contains(&e.to_crate.as_str()))
        .map(|e| {
            json!({
                "from_file": e.from_file,
                "line": e.line,
                "kind": e.kind,
            })
        })
        .collect();

    let status = if violating.is_empty() {
        RuleStatus::Pass
    } else {
        RuleStatus::Fail
    };

    RuleEvaluation {
        rule_id: rule.id.clone(),
        status,
        observed: json!({
            "edges": violating,
            "count": violating.len(),
        }),
        baseline_sha256: baseline.ref_.sha256.clone(),
        evaluated_at: evaluated_at.to_owned(),
        evaluated_by: format!("sddk-rules-cli@{EVALUATOR_VERSION}"),
        waiver_id: None,
        evaluator_kind: EvaluatorKind::Schema,
        evaluator_version: EVALUATOR_VERSION.to_owned(),
        provenance: Some(
            "ARCH002 live evaluator: checks sddk-domain→{storage,gateway,cli} edges".to_owned(),
        ),
    }
}

// ── ARCH003 ──────────────────────────────────────────────────────────────────

/// cli_must_not_own_persistence_logic: Fail if any edge from sddk-cli to sddk-storage
/// exists (import-level proxy for "no SQL/direct persistence in CLI").
fn evaluate_arch003(
    rule: &sddk_domain::ArchitectureRule,
    baseline: &Baseline,
    evaluated_at: &str,
) -> RuleEvaluation {
    let violating: Vec<_> = baseline
        .cross_crate_imports
        .iter()
        .filter(|e| e.from_crate == "sddk-cli" && e.to_crate == "sddk-storage")
        .map(|e| {
            json!({
                "from_file": e.from_file,
                "line": e.line,
                "kind": e.kind,
            })
        })
        .collect();

    let status = if violating.is_empty() {
        RuleStatus::Pass
    } else {
        RuleStatus::Fail
    };

    RuleEvaluation {
        rule_id: rule.id.clone(),
        status,
        observed: json!({
            "edges": violating,
            "count": violating.len(),
        }),
        baseline_sha256: baseline.ref_.sha256.clone(),
        evaluated_at: evaluated_at.to_owned(),
        evaluated_by: format!("sddk-rules-cli@{EVALUATOR_VERSION}"),
        waiver_id: None,
        evaluator_kind: EvaluatorKind::Schema,
        evaluator_version: EVALUATOR_VERSION.to_owned(),
        provenance: Some(
            "ARCH003 live evaluator: import-level proxy for 'no persistence logic in CLI'"
                .to_owned(),
        ),
    }
}

// ── ARCH004 ──────────────────────────────────────────────────────────────────

/// packs_must_declare_dependencies: NotApplicable in the kernel repo
/// (Phase 4 pack-host substrate not shipped here).
fn evaluate_arch004(
    rule: &sddk_domain::ArchitectureRule,
    baseline: &Baseline,
    evaluated_at: &str,
) -> RuleEvaluation {
    RuleEvaluation {
        rule_id: rule.id.clone(),
        status: RuleStatus::NotApplicable,
        observed: json!({}),
        baseline_sha256: baseline.ref_.sha256.clone(),
        evaluated_at: evaluated_at.to_owned(),
        evaluated_by: format!("sddk-rules-cli@{EVALUATOR_VERSION}"),
        waiver_id: None,
        evaluator_kind: EvaluatorKind::Schema,
        evaluator_version: EVALUATOR_VERSION.to_owned(),
        provenance: Some(
            "kernel repo, not a pack host (Phase 4 substrate not shipped here)".to_owned(),
        ),
    }
}

// ── ARCH005 ──────────────────────────────────────────────────────────────────

/// reactive_behaviors_must_not_execute_governed_effects_directly:
/// NotApplicable until Phase 5 reactive runtime ships.
fn evaluate_arch005(
    rule: &sddk_domain::ArchitectureRule,
    baseline: &Baseline,
    evaluated_at: &str,
) -> RuleEvaluation {
    RuleEvaluation {
        rule_id: rule.id.clone(),
        status: RuleStatus::NotApplicable,
        observed: json!({}),
        baseline_sha256: baseline.ref_.sha256.clone(),
        evaluated_at: evaluated_at.to_owned(),
        evaluated_by: format!("sddk-rules-cli@{EVALUATOR_VERSION}"),
        waiver_id: None,
        evaluator_kind: EvaluatorKind::Schema,
        evaluator_version: EVALUATOR_VERSION.to_owned(),
        provenance: Some("Phase 5 reactive runtime not yet shipped".to_owned()),
    }
}
