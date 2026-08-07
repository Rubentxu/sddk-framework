//! Explicit XDG path resolution for project adoption.

use std::path::{Path, PathBuf};

use sddk_domain::{AdoptionStoragePaths, ProjectId, WorkspaceId};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Explicit environment values used by XDG path resolution.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct XdgEnvironment {
    /// Home directory used only for missing XDG overrides.
    pub home: Option<PathBuf>,
    /// Optional `XDG_DATA_HOME` override.
    pub data_home: Option<PathBuf>,
    /// Optional `SDDK_DATA_DIR` override (takes precedence over `XDG_DATA_HOME`
    /// for the data root; all framework state lives under it).
    pub sddk_data_dir: Option<PathBuf>,
    /// Optional `XDG_STATE_HOME` override.
    pub state_home: Option<PathBuf>,
    /// Optional `XDG_CACHE_HOME` override.
    pub cache_home: Option<PathBuf>,
}

/// Fully resolved absolute paths for one project workspace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct AdoptionPaths {
    /// Project-shared knowledge vault directory.
    pub vault: PathBuf,
    /// Project-shared artifact directory.
    pub artifacts: PathBuf,
    /// Project-shared SQLite database.
    pub ledger: PathBuf,
    /// SDDK-wide cache directory.
    pub cache: PathBuf,
    /// Workspace-specific adoption receipt.
    pub receipt: PathBuf,
}

impl AdoptionPaths {
    /// Converts paths to the receipt wire representation after UTF-8 validation.
    pub fn to_storage_paths(&self) -> Result<AdoptionStoragePaths, PathResolutionError> {
        Ok(AdoptionStoragePaths {
            vault: path_string(&self.vault)?,
            artifacts: path_string(&self.artifacts)?,
            ledger: path_string(&self.ledger)?,
            cache: path_string(&self.cache)?,
            receipt: path_string(&self.receipt)?,
        })
    }
}

/// Errors emitted while resolving XDG storage paths.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum PathResolutionError {
    /// A supplied home or XDG directory is not absolute.
    #[error("{variable} must be an absolute path: {path:?}")]
    NonAbsolute {
        /// Environment variable represented by the value.
        variable: &'static str,
        /// Rejected path.
        path: PathBuf,
    },
    /// An XDG fallback was needed but no home directory was supplied.
    #[error("HOME is required when an XDG directory is not set")]
    MissingHome,
    /// A project or workspace identifier is unsafe for path construction.
    #[error("unsafe identity component: {0}")]
    UnsafeIdentity(String),
    /// A path cannot be represented in the JSON receipt.
    #[error("path is not valid UTF-8: {0:?}")]
    NonUtf8Path(PathBuf),
}

/// Resolves project and workspace storage paths without reading process state.
pub fn resolve_xdg_paths(
    environment: &XdgEnvironment,
    project_id: &str,
    workspace_id: &str,
) -> Result<AdoptionPaths, PathResolutionError> {
    ProjectId::new(project_id).map_err(|_| unsafe_identity(project_id))?;
    WorkspaceId::new(workspace_id).map_err(|_| unsafe_identity(workspace_id))?;
    validate_optional("HOME", environment.home.as_deref())?;
    validate_optional("XDG_DATA_HOME", environment.data_home.as_deref())?;
    validate_optional("SDDK_DATA_DIR", environment.sddk_data_dir.as_deref())?;
    validate_optional("XDG_STATE_HOME", environment.state_home.as_deref())?;
    validate_optional("XDG_CACHE_HOME", environment.cache_home.as_deref())?;

    let data_home = resolve_base(
        environment
            .sddk_data_dir
            .as_deref()
            .or(environment.data_home.as_deref()),
        environment.home.as_deref(),
        ".local/share",
        dirs::data_dir(),
    )?;
    let state_home = resolve_base(
        environment.state_home.as_deref(),
        environment.home.as_deref(),
        ".local/state",
        dirs::state_dir(),
    )?;
    let cache_home = resolve_base(
        environment.cache_home.as_deref(),
        environment.home.as_deref(),
        ".cache",
        dirs::cache_dir(),
    )?;
    let project_data = data_home.join("sddk/projects").join(project_id);
    let project_state = state_home.join("sddk/projects").join(project_id);
    Ok(AdoptionPaths {
        vault: project_data.join("vault"),
        artifacts: project_data.join("artifacts"),
        ledger: project_state.join("ledger.sqlite"),
        cache: cache_home.join("sddk"),
        receipt: project_data
            .join("workspaces")
            .join(workspace_id)
            .join("adoption.json"),
    })
}

fn validate_optional(
    variable: &'static str,
    value: Option<&Path>,
) -> Result<(), PathResolutionError> {
    if let Some(path) = value
        && !path.is_absolute()
    {
        return Err(PathResolutionError::NonAbsolute {
            variable,
            path: path.to_path_buf(),
        });
    }
    Ok(())
}

/// Resolution order for a base directory:
/// 1. Explicit override (XDG_* or SDDK_DATA_DIR).
/// 2. `HOME` fallback for the given subdirectory (Unix convention).
/// 3. Platform dir via the `dirs` crate (macOS `~/Library/...`, Windows
///    `%APPDATA%`/`%LOCALAPPDATA%`) — required where `HOME` does not exist.
fn resolve_base(
    override_path: Option<&Path>,
    home: Option<&Path>,
    fallback: &str,
    platform_dir: Option<PathBuf>,
) -> Result<PathBuf, PathResolutionError> {
    override_path
        .map(Path::to_path_buf)
        .or_else(|| home.map(|home| home.join(fallback)))
        .or(platform_dir)
        .ok_or(PathResolutionError::MissingHome)
}

fn path_string(path: &Path) -> Result<String, PathResolutionError> {
    path.to_str()
        .map(str::to_owned)
        .ok_or_else(|| PathResolutionError::NonUtf8Path(path.to_path_buf()))
}

fn unsafe_identity(identity: &str) -> PathResolutionError {
    PathResolutionError::UnsafeIdentity(identity.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uses_explicit_xdg_overrides() {
        let environment = XdgEnvironment {
            home: None,
            data_home: Some("/xdg/data".into()),
            state_home: Some("/xdg/state".into()),
            cache_home: Some("/xdg/cache".into()),
            ..XdgEnvironment::default()
        };
        let paths = resolve_xdg_paths(&environment, "p-project", "w-workspace").unwrap();
        assert_eq!(
            paths.vault,
            Path::new("/xdg/data/sddk/projects/p-project/vault")
        );
        assert_eq!(
            paths.ledger,
            Path::new("/xdg/state/sddk/projects/p-project/ledger.sqlite")
        );
        assert_eq!(paths.cache, Path::new("/xdg/cache/sddk"));
        assert_eq!(
            paths.receipt,
            Path::new("/xdg/data/sddk/projects/p-project/workspaces/w-workspace/adoption.json")
        );
    }

    #[test]
    fn sddk_data_dir_overrides_data_home() {
        let environment = XdgEnvironment {
            home: None,
            data_home: Some("/xdg/data".into()),
            sddk_data_dir: Some("/sddk-root".into()),
            state_home: Some("/xdg/state".into()),
            cache_home: Some("/xdg/cache".into()),
        };
        let paths = resolve_xdg_paths(&environment, "p-project", "w-workspace").unwrap();
        assert_eq!(
            paths.vault,
            Path::new("/sddk-root/sddk/projects/p-project/vault")
        );
        assert_eq!(
            paths.receipt,
            Path::new("/sddk-root/sddk/projects/p-project/workspaces/w-workspace/adoption.json")
        );
    }

    #[test]
    fn falls_back_to_home_for_each_missing_override() {
        let environment = XdgEnvironment {
            home: Some("/home/tester".into()),
            ..XdgEnvironment::default()
        };
        let paths = resolve_xdg_paths(&environment, "p-project", "w-workspace").unwrap();
        assert_eq!(
            paths.artifacts,
            Path::new("/home/tester/.local/share/sddk/projects/p-project/artifacts")
        );
        assert_eq!(
            paths.ledger,
            Path::new("/home/tester/.local/state/sddk/projects/p-project/ledger.sqlite")
        );
        assert_eq!(paths.cache, Path::new("/home/tester/.cache/sddk"));
    }

    #[test]
    fn falls_back_to_platform_dirs_without_home() {
        // Simulates macOS/Windows where HOME may not exist: resolution must
        // fall back to `dirs` platform directories instead of failing.
        let environment = XdgEnvironment::default();
        let paths = resolve_xdg_paths(&environment, "p-project", "w-workspace").unwrap();
        assert!(paths.vault.is_absolute());
        assert!(paths.artifacts.is_absolute());
        assert!(paths.ledger.is_absolute());
        assert!(paths.cache.is_absolute());
        assert!(paths.vault.ends_with("sddk/projects/p-project/vault"));
        assert!(
            paths
                .ledger
                .ends_with("sddk/projects/p-project/ledger.sqlite")
        );
        assert!(paths.cache.ends_with("sddk"));
    }

    #[test]
    fn rejects_relative_and_unsafe_inputs() {
        let relative = XdgEnvironment {
            home: Some("relative".into()),
            ..XdgEnvironment::default()
        };
        assert!(matches!(
            resolve_xdg_paths(&relative, "p-project", "w-workspace"),
            Err(PathResolutionError::NonAbsolute {
                variable: "HOME",
                ..
            })
        ));
        let absolute = XdgEnvironment {
            home: Some("/home/tester".into()),
            ..XdgEnvironment::default()
        };
        assert!(matches!(
            resolve_xdg_paths(&absolute, "../escape", "w-workspace"),
            Err(PathResolutionError::UnsafeIdentity(_))
        ));
    }
}
