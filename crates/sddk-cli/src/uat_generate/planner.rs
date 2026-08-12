//! E14.5 — Pure planner for the generate pipeline.
//!
//! Consumes: requirements markdown, changelog Added/Changed, last-plan continuity,
//! and (if discovery ran) AamModel scenario_candidates.
//! Produces: UatPlan with features/scenarios, NOT empty.
//!
//! Does NOT write files. Returns plan directly for atomic write by caller.

use sddk_domain::{UatFeature, UatPlan, UatPlanRelease, UatPriority, UatScenario};

use super::parsing::{
    extract_criteria_from_md, extract_req_ids, parse_changelog_sections,
    scenario_title_from_criterion, step_from_text,
};

/// Planning errors.
#[derive(Debug)]
pub enum PlanError {
    /// No features could be extracted from inputs (empty plan would be produced).
    NoFeaturesExtracted,
    /// Last plan parse/read failed.
    LastPlanParseFailed(String),
}

impl std::fmt::Display for PlanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlanError::NoFeaturesExtracted => {
                write!(f, "no features could be extracted from inputs")
            }
            PlanError::LastPlanParseFailed(msg) => write!(f, "last plan parse failed: {}", msg),
        }
    }
}

/// Planner output: built plan plus warnings collected during planning.
#[derive(Debug)]
pub struct PlanOutput {
    /// The constructed UatPlan.
    pub plan: UatPlan,
    /// Warnings collected during planning (e.g., changelog stats, last_plan stats).
    /// Intentionally part of public API for external callers to inspect.
    /// Internal code populates but does not consume this field.
    #[allow(dead_code)]
    pub warnings: Vec<String>,
}

impl PlanOutput {
    /// Validate that plan has features and scenarios (non-empty).
    pub fn validate_non_empty(&self) -> Result<(), PlanError> {
        if self.plan.features.is_empty() {
            return Err(PlanError::NoFeaturesExtracted);
        }
        let total_scenarios: usize = self.plan.features.iter().map(|f| f.scenarios.len()).sum();
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
    requirements: &Option<std::path::PathBuf>,
    changelog: &Option<std::path::PathBuf>,
    last_plan: &Option<std::path::PathBuf>,
    aam_scenario_candidates: &[crate::uat_discover::AamScenarioCandidate],
) -> Result<PlanOutput, PlanError> {
    let mut warnings = Vec::new();
    let mut all_criteria: Vec<(String, Option<String>)> = Vec::new(); // (text, req_id)

    // (B) Consume requirements markdown
    if let Some(req_dir) = requirements
        && req_dir.is_dir()
        && let Ok(entries) = std::fs::read_dir(req_dir)
    {
        let mut files: Vec<_> = entries.flatten().collect();
        files.sort_by_key(|e| e.file_name());
        for entry in files {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "md")
                && let Ok(content) = std::fs::read_to_string(&path)
            {
                let req_ids = extract_req_ids(&content);
                for criterion in extract_criteria_from_md(&content) {
                    let req_id = req_ids.first().cloned();
                    all_criteria.push((criterion, req_id));
                }
            }
        }
    }

    // (B) Consume changelog Added/Changed sections
    if let Some(cl) = changelog
        && cl.exists()
        && let Ok(content) = std::fs::read_to_string(cl)
    {
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

    // (B) Consume last-plan for continuity
    // Spec: merge real last_plan - if parsed successfully, clone features/scenarios
    // from previous plan; approval=None; release last_uat_release; combine with new
    // criteria/AAM; dedupe by scenario ID + title; renumber only new collisions.
    let last_plan_ref: Option<UatPlan> = if let Some(lp) = last_plan {
        if lp.exists() {
            let content = std::fs::read_to_string(lp)
                .map_err(|e| PlanError::LastPlanParseFailed(format!("read failed: {}", e)))?;
            let prev_plan: UatPlan = serde_saphyr::from_str(&content)
                .map_err(|e| PlanError::LastPlanParseFailed(format!("parse failed: {}", e)))?;
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
            return Err(PlanError::LastPlanParseFailed(
                "last_plan file not found".to_string(),
            ));
        }
    } else {
        None
    };

    // (B) If discovery ran, consume AamModel scenario_candidates
    let mut discovery_scenarios = Vec::new();
    for candidate in aam_scenario_candidates {
        let plain_steps: Vec<sddk_domain::UatStep> = candidate
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
                .unwrap_or_else(crate::uat_common::time::now_rfc3339),
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
    let (mut new_features, _) = build_features_from_criteria(&all_criteria);

    // Add discovery scenarios as a dedicated feature
    if !discovery_scenarios.is_empty() {
        new_features.push(build_feature(
            discovery_scenarios,
            new_features.len() + 1,
            "Discovered Flows".to_string(),
            None,
            UatPriority::P2,
        ));
    }

    // Merge: clone last_plan scenarios if no new criteria provided.
    // Spec: dedupe by scenario ID + title; renumber only new collisions;
    // keep prev IDs for preserved scenarios.
    let features: Vec<UatFeature> = if new_features.is_empty() {
        // No new criteria - preserve all scenarios from last_plan
        if let Some(ref prev_plan) = last_plan_ref {
            warnings.push("plan: cloned from last_plan (no new criteria)".to_string());
            prev_plan.features.clone()
        } else {
            Vec::new()
        }
    } else if let Some(ref prev_plan) = last_plan_ref {
        // New criteria provided - merge with last_plan scenarios.
        // Build set of (id, title) from new scenarios for dedup
        let new_scenario_keys: std::collections::HashSet<(String, String)> = new_features
            .iter()
            .flat_map(|f| f.scenarios.iter().map(|s| (s.id.clone(), s.title.clone())))
            .collect();

        // Clone prev_plan scenarios that don't collide with new ones
        let mut preserved_scenarios: Vec<UatScenario> = prev_plan
            .features
            .iter()
            .flat_map(|f| f.scenarios.iter())
            .filter(|s| !new_scenario_keys.contains(&(s.id.clone(), s.title.clone())))
            .cloned()
            .collect();

        // Assign IDs to preserved scenarios if they collide with new ones
        let max_new_id = new_features
            .iter()
            .flat_map(|f| f.scenarios.iter())
            .filter_map(|s| {
                s.id.strip_prefix("S-")
                    .and_then(|n| n.parse::<usize>().ok())
            })
            .max()
            .unwrap_or(0);

        let mut next_id = max_new_id + 1;
        for scenario in &mut preserved_scenarios {
            // Renumber only if collision detected
            if new_scenario_keys.contains(&(scenario.id.clone(), scenario.title.clone())) {
                scenario.id = format!("S-{:03}", next_id);
                next_id += 1;
            }
        }

        // Combine: new features first, then preserved scenarios grouped by original feature
        let mut combined = new_features;
        if !preserved_scenarios.is_empty() {
            combined.push(UatFeature {
                id: format!("F-{:02}", combined.len() + 1),
                name: "Preserved from Previous Plan".to_string(),
                requirement_ref: None,
                design_ref: None,
                priority: UatPriority::P2,
                scenarios: preserved_scenarios,
            });
        }
        combined
    } else {
        new_features
    };

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
