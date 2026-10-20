//! Runs each scenario in `tests/corpus/`. A scenario is a directory with:
//!
//! - `response.json`: a synthetic GraphQL response to the Queue query
//! - `config.toml`: the user's config (optional; the default config if it is not there)
//! - `expected.txt`: the Queue, one item per line: `<tier> <repo>#<number> [<team>]`
//! - `snoozes.json` and `now.txt` (optional): the user's Snoozes, and the time of the poll.
//!   With these, `expected.txt` also lists each snoozed request as
//!   `snoozed <tier> <repo>#<number> [<team>] until <time>`, then each Snooze that is
//!   still active as `keep <repo>#<number>`.
//!
//! The expected Queue is written by hand from the rules in `CONTEXT.md`.

use std::path::Path;

use review_queue::queue::{PriorityTier, QueueItem, build_queue};
use review_queue::snooze::{Snooze, apply_snoozes};
use review_queue::{config, github};

fn tier_name(tier: PriorityTier) -> &'static str {
    match tier {
        PriorityTier::PersonalReReview => "personal-re-review",
        PriorityTier::PersonalFirstReview => "personal-first-review",
        PriorityTier::TeamReReview => "team-re-review",
        PriorityTier::TeamFirstReview => "team-first-review",
    }
}

fn describe(item: &QueueItem) -> String {
    let pr = &item.pull_request;
    let mut line = format!("{} {}#{}", tier_name(item.tier), pr.repo, pr.number);
    if let Some(team) = &item.team {
        line.push(' ');
        line.push_str(team);
    }
    line
}

fn run(scenario: &Path) -> Result<(), String> {
    let response = std::fs::read_to_string(scenario.join("response.json"))
        .map_err(|e| format!("response.json: {e}"))?;
    let config = config::load(&scenario.join("config.toml"))?;
    let expected = std::fs::read_to_string(scenario.join("expected.txt"))
        .map_err(|e| format!("expected.txt: {e}"))?;

    let snapshot = github::parse_response(&response).map_err(|e| e.0)?;
    let queue = build_queue(
        &snapshot.pull_requests,
        &snapshot.viewer,
        &config.watched_teams,
    );

    let mut actual: Vec<String> = Vec::new();
    let snoozes_path = scenario.join("snoozes.json");
    if snoozes_path.exists() {
        let snoozes: Vec<Snooze> = serde_json::from_str(
            &std::fs::read_to_string(&snoozes_path).map_err(|e| format!("snoozes.json: {e}"))?,
        )
        .map_err(|e| format!("snoozes.json: {e}"))?;
        let now = std::fs::read_to_string(scenario.join("now.txt"))
            .map_err(|e| format!("now.txt: {e}"))?
            .trim()
            .parse()
            .map_err(|e| format!("now.txt: {e}"))?;
        let outcome = apply_snoozes(queue, &snoozes, now);
        actual.extend(outcome.visible.iter().map(describe));
        actual.extend(
            outcome
                .snoozed
                .iter()
                .map(|s| format!("snoozed {} until {}", describe(&s.item), s.until)),
        );
        actual.extend(
            outcome
                .kept
                .iter()
                .map(|s| format!("keep {}#{}", s.repo, s.number)),
        );
    } else {
        actual.extend(queue.iter().map(describe));
    }
    let expected: Vec<&str> = expected.lines().filter(|l| !l.trim().is_empty()).collect();
    if actual == expected {
        Ok(())
    } else {
        Err(format!(
            "expected:\n  {}\nactual:\n  {}",
            expected.join("\n  "),
            actual.join("\n  ")
        ))
    }
}

#[test]
fn each_corpus_scenario_gives_its_expected_queue() {
    let mut scenarios: Vec<_> = std::fs::read_dir("tests/corpus")
        .expect("tests/corpus exists")
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.is_dir())
        .collect();
    scenarios.sort();
    assert!(!scenarios.is_empty(), "tests/corpus has no scenarios");

    let failures: Vec<String> = scenarios
        .iter()
        .filter_map(|s| {
            run(s)
                .err()
                .map(|e| format!("{}:\n{e}", s.file_name().unwrap().to_string_lossy()))
        })
        .collect();

    assert!(
        failures.is_empty(),
        "{} of {} scenarios failed:\n\n{}",
        failures.len(),
        scenarios.len(),
        failures.join("\n\n")
    );
}
