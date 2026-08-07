//! UAT (User Acceptance Testing) domain types — data-driven YAML model
//! (ADR-012/ADR-013): agents produce YAML artifacts, a deterministic renderer
//! turns them into self-contained HTML dashboards.

use serde::{Deserialize, Serialize};

/// Closed vocabulary for scenario priority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum UatPriority {
    /// Critical path: blocks release without human verdict.
    P0,
    /// High value: normally covered by UAT.
    P1,
    /// Lower priority: optional coverage.
    #[default]
    P2,
}

/// Closed vocabulary for scenario assignee role.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UatAssignee {
    /// Functional flow validation.
    #[default]
    Developer,
    /// Design/UX/consistency validation.
    Architect,
}

/// Closed vocabulary for per-step execution status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum UatStatus {
    /// Scenario passed.
    #[default]
    Pass,
    /// Scenario failed (defect found).
    Fail,
    /// Scenario could not run (blocked).
    Blocked,
    /// Scenario partially passed.
    Partial,
}

/// Closed vocabulary for who executed a session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UatExecutor {
    /// Executed by a human tester.
    #[default]
    Human,
    /// Executed by the visual agent (pre-flight).
    Fara,
    /// Mixed human + agent execution.
    Mixed,
}

/// Closed vocabulary for the global release verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum UatVerdict {
    /// All critical scenarios pass.
    Ready,
    /// Passes with documented risks.
    ReadyWithRisks,
    /// Blocking defects found.
    NotReady,
}

/// One guided step of a scenario (plain language, junior-friendly).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UatStep {
    /// Plain-language instruction (e.g. "Abre http://localhost:3000/login").
    pub action: String,
    /// Renderer paints a copy button when true.
    #[serde(default)]
    pub copy_hint: bool,
    /// Expected observable outcome of this step.
    pub expected: String,
}

/// One acceptance scenario of a feature.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UatScenario {
    /// Stable scenario id, e.g. `S-1`.
    pub id: String,
    /// Human-readable scenario title.
    pub title: String,
    #[serde(default)]
    /// Scenario priority (P0..P2).
    pub priority: UatPriority,
    #[serde(default)]
    /// Role assigned to validate.
    pub assignee: UatAssignee,
    /// Preconditions the tester must ensure before starting.
    #[serde(default)]
    pub preconditions: Vec<String>,
    /// Junior guided view: one step per screen.
    #[serde(default)]
    pub plain_steps: Vec<UatStep>,
    /// Senior matrix view: technical shorthand (optional).
    #[serde(default)]
    pub technical_steps: Vec<String>,
    /// "Why this matters" — written by uat-guide.
    #[serde(default)]
    pub rationale: Option<String>,
    /// What evidence the tester must capture (screenshot, log, note).
    #[serde(default)]
    pub evidence_prompt: Option<String>,
    /// Semantic flags from a closed vocabulary: smoke|warning|optional|data-verify.
    #[serde(default)]
    pub flags: Vec<String>,
    /// Estimated execution time in minutes.
    #[serde(default)]
    pub est_minutes: u32,
}

/// One feature under test, grouping its scenarios.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UatFeature {
    /// Stable feature id, e.g. `F-01`.
    pub id: String,
    /// Feature display name.
    pub name: String,
    /// PRD requirement reference for the traceability view (e.g. `RF-016`).
    #[serde(default)]
    pub requirement_ref: Option<String>,
    #[serde(default)]
    /// Feature priority (P0..P2).
    pub priority: UatPriority,
    #[serde(default)]
    /// Scenarios of this feature.
    pub scenarios: Vec<UatScenario>,
}

/// Canonical acceptance plan artifact (`uat-plan.yaml`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UatPlan {
    /// Schema version of this plan (renderer supports N versions).
    pub schema_version: u32,
    /// Candidate tag under test and aggregation window.
    pub release: UatPlanRelease,
    /// Which agent generated the plan.
    #[serde(default)]
    pub generated_by: String,
    /// RFC 3339 generation timestamp.
    pub generated_at: String,
    #[serde(default)]
    /// Features under test.
    pub features: Vec<UatFeature>,
}

/// Release context of a plan: features aggregated since the last UAT'd tag.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UatPlanRelease {
    /// Candidate tag, e.g. `v1.5.0`.
    pub candidate: String,
    /// Project identifier (adopted project id or repo basename).
    #[serde(default)]
    pub project: Option<String>,
    /// Last release that went through UAT; features are aggregated from here.
    #[serde(default)]
    pub last_uat_release: Option<String>,
}

/// One executed session (`uat-session.yaml`): per-scenario results + evidence.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UatSession {
    /// Schema version of this session.
    pub schema_version: u32,
    /// Session id, e.g. `uat-<uuid>`.
    pub session_id: String,
    /// Reference to the plan this session executes.
    pub plan_ref: String,
    /// Candidate tag under test.
    pub release: String,
    #[serde(default)]
    /// Who executed this session.
    pub executor: UatExecutor,
    /// Human or agent name (e.g. "María", "fara-1.5").
    #[serde(default)]
    pub executed_by: Option<String>,
    /// RFC 3339 session start.
    pub started_at: String,
    #[serde(default)]
    /// RFC 3339 session end (None while running).
    pub finished_at: Option<String>,
    #[serde(default)]
    /// Per-scenario results.
    pub results: Vec<UatScenarioResult>,
}

/// Per-scenario result inside a session.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UatScenarioResult {
    /// Scenario id from the referenced plan.
    pub scenario_id: String,
    #[serde(default)]
    /// Execution status (PASS..PARTIAL).
    pub status: UatStatus,
    #[serde(default)]
    /// Free-text tester comment.
    pub comment: Option<String>,
    /// Evidence references by SHA-256 hash (chains with the ledger).
    #[serde(default)]
    pub evidence: Vec<UatEvidence>,
    #[serde(default)]
    /// Minutes spent on this scenario.
    pub duration_minutes: u32,
}

/// Evidence captured for a scenario result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UatEvidence {
    /// screenshot | log | note.
    pub kind: String,
    /// sha256:<hash> of the evidence payload.
    /// sha256:<hash> of the evidence payload.
    pub r#ref: String,
    #[serde(default)]
    /// Optional evidence description.
    pub note: Option<String>,
}

/// Aggregated report (`uat-report.yaml`) with the global verdict.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UatReport {
    /// Schema version of this report.
    pub schema_version: u32,
    /// Candidate tag under test.
    pub release: String,
    /// Plan reference this report aggregates.
    pub plan_ref: String,
    /// Session ids aggregated into this report.
    #[serde(default)]
    /// Session ids aggregated into this report.
    pub sessions: Vec<String>,
    /// Numeric rollup of the report.
    pub summary: UatReportSummary,
    /// Per-feature rollup for the traceability view.
    #[serde(default)]
    pub features: Vec<UatFeatureRollup>,
    /// Recommendation, not an order: READY | READY_WITH_RISKS | NOT_READY.
    /// Recommendation: READY | READY_WITH_RISKS | NOT_READY.
    pub verdict: UatVerdict,
    /// Scenarios blocking a READY verdict (with reasons).
    #[serde(default)]
    pub not_ready_blockers: Vec<String>,
}

/// Numeric rollup of a report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UatReportSummary {
    /// Total scenarios in the plan.
    pub total_scenarios: u32,
    /// Passed count.
    pub passed: u32,
    /// Failed count.
    pub failed: u32,
    /// Blocked count.
    pub blocked: u32,
    /// Partial count.
    pub partial: u32,
    /// Coverage percentage (0..=100).
    pub coverage_pct: f64,
    /// Functional defects found.
    pub defects: u32,
    /// UX issues observed.
    pub ux_issues: u32,
    #[serde(default)]
    /// Total human minutes spent across sessions.
    pub uat_duration_minutes: u32,
}

/// Per-feature rollup: coverage + scenario statuses (traceability view).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UatFeatureRollup {
    /// Feature id (matches plan).
    pub id: String,
    /// Feature display name.
    pub name: String,
    /// Percentage of scenarios covered.
    pub coverage_pct: f64,
    #[serde(default)]
    /// Scenario statuses rolled up for this feature.
    pub scenarios: Vec<UatScenarioRollup>,
}

/// Scenario status within a feature rollup.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UatScenarioRollup {
    /// Scenario id (matches plan).
    pub scenario_id: String,
    /// Rolled-up status.
    pub status: UatStatus,
    /// Executor that produced this status (last writer wins).
    #[serde(default)]
    pub executor: Option<UatExecutor>,
}

// ---------------------------------------------------------------------------
// Per-project UAT config (XDG-resident, ADR-0011 compliant)
// ---------------------------------------------------------------------------

/// What the `release-uat-approved` gate does for a given release type.
/// Default policy: major=Required, minor=Required, patch=Skip.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReleaseGateAction {
    /// The gate blocks; the release cannot proceed without a human UAT verdict.
    #[default]
    Required,
    /// The gate is bypassed.
    Skip,
    /// The gate is recorded but does not block (advisory only).
    Advisory,
}

/// Type of a release — derived by semver diff against the previous tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReleaseType {
    /// Major version bump (breaking change).
    Major,
    /// Minor version bump (new features, backwards-compatible).
    Minor,
    /// Patch version bump (bug fixes).
    Patch,
}

impl ReleaseType {
    /// String form (`"major"`/`"minor"`/`"patch"`).
    pub fn as_str(&self) -> &'static str {
        match self {
            ReleaseType::Major => "major",
            ReleaseType::Minor => "minor",
            ReleaseType::Patch => "patch",
        }
    }
}

/// Default gate policy (matches the RNF-010 spec in the knowledge vault):
/// major and minor require human verdict, patches do not.
/// Per-release-type gate policy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ReleaseGateMap {
    /// Policy for major releases (default: required).
    #[serde(default = "default_required")]
    pub major: ReleaseGateAction,
    /// Policy for minor releases (default: required).
    #[serde(default = "default_required")]
    pub minor: ReleaseGateAction,
    /// Policy for patch releases (default: skip).
    #[serde(default = "default_skip")]
    pub patch: ReleaseGateAction,
}

impl Default for ReleaseGateMap {
    fn default() -> Self {
        Self { major: ReleaseGateAction::Required, minor: ReleaseGateAction::Required, patch: ReleaseGateAction::Skip }
    }
}

fn default_required() -> ReleaseGateAction { ReleaseGateAction::Required }
fn default_skip() -> ReleaseGateAction { ReleaseGateAction::Skip }

/// Which human roles are available to validate the UAT (controls the
/// orchestrator's activation function).
/// Which human roles can validate UAT for this project.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct HumanAvailability {
    /// Whether a developer is available to validate functional flows.
    #[serde(default = "default_true")]
    pub developer: bool,
    /// Whether an architect is available to validate design/UX/consistency.
    #[serde(default = "default_true")]
    pub architect: bool,
}

fn default_true() -> bool { true }

impl Default for HumanAvailability {
    fn default() -> Self { Self { developer: true, architect: true } }
}

/// Thresholds for the orchestrator's activation function (ADR-012): when is
/// a release worth the human's time?
/// Thresholds for the orchestrator's activation function (ADR-012).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ActivationThresholds {
    /// Minimum number of features required to activate UAT.
    #[serde(default = "default_three")]
    pub min_features: u32,
    /// Minimum diff lines required to activate UAT.
    #[serde(default = "default_two_hundred")]
    pub min_diff_lines: u32,
    /// Domain keywords (e.g. "auth", "payments") that trigger UAT activation.
    #[serde(default)]
    pub critical_domains: Vec<String>,
}

fn default_three() -> u32 { 3 }
fn default_two_hundred() -> u32 { 200 }

impl Default for ActivationThresholds {
    fn default() -> Self {
        Self { min_features: 3, min_diff_lines: 200, critical_domains: Vec::new() }
    }
}

/// Per-project UAT configuration (XDG: `~/.local/share/sddk/projects/<id>/uat.toml`).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UatConfig {
    /// Per-release-type gate policy (major/minor/patch).
    #[serde(default)]
    pub release_gate: ReleaseGateMap,
    /// Which human roles are available to validate UAT.
    #[serde(default)]
    pub human: HumanAvailability,
    /// Thresholds for the orchestrator's activation function.
    #[serde(default)]
    pub activation: ActivationThresholds,
}

/// Evaluate the gate for a given release type under a config.
pub fn evaluate_release_gate(config: &UatConfig, release_type: ReleaseType) -> ReleaseGateAction {
    match release_type {
        ReleaseType::Major => config.release_gate.major,
        ReleaseType::Minor => config.release_gate.minor,
        ReleaseType::Patch => config.release_gate.patch,
    }
}

/// Derive a release type from the semver diff of two tags (`v1.5.2` vs `v1.4.0`).
/// Returns None when tags can't be parsed or are equal.
pub fn release_type_from_diff(current: &str, previous: &str) -> Option<ReleaseType> {
    let parse = |t: &str| -> Option<(u64, u64, u64)> {
        let s = t.trim_start_matches(|c: char| !c.is_ascii_digit());
        let mut parts = s.split('.');
        Some((parts.next()?.parse().ok()?, parts.next()?.parse().ok()?, parts.next()?.parse().ok()?))
    };
    let (cmaj, cmin, cpat) = parse(current)?;
    let (pmaj, pmin, ppat) = parse(previous)?;
    if cmaj > pmaj { Some(ReleaseType::Major) }
    else if cmin > pmin { Some(ReleaseType::Minor) }
    else if cpat > ppat { Some(ReleaseType::Patch) }
    else { None }
}

#[cfg(test)]
mod config_tests {
    use super::*;

    #[test]
    fn default_policy_blocks_major_minor_skips_patch() {
        let config = UatConfig::default();
        assert_eq!(evaluate_release_gate(&config, ReleaseType::Major), ReleaseGateAction::Required);
        assert_eq!(evaluate_release_gate(&config, ReleaseType::Minor), ReleaseGateAction::Required);
        assert_eq!(evaluate_release_gate(&config, ReleaseType::Patch), ReleaseGateAction::Skip);
    }

    #[test]
    fn custom_policy_overrides_defaults() {
        let toml = r#"
            [release_gate]
            major = "skip"
            minor = "advisory"
            patch = "required"
        "#;
        let config: UatConfig = toml::from_str(toml).unwrap();
        assert_eq!(evaluate_release_gate(&config, ReleaseType::Major), ReleaseGateAction::Skip);
        assert_eq!(evaluate_release_gate(&config, ReleaseType::Minor), ReleaseGateAction::Advisory);
        assert_eq!(evaluate_release_gate(&config, ReleaseType::Patch), ReleaseGateAction::Required);
    }

    #[test]
    fn release_type_from_diff_basic() {
        assert_eq!(release_type_from_diff("v1.5.2", "v1.4.0"), Some(ReleaseType::Minor));
        assert_eq!(release_type_from_diff("v2.0.0", "v1.9.9"), Some(ReleaseType::Major));
        assert_eq!(release_type_from_diff("v1.5.2", "v1.5.1"), Some(ReleaseType::Patch));
        assert_eq!(release_type_from_diff("v1.5.2", "v1.5.2"), None);
        assert_eq!(release_type_from_diff("not-a-tag", "v1.0.0"), None);
    }
}
