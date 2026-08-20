// Generator for the Rust `tokens` module, from `shared/tokens.json`.
//
// This file lives in `shared/` — not in either crate's `src/` — because both
// SmooAI/ui and SmooAI/client-shared carry the same design system, and the
// `shared/**` drift gate keeps them byte-identical. Generating the constants
// rather than hand-mirroring them is the whole point: a token can no longer
// exist in `tokens.json` and be missing from Rust, because nobody types the
// constants at all.
//
// `include!`d by each crate's `rust/build.rs`. Host-side only (build script),
// so `std` and `serde_json` are available here even though the crate itself
// is `no_std` with no runtime dependencies.

use serde_json::Value;
use std::fmt::Write as _;

/// A token as it lands in generated Rust: the constant name, the CSS custom
/// property it must agree with, and the rendered Rust literal.
struct Token {
    rust: String,
    css_var: String,
    /// The Rust expression — a quoted string, an integer, or an alias to
    /// another constant.
    expr: String,
    /// The Rust type. `&str` for strings, `u16` for pixel values.
    ty: &'static str,
    /// The value as it should appear in the CSS, once `var(--x)` references
    /// are resolved. `None` for aliases, which the CSS states as `var(--x)`
    /// and whose resolved value is the target's — checked via the target.
    css_value: String,
    doc: String,
}

fn screaming(key: &str) -> String {
    key.to_uppercase().replace('-', "_")
}

fn kebab(key: &str) -> String {
    key.replace('_', "-")
}

fn obj<'a>(root: &'a Value, path: &str) -> &'a serde_json::Map<String, Value> {
    let mut node = root;
    for part in path.split('.') {
        node = node
            .get(part)
            .unwrap_or_else(|| panic!("tokens.json: missing section {path:?}"));
    }
    node.as_object()
        .unwrap_or_else(|| panic!("tokens.json: section {path:?} is not an object"))
}

fn require_str<'a>(entry: &'a Value, field: &str, path: &str) -> &'a str {
    entry
        .get(field)
        .and_then(Value::as_str)
        .unwrap_or_else(|| panic!("tokens.json: {path} is missing a string {field:?}"))
}

/// Resolve a dotted `ref` path (e.g. `color.brand.white`) to the constant name
/// it maps to under the naming conventions.
fn ref_to_const(path: &str) -> String {
    let (section, key) = path
        .rsplit_once('.')
        .unwrap_or_else(|| panic!("tokens.json: malformed ref {path:?}"));
    match section {
        "color.brand" => format!("SMOOAI_{}", screaming(key)),
        "color.semantic" => screaming(key),
        other => panic!("tokens.json: ref into unsupported section {other:?}"),
    }
}

fn collect(root: &Value) -> Vec<Token> {
    let mut tokens = Vec::new();

    // Colours. Brand tokens are literal OKLCH; semantic tokens are either
    // literal or a `ref` to another token (rendered as a Rust alias and
    // expected to be `var(--other)` in the CSS).
    for (section, prefix) in [("color.brand", "SMOOAI_"), ("color.semantic", "")] {
        for (key, entry) in obj(root, section) {
            let path = format!("{section}.{key}");
            let doc = require_str(entry, "doc", &path).to_string();
            let rust = format!("{prefix}{}", screaming(key));
            let css_var = if section == "color.brand" {
                format!("--color-smooai-{}", kebab(key))
            } else {
                format!("--{}", kebab(key))
            };
            let (expr, css_value) = match entry.get("ref").and_then(Value::as_str) {
                Some(target) => {
                    let alias = ref_to_const(target);
                    // A ref's CSS value is its target's, so record the target's
                    // literal for the cross-check by resolving it here.
                    let (t_section, t_key) = target.rsplit_once('.').unwrap();
                    let literal = obj(root, t_section)
                        .get(t_key)
                        .and_then(|t| t.get("oklch"))
                        .and_then(Value::as_str)
                        .map(str::to_string)
                        .unwrap_or_else(|| {
                            // A ref to a ref — resolve one more hop.
                            let next = obj(root, t_section)
                                .get(t_key)
                                .and_then(|t| t.get("ref"))
                                .and_then(Value::as_str)
                                .unwrap_or_else(|| {
                                    panic!("tokens.json: {target:?} is neither oklch nor ref")
                                });
                            let (n_section, n_key) = next.rsplit_once('.').unwrap();
                            require_str(&obj(root, n_section)[n_key], "oklch", next).to_string()
                        });
                    (alias, literal)
                }
                None => {
                    let literal = require_str(entry, "oklch", &path).to_string();
                    (format!("{literal:?}"), literal)
                }
            };
            tokens.push(Token {
                rust,
                css_var,
                expr,
                ty: "&str",
                css_value,
                doc,
            });
        }
    }

    for (key, entry) in obj(root, "gradient") {
        let path = format!("gradient.{key}");
        let value = require_str(entry, "value", &path).to_string();
        tokens.push(Token {
            rust: format!("GRADIENT_{}", screaming(key)),
            css_var: format!("--gradient-{}", kebab(key)),
            expr: format!("{value:?}"),
            ty: "&str",
            css_value: value,
            doc: require_str(entry, "doc", &path).to_string(),
        });
    }

    for (key, entry) in obj(root, "radius") {
        let path = format!("radius.{key}");
        let px = entry
            .get("px")
            .and_then(Value::as_u64)
            .unwrap_or_else(|| panic!("tokens.json: {path} is missing an integer \"px\""));
        tokens.push(Token {
            rust: format!("RADIUS_{}_PX", screaming(key)),
            // The default radius is the bare `--radius`, not `--radius-md`.
            css_var: if key == "md" {
                "--radius".to_string()
            } else {
                format!("--radius-{}", kebab(key))
            },
            expr: px.to_string(),
            ty: "u16",
            css_value: format!("{px}px"),
            doc: require_str(entry, "doc", &path).to_string(),
        });
    }

    for (key, entry) in obj(root, "space") {
        let path = format!("space.{key}");
        let px = entry
            .get("px")
            .and_then(Value::as_u64)
            .unwrap_or_else(|| panic!("tokens.json: {path} is missing an integer \"px\""));
        tokens.push(Token {
            rust: format!("SPACE_{}_PX", screaming(key)),
            css_var: format!("--space-{}", kebab(key)),
            expr: px.to_string(),
            ty: "u16",
            css_value: format!("{px}px"),
            doc: require_str(entry, "doc", &path).to_string(),
        });
    }

    for (key, entry) in obj(root, "font") {
        let path = format!("font.{key}");
        let value = require_str(entry, "value", &path).to_string();
        tokens.push(Token {
            rust: format!("FONT_{}", screaming(key)),
            css_var: format!("--font-{}", kebab(key)),
            expr: format!("{value:?}"),
            ty: "&str",
            css_value: value,
            doc: require_str(entry, "doc", &path).to_string(),
        });
    }

    tokens
}

/// Render `shared/tokens.json` as the body of the crate's `tokens` module.
///
/// Emits one `pub const` per token plus `ALL`, the complete
/// `(rust_name, css_var, css_value)` table that `shared/tokens_css_check.rs`
/// walks. `ALL` is generated alongside the constants, so it cannot fall behind
/// them — that is what makes the "CSS has a token Rust omitted" case
/// detectable, which the old hand-written list could not do.
pub fn generate_tokens_rs(json: &str) -> String {
    let root: Value = serde_json::from_str(json).expect("shared/tokens.json is not valid JSON");
    let tokens = collect(&root);

    let mut out = String::from(
        "// @generated by shared/tokens_codegen.rs from shared/tokens.json — do not edit.\n\
         // Add or change a token in shared/tokens.json; this file follows.\n\n",
    );

    for t in &tokens {
        let _ = writeln!(out, "/// {}", t.doc);
        let _ = writeln!(out, "pub const {}: {} = {};", t.rust, t.ty, t.expr);
    }

    let _ = writeln!(
        out,
        "\n/// Every generated token as `(rust_name, css_var, css_value)`, for the\n\
         /// `shared/tokens_css_check.rs` cross-check against `shared/styles.css`.\n\
         /// Generated with the constants above, so it is complete by construction.\n\
         pub const ALL: &[(&str, &str, &str)] = &["
    );
    for t in &tokens {
        let _ = writeln!(
            out,
            "    ({:?}, {:?}, {:?}),",
            t.rust, t.css_var, t.css_value
        );
    }
    let _ = writeln!(out, "];");

    out
}
