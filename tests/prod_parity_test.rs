//! Phase 6 task 4, action 3 (subtask 4.3): production-FSM cross-backend
//! random-input parity (D-P6-6). For EVERY valid example: generate the
//! production FSM for both backends (property monitors ON), wrap each in a
//! thin driver with an identical stdout contract (tests/common/prod_driver),
//! drive both with IDENTICAL seeded LCG input scripts (200 steps, incl.
//! unexpected-event injections that arise naturally from random variant
//! selection - specs without `on_error` exercise the UnexpectedEvent path,
//! specs with it the fallback path), and assert byte-identical stdout.
//!
//! The drivers render raw values (state/vars/inputs/outputs as names +
//! integers; enum fields as variant indices) so the contract is independent
//! of either backend's own debug renderers. Assumption violations use a STOP
//! handler on both sides (the production default PANICs/aborts, which would
//! kill the comparison); the first property violation surfaces as a `PROP`
//! line and the FSM keeps stepping (sticky record), exactly per SEMANTICS 8.2.

use std::path::{Path, PathBuf};
use std::sync::mpsc;

mod common;
use common::prod_driver;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Pinned script parameters (task file: "Pin the seed + step count").
const SCRIPT_SEED: u64 = 43_210;
const SCRIPT_STEPS: usize = 200;

struct DriverRun {
    name: String,
    /// (exit code, stdout) per backend.
    rust: (i32, String),
    c: (i32, String),
}

fn run_one(spec: &Path) -> DriverRun {
    // Scratch tag: rustc derives the crate name from the driver file name,
    // so every non-alphanumeric (incl. the `.rfs` dot) becomes `_`.
    let rel: String = spec
        .strip_prefix(root())
        .expect("spec under root")
        .to_string_lossy()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    let src = std::fs::read_to_string(spec).unwrap_or_else(|e| panic!("read {:?}: {e}", spec));
    let ir = rfs::parse_source(&src).unwrap_or_else(|e| panic!("{:?} must parse: {e}", spec));
    // Multi-module files: the codegen command convention drives the FIRST
    // module only.
    let module = &ir.modules[0];
    let (rust_bin, c_bin) =
        prod_driver::build_drivers(&rel, module, &root().join("target").join("rfs_prod_parity"));
    let script = prod_driver::input_script(module, SCRIPT_SEED, SCRIPT_STEPS);
    let rust = prod_driver::run_driver(&rust_bin, &script);
    let c = prod_driver::run_driver(&c_bin, &script);
    DriverRun {
        name: spec.file_name().unwrap().to_string_lossy().into_owned(),
        rust,
        c,
    }
}

#[test]
fn production_drivers_byte_identical_all_valid_examples() {
    let files = rfs::collect_rfs_files(root().join("examples/valid")).expect("collect .rfs files");
    assert!(!files.is_empty(), "no valid examples found");

    // Compile + run in parallel (disjoint scratch dirs per example).
    let workers = std::thread::available_parallelism()
        .map(|n| n.get().min(8))
        .unwrap_or(4);
    let files: Vec<PathBuf> = files;
    let results: Vec<DriverRun> = std::thread::scope(|s| {
        let (tx, rx) = mpsc::channel::<DriverRun>();
        for _ in 0..workers {
            let files = &files;
            let tx = tx.clone();
            s.spawn(move || loop {
                let idx = {
                    let mut guard = NEXT.lock().unwrap();
                    let i = *guard;
                    *guard += 1;
                    i
                };
                let Some(spec) = files.get(idx) else {
                    break;
                };
                let r = run_one(spec);
                tx.send(r).expect("send result");
            });
        }
        drop(tx);
        rx.iter().collect()
    });

    assert_eq!(results.len(), files.len(), "every example must be driven");
    let mut saw_unexpected = false;
    for r in &results {
        assert_eq!(
            r.rust.0, 0,
            "[{}] rust driver exited {}: stdout:\n{}",
            r.name, r.rust.0, r.rust.1
        );
        assert_eq!(
            r.c.0, 0,
            "[{}] c driver exited {}: stdout:\n{}",
            r.name, r.c.0, r.c.1
        );
        assert_eq!(
            r.rust.1, r.c.1,
            "[{}] driver stdout diverges across backends\n[rust]\n{}\n[c]\n{}",
            r.name, r.rust.1, r.c.1
        );
        if r.rust.1.contains(" UNEXPECTED") {
            saw_unexpected = true;
        }
    }
    // The random scripts must have exercised the unexpected-event path
    // somewhere in the suite (specs without on_error + a state missing the
    // chosen variant). Guards against a degenerate script generator.
    assert!(
        saw_unexpected,
        "no UNEXPECTED line in any driver run - scripts degenerate?"
    );
}

// Shared work-stealing counter for the parallel build phase.
static NEXT: std::sync::Mutex<usize> = std::sync::Mutex::new(0);
