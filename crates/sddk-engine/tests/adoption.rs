use std::fs;
use std::path::{Path, PathBuf};

use sddk_engine::{
    AdoptionError, AdoptionPlan, AdoptionPlanInput, AdoptionStatusKind, XdgEnvironment,
    adoption_status, apply_adoption, plan_adoption, read_adoption_receipt, repair_adoption,
};
use sddk_storage::{ProjectRecord, Storage, WorkspaceRecord};
use tempfile::TempDir;

const TIMESTAMP: &str = "2026-08-04T10:00:00Z";
const SEED: &str = "a0b1c2d3-e4f5-4678-9abc-def012345678";

#[test]
fn plan_is_write_free_and_reports_identity_paths_and_hash() {
    let fixture = Fixture::new();
    let plan = fixture.remote_plan("checkout", "https://example.com/acme/backend.git", ".");

    assert!(!fixture.data.exists());
    assert!(!fixture.state.exists());
    assert!(plan.identity.project_id.as_str().starts_with("p-"));
    assert!(plan.workspace_id.starts_with("w-"));
    assert_eq!(
        plan.receipt.identity_source,
        sddk_domain::IdentitySource::Remote
    );
    assert!(plan.receipt.configuration_hash.starts_with("sha256:"));
    assert_eq!(plan.receipt.paths.vault, path_text(&plan.paths.vault));
    assert_eq!(plan.receipt.paths.ledger, path_text(&plan.paths.ledger));
}

#[test]
fn same_basename_different_remotes_and_scopes_do_not_collide() {
    let fixture = Fixture::new();
    let first = fixture.remote_plan("one/backend", "https://example.com/acme/backend", ".");
    let second = fixture.remote_plan("two/backend", "https://example.com/other/backend", ".");
    let scoped = fixture.remote_plan(
        "three/backend",
        "https://example.com/acme/backend",
        "services/api",
    );

    assert_ne!(first.identity.project_id, second.identity.project_id);
    assert_ne!(first.identity.project_id, scoped.identity.project_id);
    assert_ne!(first.paths.ledger, second.paths.ledger);
}

#[test]
fn worktrees_share_project_storage_and_have_distinct_workspace_receipts() {
    let fixture = Fixture::new();
    let first = fixture.remote_plan("repo", "git@example.com:acme/repo.git", ".");
    let second = fixture.remote_plan("repo-feature", "ssh://git@example.com/acme/repo", ".");

    assert_eq!(first.identity.project_id, second.identity.project_id);
    assert_eq!(first.paths.ledger, second.paths.ledger);
    assert_ne!(first.workspace_id, second.workspace_id);
    assert_ne!(first.paths.receipt, second.paths.receipt);
    assert_eq!(
        apply_adoption(&first).unwrap().status,
        AdoptionStatusKind::Complete
    );
    assert_eq!(
        apply_adoption(&second).unwrap().status,
        AdoptionStatusKind::Complete
    );
    let storage = Storage::open_read_only(&first.paths.ledger).unwrap();
    assert!(storage.get_workspace(&first.workspace_id).is_ok());
    assert!(storage.get_workspace(&second.workspace_id).is_ok());
}

#[test]
fn apply_replay_is_idempotent_and_preserves_original_receipt_metadata() {
    let fixture = Fixture::new();
    let first = fixture.remote_plan("repo", "https://example.com/acme/repo", ".");
    let first_status = apply_adoption(&first).unwrap();
    let bytes = fs::read(&first.paths.receipt).unwrap();

    let mut replay_input = fixture.input("repo");
    replay_input.remote_url = Some("git@example.com:acme/repo.git".into());
    replay_input.timestamp = "2026-08-04T11:00:00Z".into();
    replay_input.actor = "second-actor".into();
    let replay = plan_adoption(replay_input).unwrap();
    let replayed_status = apply_adoption(&replay).unwrap();

    assert_eq!(first_status.status, AdoptionStatusKind::Complete);
    assert_eq!(replayed_status.status, AdoptionStatusKind::Complete);
    assert_eq!(fs::read(&first.paths.receipt).unwrap(), bytes);
    assert_eq!(
        replayed_status.receipt.unwrap().timestamp,
        first.receipt.timestamp
    );
}

#[test]
fn fallback_seed_is_persisted_and_reused() {
    let fixture = Fixture::new();
    let plan = fixture.fallback_plan("local-repo", SEED);
    apply_adoption(&plan).unwrap();
    let receipt = read_adoption_receipt(&plan.paths.receipt).unwrap();

    assert_eq!(receipt.fallback_seed.as_deref(), Some(SEED));
    assert_eq!(
        receipt.identity_source,
        sddk_domain::IdentitySource::Fallback
    );
    let replay = fixture.fallback_plan("local-repo", receipt.fallback_seed.as_deref().unwrap());
    assert_eq!(replay.identity.project_id, plan.identity.project_id);
    assert_eq!(
        adoption_status(&replay).unwrap().status,
        AdoptionStatusKind::Complete
    );
}

#[test]
fn repair_completes_receipt_only_state() {
    let fixture = Fixture::new();
    let plan = fixture.remote_plan("repo", "https://example.com/acme/repo", ".");
    write_receipt_fixture(&plan);

    assert_eq!(
        adoption_status(&plan).unwrap().status,
        AdoptionStatusKind::ReceiptOnly
    );
    assert_eq!(
        repair_adoption(&plan).unwrap().status,
        AdoptionStatusKind::Complete
    );
}

#[test]
fn repair_completes_ledger_only_state() {
    let fixture = Fixture::new();
    let plan = fixture.remote_plan("repo", "https://example.com/acme/repo", ".");
    register_ledger_fixture(&plan);

    assert_eq!(
        adoption_status(&plan).unwrap().status,
        AdoptionStatusKind::LedgerOnly
    );
    assert_eq!(
        repair_adoption(&plan).unwrap().status,
        AdoptionStatusKind::Complete
    );
}

#[test]
fn repair_refuses_configuration_conflict() {
    let fixture = Fixture::new();
    let original = fixture.remote_plan("repo", "https://example.com/acme/repo", ".");
    apply_adoption(&original).unwrap();
    let mut changed_input = fixture.input("repo");
    changed_input.remote_url = Some("https://example.com/acme/repo".into());
    changed_input.runtime_version = "0.2.0".into();
    let changed = plan_adoption(changed_input).unwrap();

    assert_eq!(
        adoption_status(&changed).unwrap().status,
        AdoptionStatusKind::Conflict
    );
    assert!(matches!(
        repair_adoption(&changed),
        Err(AdoptionError::UnsafeState {
            status: AdoptionStatusKind::Conflict,
            ..
        })
    ));
}

#[test]
fn corrupt_receipt_is_classified_and_never_overwritten() {
    let fixture = Fixture::new();
    let plan = fixture.remote_plan("repo", "https://example.com/acme/repo", ".");
    fs::create_dir_all(plan.paths.receipt.parent().unwrap()).unwrap();
    fs::write(&plan.paths.receipt, b"{not-json\n").unwrap();

    assert_eq!(
        adoption_status(&plan).unwrap().status,
        AdoptionStatusKind::Corrupt
    );
    assert!(matches!(
        apply_adoption(&plan),
        Err(AdoptionError::UnsafeState {
            status: AdoptionStatusKind::Corrupt,
            ..
        })
    ));
    assert_eq!(fs::read(&plan.paths.receipt).unwrap(), b"{not-json\n");
}

struct Fixture {
    _directory: TempDir,
    root: PathBuf,
    data: PathBuf,
    state: PathBuf,
    cache: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        Self {
            root: directory.path().join("checkouts"),
            data: directory.path().join("xdg-data"),
            state: directory.path().join("xdg-state"),
            cache: directory.path().join("xdg-cache"),
            _directory: directory,
        }
    }

    fn input(&self, relative_root: &str) -> AdoptionPlanInput {
        AdoptionPlanInput {
            remote_url: None,
            scope: ".".into(),
            fallback_seed: None,
            canonical_workspace_path: self.root.join(relative_root),
            display_name: Path::new(relative_root)
                .file_name()
                .unwrap()
                .to_string_lossy()
                .into_owned(),
            xdg: XdgEnvironment {
                home: None,
                data_home: Some(self.data.clone()),
                state_home: Some(self.state.clone()),
                cache_home: Some(self.cache.clone()),
            },
            sddk_version: "3.6".into(),
            runtime_version: "0.1.0".into(),
            timestamp: TIMESTAMP.into(),
            actor: "test-runtime".into(),
        }
    }

    fn remote_plan(&self, root: &str, remote: &str, scope: &str) -> AdoptionPlan {
        let mut input = self.input(root);
        input.remote_url = Some(remote.into());
        input.scope = scope.into();
        plan_adoption(input).unwrap()
    }

    fn fallback_plan(&self, root: &str, seed: &str) -> AdoptionPlan {
        let mut input = self.input(root);
        input.fallback_seed = Some(seed.into());
        plan_adoption(input).unwrap()
    }
}

fn write_receipt_fixture(plan: &AdoptionPlan) {
    fs::create_dir_all(plan.paths.receipt.parent().unwrap()).unwrap();
    fs::write(
        &plan.paths.receipt,
        serde_json::to_vec_pretty(&plan.receipt).unwrap(),
    )
    .unwrap();
}

fn register_ledger_fixture(plan: &AdoptionPlan) {
    let mut storage = Storage::open(&plan.paths.ledger).unwrap();
    storage
        .register_project_workspace(
            &ProjectRecord {
                project_id: plan.receipt.project_id.clone(),
                display_name: plan.receipt.display_name.clone(),
                remote_url: plan.receipt.remote_url.clone(),
                scope: plan.receipt.scope.clone(),
                created_at: plan.receipt.timestamp.clone(),
            },
            &WorkspaceRecord {
                workspace_id: plan.receipt.workspace_id.clone(),
                project_id: plan.receipt.project_id.clone(),
                canonical_path: plan.receipt.canonical_workspace_path.clone(),
                created_at: plan.receipt.timestamp.clone(),
            },
        )
        .unwrap();
}

fn path_text(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}
