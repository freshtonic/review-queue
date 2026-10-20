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
            assert!(
                speed >= SPEED.start() - 1e-4 && speed <= SPEED.end() + 1e-4,
                "{speed}"
            );
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

use review_queue::burst::{Burst, DURATION, GRAVITY, Particle};

fn one_particle(p: Particle) -> Burst {
    Burst {
        style: Style::Stars,
        particles: vec![p],
    }
}

fn straight_up() -> Particle {
    Particle {
        velocity: (0.0, -1.0),
        angle: 0.5,
        spin: 2.0,
        size: 0.1,
        color: [255, 0, 0],
        fade_start: 0.2,
        fade_end: 0.4,
    }
}

#[test]
fn gravity_pulls_a_particle_down_so_that_it_moves_in_an_arc() {
    let burst = one_particle(straight_up());

    let y = |t: f32| burst.at(t)[0].position.1;

    // y = 0.5 - 1.0 t + GRAVITY t² / 2
    let expected = 0.5 - 0.3 + GRAVITY * 0.09 / 2.0;
    assert!((y(0.3) - expected).abs() < 1e-5, "{} != {expected}", y(0.3));
    // Up first (y gets smaller), then down when gravity wins.
    assert!(y(0.1) < y(0.0));
    let top = 1.0 / GRAVITY;
    assert!(y(top + 0.1) > y(top));
}

#[test]
fn a_particle_turns_at_its_spin_rate() {
    let burst = one_particle(straight_up());

    assert!((burst.at(0.25)[0].angle - (0.5 + 2.0 * 0.25)).abs() < 1e-5);
}

#[test]
fn a_particle_is_opaque_until_its_fade_starts_then_fades_to_invisible() {
    let burst = one_particle(straight_up());
    let alpha = |t: f32| burst.at(t)[0].alpha;

    assert_eq!(alpha(0.1), 1.0);
    assert_eq!(alpha(0.2), 1.0);
    assert!((alpha(0.3) - 0.5).abs() < 1e-5);
    assert_eq!(alpha(0.4), 0.0);
    assert_eq!(alpha(0.45), 0.0);
}

#[test]
fn at_the_end_of_the_burst_every_particle_is_invisible() {
    for seed in 0..20 {
        let burst = spawn(Style::Party, seed);
        assert!(burst.at(DURATION).iter().all(|p| p.alpha == 0.0));
        assert!(!burst.is_done(DURATION - 0.01));
        assert!(burst.is_done(DURATION));
    }
}
