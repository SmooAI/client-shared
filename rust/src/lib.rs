//! # smooai-client-shared
//!
//! Auth primitives shared across SmooAI's Rust clients — Supabase user
//! OAuth (localhost-callback flow with PKCE), the M2M
//! `client_credentials` grant, refresh-token rotation, and an on-disk
//! `CredentialsStore`. Consumed by `th` and `smoo admin` (the Smooth
//! CLI).
//!
//! ## Feature flags
//!
//! - `auth` — everything above. Pulls in `tokio`, `reqwest`, `serde`,
//!   `axum`. Not a default: the dependency tree is heavy enough that it
//!   should be asked for explicitly.
//!
//! ## What happened to the `ui` module
//!
//! This crate used to carry a copy of the design system (tokens, base
//! CSS, monogram) and describe itself as `smooai-ui`'s successor. That
//! migration was never finished, and an org-wide search found **nothing
//! importing `smooai_client_shared::ui`** — every real consumer of the
//! design system depends on [`SmooAI/ui`](https://github.com/SmooAI/ui)
//! directly (smooblue, observability-studio), while this crate's only
//! consumer asks for `auth` and never touched it.
//!
//! Two copies of the same files is a drift surface, and it had already
//! drifted: this crate spent weeks serving a monogram missing its inner
//! 'S' because a fix in SmooAI/ui never crossed. The copy is gone;
//! `SmooAI/ui` owns the design system.
//!
//! An `llm` feature (JWT → `llm.smoo.ai` org-scoped session exchange)
//! is planned under pearl th-f7b20f. It is deliberately **absent**
//! rather than stubbed: a feature flag that compiles to an empty
//! module advertises a capability that does not exist. It comes back
//! when there is something behind it.

#![cfg_attr(not(feature = "auth"), no_std)]
#![doc(html_root_url = "https://docs.rs/smooai-client-shared/0.1.0")]
#![warn(missing_docs)]

#[cfg(feature = "auth")]
pub mod auth;
