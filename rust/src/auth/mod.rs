//! # auth — Supabase user OAuth + M2M client-credentials + on-disk credential store
//!
//! Two distinct authentication flows, one shared on-disk store:
//!
//! - **`oauth`** — Supabase **user** OAuth via localhost-callback. The CLI
//!   spawns a tiny axum listener on a random localhost port, opens the
//!   user's browser to the Supabase authorize endpoint with that port as
//!   the redirect target, captures the returned session JWT, and persists
//!   it. The user is identified by their Smoo account (email / Google /
//!   GitHub / whatever Supabase has wired). Permission scope: `requireUser`
//!   on the backend (admin endpoints additionally check `requireSuperAdmin`).
//!
//! - **`m2m`** — RFC 6749 `client_credentials` grant against
//!   `https://auth.smoo.ai/token`. Service accounts (CI, customer-website
//!   SSR, etc.) mint a client_id / client_secret pair from the Smoo web app
//!   and exchange them for an org-scoped bearer. Permission scope: whatever
//!   the org grants the client (typically just its own org's resources).
//!
//! - **`storage`** — The shared on-disk `CredentialsStore` for both flows.
//!   By convention user-OAuth tokens go to `~/.smooth/auth/smooai-user.json`
//!   and M2M tokens go to `~/.smooth/auth/smooai.json`, so a single host
//!   can carry both simultaneously without collision. Both files are
//!   written with mode 0600.

pub mod m2m;
pub mod oauth;
pub mod password;
pub mod storage;

pub use storage::{Credentials, CredentialsStore};
