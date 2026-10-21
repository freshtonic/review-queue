//! Notifications: which new Review Requests to tell the user about at a poll.

use jiff::Timestamp;

use crate::queue::{PriorityTier, QueueItem};

/// The most notices for one poll. The rest go in one summary notice.
const MAX_NOTICES: usize = 3;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotifySettings {
    pub enabled: bool,
    /// The Priority Tiers whose new requests give a notice.
    pub tiers: Vec<PriorityTier>,
}

impl Default for NotifySettings {
    fn default() -> Self {
        Self {
            enabled: true,
            tiers: vec![
                PriorityTier::PersonalReReview,
                PriorityTier::PersonalFirstReview,
                PriorityTier::TeamReReview,
            ],
        }
    }
}

/// One system notification. A click on it selects the pull request `repo#number`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notice {
    pub title: String,
    pub subtitle: String,
    pub body: String,
    pub repo: String,
    pub number: u64,
}

/// The notices for a poll. `previous` is the whole Queue of the last poll, with the
/// snoozed requests (`None` at the first poll, which gives no notices), and `current` is
/// the visible Queue of this poll. A request is new when its pull request was not in
/// `previous` with the same request time: a re-request is new, a Snooze that ends is not.
pub fn to_notify(
    previous: Option<&[QueueItem]>,
    current: &[QueueItem],
    settings: &NotifySettings,
    muted_until: Option<Timestamp>,
    now: Timestamp,
) -> Vec<Notice> {
    let Some(previous) = previous else {
        return Vec::new();
    };
    if !settings.enabled || muted_until.is_some_and(|until| until > now) {
        return Vec::new();
    }
    let new: Vec<&QueueItem> = current
        .iter()
        .filter(|item| settings.tiers.contains(&item.tier))
        .filter(|item| {
            let pr = &item.pull_request;
            !previous.iter().any(|p| {
                p.pull_request.repo == pr.repo
                    && p.pull_request.number == pr.number
                    && p.requested_at == item.requested_at
            })
        })
        .collect();

    let mut notices: Vec<Notice> = new.iter().take(MAX_NOTICES).map(|i| notice(i)).collect();
    let rest = &new[new.len().min(MAX_NOTICES)..];
    if let Some(first) = rest.first() {
        notices.push(Notice {
            title: format!("{} more review requests", rest.len()),
            subtitle: String::new(),
            body: rest
                .iter()
                .map(|i| format!("{}#{}", i.pull_request.repo, i.pull_request.number))
                .collect::<Vec<_>>()
                .join(", "),
            repo: first.pull_request.repo.clone(),
            number: first.pull_request.number,
        });
    }
    notices
}

/// The notice for one request.
pub fn notice(item: &QueueItem) -> Notice {
    let pr = &item.pull_request;
    let team = item.team.as_deref().unwrap_or_default();
    let title = match item.tier {
        PriorityTier::PersonalReReview => "Re-review requested".to_string(),
        PriorityTier::PersonalFirstReview => "Review requested".to_string(),
        PriorityTier::TeamReReview => format!("Team re-review requested (@{team})"),
        PriorityTier::TeamFirstReview => format!("Team review requested (@{team})"),
    };
    Notice {
        title,
        subtitle: format!("{}#{}", pr.repo, pr.number),
        body: format!("{} · @{}", pr.title, pr.author.login),
        repo: pr.repo.clone(),
        number: pr.number,
    }
}

/// The mute file, next to the Snooze file.
pub fn mute_path() -> std::path::PathBuf {
    crate::snooze::store_path().with_file_name("mute.json")
}

/// Reads the end of the mute. A missing or bad file is no mute.
pub fn load_mute(path: &std::path::Path) -> Option<Timestamp> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str::<Option<Timestamp>>(&text).ok())
        .flatten()
}

/// Writes the end of the mute (`None`: not muted).
pub fn save_mute(path: &std::path::Path, until: Option<Timestamp>) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let temporary = path.with_extension("json.tmp");
    std::fs::write(&temporary, serde_json::to_string(&until)?)?;
    std::fs::rename(&temporary, path)
}
