use review_queue::burst::{AsciiCell, ParticleState, Style, render_ascii};

fn particle(position: (f32, f32), angle: f32, alpha: f32) -> ParticleState {
    ParticleState {
        position,
        angle,
        size: 0.1,
        color: [200, 100, 50],
        alpha,
    }
}

#[test]
fn a_particle_is_its_style_glyph_centered_on_its_cell() {
    let cells = render_ascii(Style::Smilies, &[particle((0.5, 0.5), 0.0, 1.0)], 40, 20);

    assert_eq!(
        cells,
        vec![AsciiCell {
            col: 19,
            row: 10,
            text: ":)".into(),
            color: [200, 100, 50],
        }]
    );
}

#[test]
fn rotation_is_ignored() {
    let straight = render_ascii(Style::Stars, &[particle((0.3, 0.7), 0.0, 1.0)], 40, 20);
    let turned = render_ascii(Style::Stars, &[particle((0.3, 0.7), 2.0, 1.0)], 40, 20);

    assert_eq!(straight, turned);
}

#[test]
fn fading_darkens_the_glyph_and_a_nearly_invisible_particle_is_not_shown() {
    let half = render_ascii(Style::Stars, &[particle((0.5, 0.5), 0.0, 0.5)], 40, 20);
    assert_eq!(half[0].color, [100, 50, 25]);

    let faint = render_ascii(Style::Stars, &[particle((0.5, 0.5), 0.0, 0.1)], 40, 20);
    assert!(faint.is_empty());
}

#[test]
fn a_glyph_at_the_edge_is_cut_and_a_particle_outside_the_pane_is_not_shown() {
    let cells = render_ascii(
        Style::FuckYeah,
        &[
            particle((0.0, 0.5), 0.0, 1.0),
            particle((0.5, 1.2), 0.0, 1.0),
            particle((-0.5, 0.5), 0.0, 1.0),
        ],
        40,
        20,
    );

    // "FUCK YEAH" is 9 characters. Centered on column 0, its middle character (the space)
    // is at column 0, so only " YEAH" is in the pane.
    assert_eq!(cells.len(), 1);
    assert_eq!((cells[0].col, cells[0].row), (0, 10));
    assert_eq!(cells[0].text, " YEAH");
}
