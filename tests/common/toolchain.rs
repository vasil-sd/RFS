//! Shared external-tool resolution for the test suites (Phase 12 portability).
//!
//! Every test that shells out to a compiler or diagram tool MUST resolve it
//! through this module instead of a hardcoded path or a bare `Command::new(
//! "gcc")`. The resolver reuses the CLI's `ExtTool` precedence so tests and the
//! production binary agree by construction:
//!
//!     CLI flag > RFS_* env var > standard Unix var (CC/RUSTC/CARGO) > bare name on PATH
//!
//! This is what lets the suites run on NixOS / nix-shell, where no tool lives at
//! a standard absolute path: the shell exports `CC`/`RUSTC` (or the user sets
//! `RFS_GCC` etc.) and every invocation picks it up. The `available()` guard
//! mirrors the resolver exactly - it reports a tool present if ANY tier would
//! resolve - so a test is never skipped just because the bare name is absent
//! from PATH while an env override supplies it.
//!
//! Tools with no `ExtTool` preset (make, plain cargo) get a small env-or-bare
//! resolver of their own. z3 has its own discovery chain in `z3_dir()`.
//
// Each integration test is its own crate and pulls in this whole module via
// `mod common;`, so functions a given suite doesn't call would otherwise warn
// as dead code - expected for a shared helper, allowed here.
#![allow(dead_code)]

use rfs::cli::ext_tool::ExtTool;

/// Resolve the C compiler path: `RFS_GCC` > `CC` > bare `gcc` on PATH.
pub fn gcc() -> String {
    ExtTool::GCC.resolve(None, None).path
}

/// Resolve the Rust compiler path: `RFS_RUSTC` > `RUSTC` > bare `rustc` on PATH.
pub fn rustc() -> String {
    ExtTool::RUSTC.resolve(None, None).path
}

/// Resolve clang (libFuzzer host): `RFS_CLANG` > bare `clang` on PATH. No
/// standard tier - libFuzzer needs clang specifically, so `CC` must not leak in.
pub fn clang() -> String {
    ExtTool::CLANG.resolve(None, None).path
}

/// Resolve make: `RFS_MAKE` > bare `make` on PATH (no universal standard var).
pub fn make() -> String {
    std::env::var("RFS_MAKE")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "make".to_string())
}

/// Resolve plain cargo: `CARGO` > bare `cargo` on PATH. (Not the CARGO_FUZZ
/// preset - that one is fuzz-subcommand-specific.)
pub fn cargo() -> String {
    std::env::var("CARGO")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "cargo".to_string())
}

/// True if `name` resolves to an executable file somewhere on `$PATH`.
pub fn on_path(name: &str) -> bool {
    std::env::var_os("PATH")
        .map(|paths| std::env::split_paths(&paths).any(|dir| dir.join(name).is_file()))
        .unwrap_or(false)
}

/// Env-aware availability for a tool by its bare name. Maps the known tools to
/// their `ExtTool` presets (so `RFS_DOT`/`RFS_PLANTUML`/`RFS_MERMAID`/`CC`/...
/// count as present) and falls back to a plain PATH scan for anything else.
/// Drop-in replacement for the old PATH-only `tool_on_path` guards.
pub fn available_named(name: &str) -> bool {
    let tool = match name {
        "dot" => Some(ExtTool::DOT),
        "plantuml" => Some(ExtTool::PLANTUML),
        "mmdc" => Some(ExtTool::MERMAID),
        "gcc" => Some(ExtTool::GCC),
        "rustc" => Some(ExtTool::RUSTC),
        "clang" => Some(ExtTool::CLANG),
        _ => None,
    };
    match tool {
        Some(t) => available(t),
        None => on_path(name),
    }
}

/// True if a tool resolved by a single env var (or its bare-name fallback) is
/// present - for tools with no `ExtTool` preset (make). Mirrors the resolver:
/// a non-empty `env_var` counts as present; otherwise the bare name on PATH.
pub fn available_env_or_path(env_var: &str, bare_name: &str) -> bool {
    if std::env::var(env_var)
        .ok()
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false)
    {
        return true;
    }
    on_path(bare_name)
}

/// True if `tool` is usable - i.e. the resolver would produce a path. Mirrors
/// `ExtTool::resolve`: an explicit override (RFS_* or standard var) set to a
/// non-empty value counts as present; otherwise the bare name must be on PATH.
pub fn available(tool: ExtTool) -> bool {
    if std::env::var(tool.env_path_var)
        .ok()
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false)
    {
        return true;
    }
    if let Some(sv) = tool.std_env_var {
        if std::env::var(sv)
            .ok()
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false)
        {
            return true;
        }
    }
    on_path(tool.name)
}

/// True when `dir` looks like a usable z3 install (headers or a lib dir).
fn z3_install_ok(dir: &str) -> bool {
    let d = std::path::Path::new(dir);
    d.join("include").join("z3.h").is_file() || d.join("bin").is_dir() || d.join("lib").is_dir()
}

/// An env var holding a z3 install dir, accepted only if it is set AND points at
/// a real install. The existence check is what makes a config-supplied default
/// safe on a foreign system: a stale `RFS_Z3_DIR` that does not exist there is
/// skipped rather than trusted, so resolution falls through to pkg-config.
fn z3_env_dir(var: &str) -> Option<String> {
    std::env::var(var)
        .ok()
        .filter(|s| !s.trim().is_empty())
        .filter(|p| z3_install_ok(p))
}

/// Resolve the z3 install dir + whether it came from an explicit env var
/// (`true`) or pkg-config auto-discovery (`false`). `None` when z3 is
/// unavailable.
///
/// Precedence: `RFS_Z3_DIR` > `Z3_SYS_Z3_DIR` > pkg-config `z3`. No absolute
/// path is baked into source - the project-local install is supplied via
/// `RFS_Z3_DIR` in `.cargo/config.toml` (see TOOLCHAIN.md section 8).
pub fn z3() -> Option<(String, bool)> {
    if let Some(p) = z3_env_dir("RFS_Z3_DIR") {
        return Some((p, true));
    }
    if let Some(p) = z3_env_dir("Z3_SYS_Z3_DIR") {
        return Some((p, true));
    }
    // pkg-config auto-discovery (finds a nixpkgs / system z3).
    if let Ok(out) = std::process::Command::new("pkg-config")
        .args(["--variable=prefix", "z3"])
        .output()
    {
        if out.status.success() {
            let prefix = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !prefix.is_empty() && z3_install_ok(&prefix) {
                return Some((prefix, false));
            }
        }
    }
    None
}

/// The resolved z3 install dir, or `None` when z3 is unavailable.
pub fn z3_dir() -> Option<String> {
    z3().map(|(p, _)| p)
}

/// True when z3 is discoverable at all.
pub fn z3_available() -> bool {
    z3().is_some()
}
