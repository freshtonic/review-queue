use review_queue::burst::{Style, spawn};

#[test]
fn every_particle_starts_at_the_center_of_the_pane() {
    let burst = spawn(Style::Stars, 7);

    let start = burst.at(0.0);

    assert!(!start.is_empty());
    for particle in start {
        assert_eq!(particle.position, (0.5, 0.5));
    }
}

#[test]
fn the_same_seed_gives_the_same_burst_and_another_seed_does_not() {
    assert_eq!(spawn(Style::Cats, 42), spawn(Style::Cats, 42));
    assert_ne!(spawn(Style::Cats, 42), spawn(Style::Cats, 43));
}

use review_queue::burst::{COUNT, FADE, SIZE, SPEED, SPIN};

fn distinct<T: PartialEq>(values: Vec<T>) -> usize {
    let mut seen: Vec<T> = Vec::new();
    for v in values {
        if !seen.contains(&v) {
            seen.push(v);
        }
    }
    seen.len()
}

#[test]
fn each_random_value_is_within_its_limits_and_differs_between_particles() {
    for seed in 0..50 {
        let burst = spawn(Style::Hotdogs, seed);
        assert!(COUNT.contains(&burst.particles.len()));
        for p in &burst.particles {
            let speed = (p.velocity.0.powi(2) + p.velocity.1.powi(2)).sqrt();
            assert!(speed >= SPEED.start() - 1e-4 && speed <= SPEED.end() + 1e-4, "{speed}");
            assert!(SPIN.contains(&p.spin));
            assert!(SIZE.contains(&p.size));
            assert!(FADE.contains(&p.fade_start) && FADE.contains(&p.fade_end));
            assert!(p.fade_start < p.fade_end);
        }
        let ps = &burst.particles;
        assert!(distinct(ps.iter().map(|p| p.color).collect()) > 1);
        assert!(distinct(ps.iter().map(|p| p.spin.to_bits()).collect()) > 1);
        assert!(distinct(ps.iter().map(|p| p.size.to_bits()).collect()) > 1);
        assert!(distinct(ps.iter().map(|p| p.fade_end.to_bits()).collect()) > 1);
    }
}
