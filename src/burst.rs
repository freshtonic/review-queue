//! The particle burst: a short animation when the Queue becomes empty.
//!
//! Positions are relative to the pane: (0, 0) is the top left and (1, 1) the bottom right.

mod sprites;

/// The look of all particles in one burst.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Style {
    Stars,
    Smilies,
    Hotdogs,
    Cats,
    Party,
    Facehuggers,
    ThumbsUp,
    Diamonds,
    FuckYeah,
}

impl Style {
    pub const ALL: [Style; 9] = [
        Style::Stars,
        Style::Smilies,
        Style::Hotdogs,
        Style::Cats,
        Style::Party,
        Style::Facehuggers,
        Style::ThumbsUp,
        Style::Diamonds,
        Style::FuckYeah,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Style::Stars => "stars",
            Style::Smilies => "smilies",
            Style::Hotdogs => "hotdogs",
            Style::Cats => "cats",
            Style::Party => "party",
            Style::Facehuggers => "facehuggers",
            Style::ThumbsUp => "thumbs-up",
            Style::Diamonds => "diamonds",
            Style::FuckYeah => "fuck yeah",
        }
    }

    /// Pixel art: `.` is transparent, `#` is the particle's color, and the other
    /// characters are fixed colors (see [`palette`]).
    pub fn sprite(self) -> &'static [&'static str] {
        match self {
            Style::Stars => sprites::STAR,
            Style::Smilies => sprites::SMILEY,
            Style::Hotdogs => sprites::HOTDOG,
            Style::Cats => sprites::CAT,
            Style::Party => sprites::PARTY,
            Style::Facehuggers => sprites::FACEHUGGER,
            Style::ThumbsUp => sprites::THUMBS_UP,
            Style::Diamonds => sprites::DIAMOND,
            Style::FuckYeah => sprites::FUCK_YEAH,
        }
    }

    /// The glyph for terminals without raster graphics.
    pub fn ascii(self) -> &'static str {
        match self {
            Style::Stars => "*",
            Style::Smilies => ":)",
            Style::Hotdogs => "(=)",
            Style::Cats => "^.^",
            Style::Party => "%",
            Style::Facehuggers => "}{",
            Style::ThumbsUp => "+1",
            Style::Diamonds => "<>",
            Style::FuckYeah => "FUCK YEAH",
        }
    }
}

/// The fixed colors of the sprites.
pub fn palette(c: char) -> Option<[u8; 3]> {
    match c {
        'k' => Some([20, 20, 20]),
        'w' => Some([245, 245, 245]),
        'p' => Some([255, 150, 180]),
        'b' => Some([222, 170, 100]),
        'r' => Some([190, 60, 50]),
        'o' => Some([255, 150, 40]),
        'y' => Some([255, 215, 0]),
        _ => None,
    }
}

/// One particle, as it was spawned.
#[derive(Debug, Clone, PartialEq)]
pub struct Particle {
    /// Pane units per second.
    pub velocity: (f32, f32),
    /// Radians.
    pub angle: f32,
    /// Radians per second.
    pub spin: f32,
    /// The height of the sprite, in pane units.
    pub size: f32,
    pub color: [u8; 3],
    /// The time (seconds) when the particle starts to fade.
    pub fade_start: f32,
    /// The time (seconds) when the particle is invisible.
    pub fade_end: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Burst {
    pub style: Style,
    pub particles: Vec<Particle>,
}

/// A particle at one time.
#[derive(Debug, Clone, PartialEq)]
pub struct ParticleState {
    pub position: (f32, f32),
    pub angle: f32,
    pub size: f32,
    pub color: [u8; 3],
    /// 0 is invisible, 1 is opaque.
    pub alpha: f32,
}

/// The length of a burst, in seconds.
pub const DURATION: f32 = 0.5;
/// The limits of the random values. Each one is chosen for each particle, independently.
pub const COUNT: std::ops::RangeInclusive<usize> = 24..=40;
pub const SPEED: std::ops::RangeInclusive<f32> = 0.6..=1.6;
pub const SPIN: std::ops::RangeInclusive<f32> = -12.0..=12.0;
pub const SIZE: std::ops::RangeInclusive<f32> = 0.05..=0.12;
/// The fade starts at a time in this range and ends at a later time in this range.
pub const FADE: std::ops::RangeInclusive<f32> = 0.15..=DURATION;

/// The downward acceleration of every particle, in pane units per second squared.
pub const GRAVITY: f32 = 4.0;

const CENTER: (f32, f32) = (0.5, 0.5);

pub fn spawn(style: Style, seed: u64) -> Burst {
    let mut random = SplitMix64(seed);
    let count = random.between_usize(COUNT);
    let particles = (0..count)
        .map(|_| {
            let direction = random.between(0.0..=std::f32::consts::TAU);
            let speed = random.between(SPEED);
            let fade_start = random.between(*FADE.start()..=FADE.end() - 0.05);
            Particle {
                velocity: (speed * direction.cos(), speed * direction.sin()),
                angle: random.between(0.0..=std::f32::consts::TAU),
                spin: random.between(SPIN),
                size: random.between(SIZE),
                color: [random.byte(), random.byte(), random.byte()],
                fade_start,
                fade_end: random.between(fade_start + 0.05..=*FADE.end()),
            }
        })
        .collect();
    Burst { style, particles }
}

impl Burst {
    /// The particles at `t` seconds after the start.
    pub fn at(&self, t: f32) -> Vec<ParticleState> {
        self.particles
            .iter()
            .map(|p| ParticleState {
                position: (
                    CENTER.0 + p.velocity.0 * t,
                    CENTER.1 + p.velocity.1 * t + GRAVITY * t * t / 2.0,
                ),
                angle: p.angle + p.spin * t,
                size: p.size,
                color: p.color,
                alpha: (1.0 - (t - p.fade_start) / (p.fade_end - p.fade_start)).clamp(0.0, 1.0),
            })
            .collect()
    }

    pub fn is_done(&self, t: f32) -> bool {
        t >= DURATION
    }
}

/// A small seeded random number generator, so that a burst is the same for the same seed.
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

    fn between(&mut self, range: std::ops::RangeInclusive<f32>) -> f32 {
        range.start() + self.unit() * (range.end() - range.start())
    }

    fn between_usize(&mut self, range: std::ops::RangeInclusive<usize>) -> usize {
        range.start() + (self.next() % (range.end() - range.start() + 1) as u64) as usize
    }

    fn byte(&mut self) -> u8 {
        (self.next() >> 56) as u8
    }
}

/// Draws the particles of a burst on a transparent canvas of `width` x `height` pixels.
///
/// A positive angle turns the sprite clockwise. Sprites use nearest-pixel sampling, so
/// they keep their hard pixel-art edges at every size and angle.
pub fn render(style: Style, states: &[ParticleState], width: u32, height: u32) -> image::RgbaImage {
    let mut canvas = image::RgbaImage::new(width, height);
    let rows: Vec<Vec<char>> = style.sprite().iter().map(|r| r.chars().collect()).collect();
    let (sprite_w, sprite_h) = (rows[0].len() as f32, rows.len() as f32);

    for state in states.iter().filter(|s| s.alpha > 0.0) {
        let scale = state.size * height as f32 / sprite_h;
        let (cx, cy) = (
            state.position.0 * width as f32,
            state.position.1 * height as f32,
        );
        let (sin, cos) = state.angle.sin_cos();
        // Every pixel of the turned sprite is inside this square.
        let reach = (sprite_w.hypot(sprite_h) / 2.0 * scale).ceil();
        let x_range =
            ((cx - reach).floor().max(0.0) as u32)..((cx + reach).ceil().min(width as f32) as u32);
        let y_range =
            ((cy - reach).floor().max(0.0) as u32)..((cy + reach).ceil().min(height as f32) as u32);

        for y in y_range {
            for x in x_range.clone() {
                let (dx, dy) = (x as f32 + 0.5 - cx, y as f32 + 0.5 - cy);
                // Turn the canvas point back by the angle to find the sprite pixel.
                let col = ((dx * cos + dy * sin) / scale + sprite_w / 2.0).floor();
                let row = ((-dx * sin + dy * cos) / scale + sprite_h / 2.0).floor();
                if col < 0.0 || row < 0.0 || col >= sprite_w || row >= sprite_h {
                    continue;
                }
                let color = match rows[row as usize][col as usize] {
                    '#' => state.color,
                    c => match palette(c) {
                        Some(color) => color,
                        None => continue,
                    },
                };
                blend(canvas.get_pixel_mut(x, y), color, state.alpha);
            }
        }
    }
    canvas
}

/// Paints `color` with `alpha` over the pixel.
fn blend(pixel: &mut image::Rgba<u8>, color: [u8; 3], alpha: f32) {
    let below = pixel.0[3] as f32 / 255.0;
    let out = alpha + below * (1.0 - alpha);
    for i in 0..3 {
        let mixed = color[i] as f32 * alpha + pixel.0[i] as f32 * below * (1.0 - alpha);
        pixel.0[i] = (mixed / out).round() as u8;
    }
    pixel.0[3] = (out * 255.0).round() as u8;
}
