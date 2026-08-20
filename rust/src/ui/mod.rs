//! # ui — SmooAI design system primitives
//!
//! Lifted verbatim from the standalone `smooai-ui` crate so existing
//! consumers can migrate by changing `smooai_ui::*` → `smooai_client_shared::ui::*`.
//! Pure `pub const &'static str` constants — zero deps, `no_std`-friendly.

/// Canonical brand stylesheet, sourced from the cross-language
/// `shared/styles.css`. Embed in your app's root component so every
/// consumer sees the same tokens + base component classes.
pub const STYLES: &str = include_str!("../../../shared/styles.css");

/// The smoo monogram, as an SVG string with `fill="currentColor"` so
/// the surrounding CSS controls the color. Pair with the
/// `.brand-badge` class (defined in [`STYLES`]) for the gradient pill
/// backdrop.
pub const MONOGRAM_SVG: &str = include_str!("../../../shared/monogram.svg");

/// Brand + semantic token *values*, for code paths that need a colour,
/// radius, spacing step, or font stack outside of CSS (custom-painted egui
/// widgets, native menu chrome, chart libraries, etc.).
///
/// **Generated** at build time from
/// [`shared/tokens.json`](https://github.com/SmooAI/client-shared/blob/main/shared/tokens.json)
/// by `shared/tokens_codegen.rs` — nobody hand-writes these, so a token cannot
/// exist in the design system and be missing here. `shared/tokens_css_check.rs`
/// then asserts each one equals the resolved value of its custom property in
/// [`STYLES`], and that every colour the CSS declares has a token.
pub mod tokens {
    include!(concat!(env!("OUT_DIR"), "/tokens.rs"));

    /// The default corner radius, as used by cards and buttons. Retained as an
    /// alias of [`RADIUS_MD_PX`] so consumers pinned to the pre-codegen name
    /// keep compiling.
    pub const RADIUS_PX: u16 = RADIUS_MD_PX;
}

#[cfg(test)]
mod tests {
    extern crate std;
    use super::*;

    #[test]
    fn styles_is_nonempty() {
        assert!(STYLES.len() > 100);
        assert!(STYLES.contains("--color-smooai-green"));
    }

    #[test]
    fn monogram_is_real_svg() {
        assert!(MONOGRAM_SVG.starts_with("<svg"));
        assert!(MONOGRAM_SVG.contains("viewBox=\"0 0 135 135\""));
        assert!(MONOGRAM_SVG.contains("fill=\"currentColor\""));
    }

    // The token <-> CSS cross-check (both directions) lives in `shared/`, so
    // SmooAI/client-shared and SmooAI/ui run the identical assertions.
    include!("../../../shared/tokens_css_check.rs");

    #[test]
    fn semantic_classes_exist() {
        for cls in [
            ".btn",
            ".btn--primary",
            ".btn--ghost",
            ".card",
            ".fab",
            ".modal__sheet",
            ".rail",
            ".rail__btn",
            ".brand-badge",
            ".input",
            ".input--lg",
            ".input-error",
            ".input-hint",
        ] {
            assert!(STYLES.contains(cls), "missing class {cls}");
        }
    }
}
