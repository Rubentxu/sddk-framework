//! Stub evaluator for ARCH001..005 (Phase 0 observational only).

use sddk_domain::{EvaluatorKind, RuleEvaluation, RuleRegistry, RuleStatus};
use serde_json::json;

use super::Baseline;

pub const EVALUATOR_VERSION: &str = "0.1.0";

/// Evaluates every registered rule against the baseline (Phase 0 stub).
/// Emits `NotApplicable` for all rules; applies waivers where
/// `baseline.ref_.head_anchor <= waiver.granted_until_sha`.
pub fn evaluate_all(registry: &RuleRegistry, baseline: &Baseline, evaluated_at: &str) -> Vec<RuleEvaluation> {
    registry.iter().map(|rule| {
        let waiver = registry.waiver_for(&rule.id);
        let (status, waiver_id, observed, provenance) = match waiver {
            Some(w) if baseline.ref_.head_anchor <= w.granted_until_sha => (
                RuleStatus::Waived, Some(w.id.clone()), json!({ "waiver_id": w.id, "reason": w.reason }), None,
            ),
            Some(w) => (
                RuleStatus::NotApplicable, None, json!({ "phase": "phase0", "rule_id": rule.id }),
                Some(format!("waiver {} expired at baseline {}", w.id, baseline.ref_.head_anchor)),
            ),
            None => (
                RuleStatus::NotApplicable, None, json!({ "phase": "phase0", "rule_id": rule.id }), None,
            ),
        };
        let provenance = provenance.or_else(|| provenance_for(&rule.id));
        RuleEvaluation {
            rule_id: rule.id.clone(), status, observed, baseline_sha256: baseline.ref_.sha256.clone(),
            evaluated_at: evaluated_at.to_owned(), evaluated_by: format!("sddk-rules-cli@{EVALUATOR_VERSION}"),
            waiver_id, evaluator_kind: EvaluatorKind::Schema, evaluator_version: EVALUATOR_VERSION.to_owned(), provenance,
        }
    }).collect()
}

fn provenance_for(rule_id: &str) -> Option<String> {
    match rule_id {
        "ARCH001" | "ARCH002" | "ARCH003" => Some("ARCH00X detailed evaluator deferred to WI-4 (SDDK2-003.M1)".to_owned()),
        "ARCH004" | "ARCH005" => Some("ARCH00X detailed evaluator deferred to WI-5 (SDDK2-003.M5)".to_owned()),
        _ => Some("out of BACKLOG §SDDK2-003 scope".to_owned()),
    }
}
