//! The empty-queue image, in the style of the "there was an attempt" meme: a crude star
//! with a wavy rainbow trail, and hand-lettered text. It is drawn from vector shapes in
//! pane-relative coordinates, so it fills the pane at any size. Every stroke wobbles and
//! changes width, from a fixed seed, so that it looks drawn by hand but is the same each time.

use image::{Rgba, RgbaImage};

/// The center of the star, relative to the image size.
pub const STAR_CENTER: (f32, f32) = (0.78, 0.5);
/// The rainbow bands, from the top band to the bottom band.
pub const RAINBOW: [[u8; 3]; 7] = [
    [235, 40, 40],
    [255, 140, 20],
    [250, 230, 30],
    [60, 200, 60],
    [40, 200, 230],
    [50, 90, 230],
    [150, 60, 220],
];
pub const STAR_YELLOW: [u8; 3] = [255, 232, 40];
/// The color of the hand-lettered text.
pub const INK: [u8; 3] = [250, 250, 250];

const SKY: [u8; 3] = [6, 6, 12];
const SKY_DOT: [u8; 3] = [220, 220, 220];
const STAR_OUTLINE: [u8; 3] = [235, 170, 20];
const TOP_TEXT: &str = "there was an attempt";
const BOTTOM_TEXT: &str = "to keep on top of PR reviews";

pub fn render(width: u32, height: u32) -> RgbaImage {
    let mut canvas = Canvas {
        image: RgbaImage::from_pixel(width, height, rgba(SKY)),
        w: width as f32,
        h: height as f32,
    };
    let unit = canvas.w.min(canvas.h);
    let mut random = SplitMix64(0x5EED_A77E);

    // Sky dots.
    for _ in 0..70 {
        let (x, y) = (random.unit() * canvas.w, random.unit() * canvas.h);
        canvas.disc(x, y, unit * (0.002 + random.unit() * 0.003), SKY_DOT);
    }

    let star = (STAR_CENTER.0 * canvas.w, STAR_CENTER.1 * canvas.h);
    let radius = (0.21 * canvas.h).min(0.18 * canvas.w);
    canvas.glow(star, radius * 1.8);

    // The rainbow: wavy bands from the left edge, rising to the star.
    let band = 0.045 * canvas.h;
    for (i, color) in RAINBOW.iter().enumerate() {
        let offset = (i as f32 - 3.0) * band * 1.08;
        let points: Vec<(f32, f32)> = (0..=60)
            .map(|step| {
                let t = step as f32 / 60.0;
                let x = -0.03 * canvas.w + t * (star.0 - 0.25 * radius + 0.03 * canvas.w);
                let rise = 0.58 + (STAR_CENTER.1 - 0.58) * t.powf(1.5);
                let wave = 0.022 * (t * 13.0).sin();
                (x, (rise + wave) * canvas.h + offset)
            })
            .collect();
        let mut pen = Pen::new(&mut random, band * 0.5, band * 0.3);
        pen.stroke(&mut canvas, &points, *color);
    }

    // The star: an uneven five-pointed polygon with a wobbly outline.
    let tilt = -0.15_f32;
    let corners: Vec<(f32, f32)> = (0..10)
        .map(|i| {
            let r = if i % 2 == 0 {
                radius * (0.85 + random.unit() * 0.3)
            } else {
                radius * (0.38 + random.unit() * 0.14)
            };
            let angle = tilt - std::f32::consts::FRAC_PI_2
                + i as f32 * std::f32::consts::PI / 5.0
                + (random.unit() - 0.5) * 0.16;
            (star.0 + r * angle.cos(), star.1 + r * angle.sin())
        })
        .collect();
    canvas.fill_polygon(&corners, STAR_YELLOW);
    let mut outline = corners.clone();
    outline.push(corners[0]);
    // Two passes with different wobble, like a sketch.
    for _ in 0..2 {
        let mut pen = Pen::new(&mut random, unit * 0.009, unit * 0.012);
        pen.stroke(&mut canvas, &outline, STAR_OUTLINE);
    }

    // The hand-lettered text.
    canvas.text(&mut random, TOP_TEXT, 0.13);
    canvas.text(&mut random, BOTTOM_TEXT, 0.87);
    canvas.image
}

fn rgba([r, g, b]: [u8; 3]) -> Rgba<u8> {
    Rgba([r, g, b, 255])
}

struct Canvas {
    image: RgbaImage,
    w: f32,
    h: f32,
}

impl Canvas {
    fn disc(&mut self, cx: f32, cy: f32, r: f32, color: [u8; 3]) {
        let r = r.max(0.5);
        let (x0, x1) = ((cx - r).floor().max(0.0), (cx + r).ceil().min(self.w));
        let (y0, y1) = ((cy - r).floor().max(0.0), (cy + r).ceil().min(self.h));
        for y in y0 as u32..y1 as u32 {
            for x in x0 as u32..x1 as u32 {
                let (dx, dy) = (x as f32 + 0.5 - cx, y as f32 + 0.5 - cy);
                if dx * dx + dy * dy <= r * r {
                    self.image.put_pixel(x, y, rgba(color));
                }
            }
        }
    }

    /// A soft yellow light around the star, over the sky.
    fn glow(&mut self, (cx, cy): (f32, f32), r: f32) {
        let (x0, x1) = ((cx - r).max(0.0) as u32, (cx + r).min(self.w) as u32);
        let (y0, y1) = ((cy - r).max(0.0) as u32, (cy + r).min(self.h) as u32);
        for y in y0..y1 {
            for x in x0..x1 {
                let d = ((x as f32 - cx).powi(2) + (y as f32 - cy).powi(2)).sqrt() / r;
                if d < 1.0 {
                    let a = (1.0 - d).powi(2) * 0.45;
                    let p = self.image.get_pixel_mut(x, y);
                    for (c, glow) in p.0.iter_mut().zip([255.0, 220.0, 60.0]) {
                        *c = (*c as f32 * (1.0 - a) + glow * a) as u8;
                    }
                }
            }
        }
    }

    fn fill_polygon(&mut self, corners: &[(f32, f32)], color: [u8; 3]) {
        let xs = corners.iter().map(|c| c.0);
        let ys = corners.iter().map(|c| c.1);
        let (x0, x1) = (
            xs.clone().fold(f32::MAX, f32::min),
            xs.fold(f32::MIN, f32::max),
        );
        let (y0, y1) = (
            ys.clone().fold(f32::MAX, f32::min),
            ys.fold(f32::MIN, f32::max),
        );
        for y in y0.max(0.0) as u32..(y1.min(self.h - 1.0) as u32 + 1) {
            for x in x0.max(0.0) as u32..(x1.min(self.w - 1.0) as u32 + 1) {
                if inside(x as f32 + 0.5, y as f32 + 0.5, corners) {
                    self.image.put_pixel(x, y, rgba(color));
                }
            }
        }
    }

    /// One line of hand-lettered text, centered, with its middle at `y` (relative).
    fn text(&mut self, random: &mut SplitMix64, text: &str, y: f32) {
        let ems: f32 = text.chars().map(|c| glyph(c).0 + SPACING).sum::<f32>() - SPACING;
        let em = (0.13 * self.h).min(0.9 * self.w / ems);
        let mut x = (self.w - ems * em) / 2.0;
        let top = y * self.h - em * 0.6;
        for c in text.chars() {
            let (advance, strokes) = glyph(c);
            // Each letter is a little bigger or smaller, sits higher or lower, and leans.
            let size = em * (0.9 + random.unit() * 0.2);
            let lift = (random.unit() - 0.5) * 0.15 * em;
            let lean = (random.unit() - 0.5) * 0.2;
            for stroke in strokes {
                let points: Vec<(f32, f32)> = stroke
                    .iter()
                    .map(|&(gx, gy)| {
                        let gy = 1.0 - (1.0 - gy) * size / em;
                        (x + (gx + lean * (1.0 - gy)) * em, top + gy * em + lift)
                    })
                    .collect();
                let mut pen = Pen::new(random, em * 0.07, em * 0.05);
                pen.stroke(self, &points, INK);
            }
            x += (advance + SPACING) * em;
        }
    }
}

/// True when the point is inside the polygon (even-odd rule).
fn inside(px: f32, py: f32, corners: &[(f32, f32)]) -> bool {
    let mut result = false;
    for i in 0..corners.len() {
        let (x1, y1) = corners[i];
        let (x2, y2) = corners[(i + corners.len() - 1) % corners.len()];
        if (y1 > py) != (y2 > py) && px < (x2 - x1) * (py - y1) / (y2 - y1) + x1 {
            result = !result;
        }
    }
    result
}

/// A marker that wobbles off its path and changes width as it goes.
struct Pen {
    radius: f32,
    wobble: f32,
    phases: [f32; 4],
}

impl Pen {
    fn new(random: &mut SplitMix64, radius: f32, wobble: f32) -> Self {
        Self {
            radius,
            wobble,
            phases: [0.0; 4].map(|_| random.unit() * std::f32::consts::TAU),
        }
    }

    fn stroke(&mut self, canvas: &mut Canvas, points: &[(f32, f32)], color: [u8; 3]) {
        let step = (self.radius * 0.4).max(0.5);
        let mut travelled = 0.0;
        let scale = self.radius * 12.0;
        let stamp = |canvas: &mut Canvas, x: f32, y: f32, nx: f32, ny: f32, s: f32| {
            let p = self.phases;
            let off =
                self.wobble * ((s / scale + p[0]).sin() + 0.5 * (s / (scale * 0.37) + p[1]).sin());
            let width = self.radius
                * (1.0
                    + 0.22 * (s / (scale * 0.6) + p[2]).sin()
                    + 0.12 * (s / (scale * 0.23) + p[3]).sin());
            canvas.disc(x + nx * off, y + ny * off, width, color);
        };
        if points.len() == 1 {
            stamp(canvas, points[0].0, points[0].1, 0.0, 0.0, 0.0);
        }
        for pair in points.windows(2) {
            let ((x1, y1), (x2, y2)) = (pair[0], pair[1]);
            let length = (x2 - x1).hypot(y2 - y1);
            let (nx, ny) = if length > 0.0 {
                (-(y2 - y1) / length, (x2 - x1) / length)
            } else {
                (0.0, 0.0)
            };
            let steps = (length / step).ceil().max(1.0) as usize;
            for i in 0..=steps {
                let t = i as f32 / steps as f32;
                stamp(
                    canvas,
                    x1 + (x2 - x1) * t,
                    y1 + (y2 - y1) * t,
                    nx,
                    ny,
                    travelled + length * t,
                );
            }
            travelled += length;
        }
    }
}

/// The space between letters, in ems.
const SPACING: f32 = 0.12;

/// A marker-pen letter: its advance width (in ems), and its strokes. The cap height is 0
/// to 1, the x-height starts at 0.4, and descenders go to 1.35.
fn glyph(c: char) -> (f32, Vec<Vec<(f32, f32)>>) {
    let line = |a: (f32, f32), b: (f32, f32)| vec![a, b];
    match c {
        'a' => (
            0.6,
            vec![
                ellipse((0.28, 0.71), (0.25, 0.29), 0.0, 360.0),
                line((0.55, 0.42), (0.57, 1.0)),
            ],
        ),
        'e' => (
            0.58,
            vec![
                [
                    vec![(0.04, 0.7), (0.54, 0.7)],
                    arc((0.29, 0.7), (0.26, 0.3), 0.0, 305.0),
                ]
                .concat(),
            ],
        ),
        'f' => (
            0.45,
            vec![
                arc((0.42, 0.2), (0.2, 0.18), 30.0, 180.0)
                    .into_iter()
                    .chain([(0.22, 1.0)])
                    .collect(),
                line((0.02, 0.45), (0.45, 0.45)),
            ],
        ),
        'h' => (0.6, vec![line((0.06, 0.0), (0.06, 1.0)), hump(0.06, 0.55)]),
        'i' => (
            0.22,
            vec![
                line((0.1, 0.42), (0.1, 1.0)),
                line((0.1, 0.18), (0.11, 0.21)),
            ],
        ),
        'k' => (
            0.55,
            vec![
                line((0.06, 0.0), (0.06, 1.0)),
                line((0.48, 0.4), (0.06, 0.76)),
                line((0.2, 0.64), (0.52, 1.0)),
            ],
        ),
        'm' => (
            0.82,
            vec![
                line((0.06, 0.4), (0.06, 1.0)),
                hump(0.06, 0.42),
                hump(0.42, 0.78),
            ],
        ),
        'n' => (0.6, vec![line((0.06, 0.4), (0.06, 1.0)), hump(0.06, 0.55)]),
        'o' => (0.6, vec![ellipse((0.3, 0.7), (0.27, 0.3), 90.0, 450.0)]),
        'p' => (
            0.6,
            vec![
                line((0.06, 0.4), (0.06, 1.35)),
                ellipse((0.31, 0.7), (0.25, 0.29), 180.0, 540.0),
            ],
        ),
        'r' => (
            0.45,
            vec![
                line((0.06, 0.4), (0.06, 1.0)),
                arc((0.3, 0.66), (0.24, 0.22), 180.0, 45.0),
            ],
        ),
        's' => (
            0.52,
            vec![vec![
                (0.48, 0.48),
                (0.34, 0.41),
                (0.14, 0.43),
                (0.07, 0.56),
                (0.2, 0.68),
                (0.4, 0.74),
                (0.5, 0.86),
                (0.43, 0.98),
                (0.22, 1.0),
                (0.04, 0.92),
            ]],
        ),
        't' => (
            0.45,
            vec![
                vec![(0.18, 0.1), (0.18, 0.9), (0.26, 1.0), (0.42, 0.98)],
                line((0.0, 0.42), (0.42, 0.42)),
            ],
        ),
        'v' => (0.58, vec![vec![(0.02, 0.4), (0.29, 1.0), (0.56, 0.4)]]),
        'w' => (
            0.8,
            vec![vec![
                (0.02, 0.4),
                (0.2, 1.0),
                (0.4, 0.55),
                (0.6, 1.0),
                (0.78, 0.4),
            ]],
        ),
        'P' => (
            0.6,
            vec![
                line((0.06, 0.0), (0.06, 1.0)),
                [
                    vec![(0.06, 0.02)],
                    arc((0.3, 0.27), (0.26, 0.25), 90.0, -90.0),
                    vec![(0.06, 0.52)],
                ]
                .concat(),
            ],
        ),
        'R' => (
            0.62,
            vec![
                line((0.06, 0.0), (0.06, 1.0)),
                [
                    vec![(0.06, 0.02)],
                    arc((0.3, 0.27), (0.26, 0.25), 90.0, -90.0),
                    vec![(0.06, 0.52)],
                ]
                .concat(),
                line((0.26, 0.52), (0.6, 1.0)),
            ],
        ),
        _ => (0.4, Vec::new()),
    }
}

/// Points on an elliptical arc, from angle `a0` to `a1` in degrees (counterclockwise, as
/// on paper: 90 is up).
fn arc((cx, cy): (f32, f32), (rx, ry): (f32, f32), a0: f32, a1: f32) -> Vec<(f32, f32)> {
    (0..=24)
        .map(|i| {
            let a = (a0 + (a1 - a0) * i as f32 / 24.0).to_radians();
            (cx + rx * a.cos(), cy - ry * a.sin())
        })
        .collect()
}

fn ellipse(center: (f32, f32), radii: (f32, f32), a0: f32, a1: f32) -> Vec<(f32, f32)> {
    arc(center, radii, a0, a1)
}

/// The arch of an h, m, or n: from the stem at `left` over to a stem at `right`.
fn hump(left: f32, right: f32) -> Vec<(f32, f32)> {
    let r = (right - left) / 2.0;
    let mut points = arc((left + r, 0.62), (r, 0.2), 180.0, 0.0);
    points.push((right, 1.0));
    points
}

/// A small seeded random number generator.
struct SplitMix64(u64);

impl SplitMix64 {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A value from 0 to 1.
    fn unit(&mut self) -> f32 {
        (self.next() >> 40) as f32 / (1u64 << 24) as f32
    }
}
