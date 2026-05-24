//! Supabase user OAuth via PKCE + localhost-callback.
//!
//! The CLI flow:
//!
//! 1. Generate a PKCE `code_verifier` (43–128 random bytes,
//!    base64url-no-pad) and `code_challenge = base64url(SHA256(code_verifier))`.
//! 2. Bind a TCP listener on a random localhost port.
//! 3. Open the user's browser to
//!    `{supabase_url}/auth/v1/authorize?provider=...&redirect_to=http://localhost:PORT/cb&code_challenge=...&code_challenge_method=S256`.
//! 4. User authenticates in browser; Supabase redirects to the
//!    localhost callback with `?code=AUTH_CODE`.
//! 5. CLI extracts `code` from the query string and exchanges it
//!    (`POST /auth/v1/token?grant_type=pkce` with `auth_code` +
//!    `code_verifier`) for `{access_token, refresh_token, expires_in}`.
//! 6. Returns the `Credentials` ready to persist via
//!    [`CredentialsStore`](crate::auth::storage::CredentialsStore).
//!
//! Prerequisites on the Supabase project:
//!
//! - `http://localhost` (any port) must be in the project's
//!   "Redirect URLs" allowlist. GoTrue defaults reject any redirect
//!   that doesn't match the configured list.
//! - PKCE flow must be enabled (GoTrue ≥ v2.95 supports it by default).

use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{Context, Result};
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Html,
    routing::get,
    Router,
};
use base64::Engine;
use chrono::Utc;
use rand::Rng;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use tokio::sync::oneshot;

use super::storage::{CredentialKind, Credentials};

/// How long we wait for the user to complete the browser auth flow
/// before giving up. Five minutes covers a slow first-login
/// (account create + email verify) without holding the CLI hostage
/// forever.
const DEFAULT_LOGIN_TIMEOUT: Duration = Duration::from_secs(5 * 60);

/// Configuration for one OAuth flow.
#[derive(Debug, Clone)]
pub struct OAuthConfig {
    /// Supabase project URL — e.g. `https://abcd1234.supabase.co`.
    pub supabase_url: String,
    /// Public anon key. Safe to embed in CLI distribution (it's
    /// designed to be embedded in browsers); needed for the
    /// `apikey` header GoTrue expects on every request.
    pub supabase_anon_key: String,
    /// Optional OAuth provider — `Some("google")`, `Some("github")`,
    /// etc. `None` sends the user to Supabase's hosted-UI default,
    /// which lets them pick.
    pub provider: Option<String>,
    /// Override the wait timeout. Falls back to 5 minutes.
    pub timeout: Option<Duration>,
}

impl OAuthConfig {
    /// Build with just `supabase_url` + `supabase_anon_key`. Provider
    /// stays `None` (browser shows Supabase's hosted UI).
    #[must_use]
    pub fn new(supabase_url: impl Into<String>, supabase_anon_key: impl Into<String>) -> Self {
        Self {
            supabase_url: supabase_url.into(),
            supabase_anon_key: supabase_anon_key.into(),
            provider: None,
            timeout: None,
        }
    }

    /// Builder: pin to a specific OAuth provider.
    #[must_use]
    pub fn with_provider(mut self, provider: impl Into<String>) -> Self {
        self.provider = Some(provider.into());
        self
    }

    /// Builder: override the default 5-minute timeout.
    #[must_use]
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }
}

/// What the callback handler sends back to the awaiting flow.
#[derive(Debug)]
enum CallbackOutcome {
    Code(String),
    /// Supabase redirected with `?error=...&error_description=...`
    /// instead of `?code=...` — for instance the user denied access
    /// or the OAuth provider rejected them.
    Error(String),
}

/// Run the full PKCE-with-localhost flow and return persistable
/// credentials.
///
/// # Errors
/// Returns an error for: PRNG failure, port bind failure, timeout
/// waiting for the callback, non-2xx from the Supabase token
/// endpoint, malformed responses.
pub async fn login(http: &reqwest::Client, cfg: &OAuthConfig) -> Result<Credentials> {
    // ── PKCE ──────────────────────────────────────────────────
    let code_verifier = generate_code_verifier();
    let code_challenge = pkce_challenge(&code_verifier);

    // ── Localhost listener ────────────────────────────────────
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.context("bind localhost listener")?;
    let local_addr: SocketAddr = listener.local_addr().context("listener.local_addr")?;
    let port = local_addr.port();
    let redirect_to = format!("http://localhost:{port}/cb");

    let (tx, rx) = oneshot::channel::<CallbackOutcome>();
    let tx = Arc::new(Mutex::new(Some(tx)));

    let app = Router::new().route("/cb", get(callback)).with_state(tx.clone());

    let server = tokio::spawn(async move {
        // Serve until either the callback fires (tx consumed → caller
        // ends the future via shutdown_signal) or the task is dropped
        // (caller timed out). axum::serve doesn't expose a graceful
        // shutdown without an explicit signal; we use a oneshot
        // tripwire driven by the same callback.
        let _ = axum::serve(listener, app).await;
    });

    // ── Browser ───────────────────────────────────────────────
    let authorize_url = build_authorize_url(cfg, &redirect_to, &code_challenge);
    if let Err(e) = webbrowser::open(&authorize_url) {
        // Don't fail the flow — just tell the user to open the URL
        // manually. Headless CI and SSH sessions hit this path.
        eprintln!("(could not open browser: {e}) — open this URL to continue:\n  {authorize_url}");
    } else {
        eprintln!("Opened browser for Supabase login (listening on http://localhost:{port}/cb).");
        eprintln!("If the browser didn't open, navigate to:\n  {authorize_url}");
    }

    // ── Await callback ────────────────────────────────────────
    let timeout = cfg.timeout.unwrap_or(DEFAULT_LOGIN_TIMEOUT);
    let outcome = tokio::time::timeout(timeout, rx).await.context("timed out waiting for OAuth callback")?.context("callback channel closed")?;

    server.abort();

    let code = match outcome {
        CallbackOutcome::Code(c) => c,
        CallbackOutcome::Error(e) => anyhow::bail!("Supabase OAuth returned an error: {e}"),
    };

    // ── Exchange code for tokens ──────────────────────────────
    exchange_code(http, cfg, &code, &code_verifier).await
}

#[derive(Debug, Deserialize)]
struct CallbackParams {
    code: Option<String>,
    error: Option<String>,
    error_description: Option<String>,
}

async fn callback(State(tx): State<Arc<Mutex<Option<oneshot::Sender<CallbackOutcome>>>>>, Query(params): Query<CallbackParams>) -> (StatusCode, Html<&'static str>) {
    let outcome = if let Some(code) = params.code {
        CallbackOutcome::Code(code)
    } else if let Some(err) = params.error {
        let desc = params.error_description.unwrap_or_default();
        CallbackOutcome::Error(format!("{err}: {desc}"))
    } else {
        CallbackOutcome::Error("missing both `code` and `error` query params".into())
    };

    if let Some(sender) = tx.lock().expect("oauth tx poisoned").take() {
        let _ = sender.send(outcome);
    }

    (
        StatusCode::OK,
        Html("<html><body style='font-family:system-ui;padding:2rem;'><h2>Login complete</h2><p>You can close this tab and return to your terminal.</p></body></html>"),
    )
}

fn build_authorize_url(cfg: &OAuthConfig, redirect_to: &str, code_challenge: &str) -> String {
    let base = cfg.supabase_url.trim_end_matches('/');
    let mut params: Vec<(&str, String)> = vec![
        ("redirect_to", redirect_to.into()),
        ("code_challenge", code_challenge.into()),
        ("code_challenge_method", "S256".into()),
    ];
    if let Some(p) = &cfg.provider {
        params.push(("provider", p.clone()));
    }
    let qs: String = params.iter().map(|(k, v)| format!("{k}={}", urlencode(v))).collect::<Vec<_>>().join("&");
    format!("{base}/auth/v1/authorize?{qs}")
}

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

async fn exchange_code(http: &reqwest::Client, cfg: &OAuthConfig, code: &str, code_verifier: &str) -> Result<Credentials> {
    let base = cfg.supabase_url.trim_end_matches('/');
    let url = format!("{base}/auth/v1/token?grant_type=pkce");
    let body = serde_json::json!({
        "auth_code": code,
        "code_verifier": code_verifier,
    });
    let resp = http.post(&url).header("apikey", &cfg.supabase_anon_key).header("Content-Type", "application/json").json(&body).send().await.with_context(|| format!("POST {url}"))?;
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        anyhow::bail!("Supabase code exchange returned HTTP {status}: {text}");
    }
    let body: TokenResponse = serde_json::from_str(&text).with_context(|| format!("parse Supabase token response: {text}"))?;
    let expires_at = body.expires_in.map(|s| Utc::now() + chrono::Duration::seconds(i64::try_from(s).unwrap_or(3600)));
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

// ── PKCE primitives ────────────────────────────────────────────────

fn generate_code_verifier() -> String {
    // RFC 7636 §4.1: 43–128 chars from [A-Z a-z 0-9 -._~]. We generate
    // 64 random bytes and base64url-no-pad encode → 86 chars (well
    // within the limit).
    let mut bytes = [0u8; 64];
    rand::thread_rng().fill(&mut bytes);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

fn pkce_challenge(verifier: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    let digest = hasher.finalize();
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(digest)
}

/// Minimal URL encoder — we only encode values, and only need to
/// escape characters that have special meaning in URL query strings.
/// Pulling in a full crate just for this would be overkill.
fn urlencode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_verifier_length_is_in_pkce_range() {
        let v = generate_code_verifier();
        assert!(v.len() >= 43 && v.len() <= 128, "verifier len {} out of PKCE range", v.len());
    }

    #[test]
    fn code_challenge_is_43_chars_base64url_nopad() {
        let v = generate_code_verifier();
        let c = pkce_challenge(&v);
        assert_eq!(c.len(), 43, "SHA256 → 32 bytes → 43 chars base64url-no-pad");
        // No padding, URL-safe alphabet only.
        assert!(!c.contains('='));
        assert!(!c.contains('+'));
        assert!(!c.contains('/'));
    }

    #[test]
    fn code_challenge_is_deterministic() {
        let v = "fixed-verifier-for-determinism-check-1234567890";
        let a = pkce_challenge(v);
        let b = pkce_challenge(v);
        assert_eq!(a, b);
    }

    #[test]
    fn code_challenge_known_vector() {
        // RFC 7636 Appendix B test vector.
        let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
        let expected_challenge = "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM";
        assert_eq!(pkce_challenge(verifier), expected_challenge);
    }

    #[test]
    fn urlencode_handles_spaces_and_slashes() {
        assert_eq!(urlencode("hello world"), "hello%20world");
        assert_eq!(urlencode("https://x.y/cb"), "https%3A%2F%2Fx.y%2Fcb");
    }

    #[test]
    fn build_authorize_url_includes_provider_and_pkce() {
        let cfg = OAuthConfig::new("https://abcd.supabase.co", "anon-key").with_provider("google");
        let url = build_authorize_url(&cfg, "http://localhost:54321/cb", "challenge123");
        assert!(url.starts_with("https://abcd.supabase.co/auth/v1/authorize?"));
        assert!(url.contains("provider=google"));
        assert!(url.contains("code_challenge=challenge123"));
        assert!(url.contains("code_challenge_method=S256"));
        assert!(url.contains("redirect_to=http%3A%2F%2Flocalhost%3A54321%2Fcb"));
    }

    #[test]
    fn build_authorize_url_omits_provider_when_none() {
        let cfg = OAuthConfig::new("https://abcd.supabase.co", "anon-key");
        let url = build_authorize_url(&cfg, "http://localhost:1/cb", "chal");
        assert!(!url.contains("provider="));
    }

    #[test]
    fn supabase_url_trailing_slash_normalized() {
        let cfg = OAuthConfig::new("https://abcd.supabase.co/", "anon-key");
        let url = build_authorize_url(&cfg, "http://localhost:1/cb", "chal");
        // No double slash before /auth/v1/...
        assert!(!url.contains("//auth/v1/"));
        assert!(url.contains("/auth/v1/"));
    }
}
