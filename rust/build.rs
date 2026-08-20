//! Generate the `tokens` module from `shared/tokens.json`.
//!
//! The generator itself lives in `shared/tokens_codegen.rs` so SmooAI/ui and
//! SmooAI/client-shared generate identically from the identical input — the
//! `shared/**` drift gate keeps those files byte-for-byte equal. This build
//! script is only the shim that feeds it.

include!("../shared/tokens_codegen.rs");

fn main() {
    println!("cargo:rerun-if-changed=../shared/tokens.json");
    println!("cargo:rerun-if-changed=../shared/tokens_codegen.rs");

    let json = std::fs::read_to_string("../shared/tokens.json").expect("read shared/tokens.json");
    let out = std::path::Path::new(&std::env::var("OUT_DIR").expect("OUT_DIR")).join("tokens.rs");
    std::fs::write(out, generate_tokens_rs(&json)).expect("write tokens.rs");
}
