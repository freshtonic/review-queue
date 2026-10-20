use review_queue::auth::resolve_token;

fn gh_must_not_run() -> Result<String, String> {
    panic!("gh must not run when GITHUB_TOKEN is set")
}

#[test]
fn github_token_overrides_gh() {
    let token = resolve_token(Some("from-env".into()), gh_must_not_run);

    assert_eq!(token.unwrap(), "from-env");
}

#[test]
fn without_github_token_the_token_comes_from_gh_without_the_newline() {
    let token = resolve_token(None, || Ok("from-gh\n".into()));

    assert_eq!(token.unwrap(), "from-gh");
}

#[test]
fn an_empty_github_token_is_the_same_as_no_github_token() {
    let token = resolve_token(Some("".into()), || Ok("from-gh\n".into()));

    assert_eq!(token.unwrap(), "from-gh");
}

#[test]
fn when_gh_fails_the_error_says_to_run_gh_auth_login() {
    let error = resolve_token(None, || Err("gh: command not found".into())).unwrap_err();

    assert!(error.0.contains("gh auth login"), "{}", error.0);
    assert!(error.0.contains("gh: command not found"), "{}", error.0);
}

#[test]
fn an_empty_token_from_gh_is_an_error() {
    let error = resolve_token(None, || Ok("\n".into())).unwrap_err();

    assert!(error.0.contains("gh auth login"), "{}", error.0);
}
