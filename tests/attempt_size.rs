//! The empty-queue image must work in a large pane. herdr does not show a Kitty image
//! above a size limit, so the image has a maximum size and is centered in the pane.

use jiff::{Timestamp, Zoned};
use ratatui::{Terminal, backend::TestBackend};
use ratatui_image::FontSize;
use ratatui_image::picker::{Picker, ProtocolType};
use review_queue::app::{App, Event};
use review_queue::ui::{self, ConfigView, Graphics};

/// The largest image, on each side, that the tool sends.
const MAX_SIDE: u32 = 1024;

/// What a draw of the empty-queue view sent to the terminal.
struct Sent {
    /// The size of each image, in pixels.
    images: Vec<(u32, u32)>,
    /// The cells (x, y) that have an image placeholder.
    placed: Vec<(u16, u16)>,
}

/// Draws the empty-queue view with Kitty graphics.
fn draw_empty_queue(cols: u16, rows: u16, font: (u16, u16)) -> Sent {
    // Deprecated for apps, but the only way to get a fixed font size without a terminal.
    #[allow(deprecated)]
    let mut picker = Picker::from_fontsize(FontSize {
        width: font.0,
        height: font.1,
    });
    picker.set_protocol_type(ProtocolType::Kitty);
    let mut graphics = Graphics::new(Some(picker), std::env::temp_dir());
    let mut app = App::default();
    let now = Zoned::now();
    app.handle(
        Event::PollSucceeded {
            queue: vec![],
            at: now.timestamp(),
            incomplete: false,
        },
        &now,
    );
    let view = ConfigView {
        path: String::new(),
        interval_secs: 60,
        watched_teams: vec![],
        working_time: String::new(),
        aging: String::new(),
        excluded: String::new(),
        snooze_file: String::new(),
        token_source: "",
        graphics: "kitty",
    };
    let mut terminal = Terminal::new(TestBackend::new(cols, rows)).unwrap();
    terminal
        .draw(|f| {
            ui::draw(f, &app, Timestamp::now(), &view, &mut graphics);
        })
        .unwrap();

    let buffer = terminal.backend().buffer();
    let mut images = Vec::new();
    for cell in buffer.content() {
        let symbol = cell.symbol();
        // The first chunk of each upload has the image size: "s=<width>,v=<height>".
        for upload in symbol.split("_G").skip(1) {
            let field = |name: &str| {
                upload
                    .split([',', ';'])
                    .find_map(|kv| kv.strip_prefix(name))
                    .and_then(|v| v.parse::<u32>().ok())
            };
            if let (Some(w), Some(h)) = (field("s="), field("v=")) {
                images.push((w, h));
            }
        }
    }
    let placed = (0..rows)
        .flat_map(|y| (0..cols).map(move |x| (x, y)))
        .filter(|&(x, y)| buffer[(x, y)].symbol().contains('\u{10EEEE}'))
        .collect();
    Sent { images, placed }
}

#[test]
fn no_image_is_larger_than_the_limit_and_the_image_is_a_centered_rectangle() {
    // 144 x 80 cells of 16 x 32 pixels is the size where the image disappeared in herdr.
    for (cols, rows, font) in [(144, 80, (16, 32)), (80, 24, (8, 16)), (320, 100, (8, 16))] {
        let Sent { images, placed } = draw_empty_queue(cols, rows, font);
        assert!(!images.is_empty(), "{cols}x{rows}: no image was sent");
        for (w, h) in &images {
            assert!(
                *w <= MAX_SIDE && *h <= MAX_SIDE,
                "{cols}x{rows} at {font:?}: an image of {w}x{h} pixels was sent"
            );
        }

        // The placeholders fill a rectangle with the same margin on each side (within a
        // cell), in the body: every row except the title and the footer.
        let (x0, x1) = (
            placed.iter().map(|p| p.0).min().unwrap(),
            placed.iter().map(|p| p.0).max().unwrap(),
        );
        let (y0, y1) = (
            placed.iter().map(|p| p.1).min().unwrap(),
            placed.iter().map(|p| p.1).max().unwrap(),
        );
        assert_eq!(
            placed.len(),
            ((x1 - x0 + 1) as usize) * ((y1 - y0 + 1) as usize),
            "{cols}x{rows}: not a rectangle"
        );
        let (left, right) = (x0, cols - 1 - x1);
        let (top, bottom) = (y0 - 1, rows - 2 - y1);
        assert!(
            left.abs_diff(right) <= 1,
            "{cols}x{rows}: margins {left} and {right}"
        );
        assert!(
            top.abs_diff(bottom) <= 1,
            "{cols}x{rows}: margins {top} and {bottom}"
        );
    }
}
