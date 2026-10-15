//! Rust backend: unit-test template generation.
//!
//! The `test` command generates the production FSM source plus a `#[cfg(test)]`
//! module with example runs of up to K steps:
//!
//! - `initial_state_is_correct`: the FSM starts in the declared initial state;
//! - `positive_sequence_runs`: drive the FSM greedily with representative
//!   inputs for up to K steps and expect progress when the module has
//!   transitions (the run ends without errors);
//! - `negative_unexpected_input_search`: look for an input that makes the FSM
//!   fail with `UnexpectedEvent`. When a statically derivable unexpected input
//!   exists (see `crate::codegen_test`), the test asserts that it is found.
//!
//! The planning logic (what to assert) is language-agnostic and lives in
//! `crate::codegen_test`; this module renders the Rust test code.

use crate::backends::rust::{representative_input_expr, rust_ident};
use crate::codegen::{BackendOptions, CodegenOutput};
use crate::codegen_fsm::FsmVariants;
use crate::codegen_test;
use crate::ir::*;
use std::fmt::Write as _;

/// Entry point for the `test` command: production FSM code + unit tests.
pub fn generate_unit_tests(
    module: &Module,
    options: &BackendOptions,
) -> Result<CodegenOutput, Vec<String>> {
    crate::backends::rust_fsm::generate_with_tests(module, options, true)
}

/// Render the `#[cfg(test)]` module appended by the `test` command.
pub(crate) fn emit_test_module(module: &Module, variants: &FsmVariants, steps: usize) -> String {
    let mut out = String::new();
    let plan = codegen_test::build_test_plan(variants, module, steps);
    let k = plan.steps;
    let candidates: Vec<String> = variants
        .inputs
        .iter()
        .map(|v| representative_input_expr(v, module))
        .collect();
    let candidate_list: String = if candidates.is_empty() {
        "Vec::new()".into()
    } else {
        format!("vec![{}]", candidates.join(", "))
    };
    let definite = plan.definite_unexpected && !plan.has_on_error;
    let on_error_assert =
        plan.has_on_error && codegen_test::has_definite_unexpected(variants, module);
    let expect_progress = plan.expect_progress;
    let _ = writeln!(
        out,
        r#"#[cfg(test)]
mod tests {{
    use super::*;
    fn candidate_inputs() -> Vec<Input> {{ {} }}
    #[test]
    fn initial_state_is_correct() {{ let fsm = Fsm::new(); assert_eq!(fsm.state, State::{}); }}
    #[test]
    fn positive_sequence_runs() {{
        let mut fsm = Fsm::new();
        let candidates = candidate_inputs();
        let mut steps = 0usize;
        for _ in 0..{} {{
            let mut progressed = false;
            for input in &candidates {{
                let mut trial = fsm.clone();
                let mut handler = StopAssumptionHandler;
                if trial.step_with_handler(*input, &mut handler).is_ok() {{ fsm = trial; steps += 1; progressed = true; break; }}
            }}
            if !progressed {{ break; }}
        }}
        const EXPECT_PROGRESS: bool = {};
        assert!(steps > 0 || !EXPECT_PROGRESS || candidates.is_empty());
    }}"#,
        candidate_list,
        rust_ident(module.initial_state.as_deref().unwrap_or("__None__")),
        k,
        expect_progress
    );
    // Negative test: without on_error it searches for UnexpectedEvent; with
    // on_error (and a statically unhandled input) it asserts the fallback
    // state is reached instead.
    let mut neg = String::new();
    let _ = writeln!(neg, "    #[test]");
    let _ = writeln!(neg, "    fn negative_unexpected_input_search() {{");
    let _ = writeln!(neg, "        let candidates = candidate_inputs();");
    let _ = writeln!(neg, "        let mut fsm = Fsm::new();");
    let _ = writeln!(neg, "        let mut found = false;");
    if plan.has_on_error {
        let _ = writeln!(neg, "        let mut on_error_found = false;");
    }
    let _ = writeln!(neg, "        for _ in 0..{} {{", k);
    let _ = writeln!(neg, "            for input in &candidates {{");
    let _ = writeln!(neg, "                let mut trial = fsm.clone();");
    let _ = writeln!(
        neg,
        "                let mut handler = ContinueAssumptionHandler;"
    );
    if plan.has_on_error {
        let oe_state = rust_ident(module.on_error.as_ref().unwrap().next_state.as_str());
        let _ = writeln!(
            neg,
            "                match trial.step_with_handler(*input, &mut handler) {{"
        );
        let _ = writeln!(
            neg,
            "                    Err(StepError::UnexpectedEvent) => {{ found = true; break; }}"
        );
        let _ = writeln!(neg, "                    Ok(_) if trial.state == State::{} => {{ on_error_found = true; break; }}", oe_state);
        let _ = writeln!(neg, "                    _ => {{}}");
        let _ = writeln!(neg, "                }}");
    } else {
        let _ = writeln!(
            neg,
            "                if matches!(trial.step_with_handler(*input, &mut handler), Err(StepError::UnexpectedEvent)) {{ found = true; break; }}"
        );
    }
    let _ = writeln!(neg, "            }}");
    if plan.has_on_error {
        let _ = writeln!(neg, "            if found || on_error_found {{ break; }}");
    } else {
        let _ = writeln!(neg, "            if found {{ break; }}");
    }
    let _ = writeln!(neg, "            let mut progressed = false;");
    let _ = writeln!(neg, "            for input in &candidates {{");
    let _ = writeln!(neg, "                let mut trial = fsm.clone();");
    let _ = writeln!(
        neg,
        "                let mut handler = ContinueAssumptionHandler;"
    );
    let _ = writeln!(
        neg,
        "                if trial.step_with_handler(*input, &mut handler).is_ok() {{ fsm = trial; progressed = true; break; }}"
    );
    let _ = writeln!(neg, "            }}");
    let _ = writeln!(neg, "            if !progressed {{ break; }}");
    let _ = writeln!(neg, "        }}");
    if on_error_assert {
        let _ = writeln!(
            neg,
            "        assert!(on_error_found, \"on_error fallback was never reached\");"
        );
    } else if definite {
        let _ = writeln!(neg, "        assert!(found);");
    } else {
        let _ = writeln!(
            neg,
            "        // No statically obvious unexpected representative input."
        );
    }
    let _ = writeln!(neg, "    }}");
    let _ = writeln!(neg, "}}");
    out.push_str(&neg);
    out
}

/// A generated aggregate unit-test crate (one module per RFS module).
pub struct GeneratedCrate {
    pub manifest: String,
    pub lib: String,
    pub warnings: Vec<String>,
    pub module_count: usize,
}

/// Generate the aggregate test crate from a list of RFS files: every module is
/// code-generated (with unit tests) into its own `pub mod`.
pub fn generate_aggregate_test_crate(
    files: &[std::path::PathBuf],
    options: &crate::codegen::BackendOptions,
) -> Result<GeneratedCrate, Vec<String>> {
    let mut lib = String::from("// Generated aggregate RFS codegen test crate.\n#![allow(dead_code, unused_variables, unused_mut, unused_parens, irrefutable_let_patterns, non_camel_case_types, non_snake_case, clippy::all)]\n");
    let mut warnings = Vec::new();
    let mut count = 0;
    for file in files {
        let ir = crate::parser::parse_file(file)
            .map_err(|e| vec![format!("{}: {}", file.display(), e.message)])?;
        let file_stem = file.file_name().and_then(|s| s.to_str()).unwrap_or("file");
        for module in &ir.modules {
            let result = crate::backends::rust_fsm::generate_with_tests(module, options, true)?;
            warnings.extend(result.warnings);
            let mod_name = format!(
                "m{:03}_{}_{}",
                count,
                crate::backends::rust::snake(file_stem),
                crate::backends::rust::snake(&module.name)
            );
            let _ = writeln!(lib, "pub mod {} {{\n{}\n}}\n", mod_name, result.code);
            count += 1;
        }
    }
    let manifest = "[package]\nname = \"rfs_codegen_tests\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[lib]\npath = \"src/lib.rs\"\n".to_string();
    Ok(GeneratedCrate {
        manifest,
        lib,
        warnings,
        module_count: count,
    })
}

/// Write the aggregate crate to `<root>/target/rfs_codegen_tests`.
pub fn write_generated_crate(
    root: &std::path::Path,
    crate_: &GeneratedCrate,
) -> std::io::Result<std::path::PathBuf> {
    let dir = root.join("target").join("rfs_codegen_tests");
    std::fs::create_dir_all(dir.join("src"))?;
    std::fs::write(dir.join("Cargo.toml"), &crate_.manifest)?;
    std::fs::write(dir.join("src/lib.rs"), &crate_.lib)?;
    Ok(dir)
}
