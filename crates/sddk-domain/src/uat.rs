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
