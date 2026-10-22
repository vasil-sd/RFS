//! Build script (Phase 5 task 12): generate the spec-43 ARQ sender for the
//! lib crate AND wire the emitted libfuzzer_sys skeleton into the cargo-fuzz
//! layout.
use std::path::Path;

fn main() {
    // Toolchain resolution: RFS_BIN > workspace target/debug/rfs > PATH
    // (same convention as the other example build scripts). A cold checkout
    // must run `cargo build` first - or set RFS_BIN.
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
    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR set by cargo");

    // 1. Production source -> the lib body. `include!` cannot carry inner
    // attributes, so strip them; src/lib.rs carries the same suppression list
    // as an outer #[allow]. (The test command below reuses the same artifact
    // path - it is already read into memory by then.)
    let artifact = rfs_build::RfsBuild::new(&spec)
        .backend("rust")
        .compile()
        .expect("rfs codegen of the spec-43 ARQ sender failed (is the rfs toolchain built? run `cargo build` or set RFS_BIN)");
    let src = std::fs::read_to_string(&artifact).expect("read generated artifact");
    let body: String = src
        .lines()
        .filter(|l| !l.starts_with("#!"))
        .map(|l| format!("{l}\n"))
        .collect();
    std::fs::write(Path::new(&out_dir).join("43_sliding_window_body.rs"), body)
        .expect("write module body to OUT_DIR");

    // 2. Test command -> the EMITTED fuzz skeleton (extra file written next
    // to the main artifact as fuzz_target.rs), adapted for this crate: the
    // skeleton's `use crate::{...}` import becomes `use fuzz_rust::{...}` so
    // it resolves against the generated module re-exported at the root.
    let _test_artifact = rfs_build::RfsBuild::new(&spec)
        .backend("rust")
        .test_command()
        .compile()
        .expect("rfs codegen (test command) of the spec-43 ARQ sender failed");
    let skeleton = std::fs::read_to_string(Path::new(&out_dir).join("fuzz_target.rs"))
        .expect("read emitted fuzz skeleton (extra file next to the test artifact)");
    let adapted = skeleton.replace("use crate::{", "use fuzz_rust::{");
    assert!(
        adapted != skeleton,
        "emitted skeleton lost its `use crate::{{` import line - adaptation failed"
    );
    let target_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("fuzz/fuzz_targets");
    std::fs::create_dir_all(&target_dir).expect("create fuzz/fuzz_targets");
    std::fs::write(target_dir.join("arq.rs"), adapted)
        .expect("write adapted fuzz target to fuzz/fuzz_targets/arq.rs");
}
