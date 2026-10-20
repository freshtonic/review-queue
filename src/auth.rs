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
    gh_token()
        .map(|t| t.trim().to_string())
        .map_err(AuthError)
}
