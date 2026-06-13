<p align="center">
  <a href="https://smoo.ai"><img src=".github/banner.png" alt="@smooai/client-shared — Shared primitives for every Smoo AI client" width="100%" /></a>
</p>

<p align="center">
  <a href="https://smoo.ai/open-source"><img src="https://img.shields.io/badge/Smoo_AI-platform-00A6A6?style=for-the-badge&labelColor=020618" alt="Smoo AI"></a>
  <img src="https://img.shields.io/badge/license-MIT-F49F0A?style=for-the-badge&labelColor=020618" alt="license">
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Rust-reference%20impl-FF6B6C?style=flat-square" alt="Rust reference implementation">
</p>

<p align="center">
  <a href="#why-one-crate-not-many"><b>Features</b></a> &nbsp;·&nbsp; <a href="#install"><b>Install</b></a> &nbsp;·&nbsp; <a href="#feature-flags"><b>Usage</b></a> &nbsp;·&nbsp; <a href="#migrating-from-smooai-ui"><b>Migrating</b></a> &nbsp;·&nbsp; <a href="#part-of-smoo-ai"><b>Platform</b></a>
</p>

---

> The single source of truth for the things every Smoo AI client — `smooblue`, observability-studio, the `th` CLI — reimplements otherwise. This repo absorbs and supersedes the standalone [`@smooai/ui`](https://github.com/SmooAI/ui) crate: `ui` is now one *module* among siblings (`auth`, `llm`, future) inside a single `smooai-client-shared` Rust crate, with matching `@smooai/client-shared` npm / NuGet / PyPI packages planned.

## Why one crate, not many

A Smoo AI Rust client (smooblue, observability-studio, `th`, `th admin`, …)
typically needs the same three things:

1. **Design tokens + monogram** — so the UI looks like Smoo AI.
2. **Auth** — Supabase user OAuth (browser login) AND M2M `client_credentials`
   grant (service accounts), with one shared on-disk `CredentialsStore`.
3. **LLM access** — exchanges a user session JWT for an org-scoped
   `llm.smoo.ai` LLM bearer so every user's spend attributes to their org's
   TPM budget.

Each of these has been re-implemented in every consumer at least once. This
crate makes them one dependency.

## Install

```toml
smooai-client-shared = { version = "0.1", features = ["ui", "auth"] }
```

The `ui` feature is the default and **inherits the same dependency tree as the
old `smooai-ui` crate** — zero runtime deps, `no_std`-compatible. Smooblue and
observability-studio can swap `smooai-ui` for `smooai-client-shared` without
pulling any new transitive dependencies.

## Feature flags

| Feature | Adds | Pulls |
|---|---|---|
| `ui` (default) | `STYLES`, `MONOGRAM_SVG`, `tokens::*` | nothing |
| `auth` | Supabase OAuth, M2M, `CredentialsStore` | `tokio`, `reqwest`, `axum`, `serde` |
| `llm` | JWT → `llm.smoo.ai` org-session exchange | (implies `auth`) |

## Layout

```
~/dev/smooai/client-shared/
├── shared/                # cross-language source of truth
│   ├── styles.css         # OKLCH tokens + base component CSS
│   ├── monogram.svg       # smoo monogram
│   └── tokens.json        # tokens as plain JSON
├── rust/                  # smooai-client-shared (crates.io)
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── ui/            # lifted from smooai-ui
│       ├── auth/          # Supabase OAuth + M2M + storage  (feature = "auth")
│       └── llm/           # JWT → LLM session exchange      (feature = "llm")
├── src/                   # @smooai/client-shared (npm) — future
├── dotnet/                # SmooAI.ClientShared (NuGet) — future
├── python/                # smooai-client-shared (PyPI) — future
└── LICENSE  README.md
```

## Migrating from `smooai-ui`

Replace

```toml
smooai-ui = "0.1"
```

with

```toml
smooai-client-shared = "0.1"   # default features include "ui" — same surface
```

and swap imports:

```rust
// before
use smooai_ui::{STYLES, MONOGRAM_SVG, tokens};

// after
use smooai_client_shared::ui::{STYLES, MONOGRAM_SVG, tokens};
```

The `ui` module is API-compatible with `smooai-ui` 0.1 — every const and
sub-module lives at the same relative path under `ui::`.

For one release cycle, the legacy `smooai-ui` crate continues to publish as a
thin re-export shim (`pub use smooai_client_shared::ui::*`) with a deprecation
warning. Yanks after the cycle.

## Related repos

- [`@smooai/ui`](https://github.com/SmooAI/ui) — the original design-system-only
  crate. Will become a deprecation shim then retire.
- [`@smooai/smooth`](https://github.com/SmooAI/smooth) — the `th` CLI; consumes
  `client-shared::{auth, llm}` for login + LLM session.
- `smooblue`, `observability-studio` — Dioxus desktop apps; consume
  `client-shared::ui`.

## Part of Smoo AI

`@smooai/client-shared` is built and open-sourced by **[Smoo AI](https://smoo.ai)** — the AI-powered business platform with AI built into every product: CRM, customer support, campaigns, field service, observability, and developer tools.

- 🧰 **More open source from Smoo AI** — [smoo.ai/open-source](https://smoo.ai/open-source)
- 🧩 **Sibling repos** — [smooblue](https://github.com/SmooAI/smooblue) + observability-studio (consumers), [@smooai/smooth](https://github.com/SmooAI/smooth) (the `th` CLI), [@smooai/config](https://github.com/SmooAI/config), [@smooai/logger](https://github.com/SmooAI/logger)

## License

MIT — see [`LICENSE`](LICENSE).

---

<p align="center">
  Built by <a href="https://smoo.ai"><strong>Smoo AI</strong></a> — AI built into every product.
</p>
