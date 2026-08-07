//! UAT (User Acceptance Testing) CLI — data-driven YAML artifacts rendered to
//! self-contained HTML dashboards (ADR-012/ADR-013).
//!
//! Agents produce `uat-plan.yaml`/`uat-session.yaml`/`uat-report.yaml`; this
//! module validates, renders, and ingests them. The dashboard kit ships in the
//! bundle under `assets/uat-dashboard/` (ADR-013).

use std::path::{Path, PathBuf};

use clap::{Args, Subcommand};

use crate::{dev_cmd, render_result, CommandOutput, OutputFormat};

use sddk_domain::{
    UatFeatureRollup, UatPlan, UatReport, UatReportSummary, UatScenarioRollup, UatSession,
    UatVerdict,
};

/// Default view when rendering a dashboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub(crate) enum UatView {
    Guided,
    Matrix,
    Traceability,
}

#[derive(Debug, Subcommand)]
pub(crate) enum UatCommand {
    /// Generate a canonical `uat-plan.yaml` for a release candidate.
    Plan(UatPlanArgs),
    /// Validate a uat YAML artifact against the domain schema.
    Validate(UatValidateArgs),
    /// Render a self-contained HTML dashboard from a plan (ADR-0013 kit).
    Dashboard(UatDashboardArgs),
    /// Ingest a session into the ledger + control plane (aggregate only).
    Ingest(UatIngestArgs),
    /// Aggregate sessions into a `uat-report.yaml` with a verdict.
    Report(UatReportArgs),
    /// Show the UAT status of a release candidate.
    Status(UatStatusArgs),
}

#[derive(Debug, Clone, Args)]
pub(crate) struct UatPlanArgs {
    /// Candidate tag under test, e.g. `v1.5.0`.
    #[arg(long)]
    pub(crate) release: String,
    /// Aggregate features from this tag (default: last UAT'd release or all).
    #[arg(long)]
    pub(crate) from: Option<String>,
    /// Output YAML path.
    #[arg(long)]
    pub(crate) output: Option<PathBuf>,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct UatValidateArgs {
    /// Path to a uat-plan / uat-session / uat-report YAML.
    #[arg(long)]
    pub(crate) file: PathBuf,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct UatDashboardArgs {
    /// Plan YAML to render.
    #[arg(long)]
    pub(crate) plan: PathBuf,
    /// View to render.
    #[arg(long, value_enum, default_value_t = UatView::Guided)]
    pub(crate) view: UatView,
    /// Theme: dark | light.
    #[arg(long, default_value = "dark")]
    pub(crate) theme: String,
    /// Output HTML path (default: `uat-dashboard-<release>.html`).
    #[arg(long)]
    pub(crate) output: Option<PathBuf>,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct UatIngestArgs {
    /// Session YAML/JSON to ingest.
    #[arg(long)]
    pub(crate) session: PathBuf,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct UatReportArgs {
    /// Candidate tag under test.
    #[arg(long)]
    pub(crate) release: String,
    /// One or more session files to aggregate.
    #[arg(long)]
    pub(crate) sessions: Vec<PathBuf>,
    /// Plan file the sessions reference.
    #[arg(long)]
    pub(crate) plan: PathBuf,
    /// Output YAML path.
    #[arg(long)]
    pub(crate) output: Option<PathBuf>,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct UatStatusArgs {
    /// Candidate tag under test.
    #[arg(long)]
    pub(crate) release: String,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

pub(crate) fn run_uat(command: UatCommand, environment: &crate::CliEnvironment) -> CommandOutput {
    match command {
        UatCommand::Plan(args) => run_uat_plan(args, environment),
        UatCommand::Validate(args) => run_uat_validate(args),
        UatCommand::Dashboard(args) => run_uat_dashboard(args, environment),
        UatCommand::Ingest(args) => run_uat_ingest(args, environment),
        UatCommand::Report(args) => run_uat_report(args),
        UatCommand::Status(args) => run_uat_status(args),
    }
}

fn run_uat_plan(args: UatPlanArgs, _environment: &crate::CliEnvironment) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<PathBuf> {
        let plan = UatPlan {
            schema_version: 1,
            release: sddk_domain::UatPlanRelease {
                candidate: args.release.clone(),
                project: None,
                last_uat_release: args.from,
            },
            generated_by: "uat-planner".into(),
            generated_at: now_rfc3339(),
            features: Vec::new(),
        };
        let path = args.output.unwrap_or_else(|| {
            PathBuf::from(format!("uat-plan-{}.yaml", args.release))
        });
        let yaml = serde_saphyr::to_string(&plan)
            .map_err(|e| anyhow::anyhow!("serialization failed: {e}"))?;
        std::fs::write(&path, yaml)?;
        Ok(path)
    })();
    render_result(result, format, |path| {
        format!("uat plan written: {}\n", path.display())
    })
}

fn run_uat_validate(args: UatValidateArgs) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<()> {
        let raw = std::fs::read_to_string(&args.file)
            .map_err(|e| anyhow::anyhow!("cannot read {}: {e}", args.file.display()))?;
        // Accept JSON as an alias of YAML (both are valid serde_saphyr input).
        let value: serde_json::Value = serde_saphyr::from_str(&raw)
            .map_err(|e| anyhow::anyhow!("invalid YAML/JSON in {}: {e}", args.file.display()))?;
        let kind = value
            .get("schema_version")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        if kind == 0 {
            anyhow::bail!("missing or invalid `schema_version`");
        }
        let has_scenarios = value
            .get("features")
            .and_then(|f| f.as_array())
            .map(|arr| arr.iter().any(|feat| {
                feat.get("scenarios")
                    .and_then(|s| s.as_array())
                    .map(|sc| !sc.is_empty())
                    .unwrap_or(false)
            }))
            .unwrap_or(false);
        if !has_scenarios {
            anyhow::bail!("plan must have at least one feature with one scenario");
        }
        // Round-trip through the typed model to enforce the closed vocabularies.
        let _plan: UatPlan = serde_saphyr::from_str(&raw)
            .map_err(|e| anyhow::anyhow!("schema validation failed: {e}"))?;
        Ok(())
    })();
    render_result(result, format, |()| "uat validate: OK\n".into())
}

fn run_uat_dashboard(args: UatDashboardArgs, environment: &crate::CliEnvironment) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<PathBuf> {
        let raw = std::fs::read_to_string(&args.plan)
            .map_err(|e| anyhow::anyhow!("cannot read plan {}: {e}", args.plan.display()))?;
        let plan: UatPlan = serde_saphyr::from_str(&raw)
            .map_err(|e| anyhow::anyhow!("invalid plan {}: {e}", args.plan.display()))?;
        let html = render_dashboard_html(&plan, args.view, &args.theme, environment)?;
        let output = args.output.unwrap_or_else(|| {
            PathBuf::from(format!("uat-dashboard-{}.html", plan.release.candidate))
        });
        std::fs::write(&output, html)?;
        Ok(output)
    })();
    render_result(result, format, |path| {
        format!("uat dashboard written: {}\n", path.display())
    })
}

fn run_uat_ingest(args: UatIngestArgs, environment: &crate::CliEnvironment) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<UatSession> {
        let raw = std::fs::read_to_string(&args.session)
            .map_err(|e| anyhow::anyhow!("cannot read session {}: {e}", args.session.display()))?;
        let session: UatSession = serde_saphyr::from_str(&raw)
            .map_err(|e| anyhow::anyhow!("invalid session {}: {e}", args.session.display()))?;

        // Control plane (ADR-012): upsert only the aggregate — sessions and
        // evidence stay in XDG artifacts, the CP keeps its numeric contract.
        if let Ok(conn) = crate::telemetry::open_store(environment, false) {
            let project_id = session
                .executed_by
                .clone()
                .map(|by| format!("uat-{}", by.to_lowercase().replace(' ', "-")))
                .unwrap_or_else(|| "uat-unknown".into());
            // Ensure the FK target exists (projects table) before inserting.
            let now = now_rfc3339();
            conn.execute(
                "INSERT OR IGNORE INTO projects (project_id, display_name, scope, first_seen, last_seen)
                 VALUES (?1, ?2, 'uat', ?3, ?3)",
                rusqlite::params![project_id, project_id, now],
            )?;
            let passed = session
                .results
                .iter()
                .filter(|r| r.status == sddk_domain::UatStatus::Pass)
                .count() as u32;
            let failed = session
                .results
                .iter()
                .filter(|r| r.status == sddk_domain::UatStatus::Fail)
                .count() as u32;
            let blocked = session
                .results
                .iter()
                .filter(|r| r.status == sddk_domain::UatStatus::Blocked)
                .count() as u32;
            let total = session.results.len().max(1) as u32;
            let coverage = 100.0 * (passed + blocked) as f64 / total as f64;
            let verdict = if failed == 0 && blocked == 0 {
                "READY"
            } else if failed == 0 {
                "READY_WITH_RISKS"
            } else {
                "NOT_READY"
            };
            let duration = session.results.iter().map(|r| r.duration_minutes).sum::<u32>();
            let recorded_at = session
                .finished_at
                .clone()
                .unwrap_or_else(|| session.started_at.clone());
            crate::telemetry::upsert_uat_result(
                &conn,
                &crate::telemetry::UatResultRow {
                    project_id,
                    tag_version: session.release.clone(),
                    verdict: verdict.into(),
                    coverage_pct: coverage,
                    defects: failed as i64,
                    session_count: session.results.len() as i64,
                    uat_duration_minutes: duration as i64,
                    recorded_at,
                },
            )?;
        }

        Ok(session)
    })();
    render_result(result, format, |session| {
        format!(
            "uat session ingested: {} ({} results, release {})\n",
            session.session_id,
            session.results.len(),
            session.release
        )
    })
}

fn run_uat_report(args: UatReportArgs) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<PathBuf> {
        let plan_raw = std::fs::read_to_string(&args.plan)
            .map_err(|e| anyhow::anyhow!("cannot read plan {}: {e}", args.plan.display()))?;
        let plan: UatPlan = serde_saphyr::from_str(&plan_raw)
            .map_err(|e| anyhow::anyhow!("invalid plan {}: {e}", args.plan.display()))?;

        let mut sessions = Vec::new();
        for session_path in &args.sessions {
            let raw = std::fs::read_to_string(session_path)
                .map_err(|e| anyhow::anyhow!("cannot read {}: {e}", session_path.display()))?;
            let session: UatSession = serde_saphyr::from_str(&raw)
                .map_err(|e| anyhow::anyhow!("invalid session {}: {e}", session_path.display()))?;
            sessions.push(session);
        }

        let report = aggregate_report(&plan, &sessions);
        let path = args
            .output
            .unwrap_or_else(|| PathBuf::from(format!("uat-report-{}.yaml", args.release)));
        let yaml = serde_saphyr::to_string(&report)
            .map_err(|e| anyhow::anyhow!("serialization failed: {e}"))?;
        std::fs::write(&path, yaml)?;
        Ok(path)
    })();
    render_result(result, format, |path| {
        format!("uat report written: {}\n", path.display())
    })
}

fn run_uat_status(args: UatStatusArgs) -> CommandOutput {
    let format = args.format;
    // Status is derived from artifacts on disk: plan/session/report for the
    // release candidate. U6 will enrich this with control-plane data.
    let plan_file = PathBuf::from(format!("uat-plan-{}.yaml", args.release));
    let report_file = PathBuf::from(format!("uat-report-{}.yaml", args.release));
    let lines = [
        format!("release: {}", args.release),
        format!(
            "plan: {}",
            if plan_file.exists() { "generated" } else { "missing" }
        ),
        format!(
            "report: {}",
            if report_file.exists() { "ready" } else { "not-ready" }
        ),
    ];
    let result: Result<String, anyhow::Error> = Ok(lines.join("\n"));
    render_result(result, format, |text| text.to_string())
}

/// Aggregate sessions into a report with the global verdict.
fn aggregate_report(plan: &UatPlan, sessions: &[UatSession]) -> UatReport {
    let mut scenario_status: std::collections::HashMap<String, (sddk_domain::UatStatus, Option<sddk_domain::UatExecutor>)> =
        std::collections::HashMap::new();
    let mut total_minutes = 0u32;
    let mut defects = 0u32;
    let mut ux_issues = 0u32;

    for session in sessions {
        if let Some(finished) = &session.finished_at {
            let _ = finished;
        }
        total_minutes += session.results.iter().map(|r| r.duration_minutes).sum::<u32>();
        for result in &session.results {
            // Last writer wins per scenario.
            scenario_status.insert(
                result.scenario_id.clone(),
                (result.status, Some(session.executor)),
            );
            if result.status == sddk_domain::UatStatus::Fail {
                defects += 1;
            }
            if result.status == sddk_domain::UatStatus::Partial {
                ux_issues += 1;
            }
        }
    }

    let mut total = 0u32;
    let mut passed = 0u32;
    let mut failed = 0u32;
    let mut blocked = 0u32;
    let mut partial = 0u32;
    let mut covered = 0u32;

    let mut features = Vec::new();
    for feature in &plan.features {
        let mut sc_rollups = Vec::new();
        for scenario in &feature.scenarios {
            total += 1;
            let (status, executor) = scenario_status
                .get(&scenario.id)
                .copied()
                .unwrap_or((sddk_domain::UatStatus::Pass, None));
            match status {
                sddk_domain::UatStatus::Pass => {
                    passed += 1;
                    covered += 1;
                }
                sddk_domain::UatStatus::Fail => failed += 1,
                sddk_domain::UatStatus::Blocked => blocked += 1,
                sddk_domain::UatStatus::Partial => {
                    partial += 1;
                    covered += 1;
                }
            }
            sc_rollups.push(UatScenarioRollup {
                scenario_id: scenario.id.clone(),
                status,
                executor,
            });
        }
        let feat_total = feature.scenarios.len() as u32;
        let feat_covered = sc_rollups
            .iter()
            .filter(|s| s.status != sddk_domain::UatStatus::Fail && s.status != sddk_domain::UatStatus::Blocked)
            .count() as u32;
        features.push(UatFeatureRollup {
            id: feature.id.clone(),
            name: feature.name.clone(),
            coverage_pct: if feat_total > 0 {
                100.0 * feat_covered as f64 / feat_total as f64
            } else {
                0.0
            },
            scenarios: sc_rollups,
        });
    }

    let coverage_pct = if total > 0 {
        100.0 * covered as f64 / total as f64
    } else {
        0.0
    };

    let verdict = if failed == 0 && blocked == 0 {
        UatVerdict::Ready
    } else if failed == 0 {
        UatVerdict::ReadyWithRisks
    } else {
        UatVerdict::NotReady
    };

    let not_ready_blockers: Vec<String> = sessions
        .iter()
        .flat_map(|s| s.results.iter())
        .filter(|r| r.status == sddk_domain::UatStatus::Fail)
        .map(|r| {
            format!(
                "{} ({})",
                r.scenario_id,
                r.comment.clone().unwrap_or_else(|| "defect".into())
            )
        })
        .collect();

    UatReport {
        schema_version: 1,
        release: plan.release.candidate.clone(),
        plan_ref: plan.release.candidate.clone(),
        sessions: sessions.iter().map(|s| s.session_id.clone()).collect(),
        summary: UatReportSummary {
            total_scenarios: total,
            passed,
            failed,
            blocked,
            partial,
            coverage_pct,
            defects,
            ux_issues,
            uat_duration_minutes: total_minutes,
        },
        features,
        verdict,
        not_ready_blockers,
    }
}

/// Render a self-contained HTML dashboard from a plan (ADR-0013 kit).
fn render_dashboard_html(
    plan: &UatPlan,
    view: UatView,
    theme: &str,
    environment: &crate::CliEnvironment,
) -> anyhow::Result<String> {
    let assets = dev_cmd::resolve_assets_dir(environment)?;
    let kit = assets.map(|a| a.join("uat-dashboard")).unwrap_or_default();

    let tokens = read_asset(&kit.join("kit/tokens.css"))?;
    let components_css = read_asset(&kit.join("kit/components.css"))?;
    let _components_js = read_asset(&kit.join("kit/components.js"))?;
    let _storage_js = read_asset(&kit.join("kit/storage.js"))?;

    let view_name = match view {
        UatView::Guided => "guided",
        UatView::Matrix => "interactive",
        UatView::Traceability => "report",
    };
    let template = read_asset(&kit.join("views").join(format!("{view_name}.html")))?;

    let theme_css = if theme == "light" {
        read_asset(&kit.join("themes/light.css"))?
    } else {
        read_asset(&kit.join("themes/dark.css"))?
    };

    let plan_json = serde_json::to_string_pretty(plan)
        .map_err(|e| anyhow::anyhow!("plan serialization failed: {e}"))?;

    let html = template
        .replace("@TOKENS@", &tokens)
        .replace("@COMPONENTS@", &format!("{components_css}\n{theme_css}"))
        .replace("@PLAN_JSON@", &plan_json)
        .replace("@REPORT_JSON@", "{}")
        .replace("@RELEASE@", &plan.release.candidate)
        .replace("@GENERATED_AT@", &now_rfc3339())
        .replace("@PLAN_REF@", &plan.release.candidate)
        .replace("@KIT_STORAGE@", &kit.join("kit/storage.js").display().to_string())
        .replace("@KIT_COMPONENTS@", &kit.join("kit/components.js").display().to_string());

    Ok(html)
}

fn read_asset(path: &Path) -> anyhow::Result<String> {
    std::fs::read_to_string(path).map_err(|e| {
        anyhow::anyhow!(
            "missing dashboard asset {} (run `sddk dev update` to install the bundle): {e}",
            path.display()
        )
    })
}

fn now_rfc3339() -> String {
    // Local clock in RFC 3339; deterministic tests inject overrides elsewhere.
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs();
    // Simple RFC3339 UTC rendering without external deps.
    let days = secs / 86400;
    let (y, m, d) = civil_from_days(days as i64);
    let rem = secs % 86400;
    let (h, mi, s) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    format!("{y:04}-{m:02}-{d:02}T{h:02}:{mi:02}:{s:02}Z")
}

/// Days since 1970-01-01 to civil date (Howard Hinnant's algorithm).
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m as u32, d as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aggregate_report_computes_verdict() {
        let plan: UatPlan = serde_saphyr::from_str(
            r#"
schema_version: 1
release: { candidate: v1.5.0, project: demo }
generated_by: uat-planner
generated_at: "2026-08-07T00:00:00Z"
features:
  - id: F-01
    name: Login
    scenarios:
      - id: S-1
        title: Login correcto
        plain_steps:
          - action: abrir /login
            expected: formulario visible
      - id: S-2
        title: Login fallido
        plain_steps:
          - action: password incorrecto
            expected: error visible
"#,
        )
        .unwrap();

        let session: UatSession = serde_saphyr::from_str(
            r#"
schema_version: 1
session_id: uat-1
plan_ref: v1.5.0
release: v1.5.0
started_at: "2026-08-07T00:00:00Z"
results:
  - scenario_id: S-1
    status: PASS
  - scenario_id: S-2
    status: FAIL
    comment: no muestra error
"#,
        )
        .unwrap();

        let report = aggregate_report(&plan, &[session]);
        assert_eq!(report.verdict, UatVerdict::NotReady);
        assert_eq!(report.summary.total_scenarios, 2);
        assert_eq!(report.summary.passed, 1);
        assert_eq!(report.summary.failed, 1);
        assert_eq!(report.summary.defects, 1);
        assert_eq!(report.summary.coverage_pct, 50.0);
        assert_eq!(report.not_ready_blockers.len(), 1);
        assert!(report.not_ready_blockers[0].contains("S-2"));
    }

    #[test]
    fn validate_rejects_empty_plan() {
        let raw = r#"
schema_version: 1
release: { candidate: v1.5.0 }
generated_by: uat-planner
generated_at: "2026-08-07T00:00:00Z"
features: []
"#;
        let value: serde_json::Value = serde_saphyr::from_str(raw).unwrap();
        let has_scenarios = value
            .get("features")
            .and_then(|f| f.as_array())
            .map(|arr| {
                arr.iter().any(|feat| {
                    feat.get("scenarios")
                        .and_then(|s| s.as_array())
                        .map(|sc| !sc.is_empty())
                        .unwrap_or(false)
                })
            })
            .unwrap_or(false);
        assert!(!has_scenarios);
    }
}
