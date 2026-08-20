// Cross-check the generated `tokens` constants against `shared/styles.css`.
//
// `include!`d from each crate's `#[cfg(test)] mod tests`. Lives in `shared/`
// so SmooAI/ui and SmooAI/client-shared run the identical check — the
// `shared/**` drift gate keeps the copies byte-identical.
//
// What this replaces: the old `tokens_match_css` asserted only that each
// hand-written Rust constant appeared *somewhere* in the CSS as a substring.
// That passes even when the constant is attached to the wrong custom property,
// and it is structurally blind to a token the CSS declares and Rust omits —
// nothing walks the CSS. This parses `:root`, resolves `var(--x)` references,
// and checks BOTH directions.
//
// Scope of the completeness half: every **colour-valued** custom property (one
// that resolves to `oklch(...)`) must be a generated token. Non-colour geometry
// vars that `tokens.json` does not claim to cover (`--rail-width`,
// `--status-bar-height`) are deliberately out of scope.

// The `ui` slice compiles `no_std`, so `String`/`Vec`/`format!` are not in the
// prelude here. The enclosing test module has already done `extern crate std`.
use std::{
    string::{String, ToString},
    vec::Vec,
};

/// Strip `/* … */` comments so a trailing hex annotation can't be read as value text.
fn strip_css_comments(css: &str) -> String {
    let mut out = String::with_capacity(css.len());
    let mut rest = css;
    while let Some(start) = rest.find("/*") {
        out.push_str(&rest[..start]);
        match rest[start..].find("*/") {
            Some(end) => rest = &rest[start + end + 2..],
            None => return out,
        }
    }
    out.push_str(rest);
    out
}

/// Collapse every run of whitespace to a single space and trim, so a
/// multi-line CSS value compares equal to its single-line JSON twin.
fn normalize(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Parse the `:root { … }` block into `(--custom-property, raw value)` pairs.
fn parse_root_vars(css: &str) -> Vec<(String, String)> {
    let css = strip_css_comments(css);
    let start = css.find(":root").expect("shared/styles.css has no :root block");
    let body_start = start + css[start..].find('{').expect(":root has no opening brace") + 1;
    let body_len = css[body_start..]
        .find('}')
        .expect(":root has no closing brace");
    let body = &css[body_start..body_start + body_len];

    body.split(';')
        .filter_map(|decl| {
            let (name, value) = decl.split_once(':')?;
            let name = name.trim();
            name.starts_with("--")
                .then(|| (name.to_string(), normalize(value)))
        })
        .collect()
}

/// Resolve `var(--x)` references against the parsed table, to a fixed point.
/// Bounded by the number of variables, so a reference cycle errors rather than
/// hanging.
fn resolve(name: &str, vars: &[(String, String)], depth: usize) -> String {
    assert!(
        depth <= vars.len(),
        "shared/styles.css: var() reference cycle reaching {name}"
    );
    let raw = vars
        .iter()
        .find(|(n, _)| n == name)
        .map(|(_, v)| v.clone())
        .unwrap_or_else(|| panic!("shared/styles.css: :root does not define {name}"));

    let mut out = String::with_capacity(raw.len());
    let mut rest = raw.as_str();
    while let Some(at) = rest.find("var(") {
        out.push_str(&rest[..at]);
        let after = &rest[at + 4..];
        let close = after
            .find(')')
            .unwrap_or_else(|| panic!("shared/styles.css: unclosed var() in {name}"));
        out.push_str(&resolve(after[..close].trim(), vars, depth + 1));
        rest = &after[close + 1..];
    }
    out.push_str(rest);
    normalize(&out)
}

/// Every generated token's value must equal the resolved value of the CSS
/// custom property it claims to mirror — not merely appear somewhere in the file.
#[test]
fn tokens_match_css() {
    let vars = parse_root_vars(STYLES);
    for (rust_name, css_var, expected) in tokens::ALL {
        let actual = resolve(css_var, &vars, 0);
        assert_eq!(
            &actual,
            &normalize(expected),
            "token {rust_name} ({css_var}) disagrees with shared/styles.css — \
             tokens.json says {expected:?}, the CSS resolves to {actual:?}",
        );
    }
}

/// The direction the old substring check could not see: a colour the CSS
/// declares that `tokens.json` never mentions, so no binding in any language
/// ever gets it.
#[test]
fn css_colors_are_all_tokens() {
    let vars = parse_root_vars(STYLES);
    for (name, _) in &vars {
        if !resolve(name, &vars, 0).starts_with("oklch(") {
            continue;
        }
        assert!(
            tokens::ALL.iter().any(|(_, css_var, _)| *css_var == name.as_str()),
            "shared/styles.css declares the colour {name} but shared/tokens.json \
             has no token for it — add it to tokens.json so every language binding \
             gets it (the Rust constants are generated from that file)",
        );
    }
}

/// `ALL` is generated alongside the constants; if that ever stops being true
/// the two checks above go quietly vacuous, so assert it is populated.
#[test]
fn token_table_is_populated() {
    assert!(
        tokens::ALL.len() >= 20,
        "tokens::ALL has only {} entries — codegen produced an near-empty table",
        tokens::ALL.len()
    );
}
