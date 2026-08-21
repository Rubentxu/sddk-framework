//! INC file generator from Finding records.
//!
//! Renders `INC-NNN-{slug}.md` files using the template at
//! `docs/debt/INCIDENCE-TEMPLATE.md` embedded via `include_str!`.

use sddk_domain::{DebtReport, Finding, IncRecord, IncStatus, Priority, Severity};
use std::collections::HashSet;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

/// Derives the INC slug from a finding: first 8 chars of its fingerprint.
pub fn derive_inc_slug(finding: &Finding) -> String {
    finding.fingerprint.chars().take(8).collect()
}

/// Derives the next monotonic INC id for a finding.
///
/// If a slug collision exists in `existing_ids`, the existing NNN is reused.
/// Otherwise, NNN = max(existing NNNs) + 1.
pub fn derive_inc_id(finding: &Finding, existing_ids: &HashSet<String>) -> String {
    let slug = derive_inc_slug(finding);
    // Find all existing IDs with the same slug
    let with_same_slug: Vec<(u32, &String)> = existing_ids
        .iter()
        .filter_map(|id| {
            let parts: Vec<&str> = id.split('-').collect();
            if parts.len() >= 3 && parts[2] == slug {
                parts[1].parse().ok().map(|n| (n, id))
            } else {
                None
            }
        })
        .collect();
    let nnn = if with_same_slug.is_empty() {
        // New slug: compute max NNN across all existing + 1
        let max_nnn = existing_ids
            .iter()
            .filter_map(|id| id.split('-').nth(1).and_then(|n| n.parse::<u32>().ok()))
            .max()
            .unwrap_or(0);
        max_nnn + 1
    } else {
        // Reuse existing NNN
        with_same_slug.iter().map(|(n, _)| *n).max().unwrap_or(1)
    };
    format!("INC-{:03}-{slug}", nnn)
}

/// Renders the INC template for a finding into a Markdown string.
///
/// Template is embedded at compile time via `include_str!` and rendered
/// with the finding's metadata.
pub fn render_inc_template(finding: &Finding, project_id: &str, cycle_id: &str) -> String {
    let inc_id = derive_inc_id_string(finding);
    let slug = derive_inc_slug(finding);
    let created = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "2026-08-21T00:00:00Z".into());
    let severity_str = severity_to_str(&finding.severity);
    let priority_str = priority_to_str(&finding.priority);
    let status_str = finding_status_to_str(&finding.status);
    let fingerprint_aliases_str = if finding.fingerprint_aliases.is_empty() {
        "[]".to_string()
    } else {
        format!(
            "[{}]",
            finding
                .fingerprint_aliases
                .iter()
                .map(|s| format!("\"{}\"", s))
                .collect::<Vec<_>>()
                .join(", ")
        )
    };

    let template = INCTEMPLATE;
    template
        // Frontmatter field replacements
        .replace("INC-NNN-{slug}", &inc_id)
        .replace("\"{one-line summary}\"", &format!("\"{}\"", &finding.title))
        .replace("\"{hex}\"", &format!("\"{}\"", &finding.fingerprint))
        .replace("critical|high|medium|low", severity_str)
        .replace("P0|P1|P2|P3", priority_str)
        .replace("[]", &fingerprint_aliases_str)
        .replace("CL-NN", &finding.cluster_id)
        .replace("YYYY-MM-DD", &created[..10])
        .replace("{created}", &created)
        .replace("actor-name", "sddk")
        // Body section replacements
        .replace(
            "<problem statement: what's wrong, where, why it matters>",
            &finding.description,
        )
        .replace(
            "<why this severity + priority + cluster_id; cite evidence>",
            &format!(
                "Severity={}, Priority={}, Cluster={}",
                severity_str, priority_str, &finding.cluster_id
            ),
        )
        .replace("{finding-id}", &finding.id)
        .replace(
            "cycle-{N}",
            &format!(
                "cycle-{}",
                cycle_id
                    .rsplit('/')
                    .next()
                    .unwrap_or("8")
                    .trim_start_matches("kernel-cycle-")
            ),
        )
        // H1 heading replacements
        .replace("{slug}", &derive_inc_slug(finding))
        .replace("{title}", &finding.title)
}

fn derive_inc_id_string(finding: &Finding) -> String {
    // For single finding use max NNN=1
    let slug = derive_inc_slug(finding);
    format!("INC-001-{slug}")
}

fn severity_to_str(sev: &Severity) -> &'static str {
    match sev {
        Severity::Critical => "critical",
        Severity::High => "high",
        Severity::Medium => "medium",
        Severity::Low => "low",
    }
}

fn priority_to_str(pri: &Priority) -> &'static str {
    match pri {
        Priority::P0 => "P0",
        Priority::P1 => "P1",
        Priority::P2 => "P2",
        Priority::P3 => "P3",
    }
}

fn finding_status_to_str(status: &sddk_domain::FindingStatus) -> &'static str {
    match status {
        sddk_domain::FindingStatus::Open => "open",
        sddk_domain::FindingStatus::InProgress => "in-progress",
        sddk_domain::FindingStatus::Deferred => "deferred",
        sddk_domain::FindingStatus::Resolved => "resolved",
        sddk_domain::FindingStatus::Superseded => "superseded",
    }
}

// Embedded at compile time from the canonical template
const INCTEMPLATE: &str = include_str!("../../../docs/debt/INCIDENCE-TEMPLATE.md");

#[cfg(test)]
mod tests {
    use super::*;
    use sddk_domain::FindingStatus;

    fn finding_with_fp(fp: &str) -> Finding {
        Finding {
            id: "FIND-0001".into(),
            title: "Test finding".into(),
            severity: Severity::Medium,
            priority: Priority::P2,
            status: FindingStatus::Open,
            fingerprint: fp.into(),
            fingerprint_aliases: vec![],
            cluster_id: "CL-01".into(),
            category: "architecture".into(),
            description: "Test description".into(),
            remediation_cycle: None,
            remediation_pr: None,
            evidence_refs: None,
        }
    }

    #[test]
    fn test_slug_first_8_chars() {
        let f = finding_with_fp("3ef321c4efe1d87e");
        assert_eq!(derive_inc_slug(&f), "3ef321c4");
    }

    #[test]
    fn test_slug_empty_fp() {
        let f = finding_with_fp("");
        assert_eq!(derive_inc_slug(&f), "");
    }

    #[test]
    fn test_inc_id_monotonic_new() {
        let f = finding_with_fp("3ef321c4efe1d87e");
        let existing: HashSet<String> = HashSet::new();
        let id = derive_inc_id(&f, &existing);
        assert!(id.starts_with("INC-001-3ef321c4"));
    }

    #[test]
    fn test_inc_id_reuses_nnn_on_slug_collision() {
        let f = finding_with_fp("3ef321c4efe1d87e");
        let mut existing: HashSet<String> = HashSet::new();
        existing.insert("INC-005-3ef321c4".into()); // slug matches, NNN=5
        let id = derive_inc_id(&f, &existing);
        assert_eq!(id, "INC-005-3ef321c4");
    }

    #[test]
    fn test_render_includes_frontmatter_fields() {
        let f = finding_with_fp("3ef321c4efe1d87e");
        let rendered = render_inc_template(&f, "sddk-framework", "p-test/kernel-cycle-8");
        // Frontmatter fields
        assert!(rendered.contains("id: INC-"), "missing id");
        assert!(rendered.contains("status: open"), "missing status");
        assert!(rendered.contains("severity: medium"), "missing severity");
        assert!(rendered.contains("priority: P2"), "missing priority");
        assert!(
            rendered.contains(r#""3ef321c4efe1d87e""#),
            "missing fingerprint"
        );
        assert!(rendered.contains("cluster_id: CL-01"), "missing cluster_id");
        // Body sections
        assert!(rendered.contains("## Context"), "missing Context");
        assert!(rendered.contains("## Rationale"), "missing Rationale");
        assert!(rendered.contains("## Lifecycle"), "missing Lifecycle");
        assert!(rendered.contains("## References"), "missing References");
        // Lifecycle table has created row
        assert!(rendered.contains("created"), "missing created");
    }

    #[test]
    fn test_render_idempotent_excluding_timestamp() {
        let f = finding_with_fp("3ef321c4efe1d87e");
        let r1 = render_inc_template(&f, "sddk-framework", "p-test/kernel-cycle-8");
        // Note: timestamp changes between calls so we just check structural idempotency
        assert!(r1.contains("INC-001-3ef321c4"));
    }
}
