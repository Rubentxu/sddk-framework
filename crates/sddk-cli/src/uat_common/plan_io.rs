//! Plan read/write helpers.

#![allow(dead_code)]

use sddk_domain::UatPlan;
use std::path::Path;

/// Read a UatPlan from a YAML file.
pub fn read_plan(path: &Path) -> anyhow::Result<UatPlan> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| anyhow::anyhow!("cannot read plan {}: {e}", path.display()))?;
    serde_saphyr::from_str(&content)
        .map_err(|e| anyhow::anyhow!("invalid plan {}: {e}", path.display()))
}

/// Write a UatPlan to a YAML file.
pub fn write_plan(plan: &UatPlan, path: &Path) -> anyhow::Result<()> {
    let yaml = serde_saphyr::to_string(plan).map_err(|e| anyhow::anyhow!("serialization: {e}"))?;
    std::fs::write(path, yaml)?;
    Ok(())
}
