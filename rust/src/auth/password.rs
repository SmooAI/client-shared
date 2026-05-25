//! Supabase email + password grant.
//!
//! `POST {supabase_url}/auth/v1/token?grant_type=password` with
//! `{email, password}` body + `apikey` header. Returns
//! `{access_token, refresh_token, expires_in, user}`.
//!
//! Simpler than the OAuth flow ([`super::oauth`]) — no browser, no
//! PKCE, no localhost callback, no Supabase redirect-URL config
//! prerequisite. CLI just prompts the user for email + password
//! (password without echo) and POSTs.
//!
//! Works in any environment that can make an HTTPS request: SSH
//! sessions, CI, Docker containers, headless servers, etc. The
//! trade-off vs OAuth is that the CLI sees the user's password
//! once (in memory only — never stored). If MFA is enabled on the
//! user's Supabase account, this flow will fail at the API level
//! with a clear error.

use anyhow::{Context, Result};
use chrono::Utc;
use serde::Deserialize;

use super::storage::{CredentialKind, Credentials};

/// Wire shape of the Supabase password-grant response.
#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    #[serde(default)]
    refresh_token: Option<String>,
    #[serde(default)]
    expires_in: Option<u64>,
    #[serde(default)]
    user: Option<TokenUser>,
}

#[derive(Debug, Deserialize)]
struct TokenUser {
    id: String,
    email: Option<String>,
}

/// Exchange an email + password pair for a Supabase user session.
///
/// # Errors
/// - Network failures
/// - Non-2xx from `/auth/v1/token` (wrong password, MFA required,
///   user not found, etc. — surfaces the upstream error verbatim
///   so the user sees Supabase's exact wording)
/// - Malformed response body
pub async fn password_grant(
    http: &reqwest::Client,
    supabase_url: &str,
    anon_key: &str,
    email: &str,
    password: &str,
) -> Result<Credentials> {
    let url = format!(
        "{}/auth/v1/token?grant_type=password",
        supabase_url.trim_end_matches('/')
    );
    let body = serde_json::json!({
        "email": email,
        "password": password,
    });
    let resp = http
        .post(&url)
        .header("apikey", anon_key)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .with_context(|| format!("POST {url}"))?;
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        anyhow::bail!("Supabase password grant returned HTTP {status}: {text}");
    }
    let body: TokenResponse = serde_json::from_str(&text)
        .with_context(|| format!("parse password-grant response: {text}"))?;
    let expires_at = body
        .expires_in
        .map(|s| Utc::now() + chrono::Duration::seconds(i64::try_from(s).unwrap_or(3600)));
    let user_display = body.user.and_then(|u| u.email.clone().or(Some(u.id)));
    Ok(Credentials {
        access_token: body.access_token,
        refresh_token: body.refresh_token,
        expires_at,
        user: user_display,
        active_org_id: None,
        client_id: None,
        client_secret: None,
        kind: CredentialKind::User,
        created_at: Utc::now(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Parser handles the minimum fields Supabase guarantees.
    #[test]
    fn token_response_parses_minimal() {
        let raw = r#"{"access_token":"jwt"}"#;
        let r: TokenResponse = serde_json::from_str(raw).expect("parse");
        assert_eq!(r.access_token, "jwt");
        assert!(r.refresh_token.is_none());
        assert!(r.expires_in.is_none());
    }

    /// Full Supabase response with nested user.
    #[test]
    fn token_response_parses_with_user() {
        let raw = r#"{
            "access_token":"jwt",
            "refresh_token":"rtok",
            "expires_in":3600,
            "user":{"id":"u-123","email":"brent@smoo.ai"}
        }"#;
        let r: TokenResponse = serde_json::from_str(raw).expect("parse");
        assert_eq!(r.refresh_token.as_deref(), Some("rtok"));
        assert_eq!(r.expires_in, Some(3600));
        let u = r.user.expect("user");
        assert_eq!(u.id, "u-123");
        assert_eq!(u.email.as_deref(), Some("brent@smoo.ai"));
    }
}
