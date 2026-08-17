//! EventEnvelope v1 wire-format types.
//!
//! Canonical JSON determinism invariant: this module relies on `serde_json`'s
//! default `Map<String, Value>` = `BTreeMap` ordering. DO NOT enable the
//! `serde_json/preserve_order` feature — it breaks canonicalization.

use serde::{Deserialize, Serialize};
use serde_json::Value;

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
