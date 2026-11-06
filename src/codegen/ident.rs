//! Identifier-mangling utilities shared by BOTH language backends (the C
//! emitters borrow these for the Rust-style identifiers the monitor wire
//! format uses). Extracted from the Rust backend so no C file depends on a
//! Rust-backend module (Phase 11 task 5 - the flagged cross-backend coupling).
//! Docs: dev:codegen-core-overview

/// Rust keywords that require raw-identifier mangling (`r#...`).
pub(crate) const RUST_KEYWORDS: &[&str] = &[
    "as", "break", "const", "continue", "crate", "else", "enum", "extern", "false", "fn", "for",

    "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref", "return",
    "self", "Self", "static", "struct", "super", "trait", "true", "type", "unsafe", "use", "where",
    "while",
];

/// Mangling of an identifier for Rust: keywords become raw identifiers.
pub(crate) fn rust_ident(name: &str) -> String {
    if RUST_KEYWORDS.contains(&name) {
        format!("r#{}", name)
    } else {
        name.to_string()
    }
}

/// Mangling of a PATTERN BINDING identifier for Rust: in addition to keyword
/// mangling, bindings that collide with identifiers the generated transition
/// body uses unqualified are renamed to `<name>__rfsb` (and further suffixed
/// until unique among `used`). Without this, e.g. a field named `out` would
/// shadow mc_step_all's effect vector and every `out.push` would hit the
/// binding instead - silently wrong code from a fully valid spec (Phase 6
/// task 6 finding; the C backend has the same bug class via its effect
/// counter `n`).
pub(crate) fn rust_binding_ident(
    name: &str,
    reserved: &[&str],
    used: &mut std::collections::HashSet<String>,
) -> String {
    let base = rust_ident(name);
    let mut ident = if reserved.iter().any(|r| *r == base) {
        format!("{base}__rfsb")
    } else {
        base
    };
    while !used.insert(ident.clone()) {
        ident = format!("{ident}__rfsb");
    }
    ident
}

/// Lowercase, non-alphanumeric characters become underscores; a leading digit
/// is prefixed with an underscore. Used for generated function/variable names.
pub(crate) fn snake(name: &str) -> String {
    let mut out = String::new();
    for c in name.to_lowercase().chars() {
        if c.is_ascii_alphanumeric() || c == '_' {
            out.push(c);
        } else {
            out.push('_');
        }
    }
    if out
        .chars()
        .next()
        .map(|c| c.is_ascii_digit())
        .unwrap_or(false)
    {
        out.insert(0, '_');
    }
    if out.is_empty() {
        out.push('_');
    }
    out
}

