//! The avatar cache: one image file for each login.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// An avatar file is downloaded again after this time.
pub const MAX_AGE: Duration = Duration::from_secs(7 * 24 * 60 * 60);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Freshness {
    Fresh,
    /// The file is older than `MAX_AGE`. It can show until the new one arrives.
    Stale,
    Missing,
}

/// The file for the avatar of `login` in the cache folder `dir`.
/// GitHub logins are not case-sensitive. Characters that are not safe in a file name are
/// replaced with `_`.
pub fn path_for(dir: &Path, login: &str) -> PathBuf {
    let name: String = login
        .chars()
        .map(|c| match c {
            'a'..='z' | '0'..='9' | '-' => c,
            'A'..='Z' => c.to_ascii_lowercase(),
            _ => '_',
        })
        .collect();
    dir.join(format!("{name}.png"))
}

pub fn freshness(path: &Path, now: SystemTime) -> Freshness {
    let Ok(modified) = std::fs::metadata(path).and_then(|m| m.modified()) else {
        return Freshness::Missing;
    };
    match now.duration_since(modified) {
        Ok(age) if age > MAX_AGE => Freshness::Stale,
        _ => Freshness::Fresh,
    }
}

/// `$XDG_CACHE_HOME/review-queue/avatars`, or `~/.cache/review-queue/avatars`.
pub fn cache_dir() -> PathBuf {
    let base = std::env::var_os("XDG_CACHE_HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".cache")
        });
    base.join("review-queue").join("avatars")
}
