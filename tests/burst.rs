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
