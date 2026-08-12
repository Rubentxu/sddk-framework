//! E14.5 — Pipeline runner for the generate command.
//!
//! Stages: optional discover → plan → enrich → quality → interactive approval → validate.
//! Propagates status/error non-zero on any failure (no continue after failure).
//!
//! Atomic write: plan is ONLY written after ALL stages pass.
//! No intermediate files are written. On error, no output file exists.

use crate::uat_common::io::{ApprovalIo, ApprovalVerdict, StdioApprovalIo, UatPlanSummary};
use crate::uat_common::plan_io::atomic_write_plan;
use sddk_domain::{UatPlanApproval, validate_form_dsl};
use std::path::{Path, PathBuf};

use super::planner::{PlanOutput, build_plan};
use super::validator::validate_inputs;

/// Pipeline errors.
#[derive(Debug)]
pub enum PipelineError {
    /// Validation failed before any file operation.
    ValidationFailed(String),
    /// Planning failed (no features extracted).
    PlanningFailed(String),
    /// Quality gate failed (blockers found).
    QualityFailed(String),
    /// Validation of final plan failed.
    SchemaValidationFailed(String),
    /// Approval rejected.
    ApprovalRejected,
    /// Approval edit requested (not supported in this version).
    ApprovalEditRequested,
    /// Discovery failed.
    DiscoveryFailed(String),
    /// IO error during pipeline.
    IoError(String),
}

impl std::fmt::Display for PipelineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PipelineError::ValidationFailed(msg) => {
                write!(f, "validation failed: {}", msg)
            }
            PipelineError::PlanningFailed(msg) => {
                write!(f, "planning failed: {}", msg)
            }
            PipelineError::QualityFailed(msg) => {
                write!(f, "quality gate failed: {}", msg)
            }
            PipelineError::SchemaValidationFailed(msg) => {
                write!(f, "schema validation failed: {}", msg)
            }
            PipelineError::ApprovalRejected => {
                write!(f, "approval rejected")
            }
            PipelineError::ApprovalEditRequested => {
                write!(f, "approval edit requested (not supported)")
            }
            PipelineError::DiscoveryFailed(msg) => {
                write!(f, "discovery failed: {}", msg)
            }
            PipelineError::IoError(msg) => {
                write!(f, "IO error: {}", msg)
            }
        }
    }
}

/// Stage output with path and status.
#[derive(Debug, Clone)]
pub struct StageOutput {
    pub stage: &'static str,
    pub path: PathBuf,
    pub tag: String,
    /// Exit status code for this stage. Intentionally part of public API.
    #[allow(dead_code)]
    pub status: i32,
    pub message: String,
}

/// Pipeline configuration.
pub struct PipelineConfig {
    pub release: String,
    pub requirements: Option<PathBuf>,
    pub changelog: Option<PathBuf>,
    pub last_plan: Option<PathBuf>,
    pub discover: bool,
    pub app_url: Option<String>,
    pub interactive: bool,
    pub output: Option<PathBuf>,
    /// Approval IO handler (injected for testing).
    pub approval_io: Option<Box<dyn ApprovalIo>>,
    /// Force quality gate to fail (test-only).
    /// When true, the quality stage always returns QualityFailed regardless of
    /// actual plan quality. This allows testing the stage gate behavior
    /// without needing to construct plans with actual quality issues.
    pub force_quality_failure: bool,
}

impl PipelineConfig {
    /// Default output path for the plan.
    pub fn output_path(&self) -> PathBuf {
        self.output
            .clone()
            .unwrap_or_else(|| PathBuf::from(format!("uat-plan-{}.yaml", self.release)))
    }
}

/// Run the full generate pipeline.
/// Returns StageOutput for each completed stage on success,
/// or PipelineError on any failure (no partial output).
///
/// Atomicity: NO file writes until after ALL gates pass.
/// On any error, no output file exists.
pub fn run_pipeline(config: PipelineConfig) -> Result<Vec<StageOutput>, PipelineError> {
    let mut stages = Vec::new();

    // Extract all config fields we need, so we can consume config
    let release = config.release.clone();
    let requirements = config.requirements.clone();
    let changelog = config.changelog.clone();
    let last_plan = config.last_plan.clone();
    let discover = config.discover;
    let app_url = config.app_url.clone();
    let interactive = config.interactive;
    // Call output_path before moving other fields
    let output_path = config.output_path();
    let approval_io = config.approval_io;

    // ── Stage 0: Validate inputs (before ANY file operation) ──────────────────
    validate_inputs(&requirements, &changelog, &last_plan, discover, &app_url)
        .map_err(|e| PipelineError::ValidationFailed(format!("{:?}", e)))?;

    // ── Stage 1: Optional Discover ────────────────────────────────────────────
    let aam_scenario_candidates = if discover {
        let url = app_url.as_ref().unwrap();
        match run_discover(url) {
            Ok(candidates) => {
                stages.push(StageOutput {
                    stage: "discover",
                    path: PathBuf::from("N/A"),
                    tag: "discovered".to_string(),
                    status: 0,
                    message: format!("discover: {} scenario candidates", candidates.len()),
                });
                candidates
            }
            Err(e) => return Err(PipelineError::DiscoveryFailed(e)),
        }
    } else {
        stages.push(StageOutput {
            stage: "discover",
            path: PathBuf::from("N/A"),
            tag: "skipped".to_string(),
            status: 0,
            message: "discover: skipped (no --discover)".to_string(),
        });
        Vec::new()
    };

    // ── Stage 2: Plan (pure planner, in-memory) ──────────────────────────────
    let plan_output: PlanOutput = build_plan(
        &release,
        &requirements,
        &changelog,
        &last_plan,
        &aam_scenario_candidates,
    )
    .map_err(|e| PipelineError::PlanningFailed(format!("{:?}", e)))?;

    stages.push(StageOutput {
        stage: "plan",
        path: PathBuf::from("N/A"),
        tag: "planned".to_string(),
        status: 0,
        message: format!(
            "plan: {} features, {} scenarios",
            plan_output.plan.features.len(),
            plan_output
                .plan
                .features
                .iter()
                .map(|f| f.scenarios.len())
                .sum::<usize>()
        ),
    });

    // ── Stage 3: Enrich (in-memory, no file write) ───────────────────────────
    let mut enriched_plan = plan_output.plan;
    for feature in &mut enriched_plan.features {
        for scenario in &mut feature.scenarios {
            crate::uat_enrich::enrich_scenario(scenario);
        }
    }

    stages.push(StageOutput {
        stage: "enrich",
        path: PathBuf::from("N/A"),
        tag: "enriched".to_string(),
        status: 0,
        message: "enrich: forms + provenance set".to_string(),
    });

    // ── Stage 4: Quality (in-memory gate) ─────────────────────────────────────
    let quality_report = run_quality_stage(&enriched_plan, config.force_quality_failure);

    if !quality_report.summary.pass {
        let blockers = quality_report.summary.blockers;
        return Err(PipelineError::QualityFailed(format!(
            "{} blockers found",
            blockers
        )));
    }

    stages.push(StageOutput {
        stage: "quality",
        path: PathBuf::from("N/A"),
        tag: "quality_pass".to_string(),
        status: 0,
        message: format!(
            "quality: {} smells ({} blockers, {} warnings) — PASS",
            quality_report.summary.total,
            quality_report.summary.blockers,
            quality_report.summary.warnings
        ),
    });

    // ── Stage 5: Interactive Approval ──────────────────────────────────────
    if interactive {
        let mut io: Box<dyn ApprovalIo> =
            approval_io.unwrap_or_else(|| Box::new(StdioApprovalIo::default()));

        let summary = UatPlanSummary::from(&enriched_plan);
        let decision = io
            .prompt(&summary)
            .map_err(|e| PipelineError::IoError(e.to_string()))?;

        match decision.verdict {
            ApprovalVerdict::Approve => {
                // io.record must be called AFTER approval and BEFORE atomic_write.
                // Error aborts pipeline (no file written).
                io.record(&decision)
                    .map_err(|e| PipelineError::IoError(e.to_string()))?;

                let approval = UatPlanApproval {
                    id: decision.id.clone(),
                    display: decision.display.clone(),
                    approved_at: decision.at.clone(),
                };
                enriched_plan.approval = Some(approval);
                stages.push(StageOutput {
                    stage: "approval",
                    path: PathBuf::from("N/A"),
                    tag: "approved".to_string(),
                    status: 0,
                    message: format!(
                        "approval: approved by {} ({})",
                        decision.display, decision.id
                    ),
                });
            }
            ApprovalVerdict::Reject => return Err(PipelineError::ApprovalRejected),
            ApprovalVerdict::Edit => return Err(PipelineError::ApprovalEditRequested),
        }
    } else {
        stages.push(StageOutput {
            stage: "approval",
            path: PathBuf::from("N/A"),
            tag: "auto_skip".to_string(),
            status: 0,
            message: "approval: auto mode — no human approval recorded".to_string(),
        });
    }

    // ── Stage 6: Final Validate (in-memory schema + form DSL) ─────────────────
    if enriched_plan.features.is_empty() {
        return Err(PipelineError::PlanningFailed(
            "final plan has no features".to_string(),
        ));
    }
    let total_scenarios: usize = enriched_plan
        .features
        .iter()
        .map(|f| f.scenarios.len())
        .sum();
    if total_scenarios == 0 {
        return Err(PipelineError::PlanningFailed(
            "final plan has no scenarios".to_string(),
        ));
    }

    // Validate form DSL for all scenarios (closed vocabulary check)
    let mut dsl_errors: Vec<String> = Vec::new();
    for feature in &enriched_plan.features {
        for scenario in &feature.scenarios {
            if let Some(form) = &scenario.form {
                for error in validate_form_dsl(form) {
                    dsl_errors.push(format!("{}: {}", scenario.id, error));
                }
            }
        }
    }
    if !dsl_errors.is_empty() {
        return Err(PipelineError::SchemaValidationFailed(format!(
            "form DSL validation failed:\n  {}",
            dsl_errors.join("\n  ")
        )));
    }

    stages.push(StageOutput {
        stage: "validate",
        path: PathBuf::from("N/A"),
        tag: "validated".to_string(),
        status: 0,
        message: format!(
            "validate: {} features, {} scenarios — OK",
            enriched_plan.features.len(),
            total_scenarios
        ),
    });

    // ── ATOMIC WRITE: Only write after ALL gates pass ─────────────────────────
    // Use temp file + rename for atomicity
    atomic_write_plan(&enriched_plan, &output_path)
        .map_err(|e| PipelineError::IoError(e.to_string()))?;

    // Update final stage path to the actual output path
    stages.push(StageOutput {
        stage: "write",
        path: output_path.clone(),
        tag: "written".to_string(),
        status: 0,
        message: format!("written: {}", output_path.display()),
    });

    Ok(stages)
}

/// Run the quality stage: detect smells in the plan.
fn run_quality_stage(
    plan: &sddk_domain::UatPlan,
    force_quality_failure: bool,
) -> crate::uat_quality::report::QualityReport {
    if force_quality_failure {
        // Test injection: force quality failure without needing actual quality issues.
        // This follows the spec's requirement for test-only quality gate injection.
        crate::uat_quality::report::QualityReport {
            schema_version: 1,
            analyzer: "pipeline-test-injection".to_string(),
            model: "test-v1".to_string(),
            analyzed_at: crate::uat_common::time::now_rfc3339(),
            plan_ref: String::new(),
            smells: vec![crate::uat_quality::report::QualitySmell {
                id: "TEST-001".to_string(),
                smell_id: "injected-blocker".to_string(),
                severity: "BLOCKER".to_string(),
                location: crate::uat_quality::report::SmellLocation {
                    feature_id: "F-01".to_string(),
                    scenario_id: "S-001".to_string(),
                    item_id: None,
                    field: None,
                },
                snippet: Some("test snippet".to_string()),
                suggestion: "Remove test injection".to_string(),
                auto_fixable: false,
            }],
            summary: crate::uat_quality::report::QualitySummary {
                total: 1,
                blockers: 1,
                errors: 0,
                warnings: 0,
                suggestions: 0,
                pass: false,
            },
            verdict: "NEEDS_REVISION".to_string(),
            threshold_applied: "BLOCKER".to_string(),
        }
    } else {
        crate::uat_quality::detect_13_smells(
            plan,
            crate::uat_quality::report::QualityThreshold::Blocker,
        )
    }
}

/// Run discovery and return scenario candidates.
/// Does NOT do pre-health check - relies on discover() fallback behavior.
/// Uses configurable Fara URL and budget from config.
fn run_discover(app_url: &str) -> Result<Vec<crate::uat_discover::AamScenarioCandidate>, String> {
    // Fara URL from environment or default
    let fara_url = std::env::var("FARA_URL")
        .ok()
        .unwrap_or_else(|| "http://127.0.0.1:8082".to_string());

    // Budget from environment or default
    let budget: u32 = std::env::var("FARA_BUDGET")
        .ok()
        .and_then(|b| b.parse().ok())
        .unwrap_or(50);

    // Goals from environment or default
    let goals: Vec<String> = std::env::var("FARA_GOALS")
        .ok()
        .map(|g| g.split(',').map(String::from).collect())
        .unwrap_or_else(|| vec!["Explore the main functionality".to_string()]);

    let args = crate::uat::DiscoverArgs {
        app_url: app_url.to_string(),
        entry: "/".to_string(),
        goals,
        budget,
        fara_url: Some(fara_url),
        output: None,
        format: crate::OutputFormat::Text,
    };

    let outcome =
        crate::uat_discover::discover(&args).map_err(|e| format!("discovery failed: {}", e))?;

    Ok(outcome.aam.scenario_candidates)
}

/// Render pipeline output as string.
pub fn render_pipeline_output(stages: &[StageOutput], final_path: &Path) -> String {
    let mut lines = Vec::new();
    for stage in stages {
        lines.push(format!(
            "  [{}] {}: {} ({})",
            stage.stage,
            stage.tag,
            stage.message,
            stage.path.display()
        ));
    }
    lines.push(String::new());
    lines.push(format!("Pipeline complete: {}", final_path.display()));
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::uat_common::io::{ApprovalDecision, ApprovalIo, ApprovalVerdict, UatPlanSummary};

    /// Scripted approval that always approves.
    struct FakeApprove;
    impl ApprovalIo for FakeApprove {
        fn prompt(&mut self, _draft: &UatPlanSummary) -> anyhow::Result<ApprovalDecision> {
            Ok(ApprovalDecision::new(
                ApprovalVerdict::Approve,
                "T-test".to_string(),
                "Test User".to_string(),
            ))
        }
        fn record(&mut self, _decision: &ApprovalDecision) -> anyhow::Result<()> {
            Ok(())
        }
    }

    /// Scripted approval that always rejects.
    struct FakeReject;
    impl ApprovalIo for FakeReject {
        fn prompt(&mut self, _draft: &UatPlanSummary) -> anyhow::Result<ApprovalDecision> {
            Ok(ApprovalDecision::new(
                ApprovalVerdict::Reject,
                "T-test".to_string(),
                "Test User".to_string(),
            ))
        }
        fn record(&mut self, _decision: &ApprovalDecision) -> anyhow::Result<()> {
            Ok(())
        }
    }

    /// Scripted approval that always requests edit.
    struct FakeEdit;
    impl ApprovalIo for FakeEdit {
        fn prompt(&mut self, _draft: &UatPlanSummary) -> anyhow::Result<ApprovalDecision> {
            Ok(ApprovalDecision::new(
                ApprovalVerdict::Edit,
                "T-test".to_string(),
                "Test User".to_string(),
            ))
        }
        fn record(&mut self, _decision: &ApprovalDecision) -> anyhow::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn pipeline_empty_source_returns_validation_error() {
        let td = tempfile::TempDir::new().unwrap();

        let config = PipelineConfig {
            release: "v1.0.0".to_string(),
            requirements: None,
            changelog: None,
            last_plan: None,
            discover: false,
            app_url: None,
            interactive: false,
            output: Some(td.path().join("uat-plan.yaml")),
            approval_io: None,
            force_quality_failure: false,
        };

        let result = run_pipeline(config);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, PipelineError::ValidationFailed(_)));
        assert!(format!("{:?}", err).contains("RequirementsRequired"));
    }

    #[test]
    fn pipeline_reject_approval_returns_approval_rejected() {
        let td = tempfile::TempDir::new().unwrap();
        let req_dir = td.path();
        // Requirements with proper content that will generate scenarios
        // that pass quality
        std::fs::write(
            req_dir.join("req.md"),
            "# Requirements\n\n## Login Feature\n- User can login with email and password\n- System returns JSON response with status 200\n",
        )
        .unwrap();

        let config = PipelineConfig {
            release: "v1.0.0".to_string(),
            requirements: Some(req_dir.to_path_buf()),
            changelog: None,
            last_plan: None,
            discover: false,
            app_url: None,
            interactive: true,
            output: Some(td.path().join("uat-plan.yaml")),
            approval_io: Some(Box::new(FakeReject)),
            force_quality_failure: false,
        };

        let result = run_pipeline(config);
        assert!(result.is_err());
        let e = result.unwrap_err();
        assert!(matches!(e, PipelineError::ApprovalRejected));

        // No output file should exist because approval was rejected
        let output_path = td.path().join("uat-plan.yaml");
        assert!(
            !output_path.exists(),
            "output file should not exist after rejection"
        );
    }

    #[test]
    fn pipeline_approve_creates_output_with_approval() {
        let td = tempfile::TempDir::new().unwrap();
        let req_dir = td.path();
        std::fs::write(
            req_dir.join("req.md"),
            "# Requirements\n\n## Login Feature\n- User can login with email and password\n",
        )
        .unwrap();

        let config = PipelineConfig {
            release: "v1.0.0".to_string(),
            requirements: Some(req_dir.to_path_buf()),
            changelog: None,
            last_plan: None,
            discover: false,
            app_url: None,
            interactive: true,
            output: Some(td.path().join("uat-plan.yaml")),
            approval_io: Some(Box::new(FakeApprove)),
            force_quality_failure: false,
        };

        let result = run_pipeline(config);
        assert!(result.is_ok(), "pipeline should succeed: {:?}", result);
        let _stages = result.unwrap();

        // Output file should exist
        let output_path = td.path().join("uat-plan.yaml");
        assert!(
            output_path.exists(),
            "output file should exist after approval"
        );

        // Check that approval was recorded
        let content = std::fs::read_to_string(&output_path).unwrap();
        assert!(
            content.contains("approval"),
            "plan should contain approval record"
        );
    }

    #[test]
    fn pipeline_auto_skips_approval() {
        let td = tempfile::TempDir::new().unwrap();
        let req_dir = td.path();
        std::fs::write(
            req_dir.join("req.md"),
            "# Requirements\n\n## Login Feature\n- User can login with email and password\n",
        )
        .unwrap();

        let config = PipelineConfig {
            release: "v1.0.0".to_string(),
            requirements: Some(req_dir.to_path_buf()),
            changelog: None,
            last_plan: None,
            discover: false,
            app_url: None,
            interactive: false, // Auto mode
            output: Some(td.path().join("uat-plan.yaml")),
            approval_io: None,
            force_quality_failure: false,
        };

        let result = run_pipeline(config);
        assert!(result.is_ok(), "auto mode should succeed: {:?}", result);
        let stages = result.unwrap();

        // Should have auto_skip tag for approval stage
        let approval_stage = stages.iter().find(|s| s.stage == "approval");
        assert!(
            approval_stage.is_some_and(|s| s.tag == "auto_skip"),
            "approval stage should be auto_skip in non-interactive mode"
        );
    }

    #[test]
    fn pipeline_edit_returns_approval_edit_requested() {
        let td = tempfile::TempDir::new().unwrap();
        let req_dir = td.path();
        std::fs::write(
            req_dir.join("req.md"),
            "# Requirements\n\n## Login Feature\n- User can login with email and password\n",
        )
        .unwrap();

        let config = PipelineConfig {
            release: "v1.0.0".to_string(),
            requirements: Some(req_dir.to_path_buf()),
            changelog: None,
            last_plan: None,
            discover: false,
            app_url: None,
            interactive: true,
            output: Some(td.path().join("uat-plan.yaml")),
            approval_io: Some(Box::new(FakeEdit)),
            force_quality_failure: false,
        };

        let result = run_pipeline(config);
        assert!(result.is_err());
        let e = result.unwrap_err();
        assert!(matches!(e, PipelineError::ApprovalEditRequested));

        // No output file should exist
        let output_path = td.path().join("uat-plan.yaml");
        assert!(
            !output_path.exists(),
            "output file should not exist after edit request"
        );
    }

    /// Test that quality failure blocks pipeline and produces no output.
    /// Uses force_quality_failure injection to test the stage gate behavior
    /// without needing to construct plans with actual quality issues.
    #[test]
    fn pipeline_atomic_no_partial_on_quality_failure() {
        let td = tempfile::TempDir::new().unwrap();
        let req_dir = td.path();
        std::fs::write(
            req_dir.join("req.md"),
            "# Requirements\n\n## Login\n- User can login\n",
        )
        .unwrap();

        let config = PipelineConfig {
            release: "v1.0.0".to_string(),
            requirements: Some(req_dir.to_path_buf()),
            changelog: None,
            last_plan: None,
            discover: false,
            app_url: None,
            interactive: false,
            output: Some(td.path().join("uat-plan.yaml")),
            approval_io: None,
            force_quality_failure: true, // Injected quality failure
        };

        let result = run_pipeline(config);
        // Pipeline MUST fail with QualityFailed (not silently)
        assert!(
            matches!(result, Err(PipelineError::QualityFailed(_))),
            "Expected QualityFailed, got: {:?}",
            result
        );

        // No output file should exist (atomic rule)
        let output_path = td.path().join("uat-plan.yaml");
        assert!(
            !output_path.exists(),
            "no output file should exist on pipeline failure"
        );

        // Also verify no temp files left behind
        let tmp_files: Vec<_> = std::fs::read_dir(td.path())
            .unwrap()
            .flatten()
            .filter(|e| e.file_name().to_string_lossy().contains(".tmp-"))
            .collect();
        assert!(
            tmp_files.is_empty(),
            "no .tmp-* files should exist after failure"
        );
    }

    /// Test that io.record is called after approval but before persistence.
    /// Uses shared state to verify record was invoked.
    #[test]
    fn pipeline_approve_calls_io_record_before_persistence() {
        use std::sync::{Arc, Mutex};

        let td = tempfile::TempDir::new().unwrap();
        let req_dir = td.path();
        std::fs::write(
            req_dir.join("req.md"),
            "# Requirements\n\n## Login\n- User can login\n",
        )
        .unwrap();

        // Track whether record was called
        let record_called = Arc::new(Mutex::new(false));
        let record_called_clone = record_called.clone();

        // Scripted approval that records the call
        struct FakeApproveWithRecord {
            called: Arc<Mutex<bool>>,
        }
        impl ApprovalIo for FakeApproveWithRecord {
            fn prompt(&mut self, _draft: &UatPlanSummary) -> anyhow::Result<ApprovalDecision> {
                Ok(ApprovalDecision::new(
                    ApprovalVerdict::Approve,
                    "T-test".to_string(),
                    "Test User".to_string(),
                ))
            }
            fn record(&mut self, _decision: &ApprovalDecision) -> anyhow::Result<()> {
                *self.called.lock().unwrap() = true;
                Ok(())
            }
        }

        let config = PipelineConfig {
            release: "v1.0.0".to_string(),
            requirements: Some(req_dir.to_path_buf()),
            changelog: None,
            last_plan: None,
            discover: false,
            app_url: None,
            interactive: true, // requires approval
            output: Some(td.path().join("uat-plan.yaml")),
            approval_io: Some(Box::new(FakeApproveWithRecord {
                called: record_called_clone,
            })),
            force_quality_failure: false,
        };

        let result = run_pipeline(config);
        assert!(
            result.is_ok(),
            "pipeline should succeed with FakeApprove: {:?}",
            result
        );

        // io.record MUST have been called
        assert!(
            *record_called.lock().unwrap(),
            "io.record must be called after approval before persistence"
        );

        // Verify output exists (persistence happened after record)
        let output_path = td.path().join("uat-plan.yaml");
        assert!(
            output_path.exists(),
            "output should exist after successful pipeline"
        );
    }

    /// Test that render_pipeline_output includes stage tags and final path.
    #[test]
    fn render_pipeline_output_includes_tags_and_path() {
        use std::path::PathBuf;

        let stages = vec![
            StageOutput {
                stage: "discover",
                path: PathBuf::from("N/A"),
                tag: "skipped".to_string(),
                status: 0,
                message: "discover: skipped".to_string(),
            },
            StageOutput {
                stage: "plan",
                path: PathBuf::from("N/A"),
                tag: "planned".to_string(),
                status: 0,
                message: "plan: 1 features".to_string(),
            },
            StageOutput {
                stage: "write",
                path: PathBuf::from("/tmp/uat-plan-v1.0.0.yaml"),
                tag: "written".to_string(),
                status: 0,
                message: "written: /tmp/uat-plan-v1.0.0.yaml".to_string(),
            },
        ];
        let final_path = PathBuf::from("/tmp/uat-plan-v1.0.0.yaml");

        let output = render_pipeline_output(&stages, &final_path);

        // Must contain stage tags
        assert!(
            output.contains("[discover]"),
            "output must include discover stage tag"
        );
        assert!(
            output.contains("[plan]"),
            "output must include plan stage tag"
        );
        assert!(
            output.contains("[write]"),
            "output must include write stage tag"
        );

        // Must contain tag values
        assert!(output.contains("skipped"), "output must include tag values");
        assert!(output.contains("planned"), "output must include tag values");
        assert!(output.contains("written"), "output must include tag values");

        // Must include final path
        assert!(
            output.contains("/tmp/uat-plan-v1.0.0.yaml"),
            "output must include final path"
        );
        assert!(
            output.contains("Pipeline complete"),
            "output must include Pipeline complete marker"
        );
    }

    /// Test that pipeline in auto mode (non-interactive) does not include approval in output.
    #[test]
    fn pipeline_auto_mode_approval_absent() {
        let td = tempfile::TempDir::new().unwrap();
        let req_dir = td.path();
        std::fs::write(
            req_dir.join("req.md"),
            "# Requirements\n\n## Login\n- User can login with email and password\n",
        )
        .unwrap();

        let output_path = td.path().join("uat-plan.yaml");
        let config = PipelineConfig {
            release: "v1.0.0".to_string(),
            requirements: Some(req_dir.to_path_buf()),
            changelog: None,
            last_plan: None,
            discover: false,
            app_url: None,
            interactive: false, // auto mode
            output: Some(output_path.clone()),
            approval_io: None,
            force_quality_failure: false,
        };

        let result = run_pipeline(config);
        assert!(result.is_ok(), "auto mode should succeed: {:?}", result);

        // Output should exist
        assert!(output_path.exists(), "output should exist in auto mode");

        // Content should NOT contain approval record (auto mode)
        let content = std::fs::read_to_string(&output_path).unwrap();
        assert!(
            !content.contains("approval:"),
            "auto mode should not include approval in output"
        );
    }

    /// Test that io.record failure blocks pipeline and produces no output (atomicity).
    /// When record() returns an error, the pipeline should fail BEFORE atomic_write,
    /// ensuring no output file exists.
    #[test]
    fn pipeline_atomic_no_output_on_record_failure() {
        use std::sync::{Arc, Mutex};

        let td = tempfile::TempDir::new().unwrap();
        let req_dir = td.path();
        std::fs::write(
            req_dir.join("req.md"),
            "# Requirements\n\n## Login\n- User can login\n",
        )
        .unwrap();

        // Track whether record was called
        let record_called = Arc::new(Mutex::new(false));
        let record_called_clone = record_called.clone();

        struct FakeApproveWithRecordError {
            called: Arc<Mutex<bool>>,
        }
        impl ApprovalIo for FakeApproveWithRecordError {
            fn prompt(&mut self, _draft: &UatPlanSummary) -> anyhow::Result<ApprovalDecision> {
                Ok(ApprovalDecision::new(
                    ApprovalVerdict::Approve,
                    "T-test".to_string(),
                    "Test User".to_string(),
                ))
            }
            fn record(&mut self, _decision: &ApprovalDecision) -> anyhow::Result<()> {
                *self.called.lock().unwrap() = true;
                Err(anyhow::anyhow!("simulated record failure"))
            }
        }

        let config = PipelineConfig {
            release: "v1.0.0".to_string(),
            requirements: Some(req_dir.to_path_buf()),
            changelog: None,
            last_plan: None,
            discover: false,
            app_url: None,
            interactive: true,
            output: Some(td.path().join("uat-plan.yaml")),
            approval_io: Some(Box::new(FakeApproveWithRecordError {
                called: record_called_clone,
            })),
            force_quality_failure: false,
        };

        let result = run_pipeline(config);
        // Pipeline MUST fail because record() returned an error
        assert!(
            result.is_err(),
            "Expected pipeline to fail on record error, got: {:?}",
            result
        );

        // io.record was called (proving it happens before atomicity check)
        assert!(
            *record_called.lock().unwrap(),
            "io.record must have been called"
        );

        // No output file should exist (atomic rule - failure before write)
        let output_path = td.path().join("uat-plan.yaml");
        assert!(
            !output_path.exists(),
            "no output file should exist when record fails"
        );
    }
}
