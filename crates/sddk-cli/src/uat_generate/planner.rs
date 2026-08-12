//! E14.5 — Pure planner for the generate pipeline.
//!
//! Consumes: requirements markdown, changelog Added/Changed, last-plan continuity,
//! and (if discovery ran) AamModel scenario_candidates.
//! Produces: UatPlan with features/scenarios, NOT empty.
//!
//! Does NOT write files. Returns plan directly for atomic write by caller.

use sddk_domain::{
    UatFeature, UatPlan, UatPlanApproval, UatPlanRelease, UatPriority, UatScenario,
    UatStep,
};
use std::path::PathBuf;

/// Planning errors.
#[derive(Debug)]
pub enum PlanError {
    /// No features could be extracted from inputs (empty plan would be produced).
    NoFeaturesExtracted,
    /// Last plan parse failed.
    LastPlanParseFailed(String),
    /// Requirements dir parse failed.
    RequirementsParseFailed(String),
}

/// Extract criterion lines from a markdown file (headings + bullets).
fn extract_criteria_from_md(content: &str) -> Vec<String> {
    let mut criteria = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        // Extract ## headings as high-level criteria
        if trimmed.starts_with("## ") || trimmed.starts_with("### ") {
            criteria.push(trimmed.trim_start_matches('#').trim().to_string());
        }
        // Extract bullet points
        if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
            criteria.push(trimmed.trim_start_matches('-').trim_start_matches('*').trim().to_string());
        }
        // Extract numbered list items
        if trimmed.len() > 2
            && trimmed.chars().next().unwrap().is_ascii_digit()
            && trimmed.contains('.')
        {
            criteria
                .push(trimmed.splitn(2, '.').nth(1).unwrap_or(trimmed).trim().to_string());
        }
    }
    criteria
}

/// Parse Added/Changed sections from a changelog.
fn parse_changelog_sections(content: &str) -> (Vec<String>, Vec<String>) {
    let mut added = Vec::new();
    let mut changed = Vec::new();
    let mut current_section = String::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.eq_ignore_ascii_case("## Added") || trimmed.eq_ignore_ascii_case("### Added") {
            current_section = "added".to_string();
        } else if trimmed.eq_ignore_ascii_case("## Changed")
            || trimmed.eq_ignore_ascii_case("### Changed")
        {
            current_section = "changed".to_string();
        } else if trimmed.starts_with("## ") || trimmed.starts_with("# ") {
            current_section = String::new();
        } else if !current_section.is_empty()
            && (trimmed.starts_with("- ") || trimmed.starts_with("* "))
        {
            let text =
                trimmed.trim_start_matches('-').trim_start_matches('*').trim().to_string();
            if current_section == "added" {
                added.push(text);
            } else if current_section == "changed" {
                changed.push(text);
            }
        }
    }
    (added, changed)
}

/// Extract REQ ids from text (e.g., "REQ-001", "RF-016").
fn extract_req_ids(content: &str) -> Vec<String> {
    let mut ids = Vec::new();
    for word in content.split_whitespace() {
        let word = word.trim_end_matches(['.', ',', ':', ')']);
        if (word.starts_with("REQ") || word.starts_with("RF")) && word.len() > 3
            && word.chars().skip(3).all(|c| c.is_ascii_digit() || c == '-')
        {
            ids.push(word.to_string());
        }
    }
    ids
}

/// Build scenario title from criterion text.
fn scenario_title_from_criterion(criterion: &str) -> String {
    let mut title = criterion.to_string();
    if !title.is_empty() {
        let mut chars = title.chars();
        if let Some(first) = chars.next() {
            title = first.to_uppercase().collect::<String>() + chars.as_str();
        }
    }
    if title.len() > 80 {
        title.truncate(77);
        title.push_str("...");
    }
    title
}

/// Build a UatStep from plain text.
fn step_from_text(text: &str) -> UatStep {
    UatStep {
        action: text.to_string(),
        copy_hint: false,
        expected: String::new(),
        step: None,
        kind: None,
        vs_expected_check: None,
    }
}

/// Planner output: built plan + any warnings.
#[derive(Debug)]
pub struct PlanOutput {
    pub plan: UatPlan,
    pub warnings: Vec<String>,
}

impl PlanOutput {
    /// Validate that plan has features and scenarios (non-empty).
    pub fn validate_non_empty(&self) -> Result<(), PlanError> {
        if self.plan.features.is_empty() {
            return Err(PlanError::NoFeaturesExtracted);
        }
        let total_scenarios: usize = self
            .plan
            .features
            .iter()
            .map(|f| f.scenarios.len())
            .sum();
        if total_scenarios == 0 {
            return Err(PlanError::NoFeaturesExtracted);
        }
        Ok(())
    }
}

/// Build a feature from scenarios with given base ID.
fn build_feature(
    scenarios: Vec<UatScenario>,
    feature_id: usize,
    name: String,
    req_ref: Option<String>,
    priority: UatPriority,
) -> UatFeature {
    UatFeature {
        id: format!("F-{:02}", feature_id),
        name,
        requirement_ref: req_ref,
        design_ref: None,
        priority,
        scenarios,
    }
}

/// Build features from criteria list.
/// Returns (features, last_feature_id_used).
fn build_features_from_criteria(
    all_criteria: &[(String, Option<String>)],
) -> (Vec<UatFeature>, usize) {
    let mut features: Vec<UatFeature> = Vec::new();
    let mut feature_scenarios: Vec<UatScenario> = Vec::new();
    let mut scenario_id = 1usize;
    let mut current_req_id: Option<String> = None;
    let mut current_feature_name = "General".to_string();
    let mut feature_scenario_count = 0usize;

    for (criterion, req_id) in all_criteria {
        // If req_id changed, start a new feature group
        if current_req_id.is_none() || current_req_id.as_ref() != req_id.as_ref() {
            if !feature_scenarios.is_empty() {
                features.push(build_feature(
                    std::mem::take(&mut feature_scenarios),
                    features.len() + 1,
                    current_feature_name.clone(),
                    current_req_id.clone(),
                    UatPriority::P1,
                ));
                feature_scenario_count = 0;
            }
            current_req_id = req_id.clone();
            current_feature_name = req_id
                .clone()
                .unwrap_or_else(|| "Feature Group".to_string());
        }

        let plain_steps = vec![step_from_text(&format!("Verify: {}", criterion))];
        let provenance = sddk_domain::UatProvenance {
            author: "uat-planner".to_string(),
            created_at: crate::uat_common::time::now_rfc3339(),
            last_modified_at: crate::uat_common::time::now_rfc3339(),
            origin: sddk_domain::UatOrigin::Spec,
            origin_ref: req_id.clone(),
        };

        let scenario = UatScenario {
            id: format!("S-{:03}", scenario_id),
            title: scenario_title_from_criterion(criterion),
            priority: UatPriority::P1,
            assignee: sddk_domain::UatAssignee::Developer,
            preconditions: Vec::new(),
            plain_steps,
            technical_steps: Vec::new(),
            rationale: None,
            evidence_prompt: None,
            flags: Vec::new(),
            est_minutes: 5,
            context: None,
            evidence: None,
            risk: None,
            automation: None,
            provenance: Some(provenance),
            executor: None,
            evidence_bundle: None,
            oracles: Vec::new(),
            review: None,
            acceptance: None,
            form: None,
            form_checkpoint: None,
            form_completion: None,
            completion: None,
            staleness: None,
        };

        feature_scenarios.push(scenario);
        scenario_id += 1;
        feature_scenario_count += 1;

        // Create a new feature every 5 scenarios
        if feature_scenario_count >= 5 {
            features.push(build_feature(
                std::mem::take(&mut feature_scenarios),
                features.len() + 1,
                current_feature_name.clone(),
                current_req_id.clone(),
                UatPriority::P1,
            ));
            feature_scenario_count = 0;
        }
    }

    // Flush remaining scenarios
    if !feature_scenarios.is_empty() {
        features.push(build_feature(
            feature_scenarios,
            features.len() + 1,
            current_feature_name,
            current_req_id,
            UatPriority::P1,
        ));
    }

    let len = features.len();
    (features, len)
}

/// Pure planner: build UatPlan from requirements, changelog, last_plan, and AAM candidates.
/// Does NOT write files. Returns PlanOutput for atomic write by caller.
pub fn build_plan(
    release: &str,
    requirements: &Option<PathBuf>,
    changelog: &Option<PathBuf>,
    last_plan: &Option<PathBuf>,
    aam_scenario_candidates: &[crate::uat_discover::AamScenarioCandidate],
) -> Result<PlanOutput, PlanError> {
    let mut warnings = Vec::new();
    let mut all_criteria: Vec<(String, Option<String>)> = Vec::new(); // (text, req_id)

    // (B) Consume requirements markdown
    if let Some(req_dir) = requirements {
        if req_dir.is_dir() {
            if let Ok(entries) = std::fs::read_dir(req_dir) {
                let mut files: Vec<_> = entries.flatten().collect();
                files.sort_by_key(|e| e.file_name());
                for entry in files {
                    let path = entry.path();
                    if path.extension().map(|e| e == "md").unwrap_or(false) {
                        if let Ok(content) = std::fs::read_to_string(&path) {
                            let req_ids = extract_req_ids(&content);
                            for criterion in extract_criteria_from_md(&content) {
                                let req_id = req_ids.first().cloned();
                                all_criteria.push((criterion, req_id));
                            }
                        }
                    }
                }
            }
        }
    }

    // (B) Consume changelog Added/Changed sections
    if let Some(cl) = changelog {
        if cl.exists() {
            if let Ok(content) = std::fs::read_to_string(cl) {
                let (added, changed) = parse_changelog_sections(&content);
                warnings.push(format!(
                    "changelog: {} added, {} changed items",
                    added.len(),
                    changed.len()
                ));
                for criterion in added.into_iter().chain(changed) {
                    all_criteria.push((criterion, None));
                }
            }
        }
    }

    // (B) Consume last-plan for continuity
    let last_plan_ref = if let Some(lp) = last_plan {
        if lp.exists() {
            if let Ok(content) = std::fs::read_to_string(lp) {
                if let Ok(prev_plan) = serde_saphyr::from_str::<UatPlan>(&content) {
                    warnings.push(format!(
                        "last_plan: {} features, {} scenarios",
                        prev_plan.features.len(),
                        prev_plan
                            .features
                            .iter()
                            .map(|f| f.scenarios.len())
                            .sum::<usize>()
                    ));
                    Some(prev_plan)
                } else {
                    warnings.push("last_plan: parse failed, starting fresh".to_string());
                    None
                }
            } else {
                warnings.push("last_plan: read failed, starting fresh".to_string());
                None
            }
        } else {
            warnings.push("last_plan: file not found, starting fresh".to_string());
            None
        }
    } else {
        None
    };

    // (B) If discovery ran, consume AamModel scenario_candidates
    let mut discovery_scenarios = Vec::new();
    for candidate in aam_scenario_candidates {
        let plain_steps: Vec<UatStep> = candidate
            .plain_steps
            .iter()
            .map(|s| step_from_text(s))
            .collect();

        let provenance = sddk_domain::UatProvenance {
            author: "uat-discovery".to_string(),
            created_at: candidate
                .provenance
                .created_at
                .clone()
                .unwrap_or_else(|| crate::uat_common::time::now_rfc3339()),
            last_modified_at: crate::uat_common::time::now_rfc3339(),
            origin: sddk_domain::UatOrigin::Regression,
            origin_ref: candidate.flow_ref.clone(),
        };

        let scenario = UatScenario {
            id: format!("S-D{:03}", discovery_scenarios.len() + 1),
            title: candidate.title.clone(),
            priority: UatPriority::P2,
            assignee: sddk_domain::UatAssignee::Developer,
            preconditions: Vec::new(),
            plain_steps,
            technical_steps: Vec::new(),
            rationale: None,
            evidence_prompt: None,
            flags: Vec::new(),
            est_minutes: candidate.estimated_duration_minutes.unwrap_or(5),
            context: None,
            evidence: None,
            risk: None,
            automation: None,
            provenance: Some(provenance),
            executor: None,
            evidence_bundle: None,
            oracles: Vec::new(),
            review: None,
            acceptance: None,
            form: None,
            form_checkpoint: None,
            form_completion: None,
            completion: None,
            staleness: None,
        };
        discovery_scenarios.push(scenario);
    }

    // Build features from criteria
    let (mut features, _) = build_features_from_criteria(&all_criteria);

    // Add discovery scenarios as a dedicated feature
    if !discovery_scenarios.is_empty() {
        features.push(build_feature(
            discovery_scenarios,
            features.len() + 1,
            "Discovered Flows".to_string(),
            None,
            UatPriority::P2,
        ));
    }

    // If no features, return error (atomic: no partial output)
    if features.is_empty() {
        return Err(PlanError::NoFeaturesExtracted);
    }

    // Build last_uat_release from previous plan
    let last_uat_release = last_plan_ref.as_ref().map(|p| p.release.candidate.clone());

    let now = crate::uat_common::time::now_rfc3339();
    let plan = UatPlan {
        schema_version: sddk_domain::LATEST_PLAN_SCHEMA_VERSION,
        release: UatPlanRelease {
            candidate: release.to_string(),
            project: None,
            last_uat_release,
        },
        generated_by: "uat-planner".to_string(),
        generated_at: now,
        features,
        runner_mode: None,
        approval: None,
    };

    let output = PlanOutput { plan, warnings };
    output.validate_non_empty()?;
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::uat_discover::AamScenarioCandidate;
    use tempfile::TempDir;

    #[test]
    fn build_plan_requires_non_empty() {
        let td = TempDir::new().unwrap();
        let result = build_plan("v1.0.0", &None, &None, &None, &[]);
        assert!(matches!(result, Err(PlanError::NoFeaturesExtracted)));
    }

    #[test]
    fn build_plan_from_requirements() {
        let td = TempDir::new().unwrap();
        let req_dir = td.path();
        std::fs::write(
            req_dir.join("req.md"),
            "# Requirements\n\n## Login Feature\n- User can login with email\n- User can reset password\n\n## API Feature\n- API returns JSON\n",
        )
        .unwrap();

        let result = build_plan("v1.0.0", &Some(req_dir.to_path_buf()), &None, &None, &[]);
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(!output.plan.features.is_empty());
    }

    #[test]
    fn build_plan_from_changelog() {
        let td = TempDir::new().unwrap();
        let changelog = td.path().join("CHANGELOG.md");
        std::fs::write(
            &changelog,
            "## Added\n- New login feature\n- API endpoint\n\n## Changed\n- Performance improvements\n",
        )
        .unwrap();

        let result = build_plan("v1.0.0", &None, &Some(changelog), &None, &[]);
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(!output.plan.features.is_empty());
        assert!(output.warnings.iter().any(|w| w.contains("added")));
    }

    #[test]
    fn build_plan_from_aam_candidates() {
        let candidate = AamScenarioCandidate {
            flow_ref: Some("flow-1".to_string()),
            title: "User Login Flow".to_string(),
            priority: Some("P1".to_string()),
            plain_steps: vec![
                "Navigate to /login".to_string(),
                "Enter credentials".to_string(),
            ],
            estimated_duration_minutes: Some(10),
            evidence: crate::uat_discover::AamEvidence {
                kinds: vec!["screenshot".to_string()],
            },
            provenance: crate::uat_discover::AamProvenance {
                generated_by: Some("fara".to_string()),
                author: None,
                created_at: Some("2024-01-01T00:00:00Z".to_string()),
                last_modified_at: None,
                origin: Some("discovered".to_string()),
                origin_ref: None,
                modified_by: None,
                linked_defect: None,
                repro_command: None,
                tags: vec![],
                confidence: Some(0.8),
                human_reviewed: false,
                fallback: None,
            },
        };

        let result = build_plan("v1.0.0", &None, &None, &None, &[candidate]);
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(!output.plan.features.is_empty());
        let scenarios: usize = output.plan.features.iter().map(|f| f.scenarios.len()).sum();
        assert_eq!(scenarios, 1);
    }

    #[test]
    fn build_plan_atomic_no_partial() {
        // If build fails, NO file should be written
        let td = TempDir::new().unwrap();
        let out_path = td.path().join("uat-plan.yaml");

        let result = build_plan("v1.0.0", &None, &None, &None, &[]);
        assert!(result.is_err());

        // File should NOT exist (atomic write rule)
        assert!(!out_path.exists());
    }
}
