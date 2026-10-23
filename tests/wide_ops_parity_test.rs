//! Phase 6 task 2a: targeted cross-backend differential for WIDE bitvector
//! ops in generated C. The pre-2a C lowering was UNDEFINED BEHAVIOR for two
//! input classes (docs/SEMANTICS.md section 2.2, footnote c-ub):
//!
//!   * variable shifts by amounts >= width - RFS semantics say the result is
//!     0, but the old rendering `(l) << (uint64_t)(r)` relied on C integer
//!     promotion and was UB once the amount reached the promoted LHS width;
//!     gcc wraps the count mod 32, so `255 << 32` evaluated to 255, not 0;
//!   * bit<16> multiplication with a product above INT_MAX (50000 * 50000 =
//!     2.5e9) - signed overflow is UB; the fixed rendering computes in a
//!     uint64_t-wide intermediate.
//!
//! This test generates PRODUCTION code for `examples/valid/50_wide_ops.rfs`
//! in BOTH backends, wraps each in a small driver that steps the boundary
//! sequence (k = 8/31/32/64/255 + the wide multiply) and prints the per-step
//! state, compiles (rustc / gcc -Wall), runs both, and asserts:
//!   1. per-step output is BYTE-IDENTICAL across backends;
//!   2. pinned expected values independent of either backend (shifts by
//!      k >= width give w == 0; the wide products give a == 63744 then 53248);
//!   3. exact-mode MC status parity on the spec (both backends pass-complete
//!      with identical summaries).

use std::path::{Path, PathBuf};
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn tool_on_path(name: &str) -> bool {
    std::env::var_os("PATH")
        .map(|paths| std::env::split_paths(&paths).any(|dir| dir.join(name).is_file()))
        .unwrap_or(false)
}

/// The boundary sequence, as (rust constructor, c initializer) pairs. Both
/// drivers must stay in lockstep - the per-step stdout lines are compared
/// byte-for-byte.
const STEPS: &[(&str, &str)] = &[
    // Left shifts of w (starts at 1): k=8 is the width boundary (defined even
    // pre-2a); k>=32 was UB - gcc wrapped to `<<0` and kept 255.
    (
        "Input::ShiftL_L { k: 8 }",
        "(input_t){ .kind = INPUT_SHIFTL_L, .payload.input_shiftl_l.k = 8 }",
    ),
    (
        "Input::Load",
        "(input_t){ .kind = INPUT_LOAD, .payload.input_load = 0 }",
    ),
    (
        "Input::ShiftL_L { k: 31 }",
        "(input_t){ .kind = INPUT_SHIFTL_L, .payload.input_shiftl_l.k = 31 }",
    ),
    (
        "Input::Load",
        "(input_t){ .kind = INPUT_LOAD, .payload.input_load = 0 }",
    ),
    (
        "Input::ShiftL_L { k: 32 }",
        "(input_t){ .kind = INPUT_SHIFTL_L, .payload.input_shiftl_l.k = 32 }",
    ),
    (
        "Input::Load",
        "(input_t){ .kind = INPUT_LOAD, .payload.input_load = 0 }",
    ),
    (
        "Input::ShiftL_L { k: 64 }",
        "(input_t){ .kind = INPUT_SHIFTL_L, .payload.input_shiftl_l.k = 64 }",
    ),
    (
        "Input::Load",
        "(input_t){ .kind = INPUT_LOAD, .payload.input_load = 0 }",
    ),
    (
        "Input::ShiftL_L { k: 255 }",
        "(input_t){ .kind = INPUT_SHIFTL_L, .payload.input_shiftl_l.k = 255 }",
    ),
    // Right shifts of the reloaded 255 - same boundary classes.
    (
        "Input::ShiftR_R { k: 8 }",
        "(input_t){ .kind = INPUT_SHIFTR_R, .payload.input_shiftr_r.k = 8 }",
    ),
    (
        "Input::Load",
        "(input_t){ .kind = INPUT_LOAD, .payload.input_load = 0 }",
    ),
    (
        "Input::ShiftR_R { k: 31 }",
        "(input_t){ .kind = INPUT_SHIFTR_R, .payload.input_shiftr_r.k = 31 }",
    ),
    (
        "Input::Load",
        "(input_t){ .kind = INPUT_LOAD, .payload.input_load = 0 }",
    ),
    (
        "Input::ShiftR_R { k: 32 }",
        "(input_t){ .kind = INPUT_SHIFTR_R, .payload.input_shiftr_r.k = 32 }",
    ),
    (
        "Input::Load",
        "(input_t){ .kind = INPUT_LOAD, .payload.input_load = 0 }",
    ),
    (
        "Input::ShiftR_R { k: 64 }",
        "(input_t){ .kind = INPUT_SHIFTR_R, .payload.input_shiftr_r.k = 64 }",
    ),
    // The wide multiply: a = a * b with a == b == 50000 (product 2.5e9).
    (
        "Input::Mul",
        "(input_t){ .kind = INPUT_MUL, .payload.input_mul = 0 }",
    ),
    (
        "Input::Mul",
        "(input_t){ .kind = INPUT_MUL, .payload.input_mul = 0 }",
    ),
];

/// Pinned per-step state (independent of either backend): line 0 is the
/// initial configuration; each following line is post-step. Shifts by k >= 8
/// of bit<8> w give 0 (RFS semantics); the wide products are
/// (50000*50000) mod 2^16 = 63744 and (63744*50000) mod 2^16 = 53248.
const EXPECTED: &str = "\
S 1 50000 50000
S 0 50000 50000
S 255 50000 50000
S 0 50000 50000
S 255 50000 50000
S 0 50000 50000
S 255 50000 50000
S 0 50000 50000
S 255 50000 50000
S 0 50000 50000
S 0 50000 50000
S 255 50000 50000
S 0 50000 50000
S 255 50000 50000
S 0 50000 50000
S 255 50000 50000
S 0 50000 50000
S 0 63744 50000
S 0 53248 50000
";

fn rust_driver_source() -> String {
    let steps: Vec<String> = STEPS.iter().map(|(s, _)| format!("{s},")).collect();
    format!(
        "\n// task 2a differential driver (appended to the generated production code)\n\
         fn main() {{\n\
             let mut f = Fsm::new();\n\
             println!(\"{{:?}} {{}} {{}} {{}}\", f.state, f.vars.w, f.vars.a, f.vars.b);\n\
             for input in [{}] {{\n\
                 let out = f.step(input).expect(\"step must succeed\");\n\
                 assert!(out.is_empty(), \"no outputs on this spec\");\n\
                 println!(\"{{:?}} {{}} {{}} {{}}\", f.state, f.vars.w, f.vars.a, f.vars.b);\n\
             }}\n\
         }}\n",
        steps.join(" ")
    )
}

fn c_driver_source() -> String {
    let steps: Vec<String> = STEPS
        .iter()
        .map(|(_, c)| format!("do_step(&f, {c});"))
        .collect();
    format!(
        "\n// task 2a differential driver (appended to the generated production code)\n\
         static void do_step(fsm_t* f, input_t in) {{\n\
             step_result_t r = fsm_step(f, in, 0, 0);\n\
             if (r.status != RFS_OK) {{ fprintf(stderr, \"step failed status=%d\\n\", r.status); exit(1); }}\n\
             printf(\"S %u %u %u\\n\", f->vars.w, f->vars.a, f->vars.b);\n\
         }}\n\
         int main(void) {{\n\
             fsm_t f;\n\
             fsm_new(&f);\n\
             printf(\"S %u %u %u\\n\", f.vars.w, f.vars.a, f.vars.b);\n\
             {}\n\
             return 0;\n\
         }}\n",
        steps.join(" ")
    )
}

fn generate(backend: &str, command: &str) -> String {
    let spec = repo_root().join("examples/valid/50_wide_ops.rfs");
    let ir = rfs::parse_file(&spec).expect("50_wide_ops parses");
    let backend_def = rfs::codegen::get_backend(backend).expect("backend registered");
    let kind = match command {
        "production" => rfs::codegen::CodegenCommandKind::Production,
        "mc" => rfs::codegen::CodegenCommandKind::ModelChecker,
        other => panic!("unexpected command {other}"),
    };
    kind.run(backend_def, &ir, &rfs::codegen::BackendOptions::default())
        .expect("codegen succeeds")
        .code
}

#[test]
fn wide_ops_production_drivers_byte_identical_and_pinned() {
    if !tool_on_path("rustc") || !tool_on_path("gcc") {
        eprintln!("note: rustc/gcc not available - wide-ops differential skipped");
        return;
    }
    let dir = std::env::temp_dir().join(format!("rfs_wide_ops_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    // --- Rust: generated production code + appended driver, one crate root.
    let rust_src = generate("rust", "production") + &rust_driver_source();
    let rust_c = dir.join("wide_ops_rust.rs");
    std::fs::write(&rust_c, rust_src).unwrap();
    let rust_bin = dir.join("wide_ops_rust");
    let out = Command::new("rustc")
        .args(["--edition", "2021"])
        .arg(&rust_c)
        .arg("-o")
        .arg(&rust_bin)
        .output()
        .expect("spawn rustc");
    assert!(
        out.status.success(),
        "rustc: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let out = Command::new(&rust_bin).output().expect("run rust driver");
    assert!(
        out.status.success(),
        "rust driver: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let rust_stdout = String::from_utf8_lossy(&out.stdout).to_string();

    // --- C: generated production code + appended driver, concatenated.
    let c_src = generate("c", "production") + &c_driver_source();
    let c_c = dir.join("wide_ops_c.c");
    std::fs::write(&c_c, c_src).unwrap();
    let c_bin = dir.join("wide_ops_c");
    let out = Command::new("gcc")
        .arg("-Wall")
        .arg(&c_c)
        .arg("-o")
        .arg(&c_bin)
        .output()
        .expect("spawn gcc");
    assert!(
        out.status.success(),
        "gcc -Wall (zero warnings is part of the contract): {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    assert!(
        stderr.trim().is_empty(),
        "gcc -Wall must be warning-free: {stderr}"
    );
    let out = Command::new(&c_bin).output().expect("run C driver");
    assert!(
        out.status.success(),
        "C driver: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let c_stdout = String::from_utf8_lossy(&out.stdout).to_string();

    // 1. Byte-identical per-step output across backends.
    assert_eq!(
        c_stdout, rust_stdout,
        "per-step state must be byte-identical"
    );
    // 2. Pinned expected values (independent of either backend).
    assert_eq!(rust_stdout, EXPECTED, "pinned boundary values");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn wide_ops_exact_mc_status_parity() {
    if !tool_on_path("rustc") || !tool_on_path("gcc") {
        eprintln!("note: rustc/gcc not available - wide-ops MC parity skipped");
        return;
    }
    let dir = std::env::temp_dir().join(format!("rfs_wide_ops_mc_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let run = |backend: &str, bin: &Path| -> String {
        let src = generate(backend, "mc");
        let ext = if backend == "c" { "c" } else { "rs" };
        let src_path = dir.join(format!("wide_ops_mc_{backend}.{ext}"));
        std::fs::write(&src_path, src).unwrap();
        let cc = if backend == "c" { "gcc" } else { "rustc" };
        let mut cmd = Command::new(cc);
        if backend == "rust" {
            cmd.args(["--edition", "2021"]);
        } else {
            cmd.arg("-Wall");
        }
        cmd.arg(&src_path).arg("-o").arg(bin);
        let out = cmd.output().expect("spawn compiler");
        assert!(
            out.status.success(),
            "{cc} for {backend} mc: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        let out = Command::new(bin).output().expect("run mc");
        assert!(
            out.status.success(),
            "{backend} mc exit: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).to_string()
    };

    let rust_out = run("rust", &dir.join("wide_ops_mc_rust"));
    let c_out = run("c", &dir.join("wide_ops_mc_c"));

    // Both pass-complete (the safety property holds) with identical summaries.
    assert!(
        rust_out.contains("status=pass-complete"),
        "rust mc: {rust_out}"
    );
    let rust_summary = rust_out
        .lines()
        .find(|l| l.starts_with("model-check summary:"))
        .expect("summary line");
    let c_summary = c_out
        .lines()
        .find(|l| l.starts_with("model-check summary:"))
        .expect("summary line");
    assert_eq!(rust_summary, c_summary, "exact-mode summaries must match");

    let _ = std::fs::remove_dir_all(&dir);
}
