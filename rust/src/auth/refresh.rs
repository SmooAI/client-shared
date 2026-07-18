//! Supabase `refresh_token` grant — exchange a long-lived refresh
//! token for a fresh access_token without re-prompting the user.
//!
//! `POST {supabase_url}/auth/v1/token?grant_type=refresh_token` with
//! `{refresh_token}` body + `apikey` header. Returns the same
//! `{access_token, refresh_token, expires_in, user}` shape as the
//! initial password grant.
//!
//! Supabase rotates refresh tokens by default: every successful
//! exchange returns a **new** refresh_token, the old one is
//! revoked. Callers MUST persist the new token (and the new
//! `expires_at`) — using a stale refresh_token will 400 with
//! `invalid_grant`, at which point the user has to `th auth login`
//! again. The 30-day refresh lifetime is per-session, not
//! per-token — staying continuously active extends indefinitely.

use anyhow::{Context, Result};
use chrono::Utc;
use serde::Deserialize;

use super::storage::{CredentialKind, Credentials};

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

/// Exchange a stored refresh_token for a fresh access_token.
///
/// `previous` is the currently-stored Credentials. We use its
/// `refresh_token` field for the exchange and preserve its
/// `active_org_id` (which is a CLI-side selection, not a Supabase
/// concept — the server doesn't return it).
///
/// # Errors
/// - `previous.refresh_token` is `None` (this session was created
///   without a refresh token — should be impossible for user
///   sessions but possible for adhoc imports)
/// - Network failures
/// - 400 from `/token` (refresh token expired, revoked, or rotated
///   away by a concurrent exchange) — caller should re-prompt for
///   password
pub async fn refresh_session(
    http: &reqwest::Client,
    supabase_url: &str,
    anon_key: &str,
    previous: &Credentials,
) -> Result<Credentials> {
    let refresh_token = previous
        .refresh_token
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("session has no refresh_token — re-run `th auth login`"))?;

    let url = format!(
        "{}/auth/v1/token?grant_type=refresh_token",
        supabase_url.trim_end_matches('/')
    );
    let body = serde_json::json!({ "refresh_token": refresh_token });
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
        anyhow::bail!(
            "refresh_token grant returned HTTP {status}: {text} (re-run `th auth login`)"
        );
    }
    let body: TokenResponse =
        serde_json::from_str(&text).with_context(|| format!("parse refresh response: {text}"))?;
    let expires_at = body
        .expires_in
        .map(|s| Utc::now() + chrono::Duration::seconds(i64::try_from(s).unwrap_or(3600)));
    let user_display = body
        .user
        .and_then(|u| u.email.clone().or(Some(u.id)))
        .or_else(|| previous.user.clone());
    Ok(Credentials {
        access_token: body.access_token,
        // Supabase rotates — use the new refresh_token if returned,
        // fall back to the previous one if the server omitted it
        // (some Supabase versions don't rotate every refresh).
        refresh_token: body
            .refresh_token
            .or_else(|| previous.refresh_token.clone()),
        expires_at,
        user: user_display,
        active_org_id: previous.active_org_id.clone(),
        client_id: None,
        client_secret: None,
        kind: CredentialKind::User,
        created_at: previous.created_at,
    })
}

/// `true` when the credentials should be refreshed proactively —
/// either already expired, or within 5 minutes of expiring. Lets
/// callers refresh ahead of the wire round-trip so the next API
/// call doesn't pay the latency.
#[must_use]
pub fn should_refresh(creds: &Credentials) -> bool {
    match creds.expires_at {
        Some(exp) => Utc::now() >= exp - chrono::Duration::minutes(5),
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(expires_at: Option<chrono::DateTime<Utc>>) -> Credentials {
        Credentials {
            access_token: "tok".into(),
            refresh_token: Some("rtok".into()),
            expires_at,
            user: Some("brent@smoo.ai".into()),
            active_org_id: Some("org_abc".into()),
            client_id: None,
            client_secret: None,
            kind: CredentialKind::User,
            created_at: Utc::now(),
        }
    }

    #[test]
    fn should_refresh_when_no_expiry_returns_false() {
        // Without an expiry we can't know — treat as "don't refresh".
        // The next API call will 401 if the token's bad and the
        // caller can fall back to password login.
        assert!(!should_refresh(&fixture(None)));
    }

    #[test]
    fn should_refresh_when_expiry_in_past_returns_true() {
        assert!(should_refresh(&fixture(Some(
            Utc::now() - chrono::Duration::hours(1)
        ))));
    }

    #[test]
    fn should_refresh_within_5_min_window_returns_true() {
        assert!(should_refresh(&fixture(Some(
            Utc::now() + chrono::Duration::minutes(3)
        ))));
    }

    #[test]
    fn should_refresh_more_than_5_min_out_returns_false() {
        assert!(!should_refresh(&fixture(Some(
            Utc::now() + chrono::Duration::minutes(15)
        ))));
    }

    #[test]
    fn refresh_response_parses_with_user() {
        let raw = r#"{
            "access_token":"new-jwt",
            "refresh_token":"new-rtok",
            "expires_in":3600,
            "user":{"id":"u-123","email":"brent@smoo.ai"}
        }"#;
        let r: TokenResponse = serde_json::from_str(raw).expect("parse");
        assert_eq!(r.access_token, "new-jwt");
        assert_eq!(r.refresh_token.as_deref(), Some("new-rtok"));
        assert_eq!(r.expires_in, Some(3600));
    }

    #[test]
    fn refresh_response_parses_without_rotated_refresh() {
        // Some Supabase versions don't rotate on every refresh —
        // the response omits refresh_token. Our caller falls back
        // to the previous one in that case.
        let raw = r#"{"access_token":"new-jwt","expires_in":3600}"#;
        let r: TokenResponse = serde_json::from_str(raw).expect("parse");
        assert!(r.refresh_token.is_none());
    }
}
