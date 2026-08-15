//! Integration tests for the baseline consumer + stub evaluator.

use sddk_domain::{BaselineRef, RuleStatus};
use sddk_engine::rules::{
    Baseline, BaselineConsumer, BaselineError, CrossCrateImport, evaluate_all,
};
use std::path::PathBuf;

fn make_baseline(imports: Vec<(&str, u32, &str)>) -> Baseline {
    let cross_crate_imports = imports
        .into_iter()
        .map(|(from_file, line, to_crate)| {
            let parts: Vec<&str> = from_file.split('/').collect();
            let from_crate = if parts.len() >= 2 && parts[0] == "crates" {
                parts[1].to_owned()
            } else {
                "unknown".to_owned()
            };
            let to_crate = if to_crate.starts_with("sddk-") {
                to_crate.to_owned()
            } else {
                format!("sddk-{}", to_crate)
            };
            CrossCrateImport {
                from_file: from_file.to_owned(),
                line,
                from_crate,
                to_crate_raw: to_crate.to_owned(),
                to_crate,
            }
        })
        .collect();
    Baseline {
        ref_: BaselineRef {
            schema_version: "1.0.0".to_owned(),
            head_anchor: "1dd72d0".to_owned(),
            sha256: "sha256:test".to_owned(),
            cycle_id: None,
            captured_at: "2026-08-13T12:00:00Z".to_owned(),
        },
        cross_crate_imports,
    }
}

#[test]
fn baseline_consumer_rejects_unsupported_schema_version() {
    let json = r#"{"schema_version": "99.0.0", "head_anchor": "deadbeef", "captured_at": "2026-08-13T12:00:00Z", "cross_crate_coupling_baseline": {"cross_crate_imports": []}}"#;
    let tmp = tempfile::NamedTempFile::new().expect("tempfile");
    std::fs::write(tmp.path(), json).expect("write");
    let consumer = BaselineConsumer::new(tmp.path(), &["1.0.0"]).expect("constructor accepts");
    let err = consumer.load().expect_err("load should fail");
    match err {
        BaselineError::UnsupportedSchemaVersion { actual, .. } => assert_eq!(actual, "99.0.0"),
        other => panic!("expected UnsupportedSchemaVersion, got {other:?}"),
    }
}

#[test]
fn baseline_consumer_parses_and_normalizes_crates() {
    let json = r#"{"schema_version": "1.0.0", "head_anchor": "1dd72d0", "captured_at": "2026-08-13T12:00:00Z", "cross_crate_coupling_baseline": {"cross_crate_imports": [{"from_file": "crates/sddk-engine/src/lib.rs", "line": 23, "to_crate": "storage"}]}}"#;
    let tmp = tempfile::NamedTempFile::new().expect("tempfile");
    std::fs::write(tmp.path(), json).expect("write");
    let consumer = BaselineConsumer::new(tmp.path(), &["1.0.0"]).expect("constructor accepts");
    let baseline = consumer.load().expect("load should succeed");
    assert_eq!(baseline.ref_.schema_version, "1.0.0");
    assert_eq!(baseline.cross_crate_imports.len(), 1);
    let import = &baseline.cross_crate_imports[0];
    assert_eq!(import.from_crate, "sddk-engine");
    assert_eq!(import.to_crate, "sddk-storage");
    assert_eq!(import.to_crate_raw, "storage");
}

#[test]
fn evaluate_all_returns_not_applicable_for_all_rules() {
    let yaml = r#"schema_version: 1.0.0
rules:
  - id: ARCH001
    severity: error
    rule: engine_must_not_depend_on_storage
    target: dependency_graph
  - id: ARCH004
    severity: error
    rule: packs_must_declare_dependencies
    target: pack_manifest
"#;
    let registry = sddk_domain::RuleRegistry::from_yaml_str(yaml).expect("parse succeeds");
    let baseline = make_baseline(vec![]);
    let results = evaluate_all(&registry, &baseline, "2026-08-13T12:00:00Z");
    assert_eq!(results.len(), 2);
    for r in &results {
        assert_eq!(
            r.status,
            RuleStatus::NotApplicable,
            "ARCH{} should be NotApplicable",
            &r.rule_id[4..]
        );
        assert!(r.provenance.is_some(), "every rule needs provenance");
    }
}

#[test]
fn evaluate_all_applies_waiver_when_head_anchor_within_granted_sha() {
    let yaml = r#"schema_version: 1.0.0
rules:
  - id: ARCH001
    severity: error
    rule: x
    target: dependency_graph
waivers:
  - id: WV-0001
    rule_id: ARCH001
    reason: "transitive dep in flight"
    granted_until_sha: "1dd72d0"
    granted_by: "reviewer"
    granted_at: "2026-08-13T12:00:00Z"
"#;
    let registry = sddk_domain::RuleRegistry::from_yaml_str(yaml).expect("parse succeeds");
    let baseline = make_baseline(vec![]); // head_anchor = "1dd72d0"
    let results = evaluate_all(&registry, &baseline, "2026-08-13T12:00:00Z");
    assert_eq!(results.len(), 1);
    let r = &results[0];
    assert_eq!(r.status, RuleStatus::Waived);
    assert_eq!(r.waiver_id.as_deref(), Some("WV-0001"));
}

#[test]
fn evaluate_all_returns_not_applicable_when_waiver_expired() {
    let yaml = r#"schema_version: 1.0.0
rules:
  - id: ARCH001
    severity: error
    rule: x
    target: dependency_graph
waivers:
  - id: WV-0001
    rule_id: ARCH001
    reason: "old waiver"
    granted_until_sha: "00001111"
    granted_by: "reviewer"
    granted_at: "2026-08-13T12:00:00Z"
"#;
    let registry = sddk_domain::RuleRegistry::from_yaml_str(yaml).expect("parse succeeds");
    let baseline = make_baseline(vec![]); // head_anchor = "1dd72d0" > "00001111"
    let results = evaluate_all(&registry, &baseline, "2026-08-13T12:00:00Z");
    assert_eq!(results.len(), 1);
    let r = &results[0];
    assert_eq!(r.status, RuleStatus::NotApplicable);
    assert!(r.waiver_id.is_none());
    assert!(r.provenance.as_ref().unwrap().contains("expired"));
}

#[test]
fn shipped_catalog_parses_with_five_rules() {
    // Regression: shipped architecture-rules.yaml must parse with ARCH001..ARCH005 only.
    let yaml_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/sddk-2.0-architecture-consolidation/data/architecture-rules.yaml");
    let yaml = std::fs::read_to_string(&yaml_path).expect("shipped YAML must be readable");
    let registry = sddk_domain::RuleRegistry::from_yaml_str(&yaml)
        .expect("shipped YAML must parse with 5 rules");
    let ids: Vec<&str> = registry.iter().map(|r| r.id.as_str()).collect();
    assert_eq!(
        ids,
        vec!["ARCH001", "ARCH002", "ARCH003", "ARCH004", "ARCH005"]
    );
}

#[test]
fn shipped_catalog_against_baseline_produces_five_evaluations() {
    // Regression: shipped YAML + baseline produces 5 evaluations (ARCH001..ARCH005),
    // all with status=NotApplicable, all with provenance.
    let yaml_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/sddk-2.0-architecture-consolidation/data/architecture-rules.yaml");
    let yaml = std::fs::read_to_string(&yaml_path).expect("shipped YAML must be readable");
    let registry =
        sddk_domain::RuleRegistry::from_yaml_str(&yaml).expect("shipped YAML must parse");

    let baseline_path = dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("~/.local/share"))
        .join("sddk/projects/p-52b95ef55999f9de/cycle-artifacts/p-52b95ef55999f9de/sddk-2-0-phase0-baseline/baseline-dependency-entropy.json");
    let consumer = BaselineConsumer::new(&baseline_path, &["1.0.0"])
        .expect("baseline consumer must be created");
    let baseline = consumer.load().expect("baseline must load");

    let results = evaluate_all(&registry, &baseline, "2026-08-13T12:00:00Z");
    assert_eq!(
        results.len(),
        5,
        "shipped catalog must produce 5 evaluations"
    );
    for r in &results {
        assert_eq!(r.status, RuleStatus::NotApplicable);
        assert!(r.provenance.is_some(), "every rule needs provenance");
    }
}
