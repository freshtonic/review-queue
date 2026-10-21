use review_queue::config::Config;
use review_queue::notify::NotifySettings;
use review_queue::queue::PriorityTier;

fn parse(text: &str) -> Config {
    toml::from_str(text).unwrap()
}

#[test]
fn notifications_default_to_on_for_personal_requests_and_team_re_reviews() {
    assert_eq!(parse("").notify_settings(), Ok(NotifySettings::default()));
}

#[test]
fn notifications_can_be_turned_off_and_their_tiers_chosen_by_name() {
    let config = parse(
        r#"
        notifications = false
        notify_tiers = ["personal-re-review", "team-first-review"]
        "#,
    );
    assert_eq!(
        config.notify_settings(),
        Ok(NotifySettings {
            enabled: false,
            tiers: vec![
                PriorityTier::PersonalReReview,
                PriorityTier::TeamFirstReview
            ],
        })
    );
}

#[test]
fn an_unknown_tier_name_is_an_error_that_lists_the_names() {
    let error = parse(r#"notify_tiers = ["personal"]"#)
        .notify_settings()
        .unwrap_err();
    assert!(error.contains("\"personal\""), "{error}");
    assert!(error.contains("team-re-review"), "{error}");
}
