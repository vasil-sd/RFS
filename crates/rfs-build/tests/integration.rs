//! End-to-end: run `rfs-build` against a REAL spec with the real workspace
//! `rfs` binary (built by cargo as part of `cargo test`) and prove the
//! generated artifact compiles. The hermetic stub-binary tests live in the
//! lib unit tests; this file is the one real end-to-end check.
//! Docs: dev:workspace-crates-overview, llm:workspace-crates

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

static ENV_LOCK: Mutex<()> = Mutex::new(());

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

/// The rfs binary cargo just built for the workspace (target/debug/rfs).
fn real_rfs_bin() -> PathBuf {
    let name = if cfg!(windows) { "rfs.exe" } else { "rfs" };
    let p = workspace_root().join("target").join("debug").join(name);
    assert!(
        p.is_file(),
        "workspace rfs binary not built at {}",
        p.display()
    );
    p
}

fn out_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("rfs_build_e2e_{}_{}", std::process::id(), tag));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn rustc() -> String {
    // cargo sets RUSTC for build scripts; fall back to PATH (the pinned
    // project toolchain, see TOOLCHAIN.md).
    std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_string())
}

#[test]
fn real_end_to_end_rust_backend_compiles() {
    let _g = ENV_LOCK.lock().unwrap();
    let out = out_dir("rust");
    let spec = workspace_root()
        .join("examples")
        .join("valid")
        .join("01_trivial_two_state.rfs");
    assert!(spec.is_file(), "missing example spec {}", spec.display());

    std::env::set_var("RFS_BIN", real_rfs_bin());
    std::env::set_var("OUT_DIR", &out);

    let artifact = rfs_build::RfsBuild::new(&spec)
        .backend("rust")
        .production()
        .compile()
        .unwrap_or_else(|e| panic!("rfs-build failed: {}", e.message));

    assert_eq!(artifact, out.join("01_trivial_two_state.rs"));
    let text = std::fs::read_to_string(&artifact).expect("artifact readable");
    assert!(!text.is_empty());
    assert!(
        text.contains("pub enum State"),
        "generated code should contain the state enum"
    );

    // The consumer contract: the generated file compiles standalone.
    let rlib = out.join("probe.rlib");
    let status = Command::new(rustc())
        .args(["--edition", "2021", "--crate-type", "lib", "-o"])
        .arg(&rlib)
        .arg(&artifact)
        .status()
        .expect("spawn rustc (project toolchain on PATH, see TOOLCHAIN.md)");
    assert!(status.success(), "rustc failed on the generated artifact");
    assert!(rlib.is_file());

    std::env::remove_var("RFS_BIN");
    std::env::remove_var("OUT_DIR");
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn real_end_to_end_c_backend_generates_c_file() {
    let _g = ENV_LOCK.lock().unwrap();
    let out = out_dir("c");
    let spec = workspace_root()
        .join("examples")
        .join("valid")
        .join("01_trivial_two_state.rfs");

    std::env::set_var("RFS_BIN", real_rfs_bin());
    std::env::set_var("OUT_DIR", &out);

    let artifact = rfs_build::RfsBuild::new(&spec)
        .backend("c")
        .compile()
        .unwrap_or_else(|e| panic!("rfs-build failed: {}", e.message));

    assert_eq!(artifact, out.join("01_trivial_two_state.c"));
    let text = std::fs::read_to_string(&artifact).expect("artifact readable");
    assert!(
        text.contains("#include"),
        "generated C should include headers"
    );

    std::env::remove_var("RFS_BIN");
    std::env::remove_var("OUT_DIR");
    let _ = std::fs::remove_dir_all(&out);
}
