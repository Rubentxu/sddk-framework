//! Declarative pack manifests (RF-012 / ADR-0004).

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Declared risk level of a pack.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PackRisk {
    /// Low impact.
    Low,
    /// Medium impact.
    Medium,
    /// High impact.
    High,
    /// Critical impact.
    Critical,
}

/// Declared consequence class of a pack.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PackConsequence {
    /// Creates effects.
    Creates,
    /// Modifies shared state.
    Modifies,
    /// Destructive or hard to reverse.
    Irreversible,
}

/// One declared pack command with its CLI surface.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PackCommand {
    /// Command name.
    pub name: String,
    /// CLI surface tokens.
    pub surface: Vec<String>,
}

/// Declared pack dependencies.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub struct PackDependencies {
    /// Required dependency names.
    #[serde(default)]
    pub required: Vec<String>,
    /// Optional dependency names.
    #[serde(default)]
    pub optional: Vec<String>,
}

/// Pack fixture declarations.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub struct PackFixtures {
    /// Fixture paths relative to the pack root.
    #[serde(default)]
    pub paths: Vec<String>,
}

/// Pack artifact declarations.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub struct PackArtifacts {
    /// Artifact paths produced by the pack.
    #[serde(default)]
    pub paths: Vec<String>,
}

/// Root manifest structure.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PackManifest {
    /// Pack identity and metadata.
    pub pack: PackIdentity,
    /// Declared dependencies.
    #[serde(default)]
    pub dependencies: PackDependencies,
    /// Declared commands.
    #[serde(default)]
    pub commands: Vec<PackCommand>,
    /// Capability name to consequence class.
    #[serde(default)]
    pub capabilities: BTreeMap<String, PackConsequence>,
    /// Declared fixtures.
    #[serde(default)]
    pub fixtures: PackFixtures,
    /// Declared artifacts.
    #[serde(default)]
    pub artifacts: PackArtifacts,
}

/// Identity block of a pack manifest.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PackIdentity {
    /// Stable pack identifier.
    pub id: String,
    /// Pack version.
    pub version: String,
    /// Schema version of the manifest.
    pub schema_version: i32,
    /// Minimum compatible runtime.
    pub compatibility: String,
    /// Declared risk level.
    pub risk: PackRisk,
    /// Declared consequence class.
    pub consequence: PackConsequence,
    /// Human-readable description.
    #[serde(default)]
    pub description: Option<String>,
}

/// One stable pack validation diagnostic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PackDiagnostic {
    /// Stable machine-readable code.
    pub code: String,
    /// Human-readable problem.
    pub message: String,
    /// Suggested remediation.
    pub hint: String,
}

/// Errors emitted while loading a pack manifest.
#[derive(Debug, Error)]
pub enum PackError {
    /// The manifest file could not be read.
    #[error("failed to read pack manifest {path:?}: {source}")]
    Io {
        /// Requested manifest path.
        path: std::path::PathBuf,
        /// Underlying I/O error.
        source: std::io::Error,
    },
    /// The manifest is not valid TOML or does not match the pack model.
    #[error("invalid pack manifest: {0}")]
    Parse(String),
}

const MISSING_ID: &str = "PACK001";
const MISSING_VERSION: &str = "PACK002";
const INVALID_RISK: &str = "PACK003";
const INVALID_CONSEQUENCE: &str = "PACK004";
const UNKNOWN_REQUIRED_DEPENDENCY: &str = "PACK005";
const NO_COMMANDS: &str = "PACK006";
const NO_FIXTURES: &str = "PACK007";

/// Parses a pack manifest from TOML.
pub fn parse_pack_manifest(toml: &str) -> Result<PackManifest, PackError> {
    toml::from_str(toml).map_err(|error| PackError::Parse(error.to_string()))
}

/// Loads a pack manifest from a file.
pub fn load_pack_manifest(path: impl AsRef<Path>) -> Result<PackManifest, PackError> {
    let path = path.as_ref();
    let source = std::fs::read_to_string(path).map_err(|source| PackError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    parse_pack_manifest(&source)
}

/// Validates a pack manifest deterministically.
pub fn validate_pack_manifest(manifest: &PackManifest) -> Vec<PackDiagnostic> {
    let mut diagnostics = Vec::new();
    if manifest.pack.id.trim().is_empty() {
        diagnostics.push(PackDiagnostic {
            code: MISSING_ID.into(),
            message: "pack id is missing".into(),
            hint: "declare `pack.id` with a stable identifier".into(),
        });
    }
    if manifest.pack.version.trim().is_empty() {
        diagnostics.push(PackDiagnostic {
            code: MISSING_VERSION.into(),
            message: "pack version is missing".into(),
            hint: "declare `pack.version` with a semantic version".into(),
        });
    }
    let risk_valid = matches!(
        manifest.pack.risk,
        PackRisk::Low | PackRisk::Medium | PackRisk::High | PackRisk::Critical
    );
    if !risk_valid {
        diagnostics.push(PackDiagnostic {
            code: INVALID_RISK.into(),
            message: "pack risk is invalid".into(),
            hint: "use low, medium, high, or critical".into(),
        });
    }
    let consequence_valid = matches!(
        manifest.pack.consequence,
        PackConsequence::Creates | PackConsequence::Modifies | PackConsequence::Irreversible
    );
    if !consequence_valid {
        diagnostics.push(PackDiagnostic {
            code: INVALID_CONSEQUENCE.into(),
            message: "pack consequence is invalid".into(),
            hint: "use creates, modifies, or irreversible".into(),
        });
    }
    for dependency in &manifest.dependencies.required {
        if !manifest
            .dependencies
            .optional
            .iter()
            .any(|candidate| candidate == dependency)
        {
            diagnostics.push(PackDiagnostic {
                code: UNKNOWN_REQUIRED_DEPENDENCY.into(),
                message: format!("required dependency {dependency:?} is not declared"),
                hint: "list the dependency in `dependencies.optional` or `dependencies.required`"
                    .into(),
            });
        }
    }
    if manifest.commands.is_empty() {
        diagnostics.push(PackDiagnostic {
            code: NO_COMMANDS.into(),
            message: "pack declares no commands".into(),
            hint: "declare at least one [[commands]] entry".into(),
        });
    }
    if manifest.fixtures.paths.is_empty() {
        diagnostics.push(PackDiagnostic {
            code: NO_FIXTURES.into(),
            message: "pack declares no fixtures".into(),
            hint: "list deterministic fixtures under [fixtures]".into(),
        });
    }
    diagnostics
}

/// Counts validation errors.
pub fn pack_error_count(diagnostics: &[PackDiagnostic]) -> usize {
    diagnostics.len()
}

#[cfg(test)]
mod tests {
    use super::{load_pack_manifest, parse_pack_manifest, validate_pack_manifest};

    const VALID: &str = r#"
[pack]
id = "sddk-framework"
version = "0.1.0"
schema_version = 1
compatibility = ">=1.85"
risk = "medium"
consequence = "modifies"
description = "SDDK framework pack"

[dependencies]
required = []
optional = ["sddk-domain"]

[[commands]]
name = "lint"
surface = ["lint"]

[capabilities]
"git.commit" = "modifies"

[fixtures]
paths = ["tests/contract.sh"]

[artifacts]
paths = ["dist/sddk"]
"#;

    #[test]
    fn parses_valid_manifest() {
        let manifest = parse_pack_manifest(VALID).unwrap();
        assert_eq!(manifest.pack.id, "sddk-framework");
        assert_eq!(
            format!("{:?}", manifest.pack.risk).to_ascii_lowercase(),
            "medium"
        );
        assert_eq!(manifest.commands.len(), 1);
        assert_eq!(manifest.capabilities.len(), 1);
        assert!(validate_pack_manifest(&manifest).is_empty());
    }

    #[test]
    fn reports_missing_id_version_and_fixtures() {
        let manifest = parse_pack_manifest(
            r#"
[pack]
id = ""
version = ""
schema_version = 1
compatibility = ">=1.85"
risk = "medium"
consequence = "modifies"
"#,
        )
        .unwrap();
        let diagnostics = validate_pack_manifest(&manifest);
        let codes = diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code.as_str())
            .collect::<Vec<_>>();
        assert!(codes.contains(&"PACK001"));
        assert!(codes.contains(&"PACK002"));
        assert!(codes.contains(&"PACK006"));
        assert!(codes.contains(&"PACK007"));
    }

    #[test]
    fn reports_unknown_required_dependency() {
        let manifest = parse_pack_manifest(
            r#"
[pack]
id = "x"
version = "0.1.0"
schema_version = 1
compatibility = ">=1.85"
risk = "low"
consequence = "creates"

[dependencies]
required = ["ghost"]
optional = []

[[commands]]
name = "a"
surface = ["a"]
"#,
        )
        .unwrap();
        let diagnostics = validate_pack_manifest(&manifest);
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "PACK005")
        );
    }

    #[test]
    fn rejects_invalid_toml_and_missing_file() {
        assert!(parse_pack_manifest("not toml [").is_err());
        assert!(load_pack_manifest("/nonexistent/manifest.toml").is_err());
    }
}
