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

/// Brand + semantic token *values* as `&'static str`, for code paths
/// that need a colour outside of CSS (custom-painted egui widgets,
/// native menu chrome, chart libraries, etc.). The single source of
/// truth is the CSS in [`STYLES`]; these constants are mirrored from
/// it and validated by the `tokens_match_css` test below.
pub mod tokens {
    /// Brand orange — primary CTA stop, accent.
    pub const SMOOAI_ORANGE: &str = "oklch(0.769 0.164 71)";
    /// Brand red — destructive, "like" affordance.
    pub const SMOOAI_RED: &str = "oklch(0.712 0.181 22.4)";
    /// Brand green — confirm / "repost" affordance / focus ring.
    pub const SMOOAI_GREEN: &str = "oklch(0.657 0.112 194.8)";
    /// Lightest brand blue — reply hover.
    pub const SMOOAI_BLUE_300: &str = "oklch(0.803 0.074 230.9)";
    /// Mid brand blue.
    pub const SMOOAI_BLUE_400: &str = "oklch(0.725 0.102 233.4)";
    /// Active nav-rail item background.
    pub const SMOOAI_BLUE_500: &str = "oklch(0.55 0.13 233)";
    /// Brand dark blue — base background.
    pub const SMOOAI_DARK_BLUE: &str = "oklch(0.13 0.043 265.1)";
    /// Sidebar tint.
    pub const SMOOAI_DARK_BLUE_850: &str = "oklch(0.177 0.074 266)";
    /// Hover on rail items.
    pub const SMOOAI_DARK_BLUE_700: &str = "oklch(0.303 0.154 265.8)";
    /// Brand white.
    pub const SMOOAI_WHITE: &str = "oklch(0.984 0.003 247.9)";
    /// Muted text.
    pub const SMOOAI_GRAY_400: &str = "oklch(0.715 0 89.9)";

    /// Default page background.
    pub const BACKGROUND: &str = "oklch(0.145 0.014 265)";
    /// Default text color.
    pub const FOREGROUND: &str = SMOOAI_WHITE;
    /// Card surface background.
    pub const CARD: &str = "oklch(0.205 0.015 265)";
    /// Border / divider.
    pub const BORDER: &str = "oklch(0.3 0.008 260)";

    /// Signature brand gradient — gradient-as-string for `background: …;`.
    /// Note that CSS doesn't accept gradients in inline `color:` — this is
    /// for `background` and SVG paint only.
    pub const GRADIENT_BRAND: &str =
        "linear-gradient(135deg, oklch(0.769 0.164 71) 0%, oklch(0.712 0.181 22.4) 100%)";

    /// Default radius for cards + buttons (matches `--radius` in CSS).
    pub const RADIUS_PX: u16 = 10;
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

    /// Validate that every token value in the `tokens` module is also
    /// present in the CSS, so the two sources of truth can't silently
    /// drift.
    #[test]
    fn tokens_match_css() {
        let css = STYLES;
        for (name, value) in [
            ("SMOOAI_ORANGE", tokens::SMOOAI_ORANGE),
            ("SMOOAI_RED", tokens::SMOOAI_RED),
            ("SMOOAI_GREEN", tokens::SMOOAI_GREEN),
            ("SMOOAI_BLUE_300", tokens::SMOOAI_BLUE_300),
            ("SMOOAI_BLUE_400", tokens::SMOOAI_BLUE_400),
            ("SMOOAI_BLUE_500", tokens::SMOOAI_BLUE_500),
            ("SMOOAI_DARK_BLUE", tokens::SMOOAI_DARK_BLUE),
            ("SMOOAI_DARK_BLUE_850", tokens::SMOOAI_DARK_BLUE_850),
            ("SMOOAI_DARK_BLUE_700", tokens::SMOOAI_DARK_BLUE_700),
            ("SMOOAI_WHITE", tokens::SMOOAI_WHITE),
            ("SMOOAI_GRAY_400", tokens::SMOOAI_GRAY_400),
            ("BACKGROUND", tokens::BACKGROUND),
            ("CARD", tokens::CARD),
            ("BORDER", tokens::BORDER),
        ] {
            assert!(
                css.contains(value),
                "token {name} = {value:?} not found in shared/styles.css — Rust \
                 constants and CSS have drifted",
            );
        }
    }

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
        ] {
            assert!(STYLES.contains(cls), "missing class {cls}");
        }
    }
}
