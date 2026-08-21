//! Declarative gate evaluator for debt-report findings.
//!
//! Evaluates the 2 gates declared in the cycle-7b workflow contract:
//! - `debt-severity-assigned`: every finding has severity ∈ {critical, high, medium, low}
//! - `debt-priority-assigned`: every finding have priority ∈ {P0, P1, P2, P3}

use sddk_domain::{
    DebtReport, Finding, GateOutcomeStatus, Ledger, Severity,
};
use serde_json::Value;

/// Outcome of a gate evaluation (without persistence).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GateOutcome {
    /// The gate passed.
    Passed { notes: String },
    /// The gate failed — one or more findings violate the gate contract.
    Failed { offending_ids: Vec<String>, notes: String },
}

/// Valid severity values per schema.
const VALID_SEVERITIES: &[&str] = &["critical", "high", "medium", "low"];

/// Valid priority values per schema.
const VALID_PRIORITIES: &[&str] = &["P0", "P1", "P2", "P3"];

fn is_valid_severity(sev: &Severity) -> bool {
    let s = match sev {
        Severity::Critical => "critical",
        Severity::High => "high",
        Severity::Medium => "medium",
        Severity::Low => "low",
    };
    VALID_SEVERITIES.contains(&s)
}

fn is_valid_priority(pri: &sddk_domain::Priority) -> bool {
    let p = match pri {
        sddk_domain::Priority::P0 => "P0",
        sddk_domain::Priority::P1 => "P1",
        sddk_domain::Priority::P2 => "P2",
        sddk_domain::Priority::P3 => "P3",
    };
    VALID_PRIORITIES.contains(&p)
}

/// Evaluates a named gate against a debt report.
///
/// Returns `GateOutcome::Passed` if all findings satisfy the gate contract.
/// Returns `GateOutcome::Failed` with offending finding IDs otherwise.
pub fn evaluate_named_gate(name: &str, report: &DebtReport) -> GateOutcome {
    match name {
        "debt-severity-assigned" => evaluate_severity_gate(report),
        "debt-priority-assigned" => evaluate_priority_gate(report),
        other => GateOutcome::Failed {
            offending_ids: vec![],
            notes: format!("unknown gate: {other}"),
        },
    }
}

fn evaluate_severity_gate(report: &DebtReport) -> GateOutcome {
    let invalid: Vec<String> = report
        .findings
        .iter()
        .filter(|f| !is_valid_severity(&f.severity))
        .map(|f| f.id.clone())
        .collect();
    if invalid.is_empty() {
        GateOutcome::Passed {
            notes: format!("{} findings checked", report.findings.len()),
        }
    } else {
        GateOutcome::Failed {
            offending_ids: invalid,
            notes: "findings with invalid or missing severity".into(),
        }
    }
}

fn evaluate_priority_gate(report: &DebtReport) -> GateOutcome {
    let invalid: Vec<String> = report
        .findings
        .iter()
        .filter(|f| !is_valid_priority(&f.priority))
        .map(|f| f.id.clone())
        .collect();
    if invalid.is_empty() {
        GateOutcome::Passed {
            notes: format!("{} findings checked", report.findings.len()),
        }
    } else {
        GateOutcome::Failed {
            offending_ids: invalid,
            notes: "findings with invalid or missing priority".into(),
        }
    }
}

/// Converts a `GateOutcome` to the corresponding `GateOutcomeStatus`.
fn to_status(outcome: &GateOutcome) -> GateOutcomeStatus {
    match outcome {
        GateOutcome::Passed { .. } => GateOutcomeStatus::Passed,
        GateOutcome::Failed { .. } => GateOutcomeStatus::Failed,
    }
}

/// Builds the evidence JSON for a gate evaluation.
fn build_evidence(outcome: &GateOutcome) -> Value {
    match outcome {
        GateOutcome::Passed { notes } => serde_json::json!({ "notes": notes }),
        GateOutcome::Failed { offending_ids, notes } => {
            serde_json::json!({ "offending_ids": offending_ids, "notes": notes })
        }
    }
}

/// Evaluates a named gate and records the receipt via the engine.
///
/// This is the wiring entry point used by the CLI and orchestrator:
/// it calls `evaluate_named_gate` and then emits a `GateReceipt` through
/// `Engine::evaluate_gate` for durable storage.
pub fn evaluate_and_record<L: Ledger>(
    engine: &mut crate::Engine<L>,
    cycle_id: &str,
    transition_id: &str,
    gate_name: &str,
    report: &DebtReport,
    evaluator: &str,
    actor: &str,
    command_id: &str,
    evaluated_at: &str,
) -> Result<GateOutcome, crate::EngineError> {
    let outcome = evaluate_named_gate(gate_name, report);
    let status = to_status(&outcome);
    let evidence = build_evidence(&outcome);

    engine.evaluate_gate(&crate::GateEvaluationInput {
        cycle_id: cycle_id.into(),
        transition_id: transition_id.into(),
        gate: gate_name.into(),
        evaluator: evaluator.into(),
        evidence,
        outcome: status,
        evaluated_at: evaluated_at.into(),
        actor: actor.into(),
        command_id: command_id.into(),
    })?;
    Ok(outcome)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sddk_domain::{DebtReport, Finding, FindingStatus, Priority, Severity};

    fn valid_report() -> DebtReport {
        DebtReport {
            schema_version: "1.1.0".into(),
            cycle_id: "p-test/kernel-cycle-8".into(),
            generated_at: "2026-08-21T00:00:00Z".into(),
            findings: vec![
                Finding {
                    id: "FIND-0001".into(),
                    title: "Test".into(),
                    severity: Severity::Medium,
                    priority: Priority::P2,
                    status: FindingStatus::Open,
                    fingerprint: "3ef321c4efe1d87e".into(),
                    fingerprint_aliases: vec![],
                    cluster_id: "CL-01".into(),
                    category: "architecture".into(),
                    description: "Test finding".into(),
                    remediation_cycle: None,
                    remediation_pr: None,
                    evidence_refs: None,
                },
                Finding {
                    id: "FIND-0002".into(),
                    title: "Test 2".into(),
                    severity: Severity::Critical,
                    priority: Priority::P0,
                    status: FindingStatus::Open,
                    fingerprint: "efa9e569e7c7b602".into(),
                    fingerprint_aliases: vec![],
                    cluster_id: "CL-02".into(),
                    category: "risk".into(),
                    description: "Critical finding".into(),
                    remediation_cycle: None,
                    remediation_pr: None,
                    evidence_refs: None,
                },
            ],
        }
    }

    #[test]
    fn test_gate_severity_pass() {
        let report = valid_report();
        let outcome = evaluate_named_gate("debt-severity-assigned", &report);
        assert!(matches!(outcome, GateOutcome::Passed { .. }));
    }

    #[test]
    fn test_gate_priority_pass() {
        let report = valid_report();
        let outcome = evaluate_named_gate("debt-priority-assigned", &report);
        assert!(matches!(outcome, GateOutcome::Passed { .. }));
    }

    #[test]
    fn test_gate_unknown() {
        let report = valid_report();
        let outcome = evaluate_named_gate("unknown-gate", &report);
        match &outcome {
            GateOutcome::Failed { notes, .. } => {
                assert!(notes.contains("unknown gate"));
            }
            _ => panic!("expected Failed for unknown gate"),
        }
    }

    #[test]
    fn test_gate_empty_report() {
        let report = DebtReport {
            schema_version: "1.1.0".into(),
            cycle_id: "p-test/kernel-cycle-8".into(),
            generated_at: "2026-08-21T00:00:00Z".into(),
            findings: vec![],
        };
        // Empty findings means no violations → both gates pass
        let outcome = evaluate_named_gate("debt-severity-assigned", &report);
        assert!(matches!(outcome, GateOutcome::Passed { .. }));
        let outcome = evaluate_named_gate("debt-priority-assigned", &report);
        assert!(matches!(outcome, GateOutcome::Passed { .. }));
    }
}
