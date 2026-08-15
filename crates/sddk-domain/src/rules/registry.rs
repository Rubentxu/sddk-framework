//! Rule registry: parses `architecture-rules.yaml`.

use super::ARCHITECTURE_RULES_SCHEMA_VERSION;
use super::types::{ArchitectureRule, Waiver};
use serde::Deserialize;
use std::collections::BTreeMap;
use thiserror::Error;

#[derive(Debug, Deserialize)]
struct RulesFile {
    #[serde(default)]
    schema_version: Option<String>,
    rules: Vec<ArchitectureRule>,
    #[serde(default)]
    waivers: Vec<Waiver>,
}

#[derive(Debug, Error)]
pub enum RegistryError {
    #[error("failed to parse architecture-rules YAML: {0}")]
    Parse(#[from] serde_saphyr::Error),
    #[error("unsupported architecture-rules schema_version {actual}; supported is {supported}")]
    UnsupportedSchemaVersion {
        actual: String,
        supported: &'static str,
    },
    #[error("architecture rule entry missing id field")]
    MissingRuleId,
    #[error("duplicate architecture rule id: {0}")]
    DuplicateRuleId(String),
}

#[derive(Debug, Clone)]
pub struct RuleRegistry {
    rules: Vec<ArchitectureRule>,
    waivers: BTreeMap<String, Waiver>,
}

impl RuleRegistry {
    pub fn from_yaml_str(yaml: &str) -> Result<Self, RegistryError> {
        let file: RulesFile = serde_saphyr::from_str(yaml)?;
        if let Some(v) = file.schema_version
            && v != ARCHITECTURE_RULES_SCHEMA_VERSION
        {
            return Err(RegistryError::UnsupportedSchemaVersion {
                actual: v,
                supported: ARCHITECTURE_RULES_SCHEMA_VERSION,
            });
        }
        let mut rules = Vec::with_capacity(file.rules.len());
        let mut seen = std::collections::HashSet::new();
        for rule in file.rules {
            if rule.id.is_empty() {
                return Err(RegistryError::MissingRuleId);
            }
            if !seen.insert(rule.id.clone()) {
                return Err(RegistryError::DuplicateRuleId(rule.id));
            }
            rules.push(rule);
        }
        let waivers = file
            .waivers
            .into_iter()
            .map(|w| (w.rule_id.clone(), w))
            .collect();
        Ok(Self { rules, waivers })
    }
    pub fn waiver_for(&self, rule_id: &str) -> Option<&Waiver> {
        self.waivers.get(rule_id)
    }
    pub fn iter(&self) -> impl Iterator<Item = &ArchitectureRule> {
        self.rules.iter()
    }
}
