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
    )
}

// ---------------------------------------------------------------------------
// Per-process binary cache + codegen
// ---------------------------------------------------------------------------

static BIN_CACHE: std::sync::LazyLock<
    std::sync::Mutex<std::collections::HashMap<String, PathBuf>>,
> = std::sync::LazyLock::new(|| std::sync::Mutex::new(std::collections::HashMap::new()));

fn cached_binary(key: &str, build: impl FnOnce() -> PathBuf) -> PathBuf {
    let mut cache = BIN_CACHE.lock().expect("bin cache lock");
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
fn ref_smt(src: &str, bound: usize) -> (i32, String) {
    let ir = rfs::parse_source(src).unwrap_or_else(|e| panic!("spec must parse: {e}"));
    assert_eq!(ir.modules.len(), 1);
    let module = &ir.modules[0];
    let out = rfs::mc::ref_mc::run_smt(module, bound)
        .unwrap_or_else(|e| panic!("ref-mc smt run failed: {e}"));
    (out.exit_code, out.output)
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
        let (ref_code, ref_out) = ref_smt(SMT_FAIL_SRC, bound);
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
        let (ref_code, ref_out) = ref_smt(SMT_LIV_SRC, bound);
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
        let (ref_code, ref_out) = ref_smt(SMT_PASS_SRC, bound);
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
fn ref_smt_rejects_assumption_and_on_error_specs() {
    // build_smt_spec's rejection errors surface verbatim (same as codegen).
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
    let err = rfs::mc::ref_mc::run_smt(&ir.modules[0], 3).expect_err("must reject");
    assert!(
        err.starts_with("smt-bounded mode does not support assumptions"),
        "{err}"
    );
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
    let err = rfs::mc::ref_mc::run_smt(&ir.modules[0], 3).expect_err("must reject");
    assert_eq!(err, "smt-bounded mode does not support on_error blocks");
}
