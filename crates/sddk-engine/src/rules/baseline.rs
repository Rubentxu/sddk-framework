//! Baseline consumer for `baseline-dependency-entropy.json`.

use std::path::PathBuf;
use serde::Deserialize;
use sddk_domain::BaselineRef;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrossCrateImport {
    pub from_file: String,
    pub line: u32,
    pub from_crate: String,
    pub to_crate_raw: String,
    pub to_crate: String,
}

#[derive(Debug, Clone)]
pub struct Baseline {
    pub ref_: BaselineRef,
    pub cross_crate_imports: Vec<CrossCrateImport>,
}

#[derive(Debug, Deserialize)]
struct BaselineFile {
    schema_version: String,
    #[serde(default)] head_anchor: Option<String>,
    #[serde(default)] captured_at: Option<String>,
    #[serde(default)] cross_crate_coupling_baseline: CrossCrateCouplingBaseline,
}

#[derive(Debug, Default, Deserialize)]
struct CrossCrateCouplingBaseline {
    #[serde(default)] cross_crate_imports: Vec<RawCrossCrateImport>,
}

#[derive(Debug, Deserialize)]
struct RawCrossCrateImport {
    #[serde(default)] from_file: String,
    #[serde(default)] line: u32,
    to_crate: String,
}

#[derive(Debug, Error)]
pub enum BaselineError {
    #[error("baseline I/O {path:?}: {message}")]
    Io { path: PathBuf, message: String },
    #[error("baseline schema_version {actual} not in supported set {supported:?}")]
    UnsupportedSchemaVersion { actual: String, supported: Vec<String> },
}

#[derive(Debug, Clone)]
pub struct BaselineConsumer {
    path: PathBuf,
    supported_versions: Vec<String>,
}

impl BaselineConsumer {
    pub fn new(path: impl AsRef<std::path::Path>, supported_versions: &[&str]) -> Result<Self, BaselineError> {
        Ok(Self { path: path.as_ref().to_path_buf(), supported_versions: supported_versions.iter().map(|s| (*s).to_owned()).collect() })
    }

    pub fn load(&self) -> Result<Baseline, BaselineError> {
        let bytes = std::fs::read(&self.path).map_err(|e| BaselineError::Io { path: self.path.clone(), message: e.to_string() })?;
        let file: BaselineFile = serde_json::from_slice(&bytes).map_err(|e| BaselineError::Io { path: self.path.clone(), message: e.to_string() })?;
        if !self.supported_versions.iter().any(|v| v == &file.schema_version) {
            return Err(BaselineError::UnsupportedSchemaVersion { actual: file.schema_version, supported: self.supported_versions.clone() });
        }
        let sha = {
            use sha2::{Digest, Sha256};
            let mut h = Sha256::new();
            h.update(&bytes);
            format!("sha256:{:x}", h.finalize())
        };
        let ref_ = BaselineRef {
            schema_version: file.schema_version,
            head_anchor: file.head_anchor.unwrap_or_else(|| "unknown".to_owned()),
            sha256: sha,
            cycle_id: None,
            captured_at: file.captured_at.unwrap_or_else(|| "unknown".to_owned()),
        };
        let cross_crate_imports = file.cross_crate_coupling_baseline.cross_crate_imports.into_iter().map(|raw| {
            let parts: Vec<&str> = raw.from_file.split('/').collect();
            let from_crate = if parts.len() >= 2 && parts[0] == "crates" { parts[1].to_owned() }
                else if parts.len() >= 3 && parts[0] == ".." && parts[1] == "crates" { parts[2].to_owned() }
                else { "unknown".to_owned() };
            let to_crate = if raw.to_crate.starts_with("sddk-") { raw.to_crate.clone() } else { format!("sddk-{}", raw.to_crate) };
            CrossCrateImport { from_file: raw.from_file, line: raw.line, from_crate, to_crate_raw: raw.to_crate, to_crate }
        }).collect();
        Ok(Baseline { ref_, cross_crate_imports })
    }
}
