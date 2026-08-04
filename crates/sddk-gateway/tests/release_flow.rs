//! Release flow integration tests: plan, idempotent apply, reconciliation.

use sddk_gateway::{
    CapabilityGateway, CapabilityPlanInput, CapabilityPolicy, Forge, MockForge, ReleasePlanInput,
    apply_release, plan_release, reconcile_pending,
};
use sddk_storage::{CapabilityStatus, ProjectRecord, Storage};

const WORKFLOW_YAML: &str = include_str!("../../../workflow/workflow.yaml");

fn gateway() -> (tempfile::TempDir, CapabilityGateway) {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("ledger.sqlite");
    let storage = Storage::open(&path).unwrap();
    storage
        .insert_project(&ProjectRecord {
            project_id: "project-1".into(),
            display_name: "project".into(),
            remote_url: Some("https://example.com/owner/project".into()),
            scope: "owner".into(),
            created_at: "2026-08-04T10:00:00Z".into(),
        })
        .unwrap();
    let workflow = sddk_engine::load_workflow_str(WORKFLOW_YAML).unwrap();
    let policy = CapabilityPolicy::from_workflow(&workflow);
    let gateway = CapabilityGateway::new(policy, Storage::open(&path).unwrap());
    (directory, gateway)
}

fn release_input(tag: &str) -> ReleasePlanInput {
    ReleasePlanInput {
        project_id: "project-1".into(),
        cycle_id: None,
        branch: "feat/release".into(),
        base_branch: "main".into(),
        pr_title: "Release".into(),
        pr_body: "body".into(),
        tag: tag.into(),
        release_title: "v1".into(),
        release_notes: "notes".into(),
        approve: true,
        timestamp: "2026-08-04T10:00:00Z".into(),
        actor: "release-test".into(),
    }
}

#[test]
fn full_release_creates_pr_merges_and_publishes() {
    let (_directory, mut gateway) = gateway();
    let mut forge = MockForge::new();
    forge.seed_checks(0, vec![]);

    let plan = plan_release(release_input("v1.0.0"), &forge).unwrap();
    assert_eq!(plan.steps.len(), 3);

    let outcome = apply_release(&mut gateway, &plan, &mut forge).unwrap();
    assert_eq!(outcome.applied.len(), 3);
    assert!(outcome.skipped.is_empty());
    assert!(outcome.converged);

    assert_eq!(forge.find_open_pr("feat/release", "main").unwrap(), None);
    assert!(forge.release_state("v1.0.0").unwrap().unwrap().published);
    let receipts = gateway.receipts("project-1").unwrap();
    assert_eq!(receipts.len(), 3);
    assert!(
        receipts
            .iter()
            .all(|receipt| receipt.status == CapabilityStatus::Succeeded)
    );
}

#[test]
fn interrupted_release_converges_without_duplicating_effects() {
    let (_directory, mut gateway) = gateway();
    let mut forge = MockForge::new();
    forge.seed_open_pr("feat/release", "main", 3);
    forge.seed_checks(3, vec![]);
    forge.seed_release("v1.0.0");

    let plan = plan_release(release_input("v1.0.0"), &forge).unwrap();
    assert_eq!(plan.steps, vec![sddk_gateway::ReleaseStep::MergePr]);

    let outcome = apply_release(&mut gateway, &plan, &mut forge).unwrap();
    assert_eq!(outcome.applied.len(), 1);
    assert!(outcome.converged);
    assert_eq!(outcome.skipped.len(), 0);
    assert!(forge.is_merged(3));

    let second = apply_release(&mut gateway, &plan, &mut forge).unwrap();
    assert!(second.applied.is_empty());
    assert_eq!(second.skipped.len(), 1);
    assert!(second.converged);

    let receipts = gateway.receipts("project-1").unwrap();
    assert_eq!(receipts.len(), 1);
}

#[test]
fn release_without_open_pr_creates_and_merges() {
    let (_directory, mut gateway) = gateway();
    let mut forge = MockForge::new();
    forge.seed_release("v1.0.0");

    let plan = plan_release(release_input("v1.0.0"), &forge).unwrap();
    assert_eq!(plan.steps.len(), 2);
    assert!(
        forge
            .find_open_pr("feat/release", "main")
            .unwrap()
            .is_none()
    );

    let outcome = apply_release(&mut gateway, &plan, &mut forge).unwrap();
    assert_eq!(outcome.applied.len(), 2);
    assert!(outcome.skipped.is_empty());
    assert!(outcome.converged);
    assert_eq!(forge.find_open_pr("feat/release", "main").unwrap(), None);
    assert!(forge.release_state("v1.0.0").unwrap().unwrap().published);
}

#[test]
fn reconcile_finalizes_interrupted_receipts_against_provider() {
    let (_directory, mut gateway) = gateway();
    let mut forge = MockForge::new();
    forge.seed_release("v9.9.9");

    let begin = |gateway: &mut CapabilityGateway, tag: &str| {
        gateway
            .begin_effect(&CapabilityPlanInput {
                project_id: "project-1".into(),
                cycle_id: None,
                capability: "release.create".into(),
                reason: "interrupted".into(),
                program: "forge".into(),
                args: vec![tag.into()],
                env: Default::default(),
                timeout_ms: 60_000,
                output_max_bytes: 1_048_576,
                approve: true,
                timestamp: "2026-08-04T10:00:00Z".into(),
                actor: "release-test".into(),
            })
            .unwrap()
            .receipt_id
    };
    let present = begin(&mut gateway, "v9.9.9");
    let absent = begin(&mut gateway, "v0.0.1");

    let reconciled = reconcile_pending(&mut gateway, &forge).unwrap();
    assert_eq!(reconciled.len(), 2);
    let by_id = |id: &str| {
        reconciled
            .iter()
            .find(|receipt| receipt.receipt_id == id)
            .unwrap()
    };
    assert_eq!(by_id(&present).status, CapabilityStatus::Succeeded);
    assert_eq!(by_id(&absent).status, CapabilityStatus::Failed);

    let again = reconcile_pending(&mut gateway, &forge).unwrap();
    assert!(again.is_empty());
}
