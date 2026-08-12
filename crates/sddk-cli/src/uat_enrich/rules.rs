//! Deterministic enrichment rules for building form items.
//!
//! Decision tree per `agents/uat-ux-form.md`:
//!
//! ```text
//! For each scenario without a form:
//! ├── Has API/Http step?
//! │   └── YES → Machine oracle check (Http/Json/Dom)
//! ├── Has subjective UX criterion?
//! │   └── YES → Human rating with scale anchors
//! ├── Has expected textual observable?
//! │   └── YES → Blind observation (visibility=blind)
//! └── Otherwise → Human confirmation (fallback)
//!
//! Every 5 items → insert Checkpoint
//! P0/P1 blocking checks → evidence_requirement: [Screenshot]
//! Provenance → generated_by: "uat-ux-form", model: "heuristic-v1"
//! ```

use sddk_domain::{
    UatFormCheck, UatFormElementKind as FEK, UatFormEvidenceKind as FEVK, UatFormInputKind as FIK,
    UatFormItem, UatFormOracleKind as FOK, UatFormSpec, UatFormVisibility as FVIS, UatPriority,
    UatScenario,
};

/// Build form items for a scenario using deterministic rules.
pub fn build_form_for_scenario(scenario: &UatScenario) -> UatFormSpec {
    let priority = match scenario.priority {
        UatPriority::P0 => "P0",
        UatPriority::P1 => "P1",
        UatPriority::P2 => "P2",
    };
    let p0_p1 = priority == "P0" || priority == "P1";

    let mut items = Vec::new();

    // Decision 1: Check if any step indicates machine-checkable criteria
    let machine_items = detect_machine_check(scenario, p0_p1);
    if !machine_items.is_empty() {
        items.extend(machine_items);
    } else if is_ux_subjective(scenario) {
        // Decision 2: UX subjective → rating
        items.extend(build_rating_items(scenario, p0_p1));
    } else if has_expected_textual(scenario) {
        // Decision 3: Expected textual observable → blind observation
        items.extend(build_blind_observation_items(scenario, p0_p1));
    } else {
        // Decision 4: Fallback → human confirmation
        items.extend(build_confirmation_items(scenario, p0_p1));
    }

    // Insert checkpoints if > 5 items
    items = insert_checkpoints(items);

    UatFormSpec {
        dsl_version: 1,
        items,
        completion: None,
    }
}

/// Detect machine-checkable criteria from steps.
/// Only triggers when criteria/steps EXPLICITLY indicate HTTP/API/DOM/JSON verifiable.
fn detect_machine_check(scenario: &UatScenario, p0_p1: bool) -> Vec<UatFormItem> {
    // Machine check is ONLY for explicit machine-verifiable indicators
    let has_http_indicator = scenario.plain_steps.iter().any(|s| {
        // More precise: must contain actual HTTP indicators
        let action_lower = s.action.to_lowercase();
        // HTTP endpoint or response status indicators
        (action_lower.contains("http://")
            || action_lower.contains("https://")
            || action_lower.contains("/health")
            || action_lower.contains("/api/")
            || action_lower.contains("status code")
            || action_lower.contains("status_code")
            || action_lower.contains("response status")
            || action_lower.contains("status 200")
            || action_lower.contains("status 404")
            || action_lower.contains("response.status"))
            && !action_lower.contains("json") // exclude JSON which is separate
    });

    let has_json_indicator = scenario.plain_steps.iter().any(|s| {
        let action_lower = s.action.to_lowercase();
        let expected_lower = s.expected.to_lowercase();
        // JSON indicators: explicit json mentions or body structure checks
        (action_lower.contains("json") || action_lower.contains("/api/"))
            && (expected_lower.contains("json")
                || expected_lower.contains("body")
                || expected_lower.contains("field")
                || expected_lower.contains("property"))
    });

    let has_dom_indicator = scenario.plain_steps.iter().any(|s| {
        let action_lower = s.action.to_lowercase();
        // DOM: more specific - must have selector or explicit DOM check
        action_lower.contains("selector")
            || action_lower.contains("css selector")
            || action_lower.contains("xpath")
            || action_lower.contains("dom element")
            || (action_lower.contains("check") && action_lower.contains("element"))
    });

    let oracle = if has_http_indicator {
        Some(FOK::Http)
    } else if has_json_indicator {
        Some(FOK::Json)
    } else if has_dom_indicator {
        Some(FOK::Dom)
    } else {
        None
    };

    if oracle.is_some() {
        let ev_required = if p0_p1 {
            vec![FEVK::Screenshot]
        } else {
            vec![]
        };

        vec![
            // Info item: scenario title
            mk_info_item(&format!("{}-title", scenario.id), &scenario.title),
            // Machine oracle check
            mk_check_item(CheckConfig {
                id: format!("{}-machine-check", scenario.id),
                prompt: "Verify the expected result automatically".into(),
                oracle,
                visibility: FVIS::Visible,
                blocking: true,
                evidence_requirement: ev_required.clone(),
                expected: None,
            }),
        ]
    } else {
        vec![]
    }
}

/// Check if scenario has subjective UX criterion.
fn is_ux_subjective(scenario: &UatScenario) -> bool {
    // UX subjective indicators:
    // - context.user_story contains UX-related keywords
    // - title contains UX-related terms
    let ux_keywords = [
        "helpful",
        "usability",
        "UX",
        "user experience",
        "intuitive",
        "easy to use",
        "design",
        "appearance",
        "look and feel",
        "color",
        "font",
        "layout",
        "navigate",
    ];

    let text_to_check = scenario
        .title
        .to_lowercase()
        .chars()
        .chain(
            scenario
                .context
                .as_ref()
                .and_then(|c| c.user_story.as_ref())
                .map(|s| s.to_lowercase())
                .unwrap_or_default()
                .chars(),
        )
        .collect::<String>();

    ux_keywords.iter().any(|kw| text_to_check.contains(kw))
}

/// Check if scenario has expected textual observable (for blind observation).
/// Only triggers when steps have explicit expected text but NOT machine-checkable indicators.
fn has_expected_textual(scenario: &UatScenario) -> bool {
    // Blind observation: steps with expected text where user observes result
    // but shouldn't know the expected beforehand (for surprise/shock detection)
    // Only applies when NOT machine-checkable
    !scenario.plain_steps.is_empty()
        && scenario.plain_steps.iter().any(|s| {
            !s.expected.is_empty()
                && s.expected.len() > 3
                && !s.action.to_lowercase().contains("http://")
                && !s.action.to_lowercase().contains("https://")
                && !s.action.to_lowercase().contains("/api/")
                && !s.action.to_lowercase().contains("json")
                && !s.action.to_lowercase().contains("selector")
                && !s.action.to_lowercase().contains("dom")
        })
        && !is_ux_subjective(scenario) // Not UX subjective (that's rating)
}

/// Build rating items for UX subjective scenarios.
fn build_rating_items(scenario: &UatScenario, p0_p1: bool) -> Vec<UatFormItem> {
    let ev_required: Vec<FEVK> = if p0_p1 {
        vec![FEVK::Screenshot]
    } else {
        vec![]
    };

    vec![
        mk_info_item(&format!("{}-title", scenario.id), &scenario.title),
        mk_rating_item(
            &format!("{}-rating", scenario.id),
            "Rate the UX quality (1=Poor, 5=Excellent)",
            ev_required,
        ),
        mk_confirm_item(
            &format!("{}-confirm", scenario.id),
            "Confirm the UX meets expectations",
            FVIS::Visible,
            p0_p1,
            vec![],
        ),
    ]
}

/// Build blind observation items.
fn build_blind_observation_items(scenario: &UatScenario, p0_p1: bool) -> Vec<UatFormItem> {
    let ev_required: Vec<FEVK> = if p0_p1 {
        vec![FEVK::Screenshot]
    } else {
        vec![]
    };

    // For blind observation: the expected is hidden from the user
    // We show a prompt without revealing what the expected result is
    vec![
        mk_info_item(&format!("{}-title", scenario.id), &scenario.title),
        mk_check_item(CheckConfig {
            id: format!("{}-blind-check", scenario.id),
            prompt: "Observe and confirm the result".into(),
            oracle: None, // no oracle - human blind observation
            visibility: FVIS::Blind,
            blocking: true,
            evidence_requirement: ev_required,
            expected: Some("Expected result hidden for blind observation".into()),
        }),
    ]
}

/// Build human confirmation items (fallback).
fn build_confirmation_items(scenario: &UatScenario, p0_p1: bool) -> Vec<UatFormItem> {
    let ev_required: Vec<FEVK> = if p0_p1 {
        vec![FEVK::Screenshot]
    } else {
        vec![]
    };

    vec![
        mk_info_item(&format!("{}-title", scenario.id), &scenario.title),
        mk_confirm_item(
            &format!("{}-confirm", scenario.id),
            "Verify this scenario passes",
            FVIS::Visible,
            p0_p1,
            ev_required,
        ),
    ]
}

/// Insert checkpoints every 5 items.
fn insert_checkpoints(items: Vec<UatFormItem>) -> Vec<UatFormItem> {
    if items.len() <= 5 {
        return items;
    }

    let mut result = Vec::with_capacity(items.len() * 2);
    let mut count = 0;

    for item in &items {
        result.push(item.clone());
        count += 1;

        // Insert checkpoint after every 5 items
        if count % 5 == 0 && count < items.len() {
            result.push(UatFormItem {
                kind: FEK::Checkpoint,
                id: Some(format!("cp-{}", count)),
                check: None,
                text: Some(format!("Checkpoint after {} items", count)),
                flow: None,
                target: None,
                checkpoint: Some(sddk_domain::UatCheckpoint {
                    id: format!("cp-{}", count),
                    label: Some(format!("Checkpoint after {} items", count)),
                    evidence_summary: sddk_domain::UatEvidenceSummary::default(),
                    items: vec![],
                }),
            });
        }
    }

    result
}

// ─── Helper constructors ────────────────────────────────────────────────────────

fn mk_info_item(id: &str, text: &str) -> UatFormItem {
    UatFormItem {
        kind: FEK::Info,
        id: Some(id.to_string()),
        check: None,
        text: Some(text.to_string()),
        flow: None,
        target: None,
        checkpoint: None,
    }
}

/// Config for creating a check item.
struct CheckConfig {
    id: String,
    prompt: String,
    oracle: Option<FOK>,
    visibility: FVIS,
    blocking: bool,
    evidence_requirement: Vec<FEVK>,
    expected: Option<String>,
}

fn mk_check_item(cfg: CheckConfig) -> UatFormItem {
    UatFormItem {
        kind: FEK::Check,
        id: Some(cfg.id),
        check: Some(UatFormCheck {
            kind: FIK::Confirm,
            prompt: cfg.prompt,
            oracle: cfg.oracle,
            visibility: cfg.visibility,
            required: cfg.blocking,
            blocking: cfg.blocking,
            confidence_requirement: None,
            evidence_requirement: cfg.evidence_requirement,
            comment_required_when: None,
            options: vec![],
            expected: cfg.expected,
        }),
        text: None,
        flow: None,
        target: None,
        checkpoint: None,
    }
}

fn mk_rating_item(id: &str, prompt: &str, evidence_requirement: Vec<FEVK>) -> UatFormItem {
    UatFormItem {
        kind: FEK::Check,
        id: Some(id.to_string()),
        check: Some(UatFormCheck {
            kind: FIK::Rating,
            prompt: prompt.to_string(),
            oracle: None,
            visibility: FVIS::Visible,
            required: true,
            blocking: true,
            confidence_requirement: None,
            evidence_requirement,
            // Rating has anchors represented via options or expected
            comment_required_when: Some("below_3".to_string()),
            options: vec![
                "1 - Poor".to_string(),
                "2 - Below Average".to_string(),
                "3 - Average".to_string(),
                "4 - Good".to_string(),
                "5 - Excellent".to_string(),
            ],
            expected: None,
        }),
        text: None,
        flow: None,
        target: None,
        checkpoint: None,
    }
}

fn mk_confirm_item(
    id: &str,
    prompt: &str,
    visibility: FVIS,
    p0_p1: bool,
    evidence_requirement: Vec<FEVK>,
) -> UatFormItem {
    let ev = if p0_p1 && evidence_requirement.is_empty() {
        vec![FEVK::Screenshot]
    } else {
        evidence_requirement
    };

    UatFormItem {
        kind: FEK::Check,
        id: Some(id.to_string()),
        check: Some(UatFormCheck {
            kind: FIK::Confirm,
            prompt: prompt.to_string(),
            oracle: None,
            visibility,
            required: true,
            blocking: true,
            confidence_requirement: None,
            evidence_requirement: ev,
            comment_required_when: None,
            options: vec![],
            expected: None,
        }),
        text: None,
        flow: None,
        target: None,
        checkpoint: None,
    }
}
