use review_queue::snooze::{Snooze, load, save};

fn snooze(number: u64) -> Snooze {
    Snooze {
        repo: "acme/widgets".into(),
        number,
        started_at: "2026-10-20T00:00:00Z".parse().unwrap(),
        until: "2026-10-22T00:00:00Z".parse().unwrap(),
    }
}

#[test]
fn saved_snoozes_load_back_the_same_and_the_directory_is_created() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("state").join("review-queue").join("snoozes.json");

    save(&path, &[snooze(1), snooze(2)]).unwrap();

    assert_eq!(load(&path), vec![snooze(1), snooze(2)]);
}

#[test]
fn a_missing_file_loads_as_no_snoozes() {
    let dir = tempfile::tempdir().unwrap();

    assert_eq!(load(&dir.path().join("snoozes.json")), Vec::<Snooze>::new());
}

#[test]
fn a_damaged_file_loads_as_no_snoozes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("snoozes.json");
    std::fs::write(&path, "{ not json").unwrap();

    assert_eq!(load(&path), Vec::<Snooze>::new());
}
