use review_queue::auth::resolve_token;

fn gh_must_not_run() -> Result<String, String> {
    panic!("gh must not run when GITHUB_TOKEN is set")
}

#[test]
fn github_token_overrides_gh() {
    let token = resolve_token(Some("from-env".into()), gh_must_not_run);

    assert_eq!(token.unwrap(), "from-env");
}
