use review_queue::burst::{Style, palette};

#[test]
fn every_style_has_a_rectangular_sprite_with_known_colors_and_the_particle_color() {
    assert_eq!(Style::ALL.len(), 9);
    for style in Style::ALL {
        let rows = style.sprite();
        assert!(!rows.is_empty(), "{style:?}");
        let width = rows[0].chars().count();
        for row in rows {
            assert_eq!(row.chars().count(), width, "{style:?}: {row:?}");
            for c in row.chars() {
                assert!(
                    c == '.' || c == '#' || palette(c).is_some(),
                    "{style:?}: {c:?}"
                );
            }
        }
        assert!(rows.iter().any(|r| r.contains('#')), "{style:?}");
    }
}

#[test]
fn every_style_has_an_ascii_glyph_and_a_distinct_name() {
    let mut names = Vec::new();
    for style in Style::ALL {
        let glyph = style.ascii();
        assert!(!glyph.is_empty() && glyph.is_ascii(), "{style:?}");
        assert!(!names.contains(&style.name()), "{style:?}");
        names.push(style.name());
    }
}
