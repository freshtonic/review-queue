use review_queue::attempt::{INK, RAINBOW, STAR_CENTER, STAR_YELLOW, render};

const SIZES: [(u32, u32); 3] = [(320, 160), (1280, 640), (600, 600)];

fn rgb(image: &image::RgbaImage, x: u32, y: u32) -> [u8; 3] {
    let p = image.get_pixel(x, y).0;
    [p[0], p[1], p[2]]
}

#[test]
fn the_image_has_the_size_of_the_pane_and_is_the_same_each_time() {
    for (w, h) in SIZES {
        let image = render(w, h);
        assert_eq!(image.dimensions(), (w, h));
        assert_eq!(image, render(w, h));
    }
}

#[test]
fn the_star_is_yellow_at_its_center_at_every_size() {
    for (w, h) in SIZES {
        let image = render(w, h);
        let (x, y) = (
            (STAR_CENTER.0 * w as f32) as u32,
            (STAR_CENTER.1 * h as f32) as u32,
        );
        assert_eq!(rgb(&image, x, y), STAR_YELLOW, "{w}x{h}");
    }
}

#[test]
fn the_rainbow_bands_at_the_left_are_in_order_from_red_to_violet() {
    for (w, h) in SIZES {
        let image = render(w, h);
        let x = (0.1 * w as f32) as u32;
        // The first row of each band color, from the top down this column.
        let first_rows: Vec<u32> = RAINBOW
            .iter()
            .map(|band| {
                (0..h)
                    .find(|&y| rgb(&image, x, y) == *band)
                    .unwrap_or_else(|| panic!("{w}x{h}: no {band:?} at x = {x}"))
            })
            .collect();
        let mut sorted = first_rows.clone();
        sorted.sort();
        assert_eq!(first_rows, sorted, "{w}x{h}");
    }
}

#[test]
fn hand_lettered_text_is_above_and_below_the_star_and_rainbow() {
    for (w, h) in SIZES {
        let image = render(w, h);
        let ink_in = |rows: std::ops::Range<f32>| {
            let rows = (rows.start * h as f32) as u32..(rows.end * h as f32) as u32;
            rows.flat_map(|y| (0..w).map(move |x| (x, y)))
                .filter(|&(x, y)| rgb(&image, x, y) == INK)
                .count()
        };
        assert!(ink_in(0.0..0.25) > 50, "{w}x{h}: no text at the top");
        assert!(ink_in(0.75..1.0) > 50, "{w}x{h}: no text at the bottom");
    }
}

#[test]
fn the_sky_is_dark() {
    for (w, h) in SIZES {
        let image = render(w, h);
        let corner = (w - 10..w).flat_map(|x| (0..10).map(move |y| (x, y)));
        let dark = corner
            .filter(|&(x, y)| rgb(&image, x, y).iter().all(|&c| c < 60))
            .count();
        assert!(dark >= 90, "{w}x{h}: {dark} of 100 corner pixels are dark");
    }
}

use review_queue::attempt::text_art;

fn plain(line: &[(String, [u8; 3])]) -> String {
    line.iter().map(|(text, _)| text.as_str()).collect()
}

#[test]
fn the_text_art_has_the_two_lines_of_text_around_the_rainbow_and_star() {
    let art = text_art(80, 24);

    let lines: Vec<String> = art.iter().map(|l| plain(l)).collect();
    let top = lines
        .iter()
        .position(|l| l.contains("there was an attempt"))
        .unwrap();
    let bottom = lines
        .iter()
        .position(|l| l.contains("to keep on top of PR reviews"))
        .unwrap();
    assert!(top < bottom);

    // The rainbow rows, from the top down, have the rainbow colors in order.
    let band_colors: Vec<[u8; 3]> = art
        .iter()
        .filter_map(|line| {
            line.iter()
                .find(|(text, _)| text.contains('~'))
                .map(|(_, c)| *c)
        })
        .collect();
    assert_eq!(band_colors, RAINBOW.to_vec());
    assert!(
        art.iter()
            .flatten()
            .any(|(text, color)| *color == STAR_YELLOW && !text.trim().is_empty())
    );
}

#[test]
fn the_text_art_fits_the_pane_or_is_left_out() {
    let art = text_art(80, 24);
    assert!(art.len() <= 24);
    assert!(art.iter().all(|l| plain(l).chars().count() <= 80));

    assert!(text_art(20, 24).is_empty());
    assert!(text_art(80, 5).is_empty());
}
