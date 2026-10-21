//! Phase 4 task 11 - SMT bounded mode (feature-gated: `--features smt`).
//!
//! End-to-end parity gate for the z3-based bounded checker: inline specs with
//! wide bitvectors (bit<16> vars + fields, shifts, comparisons) are generated
//! for BOTH backends, compiled against the project-local z3 5.3.1
//! (TOOLCHAIN.md section 5), run, and compared byte-for-byte (D-P4-12):
//! - FAIL spec: sound property-violation report at step 0 (initial config);
//! - LIVENESS spec: k-liveness cycle report;
//! - PASS spec: pass-within-bound (incomplete - no completeness claim).
//!
//! The Rust binaries are built in a scratch cargo project under `target/`
//! (offline, from the local registry cache) so the test never touches the
//! network; C links libz3 directly. Both runtimes need LD_LIBRARY_PATH for
//! libz3.so at execution time.

#![cfg(feature = "smt")]

use std::path::{Path, PathBuf};
use std::process::Command;

/// gcc location (plan assumption 5, verified since Phase 2).
const GCC: &str = "/usr/bin/gcc";
/// Project-local z3 5.3.1 install (TOOLCHAIN.md section 5.1).
const Z3_DIR: &str = "/workspace/host/z3";

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

// ---------------------------------------------------------------------------
// Inline specs (test inputs, not part of the valid-example set)
// ---------------------------------------------------------------------------

/// FAIL variant: `flag_or_counter` is violated in the INITIAL configuration
/// (flag=false, counter=0), so SMT must report a property-violation at step 0
/// - the same violation exact mode finds (cross-checked manually, task 11).
///
/// Wide-bitvector stress: bit<16> var + field, shifts with literal/narrow/
/// wide amounts, oversized-literal comparison (`counter < 256` on bit<8>).
const SMT_FAIL_SRC: &str = r#"
module smt_fail {
  enum State { A, B }
  initial A

  event Go(F(x: bit<8>, y: bit<16>, ok: bool))
  event Noop

  var counter: bit<8> init 0
  var flag: bool init false
  var big: bit<16> init 256

  transition A on Go(F(x, y, ok)) where x > 10 && ok {
    next B;
    set counter = counter + x;
    set flag = true;
  }
  transition A on Noop where counter < 4 {
    next A;
    set counter = counter << 2;
    set big = big >> 1;
  }
  transition B on Go(F(x, y, ok)) where !(x == 0) || x <= 3 {
    next A;
    set counter = counter - 1;
    set flag = false;
    set big = big * y;
  }

  prop counter_bounded : always (counter < 256) ;
  prop flag_or_counter : always (flag == true || counter > 0) ;
}
"#;

/// LIVENESS variant: the machine is stuck in A, so `eventually (state == B)`
/// is violated - k-liveness must report a cycle.
const SMT_LIV_SRC: &str = r#"
module smt_liv {
  enum State { A, B }
  initial A

  event E

  transition A on E { next A; }

  prop reach_b : eventually (state == B) ;
}
"#;

/// PASS variant: both properties hold within the bound (counter is bit<8>,
/// so `counter < 256` is trivially true; big stays in range under the ops).
const SMT_PASS_SRC: &str = r#"
module smt_pass {
  enum State { A, B }
  initial A

  event Go(F(x: bit<8>, y: bit<16>, ok: bool))
  event Noop

  var counter: bit<8> init 0
  var flag: bool init false
  var big: bit<16> init 256

  transition A on Go(F(x, y, ok)) where x > 10 && ok {
    next B;
    set counter = counter + x;
    set flag = true;
  }
  transition A on Noop where counter < 4 {
    next A;
    set counter = counter << 2;
    set big = big >> 1;
  }
  transition B on Go(F(x, y, ok)) where !(x == 0) || x <= 3 {
    next A;
    set counter = counter - 1;
    set flag = false;
    set big = big * y;
  }

  prop counter_bounded : always (counter < 256) ;
  prop big_in_range : always (big < 65536) ;
}
"#;

// ---------------------------------------------------------------------------
// Codegen + build helpers
// ---------------------------------------------------------------------------

fn generate(backend_name: &str, src: &str, what: &str) -> String {
    let ir = rfs::parse_source(src).unwrap_or_else(|e| panic!("{what} must parse: {e}"));
    assert_eq!(ir.modules.len(), 1);
    let backend = rfs::codegen::get_backend(backend_name)
        .unwrap_or_else(|| panic!("backend {backend_name} registered"));
    let options = rfs::codegen::BackendOptions {
        mc_mode: rfs::codegen::McMode::SmtBounded,
        mc_smt_bound: 3,
        ..rfs::codegen::BackendOptions::default()
    };
    rfs::codegen::CodegenCommandKind::ModelChecker
        .run(backend, &ir.modules[0], &options)
        .unwrap_or_else(|e| panic!("smt-bounded codegen {backend_name} for {what}: {e:?}"))
        .code
}

fn compile_c(src: &Path, bin: &Path) {
    let out = Command::new(GCC)
        .args(["-Wall", "-O2"])
        .arg(src)
        .arg("-o")
        .arg(bin)
        .arg(format!("-I{Z3_DIR}/include"))
        .arg(format!("-L{Z3_DIR}/bin"))
        .args(["-lz3", "-lm"])
        .output()
        .expect("run gcc on generated C");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "gcc failed on {}:\n{stderr}",
        src.display()
    );
    // Hard bar (task 7 convention): generated C compiles warning-free.
    assert!(
        !stderr.lines().any(|l| l.contains("warning:")),
        "gcc -Wall emitted warnings on {} (generated C must be warning-free):\n{stderr}",
        src.display()
    );
}

/// Scratch cargo project for the generated Rust binaries (offline build from
/// the local registry cache; z3-sys links the project-local libz3 via
/// Z3_LIBRARY_PATH_OVERRIDE - TOOLCHAIN.md section 5.2). One directory per
/// test: tests run in parallel and each owns its main.rs + binary.
fn rust_e2e_project(tag: &str) -> PathBuf {
    let dir = root().join("target").join(format!("smt_e2e_{tag}"));
    std::fs::create_dir_all(dir.join("src")).expect("create scratch project");
    std::fs::write(
        dir.join("Cargo.toml"),
        // The empty [workspace] table keeps cargo from walking up into the
        // repository workspace (Phase 5 task 1) - same fix as the aggregate
        // generated-code crate.
        "[package]\nname = \"smt_gen\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[workspace]\n\n[dependencies]\nz3 = \"0.21\"\n\n[profile.release]\nopt-level = 2\n",
    )
    .expect("write scratch Cargo.toml");
    dir
}

fn build_rust(project: &Path, src_code: &str) -> PathBuf {
    let src = project.join("src").join("main.rs");
    std::fs::write(&src, src_code).expect("write generated Rust");
    let out = Command::new("cargo")
        .args(["build", "--release", "--offline", "--quiet"])
        .current_dir(project)
        .env("Z3_LIBRARY_PATH_OVERRIDE", format!("{Z3_DIR}/bin"))
        .env("Z3_SYS_Z3_VERSION", "5.3.1")
        .output()
        .expect("run cargo in scratch project");
    assert!(
        out.status.success(),
        "cargo build failed on generated Rust:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    project.join("target").join("release").join("smt_gen")
}

fn run_bin(bin: &Path, args: &[&str]) -> (i32, String) {
    let out = Command::new(bin)
        .args(args)
        .env("LD_LIBRARY_PATH", format!("{Z3_DIR}/bin"))
        .output()
        .expect("run generated binary");
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).to_string(),
    )
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Both backends, three specs: byte-identical stdout + exit codes, pinned
/// status/label/step lines, incomplete labeling on the PASS spec.
#[test]
fn smt_bounded_backend_parity() {
    let project = rust_e2e_project("parity");
    for (what, src) in [
        ("fail", SMT_FAIL_SRC),
        ("liveness", SMT_LIV_SRC),
        ("pass", SMT_PASS_SRC),
    ] {
        let dir = root().join("target").join("rfs_smt_bounded");
        std::fs::create_dir_all(&dir).expect("create scratch dir");

        // C backend.
        let c_src = dir.join(format!("smt_{what}.c"));
        std::fs::write(&c_src, generate("c", src, what)).expect("write C source");
        let c_bin = dir.join(format!("smt_{what}_c"));
        compile_c(&c_src, &c_bin);

        // Rust backend.
        let r_bin = build_rust(&project, &generate("rust", src, what));

        let (rc, cout) = run_bin(&c_bin, &["--bound", "3"]);
        let (rr, rout) = run_bin(&r_bin, &["--bound", "3"]);
        assert_eq!(
            rr, rc,
            "{what}: exit codes differ (rust={rr} c={rc})\nrust:\n{rout}\nc:\n{cout}"
        );
        assert_eq!(
            rout, cout,
            "{what}: stdout differs between backends\n--- rust ---\n{rout}--- c ---\n{cout}"
        );

        match what {
            "fail" => {
                assert_eq!(rc, 1, "fail spec must exit 1:\n{cout}");
                assert!(
                    cout.contains("model-check summary: status=fail mode=smt-bounded bound=3 reason=property-violation label=flag_or_counter step=0"),
                    "pinned FAIL summary line missing:\n{cout}"
                );
                assert!(cout.starts_with("FAIL\n"), "FAIL banner first:\n{cout}");
            }
            "liveness" => {
                assert_eq!(rc, 1, "liveness spec must exit 1:\n{cout}");
                assert!(
                    cout.contains("reason=liveness-violation label=reach_b step=0"),
                    "pinned liveness summary line missing:\n{cout}"
                );
                assert!(
                    cout.contains("cycle length: 1"),
                    "cycle length line missing:\n{cout}"
                );
            }
            "pass" => {
                assert_eq!(rc, 0, "pass spec must exit 0:\n{cout}");
                assert!(
                    cout.contains("PASS WITHIN BOUND 3 (incomplete - no completeness claim)"),
                    "incomplete labeling missing:\n{cout}"
                );
                assert!(
                    cout.contains(
                        "model-check summary: status=pass-within-bound mode=smt-bounded bound=3"
                    ),
                    "pinned PASS summary line missing:\n{cout}"
                );
            }
            _ => unreachable!(),
        }
    }
}

/// The runtime `--bound` flag overrides the codegen default in both backends
/// (same behavior, same output shape).
#[test]
fn smt_bounded_runtime_bound_flag() {
    let project = rust_e2e_project("boundflag");
    let dir = root().join("target").join("rfs_smt_bounded");
    std::fs::create_dir_all(&dir).expect("create scratch dir");

    let c_src = dir.join("smt_boundflag.c");
    std::fs::write(&c_src, generate("c", SMT_PASS_SRC, "boundflag")).expect("write C source");
    let c_bin = dir.join("smt_boundflag_c");
    compile_c(&c_src, &c_bin);
    let r_bin = build_rust(&project, &generate("rust", SMT_PASS_SRC, "boundflag"));

    for (name, bin) in [("c", &c_bin), ("rust", &r_bin)] {
        let (rc, out) = run_bin(bin, &["--bound", "5"]);
        assert_eq!(rc, 0, "{name} bound=5 must pass:\n{out}");
        assert!(
            out.contains("PASS WITHIN BOUND 5"),
            "{name}: runtime --bound not honored:\n{out}"
        );
    }
}
