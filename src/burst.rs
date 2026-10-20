//! The particle burst: a short animation when the Queue becomes empty.
//!
//! Positions are relative to the pane: (0, 0) is the top left and (1, 1) the bottom right.

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
                position: (CENTER.0 + p.velocity.0 * t, CENTER.1 + p.velocity.1 * t),
                angle: p.angle,
                size: p.size,
                color: p.color,
                alpha: 1.0,
            })
            .collect()
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
