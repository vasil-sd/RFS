//! Rust backend: unit-test template generation.
//!
//! The `test` command generates the production FSM source plus a
//! `#[cfg(test)]` module with (all planned by the shared planner in
//! `crate::codegen_test` - one plan, rendered per backend):
//!
//! Module-level tests:
//! - `initial_state_is_correct`: the FSM starts in the declared initial state;
//! - `positive_sequence_runs`: drive the FSM greedily with representative
//!   inputs for up to K steps and expect progress when the module has
//!   transitions (the run ends without errors);
//! - `negative_unexpected_input_search`: look for an input that makes the FSM
//!   fail with `UnexpectedEvent`. When a statically derivable unexpected input
//!   exists (see `crate::codegen_test`), the test asserts that it is found.
//!
//! Extended plan classes (Phase 5 task 4/5):
//! - `t<id>_<state>_on_<event>`: per-transition unit tests - install the
//!   planned configuration, drive the concrete input, assert the outcome via
//!   the recorded trace step (robust against property-violation halts). The
//!   not-taken variant (`..._not_taken`) asserts the step either fails with
//!   `UnexpectedEvent` or lands somewhere observably different from the
//!   transition's own signature.
//! - `invariant_<label>`: re-check a safety property's per-sample atom at
//!   every sample of the positive drive trace; fail with the step index.
//! - `monitor_<label>_fires_on_violation`: drive the planned violating trace
//!   and assert the sticky `property_violation` record holds the target label.
//! - `random_trace_seed<seed>_steps<K>`: drive the precomputed LCG input
//!   sequence (same constants as the fuzz wire format) and print deterministic
//!   coverage counts.
//!
//! The command ALSO emits a separate `fuzz_target.rs` file (a
//! `libfuzzer_sys` skeleton decoding inputs per the planner's strategy): it
//! depends on an external crate, so the main test artifact must stay
//! dependency-free and the aggregate generated-code crate keeps compiling
//! without external deps. The CLI writes it next to `--out` (stdout mode is
//! refused - see `main.rs`).
//! Docs: dev:backend-rust-overview, dev:backend-rust-test, llm:backend-rust

use crate::backends::rust::{representative_input_expr, rust_ident, snake};
use crate::codegen::{BackendOptions, CodegenOutput, ExtraFile};
use crate::codegen_fsm::{self, FsmVariants, VariantInfo};
use crate::codegen_test::{
    self, build_fuzz_decode, FuzzFieldStrategy, InputCase, InvariantCase, MonitorViolationCase,
    OutputCase, RandomTraceCase, TransitionExpectation, TransitionTestCase, VarValue,
};
use crate::ir::*;
use std::fmt::Write as _;

/// Entry point for the `test` command: production FSM code + unit tests +
/// the separate libfuzzer_sys fuzz-target skeleton (extra file).
/// Property monitors are always included (D-P3-6).
pub fn generate_unit_tests(
    module: &Module,
    options: &BackendOptions,
) -> Result<CodegenOutput, Vec<String>> {
    if options.no_std {
        return Err(vec![
            crate::codegen::NO_STD_PRODUCTION_ONLY_ERROR.to_string()
        ]);
    }
    let mut result = crate::backends::rust_fsm::generate_core(
        module,
        options,
        true,
        crate::codegen::PropertyMonitorMode::Always,
    )?;
    // The fuzz skeleton is a SEPARATE file: it depends on libfuzzer_sys,
    // which the dependency-free test artifact (and the aggregate generated
    // crate) must not pull in.
    let variants = codegen_fsm::build_variants(module);
    result.extra_files.push(ExtraFile {
        name: "fuzz_target.rs".to_string(),
        content: emit_fuzz_target(module, &variants, &options.spec_hash)?,
    });
    Ok(result)
}

/// Render the `#[cfg(test)]` module appended by the `test` command.
pub(crate) fn emit_test_module(
    module: &Module,
    variants: &FsmVariants,
    steps: usize,
) -> Result<String, Vec<String>> {
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
    out.push_str(&neg);

    // Extended plan classes (Phase 5 task 4/5) - additive, after the
    // module-level tests.
    for c in &plan.transitions {
        out.push_str(&render_transition_test(module, variants, c)?);
    }
    for inv in &plan.invariants {
        out.push_str(&render_invariant_test(module, variants, inv, k)?);
    }
    for mv in &plan.monitor_violations {
        out.push_str(&render_monitor_test(module, variants, mv)?);
    }
    out.push_str(&render_random_trace_test(&plan.random_trace));
    let _ = writeln!(out, "}}");
    Ok(out)
}

// ---------------------------------------------------------------------------
// Literal rendering helpers (plan data -> Rust source)
// ---------------------------------------------------------------------------

/// Render a concrete value for a typed position (struct field / initializer).
/// Integer values are within the declared bit width by construction, so a
/// plain decimal literal is inferred to the target type.
fn var_value_literal(v: &VarValue) -> String {
    match v {
        VarValue::Bool(b) => b.to_string(),
        VarValue::Int(i) => i.to_string(),
        VarValue::Enum(path) => path
            .iter()
            .map(|s| rust_ident(s))
            .collect::<Vec<_>>()
            .join("::"),
    }
}

/// Render the complete `Vars` initializer from a case's variable
/// configuration (the planner always covers every module variable).
fn vars_initializer(module: &Module, vars: &[(String, VarValue)]) -> String {
    let fields = module
        .vars
        .iter()
        .map(|v| {
            let val = vars
                .iter()
                .find(|(n, _)| n == &v.name)
                .unwrap_or_else(|| panic!("planner must cover variable '{}'", v.name));
            format!("{}: {}", rust_ident(&v.name), var_value_literal(&val.1))
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!("Vars {{ {} }}", fields)
}

/// Render a concrete input case as a Rust expression.
fn input_case_expr(variants: &FsmVariants, case: &InputCase) -> Option<String> {
    let v = variants.input_variant(&case.group, &case.variant)?;
    if v.fields.is_empty() {
        return Some(format!("Input::{}", rust_ident(&v.unique_name)));
    }
    if v.fields.len() != case.fields.len() {
        return None;
    }
    let fields = v
        .fields
        .iter()
        .zip(case.fields.iter())
        .map(|((fname, _), (cname, val))| {
            (fname == cname).then(|| format!("{}: {}", rust_ident(fname), var_value_literal(val)))
        })
        .collect::<Option<Vec<_>>>()?;
    Some(format!(
        "Input::{} {{ {} }}",
        rust_ident(&v.unique_name),
        fields.join(", ")
    ))
}

/// Resolve an output case to its concrete variant. A bare `emit Group;`
/// resolves to the group's single variant - multi-variant groups are rejected
/// by production codegen, so this cannot fail for a generated spec.
fn resolve_output_variant<'a>(
    variants: &'a FsmVariants,
    o: &OutputCase,
) -> Option<&'a VariantInfo> {
    match &o.variant {
        Some(v) => variants.output_variant(&o.group, v),
        None => {
            let group_variants: Vec<&VariantInfo> = variants
                .outputs
                .iter()
                .filter(|v| v.group == o.group)
                .collect();
            if group_variants.len() != 1 {
                return None;
            }
            Some(group_variants[0])
        }
    }
}

/// Render one output case as a full literal. `None` when some arg could not
/// be evaluated statically (the caller falls back to a partial assertion).
fn output_case_full_literal(variants: &FsmVariants, o: &OutputCase) -> Option<String> {
    let v = resolve_output_variant(variants, o)?;
    if v.fields.is_empty() {
        return Some(format!("Output::{}", rust_ident(&v.unique_name)));
    }
    if v.fields.len() != o.args.len() || o.args.iter().any(|a| a.is_none()) {
        return None;
    }
    let fields = v
        .fields
        .iter()
        .zip(o.args.iter())
        .map(|((fname, _), val)| {
            Some(format!(
                "{}: {}",
                rust_ident(fname),
                var_value_literal(val.as_ref()?)
            ))
        })
        .collect::<Option<Vec<_>>>()?;
    Some(format!(
        "Output::{} {{ {} }}",
        rust_ident(&v.unique_name),
        fields.join(", ")
    ))
}

/// Render the outputs assertion for a taken case: full `assert_eq!` when every
/// arg is statically known, otherwise per-output partial match assertions
/// (variant + known fields only - the documented planner fallback).
fn render_outputs_assert(
    variants: &FsmVariants,
    outputs: &[OutputCase],
) -> Result<String, Vec<String>> {
    if outputs.is_empty() {
        return Ok("        assert!(last.outputs.is_empty());".to_string());
    }
    let fulls: Vec<Option<String>> = outputs
        .iter()
        .map(|o| output_case_full_literal(variants, o))
        .collect();
    if fulls.iter().all(|f| f.is_some()) {
        return Ok(format!(
            "        assert_eq!(last.outputs, vec![{}]);",
            fulls
                .into_iter()
                .map(|f| f.unwrap())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    // Partial: per-output match asserting the variant and every known field.
    let mut blocks = String::new();
    for (i, o) in outputs.iter().enumerate() {
        let v = resolve_output_variant(variants, o)
            .ok_or_else(|| vec![format!("output {}: unresolvable variant", o.group)])?;
        let name = rust_ident(&v.unique_name);
        if v.fields.is_empty() {
            let _ = writeln!(
                blocks,
                "        match &last.outputs[{i}] {{ Output::{name} => {{}}, _ => panic!(\"expected output {name}\") }}"
            );
            continue;
        }
        let binds: Vec<String> = v
            .fields
            .iter()
            .enumerate()
            .filter(|(j, _)| o.args.get(*j).and_then(|a| a.as_ref()).is_some())
            .map(|(_, (fname, _))| rust_ident(fname))
            .collect();
        let pat = if binds.is_empty() {
            format!("Output::{name} {{ .. }}")
        } else {
            format!("Output::{name} {{ {}, .. }}", binds.join(", "))
        };
        let asserts: Vec<String> = v
            .fields
            .iter()
            .enumerate()
            .filter_map(|(j, (fname, _))| {
                o.args.get(j).and_then(|a| a.as_ref()).map(|val| {
                    format!(
                        "assert_eq!(*{}, {});",
                        rust_ident(fname),
                        var_value_literal(val)
                    )
                })
            })
            .collect();
        let body = asserts.join(" ");
        let _ = writeln!(
            blocks,
            "        match &last.outputs[{i}] {{ {pat} => {{ {body} }}, _ => panic!(\"expected output {name}\") }}"
        );
    }
    Ok(blocks)
}

// ---------------------------------------------------------------------------
// Per-transition tests
// ---------------------------------------------------------------------------

/// Render one per-transition unit test (taken or not-taken case).
fn render_transition_test(
    module: &Module,
    variants: &FsmVariants,
    c: &TransitionTestCase,
) -> Result<String, Vec<String>> {
    let input = input_case_expr(variants, &c.input).ok_or_else(|| {
        vec![format!(
            "transition #{}: unresolvable input case",
            c.transition_id
        )]
    })?;
    let vinfo = variants
        .input_variant(&c.input.group, &c.input.variant)
        .ok_or_else(|| {
            vec![format!(
                "transition #{}: unknown input variant",
                c.transition_id
            )]
        })?;
    let base = format!(
        "t{}_{}_on_{}",
        c.transition_id,
        snake(&c.from_state),
        snake(&vinfo.unique_name)
    );
    let vars = vars_initializer(module, &c.vars);
    let from = rust_ident(&c.from_state);
    match &c.expectation {
        TransitionExpectation::Taken {
            next_state,
            outputs,
        } => {
            let outs = render_outputs_assert(variants, outputs)?;
            Ok(format!(
                "    #[test]
    fn {base}() {{
        let mut fsm = Fsm::new();
        fsm.state = State::{from};
        fsm.vars = {vars};
        let mut handler = ContinueAssumptionHandler;
        let _ = fsm.step_with_handler({input}, &mut handler);
        let last = fsm.trace.steps.last().expect(\"transition #{id} should fire\");
        assert_eq!(last.to_state, State::{next});
{outs}
    }}
",
                id = c.transition_id,
                next = rust_ident(next_state),
            ))
        }
        TransitionExpectation::NotTaken { signature, .. } => {
            let (sig_state, sig_outputs) = signature;
            // The "differs from the transition's own signature" assertion
            // needs fully-evaluated expected outputs. The planner guarantees
            // observability at the expression level, so when values are not
            // statically known we refuse to emit a toothless test (strict).
            let sig_lits: Vec<Option<String>> = sig_outputs
                .iter()
                .map(|o| output_case_full_literal(variants, o))
                .collect();
            if !sig_lits.iter().all(|f| f.is_some()) {
                return Err(vec![format!(
                    "transition #{}: not-taken signature has un-evaluable output args; refusing to emit a toothless test",
                    c.transition_id
                )]);
            }
            let sig_lit = if sig_lits.is_empty() {
                "Vec::new()".to_string()
            } else {
                format!(
                    "vec![{}]",
                    sig_lits
                        .into_iter()
                        .map(|f| f.unwrap())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            };
            Ok(format!(
                "    #[test]
    fn {base}_not_taken() {{
        let mut fsm = Fsm::new();
        fsm.state = State::{from};
        fsm.vars = {vars};
        let mut handler = ContinueAssumptionHandler;
        match fsm.step_with_handler({input}, &mut handler) {{
            Err(StepError::UnexpectedEvent) => {{
                assert_eq!(fsm.state, State::{from}, \"no progress expected\");
                assert_eq!(fsm.trace.steps.len(), 0, \"no step may be recorded\");
            }}
            _ => {{
                let last = fsm.trace.steps.last().expect(\"a step was recorded\");
                assert!(last.to_state != State::{sig} || last.outputs != {sig_lit}, \"transition #{id} fired although its guard is false under this configuration\");
            }}
        }}
    }}
",
                sig = rust_ident(sig_state),
                id = c.transition_id,
            ))
        }
    }
}

// ---------------------------------------------------------------------------
// Invariant tests
// ---------------------------------------------------------------------------

/// Render the invariant re-check test: drive the positive trace (same greedy
/// policy as `positive_sequence_runs`) and re-evaluate the property's
/// per-sample atom at every recorded sample, failing with the step index.
fn render_invariant_test(
    module: &Module,
    variants: &FsmVariants,
    inv: &InvariantCase,
    k: usize,
) -> Result<String, Vec<String>> {
    let sig = crate::backends::rust_fsm::render_signal_expr(module, variants, &inv.signal)
        .map_err(|e| vec![format!("invariant '{}': {}", inv.label, e)])?;
    let bound = match inv.bound {
        None => "None".to_string(),
        Some(n) => format!("Some({n})"),
    };
    Ok(format!(
        "    #[test]
    fn invariant_{label}() {{
        let mut fsm = Fsm::new();
        const BOUND: Option<u64> = {bound};
        let check = |s: &Sample| -> bool {{ {sig} }};
        let s0 = Sample {{ input: None, outputs: Vec::new(), from_state: fsm.state, to_state: fsm.state, vars: fsm.vars.clone() }};
        if !check(&s0) {{ panic!(\"invariant '{label}' violated at sample 0\"); }}
        let candidates = candidate_inputs();
        for _ in 0..{k} {{
            let mut progressed = false;
            for input in &candidates {{
                let mut trial = fsm.clone();
                let mut handler = StopAssumptionHandler;
                if trial.step_with_handler(*input, &mut handler).is_ok() {{ fsm = trial; progressed = true; break; }}
            }}
            if !progressed {{ break; }}
        }}
        for (i, st) in fsm.trace.steps.iter().enumerate() {{
            let idx = i + 1;
            if BOUND.map_or(true, |b| (idx as u64) < b) {{
                let s = Sample {{ input: Some(st.input), outputs: st.outputs.clone(), from_state: st.from_state, to_state: st.to_state, vars: st.vars }};
                if !check(&s) {{ panic!(\"invariant '{label}' violated at sample {{idx}}\"); }}
            }}
        }}
    }}
",
        label = snake(&inv.label),
    ))
}

// ---------------------------------------------------------------------------
// Property-monitor debug tests
// ---------------------------------------------------------------------------

/// Render the monitor-violating-trace test: drive the planned trace and assert
/// the sticky `property_violation` record holds the target label (the planner
/// guarantees no other monitored property records first - see
/// `planned_monitor_violations_record_target_label_first`).
fn render_monitor_test(
    module: &Module,
    variants: &FsmVariants,
    mv: &MonitorViolationCase,
) -> Result<String, Vec<String>> {
    let name = format!("monitor_{}_fires_on_violation", snake(&mv.label));
    if mv.step_input.is_none() {
        // The initial configuration already violates: Fsm::new() records it
        // at sample 0.
        return Ok(format!(
            "    #[test]
    fn {name}() {{
        let fsm = Fsm::new();
        assert_eq!(fsm.property_violation.as_ref().map(|pv| pv.label.clone()), Some(\"{label}\".to_string()), \"the initial configuration must record property '{label}' first\");
    }}
",
            label = mv.label,
        ));
    }
    let input = input_case_expr(variants, mv.step_input.as_ref().unwrap())
        .ok_or_else(|| vec![format!("property {}: unresolvable step input", mv.label)])?;
    let vars = vars_initializer(module, &mv.vars);
    Ok(format!(
        "    #[test]
    fn {name}() {{
        let mut fsm = Fsm::new();
        fsm.state = State::{state};
        fsm.vars = {vars};
        let mut handler = ContinueAssumptionHandler;
        let _ = fsm.step_with_handler({input}, &mut handler);
        assert_eq!(fsm.property_violation.as_ref().map(|pv| pv.label.clone()), Some(\"{label}\".to_string()), \"the violating drive must record property '{label}' first\");
    }}
",
        state = rust_ident(&mv.state),
        label = mv.label,
    ))
}

// ---------------------------------------------------------------------------
// Random-trace test
// ---------------------------------------------------------------------------

/// Render the deterministic random-trace test: drive the precomputed LCG input
/// sequence (seed + indices fixed at plan time - same constants as the fuzz
/// wire format) and print deterministic coverage counts.
fn render_random_trace_test(rt: &RandomTraceCase) -> String {
    let table = if rt.input_indices.is_empty() {
        "[] as [usize; 0]".to_string()
    } else {
        format!(
            "[{}]",
            rt.input_indices
                .iter()
                .map(|i| format!("{i}usize"))
                .collect::<Vec<_>>()
                .join(", ")
        )
    };
    format!(
        "    #[test]
    fn random_trace_seed{seed}_steps{k}() {{
        let mut fsm = Fsm::new();
        let initial = fsm.state;
        let mut handler = StopAssumptionHandler;
        let mut ok = 0usize;
        let mut unexpected = 0usize;
        let mut assumption = 0usize;
        let mut property = 0usize;
        for i in {table} {{
            let input = ALL_INPUTS[i];
            match fsm.step_with_handler(input, &mut handler) {{
                Ok(_) => ok += 1,
                Err(StepError::UnexpectedEvent) => unexpected += 1,
                Err(StepError::AssumptionViolation {{ .. }}) => assumption += 1,
                Err(StepError::PropertyViolation {{ .. }}) => property += 1,
            }}
        }}
        let mut seen = vec![initial];
        for st in &fsm.trace.steps {{ if !seen.contains(&st.to_state) {{ seen.push(st.to_state); }} }}
        let states_visited = seen.len();
        println!(\"random_trace seed={seed} steps={k} ok={{ok}} unexpected={{unexpected}} assumption={{assumption}} property={{property}} states_visited={{states_visited}}\");
    }}
",
        seed = rt.seed,
        k = rt.steps,
    )
}

// ---------------------------------------------------------------------------
// libfuzzer_sys fuzz-target skeleton (separate file)
// ---------------------------------------------------------------------------

/// Number of little-endian bytes a `bit<W>` payload field decodes from.
fn payload_byte_count(width: u32) -> usize {
    (width as usize).div_ceil(8)
}

/// Emit the `fuzz_target.rs` skeleton: `#![no_main]` + a `fuzz_target!` that
/// decodes inputs per the planner's strategy and loops them over
/// `Fsm::step`. Depends on `libfuzzer_sys` - hence a separate file (the main
/// test artifact stays dependency-free). Building it is task 12's job.
pub fn emit_fuzz_target(
    module: &Module,
    variants: &FsmVariants,
    spec_hash: &str,
) -> Result<String, Vec<String>> {
    let decode = build_fuzz_decode(variants);
    let n_variants = decode.variants.len();
    let mut out = String::new();
    // Line 1 is the v1.0 unified artifact header (Phase 6, D-P6-3) - same
    // text as the main test artifact's line 1.
    let _ = writeln!(
        out,
        "// Generated by RFS v{} | spec sha256:{} | module {}",
        env!("CARGO_PKG_VERSION"),
        spec_hash,
        module.name
    );
    let _ = writeln!(
        out,
        "// libFuzzer target skeleton for module `{}`.",
        module.name
    );
    let _ = writeln!(out, "//");
    let _ = writeln!(
        out,
        "// This file is intentionally SEPARATE from the main test artifact: it"
    );
    let _ = writeln!(
        out,
        "// depends on `libfuzzer_sys`, which the dependency-free test artifact"
    );
    let _ = writeln!(out, "// must not pull in.");
    let _ = writeln!(out, "//");
    let _ = writeln!(
        out,
        "// Wiring (see the Phase 5 task 12 example): put the RFS-generated"
    );
    let _ = writeln!(
        out,
        "// source at your lib crate root (or re-export the items imported"
    );
    let _ = writeln!(out, "// below from it), `cargo fuzz init`, replace");
    let _ = writeln!(
        out,
        "// fuzz/fuzz_targets/fuzz_target.rs with this file and adjust the"
    );
    let _ = writeln!(
        out,
        "// import to your crate name, then `cargo fuzz run fuzz_target`."
    );
    let _ = writeln!(out, "//");
    let _ = writeln!(
        out,
        "// Decoding: the first byte selects the input variant (byte % {n_variants});"
    );
    let _ = writeln!(
        out,
        "// each field follows - bool = one non-zero byte, bit<W> ="
    );
    let _ = writeln!(
        out,
        "// ceil(W/8) little-endian bytes masked to W bits, enum = constant"
    );
    let _ = writeln!(
        out,
        "// representative. The loop steps the FSM until the input is exhausted."
    );
    let _ = writeln!(out);
    let _ = writeln!(out, "#![no_main]");
    let _ = writeln!(out);
    let _ = writeln!(out, "use libfuzzer_sys::fuzz_target;");

    if n_variants == 0 {
        let _ = writeln!(out);
        let _ = writeln!(out, "fuzz_target!(|_data: &[u8]| {{");
        let _ = writeln!(
            out,
            "    // The module declares no inputs; there is nothing to fuzz."
        );
        let _ = writeln!(out, "}});");
        return Ok(out);
    }

    // Import list: Fsm + Input + handler + every enum type a Representative
    // field decodes to (field types come from the variant table).
    let mut imports: Vec<String> = vec![
        "Fsm".to_string(),
        "Input".to_string(),
        "StopAssumptionHandler".to_string(),
    ];
    for d in &decode.variants {
        let Some(info) = variants
            .inputs
            .iter()
            .find(|v| v.unique_name == d.unique_name)
        else {
            continue;
        };
        for ((_, strat), (_, fty)) in d.fields.iter().zip(info.fields.iter()) {
            if matches!(strat, FuzzFieldStrategy::Representative) {
                if let Type::Enum { name } = fty {
                    let ident = rust_ident(name);
                    if !imports.contains(&ident) {
                        imports.push(ident);
                    }
                }
            }
        }
    }
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "// ADJUST to your wiring: the generated FSM items must be in scope"
    );
    let _ = writeln!(
        out,
        "// (with cargo-fuzz this is `use <your_crate>::{{ ... }};`)."
    );
    let _ = writeln!(out, "use crate::{{{}}};", imports.join(", "));
    let _ = writeln!(out);

    // The step loop.
    let _ = writeln!(out, "fuzz_target!(|data: &[u8]| {{");
    let _ = writeln!(out, "    let mut fsm = Fsm::new();");
    let _ = writeln!(out, "    let mut handler = StopAssumptionHandler;");
    let _ = writeln!(out, "    let mut pos = 0usize;");
    let _ = writeln!(out, "    loop {{");
    let _ = writeln!(out, "        if pos >= data.len() {{ break; }}");
    let _ = writeln!(
        out,
        "        let vi = (data[pos] as usize) % {n};",
        n = n_variants
    );
    let _ = writeln!(out, "        pos += 1;");
    let _ = writeln!(out, "        let input = match vi {{");
    for (vi, d) in decode.variants.iter().enumerate() {
        let info = variants
            .inputs
            .iter()
            .find(|v| v.unique_name == d.unique_name)
            .ok_or_else(|| vec![format!("fuzz decode: unknown variant {}", d.unique_name)])?;
        // Total bytes the field decoders consume after the selector.
        let budget: usize = d
            .fields
            .iter()
            .map(|(_, s)| match s {
                FuzzFieldStrategy::ByteNonZero => 1,
                FuzzFieldStrategy::PayloadBytes { width } => payload_byte_count(*width),
                FuzzFieldStrategy::Representative => 0,
            })
            .sum();
        let name = rust_ident(&info.unique_name);
        if info.fields.is_empty() {
            let _ = writeln!(out, "            {vi} => Input::{name},");
            continue;
        }
        let _ = writeln!(out, "            {vi} => {{");
        if budget > 0 {
            let _ = writeln!(
                out,
                "                if pos + {budget} > data.len() {{ break; }}"
            );
        }
        for ((fname, strat), (_, fty)) in d.fields.iter().zip(info.fields.iter()) {
            let bind = rust_ident(fname);
            match strat {
                FuzzFieldStrategy::ByteNonZero => {
                    let _ = writeln!(
                        out,
                        "                let {bind} = data[pos] != 0; pos += 1;"
                    );
                }
                FuzzFieldStrategy::PayloadBytes { width } => {
                    let ty = crate::backends::rust::bit_rust_type(*width);
                    let nbytes = payload_byte_count(*width);
                    let mask = if *width >= 64 {
                        format!("{ty}::MAX")
                    } else {
                        format!("0x{:X}{ty}", (1u64 << width) - 1)
                    };
                    // Accumulate little-endian bytes into the target type.
                    let mut acc = format!("(data[pos] as {ty})");
                    for i in 1..nbytes {
                        acc = format!("({acc} | ((data[pos + {i}] as {ty}) << {}))", 8 * i);
                    }
                    let _ = writeln!(
                        out,
                        "                let {bind} = ({acc}) & {mask}; pos += {nbytes};"
                    );
                }
                FuzzFieldStrategy::Representative => match fty {
                    Type::Enum { name: enum_name } => {
                        let eident = rust_ident(enum_name);
                        let rep = module
                            .enums
                            .iter()
                            .find(|e| e.name.as_str() == enum_name)
                            .and_then(|e| e.variants.first())
                            .map(|v| format!("{eident}::{}", rust_ident(v)))
                            .unwrap_or_else(|| format!("{eident}::__Unknown__"));
                        let _ = writeln!(
                            out,
                            "                // {fname}: constant representative (enum)."
                        );
                        let _ = writeln!(out, "                let {bind} = {rep};");
                    }
                    _ => unreachable!("representative strategy only for enum fields"),
                },
            }
        }
        let field_names: Vec<String> = info.fields.iter().map(|(f, _)| rust_ident(f)).collect();
        let _ = writeln!(
            out,
            "                Input::{name} {{ {} }}",
            field_names.join(", ")
        );
        let _ = writeln!(out, "            }}");
    }
    let _ = writeln!(out, "            _ => break,");
    let _ = writeln!(out, "        }};");
    let _ = writeln!(
        out,
        "        let _ = fsm.step_with_handler(input, &mut handler);"
    );
    let _ = writeln!(out, "    }}");
    let _ = writeln!(out, "}});");
    Ok(out)
}

/// A generated aggregate unit-test crate (one module per RFS module).
pub struct GeneratedCrate {
    pub manifest: String,
    pub lib: String,
    pub warnings: Vec<String>,
    pub module_count: usize,
}

/// Generate the aggregate test crate from a list of RFS files: every module is
/// code-generated (with unit tests) into its own `pub mod`. The fuzz skeleton
/// is NOT part of the aggregate (it depends on libfuzzer_sys; the aggregate
/// must stay dependency-free).
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
        // v1.0 spec hash (D-P6-3): the WHOLE file's canonical IR JSON text
        // (deterministic per D-P5-2), one hash per artifact - stamped onto a
        // per-file options clone so every module of the file carries it in its
        // line-1 header.
        let json_text = ir
            .to_json()
            .map_err(|e| vec![format!("{}: {}", file.display(), e)])?
            .to_string();
        let mut file_options = options.clone();
        file_options.spec_hash = crate::spec_hash::spec_hash16(&json_text);
        let file_stem = file.file_name().and_then(|s| s.to_str()).unwrap_or("file");
        for module in &ir.modules {
            let result = crate::backends::rust_fsm::generate_core(
                module,
                &file_options,
                true,
                crate::codegen::PropertyMonitorMode::Always,
            )?;
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
    // The empty [workspace] table makes the generated crate its OWN workspace
    // root (Phase 5 task 1): without it, cargo walking up from target/
    // discovers the repository's workspace and refuses to build a manifest
    // that is not one of its members.
    let manifest = "[package]\nname = \"rfs_codegen_tests\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[workspace]\n\n[lib]\npath = \"src/lib.rs\"\n".to_string();
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
