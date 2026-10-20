//! Draws the App on the terminal.

use jiff::Timestamp;
use jiff::tz::TimeZone;
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph};

use image::DynamicImage;
use ratatui_image::Resize;
use ratatui_image::picker::{Picker, ProtocolType};
use ratatui_image::protocol::Protocol;

use std::collections::HashMap;
use std::path::PathBuf;

use image::ImageReader;
use ratatui::layout::Size;

use crate::app::{App, Picker as SnoozePicker, short_duration};
use crate::avatar;
use crate::burst::{Burst, DURATION, render, render_ascii};
use crate::details::{description_lines, detail_fields, panel_layout};
use crate::queue::{CiStatus, PriorityTier, QueueItem};

/// The current config, as the help page shows it.
pub struct ConfigView {
    pub path: String,
    pub interval_secs: u64,
    pub watched_teams: Vec<String>,
    /// The working hours and days, for example "09:00–17:30, Mon Tue Wed Thu Fri".
    pub working_time: String,
    pub snooze_file: String,
    pub token_source: &'static str,
    /// The raster graphics protocol, or "none (ASCII)".
    pub graphics: &'static str,
}

const TIERS: [(PriorityTier, &str); 4] = [
    (PriorityTier::PersonalReReview, "Personal re-review"),
    (PriorityTier::PersonalFirstReview, "Personal first review"),
    (PriorityTier::TeamReReview, "Team re-review"),
    (PriorityTier::TeamFirstReview, "Team first review"),
];

/// Frames per second of a burst.
const BURST_FPS: f32 = 30.0;

/// The terminal's raster graphics, and the burst frames that were made for it.
pub struct Graphics {
    /// `None` when the terminal has no raster graphics, or with `--no-images`.
    picker: Option<Picker>,
    /// For the panel avatar when there are no raster graphics.
    halfblocks: Picker,
    frames: Option<BurstFrames>,
    avatar_dir: PathBuf,
    /// The decoded avatars and their protocols. `None` when the file cannot be read.
    avatars: HashMap<String, Option<Avatar>>,
}

struct Avatar {
    image: DynamicImage,
    row: Option<Protocol>,
    panel: Option<Protocol>,
}

/// The size of the panel avatar, in cells. Cells are about twice as tall as wide.
const PANEL_AVATAR: Size = Size {
    width: 12,
    height: 6,
};

/// The frames of one burst, each made when it is first needed. With Kitty, each frame is
/// uploaded once, the first time that it shows.
struct BurstFrames {
    started_at: Timestamp,
    area: Rect,
    frames: Vec<Option<Protocol>>,
}

impl Graphics {
    pub fn new(picker: Option<Picker>, avatar_dir: PathBuf) -> Self {
        Self {
            picker,
            halfblocks: Picker::halfblocks(),
            frames: None,
            avatar_dir,
            avatars: HashMap::new(),
        }
    }

    /// The avatar file of `login` changed, so its images must be made again.
    pub fn avatar_changed(&mut self, login: &str) {
        self.avatars.remove(login);
    }

    fn avatar(&mut self, login: &str) -> Option<&mut Avatar> {
        let path = avatar::path_for(&self.avatar_dir, login);
        self.avatars
            .entry(login.to_string())
            .or_insert_with(|| {
                let image = ImageReader::open(path)
                    .ok()?
                    .with_guessed_format()
                    .ok()?
                    .decode()
                    .ok()?;
                Some(Avatar {
                    image,
                    row: None,
                    panel: None,
                })
            })
            .as_mut()
    }

    /// The 2 x 1 cell avatar for a row. Only with raster graphics.
    fn row_avatar(&mut self, login: &str) -> Option<&Protocol> {
        let picker = self.picker.clone()?;
        let avatar = self.avatar(login)?;
        if avatar.row.is_none() {
            avatar.row = picker
                .new_protocol(avatar.image.clone(), Size::new(2, 1), Resize::Fit(None))
                .ok();
        }
        avatar.row.as_ref()
    }

    /// The panel avatar: raster graphics, or else half-blocks.
    fn panel_avatar(&mut self, login: &str) -> Option<&Protocol> {
        let picker = self
            .picker
            .clone()
            .unwrap_or_else(|| self.halfblocks.clone());
        let avatar = self.avatar(login)?;
        if avatar.panel.is_none() {
            avatar.panel = picker
                .new_protocol(avatar.image.clone(), PANEL_AVATAR, Resize::Fit(None))
                .ok();
        }
        avatar.panel.as_ref()
    }

    pub fn description(&self) -> &'static str {
        match self.picker.as_ref().map(|p| p.protocol_type()) {
            Some(ProtocolType::Kitty) => "kitty",
            Some(ProtocolType::Sixel) => "sixel",
            Some(ProtocolType::Iterm2) => "iterm2",
            Some(ProtocolType::Halfblocks) | None => "none (ASCII)",
        }
    }
}

pub fn draw(
    frame: &mut Frame,
    app: &App,
    now: Timestamp,
    config: &ConfigView,
    graphics: &mut Graphics,
) {
    let [title, body, footer] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .areas(frame.area());

    draw_title(frame, title, app, now);
    let (list_width, panel_width) = panel_layout(body.width);
    let [list, panel] = Layout::horizontal([
        Constraint::Length(list_width),
        Constraint::Length(panel_width.unwrap_or(0)),
    ])
    .areas(body);
    draw_queue(frame, list, app, now, graphics);
    if panel_width.is_some() {
        draw_panel(frame, panel, app, now, graphics);
    }
    let keys = match &app.picker {
        Some(picker) if picker.input.is_some() => "type a duration · enter snooze · esc cancel",
        Some(_) => "1–9 choose · esc cancel",
        None if app.show_snoozed => {
            "j/k move · enter open · s change · u unsnooze · z/esc back · ? help · q quit"
        }
        None => {
            "j/k move · enter open · s snooze · d dismiss · z snoozed · r refresh · ? help · q quit"
        }
    };
    frame.render_widget(Paragraph::new(keys).dark_gray(), footer);

    if let Some((burst, t)) = app.burst_at(now) {
        let started_at = now - jiff::SignedDuration::from_secs_f32(t);
        draw_burst(frame, body, burst, t, started_at, graphics);
    }

    if let Some(picker) = &app.picker {
        draw_picker(frame, body, picker);
    } else if app.show_help {
        // The whole window, so that the config at the bottom has space.
        draw_help(frame, frame.area(), config);
    }
}

fn draw_burst(
    frame: &mut Frame,
    area: Rect,
    burst: &Burst,
    t: f32,
    started_at: Timestamp,
    graphics: &mut Graphics,
) {
    let index = (t * BURST_FPS) as usize;
    let frame_time = index as f32 / BURST_FPS;
    let Some(picker) = &graphics.picker else {
        for cell in render_ascii(burst.style, &burst.at(frame_time), area.width, area.height) {
            let [r, g, b] = cell.color;
            frame.buffer_mut().set_string(
                area.x + cell.col,
                area.y + cell.row,
                &cell.text,
                Style::new().fg(Color::Rgb(r, g, b)).bold(),
            );
        }
        return;
    };

    // The frames of another burst, or of another pane size, are not useful any more.
    let same = graphics.frames.as_ref().is_some_and(|f| {
        f.area == area
            && f.started_at.duration_since(started_at).abs() < jiff::SignedDuration::from_millis(5)
    });
    if !same {
        let count = (DURATION * BURST_FPS).ceil() as usize;
        graphics.frames = Some(BurstFrames {
            started_at,
            area,
            frames: (0..count).map(|_| None).collect(),
        });
    }
    let frames = graphics.frames.as_mut().expect("set above");
    let Some(slot) = frames.frames.get_mut(index) else {
        return;
    };
    if slot.is_none() {
        let font = picker.font_size();
        let canvas = render(
            burst.style,
            &burst.at(frame_time),
            area.width as u32 * font.width as u32,
            area.height as u32 * font.height as u32,
        );
        *slot = picker
            .new_protocol(
                DynamicImage::ImageRgba8(canvas),
                area.as_size(),
                Resize::Fit(None),
            )
            .ok();
    }
    if let Some(protocol) = slot {
        frame.render_widget(ratatui_image::Image::new(protocol), area);
    }
}

/// A local time as the user reads it, for example "Fri 15:30".
fn local_time(at: Timestamp) -> String {
    at.to_zoned(TimeZone::system())
        .strftime("%a %H:%M")
        .to_string()
}

fn centered(area: Rect, width: u16, height: u16) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);
    Rect {
        x: area.x + (area.width - width) / 2,
        y: area.y + (area.height - height) / 2,
        width,
        height,
    }
}

fn draw_picker(frame: &mut Frame, area: Rect, picker: &SnoozePicker) {
    let mut lines: Vec<Line> = Vec::new();
    match &picker.input {
        None => {
            for (number, choice) in picker.choices.iter().enumerate() {
                let until = choice
                    .until
                    .map(|at| format!("until {}", local_time(at)))
                    .unwrap_or_default();
                lines.push(Line::from(vec![
                    Span::styled(
                        format!(" {} ", number + 1),
                        Style::new().fg(Color::Cyan).bold(),
                    ),
                    Span::raw(format!("{:<16}", choice.label)),
                    Span::styled(until, Style::new().fg(Color::DarkGray)),
                ]));
            }
        }
        Some(input) => {
            lines.push(Line::raw(" Snooze for (for example 45m, 3h, 2d, 1h30m):"));
            lines.push(Line::from(vec![
                Span::raw(" > "),
                Span::styled(format!("{input}▏"), Style::new().bold()),
            ]));
            if let Some(error) = &picker.error {
                lines.push(Line::styled(
                    format!(" {error}"),
                    Style::new().fg(Color::Red),
                ));
            }
        }
    }
    let popup = centered(area, 50, lines.len() as u16 + 2);
    frame.render_widget(Clear, popup);
    frame.render_widget(
        Paragraph::new(lines).block(Block::new().borders(Borders::ALL).title(" Snooze ")),
        popup,
    );
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
    let title = if app.show_snoozed {
        " Review Queue · snoozed"
    } else {
        " Review Queue"
    };
    frame.render_widget(Paragraph::new(title).bold(), left);
    frame.render_widget(Paragraph::new(status).style(status_style), right);
}

fn draw_queue(frame: &mut Frame, area: Rect, app: &App, now: Timestamp, graphics: &mut Graphics) {
    let empty = if app.show_snoozed {
        app.snoozed.is_empty()
    } else {
        app.queue.is_empty()
    };
    if empty && app.updated_at.is_some() {
        let message = if app.show_snoozed {
            "Nothing snoozed"
        } else {
            "Nothing to review"
        };
        let [_, middle, _] = Layout::vertical([
            Constraint::Fill(1),
            Constraint::Length(1),
            Constraint::Fill(1),
        ])
        .areas(area);
        frame.render_widget(Paragraph::new(message).centered().bold(), middle);
        return;
    }
    let with_avatars = graphics.picker.is_some();
    let width = area.width as usize;
    let mut lines: Vec<ListItem> = Vec::new();
    let mut selected_line = None;
    let mut index = 0;
    // The list line of each row, and the login of its author.
    let mut avatar_lines: Vec<(usize, String)> = Vec::new();

    // Each row: the item, the text of the last column, and whether it is dimmed.
    let rows: Vec<(&QueueItem, String, bool)> = if app.show_snoozed {
        app.snoozed
            .iter()
            .map(|s| (&s.item, format!("→{}", local_time(s.until)), true))
            .collect()
    } else {
        app.queue
            .iter()
            .map(|i| (i, short_duration(i.requested_at, now), false))
            .collect()
    };

    for (tier, name) in TIERS {
        let in_tier: Vec<&(&QueueItem, String, bool)> =
            rows.iter().filter(|(i, _, _)| i.tier == tier).collect();
        lines.push(ListItem::new(tier_header(name, in_tier.len(), width)));
        for (item, last, dim) in in_tier {
            if index == app.selected {
                selected_line = Some(lines.len());
            }
            if with_avatars {
                avatar_lines.push((lines.len(), item.pull_request.author.login.clone()));
            }
            let mut line = row(item, last, width, with_avatars);
            if *dim {
                line = line.patch_style(Style::new().add_modifier(Modifier::DIM));
            }
            lines.push(ListItem::new(line));
            index += 1;
        }
    }

    let list = List::new(lines)
        .highlight_style(Style::new().add_modifier(Modifier::REVERSED))
        .highlight_symbol("▶ ");
    let mut state = ListState::default().with_selected(selected_line);
    frame.render_stateful_widget(list, area, &mut state);

    // The avatars go in the 2 cells after the highlight symbol, on the rows that show.
    for (line, login) in avatar_lines {
        let Some(offset) = line.checked_sub(state.offset()) else {
            continue;
        };
        if offset >= area.height as usize {
            continue;
        }
        if let Some(protocol) = graphics.row_avatar(&login) {
            let cell = Rect::new(area.x + 2, area.y + offset as u16, 2, 1);
            frame.render_widget(ratatui_image::Image::new(protocol), cell);
        }
    }
}

fn draw_panel(frame: &mut Frame, area: Rect, app: &App, now: Timestamp, graphics: &mut Graphics) {
    let block = Block::new()
        .borders(Borders::LEFT)
        .border_style(Style::new().fg(Color::DarkGray));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let Some(item) = app.selected_item() else {
        return;
    };
    let inner = Rect {
        x: inner.x + 1,
        width: inner.width.saturating_sub(2),
        ..inner
    };
    let pr = &item.pull_request;

    // The avatar, with the title at its right.
    let [avatar_area, title_area] = Layout::horizontal([
        Constraint::Length(PANEL_AVATAR.width + 1),
        Constraint::Min(0),
    ])
    .areas(Rect {
        height: PANEL_AVATAR.height.min(inner.height),
        ..inner
    });
    if let Some(protocol) = graphics.panel_avatar(&pr.author.login) {
        frame.render_widget(ratatui_image::Image::new(protocol), avatar_area);
    }
    let title: Vec<Line> = description_lines(
        &pr.title,
        title_area.width as usize,
        title_area.height as usize,
    )
    .into_iter()
    .map(|l| Line::styled(l, Style::new().bold()))
    .collect();
    frame.render_widget(Paragraph::new(title), title_area);

    // The fields, then the description.
    let now_local = now.to_zoned(TimeZone::system());
    let mut lines: Vec<Line> = vec![Line::raw("")];
    for (name, value) in detail_fields(item, app.selected_snoozed_until(), &now_local) {
        let mut spans = vec![Span::styled(
            format!("{name:<9}"),
            Style::new().fg(Color::Cyan),
        )];
        if name == "Labels" {
            for label in &pr.details.labels {
                let [r, g, b] = label.color;
                // Dark text on light labels, light text on dark labels.
                let light = (r as u32 * 299 + g as u32 * 587 + b as u32 * 114) / 1000 > 140;
                let fg = if light { Color::Black } else { Color::White };
                spans.push(Span::styled(
                    format!(" {} ", label.name),
                    Style::new().bg(Color::Rgb(r, g, b)).fg(fg),
                ));
                spans.push(Span::raw(" "));
            }
        } else {
            let style = match (name, pr.details.ci) {
                ("CI", CiStatus::Passing) => Style::new().fg(Color::Green),
                ("CI", CiStatus::Failing) => Style::new().fg(Color::Red),
                ("CI", CiStatus::Pending) => Style::new().fg(Color::Yellow),
                _ => Style::new(),
            };
            spans.push(Span::styled(value, style));
        }
        lines.push(Line::from(spans));
    }
    lines.push(Line::raw(""));
    lines.push(Line::styled("Description", Style::new().bold()));
    let rest = Rect {
        y: inner.y + PANEL_AVATAR.height,
        height: inner.height.saturating_sub(PANEL_AVATAR.height),
        ..inner
    };
    let room = (rest.height as usize).saturating_sub(lines.len());
    let description = description_lines(&pr.details.body, rest.width as usize, room);
    if description.is_empty() {
        lines.push(Line::styled("No description.", Style::new().dark_gray()));
    }
    lines.extend(description.into_iter().map(Line::raw));
    frame.render_widget(Paragraph::new(lines), rest);
}

fn tier_header(name: &str, count: usize, width: usize) -> Line<'static> {
    let label = format!("── {name} ({count}) ");
    let fill = "─".repeat(width.saturating_sub(label.chars().count() + 2));
    Line::from(format!("{label}{fill}")).style(Style::new().fg(Color::Cyan).bold())
}

/// `last` is the text of the last column: the Wait Time, or the end of a Snooze.
/// With `avatar_slot`, the row starts with 3 empty cells for the avatar and a space.
fn row(item: &QueueItem, last: &str, width: usize, avatar_slot: bool) -> Line<'static> {
    let slot = if avatar_slot { 3 } else { 0 };
    let width = width.saturating_sub(slot);
    let pr = &item.pull_request;
    let reference = format!("{}#{}", pr.repo, pr.number);
    let author = format!("@{}", pr.author.login);
    let size = format!("+{} −{}", pr.additions, pr.deletions);
    let last_width = last.chars().count().max(4);

    let mut tags = String::new();
    if pr.is_draft {
        tags.push_str(" [draft]");
    }
    if let Some(team) = &item.team {
        tags.push_str(&format!(" [@{team}]"));
    }

    // The reference and the last column always show. In a narrow terminal, the diff size
    // goes first, then the author. The title and tags get the remaining columns.
    // Each column width includes the space in front of it. The highlight symbol takes 2.
    const REFERENCE: usize = 28;
    const AUTHOR: usize = 1 + 18;
    const SIZE: usize = 1 + 14;
    const MIN_TITLE: usize = 12;
    let available = width.saturating_sub(2 + REFERENCE + 1 + last_width);
    let show_author = available >= MIN_TITLE + AUTHOR;
    let show_size = available >= MIN_TITLE + AUTHOR + SIZE;
    let flexible =
        available - if show_author { AUTHOR } else { 0 } - if show_size { SIZE } else { 0 };
    let tags = truncate(&tags, flexible.saturating_sub(1));
    let title_width = flexible.saturating_sub(1 + tags.chars().count());

    let mut spans = vec![
        Span::raw(" ".repeat(slot)),
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
    spans.push(Span::styled(
        format!(" {last:>last_width$}"),
        Style::new().bold(),
    ));
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
        (
            "s",
            "Snooze the selected pull request (in the snoozed view: change the time)",
        ),
        (
            "d",
            "Dismiss: snooze the selected pull request for 1 working day",
        ),
        ("z", "Show or close the snoozed pull requests"),
        (
            "u",
            "In the snoozed view: end the snooze of the selected pull request",
        ),
        ("r", "Refresh the queue now"),
        ("p", "Party: play a random particle burst"),
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
        Line::raw("  Submit your review to remove an item. Snooze hides an item until a time you choose;"),
        Line::raw("  a re-request or an @-mention of you ends the snooze early. Nothing is hidden forever."),
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
        Line::raw(format!("  working time   {}", config.working_time)),
        Line::raw(format!("  snooze file    {}", config.snooze_file)),
        Line::raw(format!("  token          {}", config.token_source)),
        Line::raw(format!("  graphics       {}", config.graphics)),
    ]);

    let popup = centered(area, 96, lines.len() as u16 + 2);
    frame.render_widget(Clear, popup);
    frame.render_widget(
        Paragraph::new(lines).block(Block::new().borders(Borders::ALL).title(" Help ")),
        popup,
    );
}
