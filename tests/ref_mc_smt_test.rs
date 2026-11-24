//! Phase 16 task 4 - SMT-bounded mode for `rfs mc-ref` (feature-gated:
//! `--features smt`).
//!
//! Cross-backend byte-parity gate for the in-process z3 executor
//! (`src/mc/ref_mc_smt.rs`, the third consumer of the shared `SmtSpec`
//! encoding model): the spec family mirrors `smt_bounded_test`'s inline specs
//! (fail-at-initial / liveness-cycle / pass-within-bound - wide bitvectors,
//! shifts, oversized-literal comparisons) and is checked by ref-mc in-process
//! AND by BOTH generated SMT binaries at multiple bounds (incl. bound 1 and
//! bounds beyond the violation depth), byte-comparing the FULL stdout (human
//! lines + witness trace + summary line) and the exit code.
//!
//! Solver versions: the executor links whatever z3-sys resolved at rfs build
//! time (pkg-config - the system z3 on this host, the nix unified prefix under
//! `nix develop`); the generated binaries link the unified resolver's prefix
//! (`RFS_Z3_DIR` > `Z3_SYS_Z3_DIR` > pkg-config, loadability-probed). When the
//! two differ (host: system z3 vs project-local 5.3.1) this suite pins
//! CROSS-solver-version parity - the Phase 15 task-2b lesson: parity must hold
//! under ANY solver version. The versions are embedded in every assertion
//! message so a failure names both sides.

#![cfg(feature = "smt")]

use std::path::{Path, PathBuf};
use std::process::Command;

mod common;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

// ---------------------------------------------------------------------------
// Spec family (mirrors smt_bounded_test's inline specs, defined locally per
// the task file - that suite is not restructured)
// ---------------------------------------------------------------------------

/// FAIL variant: `flag_or_counter` is violated in the INITIAL configuration
/// (flag=false, counter=0), so SMT must report a property-violation at step 0.
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

/// PASS variant: both properties hold within the bound.
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
// z3 resolution + build helpers (mirror smt_bounded_test's - the task file
// pins them locally so that suite is not restructured)
// ---------------------------------------------------------------------------

/// The z3 prefix usable in THIS environment + whether it came from an explicit
/// var (`true`) or pkg-config auto-discovery (`false`). An explicit install is
/// PROBED before being trusted (the host-local RFS_Z3_DIR default cannot load
/// under a nix develop shell's glibc); on probe failure we fall back to the
/// pkg-config tier. All consumers share this one resolution, so both generated
/// backends always link and run the SAME solver (Phase 15 task-2b guarantee).
fn resolved_z3() -> &'static (String, bool) {
    static RESOLVED: std::sync::OnceLock<(String, bool)> = std::sync::OnceLock::new();
    RESOLVED.get_or_init(|| {
        let (z3, explicit) =
            common::toolchain::z3().expect("z3 availability checked at test entry");
        if !explicit || z3_prefix_runs(&z3) {
            return (z3, explicit);
        }
        if let Ok(out) = Command::new("pkg-config")
            .args(["--variable=prefix", "z3"])
            .output()
        {
            if out.status.success() {
                let prefix = String::from_utf8_lossy(&out.stdout).trim().to_string();
                let d = Path::new(&prefix);
                let install_ok = !prefix.is_empty()
                    && (d.join("include").join("z3.h").is_file()
                        || d.join("bin").is_dir()
                        || d.join("lib").is_dir());
                if install_ok && z3_prefix_runs(&prefix) {
                    return (prefix, false);
                }
            }
        }
        (z3, explicit)
    })
}

/// True when a no-op program linked against `prefix`'s libz3 actually RUNS in
/// this environment (exit 0).
fn z3_prefix_runs(prefix: &str) -> bool {
    let dir = root().join("target").join("rfs_ref_mc_smt");
    std::fs::create_dir_all(&dir).expect("create scratch dir");
    let src = dir.join("z3_probe.c");
    std::fs::write(&src, "int main(void){return 0;}\n").expect("write probe source");
    let bin = dir.join("z3_probe");
    let out = Command::new(common::toolchain::gcc())
        .arg("-O1")
        .arg(&src)
        .arg("-o")
        .arg(&bin)
        .arg(format!("-L{prefix}/lib"))
        .arg(format!("-L{prefix}/bin"))
        .args(["-lz3", "-lm"])
        .output()
        .expect("run gcc on z3 probe");
    if !out.status.success() {
        return false;
    }
    let existing = std::env::var("LD_LIBRARY_PATH").unwrap_or_default();
    let run = Command::new(&bin)
        .env(
            "LD_LIBRARY_PATH",
            format!("{prefix}/lib:{prefix}/bin:{existing}"),
        )
        .output()
        .expect("run z3 probe binary");
    run.status.code() == Some(0)
}

/// z3 compile+link flags from the unified resolver (same rules as
/// smt_bounded_test's `z3_compile_args`).
fn z3_compile_args() -> Vec<String> {
    let (z3, explicit) = resolved_z3().to_owned();
    let prefix_flags = vec![
        format!("-I{z3}/include"),
        format!("-L{z3}/lib"),
        format!("-L{z3}/bin"),
        "-lz3".to_string(),
    ];
    let mut args = if explicit {
        prefix_flags
    } else if let Ok(out) = Command::new("pkg-config")
        .args(["--cflags", "--libs", "z3"])
        .output()
    {
        if out.status.success() {
            let s = String::from_utf8_lossy(&out.stdout);
            let pc: Vec<String> = s.split_whitespace().map(str::to_string).collect();
            if !pc.is_empty() {
                pc
            } else {
                prefix_flags
            }
        } else {
            prefix_flags
        }
    } else {
        prefix_flags
    };
    args.push("-lm".to_string());
    args
}

fn compile_c(src: &Path, bin: &Path) {
    let out = Command::new(common::toolchain::gcc())
        .args(["-Wall", "-O2"])
        .arg(src)
        .arg("-o")
        .arg(bin)
        .args(z3_compile_args())
        .output()
        .expect("run gcc on generated C");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "gcc failed on {}:\n{stderr}",
        src.display()
    );
    assert!(
        !stderr.lines().any(|l| l.contains("warning:")),
        "gcc -Wall emitted warnings on {} (generated C must be warning-free):\n{stderr}",
        src.display()
    );
}

/// Scratch cargo project for the generated Rust binaries (offline build from
/// the local registry cache; z3-sys links the resolved prefix).
fn rust_e2e_project(tag: &str) -> PathBuf {
    let dir = root().join("target").join(format!("ref_mc_smt_e2e_{tag}"));
    std::fs::create_dir_all(dir.join("src")).expect("create scratch project");
    std::fs::write(
        dir.join("Cargo.toml"),
        "[package]\nname = \"smt_gen\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[workspace]\n\n[dependencies]\nz3 = \"0.21\"\n\n[profile.release]\nopt-level = 2\n",
    )
    .expect("write scratch Cargo.toml");
    dir
}

fn build_rust(project: &Path, src_code: &str) -> PathBuf {
    let src = project.join("src").join("main.rs");
    std::fs::write(&src, src_code).expect("write generated Rust");
    let (z3, explicit) = resolved_z3().to_owned();
    let lib_override = format!("{z3}/bin");
    let mut cmd = Command::new(common::toolchain::cargo());
    cmd.args(["build", "--release", "--offline", "--quiet"]);
    cmd.current_dir(project);
    cmd.env("Z3_SYS_Z3_DIR", &z3);
    cmd.env("Z3_LIBRARY_PATH_OVERRIDE", &lib_override);
    if explicit {
        cmd.env("Z3_NO_PKG_CONFIG", "1");
        cmd.env("Z3_SYS_Z3_VERSION", "5.3.1");
    }
    let out = cmd.output().expect("run cargo in scratch project");
    assert!(
        out.status.success(),
        "cargo build failed on generated Rust:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    project.join("target").join("release").join("smt_gen")
}

fn run_bin(bin: &Path, args: &[&str]) -> (i32, String) {
    let (code, out, _err) = run_bin_full(bin, args);
    (code, out)
}

/// Like `run_bin` but also returns stderr (the omission warnings land there -
/// task 1A pins them byte-for-byte against the ref engine's warning list).
fn run_bin_full(bin: &Path, args: &[&str]) -> (i32, String, String) {
    let (z3, _) = resolved_z3().to_owned();
    let existing = std::env::var("LD_LIBRARY_PATH").unwrap_or_default();
    let out = Command::new(bin)
        .args(args)
        .env("LD_LIBRARY_PATH", format!("{z3}/lib:{z3}/bin:{existing}"))
        .output()
        .expect("run generated SMT binary");
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

// ---------------------------------------------------------------------------
// Per-process binary cache + codegen
// ---------------------------------------------------------------------------

static BIN_CACHE: std::sync::LazyLock<
    std::sync::Mutex<std::collections::HashMap<String, PathBuf>>,
> = std::sync::LazyLock::new(|| std::sync::Mutex::new(std::collections::HashMap::new()));

fn cached_binary(key: &str, build: impl FnOnce() -> PathBuf) -> PathBuf {
    // Task 3K: recover from a poisoned mutex instead of cascading. A build-closure
    // panic (e.g. rustc's target-probe dying on a corrupted /dev/null - the
    // 2026-11-23 mid-run re-corruption incident) used to poison this lock and kill
    // every remaining parity test with PoisonError (14 instant failures, 0.11 s).
    // into_inner() degrades that to "rebuild the binary for this one test".
    let mut cache = BIN_CACHE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(p) = cache.get(key) {
        return p.clone();
    }
    let p = build();
    cache.insert(key.to_string(), p.clone());
    p
}

/// Generate + compile one backend's SMT binary for a spec (cached per
/// (backend, spec-tag)). The runtime `--bound` flag overrides the baked-in
/// default, so one binary serves every bound under test.
fn smt_binary(backend: &str, src: &str, tag: &str) -> PathBuf {
    cached_binary(&format!("{backend}:{tag}"), || {
        let ir = rfs::parse_source(src).unwrap_or_else(|e| panic!("{tag} must parse: {e}"));
        assert_eq!(ir.modules.len(), 1);
        let backend_impl = rfs::codegen::get_backend(backend)
            .unwrap_or_else(|| panic!("backend {backend} registered"));
        let options = rfs::codegen::BackendOptions {
            mc_mode: rfs::codegen::McMode::SmtBounded,
            mc_smt_bound: 3,
            ..rfs::codegen::BackendOptions::default()
        };
        let code = rfs::codegen::CodegenCommandKind::ModelChecker
            .run(backend_impl, &ir, &options)
            .unwrap_or_else(|e| panic!("smt-bounded codegen {backend} for {tag}: {e:?}"))
            .primary_code()
            .to_string();
        let dir = root()
            .join("target")
            .join(format!("ref_mc_smt_{backend}_{tag}"));
        std::fs::create_dir_all(&dir).expect("create output dir");
        if backend == "c" {
            let src_path = dir.join("main.c");
            std::fs::write(&src_path, &code).expect("write generated C");
            let bin = dir.join("smt_bin");
            compile_c(&src_path, &bin);
            bin
        } else {
            let project = rust_e2e_project(&format!("{backend}_{tag}"));
            build_rust(&project, &code)
        }
    })
}

/// Run ref-mc's in-process SMT executor on a spec source at one bound.
/// `full_properties` mirrors the generated binaries' `--full-properties`
/// flag (task 1A): without it, FULL-class properties are omitted with a
/// per-property warning (returned as the third tuple element).
/// `full_assumptions` is the DISTINCT gate for the FULL assumption class
/// (task 3C); `assumes_arg` is the `--assumes` wire string (`all` / `none` /
/// comma-separated labels).
fn ref_smt(
    src: &str,
    bound: usize,
    full_properties: bool,
    full_assumptions: bool,
    assumes_arg: &str,
) -> (i32, String, Vec<String>) {
    let ir = rfs::parse_source(src).unwrap_or_else(|e| panic!("spec must parse: {e}"));
    assert_eq!(ir.modules.len(), 1);
    let module = &ir.modules[0];
    let out = rfs::mc::ref_mc::run_smt(
        module,
        bound,
        full_properties,
        full_assumptions,
        assumes_arg,
    )
    .unwrap_or_else(|e| panic!("ref-mc smt run failed: {e}"));
    (out.exit_code, out.output, out.warnings)
}

/// The solver versions under test, embedded in every assertion message so a
/// failure names both sides (in-process = what z3-sys linked at rfs build
/// time; generated = the unified resolver's prefix).
fn version_note() -> String {
    format!(
        "[in-process z3 {} | generated binaries: {}]",
        rfs::mc::ref_mc_smt::z3_version(),
        resolved_z3().0
    )
}

// ---------------------------------------------------------------------------
// The parity suite
// ---------------------------------------------------------------------------

/// Multiple bounds: 1 (the minimum - the liveness cycle query is vacuous), 3
/// (the codegen default), and 5 (beyond any violation depth in the family).
const BOUNDS: [usize; 3] = [1, 3, 5];

#[test]
fn ref_smt_vs_generated_fail_spec() {
    // Violated at the INITIAL configuration: every bound >= 1 fails at step 0.
    let note = version_note();
    for &bound in &BOUNDS {
        let (ref_code, ref_out, _warnings) = ref_smt(SMT_FAIL_SRC, bound, false, false, "all");
        assert_eq!(ref_code, 1, "ref exit {note}");
        for backend in ["rust", "c"] {
            let bin = smt_binary(backend, SMT_FAIL_SRC, "fail");
            let (gen_code, gen_out) = run_bin(&bin, &["--bound", &bound.to_string()]);
            assert_eq!(
                ref_code, gen_code,
                "{backend} exit mismatch at bound {bound} {note}"
            );
            assert_eq!(
                ref_out, gen_out,
                "{backend} stdout mismatch at bound {bound} {note}\n--- ref ---\n{ref_out}\n--- {backend} ---\n{gen_out}"
            );
        }
    }
}

#[test]
fn ref_smt_vs_generated_liveness_spec() {
    // Bound 1: no a<b pair exists -> the cycle query is vacuous -> pass.
    // Bounds >= 2: the stuck-in-A cycle is found -> liveness fail at step 0.
    let note = version_note();
    for &bound in &BOUNDS {
        let (ref_code, ref_out, _warnings) = ref_smt(SMT_LIV_SRC, bound, false, false, "all");
        let expected = if bound == 1 { 0 } else { 1 };
        assert_eq!(ref_code, expected, "ref exit at bound {bound} {note}");
        for backend in ["rust", "c"] {
            let bin = smt_binary(backend, SMT_LIV_SRC, "liv");
            let (gen_code, gen_out) = run_bin(&bin, &["--bound", &bound.to_string()]);
            assert_eq!(
                ref_code, gen_code,
                "{backend} exit mismatch at bound {bound} {note}"
            );
            assert_eq!(
                ref_out, gen_out,
                "{backend} stdout mismatch at bound {bound} {note}\n--- ref ---\n{ref_out}\n--- {backend} ---\n{gen_out}"
            );
        }
    }
}

#[test]
fn ref_smt_vs_generated_pass_spec() {
    let note = version_note();
    for &bound in &BOUNDS {
        let (ref_code, ref_out, _warnings) = ref_smt(SMT_PASS_SRC, bound, false, false, "all");
        assert_eq!(ref_code, 0, "ref exit at bound {bound} {note}");
        for backend in ["rust", "c"] {
            let bin = smt_binary(backend, SMT_PASS_SRC, "pass");
            let (gen_code, gen_out) = run_bin(&bin, &["--bound", &bound.to_string()]);
            assert_eq!(
                ref_code, gen_code,
                "{backend} exit mismatch at bound {bound} {note}"
            );
            assert_eq!(
                ref_out, gen_out,
                "{backend} stdout mismatch at bound {bound} {note}\n--- ref ---\n{ref_out}\n--- {backend} ---\n{gen_out}"
            );
        }
    }
}

#[test]
fn ref_smt_accepts_assumption_and_on_error_specs() {
    // Task 3C: the old shared rejections are gone - specs with assumptions or
    // on_error blocks now run. The pre-3C error strings must not resurface.
    let assume_src = r#"
module smt_ref_cli_assume {
  enum State { A }
  initial A
  event E
  assume a1 : always (true) ;
  transition A on E { next A; }
}
"#;
    let ir = rfs::parse_source(assume_src).expect("parses");
    let out = rfs::mc::ref_mc::run_smt(&ir.modules[0], 3, false, false, "all")
        .expect("assumption specs are accepted now");
    assert_eq!(out.exit_code, 0);
    assert!(!out.output.contains("does not support assumptions"));

    let onerr_src = r#"
module smt_ref_cli_onerr {
  enum State { A }
  initial A
  event E
  transition A on E { next A; }
  on_error { next A; }
}
"#;
    let ir = rfs::parse_source(onerr_src).expect("parses");
    let out = rfs::mc::ref_mc::run_smt(&ir.modules[0], 3, false, false, "all")
        .expect("on_error specs are accepted now");
    assert_eq!(out.exit_code, 0);
    assert!(!out.output.contains("does not support on_error"));
}

// ---------------------------------------------------------------------------
// Task 1A - extended property language: input_is / output_has / next are
// default-class; the FULL class (until, nested temporals) is behind
// --full-properties with per-property omission warnings; renamed event
// parameters encode unconditionally. Every spec below is parity-pinned
// against BOTH generated backends exactly like the family above.
// ---------------------------------------------------------------------------

/// input_is PASS: a Go with ok=false can only be consumed from B (A's guard
/// blocks it), and that transition lands in A - so every sample whose just-
/// taken transition was such a Go is in state A. Wildcard x and literal
/// ok=false exercise constraining field patterns in an input_is atom.
const SMT_II_PASS_SRC: &str = r#"
module smt_ii_pass {
  enum State { A, B }
  initial A

  event Go(F(x: bit<8>, ok: bool))
  event Noop

  var counter: bit<8> init 0

  transition A on Go(F(x, ok)) where ok {
    next B;
    set counter = x;
  }
  transition A on Noop {
    next A;
    set counter = counter + 1;
  }
  transition B on Go(F(x, ok)) {
    next A;
    set counter = counter + 1;
  }

  prop bad_go_lands_in_a : always (input_is(Go(F(_, false))) -> state == A) ;
}
"#;

/// input_is FAIL: in state B a Go (any payload) fires, so the implication is
/// violated at the first such step.
const SMT_II_FAIL_SRC: &str = r#"
module smt_ii_fail {
  enum State { A, B }
  initial A

  event Go

  var counter: bit<8> init 0

  transition A on Go {
    next B;
    set counter = counter + 1;
  }
  transition B on Go {
    next A;
  }

  prop no_go_in_b : always (state == B -> !input_is(Go)) ;
}
"#;

/// output_has PASS: every emitted Ans carries a nonzero value. Emit arguments
/// read OLD values (SEMANTICS 3.2), so `emit Ans(counter)` in A emits the
/// pre-assignment counter (init 1); B emits Ans(counter + 1) = 6. Unit event,
/// no guards: every transition is always enabled, so the unrolled path is
/// fully forced (no solver freedom in the witness tail).
const SMT_OH_PASS_SRC: &str = r#"
module smt_oh_pass {
  enum State { A, B }
  initial A

  event Req
  output Ans(Value(y: bit<8>))

  var counter: bit<8> init 1

  transition A on Req {
    next B;
    set counter = 5;
    emit Ans(Value(counter));
  }
  transition B on Req {
    next A;
    emit Ans(Value(counter + 1));
  }

  prop no_zero_ans : always (!output_has(Ans(Value(0)))) ;
}
"#;

/// output_has FAIL: the first Req consumed in A emits `Ans(counter)` with the
/// OLD counter (0) - a literal-argument output_has atom catches it. Unit
/// event, no guards: the path is fully forced after the violation too.
const SMT_OH_FAIL_SRC: &str = r#"
module smt_oh_fail {
  enum State { A, B }
  initial A

  event Req
  output Ans(Value(y: bit<8>))

  var counter: bit<8> init 0

  transition A on Req {
    next B;
    set counter = 5;
    emit Ans(Value(counter));
  }
  transition B on Req {
    next A;
    emit Ans(Value(counter + 1));
  }

  prop no_zero_ans : always (!output_has(Ans(Value(0)))) ;
}
"#;

/// next PASS - the boundary-skip negative control. The machine only visits
/// A/B, so `next (state == A || state == B)` holds at every evaluable anchor
/// (0..k-2). Anchor k-1 would evaluate state(k) - a GHOST constant outside
/// the unrolling that the solver could set to C (or an out-of-range value);
/// only the next-depth clip keeps this spec green. Three states make the
/// ghost detectable (STATE_BITS >= 2).
const SMT_NEXT_PASS_SRC: &str = r#"
module smt_next_pass {
  enum State { A, B, C }
  initial A

  event Tick

  var counter: bit<8> init 0

  transition A on Tick {
    next B;
    set counter = counter + 1;
  }
  transition B on Tick {
    next A;
    set counter = counter + 1;
  }

  prop state_valid_next : always (next (state == A || state == B)) ;
}
"#;

/// next FAIL: counter(1) = 1 != 0, so the anchor-0 evaluation already fails.
const SMT_NEXT_FAIL_SRC: &str = r#"
module smt_next_fail {
  enum State { A, B }
  initial A

  event Tick

  var counter: bit<8> init 0

  transition A on Tick {
    next B;
    set counter = counter + 1;
  }
  transition B on Tick {
    next A;
    set counter = counter + 1;
  }

  prop counter_zero_next : always (next (counter == 0)) ;
}
"#;

/// Renamed event parameter: the pattern `Reply(R(rk))` renames declared field
/// `rkey` to binding `rk`; the guard `rk == key` must resolve through the
/// alias. A green FAIL pin proves it: if the alias were broken, codegen would
/// die with "unknown identifier 'rk'" (correction 1a's failure mode), and if
/// the guard silently matched nothing, key would never grow and the property
/// would PASS - so only a correct alias yields this pinned violation.
const SMT_RENAME_SRC: &str = r#"
module smt_rename {
  enum State { A, B }
  initial A

  event Reply(R(rkey: bit<8>))

  var key: bit<8> init 0

  transition A on Reply(R(rk)) where rk == key {
    next B;
    set key = key + 1;
  }
  transition B on Reply(R(_)) {
    next A;
    set key = key + 1;
  }

  prop key_bounded : always (key < 3) ;
}
"#;

/// Top-level bounded_eventually is DEFAULT class (no flag needed): the
/// machine reaches B at step 1, inside the window.
const SMT_BE_PASS_SRC: &str = r#"
module smt_be_pass {
  enum State { A, B }
  initial A

  event Tick

  transition A on Tick { next B; }
  transition B on Tick { next B; }

  prop reach_b : bounded_eventually[3] (state == B) ;
}
"#;

/// Top-level bounded_eventually FAIL: B is never reached, so the one-shot
/// obligation is violated at its deadline (step N-1 = 2).
const SMT_BE_FAIL_SRC: &str = r#"
module smt_be_fail {
  enum State { A, B }
  initial A

  event Tick

  transition A on Tick { next A; }

  prop reach_b : bounded_eventually[3] (state == B) ;
}
"#;

// ---------------------------------------------------------------------------
// Task 3A - trace-death soundness controls (D1 next / D2 bounded_eventually).
// Each NEGATIVE control is a spec that FALSE-FAILED before the liveness gate:
// the path dies (an unhandled input freezes the config) before the temporal
// obligation's deadline, so exact mode would leave it weak/vacuous. After 3A
// they PASS. Each POSITIVE control is its alive-path twin: the same obligation
// on a path that survives to the deadline and genuinely misses the target - it
// must STILL FAIL (the gate masks nothing).
// ---------------------------------------------------------------------------

/// D1 negative: `next` body checked at a successor the path never reaches. A
/// consumes Go into B (x=true); B handles NO input, so any step-1 input is
/// unhandled and freezes sample 2 (alive[2]=false). Before 3A the body
/// x(2)==false was checked against the frozen artifact -> false FAIL; after 3A
/// the next-obligation is vacuous (NOT alive[2]) -> PASS at every bound.
const SMT_D1_NEXT_DEATH_SRC: &str = r#"
module d1_next_death {
  enum State { A, B }
  initial A

  event Go

  var x: bool init false

  transition A on Go { next B; set x = true; }
  # B handles no input: every step-1 input is unhandled (frozen self-loop).

  prop p : always (input_is(Go) -> next (x == false)) ;
}
"#;

/// D1 positive: the alive-path twin. B now handles Go (self-loop), so sample 2
/// is a REAL successor with x still true - a genuine violation of next(x==false)
/// at anchor 1. Must FAIL for bound >= 2 (anchor 1 needs k >= 2).
const SMT_D1_NEXT_ALIVE_SRC: &str = r#"
module d1_next_alive {
  enum State { A, B }
  initial A

  event Go

  var x: bool init false

  transition A on Go { next B; set x = true; }
  transition B on Go { next B; }

  prop p : always (input_is(Go) -> next (x == false)) ;
}
"#;

/// D2 negative: a top-level bounded_eventually[3] whose deadline (sample 2) is
/// past path death. No transition handles any input, so the path dies at sample
/// 1 (alive[1]=false). Before 3A the disjunction was evaluated over the frozen
/// tail -> false FAIL; after 3A the violation requires alive[2] (false) -> PASS.
const SMT_D2_BE_DEATH_SRC: &str = r#"
module d2_be_death {
  enum State { A, B }
  initial A

  event Kill
  # No transition handles Kill: the path dies at sample 1.

  prop p : bounded_eventually[3] (state == B) ;
}
"#;

/// D2 positive: the alive-path twin. A Tick self-loop keeps the path alive to
/// the deadline, but B is never reached - a genuine bounded_eventually[3]
/// violation. Must FAIL for bound >= 3 (the deadline sample 2 needs k >= 3).
const SMT_D2_BE_ALIVE_SRC: &str = r#"
module d2_be_alive {
  enum State { A, B }
  initial A

  event Tick

  transition A on Tick { next A; }

  prop p : bounded_eventually[3] (state == B) ;
}
"#;

/// FULL class (until): without --full-properties the until property is
/// OMITTED (pinned warning + properties_omitted=1 suffix) while the default
/// property is still checked; with the flag the expansion runs and passes.
const SMT_UNTIL_PASS_SRC: &str = r#"
module smt_until_pass {
  enum State { A, B }
  initial A

  event Tick

  transition A on Tick { next B; }
  transition B on Tick { next B; }

  prop a_until_b : (state == A) until (state == B) ;
  prop states_ok : always (state == A || state == B) ;
}
"#;

/// FULL class (until) FAIL: the machine cycles A/C forever - `state == A`
/// fails at every C step and `state == B` never holds, so the weak until is
/// false on the infinite path (cycle witness, length 2). Checked only WITH
/// --full-properties; without it the spec passes with the omission suffix.
const SMT_UNTIL_FAIL_SRC: &str = r#"
module smt_until_fail {
  enum State { A, B, C }
  initial A

  event Tick

  transition A on Tick { next C; }
  transition C on Tick { next A; }

  prop a_until_b : (state == A) until (state == B) ;
}
"#;

/// The pinned per-property omission warning (task 1A contract).
fn omit_warning(name: &str) -> String {
    format!(
        "warning: smt-bounded mode omits property '{name}' by default - it uses bounded_eventually/until/nested temporal operators, which greatly affect solver time/memory at large bounds (O(k*N) formula growth). Pass --full-properties to include it."
    )
}

/// Parity helper for the task-1A family: ref vs both generated backends at
/// every bound, full stdout byte-compare + exit code. `full` selects the
/// --full-properties contract on all three surfaces; `expect_exit` is
/// per-bound (a violation deeper than the bound leaves the spec passing -
/// bounded mode is incomplete by construction).
fn parity_task1a(src: &str, tag: &str, full: bool, expect_exit: impl Fn(usize) -> i32) {
    let note = version_note();
    for &bound in &BOUNDS {
        let (ref_code, ref_out, _warnings) = ref_smt(src, bound, full, false, "all");
        assert_eq!(
            ref_code,
            expect_exit(bound),
            "ref exit at bound {bound} {note}"
        );
        for backend in ["rust", "c"] {
            let bin = smt_binary(backend, src, tag);
            let bound_s = bound.to_string();
            let args: Vec<&str> = if full {
                vec!["--bound", &bound_s, "--full-properties"]
            } else {
                vec!["--bound", &bound_s]
            };
            let (gen_code, gen_out) = run_bin(&bin, &args);
            assert_eq!(
                ref_code, gen_code,
                "{backend} exit mismatch at bound {bound} (full={full}) {note}"
            );
            assert_eq!(
                ref_out, gen_out,
                "{backend} stdout mismatch at bound {bound} (full={full}) {note}\n--- ref ---\n{ref_out}\n--- {backend} ---\n{gen_out}"
            );
        }
    }
}

#[test]
fn task1a_input_is_pass_and_fail() {
    parity_task1a(SMT_II_PASS_SRC, "ii_pass", false, |_| 0);
    // Reaching B needs step 1; a Go consumed IN B lands at sample >= 2.
    // The violation lands at sample 1 (the first ok-Go consumed in A).
    parity_task1a(SMT_II_FAIL_SRC, "ii_fail", false, |b| {
        if b >= 2 {
            1
        } else {
            0
        }
    });
}

#[test]
fn task1a_output_has_pass_and_fail() {
    parity_task1a(SMT_OH_PASS_SRC, "oh_pass", false, |_| 0);
    // The zero-ans emit lands at sample 1 (the first Req consumed in A).
    parity_task1a(SMT_OH_FAIL_SRC, "oh_fail", false, |b| {
        if b >= 2 {
            1
        } else {
            0
        }
    });
}

#[test]
fn task1a_next_pass_boundary_skip_and_fail() {
    // The PASS spec is the negative control: it stays green at every bound
    // ONLY because anchor k-1 is clipped (ghost state(k) would fail).
    parity_task1a(SMT_NEXT_PASS_SRC, "next_pass", false, |_| 0);
    // Anchor 0 exists only for k >= 2; counter(1) = 1 != 0 fails it.
    parity_task1a(SMT_NEXT_FAIL_SRC, "next_fail", false, |b| {
        if b >= 2 {
            1
        } else {
            0
        }
    });
}

#[test]
fn task1a_renamed_event_parameter() {
    // key grows by one per matching Reply; reaching 3 needs three increments,
    // i.e. bound >= 4 - so bounds 1 and 3 pass and bound 5 fails (the pinned
    // violation proves the alias resolved: a broken guard would never grow
    // key, an unresolvable binding would die at codegen).
    let note = version_note();
    for &bound in &BOUNDS {
        let expect_exit = if bound >= 4 { 1 } else { 0 };
        let (ref_code, ref_out, _warnings) = ref_smt(SMT_RENAME_SRC, bound, false, false, "all");
        assert_eq!(ref_code, expect_exit, "ref exit at bound {bound} {note}");
        for backend in ["rust", "c"] {
            let bin = smt_binary(backend, SMT_RENAME_SRC, "rename");
            let (gen_code, gen_out) = run_bin(&bin, &["--bound", &bound.to_string()]);
            assert_eq!(
                ref_code, gen_code,
                "{backend} exit mismatch at bound {bound} {note}"
            );
            assert_eq!(
                ref_out, gen_out,
                "{backend} stdout mismatch at bound {bound} {note}\n--- ref ---\n{ref_out}\n--- {backend} ---\n{gen_out}"
            );
        }
    }
}

#[test]
fn task1a_top_level_bounded_eventually_default_class() {
    parity_task1a(SMT_BE_PASS_SRC, "be_pass", false, |_| 0);
    // The window [0,3) exceeds the horizon for k < 3: pending, no claim.
    parity_task1a(SMT_BE_FAIL_SRC, "be_fail", false, |b| {
        if b >= 3 {
            1
        } else {
            0
        }
    });
}

#[test]
fn task3a_trace_death_negative_and_positive_controls() {
    // D1: the next-obligation is weak at trace death. The death variant PASSES
    // at every bound (before 3A it false-failed on the frozen successor); the
    // alive twin still FAILS for bound >= 2 (a genuine violation is not masked).
    parity_task1a(SMT_D1_NEXT_DEATH_SRC, "d1_next_death", false, |_| 0);
    parity_task1a(SMT_D1_NEXT_ALIVE_SRC, "d1_next_alive", false, |b| {
        if b >= 2 {
            1
        } else {
            0
        }
    });
    // D2: a top-level bounded_eventually is unconfirmable past path death. The
    // death variant PASSES at every bound (before 3A it false-failed over the
    // frozen tail); the alive twin still FAILS for bound >= 3 (the deadline is
    // reached and B is genuinely never entered).
    parity_task1a(SMT_D2_BE_DEATH_SRC, "d2_be_death", false, |_| 0);
    parity_task1a(SMT_D2_BE_ALIVE_SRC, "d2_be_alive", false, |b| {
        if b >= 3 {
            1
        } else {
            0
        }
    });
}

#[test]
fn task1a_full_class_omission_and_flag() {
    let note = version_note();
    for &bound in &BOUNDS {
        // WITHOUT the flag: the until property is omitted with the pinned
        // warning; the default property is still checked; the summary line
        // carries properties_omitted=1.
        for (src, tag, expect_exit) in [
            (SMT_UNTIL_PASS_SRC, "until_pass", 0),
            (SMT_UNTIL_FAIL_SRC, "until_fail", 0),
        ] {
            let (ref_code, ref_out, warnings) = ref_smt(src, bound, false, false, "all");
            assert_eq!(ref_code, expect_exit, "ref exit at bound {bound} {note}");
            assert_eq!(
                warnings,
                vec![omit_warning("a_until_b")],
                "ref omission warning at bound {bound} {note}"
            );
            assert!(
                ref_out.contains(&format!(
                    "model-check summary: status=pass-within-bound mode=smt-bounded bound={bound} properties_omitted=1"
                )),
                "missing omission suffix in ref output at bound {bound}:\n{ref_out}"
            );
            for backend in ["rust", "c"] {
                let bin = smt_binary(backend, src, tag);
                let (gen_code, gen_out, gen_err) =
                    run_bin_full(&bin, &["--bound", &bound.to_string()]);
                assert_eq!(
                    ref_code, gen_code,
                    "{backend} exit mismatch at bound {bound} {note}"
                );
                assert_eq!(
                    ref_out, gen_out,
                    "{backend} stdout mismatch at bound {bound} {note}\n--- ref ---\n{ref_out}\n--- {backend} ---\n{gen_out}"
                );
                assert_eq!(
                    gen_err,
                    format!("{}\n", omit_warning("a_until_b")),
                    "{backend} stderr (omission warning) mismatch at bound {bound} {note}"
                );
            }
        }
        // WITH the flag: no warnings, no suffix; the expansion runs and its
        // verdict is pinned (pass spec passes, fail spec fails on the cycle -
        // except at bound 1 where no a<b pair exists and the cycle query is
        // vacuous, mirroring the liveness family's pin).
        for (src, tag) in [
            (SMT_UNTIL_PASS_SRC, "until_pass"),
            (SMT_UNTIL_FAIL_SRC, "until_fail"),
        ] {
            let expect_exit = match (src == SMT_UNTIL_FAIL_SRC, bound) {
                (true, 1) => 0, // vacuous cycle query at bound 1
                (true, _) => 1,
                (false, _) => 0,
            };
            let (ref_code, ref_out, warnings) = ref_smt(src, bound, true, false, "all");
            assert_eq!(
                ref_code, expect_exit,
                "ref exit at bound {bound} (full) {note}"
            );
            assert!(
                warnings.is_empty(),
                "no warnings with the flag: {warnings:?}"
            );
            assert!(
                !ref_out.contains("properties_omitted"),
                "no omission suffix with the flag:\n{ref_out}"
            );
            for backend in ["rust", "c"] {
                let bin = smt_binary(backend, src, tag);
                let (gen_code, gen_out, gen_err) =
                    run_bin_full(&bin, &["--bound", &bound.to_string(), "--full-properties"]);
                assert_eq!(
                    ref_code, gen_code,
                    "{backend} exit mismatch at bound {bound} (full) {note}"
                );
                assert_eq!(
                    ref_out, gen_out,
                    "{backend} stdout mismatch at bound {bound} (full) {note}\n--- ref ---\n{ref_out}\n--- {backend} ---\n{gen_out}"
                );
                assert_eq!(gen_err, "", "{backend} stderr must be empty (full)");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Task 3C - on_error blocks (D4) + assumptions (D3): the new contract,
// parity-pinned against BOTH generated backends exactly like the family above.
// ---------------------------------------------------------------------------

/// Parity helper for task 3C: ref vs both generated backends at every bound,
/// with the full-properties / full-assumptions flags and the --assumes
/// selection threaded through. Byte-identical stdout is asserted.
fn parity_task3c(
    src: &str,
    tag: &str,
    full_props: bool,
    full_assumes: bool,
    assumes: &str,
    expect_exit: impl Fn(usize) -> i32,
) {
    let note = version_note();
    for &bound in &BOUNDS {
        let (ref_code, ref_out, _warnings) = ref_smt(src, bound, full_props, full_assumes, assumes);
        assert_eq!(
            ref_code,
            expect_exit(bound),
            "ref exit at bound {bound} (full_props={full_props} full_assumes={full_assumes} assumes={assumes}) {note}"
        );
        for backend in ["rust", "c"] {
            let bin = smt_binary(backend, src, tag);
            let bound_s = bound.to_string();
            let mut args: Vec<&str> = vec!["--bound", &bound_s];
            if full_props {
                args.push("--full-properties");
            }
            if full_assumes {
                args.push("--full-assumptions");
            }
            if assumes != "all" {
                args.push("--assumes");
                args.push(assumes);
            }
            let (gen_code, gen_out) = run_bin(&bin, &args);
            assert_eq!(
                ref_code, gen_code,
                "{backend} exit mismatch at bound {bound} (full_props={full_props} full_assumes={full_assumes} assumes={assumes}) {note}"
            );
            assert_eq!(
                ref_out, gen_out,
                "{backend} stdout mismatch at bound {bound} (full_props={full_props} full_assumes={full_assumes} assumes={assumes}) {note}\n--- ref ---\n{ref_out}\n--- {backend} ---\n{gen_out}"
            );
        }
    }
}

/// (a) on_error PASS variant: the single unhandled input takes the fallback
/// to Safe at every step; the property holds because the fallback IS the
/// only successor. Deterministic PASS at every bound.
const SMT_OE_PASS_SRC: &str = r#"
module smt_oe_pass {
  enum State { A, Safe }
  initial A
  event Weird
  prop weird_lands_safe : always (input_is(Weird) -> next (state == Safe)) ;
  on_error { next Safe; }
}
"#;

/// (a) on_error FAIL variant: the fallback leads to the violation. Single
/// input class -> every step consumes Weird -> the violation is at sample 1
/// for every bound >= 2 (deterministic). The witness trace must carry the
/// [on_error] marker on the fallback step.
const SMT_OE_FAIL_SRC: &str = r#"
module smt_oe_fail {
  enum State { A, Safe }
  initial A
  event Weird
  prop never_safe : always (state != Safe) ;
  on_error { next Safe; }
}
"#;

/// (b) Cheap-assumption pruning: Bad sets x, the property forbids x > 0, the
/// assumption prunes Bad. WITH the assumption: PASS (no input survives
/// pruning - the path dies after the initial sample). WITH --assumes none:
/// FAIL (Bad is available and violates).
/// Bad is the ONLY event, so without the assumption every step consumes it
/// and x becomes 1 at sample 1 - the violation step is deterministic (sample
/// 1), keeping ref/generated stdout byte-identical. WITH the assumption no
/// input survives pruning: the path dies after the initial sample and the
/// property holds vacuously on the only alive sample.
const SMT_ASSUME_PRUNE_SRC: &str = r#"
module smt_assume_prune {
  enum State { A }
  initial A
  event Bad
  var x: bit<8> init 0
  transition A on Bad { next A; set x = 1; }
  prop x_zero : always (x == 0) ;
  assume no_bad : always (!input_is(Bad)) ;
}
"#;

/// The assumption-free TWIN of SMT_ASSUME_PRUNE_SRC: `--assumes none` on the
/// original must reproduce this spec's verdict byte-for-byte.
const SMT_ASSUME_PRUNE_NOASSUME_SRC: &str = r#"
module smt_assume_prune {
  enum State { A }
  initial A
  event Bad
  var x: bit<8> init 0
  transition A on Bad { next A; set x = 1; }
  prop x_zero : always (x == 0) ;
}
"#;

/// (c-FAIL) The assumed input ITSELF leads to a violation: the assumption
/// forces Tick at every input-consuming step (`next(input_is(Tick))` - the
/// `next` shifts the obligation off sample 0, where no input has been
/// consumed yet and a bare `input_is` would be statically false), so x
/// increments forever and `x < 3` fails at sample 3. Deterministic for bound
/// >= 4.
const SMT_ASSUME_FAIL_SRC: &str = r#"
module smt_assume_fail {
  enum State { A }
  initial A
  event Tick
  var x: bit<8> init 0
  transition A on Tick { next A; set x = x + 1; }
  prop x_bounded : always (x < 3) ;
  assume tick_ok : always (next(input_is(Tick))) ;
}
"#;

/// (d/e) Expensive assumption: nested temporal -> FULL class. WITHOUT
/// --full-assumptions it is omitted (warning + assumptions_omitted=1) and the
/// relaxed spec FAILs (NonTick x4 drives c to 4 at sample 4 - bound >= 5).
/// WITH the flag the assumption is asserted and the spec PASSES (c <= 3).
const SMT_ASSUME_FULL_SRC: &str = r#"
module smt_assume_full {
  enum State { A }
  initial A
  event Tick
  event NonTick
  var c: bit<8> init 0
  transition A on Tick { next A; set c = 0; }
  transition A on NonTick { next A; set c = c + 1; }
  prop bounded_counter : always (c < 4) ;
  assume tick_soon : always (bounded_eventually[3] (input_is(Tick))) ;
}
"#;

#[test]
fn task3c_on_error_pass_and_fail() {
    // PASS variant: the fallback is taken at every step and the property holds.
    parity_task3c(SMT_OE_PASS_SRC, "oe_pass", false, false, "all", |_| 0);
    // FAIL variant: deterministic step-1 violation for bound >= 2; the CE must
    // show the [on_error] marker on the fallback step.
    let note = version_note();
    parity_task3c(SMT_OE_FAIL_SRC, "oe_fail", false, false, "all", |b| {
        if b >= 2 {
            1
        } else {
            0
        }
    });
    for &bound in &BOUNDS {
        if bound < 2 {
            continue;
        }
        let (_code, out, _w) = ref_smt(SMT_OE_FAIL_SRC, bound, false, false, "all");
        assert!(
            out.contains("reason=property-violation label=never_safe step=1"),
            "pinned on_error FAIL summary missing at bound {bound} {note}:\n{out}"
        );
        assert!(
            out.contains("[on_error]"),
            "the witness trace must mark the fallback step at bound {bound} {note}:\n{out}"
        );
    }
}

#[test]
fn task3c_assumption_pruning() {
    // WITH the cheap assumption: Bad is pruned, x stays 0 -> PASS.
    parity_task3c(
        SMT_ASSUME_PRUNE_SRC,
        "assume_prune",
        false,
        false,
        "all",
        |_| 0,
    );
    // WITH --assumes none: Bad is available -> FAIL once the Bad step's
    // effect (x = 1) lands inside the checked horizon - bound 1 only sees
    // the initial sample, so it still passes. The violation step is
    // solver-chosen, so pin the reason/label, not the step.
    let note = version_note();
    parity_task3c(
        SMT_ASSUME_PRUNE_SRC,
        "assume_prune_none",
        false,
        false,
        "none",
        |b| {
            if b >= 2 {
                1
            } else {
                0
            }
        },
    );
    for &bound in &BOUNDS {
        let (_code, out, _w) = ref_smt(SMT_ASSUME_PRUNE_SRC, bound, false, false, "none");
        if bound >= 2 {
            assert!(
                out.contains("reason=property-violation label=x_zero"),
                "pinned pruned-assumption FAIL summary missing at bound {bound} {note}:\n{out}"
            );
        }
    }
    // --assumes none reproduces the assumption-free TWIN byte-for-byte (ref AND
    // both generated backends).
    for &bound in &BOUNDS {
        let (_c1, out_none, _w1) = ref_smt(SMT_ASSUME_PRUNE_SRC, bound, false, false, "none");
        let (_c2, out_twin, _w2) =
            ref_smt(SMT_ASSUME_PRUNE_NOASSUME_SRC, bound, false, false, "all");
        assert_eq!(out_none, out_twin, "ref: --assumes none must equal the twin at bound {bound} {note}\n--- none ---\n{out_none}\n--- twin ---\n{out_twin}");
        for backend in ["rust", "c"] {
            let bin_none = smt_binary(backend, SMT_ASSUME_PRUNE_SRC, "assume_prune_none");
            let bin_twin = smt_binary(backend, SMT_ASSUME_PRUNE_NOASSUME_SRC, "assume_prune_twin");
            let bound_s = bound.to_string();
            let (g1, o1) = run_bin(&bin_none, &["--bound", &bound_s, "--assumes", "none"]);
            let (g2, o2) = run_bin(&bin_twin, &["--bound", &bound_s]);
            assert_eq!(
                g1, g2,
                "{backend}: --assumes none exit must equal the twin at bound {bound}"
            );
            assert_eq!(o1, o2, "{backend}: --assumes none stdout must equal the twin at bound {bound}\n--- none ---\n{o1}\n--- twin ---\n{o2}");
        }
    }
}

#[test]
fn task3c_assumed_input_still_fails() {
    // The assumption is satisfied (every input IS Tick) but the assumed input
    // itself drives x past the bound: deterministic FAIL at sample 3 for
    // bound >= 4.
    parity_task3c(
        SMT_ASSUME_FAIL_SRC,
        "assume_fail",
        false,
        false,
        "all",
        |b| {
            if b >= 4 {
                1
            } else {
                0
            }
        },
    );
}

#[test]
fn task3c_expensive_assumption_omission_and_flag() {
    let note = version_note();
    // WITHOUT --full-assumptions: the nested-temporal assumption is omitted
    // (warning + assumptions_omitted=1) and the relaxed spec FAILs at bound >= 5.
    parity_task3c(
        SMT_ASSUME_FULL_SRC,
        "assume_full_off",
        false,
        false,
        "all",
        |b| {
            if b >= 5 {
                1
            } else {
                0
            }
        },
    );
    for &bound in &BOUNDS {
        let (_code, out, warnings) = ref_smt(SMT_ASSUME_FULL_SRC, bound, false, false, "all");
        assert!(
            warnings
                .iter()
                .any(|w| w.contains("omits assumption 'tick_soon' by default")),
            "pinned omission warning missing at bound {bound} {note}:\n{warnings:?}"
        );
        assert!(
            out.contains("assumptions_omitted=1"),
            "pinned assumptions_omitted suffix missing at bound {bound} {note}:\n{out}"
        );
        for backend in ["rust", "c"] {
            let bin = smt_binary(backend, SMT_ASSUME_FULL_SRC, "assume_full_off");
            let (gen_code, gen_out, gen_err) = run_bin_full(&bin, &["--bound", &bound.to_string()]);
            assert_eq!(
                gen_code,
                if bound >= 5 { 1 } else { 0 },
                "{backend} exit at bound {bound} (omitted) {note}"
            );
            assert!(
                gen_err.contains("omits assumption 'tick_soon' by default"),
                "{backend} stderr must carry the omission warning:\n{gen_err}"
            );
            assert!(
                gen_out.contains("assumptions_omitted=1"),
                "{backend} summary must carry the suffix:\n{gen_out}"
            );
        }
    }
    // WITH --full-assumptions: the assumption is asserted, c stays <= 3 -> PASS
    // at every bound, no warning, no suffix.
    parity_task3c(
        SMT_ASSUME_FULL_SRC,
        "assume_full_on",
        false,
        true,
        "all",
        |_| 0,
    );
    for &bound in &BOUNDS {
        let (_code, out, warnings) = ref_smt(SMT_ASSUME_FULL_SRC, bound, false, true, "all");
        assert!(
            warnings.is_empty(),
            "no warning with the flag:\n{warnings:?}"
        );
        assert!(
            !out.contains("assumptions_omitted"),
            "no omission suffix with the flag:\n{out}"
        );
    }
}

// ---------------------------------------------------------------------------
// Phase 18 task 3D - enum index encodings + multi-variant wildcards
// ---------------------------------------------------------------------------
// Every encoding is a bijection between an enum value and its
// declaration-order index in bit<ceil_log2(N)> (faithful to exact mode), so
// the pins below assert BOTH the verdict parity (ref vs both generated
// backends, byte-for-byte at every bound) AND that witness traces render the
// variant NAME, not the raw index.

/// (a-FAIL) Enum-typed var: ToGreen is the only input and its guard holds at
/// the initial sample (color=Red), so color becomes Green at sample 1 - the
/// violation step is deterministic (step 1, bound >= 2). The CE must print
/// `color=Green`, not `color=1`.
const SMT_ENUM_VAR_FAIL_SRC: &str = r#"
module smt_enum_var_fail {
  enum State { A }
  initial A
  enum Color { Red, Green, Blue }
  var color: Color init Color::Red
  event ToGreen
  transition A on ToGreen where color != Color::Green {
    next A;
    set color = Color::Green;
  }
  prop never_green : always (color != Color::Green) ;
}
"#;

/// (a-PASS) The var only ever holds Red or Green - Blue is unreachable.
const SMT_ENUM_VAR_PASS_SRC: &str = r#"
module smt_enum_var_pass {
  enum State { A }
  initial A
  enum Color { Red, Green, Blue }
  var color: Color init Color::Red
  event ToGreen
  transition A on ToGreen where color != Color::Green {
    next A;
    set color = Color::Green;
  }
  prop not_blue : always (color != Color::Blue) ;
}
"#;

/// (b-FAIL) Enum-typed concrete payload field: the guard admits only c=Blue
/// and Go is the ONLY transition, so keeping the path alive REQUIRES
/// consuming Go(G(c=Blue)) at step 0 - color becomes Blue at sample 1,
/// deterministic step 1 for bound >= 2. The CE must print the field by name
/// (`Go_G(c=Blue)`) and the var it flows into (`color=Blue`). Noop stays as a
/// second class (it has no transition - consuming it kills the path) so the
/// trace carries the event column.
const SMT_ENUM_FIELD_FAIL_SRC: &str = r#"
module smt_enum_field_fail {
  enum State { A }
  initial A
  enum Color { Red, Green, Blue }
  event Go(G(c: Color))
  event Noop
  var color: Color init Color::Red
  transition A on Go(G(c)) where c == Color::Blue {
    next A;
    set color = c;
  }
  prop never_blue : always (color != Color::Blue) ;
}
"#;

/// (b-PASS) The guard admits only c=Red - Blue never enters the system.
const SMT_ENUM_FIELD_PASS_SRC: &str = r#"
module smt_enum_field_pass {
  enum State { A }
  initial A
  enum Color { Red, Green, Blue }
  event Go(G(c: Color))
  event Noop
  var color: Color init Color::Red
  transition A on Go(G(c)) where c == Color::Red {
    next A;
    set color = c;
  }
  transition A on Noop { next A; }
  prop never_blue : always (color != Color::Blue) ;
}
"#;

/// (c-FAIL) Multi-variant wildcard: the bare `on Msg` pattern at A fires on
/// ANY of the group's variants and moves to B; at B only Good is accepted.
/// The property forbids consuming Bad - the ONLY way to violate is Bad at
/// step 0 (later, B accepts no Bad and Good-only keeps the path alive without
/// violating), so the violation is deterministic: sample 1, bound >= 2. The
/// CE carries `event=Msg_Bad()`.
const SMT_WILDCARD_FAIL_SRC: &str = r#"
module smt_wildcard_fail {
  enum State { A, B }
  initial A
  event Msg(Good)
  event Msg(Bad)
  transition A on Msg { next B; }
  transition B on Msg(Good) { next B; }
  prop bad_never_consumed : always (!input_is(Msg(Bad))) ;
}
"#;

/// (c-PASS) The wildcard's soundness pin: at every live sample EXACTLY ONE of
/// the group's variants came in, so the two input_is atoms are never true at
/// once. Before the per-arm `ev_i == ci` conjunct (task 3D), both arms were
/// driven by the same fires[t] and this spec FALSE-FAILED - the wildcard made
/// input_is(Msg(Ping)) hold while the consumed input was Msg(Pong).
const SMT_WILDCARD_PASS_SRC: &str = r#"
module smt_wildcard_pass {
  enum State { A }
  initial A
  event Msg(Ping)
  event Msg(Pong)
  transition A on Msg { next A; }
  prop not_both : always (!(input_is(Msg(Ping)) && input_is(Msg(Pong)))) ;
}
"#;

/// (d-FAIL, output side) Enum path in a property: the transition emits
/// Painted(P(Color::Green)) and sets color to the same literal; the pattern
/// demands Color::Red. Tick is the only input and fires at step 0, so the
/// mismatch lands at sample 1 - deterministic step 1 for bound >= 2 (single
/// class: fully forced trace). The verdict pins the output_has enum-index
/// constraint (a wrong index on either side would flip it); the CE prints
/// `color=Green` - the enum-literal assignment encoded as an index constant.
const SMT_ENUM_OUT_FAIL_SRC: &str = r#"
module smt_enum_out_fail {
  enum State { A }
  initial A
  enum Color { Red, Green }
  output Painted(P(c: Color))
  var color: Color init Color::Red
  event Tick
  transition A on Tick {
    next A;
    set color = Color::Green;
    emit Painted(P(Color::Green));
  }
  prop only_red : always (input_is(Tick) -> output_has(Painted(P(Color::Red)))) ;
}
"#;

/// (d-PASS, output side) The same machine, pattern demanding the variant that
/// is actually emitted - the enum-index constraint matches on both sides.
const SMT_ENUM_OUT_PASS_SRC: &str = r#"
module smt_enum_out_pass {
  enum State { A }
  initial A
  enum Color { Red, Green }
  output Painted(P(c: Color))
  var color: Color init Color::Red
  event Tick
  transition A on Tick {
    next A;
    set color = Color::Green;
    emit Painted(P(Color::Green));
  }
  prop only_green : always (input_is(Tick) -> output_has(Painted(P(Color::Green)))) ;
}
"#;

/// (d-FAIL, input side) Enum-variant input pattern: the guard admits only
/// c=Green and Go is the ONLY transition, so the machine is FORCED to consume
/// Go(G(c=Green)) at step 0 - the pattern matches at sample 1 and the
/// property is violated there, deterministic for bound >= 2. This pins the
/// enum-index constraint in the input_is arm: if the pattern's Color::Green
/// did not encode as the same index the field carries, the atom would never
/// match and the spec would false-PASS.
const SMT_ENUM_IN_FAIL_SRC: &str = r#"
module smt_enum_in_fail {
  enum State { A }
  initial A
  enum Color { Red, Green }
  event Go(G(c: Color))
  event Noop
  transition A on Go(G(c)) where c == Color::Green { next A; }
  prop never_green_in : always (!input_is(Go(G(Color::Green)))) ;
}
"#;

/// (d-PASS, input side) The guard admits only c=Red - the Green pattern is
/// never matched.
const SMT_ENUM_IN_PASS_SRC: &str = r#"
module smt_enum_in_pass {
  enum State { A }
  initial A
  enum Color { Red, Green }
  event Go(G(c: Color))
  event Noop
  transition A on Go(G(c)) where c == Color::Red { next A; }
  transition A on Noop { next A; }
  prop never_green_in : always (!input_is(Go(G(Color::Green)))) ;
}
"#;

/// Task 3D parity driver: ref in-process vs BOTH generated backends, full
/// stdout byte-comparison + exit code at every listed bound (the specs carry
/// no assumptions and no FULL-class properties, so no flags are needed).
fn parity_task3d(src: &str, tag: &str, bounds: &[usize], expect_exit: impl Fn(usize) -> i32) {
    let note = version_note();
    for &bound in bounds {
        let (ref_code, ref_out, _warnings) = ref_smt(src, bound, false, false, "all");
        assert_eq!(
            ref_code,
            expect_exit(bound),
            "ref exit at bound {bound} {note}"
        );
        for backend in ["rust", "c"] {
            let bin = smt_binary(backend, src, tag);
            let (gen_code, gen_out) = run_bin(&bin, &["--bound", &bound.to_string()]);
            assert_eq!(
                ref_code, gen_code,
                "{backend} exit mismatch at bound {bound} {note}"
            );
            assert_eq!(
                ref_out, gen_out,
                "{backend} stdout mismatch at bound {bound} {note}\n--- ref ---\n{ref_out}\n--- {backend} ---\n{gen_out}"
            );
        }
    }
}

/// Exit-code-only parity at bounds where the witness trace carries FREE
/// variables (multi-class FAIL specs: after the path dies, the unconstrained
/// ev/fld vars take solver-version-dependent values - the VERDICT stays
/// deterministic, the trace bytes do not). Byte-parity is pinned at the
/// forced-trace bounds by the caller.
fn parity_task3d_exit_only(
    src: &str,
    tag: &str,
    bounds: &[usize],
    expect_exit: impl Fn(usize) -> i32,
) {
    let note = version_note();
    for &bound in bounds {
        let (ref_code, _ref_out, _warnings) = ref_smt(src, bound, false, false, "all");
        assert_eq!(
            ref_code,
            expect_exit(bound),
            "ref exit at bound {bound} {note}"
        );
        for backend in ["rust", "c"] {
            let bin = smt_binary(backend, src, tag);
            let (gen_code, _gen_out) = run_bin(&bin, &["--bound", &bound.to_string()]);
            assert_eq!(
                ref_code, gen_code,
                "{backend} exit mismatch at bound {bound} {note}"
            );
        }
    }
}

#[test]
fn task3d_enum_var_pass_and_fail() {
    // (a-PASS) Blue is unreachable at every bound. Single-class spec: no ev
    // variable, every step forced - full parity at every bound.
    parity_task3d(SMT_ENUM_VAR_PASS_SRC, "enum_var_pass", &BOUNDS, |_| 0);
    // (a-FAIL) deterministic step-1 violation for bound >= 2; the CE prints
    // the variant NAME, not the index. Single-class: fully deterministic
    // trace at every bound.
    let note = version_note();
    parity_task3d(SMT_ENUM_VAR_FAIL_SRC, "enum_var_fail", &BOUNDS, |b| {
        if b >= 2 {
            1
        } else {
            0
        }
    });
    for &bound in &BOUNDS {
        if bound < 2 {
            continue;
        }
        let (_code, out, _w) = ref_smt(SMT_ENUM_VAR_FAIL_SRC, bound, false, false, "all");
        assert!(
            out.contains("reason=property-violation label=never_green step=1"),
            "pinned enum-var FAIL summary missing at bound {bound} {note}:\n{out}"
        );
        assert!(
            out.contains("color=Green"),
            "the CE must print the variant name, not the index, at bound {bound} {note}:\n{out}"
        );
        assert!(
            !out.contains("color=1}"),
            "raw index leaked into the witness at bound {bound} {note}:\n{out}"
        );
    }
}

#[test]
fn task3d_enum_field_pass_and_fail() {
    // (b-PASS) only Red is admitted - PASS prints no trace, so full parity
    // holds at every bound.
    parity_task3d(SMT_ENUM_FIELD_PASS_SRC, "enum_field_pass", &BOUNDS, |_| 0);
    // (b-FAIL) step 0's input is forced by the violation requirement
    // (Go+Blue is the only way to both stay alive and violate), so the
    // bound-2 trace is fully deterministic - byte-parity there. Beyond it,
    // the dead path's free vars are version-dependent - exit parity only.
    let note = version_note();
    parity_task3d(SMT_ENUM_FIELD_FAIL_SRC, "enum_field_fail", &[1, 2], |b| {
        if b >= 2 {
            1
        } else {
            0
        }
    });
    parity_task3d_exit_only(SMT_ENUM_FIELD_FAIL_SRC, "enum_field_fail", &[5], |_| 1);
    let (_code, out, _w) = ref_smt(SMT_ENUM_FIELD_FAIL_SRC, 2, false, false, "all");
    assert!(
        out.contains("reason=property-violation label=never_blue step=1"),
        "pinned enum-field FAIL summary missing {note}:\n{out}"
    );
    assert!(
        out.contains("Go_G(c=Blue)"),
        "the CE must print the enum field by name {note}:\n{out}"
    );
    assert!(
        out.contains("color=Blue"),
        "the var fed by the enum field must print by name {note}:\n{out}"
    );
}

#[test]
fn task3d_wildcard_pass_and_fail() {
    // (c-PASS) exactly one variant per live sample - the soundness pin for
    // the per-arm ev_i == ci conjunct. PASS prints no trace: full parity at
    // every bound.
    parity_task3d(SMT_WILDCARD_PASS_SRC, "wildcard_pass", &BOUNDS, |_| 0);
    // (c-FAIL) Bad is forced at step 0 (the only way to violate - later, B
    // accepts no Bad), so the bound-2 trace is fully deterministic -
    // byte-parity there; beyond it the dead path's free ev vars are
    // version-dependent - exit parity only.
    let note = version_note();
    parity_task3d(SMT_WILDCARD_FAIL_SRC, "wildcard_fail", &[1, 2], |b| {
        if b >= 2 {
            1
        } else {
            0
        }
    });
    parity_task3d_exit_only(SMT_WILDCARD_FAIL_SRC, "wildcard_fail", &[5], |_| 1);
    let (_code, out, _w) = ref_smt(SMT_WILDCARD_FAIL_SRC, 2, false, false, "all");
    assert!(
        out.contains("reason=property-violation label=bad_never_consumed step=1"),
        "pinned wildcard FAIL summary missing {note}:\n{out}"
    );
    assert!(
        out.contains("event=Msg_Bad()"),
        "the CE must show the variant that actually came in {note}:\n{out}"
    );
}

#[test]
fn task3d_enum_patterns_pass_and_fail() {
    // (d-PASS, output) the emitted variant matches the pattern's index - PASS
    // prints no trace: full parity at every bound.
    parity_task3d(SMT_ENUM_OUT_PASS_SRC, "enum_out_pass", &BOUNDS, |_| 0);
    // (d-FAIL, output) single-class spec: every step is forced (Tick is the
    // only input and fires everywhere), so the trace is deterministic at all
    // bounds; the CE prints the emitted variant by name.
    let note = version_note();
    parity_task3d(SMT_ENUM_OUT_FAIL_SRC, "enum_out_fail", &BOUNDS, |b| {
        if b >= 2 {
            1
        } else {
            0
        }
    });
    for &bound in &BOUNDS {
        if bound < 2 {
            continue;
        }
        let (_code, out, _w) = ref_smt(SMT_ENUM_OUT_FAIL_SRC, bound, false, false, "all");
        assert!(
            out.contains("reason=property-violation label=only_red step=1"),
            "pinned enum-output FAIL summary missing at bound {bound} {note}:\n{out}"
        );
        assert!(
            out.contains("color=Green"),
            "the enum-literal assignment must print by name at bound {bound} {note}:\n{out}"
        );
    }
    // (d-PASS, input) only Red is admitted - the Green pattern never matches.
    parity_task3d(SMT_ENUM_IN_PASS_SRC, "enum_in_pass", &BOUNDS, |_| 0);
    // (d-FAIL, input) Green is forced at step 0 (Go+Green is the only
    // transition), so the bound-2 trace is fully deterministic - byte-parity
    // there; beyond it the dead path's free vars are version-dependent - exit
    // parity only.
    parity_task3d(SMT_ENUM_IN_FAIL_SRC, "enum_in_fail", &[1, 2], |b| {
        if b >= 2 {
            1
        } else {
            0
        }
    });
    parity_task3d_exit_only(SMT_ENUM_IN_FAIL_SRC, "enum_in_fail", &[5], |_| 1);
    let (_code, out, _w) = ref_smt(SMT_ENUM_IN_FAIL_SRC, 2, false, false, "all");
    assert!(
        out.contains("reason=property-violation label=never_green_in step=1"),
        "pinned enum-input FAIL summary missing {note}:\n{out}"
    );
    assert!(
        out.contains("Go_G(c=Green)"),
        "the consumed enum field must print by name {note}:\n{out}"
    );
}
