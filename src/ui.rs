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

use crate::app::{App, Picker as SnoozePicker, arrival_color, short_duration};
use crate::details::{description_lines, detail_fields, panel_layout};
use crate::queue::{CiStatus, PriorityTier, QueueItem};
use crate::queue::{Lateness, lateness};
use crate::{attempt, avatar};

/// The current config, as the help page shows it.
pub struct ConfigView {
    pub path: String,
    pub interval_secs: u64,
    pub watched_teams: Vec<String>,
    /// The working hours and days, for example "09:00–17:30, Mon Tue Wed Thu Fri".
    pub working_time: String,
    /// The aging thresholds, for example "stale after 4h, overdue after 8h (working time)".
    pub aging: String,
    pub snooze_file: String,
    pub token_source: &'static str,
    /// The raster graphics protocol, or "none (ASCII)".
    pub graphics: &'static str,
}

/// The color of a row that just arrived. It fades to the background in 1 second.
const ARRIVAL_HIGHLIGHT: [u8; 3] = [250, 210, 60];

const TIERS: [(PriorityTier, &str); 4] = [
    (PriorityTier::PersonalReReview, "Personal re-review"),
    (PriorityTier::PersonalFirstReview, "Personal first review"),
    (PriorityTier::TeamReReview, "Team re-review"),
    (PriorityTier::TeamFirstReview, "Team first review"),
];

/// The terminal's raster graphics, and the images that were made for it.
pub struct Graphics {
    /// `None` when the terminal has no raster graphics, or with `--no-images`.
    picker: Option<Picker>,
    /// For the panel avatar when there are no raster graphics.
    halfblocks: Picker,
    avatar_dir: PathBuf,
    /// The decoded avatars and their protocols. `None` when the file cannot be read.
    avatars: HashMap<String, Option<Avatar>>,
    /// The empty-queue image, for the size (in cells) that it was made for.
    attempt: Option<(Size, Protocol)>,
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

impl Graphics {
    pub fn new(picker: Option<Picker>, avatar_dir: PathBuf) -> Self {
        Self {
            picker,
            halfblocks: Picker::halfblocks(),
            avatar_dir,
            avatars: HashMap::new(),
            attempt: None,
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
            Some(ProtocolType::Halfblocks) | None => "none (text only)",
        }
    }
}

/// What the shell must give back to the App after a draw.
#[derive(Debug, Default, Clone, Copy)]
pub struct Drawn {
    pub panel_shown: bool,
    /// The largest description scroll: the lines that do not fit.
    pub max_scroll: usize,
}

pub fn draw(
    frame: &mut Frame,
    app: &App,
    now: Timestamp,
    config: &ConfigView,
    graphics: &mut Graphics,
) -> Drawn {
    let mut drawn = Drawn::default();
    let [title, body, footer] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .areas(frame.area());

    draw_title(frame, title, app, now);
    // With an empty queue, the attempt image uses the whole body and there is no panel.
    let attempt =
        app.show_attempt || (!app.show_snoozed && app.queue.is_empty() && app.updated_at.is_some());
    let (list_height, panel_height) = match attempt {
        true => (body.height, None),
        false => panel_layout(body.height, app.panel_percent()),
    };
    let [list, panel] = Layout::vertical([
        Constraint::Length(list_height),
        Constraint::Length(panel_height.unwrap_or(0)),
    ])
    .areas(body);
    if attempt {
        draw_attempt(frame, list, graphics);
    } else {
        draw_queue(frame, list, app, now, graphics);
    }
    if panel_height.is_some() {
        drawn = Drawn {
            panel_shown: true,
            max_scroll: draw_panel(frame, panel, app, now, graphics),
        };
    }
    let keys = match &app.picker {
        Some(picker) if picker.input.is_some() => "type a duration · enter snooze · esc cancel",
        Some(_) => "1–9 choose · esc cancel",
        None if app.show_attempt => "a/esc close the preview · q quit",
        None if app.panel_focused => {
            "j/k scroll · pgdn/pgup page · tab/esc back to the list · q quit"
        }
        None if app.show_snoozed => {
            "j/k move · enter open · s change · u unsnooze · z/esc back · ? help · q quit"
        }
        None => {
            "j/k move · enter open · s snooze · d dismiss · z snoozed · r refresh · ? help · q quit"
        }
    };
    frame.render_widget(Paragraph::new(keys).dark_gray(), footer);

    if let Some(picker) = &app.picker {
        draw_picker(frame, body, picker);
    } else if app.show_help {
        // The whole window, so that the config at the bottom has space.
        draw_help(frame, frame.area(), config);
    }
    drawn
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

/// The largest empty-queue image, in pixels. herdr does not show a Kitty image above a
/// size limit (2304 x 2496 pixels did not show), and a very large image does not look
/// better, so the image stops growing at this size and is centered in the pane.
const MAX_ATTEMPT: (u32, u32) = (1024, 640);

/// The "there was an attempt" image at the size of the area, or its text art, or else a
/// short message.
fn draw_attempt(frame: &mut Frame, area: Rect, graphics: &mut Graphics) {
    if let Some(picker) = &graphics.picker {
        // At most MAX_ATTEMPT pixels, in whole cells, centered in the area.
        let font = picker.font_size();
        let (font_w, font_h) = (font.width.max(1), font.height.max(1));
        let size = Size::new(
            area.width.min((MAX_ATTEMPT.0 / font_w as u32) as u16),
            area.height.min((MAX_ATTEMPT.1 / font_h as u32) as u16),
        );
        let place = centered(area, size.width, size.height);
        if graphics
            .attempt
            .as_ref()
            .is_none_or(|(made_for, _)| *made_for != size)
        {
            let image = attempt::render(
                size.width as u32 * font_w as u32,
                size.height as u32 * font_h as u32,
            );
            graphics.attempt = picker
                .new_protocol(DynamicImage::ImageRgba8(image), size, Resize::Fit(None))
                .ok()
                .map(|protocol| (size, protocol));
        }
        if let Some((_, protocol)) = &graphics.attempt {
            frame.render_widget(ratatui_image::Image::new(protocol), place);
            return;
        }
    }
    let art = attempt::text_art(area.width, area.height);
    if art.is_empty() {
        let [_, middle, _] = Layout::vertical([
            Constraint::Fill(1),
            Constraint::Length(1),
            Constraint::Fill(1),
        ])
        .areas(area);
        frame.render_widget(
            Paragraph::new("Nothing to review").centered().bold(),
            middle,
        );
        return;
    }
    let lines: Vec<Line> = art
        .into_iter()
        .map(|pieces| {
            Line::from(
                pieces
                    .into_iter()
                    .map(|(text, [r, g, b])| {
                        Span::styled(text, Style::new().fg(Color::Rgb(r, g, b)).bold())
                    })
                    .collect::<Vec<_>>(),
            )
        })
        .collect();
    let block = centered(area, lines[0].width() as u16, lines.len() as u16);
    frame.render_widget(Paragraph::new(lines), block);
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
    let arrivals = app.arrival_progress(now);
    // The list line of each row, and the login of its author.
    let mut avatar_lines: Vec<(usize, String)> = Vec::new();

    // Each row: the item, the text of the last column, its color, and whether the row is
    // dimmed. The Wait Time is amber when the request is Stale, and red when Overdue.
    let zoned_now = now.to_zoned(jiff::tz::TimeZone::system());
    let rows: Vec<(&QueueItem, String, Option<Color>, bool)> = if app.show_snoozed {
        app.snoozed
            .iter()
            .map(|s| (&s.item, format!("→{}", local_time(s.until)), None, true))
            .collect()
    } else {
        app.queue
            .iter()
            .map(|i| {
                let color =
                    match lateness(i.requested_at, &zoned_now, &app.working, &app.thresholds) {
                        Lateness::Fresh => None,
                        Lateness::Stale => Some(Color::Rgb(255, 176, 0)),
                        Lateness::Overdue => Some(Color::Rgb(240, 60, 60)),
                    };
                (i, short_duration(i.requested_at, now), color, false)
            })
            .collect()
    };

    for (tier, name) in TIERS {
        let in_tier: Vec<&(&QueueItem, String, Option<Color>, bool)> =
            rows.iter().filter(|(i, ..)| i.tier == tier).collect();
        lines.push(ListItem::new(tier_header(name, in_tier.len(), width)));
        for (item, last, last_color, dim) in in_tier {
            if index == app.selected {
                selected_line = Some(lines.len());
            }
            if with_avatars {
                avatar_lines.push((lines.len(), item.pull_request.author.login.clone()));
            }
            let mut line = row(item, last, *last_color, width, with_avatars);
            if *dim {
                line = line.patch_style(Style::new().add_modifier(Modifier::DIM));
            }
            if let Some((_, progress)) = arrivals.iter().find(|((repo, number), _)| {
                *repo == item.pull_request.repo && *number == item.pull_request.number
            }) {
                // A bright row that fades to the background. Dark text while it is bright.
                let [r, g, b] = arrival_color(*progress, ARRIVAL_HIGHLIGHT, [0, 0, 0]);
                let mut style = Style::new().bg(Color::Rgb(r, g, b));
                if *progress < 0.4 {
                    style = style.fg(Color::Black);
                }
                line = line.patch_style(style);
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

/// Draws the panel, and returns the largest description scroll.
fn draw_panel(
    frame: &mut Frame,
    area: Rect,
    app: &App,
    now: Timestamp,
    graphics: &mut Graphics,
) -> usize {
    let border = if app.panel_focused {
        Style::new().fg(Color::Cyan).bold()
    } else {
        Style::new().fg(Color::DarkGray)
    };
    let block = Block::new()
        .borders(Borders::TOP)
        .border_style(border)
        .title(if app.panel_focused {
            " Details · tab to go back "
        } else {
            " Details · tab to scroll "
        });
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let Some(item) = app.selected_item() else {
        return 0;
    };
    let inner = Rect {
        x: inner.x + 1,
        width: inner.width.saturating_sub(2),
        ..inner
    };
    let pr = &item.pull_request;

    // The summary (avatar, title, fields) at the left, and the description at the right.
    // A narrow panel has the description below the summary.
    const SUMMARY_WIDTH: u16 = 64;
    let side_by_side = inner.width >= 100;
    let (summary, description) = if side_by_side {
        let [summary, _, description] = Layout::horizontal([
            Constraint::Length(SUMMARY_WIDTH),
            Constraint::Length(2),
            Constraint::Min(0),
        ])
        .areas(inner);
        (summary, description)
    } else {
        (inner, Rect::default())
    };

    // The avatar, with the title at its right.
    let [avatar_area, title_area] = Layout::horizontal([
        Constraint::Length(PANEL_AVATAR.width + 1),
        Constraint::Min(0),
    ])
    .areas(Rect {
        height: PANEL_AVATAR.height.min(summary.height),
        ..summary
    });
    if let Some(protocol) = graphics.panel_avatar(&pr.author.login) {
        frame.render_widget(ratatui_image::Image::new(protocol), avatar_area);
    }

    // The title, then the fields, in the space at the right of the avatar and below it.
    let now_local = now.to_zoned(TimeZone::system());
    let mut lines: Vec<Line> = description_lines(&pr.title, title_area.width as usize, 2)
        .into_iter()
        .map(|l| Line::styled(l, Style::new().bold()))
        .collect();
    for (name, value) in detail_fields(item, app.selected_snoozed_until(), &now_local) {
        lines.push(field_line(name, value, item));
    }
    let fields_area = Rect {
        height: summary.height,
        ..title_area
    };
    let mut description_area = description;
    if !side_by_side {
        let used = lines.len() as u16 + 1;
        description_area = Rect {
            y: summary.y + used.max(PANEL_AVATAR.height),
            height: summary.height.saturating_sub(used.max(PANEL_AVATAR.height)),
            ..summary
        };
    }
    frame.render_widget(Paragraph::new(lines), fields_area);

    let room = (description_area.height as usize).saturating_sub(1);
    let body = description_lines(
        &pr.details.body,
        description_area.width as usize,
        usize::MAX,
    );
    let max_scroll = body.len().saturating_sub(room);
    let scroll = app.description_scroll.min(max_scroll);
    let mut heading = vec![Span::styled("Description", Style::new().bold())];
    if max_scroll > 0 {
        heading.push(Span::styled(
            format!(
                "  lines {}–{} of {}",
                scroll + 1,
                (scroll + room).min(body.len()),
                body.len()
            ),
            Style::new().dark_gray(),
        ));
    }
    let mut text = vec![Line::from(heading)];
    if body.is_empty() {
        text.push(Line::styled("No description.", Style::new().dark_gray()));
    }
    text.extend(body.into_iter().skip(scroll).take(room).map(Line::raw));
    frame.render_widget(Paragraph::new(text), description_area);
    max_scroll
}

/// One row of the panel: the name in a fixed column, then the value. Labels show as
/// colored chips, and the CI status has the color of its state.
fn field_line(name: &'static str, value: String, item: &QueueItem) -> Line<'static> {
    let pr = &item.pull_request;
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
    Line::from(spans)
}

fn tier_header(name: &str, count: usize, width: usize) -> Line<'static> {
    let label = format!("── {name} ({count}) ");
    let fill = "─".repeat(width.saturating_sub(label.chars().count() + 2));
    Line::from(format!("{label}{fill}")).style(Style::new().fg(Color::Cyan).bold())
}

/// With `avatar_slot`, the row starts with 3 empty cells for the avatar and a space.
fn row(
    item: &QueueItem,
    last: &str,
    last_color: Option<Color>,
    width: usize,
    avatar_slot: bool,
) -> Line<'static> {
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
    let mut last_style = Style::new().bold();
    if let Some(color) = last_color {
        last_style = last_style.fg(color);
    }
    spans.push(Span::styled(format!(" {last:>last_width$}"), last_style));
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
        (
            "enter / o",
            "Open the pull request (a re-review: the changes since your review)",
        ),
        ("O", "Open the whole pull request"),
        ("y", "Copy the link that enter opens (OSC 52)"),
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
        (
            "tab",
            "Focus the detail panel (j/k, pgdn/pgup, space scroll), or go back",
        ),
        ("+ / -", "Make the detail panel taller or shorter"),
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
        Line::raw(format!("  aging          {}", config.aging)),
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
