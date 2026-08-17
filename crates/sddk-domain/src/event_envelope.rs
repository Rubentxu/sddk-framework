//! EventEnvelope v1 wire-format types.
//!
//! Canonical JSON determinism invariant: this module relies on `serde_json`'s
//! default `Map<String, Value>` = `BTreeMap` ordering. DO NOT enable the
//! `serde_json/preserve_order` feature — it breaks canonicalization.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::sync::OnceLock;
use regex::Regex;

/// Entity reference within an event envelope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EntityRef {
    #[serde(rename = "type")]
    pub kind: String,
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<EntityRefVersion>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_hash: Option<String>,
}

/// Version variant for an entity reference — can be a string or integer tag.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum EntityRefVersion {
    String(String),
    Integer(i64),
}

/// Actor (principal) who authored or initiated the event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActorRef {
    pub kind: ActorKind,
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub definition_hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub policy_hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

/// Kind of actor that initiated an event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ActorKind {
    Human,
    Agent,
    System,
}

/// Error arising from invalid event type formatting.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum EventTypeError {
    #[error("event_type must match `[a-z][a-z0-9_]*(\\.[a-z][a-z0-9_]*){{2,}}` (got: {0:?})")]
    InvalidFormat(String),
}

/// Wire-format envelope for SDDK domain events (CEP-1 compatible).
///
/// The `content_hash` field is required and carries a SHA-256 digest of the
/// canonical JSON representation (excluding the `content_hash` field itself).
/// Canonicalization relies on `serde_json`'s default `Map<String, Value>` =
/// `BTreeMap` ordering; struct fields serialize in declaration order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventEnvelopeV1 {
    /// Globally unique event identifier.
    pub event_id: String,
    /// Namespaced event type in `realm.object.verb` form, e.g. `uat.acceptance.granted`.
    pub event_type: String,
    /// Schema version; always `1` for this type.
    pub schema_version: u32,
    /// Stream this event belongs to.
    pub stream_id: String,
    /// Monotonic sequence number within the stream.
    pub sequence: u64,
    /// Project that produced or owns this event.
    pub project_id: String,
    /// Wall-clock time when the event occurred (RFC 3339).
    pub occurred_at: String,
    /// Wall-clock time when the event was recorded (RFC 3339).
    pub recorded_at: String,
    /// Actor who authored or initiated the event.
    pub actor: ActorRef,
    /// Zero or more entities affected by or related to this event.
    pub subjects: Vec<EntityRef>,
    /// Arbitrary JSON payload specific to the event type.
    pub payload: Value,
    /// References to external evidence (e.g. UAT check receipts).
    pub evidence_refs: Vec<String>,
    /// SHA-256 content hash in `sha256:<64-hex>` format.
    pub content_hash: String,
    /// Optional metadata bag.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
    /// ID of the event that directly caused this one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub causation_id: Option<String>,
    /// ID used to correlate related events across a session or operation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<String>,
    /// Cycle this event is part of.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cycle_id: Option<String>,
    /// Frame within the cycle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame_id: Option<String>,
    /// Fork this event originated from, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fork_id: Option<String>,
}

impl EventEnvelopeV1 {
    /// Schema version constant for V1 envelopes.
    pub const SCHEMA_VERSION: u32 = 1;
    /// Prefix for content_hash values per JSON schema regex.
    pub const CONTENT_HASH_PREFIX: &'static str = "sha256:";

    /// Canonical JSON serialization.
    ///
    /// Determinism invariant: this relies on `serde_json::Map<String, Value>`
    /// using `BTreeMap` (the workspace default). DO NOT enable the
    /// `serde_json` `preserve_order` feature — that breaks canonicalization.
    pub fn to_canonical_json(&self) -> String {
        serde_json::to_string(self)
            .expect("EventEnvelopeV1 is always serializable; this is a bug")
    }

    /// Computes `sha256:<64-hex-lowercase>` over the canonical JSON
    /// representation. The `content_hash` field itself is part of the
    /// serialized form; to produce a self-consistent hash, callers must
    /// pre-fill the `content_hash` field with a stable placeholder before
    /// calling this method.
    pub fn compute_content_hash(&self) -> String {
        let canonical = self.to_canonical_json();
        let digest = Sha256::digest(canonical.as_bytes());
        format!("{}{:x}", Self::CONTENT_HASH_PREFIX, digest)
    }

    /// Validates `event_type` against the namespacing regex.
    ///
    /// The regex requires `realm.object.verb` form: at least three segments
    /// separated by dots, each segment starting with a lowercase letter and
    /// containing only lowercase letters, digits, or underscores.
    pub fn validate_event_type(s: &str) -> Result<(), EventTypeError> {
        static RE: OnceLock<Regex> = OnceLock::new();
        let re = RE.get_or_init(|| {
            Regex::new(r"^[a-z][a-z0-9_]*(\.[a-z][a-z0-9_]*){2,}$")
                .expect("static regex compilation")
        });
        if re.is_match(s) {
            Ok(())
        } else {
            Err(EventTypeError::InvalidFormat(s.to_owned()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn minimal_envelope() -> EventEnvelopeV1 {
        EventEnvelopeV1 {
            event_id: "e-1".into(),
            event_type: "workflow.phase.entered".into(),
            schema_version: EventEnvelopeV1::SCHEMA_VERSION,
            stream_id: "s-1".into(),
            sequence: 1,
            project_id: "p-1".into(),
            occurred_at: "2026-08-17T10:00:00Z".into(),
            recorded_at: "2026-08-17T10:00:01Z".into(),
            actor: ActorRef {
                kind: ActorKind::System,
                id: "sddk-cli".into(),
                definition_hash: None,
                policy_hash: None,
                model: None,
            },
            subjects: vec![],
            payload: json!({}),
            evidence_refs: vec![],
            content_hash: "sha256:placeholder".into(),
            metadata: None,
            causation_id: None,
            correlation_id: None,
            cycle_id: None,
            frame_id: None,
            fork_id: None,
        }
    }

    #[test]
    fn compute_content_hash_format_matches_regex() {
        let h = minimal_envelope().compute_content_hash();
        assert!(h.starts_with("sha256:"));
        assert_eq!(h.len(), "sha256:".len() + 64);
        assert!(h[7..].chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
    }

    #[test]
    fn compute_content_hash_is_stable() {
        let env1 = minimal_envelope();
        let env2 = minimal_envelope();
        assert_eq!(env1.compute_content_hash(), env2.compute_content_hash());
    }

    #[test]
    fn to_canonical_json_is_brace_terminated() {
        let j = minimal_envelope().to_canonical_json();
        assert!(j.starts_with('{'));
        assert!(j.ends_with('}'));
    }

    #[test]
    fn validate_event_type_accepts_valid() {
        let valid = [
            "workflow.phase.entered",
            "uat.acceptance.granted",
            "capability.execution.completed",
            "graph.staleness.detected",
        ];
        for s in valid {
            assert_eq!(
                EventEnvelopeV1::validate_event_type(s),
                Ok(()),
                "expected {s:?} to be valid"
            );
        }
    }

    #[test]
    fn validate_event_type_rejects_invalid() {
        let invalid = [
            "invalid_type",
            "Upper.Started",
            "no_double_dot",
            ".starts.with.dot",
            "trailing.dot.",
            "1starts.with.digit.thing",
        ];
        for s in invalid {
            assert!(
                EventEnvelopeV1::validate_event_type(s).is_err(),
                "expected {s:?} to be invalid"
            );
        }
    }
}
