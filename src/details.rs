//! The content and layout of the detail panel.

use jiff::{Timestamp, Zoned};

use crate::app::short_duration;
use crate::queue::{CiStatus, PriorityTier, QueueItem};

/// Marks where markup was removed, so that a line with only markup can be dropped.
const REMOVED: char = '\u{0}';

/// The description as plain text lines of at most `width` characters, at most `max_lines`
/// of them. HTML comments, images, and runs of blank lines are removed. If the text is
/// cut, the last line ends with `…`.
pub fn description_lines(body: &str, width: usize, max_lines: usize) -> Vec<String> {
    let width = width.max(1);
    let text = remove_between(body, "![", ")");
    let text = remove_between(&text, "<!--", "-->");
    let text = remove_between(&text, "<img", ">");

    // A line that had only removed markup goes away. Then runs of blank lines become one.
    let mut paragraphs: Vec<&str> = Vec::new();
    for line in text.lines() {
        let kept = line.replace(REMOVED, "");
        let kept = kept.trim();
        if kept.is_empty() && line.contains(REMOVED) {
            continue;
        }
        let blank = kept.is_empty();
        if blank && paragraphs.last().is_none_or(|p| p.is_empty()) {
            continue;
        }
        paragraphs.push(if blank { "" } else { line });
    }
    while paragraphs.last().is_some_and(|p| p.is_empty()) {
        paragraphs.pop();
    }

    let mut lines: Vec<String> = Vec::new();
    for paragraph in paragraphs {
        let paragraph = paragraph.replace(REMOVED, " ");
        if paragraph.trim().is_empty() {
            lines.push(String::new());
            continue;
        }
        wrap(&paragraph, width, &mut lines);
    }

    if lines.len() > max_lines {
        lines.truncate(max_lines);
        if let Some(last) = lines.last_mut() {
            let keep = last.chars().count().min(width - 1);
            *last = last.chars().take(keep).collect::<String>() + "…";
        }
    }
    lines
}

/// Replaces each `start…end` in `text` with one `REMOVED` mark. Without an end, the rest
/// of the text goes.
fn remove_between(text: &str, start: &str, end: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(i) = rest.find(start) {
        out.push_str(&rest[..i]);
        out.push(REMOVED);
        rest = match rest[i + start.len()..].find(end) {
            Some(j) => &rest[i + start.len() + j + end.len()..],
            None => "",
        };
    }
    out.push_str(rest);
    out
}

/// Wraps the words of `text` into lines of at most `width` characters. A word that is
/// longer than `width` is split.
fn wrap(text: &str, width: usize, lines: &mut Vec<String>) {
    let mut line = String::new();
    for word in text.split_whitespace() {
        let mut word: Vec<char> = word.chars().collect();
        let used = line.chars().count();
        if !line.is_empty() && used + 1 + word.len() > width {
            lines.push(std::mem::take(&mut line));
        }
        if !line.is_empty() {
            line.push(' ');
        }
        while word.len() > width - line.chars().count() {
            let room = width - line.chars().count();
            line.extend(word.drain(..room));
            lines.push(std::mem::take(&mut line));
        }
        line.extend(word);
    }
    if !line.is_empty() {
        lines.push(line);
    }
}

/// The heights of the list and of the detail panel below it, for a body `height` rows
/// high. The panel shows at 24 rows or more and uses `percent` of the height, with at
/// least 12 rows for the panel and 6 for the list.
pub fn panel_layout(height: u16, percent: u16) -> (u16, Option<u16>) {
    if height < 24 {
        return (height, None);
    }
    let panel = ((height as u32 * percent as u32 / 100) as u16)
        .max(12)
        .min(height - 6);
    (height - panel, Some(panel))
}

/// The label and value of each row of the detail panel, except the title and the
/// description. Rows that do not apply are left out.
pub fn detail_fields(
    item: &QueueItem,
    snoozed_until: Option<Timestamp>,
    now: &Zoned,
) -> Vec<(&'static str, String)> {
    let pr = &item.pull_request;
    let details = &pr.details;
    let mut fields = Vec::new();

    let mut reference = format!("{}#{}", pr.repo, pr.number);
    if pr.is_draft {
        reference.push_str(" · draft");
    }
    fields.push(("PR", reference));

    let mut author = format!("@{}", pr.author.login);
    if pr.author.is_bot {
        author.push_str(" (bot)");
    }
    fields.push(("Author", author));

    let mut tier = match item.tier {
        PriorityTier::PersonalReReview => "Personal re-review",
        PriorityTier::PersonalFirstReview => "Personal first review",
        PriorityTier::TeamReReview => "Team re-review",
        PriorityTier::TeamFirstReview => "Team first review",
    }
    .to_string();
    if let Some(team) = &item.team {
        tier.push_str(&format!(" · @{team}"));
    }
    fields.push(("Tier", tier));

    fields.push((
        "Waiting",
        short_duration(item.requested_at, now.timestamp()),
    ));
    if let Some(until) = snoozed_until {
        let local = until.to_zoned(now.time_zone().clone());
        fields.push(("Snoozed", format!("until {}", local.strftime("%a %H:%M"))));
    }
    fields.push((
        "Branch",
        format!("{} → {}", details.head_ref, details.base_ref),
    ));
    fields.push((
        "Changes",
        format!(
            "{} files · +{} −{}",
            details.changed_files, pr.additions, pr.deletions
        ),
    ));
    let ci = match details.ci {
        CiStatus::Passing => "✓ passing",
        CiStatus::Failing => "✗ failing",
        CiStatus::Pending => "● pending",
        CiStatus::NoChecks => "– no checks",
    };
    fields.push(("CI", ci.to_string()));
    if !details.labels.is_empty() {
        let names: Vec<&str> = details.labels.iter().map(|l| l.name.as_str()).collect();
        fields.push(("Labels", names.join(", ")));
    }
    fields
}
