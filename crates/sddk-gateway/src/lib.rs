//! Default-deny capability gateway for SDDK external effects.
//!
//! The gateway owns the pipeline from ADR-0005: policy evaluation, approval
//! resolution, typed execution without a shell, safe filesystem access, output
//! sanitization, and receipt lifecycle (`started` -> `succeeded|failed`).

#![forbid(unsafe_code)]
#![deny(clippy::all)]
#![warn(missing_docs)]

mod artifact_store;
mod filesystem;
mod forge;
mod gateway;
mod git;
mod permissions;
mod policy;
mod release;
mod runner;

pub use artifact_store::{ArtifactMeta, ArtifactStore, ArtifactStoreError};
pub use filesystem::{FsError, ScopedFs};
pub use forge::{
    CheckState, Forge, ForgeError, GitHubForge, MergeReceipt, MockForge, PrReceipt, PrRequest,
    ReleaseReceipt, ReleaseRequest, ReleaseState,
};
pub use gateway::{CapabilityGateway, CapabilityPlan, CapabilityPlanInput, GatewayError};
pub use git::{GitBranch, GitCommit, GitError, GitExecutor, GitInspect, GitTag};
pub use permissions::{AgentPermissions, PermissionDecision, PermissionPolicy, PermissionsError};
pub use policy::{CapabilityPolicy, Consequence, PolicyDecision, Risk};
pub use release::{
    ReleaseOutcome, ReleasePlan, ReleasePlanInput, ReleaseStep, apply_release, plan_release,
    reconcile_pending,
};
pub use runner::{RunOutcome, RunSpec, RunnerError, run};
pub use sddk_storage::CapabilityReceipt;

use serde_json::Value;

/// Keys whose values are treated as secrets and redacted from persisted output.
const SECRET_KEY_PATTERN: [&str; 9] = [
    "api_key",
    "api_key_id",
    "authorization",
    "auth_token",
    "cookie",
    "credential",
    "password",
    "secret",
    "token",
];

/// Deterministic request key used to derive idempotency and receipt identifiers.
pub(crate) fn stable_request_key(
    project_id: &str,
    cycle_id: &Option<String>,
    capability: &str,
    args: &[String],
    reason: &str,
) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(project_id.as_bytes());
    if let Some(cycle_id) = cycle_id {
        hasher.update(cycle_id.as_bytes());
    }
    hasher.update(capability.as_bytes());
    for arg in args {
        hasher.update(arg.as_bytes());
    }
    hasher.update(reason.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Recursively masks values under secret-like keys.
pub fn redact(value: Value) -> Value {
    match value {
        Value::Object(mut object) => {
            for key in object.keys().cloned().collect::<Vec<_>>() {
                let normalized = key.to_ascii_lowercase();
                if SECRET_KEY_PATTERN.iter().any(|pattern| {
                    normalized == *pattern || normalized.ends_with(&format!("_{pattern}"))
                }) {
                    object.insert(key, Value::String("<redacted>".to_owned()));
                } else if let Some(inner) = object.get(&key).cloned() {
                    object.insert(key, redact(inner));
                }
            }
            Value::Object(object)
        }
        Value::Array(values) => Value::Array(values.into_iter().map(redact).collect()),
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::redact;

    #[test]
    fn redaction_masks_secret_keys_recursively() {
        let input = json!({
            "branch": "feature/x",
            "credentials": {"password": "hunter2", "username": "alice"},
            "headers": {"authorization": "Bearer abc", "x-request-id": "123"}
        });
        let output = redact(input);
        assert_eq!(output["credentials"]["password"], "<redacted>");
        assert_eq!(output["credentials"]["username"], "alice");
        assert_eq!(output["headers"]["authorization"], "<redacted>");
        assert_eq!(output["headers"]["x-request-id"], "123");
        assert_eq!(output["branch"], "feature/x");
    }

    #[test]
    fn redaction_masks_keys_in_arrays() {
        let input = json!([{"token": "abc"}, {"value": 1}]);
        let output = redact(input);
        assert_eq!(output[0]["token"], "<redacted>");
        assert_eq!(output[1]["value"], 1);
    }
}
