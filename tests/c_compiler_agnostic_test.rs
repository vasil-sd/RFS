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
    assert!(
        !out.is_empty(),
        "dynamic enumeration found no property-bearing examples"
    );
    out
}

fn generate(backend_name: &str, command: rfs::codegen::CodegenCommandKind, spec: &Path) -> String {
    generate_full(backend_name, command, spec)
        .primary_code()
        .to_string()
}

fn generate_full(
    backend_name: &str,
    command: rfs::codegen::CodegenCommandKind,
    spec: &Path,
) -> rfs::codegen::CodegenOutput {
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
}

fn c_options(header_name: Option<&str>) -> rfs::codegen::BackendOptions {
    rfs::codegen::BackendOptions {
        assumptions: rfs::codegen::AssumptionSelection::All,
        steps: 4,
        with_property_monitors: false,
        fsm_header_name: header_name.map(|s| s.to_string()),
        ..rfs::codegen::BackendOptions::default()
    }
}

fn generate_with_header(
    backend_name: &str,
    command: rfs::codegen::CodegenCommandKind,
    spec: &Path,
    header_name: &str,
) -> String {
    let src = std::fs::read_to_string(spec).unwrap();
    let ir = rfs::parse_source(&src).unwrap();
    let backend = rfs::codegen::get_backend(backend_name).unwrap();
    command
        .run(backend, &ir, &c_options(Some(header_name)))
        .unwrap()
        .primary_code()
        .to_string()
}

fn c_production_header(
    command: rfs::codegen::CodegenCommandKind,
    spec: &Path,
    header_name: &str,
) -> String {
    let src = std::fs::read_to_string(spec).unwrap();
    let ir = rfs::parse_source(&src).unwrap();
    let backend = rfs::codegen::get_backend("c").unwrap();
    command
        .run(backend, &ir, &c_options(Some(header_name)))
        .unwrap()
        .fsm_header
        .clone()
        .expect("C production fills the fsm-header slot")
}

fn compile_warning_free_i(cc: &str, src: &Path, inc: &Path, obj: &Path) {
    let out = Command::new(cc)
        .args(["-Wall", "-O2", "-I"])
        .arg(inc)
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
        "{cc} -O2 -Wall emitted warnings on {}:\n{stderr}",
        src.display()
    );
}

/// Phase 14 task 3: build a C MC binary from the four-file artifact set under
/// compiler `cc` at -O2 -Wall. Writes fsm.h/fsm.c/mc.h/mc.c into `subdir` under
/// their standalone default names, compiles BOTH sources to objects (zero-
/// warning bar), and links them into `bin`. Used by the gcc/clang parity legs.
fn build_mc_pair(cc: &str, out: &rfs::codegen::CodegenOutput, subdir: &Path, bin: &Path) {
    std::fs::create_dir_all(subdir).expect("create mc pair subdir");
    let fsm_h = subdir.join("rfs_fsm.h");
    let fsm_c = subdir.join("rfs_fsm.c");
    let mc_h = subdir.join("rfs_mc.h");
    let mc_c = subdir.join("rfs_mc.c");
    std::fs::write(
        &fsm_h,
        out.fsm_header.as_deref().expect("c mc fills fsm-header"),
    )
    .expect("w");
    std::fs::write(
        &fsm_c,
        out.fsm_code.as_deref().expect("c mc fills fsm-code"),
    )
    .expect("w");
    std::fs::write(
        &mc_h,
        out.mc_header.as_deref().expect("c mc fills mc-header"),
    )
    .expect("w");
    std::fs::write(&mc_c, out.mc_code.as_deref().expect("c mc fills mc-code")).expect("w");
    compile_warning_free_i(cc, &fsm_c, subdir, &subdir.join("rfs_fsm.o"));
    compile_warning_free_i(cc, &mc_c, subdir, &subdir.join("rfs_mc.o"));
    let linked = Command::new(cc)
        .args(["-O2"])
        .arg(subdir.join("rfs_fsm.o"))
        .arg(subdir.join("rfs_mc.o"))
        .arg("-o")
        .arg(bin)
        .output()
        .expect("link mc objects");
    assert!(
        linked.status.success(),
        "link of {cc} mc objects failed:\n{}",
        String::from_utf8_lossy(&linked.stderr)
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
            if variant == "production" {
                // Phase 14 task 2: C production is a header/source pair. Stamp
                // a per-spec header name (so specs don't collide on the default
                // rfs_fsm.h), write both files, and compile the source with -I.
                let src_text = generate_with_header("c", command, &spec, &format!("{stem}_fsm.h"));
                std::fs::write(&src, &src_text).expect("write generated C source");
                let header = c_production_header(command, &spec, &format!("{stem}_fsm.h"));
                std::fs::write(dir.join(format!("{stem}_fsm.h")), header).expect("write header");
                compile_warning_free_i(
                    &gcc,
                    &src,
                    &dir,
                    &dir.join(format!("{stem}_{variant}_gcc.o")),
                );
                if have_clang {
                    compile_warning_free_i(
                        &common::toolchain::clang(),
                        &src,
                        &dir,
                        &dir.join(format!("{stem}_{variant}_clang.o")),
                    );
                }
            } else if variant == "mc" {
                // Phase 14 task 3: the C MC is a header/source pair referencing
                // the fsm pair - compile BOTH sources under each compiler and
                // link (the single-file shape no longer exists).
                let out = generate_full("c", command, &spec);
                build_mc_pair(
                    &gcc,
                    &out,
                    &dir.join(format!("{stem}_mc_gcc")),
                    &dir.join(format!("{stem}_{variant}_gcc.bin")),
                );
                if have_clang {
                    build_mc_pair(
                        &common::toolchain::clang(),
                        &out,
                        &dir.join(format!("{stem}_mc_clang")),
                        &dir.join(format!("{stem}_{variant}_clang.bin")),
                    );
                }
            } else {
                // Phase 14 task 4: the C test source references the fsm header -
                // compile BOTH sources under each compiler (object+link shape).
                let out = generate_full("c", command, &spec);
                for (cc, tag) in [(&gcc, "gcc"), (&common::toolchain::clang(), "clang")] {
                    if tag == "clang" && !have_clang {
                        continue;
                    }
                    let sub = dir.join(format!("{stem}_test_{tag}"));
                    std::fs::create_dir_all(&sub).expect("create test pair dir");
                    let fsm_h = sub.join("rfs_fsm.h");
                    let fsm_c = sub.join("rfs_fsm.c");
                    let test_c = sub.join("spec_test.c");
                    std::fs::write(
                        &fsm_h,
                        out.fsm_header
                            .as_deref()
                            .expect("c test fills the fsm-header slot"),
                    )
                    .expect("write fsm header");
                    std::fs::write(
                        &fsm_c,
                        out.fsm_code
                            .as_deref()
                            .expect("c test fills the fsm-code slot"),
                    )
                    .expect("write fsm source");
                    std::fs::write(&test_c, out.primary_code()).expect("write test source");
                    compile_warning_free_i(cc, &fsm_c, &sub, &sub.join("rfs_fsm.o"));
                    compile_warning_free_i(cc, &test_c, &sub, &sub.join("spec_test.o"));
                    let linked = Command::new(cc)
                        .args(["-O2"])
                        .arg(sub.join("rfs_fsm.o"))
                        .arg(sub.join("spec_test.o"))
                        .arg("-o")
                        .arg(dir.join(format!("{stem}_{variant}_{tag}.bin")))
                        .output()
                        .expect("link test objects");
                    assert!(
                        linked.status.success(),
                        "{cc} link of c test objects failed:\n{}",
                        String::from_utf8_lossy(&linked.stderr)
                    );
                }
            }
            compiled += 1;
        }
    }
    assert!(
        compiled >= 3,
        "expected at least one example x 3 variants, got {compiled}"
    );
}

// ---------------------------------------------------------------------------
// Leg 2: no double-parenthesized single-equality guards in the generated C
// ---------------------------------------------------------------------------

/// True if `line` contains an `if ((a == b))` guard - a single equality
/// comparison wrapped in one redundant outer paren pair (the shape clang's
/// -Wparentheses-equality flags). Compound guards like `((a == b) && c)` are
/// fine: their outer parens are load-bearing.
fn has_double_paren_equality_guard(line: &str) -> bool {
    let Some(idx) = line.find("if (") else {
        return false;
    };
    let rest = &line[idx + 4..]; // starts at the `(` of the if-parens
    let Some(inner_open) = rest.find('(') else {
        return false;
    };
    let inner = &rest[inner_open..];
    // The guard is a single equality iff the text between the second `(` and
    // its matching `)` contains exactly one `==` and no other operator that
    // would make the outer parens load-bearing.
    let Some(close) = inner.rfind(')') else {
        return false;
    };
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
        let label: String = t
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect();
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
        let Some(label) = first_assume_label(&spec) else {
            continue;
        };
        let stem = spec.file_stem().unwrap().to_string_lossy().into_owned();
        // Phase 14 task 3: object+link build of the mc/fsm pair under each
        // compiler (the single-file MC source no longer exists).
        let out = generate_full("c", rfs::codegen::CodegenCommandKind::ModelChecker, &spec);

        let mut bins: Vec<(String, PathBuf)> = Vec::new();
        for (cc, name) in [(&gcc, "gcc"), (&common::toolchain::clang(), "clang")] {
            if name == "clang" && !have_clang {
                continue;
            }
            let bin = dir.join(format!("{stem}_vio_{name}"));
            build_mc_pair(cc, &out, &dir.join(format!("{stem}_vio_{name}_pair")), &bin);
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
            let out = Command::new(bin)
                .args(&args)
                .output()
                .unwrap_or_else(|e| panic!("run {bin:?}: {e}"));
            outputs.push((
                name.clone(),
                String::from_utf8_lossy(&out.stdout).into_owned(),
                out.status.code().unwrap_or(-1),
            ));
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
            assert_eq!(
                outputs[i].2, outputs[0].2,
                "{stem}: exit code differs across compilers"
            );
        }

        // Anchored behaviour: on the safety-assumption demo every run must stop
        // at the forced violation (this is the exact case the UB broke).
        if stem == "37_assumption_demo" {
            let summary = &outputs[0].1;
            assert!(
                summary.contains(&format!("assumption_stops={RUNS}"))
                    && summary.contains("completed=0"),
                "37_assumption_demo: forced violation must stop every run, got:\n{summary}"
            );
        }
        checked += 1;
    }
    assert!(checked >= 1, "no assume-bearing examples found to check");
}
