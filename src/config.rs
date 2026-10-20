//! The config file: `~/.config/review-queue/config.toml`.

use std::path::{Path, PathBuf};

use jiff::SignedDuration;
use jiff::civil::{Time, Weekday};
use serde::Deserialize;

use crate::queue::{Exclusions, LatenessThresholds};
use crate::snooze::WorkingTime;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    /// The time between two polls, in seconds.
    pub interval_secs: u64,
    /// The Watched Teams, as "org/team-slug".
    pub watched_teams: Vec<String>,
    /// The start of the working day, as "HH:MM".
    pub work_start: String,
    /// The end of the working day, as "HH:MM".
    pub work_end: String,
    /// The working days, as "Mon", "Tue", and so on.
    pub work_days: Vec<String>,
    /// The working hours after which a request is Stale (its Wait Time is amber).
    pub stale_after_hours: u64,
    /// The working hours after which a request is Overdue (its Wait Time is red).
    pub overdue_after_hours: u64,
    /// Repositories never shown in the Queue, as "owner/name".
    pub exclude_repos: Vec<String>,
    /// Authors whose pull requests are never shown in the Queue, as logins.
    pub exclude_authors: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            interval_secs: 60,
            watched_teams: Vec::new(),
            work_start: "09:00".into(),
            work_end: "17:30".into(),
            work_days: ["Mon", "Tue", "Wed", "Thu", "Fri"]
                .map(String::from)
                .to_vec(),
            stale_after_hours: 4,
            overdue_after_hours: 8,
            exclude_repos: Vec::new(),
            exclude_authors: Vec::new(),
        }
    }
}

impl Config {
    pub fn working_time(&self) -> Result<WorkingTime, String> {
        let time = |text: &str| {
            text.parse::<Time>().map_err(|_| {
                format!("{text:?} is not a time. Use \"HH:MM\", for example \"09:00\".")
            })
        };
        let day = |text: &String| match text.to_ascii_lowercase().as_str() {
            "mon" => Ok(Weekday::Monday),
            "tue" => Ok(Weekday::Tuesday),
            "wed" => Ok(Weekday::Wednesday),
            "thu" => Ok(Weekday::Thursday),
            "fri" => Ok(Weekday::Friday),
            "sat" => Ok(Weekday::Saturday),
            "sun" => Ok(Weekday::Sunday),
            _ => Err(format!(
                "{text:?} is not a day. Use \"Mon\", \"Tue\", and so on."
            )),
        };
        let working = WorkingTime {
            start: time(&self.work_start)?,
            end: time(&self.work_end)?,
            days: self.work_days.iter().map(day).collect::<Result<_, _>>()?,
        };
        if working.start >= working.end {
            return Err("work_start must be before work_end.".into());
        }
        if working.days.is_empty() {
            return Err("work_days must have at least one day.".into());
        }
        Ok(working)
    }
}

impl Config {
    pub fn exclusions(&self) -> Exclusions {
        Exclusions {
            repos: self.exclude_repos.clone(),
            authors: self.exclude_authors.clone(),
        }
    }

    pub fn lateness_thresholds(&self) -> LatenessThresholds {
        LatenessThresholds {
            stale_after: SignedDuration::from_hours(self.stale_after_hours as i64),
            overdue_after: SignedDuration::from_hours(self.overdue_after_hours as i64),
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
