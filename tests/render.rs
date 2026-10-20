use image::Rgba;
use review_queue::burst::{ParticleState, Style, render};

const RED: [u8; 3] = [255, 0, 0];

/// A particle whose sprite pixels land on whole canvas pixels: a 100 x 100 canvas, and a
/// size that gives 1 canvas pixel for each sprite pixel of the 11 x 11 star.
fn star_at_center(angle: f32, alpha: f32) -> ParticleState {
    ParticleState {
        position: (0.505, 0.505),
        angle,
        size: 0.11,
        color: RED,
        alpha,
    }
}

fn expected(sprite_pixel: char) -> Rgba<u8> {
    match sprite_pixel {
        '#' => Rgba([255, 0, 0, 255]),
        _ => Rgba([0, 0, 0, 0]),
    }
}

#[test]
fn a_particle_paints_its_sprite_in_its_color_around_its_position() {
    let canvas = render(Style::Stars, &[star_at_center(0.0, 1.0)], 100, 100);

    for (r, row) in Style::Stars.sprite().iter().enumerate() {
        for (c, pixel) in row.chars().enumerate() {
            let (x, y) = (45 + c as u32, 45 + r as u32);
            assert_eq!(
                *canvas.get_pixel(x, y),
                expected(pixel),
                "sprite ({c}, {r})"
            );
        }
    }
    assert_eq!(*canvas.get_pixel(10, 10), Rgba([0, 0, 0, 0]));
}

#[test]
fn a_quarter_turn_turns_the_sprite_clockwise() {
    let canvas = render(
        Style::Stars,
        &[star_at_center(std::f32::consts::FRAC_PI_2, 1.0)],
        100,
        100,
    );

    // Clockwise: sprite pixel (c, r) goes to (10 - r, c).
    for (r, row) in Style::Stars.sprite().iter().enumerate() {
        for (c, pixel) in row.chars().enumerate() {
            let (x, y) = (45 + 10 - r as u32, 45 + c as u32);
            assert_eq!(
                *canvas.get_pixel(x, y),
                expected(pixel),
                "sprite ({c}, {r})"
            );
        }
    }
}

#[test]
fn alpha_makes_the_particle_transparent() {
    let canvas = render(Style::Stars, &[star_at_center(0.0, 0.5)], 100, 100);

    let Rgba([r, g, b, a]) = *canvas.get_pixel(50, 50);
    assert_eq!((r, g, b), (255, 0, 0));
    assert!((126..=129).contains(&a), "{a}");
}

#[test]
fn fixed_sprite_colors_are_not_tinted() {
    // The smiley's eyes are 'k' (near black), at sprite (4, 3) of the 12 x 12 grid.
    let smiley = ParticleState {
        position: (0.5, 0.5),
        angle: 0.0,
        size: 0.12,
        color: RED,
        alpha: 1.0,
    };
    let canvas = render(Style::Smilies, &[smiley], 100, 100);

    assert_eq!(*canvas.get_pixel(44 + 4, 44 + 3), Rgba([20, 20, 20, 255]));
}

#[test]
fn a_particle_at_the_edge_is_clipped() {
    let corner = ParticleState {
        position: (0.0, 0.0),
        ..star_at_center(0.0, 1.0)
    };

    let canvas = render(Style::Stars, &[corner], 100, 100);

    // The star's center row is solid, so the pixels right of the corner are painted.
    assert_eq!(*canvas.get_pixel(0, 0), Rgba([255, 0, 0, 255]));
}
