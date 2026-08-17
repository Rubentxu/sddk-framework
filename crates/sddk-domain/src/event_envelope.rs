//! EventEnvelope v1 wire-format types.
//!
//! Canonical JSON determinism invariant: this module relies on `serde_json`'s
//! default `Map<String, Value>` = `BTreeMap` ordering. DO NOT enable the
//! `serde_json/preserve_order` feature — it breaks canonicalization.

use serde::{Deserialize, Serialize};

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

pub struct EventEnvelopeV1;
