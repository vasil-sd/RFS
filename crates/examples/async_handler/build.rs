//! Build script: generate the spec-43 ARQ sender at build time via rfs-build
//! (Phase 5 task 11), with the opt-in `--with-protocol-fsm` flag so the
//! generated `Fsm` implements `rfs_adapters::ProtocolFsm`.
use std::path::Path;

fn main() {
    // Toolchain resolution is rfs-build's (RFS_BIN -> PATH). For in-workspace
    // convenience, when RFS_BIN is not pinned, prefer the workspace-built
    // toolchain over a bare PATH lookup. A cold checkout must therefore run
    // `cargo build` (which produces target/debug/rfs) first - or set RFS_BIN.
    let rfs_bin_pinned = std::env::var("RFS_BIN")
        .map(|v| !v.trim().is_empty())
        .unwrap_or(false);
    if !rfs_bin_pinned {
        let ws_rfs = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../target/debug/rfs");
        if ws_rfs.is_file() {
            std::env::set_var("RFS_BIN", &ws_rfs);
        }
    }
    let spec =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../examples/valid/43_sliding_window.rfs");
    let artifact = rfs_build::RfsBuild::new(spec)
        .backend("rust")
        .with_protocol_fsm(true)
        .compile()
        .expect("rfs codegen of the spec-43 ARQ sender failed (is the rfs toolchain built? run `cargo build` or set RFS_BIN)");
    // lib.rs pulls the artifact in via `include!`, which rejects inner
    // attributes (`#![allow(...)]`) - so strip them here and let the outer
    // `#[allow]` on the module in lib.rs carry the same suppression list.
    // Everything else stays byte-identical; the body file lives in OUT_DIR,
    // outside the source tree, so `cargo fmt` never sees generated code.
    let src = std::fs::read_to_string(&artifact).expect("read generated artifact");
    let body: String = src
        .lines()
        .filter(|l| !l.starts_with("#!"))
        .map(|l| format!("{l}\n"))
        .collect();
    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR set by cargo");
    std::fs::write(Path::new(&out_dir).join("43_sliding_window_body.rs"), body)
        .expect("write module body to OUT_DIR");
}
