use std::fs::File;
use std::time::{Duration, SystemTime};

use review_queue::avatar::{Freshness, MAX_AGE, freshness, path_for};

#[test]
fn the_path_is_the_lowercase_login_with_unsafe_characters_replaced() {
    let dir = std::path::Path::new("/cache/avatars");

    assert_eq!(path_for(dir, "Alice-B"), dir.join("alice-b.png"));
    assert_eq!(path_for(dir, "../x/y"), dir.join("___x_y.png"));
}

#[test]
fn a_file_is_fresh_for_seven_days_then_stale() {
    let dir = tempfile::tempdir().unwrap();
    let path = path_for(dir.path(), "alice");
    let written = SystemTime::now() - Duration::from_secs(60);
    File::create(&path).unwrap().set_modified(written).unwrap();

    assert_eq!(
        freshness(&path, written + Duration::from_secs(60)),
        Freshness::Fresh
    );
    assert_eq!(
        freshness(&path, written + MAX_AGE - Duration::from_secs(1)),
        Freshness::Fresh
    );
    assert_eq!(
        freshness(&path, written + MAX_AGE + Duration::from_secs(1)),
        Freshness::Stale
    );
}

#[test]
fn a_file_that_does_not_exist_is_missing() {
    let dir = tempfile::tempdir().unwrap();

    assert_eq!(
        freshness(&path_for(dir.path(), "bob"), SystemTime::now()),
        Freshness::Missing
    );
}
