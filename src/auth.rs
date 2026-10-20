//! The GitHub token: from `GITHUB_TOKEN`, or else from `gh auth token`.

#[derive(Debug, PartialEq, Eq)]
pub struct AuthError(pub String);

/// `env_token` is the value of `GITHUB_TOKEN`. `gh_token` runs `gh auth token`.
pub fn resolve_token(
    env_token: Option<String>,
    gh_token: impl FnOnce() -> Result<String, String>,
) -> Result<String, AuthError> {
    if let Some(token) = env_token.filter(|t| !t.trim().is_empty()) {
        return Ok(token);
    }
    let advice = "Set GITHUB_TOKEN, or run `gh auth login`.";
    match gh_token() {
        Ok(token) if !token.trim().is_empty() => Ok(token.trim().to_string()),
        Ok(_) => Err(AuthError(format!(
            "`gh auth token` gave no token. {advice}"
        ))),
        Err(e) => Err(AuthError(format!("`gh auth token` failed: {e}. {advice}"))),
    }
}
