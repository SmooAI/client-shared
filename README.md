<p align="center">
  <a href="https://smoo.ai"><img src=".github/banner.png" alt="@smooai/client-shared — Shared primitives for every Smoo AI client" width="100%" /></a>
</p>

<p align="center">
  <a href="https://smoo.ai/open-source"><img src="https://img.shields.io/badge/Smoo_AI-platform-00A6A6?style=for-the-badge&labelColor=020618" alt="Smoo AI"></a>
  <a href="./LICENSE"><img src="https://img.shields.io/badge/license-MIT-F49F0A?style=for-the-badge&labelColor=020618" alt="license"></a>
  <a href="https://github.com/SmooAI/smooth"><img src="https://img.shields.io/badge/consumed_by-th_CLI-FF6B6C?style=for-the-badge&labelColor=020618" alt="consumed by the th CLI"></a>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/OAuth_PKCE_·_localhost_callback-00A6A6?style=flat-square" alt="OAuth PKCE localhost callback">
  <img src="https://img.shields.io/badge/M2M_client__credentials-00A6A6?style=flat-square" alt="M2M client credentials">
  <img src="https://img.shields.io/badge/0600_credential_store-F49F0A?style=flat-square" alt="0600 credential store">
  <img src="https://img.shields.io/badge/PKCE_·_M2M_·_0600_store-FF6B6C?style=flat-square" alt="PKCE M2M 0600 store">
</p>

<p align="center">
  <a href="#what-is-this"><b>What it is</b></a> &nbsp;·&nbsp; <a href="#feature-tour"><b>Feature tour</b></a> &nbsp;·&nbsp; <a href="#quickstart"><b>Quickstart</b></a> &nbsp;·&nbsp; <a href="#honest-status"><b>Honest status</b></a> &nbsp;·&nbsp; <a href="#-part-of-smoo-ai"><b>Platform</b></a>
</p>

---

> **One Supabase auth story, shared by every Smoo AI Rust client.** Browser OAuth with PKCE and a localhost callback, email+password for headless environments, refresh-token rotation, the M2M `client_credentials` grant — and one `0600` on-disk credential store holding user and machine sessions side by side. Consumed in production by the [`th` CLI](https://github.com/SmooAI/smooth). **Rust-only today; not yet on crates.io** — a git dependency is the install path.

## What is this?

The auth plumbing every Smoo AI Rust client needs identically, in one crate instead of re-implemented per app:

1. **Sign a human in** — browser OAuth (PKCE + localhost callback), or email+password where no browser exists.
2. **Keep them signed in** — refresh-token grant with Supabase's rotation handled, plus a renew-ahead window.
3. **Sign a machine in** — the RFC 6749 `client_credentials` grant against `auth.smoo.ai/token`.
4. **Put the credentials somewhere sane** — a `0600` store holding user and M2M sessions together.

> **This crate used to carry the design system too**, and called itself [`SmooAI/ui`](https://github.com/SmooAI/ui)'s successor. That migration was never finished: nothing ever imported `smooai_client_shared::ui`, while the real design-system consumers (smooblue, observability-studio) depended on `SmooAI/ui` directly. Two copies of the same files is a drift surface, and it had already drifted — this crate spent weeks serving a monogram missing its inner 'S'. The copy is gone. **`SmooAI/ui` owns the design system**; this crate is auth.

## Feature tour

| | Capability | What you get |
| --- | --- | --- |
| 🔐 | [**Browser OAuth (PKCE)**](#-browser-oauth-pkce--localhost-callback) | Spawns a localhost callback, opens the browser, captures the Supabase session |
| 🔑 | [**Email + password grant**](#-email--password-grant) | Headless-friendly login — SSH, CI, Docker, no browser needed |
| ♻️ | [**Session refresh**](#-session-refresh) | `refresh_token` grant with rotation handling + a renew-ahead window |
| 🤖 | [**M2M `client_credentials`**](#-m2m-client_credentials) | RFC 6749 service-account grant against `auth.smoo.ai/token` |
| 💾 | [**`CredentialsStore`**](#-the-credentials-store) | One 0600 on-disk store for user + M2M sessions, side by side |

All snippets below are the actual API, verified against `rust/src/`.

### 🔐 Browser OAuth (PKCE + localhost callback)

The CLI login flow: generate a PKCE verifier/challenge, bind a random localhost port, open the browser to Supabase's authorize endpoint, capture the redirect, exchange the code for a session — and hand back `Credentials` ready to persist:

```rust
use smooai_client_shared::auth::{oauth::{login, OAuthConfig}, CredentialsStore};

let http = reqwest::Client::new();
let cfg = OAuthConfig::new("https://abcd1234.supabase.co", anon_key)
    .with_provider("google");

let creds = login(&http, &cfg).await?;      // opens the browser, waits ≤5 min
CredentialsStore::default_user()?.save(&creds)?;   // ~/.smooth/auth/smooai-user.json, mode 0600
```

> Prerequisite: `http://localhost` must be in the Supabase project's Redirect URLs allowlist, and PKCE enabled (GoTrue ≥ v2.95 default).

### 🔑 Email + password grant

No browser, no PKCE, no redirect-URL config — works over SSH, in CI, in containers. The password is held in memory only, never stored; MFA-enabled accounts fail with the upstream error verbatim:

```rust
use smooai_client_shared::auth::password::password_grant;

let creds = password_grant(&http, supabase_url, anon_key, "you@smoo.ai", &password).await?;
```

### ♻️ Session refresh

Supabase rotates refresh tokens on every exchange — the returned `Credentials` carries the **new** one and must be persisted, or the next refresh 400s. `should_refresh` reports the 5-minute-ahead window so long-running processes renew before a wire call fails:

```rust
use smooai_client_shared::auth::refresh::{refresh_session, should_refresh};

if should_refresh(&creds) {
    let fresh = refresh_session(&http, supabase_url, anon_key, &creds).await?;
    store.save(&fresh)?;   // MUST persist — the old refresh_token is now revoked
}
```

### 🤖 M2M `client_credentials`

RFC 6749 service-account grant: mint a `client_id`/`client_secret` in the Smoo web app, exchange for an org-scoped bearer at `https://auth.smoo.ai/token` (override with `SMOOAI_AUTH_URL` for staging):

```rust
use smooai_client_shared::auth::{m2m::client_credentials_grant, CredentialsStore};

// The token URL comes from token_url(): SMOOAI_AUTH_URL override, else auth.smoo.ai/token.
let creds = client_credentials_grant(&http, &client_id, &client_secret).await?;
CredentialsStore::default_m2m()?.save(&creds)?;    // ~/.smooth/auth/smooai.json
```

### 💾 The credentials store

Both flows share one on-disk shape. Two well-known files by convention — a single host carries a user session and an M2M session simultaneously without collision — written with **mode 0600**:

```rust
use smooai_client_shared::auth::{Credentials, CredentialsStore};

let store = CredentialsStore::default_user()?;   // or ::default_m2m(), or ::at(path)
if let Some(creds) = store.load()? {
    if creds.is_expired() { /* refresh or re-login */ }
}
```

## Quickstart

**`smooai-client-shared` is not published to crates.io.** Consume it as a git dependency — this is exactly how the [`th` CLI](https://github.com/SmooAI/smooth) consumes it in production (rev-pinned, `features = ["auth"]`):

```toml
[dependencies]
smooai-client-shared = { git = "https://github.com/SmooAI/client-shared.git", features = ["auth"] }
```

### Feature flags

| Feature | Adds | Pulls | Status |
| --- | --- | --- | --- |
| `auth` | Supabase OAuth + password + refresh, M2M, `CredentialsStore` | `tokio`, `reqwest`, `axum`, `serde`, … | ✅ working, 28 unit tests |

`auth` is not a default. It is the crate's only surface and its one consumer always asks for it, but the tree it pulls in is heavy enough to be explicit about.

## Honest status

| Surface | Status |
| --- | --- |
| **Rust `auth`** | ✅ Working — OAuth PKCE localhost-callback (387 LOC), password grant, refresh with rotation, M2M, 0600 `CredentialsStore`; 28 unit tests; consumed by the `th` CLI in production |
| **Rust `llm`** | ❌ Not built — no module, no feature flag. Pearl th-f7b20f tracks it |
| **crates.io** | ❌ Not published — git dependency is the only install path |
| **npm / NuGet / PyPI** | 📦 Planned, no code — there is no `src/`, `dotnet/` or `python/` directory |

## Layout

```
client-shared/
└── rust/                  # smooai-client-shared (git dependency; crates.io planned)
    ├── Cargo.toml
    └── src/
        ├── lib.rs
        └── auth/          # oauth · password · refresh · m2m · storage  (feature = "auth")
```

npm (`src/`), NuGet (`dotnet/`), and PyPI (`python/`) packages are roadmap, not directories.

## Related repos

- [`SmooAI/ui`](https://github.com/SmooAI/ui) — **the design system**: tokens, base CSS, the smoo monogram. Consumed directly by [smooblue](https://github.com/SmooAI/smooblue) and observability-studio. This crate used to carry a copy; it no longer does.
- [`SmooAI/smooth`](https://github.com/SmooAI/smooth) — the `th` CLI; consumes this crate (`features = ["auth"]`) for login + credential storage.

## 🧩 Part of Smoo AI

`@smooai/client-shared` is built and open-sourced by **[Smoo AI](https://smoo.ai)** — the AI-powered business platform with AI built into every product: CRM, customer support, campaigns, field service, observability, and developer tools.

- 🧰 **More open source from Smoo AI** — [smoo.ai/open-source](https://smoo.ai/open-source)
- 🧩 **Sibling repos** — [ui](https://github.com/SmooAI/ui) (the design-system-only crate), [smooth](https://github.com/SmooAI/smooth) (the `th` CLI), [@smooai/config](https://github.com/SmooAI/config), [@smooai/logger](https://github.com/SmooAI/logger)

## 🤝 Contributing

PRs welcome. `cd rust && cargo test --all-features` must pass; keep the bare `ui` build zero-dep and `no_std`, and gate anything heavier behind a feature flag.

## 📄 License

MIT — see [`LICENSE`](LICENSE).

---

<p align="center">
  Built by <a href="https://smoo.ai"><strong>Smoo AI</strong></a> — AI built into every product.
</p>
