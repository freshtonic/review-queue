//! Draws the App on the terminal.

use jiff::Timestamp;
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap};

use crate::app::{App, short_duration};
use crate::queue::{PriorityTier, QueueItem};

/// The current config, as the help page shows it.
pub struct ConfigView {
    pub path: String,
    pub interval_secs: u64,
    pub watched_teams: Vec<String>,
    pub token_source: &'static str,
}

const TIERS: [(PriorityTier, &str); 4] = [
    (PriorityTier::PersonalReReview, "Personal re-review"),
    (PriorityTier::PersonalFirstReview, "Personal first review"),
    (PriorityTier::TeamReReview, "Team re-review"),
    (PriorityTier::TeamFirstReview, "Team first review"),
];

pub fn draw(frame: &mut Frame, app: &App, now: Timestamp, config: &ConfigView) {
    let [title, body, footer] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .areas(frame.area());

    draw_title(frame, title, app, now);
    draw_queue(frame, body, app, now);
    frame.render_widget(
        Paragraph::new("j/k move · enter open · r refresh · ? help · q quit").dark_gray(),
        footer,
    );

    if app.show_help {
        draw_help(frame, body, config);
    }
}

fn draw_title(frame: &mut Frame, area: Rect, app: &App, now: Timestamp) {
    let status = app.status(now);
    let status_style = if app.error.is_some() {
        Style::new().fg(Color::Red)
    } else {
        Style::new().fg(Color::DarkGray)
    };
    let [left, right] = Layout::horizontal([
        Constraint::Min(0),
        Constraint::Length(status.chars().count() as u16),
    ])
    .areas(area);
    frame.render_widget(Paragraph::new(" Review Queue").bold(), left);
    frame.render_widget(Paragraph::new(status).style(status_style), right);
}

fn draw_queue(frame: &mut Frame, area: Rect, app: &App, now: Timestamp) {
    let width = area.width as usize;
    let mut lines: Vec<ListItem> = Vec::new();
    let mut selected_line = None;
    let mut index = 0;

    for (tier, name) in TIERS {
        let items: Vec<&QueueItem> = app.queue.iter().filter(|i| i.tier == tier).collect();
        lines.push(ListItem::new(tier_header(name, items.len(), width)));
        for item in items {
            if index == app.selected {
                selected_line = Some(lines.len());
            }
            lines.push(ListItem::new(row(item, now, width)));
            index += 1;
        }
    }

    let list = List::new(lines)
        .highlight_style(Style::new().add_modifier(Modifier::REVERSED))
        .highlight_symbol("▶ ");
    let mut state = ListState::default().with_selected(selected_line);
    frame.render_stateful_widget(list, area, &mut state);
}

fn tier_header(name: &str, count: usize, width: usize) -> Line<'static> {
    let label = format!("── {name} ({count}) ");
    let fill = "─".repeat(width.saturating_sub(label.chars().count() + 2));
    Line::from(format!("{label}{fill}")).style(Style::new().fg(Color::Cyan).bold())
}

fn row(item: &QueueItem, now: Timestamp, width: usize) -> Line<'static> {
    let pr = &item.pull_request;
    let reference = format!("{}#{}", pr.repo, pr.number);
    let author = format!("@{}", pr.author.login);
    let size = format!("+{} −{}", pr.additions, pr.deletions);
    let age = short_duration(item.requested_at, now);

    let mut tags = String::new();
    if pr.is_draft {
        tags.push_str(" [draft]");
    }
    if let Some(team) = &item.team {
        tags.push_str(&format!(" [@{team}]"));
    }

    // The reference and the Wait Time always show. In a narrow terminal, the diff size
    // goes first, then the author. The title and tags get the remaining columns.
    // Each column width includes the space in front of it. The highlight symbol takes 2.
    const REFERENCE: usize = 28;
    const AGE: usize = 1 + 4;
    const AUTHOR: usize = 1 + 18;
    const SIZE: usize = 1 + 14;
    const MIN_TITLE: usize = 12;
    let available = width.saturating_sub(2 + REFERENCE + AGE);
    let show_author = available >= MIN_TITLE + AUTHOR;
    let show_size = available >= MIN_TITLE + AUTHOR + SIZE;
    let flexible =
        available - if show_author { AUTHOR } else { 0 } - if show_size { SIZE } else { 0 };
    let tags = truncate(&tags, flexible.saturating_sub(1));
    let title_width = flexible.saturating_sub(1 + tags.chars().count());

    let mut spans = vec![
        Span::raw(format!("{:<REFERENCE$}", truncate(&reference, REFERENCE))),
        Span::raw(format!(
            " {:<title_width$}",
            truncate(&pr.title, title_width)
        )),
        Span::styled(tags, Style::new().fg(Color::Yellow)),
    ];
    if show_author {
        spans.push(Span::styled(
            format!(" {:<18}", truncate(&author, 18)),
            Style::new().fg(Color::Magenta),
        ));
    }
    if show_size {
        spans.push(Span::styled(
            format!(" {size:>14}"),
            Style::new().fg(Color::DarkGray),
        ));
    }
    spans.push(Span::styled(format!(" {age:>4}"), Style::new().bold()));
    Line::from(spans)
}

fn truncate(text: &str, width: usize) -> String {
    if text.chars().count() <= width {
        text.to_string()
    } else {
        let mut cut: String = text.chars().take(width.saturating_sub(1)).collect();
        cut.push('…');
        cut
    }
}

fn draw_help(frame: &mut Frame, area: Rect, config: &ConfigView) {
    let keys = [
        ("j / ↓", "Move the selection down"),
        ("k / ↑", "Move the selection up"),
        ("enter", "Open the selected pull request in the browser"),
        ("r", "Refresh the queue now"),
        ("? / esc", "Show or close this help page"),
        ("q", "Quit"),
    ];
    let mut lines: Vec<Line> = vec![Line::from("Actions").bold()];
    for (key, action) in keys {
        lines.push(Line::from(vec![
            Span::styled(format!("  {key:<10}"), Style::new().fg(Color::Cyan)),
            Span::raw(action),
        ]));
    }
    lines.extend([
        Line::raw(""),
        Line::from("How the queue works").bold(),
        Line::raw("  A pull request is in the queue while GitHub lists you, or one of your"),
        Line::raw("  watched teams, as a requested reviewer. Your own pull requests are never shown."),
        Line::raw("  Order: personal re-review, personal first review, team re-review, team first review."),
        Line::raw("  A re-review is a request after you submitted a review in any state."),
        Line::raw("  In each tier, the longest wait is first. Bot pull requests are last in their tier."),
        Line::raw("  The queue shows only the state on GitHub. Submit your review to remove an item."),
        Line::raw("  This tool gets its token from `gh auth token`, unless GITHUB_TOKEN is set."),
        Line::raw(""),
        Line::from("Current config").bold(),
        Line::raw(format!("  file           {}", config.path)),
        Line::raw(format!("  interval_secs  {}", config.interval_secs)),
        Line::raw(format!(
            "  watched_teams  {}",
            if config.watched_teams.is_empty() {
                "(none: personal requests only)".to_string()
            } else {
                config.watched_teams.join(", ")
            }
        )),
        Line::raw(format!("  token          {}", config.token_source)),
    ]);

    let height = (lines.len() as u16 + 2).min(area.height);
    let width = 92.min(area.width);
    let popup = Rect {
        x: area.x + (area.width - width) / 2,
        y: area.y + (area.height - height) / 2,
        width,
        height,
    };
    frame.render_widget(Clear, popup);
    frame.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .block(Block::new().borders(Borders::ALL).title(" Help ")),
        popup,
    );
}
