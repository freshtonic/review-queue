//! The config file: `~/.config/review-queue/config.toml`.

use std::path::{Path, PathBuf};

use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    /// The time between two polls, in seconds.
    pub interval_secs: u64,
    /// The Watched Teams, as "org/team-slug".
    pub watched_teams: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            interval_secs: 60,
            watched_teams: Vec::new(),
        }
    }
}

/// `$XDG_CONFIG_HOME/review-queue/config.toml`, or `~/.config/review-queue/config.toml`.
pub fn path() -> PathBuf {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".config")
        });
    base.join("review-queue").join("config.toml")
}

/// Reads the config file. If the file does not exist, the result is the default config.
pub fn load(path: &Path) -> Result<Config, String> {
    match std::fs::read_to_string(path) {
        Ok(text) => toml::from_str(&text).map_err(|e| format!("{}: {e}", path.display())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Config::default()),
        Err(e) => Err(format!("{}: {e}", path.display())),
    }
}
