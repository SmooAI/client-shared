//! # smooai-client-shared
//!
//! Cross-runtime shared library for SmooAI Rust clients — the home for
//! design system primitives, auth flows, and LLM session plumbing that
//! every SmooAI Rust app needs identically. Consumed by `smooblue`
//! (Dioxus desktop), `observability-studio` (Dioxus viewer), `th` and
//! `smoo admin` (the Smooth CLI), and any future Rust client.
//!
//! Replaces the standalone `smooai-ui` crate (which only carried the
//! `ui` slice) by adding an `auth` module behind a feature flag. The bare `default-features = ["ui"]` build stays
//! `no_std`-compatible with zero runtime dependencies — same shape as
//! the old `smooai-ui` so existing consumers don't inherit any new
//! tree.
//!
//! ## Feature flags
//!
//! - `ui` (default) — design tokens, base CSS, monogram. Zero deps,
//!   `no_std`.
//! - `auth` — Supabase user OAuth (localhost-callback flow), M2M
//!   `client_credentials` grant, refresh-token rotation, on-disk
//!   `CredentialsStore`. Pulls in `tokio`, `reqwest`, `serde`, `axum`.
//!
//! An `llm` feature (JWT → `llm.smoo.ai` org-scoped session exchange)
//! is planned under pearl th-f7b20f. It is deliberately **absent**
//! rather than stubbed: a feature flag that compiles to an empty
//! module advertises a capability that does not exist. It comes back
//! when there is something behind it.
//!
//! ## Migrating from `smooai-ui`
//!
//! Replace
//!
//! ```toml
//! smooai-ui = "0.1"
//! ```
//!
//! with
//!
//! ```toml
//! smooai-client-shared = "0.1"
//! ```
//!
//! and swap `smooai_ui::` → `smooai_client_shared::ui::` in your
//! imports. Everything in the `ui` module is re-exported at the same
//! path it lived at under `smooai_ui::` (e.g. `STYLES`,
//! `MONOGRAM_SVG`, `tokens::*`).

#![cfg_attr(not(feature = "auth"), no_std)]
#![doc(html_root_url = "https://docs.rs/smooai-client-shared/0.1.0")]
#![warn(missing_docs)]

#[cfg(feature = "ui")]
pub mod ui;

#[cfg(feature = "auth")]
pub mod auth;
