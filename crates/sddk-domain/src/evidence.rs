//! Universal Evidence Model for SDDK governed capabilities.
//!
//! This module defines the canonical evidence types used across ALL governed
//! capabilities in SDDK — not just UAT. The `EvidenceBundle` structure with
//! content-addressable artifacts, environment context, and execution metadata is
//! the universal substrate for assurance and auditability.
//!
//! ## Design principles (ADR-0016)
//!
//! - **Content-addressable**: every artifact is identified by `sha256:<hex>` of its bytes.
//!   The bundle is verifiable independently of where it is stored.
//! - **Extensible kinds**: `EvidenceKind` is a closed enum; adding a new variant
//!   is a schema extension (backward-compatible), not a modification.
//! - **Separation**: environment (`where`) vs execution (`who/what`) vs artifacts
//!   (`what was captured`) — three orthogonal concerns in one bundle.
//!
//! ## UAT specialization
//!
//! The UAT-specific aliases (`UatEvidenceBundle`, `UatEvidenceArtifact`, etc.)
//! in [`crate::uat`] are re-exports of these types for backward compatibility.
//! New code should use the names in this module directly.

#![allow(missing_docs)]

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Shared enums (used by evidence types AND by UAT-specific types in uat.rs).
// Kept here so evidence.rs is self-contained.
// ---------------------------------------------------------------------------

/// How strict the comparison between `expected` and `observed` should be.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceExpectedCheck {
    #[default]
    ExactMatch,
    Contains,
    Regex,
    JsonPath,
    ExitCode,
}

/// Closed vocabulary for risk classification.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceRiskClassification {
    Critical,
    High,
    #[default]
    Medium,
    Low,
}

/// How much a single scenario failure can impact the release.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceBlastRadius {
    #[default]
    FeatureBlocker,
    ReleaseBlocker,
    Advisory,
}

/// Status of automation for a scenario.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceAutomationStatus {
    #[default]
    Manual,
    Scripted,
    Automated,
}

/// Origin of a scenario: why this test exists.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceOrigin {
    Spec,
    Bug,
    Incident,
    #[default]
    Regression,
}

// ---------------------------------------------------------------------------
// Evidence types
// ---------------------------------------------------------------------------

/// Closed vocabulary for evidence capture kinds.
///
/// These are the capture taxonomy used by ALL governed capabilities,
/// not just UAT. Adding a new kind is an extension (backward-compatible).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceKind {
    File,
    Screenshot,
    CommandOutput,
    Assertion,
    Metric,
    /// Playwright trace archive.
    Trace,
    /// Captured console messages (JSON).
    Console,
    /// Captured network failures (JSON array).
    Network,
    /// HTTP response snapshot of the main navigation (status/url/headers).
    Http,
    /// DOM snapshot (HTML).
    Dom,
    /// ARIA accessibility snapshot (JSON).
    Aria,
    /// Bounding-box geometry of selectors (JSON).
    Geometry,
    /// Video recording (webm).
    Video,
    /// Computer-use trajectory (JSON).
    Trajectory,
    #[default]
    Note,
}

/// One evidence kind descriptor: what to capture and how to evaluate it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct EvidenceKindItem {
    pub kind: EvidenceKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r#ref: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub match_mode: Option<EvidenceExpectedCheck>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_value: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_bytes: Option<u64>,
}

/// A captured evidence artifact. Content-addressable:
/// `sha256:<hex>` of the payload bytes (ADR-014 §2.3).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct EvidenceArtifact {
    pub kind: EvidenceKind,
    /// `sha256:<hex>` of the payload — verifiable against the referenced file.
    pub r#ref: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mime: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// Environment snapshot for an evidence bundle: what environment the execution ran in.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct EvidenceEnvironment {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub git_sha: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub app_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub browser: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub viewport: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub os: Option<String>,
}

/// Execution metadata for an evidence bundle: who executed and with what model/prompt.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct EvidenceExecution {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub executor: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt_hash: Option<String>,
}

/// Universal evidence bundle. All artifacts are content-addressable;
/// `environment` + `execution` make the execution reproducible and auditable
/// (ADR-014 §2.3).
///
/// This is the canonical evidence type for ANY governed capability in SDDK,
/// not just UAT. `UatEvidenceBundle` in [`crate::uat`] is a type alias
/// pointing here for backward compatibility.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct EvidenceBundle {
    #[serde(default)]
    pub artifacts: Vec<EvidenceArtifact>,
    #[serde(default)]
    pub environment: EvidenceEnvironment,
    #[serde(default)]
    pub execution: EvidenceExecution,
}
