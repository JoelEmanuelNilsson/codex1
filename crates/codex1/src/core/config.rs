//! Optional user config at `~/.codex1/config.toml`.
//!
//! Codex1 does not require auth (it is a local mission harness), so this
//! module is intentionally small. Config, if present, currently stores
//! nothing enforced by the CLI — it exists so the `doctor` command can
//! report whether the path exists and is readable.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::core::error::CliError;

/// Top-level config shape. All fields are optional.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub default_repo_root: Option<PathBuf>,
}

/// Compute the default config path (`$HOME/.codex1/config.toml`).
#[must_use]
pub fn default_config_path() -> Option<PathBuf> {
    home_dir().map(|home| home.join(".codex1").join("config.toml"))
}

/// Read the config file if it exists. Missing file = `Ok(None)`.
/// Parse/IO failures surface as canonical `CliError` variants so the
/// error set stays closed — there is no `INTERNAL` escape hatch.
pub fn load(path: &Path) -> Result<Option<Config>, CliError> {
    if !path.exists() {
        return Ok(None);
    }
    let raw = std::fs::read_to_string(path)?;
    let parsed: Config = toml::from_str(&raw).map_err(|err| CliError::ParseError {
        message: format!("config.toml parse error: {err}"),
    })?;
    Ok(Some(parsed))
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}
