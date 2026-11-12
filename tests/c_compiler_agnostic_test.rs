//! C-backend compiler-agnosticism regression suite.
//!
//! The generated C must compile warning-free and behave identically under any
//! C11 compiler, not just gcc. This suite pins the defects found when the
//! model-checking tests ran under nix's clang toolchain (and one more that a
//! full -O0..-O2 x gcc/clang audit then exposed):
//!
//!   1. clang -Wall -Wunused-function on the shared helper API (static inline
//!      helpers a given program does not call) - fixed by a guarded pragma in
//!      the C prologue.
//!   2. clang -Wall -Wparentheses-equality on single-equality guards rendered
//!      as `if ((c == X))` - fixed by strip_redundant_outer_parens at both
//!      guard-emission sites.
//!   3. use-after-scope UB: the --violate label buffer was branch-local, so
//!      specs[i].label dangled after the block ended; clang -O2 reused the slot
//!      and forced violations never matched (completed=N instead of
//!      assumption_stops=N) - fixed by hoisting the buffer to function scope.
//!   4. gcc -O1+ -Wmaybe-uninitialized on the `mc_effect_t effs[]` arrays (gcc
//!      cannot track writes through mc_step_all's out pointer) - fixed by
//!      zero-initializing at every call site.
//!
//! Legs:
//!   * warning-free compile of every generated variant (mc / production / test)
//!     for every property-bearing example, under gcc and (when present) clang,
//!     at -O2 - the level where defects 3 and 4 manifest. The -O0 gcc leg is
//!     already pinned by backend_parity_test's compile_c.
//!   * source-level guard rendering: no `if ((a == b))` double-parenthesized
//!     single-equality guard anywhere in the generated C (defect 2, checked
//!     without needing a compiler).
//!   * forced-violation parity across compilers: `--violate <label>:1.0:100%`
//!     must give byte-identical output under gcc -O2 and clang -O2 alike, and
//!     on the anchored safety-assumption example it must stop every run
//!     (assumption_stops=N, completed=0) - defect 3.

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// All valid examples carrying `prop `/`assume ` declarations (dynamic set -
/// same enumeration as backend_parity_test, so future additions are covered).
fn property_bearing_examples() -> Vec<PathBuf> {
    let files = rfs::collect_rfs_files(root().join("examples/valid")).expect("collect .rfs files");
    let out: Vec<PathBuf> = files
        .into_iter()
        .filter(|p| {
            let src = std::fs::read_to_string(p).unwrap_or_else(|e| panic!("read {:?}: {e}", p));
            src.lines().any(|l| {
                let t = l.trim_start();
                t.starts_with("prop ") || t.starts_with("assume ")
            })
        })
        .collect();
    assert!(!out.is_empty(), "dynamic enumeration found no property-bearing examples");
    out
}

fn generate(backend_name: &str, command: rfs::codegen::CodegenCommandKind, spec: &Path) -> String {
    let src = std::fs::read_to_string(spec).unwrap_or_else(|e| panic!("read {:?}: {e}", spec));
    let ir = rfs::parse_source(&src)
        .unwrap_or_else(|e| panic!("{:?} must parse ({backend_name}): {e}", spec));
    assert_eq!(ir.modules.len(), 1);
    let backend = rfs::codegen::get_backend(backend_name)
        .unwrap_or_else(|| panic!("backend {backend_name} registered"));
    let options = rfs::codegen::BackendOptions {
        assumptions: rfs::codegen::AssumptionSelection::All,
        steps: 4,
        with_property_monitors: false,
        ..rfs::codegen::BackendOptions::default()
    };
    command
        .run(backend, &ir, &options)
        .unwrap_or_else(|e| panic!("codegen {backend_name} {:?} must succeed: {e:?}", spec))
        .code
}

/// Compile `src` with `cc` at -O2 -Wall into `obj`; assert success and zero
/// warnings. A real `-c` compile (not -fsyntax-only) is required: gcc's
/// -Wmaybe-uninitialized only fires in the optimization passes.
fn compile_warning_free(cc: &str, src: &Path, obj: &Path) {
    let out = Command::new(cc)
        .args(["-Wall", "-O2"])
        .arg(src)
        .arg("-c")
        .arg("-o")
        .arg(obj)
        .output()
        .unwrap_or_else(|e| panic!("run {cc} on {:?}: {e}", src));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "{cc} -O2 failed on {}:\n{stderr}",
        src.display()
    );
    assert!(
        !stderr.lines().any(|l| l.contains("warning:")),
        "{cc} -O2 -Wall emitted warnings on {} (generated C must be warning-free under every compiler):\n{stderr}",
        src.display()
    );
}

// ---------------------------------------------------------------------------
// Leg 1: warning-free compile under every available compiler, all variants
// ---------------------------------------------------------------------------

#[test]
fn generated_c_compiles_warning_free_under_every_compiler() {
    let dir = root().join("target").join("rfs_c_agnostic");
    std::fs::create_dir_all(&dir).expect("create scratch dir");
    let gcc = common::toolchain::gcc();
    let have_clang = common::toolchain::available_named("clang");
    if !have_clang {
        eprintln!("note: clang not available - the clang leg is skipped (gcc leg still runs)");
    }

    let mut compiled = 0;
    for spec in property_bearing_examples() {
        let stem = spec.file_stem().unwrap().to_string_lossy().into_owned();
        for (variant, command) in [
            ("mc", rfs::codegen::CodegenCommandKind::ModelChecker),
            ("production", rfs::codegen::CodegenCommandKind::Production),
            ("test", rfs::codegen::CodegenCommandKind::UnitTests),
        ] {
            let src = dir.join(format!("{stem}_{variant}.c"));
            std::fs::write(&src, generate("c", command, &spec)).expect("write generated C");
            compile_warning_free(&gcc, &src, &dir.join(format!("{stem}_{variant}_gcc.o")));
            if have_clang {
                compile_warning_free(
                    &common::toolchain::clang(),
                    &src,
                    &dir.join(format!("{stem}_{variant}_clang.o")),
                );
            }
            compiled += 1;
        }
    }
    assert!(compiled >= 3, "expected at least one example x 3 variants, got {compiled}");
}

// ---------------------------------------------------------------------------
// Leg 2: no double-parenthesized single-equality guards in the generated C
// ---------------------------------------------------------------------------

/// True if `line` contains an `if ((a == b))` guard - a single equality
/// comparison wrapped in one redundant outer paren pair (the shape clang's
/// -Wparentheses-equality flags). Compound guards like `((a == b) && c)` are
/// fine: their outer parens are load-bearing.
fn has_double_paren_equality_guard(line: &str) -> bool {
    let Some(idx) = line.find("if (") else { return false };
    let rest = &line[idx + 4..]; // starts at the `(` of the if-parens
    let Some(inner_open) = rest.find('(') else { return false };
    let inner = &rest[inner_open..];
    // The guard is a single equality iff the text between the second `(` and
    // its matching `)` contains exactly one `==` and no other operator that
    // would make the outer parens load-bearing.
    let Some(close) = inner.rfind(')') else { return false };
    if close < inner.len() - 1 {
        return false; // more text after the closing paren: not a bare guard
    }
    let expr = &inner[1..close];
    if expr.contains('(') || expr.contains(')') {
        return false; // nested parens: compound expression, outer pair is needed
    }
    let n_eq = expr.matches("==").count();
    n_eq == 1 && !expr.contains("&&") && !expr.contains("||") && !expr.contains("!=")
}

#[test]
fn no_double_parenthesized_equality_guards() {
    for spec in property_bearing_examples() {
        let stem = spec.file_stem().unwrap().to_string_lossy().into_owned();
        for variant in ["mc", "production"] {
            let code = generate(
                "c",
                match variant {
                    "mc" => rfs::codegen::CodegenCommandKind::ModelChecker,
                    _ => rfs::codegen::CodegenCommandKind::Production,
                },
                &spec,
            );
            for (lineno, line) in code.lines().enumerate() {
                assert!(
                    !has_double_paren_equality_guard(line),
                    "{stem}_{variant}: line {lineno} renders a single-equality guard as if ((...)): {}",
                    line.trim()
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Leg 3: forced-violation parity across compilers (the use-after-scope UB)
// ---------------------------------------------------------------------------

/// First `assume <label> :` label in the spec source, if any.
fn first_assume_label(spec: &Path) -> Option<String> {
    let src = std::fs::read_to_string(spec).unwrap_or_else(|e| panic!("read {:?}: {e}", spec));
    src.lines().find_map(|l| {
        let t = l.trim_start();
        let t = t.strip_prefix("assume ")?;
        let label: String = t.chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '_').collect();
        (!label.is_empty()).then_some(label)
    })
}

#[test]
fn forced_violation_parity_across_compilers() {
    let dir = root().join("target").join("rfs_c_agnostic");
    std::fs::create_dir_all(&dir).expect("create scratch dir");
    let gcc = common::toolchain::gcc();
    let have_clang = common::toolchain::available_named("clang");

    const RUNS: i32 = 7;
    let mut checked = 0;
    for spec in property_bearing_examples() {
        let Some(label) = first_assume_label(&spec) else { continue };
        let stem = spec.file_stem().unwrap().to_string_lossy().into_owned();
        let src = dir.join(format!("{stem}_vio.c"));
        std::fs::write(&src, generate("c", rfs::codegen::CodegenCommandKind::ModelChecker, &spec))
            .expect("write MC source");

        let mut bins: Vec<(String, PathBuf)> = Vec::new();
        for (cc, name) in [(&gcc, "gcc"), (&common::toolchain::clang(), "clang")] {
            if name == "clang" && !have_clang {
                continue;
            }
            let bin = dir.join(format!("{stem}_vio_{name}"));
            let out = Command::new(cc)
                .args(["-Wall", "-O2"])
                .arg(&src)
                .arg("-o")
                .arg(&bin)
                .output()
                .expect("compile MC binary");
            assert!(out.status.success(), "compile failed for {stem} with {name}");
            bins.push((name.to_string(), bin));
        }

        let args: Vec<String> = vec![
            "--mode".into(),
            "fuzz".into(),
            "--runs".into(),
            RUNS.to_string(),
            "--bound".into(),
            "10".into(),
            "--seed".into(),
            "987654".into(),
            "--violate".into(),
            format!("{label}:1.0:100%"),
        ];
        let mut outputs: Vec<(String, String, i32)> = Vec::new();
        for (name, bin) in &bins {
            let out = Command::new(bin).args(&args).output().unwrap_or_else(|e| panic!("run {bin:?}: {e}"));
            outputs.push((name.clone(), String::from_utf8_lossy(&out.stdout).into_owned(), out.status.code().unwrap_or(-1)));
        }

        // Cross-compiler parity: identical stdout and exit code. Under the old
        // branch-local buffer, clang -O2 diverged here (completed=N instead of
        // assumption_stops=N) because the dangling label never matched.
        for i in 1..outputs.len() {
            assert_eq!(
                outputs[i].1, outputs[0].1,
                "{stem}: forced-violation output differs between {} and {}:\n--- {} ---\n{}\n--- {} ---\n{}",
                outputs[0].0, outputs[i].0, outputs[0].0, outputs[0].1, outputs[i].0, outputs[i].1
            );
            assert_eq!(outputs[i].2, outputs[0].2, "{stem}: exit code differs across compilers");
        }

        // Anchored behaviour: on the safety-assumption demo every run must stop
        // at the forced violation (this is the exact case the UB broke).
        if stem == "37_assumption_demo" {
            let summary = &outputs[0].1;
            assert!(
                summary.contains(&format!("assumption_stops={RUNS}")) && summary.contains("completed=0"),
                "37_assumption_demo: forced violation must stop every run, got:\n{summary}"
            );
        }
        checked += 1;
    }
    assert!(checked >= 1, "no assume-bearing examples found to check");
}
