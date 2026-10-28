//! C backend: unit-test file generation (plain-assert harness, no framework).
//!
//! The `test` command emits the production FSM source plus a `main()` that
//! runs the same planned checks as the Rust backend's generated tests
//! (`codegen_test::TestPlan`) - one plan, rendered per backend (parity
//! principle): module-level tests (initial state, positive bounded sequence,
//! negative unexpected-input search) plus the extended plan classes:
//!
//! - `t<id>_<state>_on_<event>` / `..._not_taken`: per-transition unit tests;
//! - `invariant_<label>`: safety-atom re-check at every driven sample;
//! - `monitor_<label>_fires_on_violation`: injected violating trace must set
//!   the sticky property-violation record to the target label;
//! - `random_trace_seed<seed>_steps<K>`: precomputed LCG input sequence with
//!   deterministic coverage counts.
//!
//! Each test is a function run from a table in `main()`; one line per test
//! (`PASS <name>` / `FAIL <name>`) keeps the wire format comparable to the
//! Rust harness (the parity suite normalizes both). Failures are counted
//! (not aborting) so one run reports every broken check.
//!
//! The command ALSO emits a separate `fuzz_target.c` file (a libFuzzer
//! skeleton decoding inputs per the planner's strategy): building it needs a
//! libFuzzer-enabled clang, which the plain `gcc -Wall` test artifact must not
//! require. The CLI writes it next to `--out` (stdout mode is refused).
//! Docs: dev:backend-c-overview, llm:backend-c

use crate::backends::c::{bit_c_type, c_ident};
use crate::backends::rust::snake;
use crate::codegen::{BackendOptions, CodegenOutput, ExtraFile};
use crate::codegen_fsm::{self, FsmVariants, VariantInfo};
use crate::codegen_test::{
    self, build_fuzz_decode, FuzzFieldStrategy, InputCase, InvariantCase, MonitorViolationCase,
    OutputCase, RandomTraceCase, TransitionExpectation, TransitionTestCase, VarValue,
};
use crate::ir::*;
use std::fmt::Write as _;

/// A representative `input_t` initializer for a variant (default field values:
/// bool false, zero integers, first declared enum variant).
fn rep_input_c(v: &VariantInfo, module: &Module) -> String {
    if v.fields.is_empty() {
        return format!("{{ .kind = INPUT_{} }}", v.unique_name.to_uppercase());
    }
    let pl = format!("payload.input_{}", v.unique_name.to_lowercase());
    let mut fields = vec![format!(".kind = INPUT_{}", v.unique_name.to_uppercase())];
    for (fname, fty) in &v.fields {
        let val = match fty {
            Type::Bool => "false".to_string(),
            Type::Bit { width } if *width >= 64 => "0ULL".to_string(),
            Type::Bit { .. } => "0u".to_string(),
            Type::Enum { name } => {
                let first = module
                    .enums
                    .iter()
                    .find(|e| e.name == *name)
                    .and_then(|e| e.variants.first())
                    .cloned()
                    .unwrap_or_default();
                format!(
                    "{}_{}",
                    c_ident(name).to_uppercase(),
                    c_ident(&first).to_uppercase()
                )
            }
        };
        fields.push(format!(".{}.{} = {}", pl, c_ident(fname), val));
    }
    format!("{{ {} }}", fields.join(", "))
}

/// Render a concrete value for a typed position (struct field / initializer).
/// Integer values are within the declared bit width by construction.
fn var_value_literal_c(v: &VarValue) -> String {
    match v {
        VarValue::Bool(b) => {
            if *b {
                "true".into()
            } else {
                "false".into()
            }
        }
        VarValue::Int(i) => i.to_string(),
        VarValue::Enum(path) => {
            let base = c_ident(&path[0]).to_uppercase();
            let rest = path[1..]
                .iter()
                .map(|s| c_ident(s).to_uppercase())
                .collect::<Vec<_>>()
                .join("_");
            format!("{base}_{rest}")
        }
    }
}

/// Render the complete `vars_t` initializer from a case's variable
/// configuration (the planner always covers every module variable).
fn vars_initializer_c(module: &Module, vars: &[(String, VarValue)]) -> String {
    if module.vars.is_empty() {
        return "(vars_t){ 0 }".to_string();
    }
    let fields = module
        .vars
        .iter()
        .map(|v| {
            let val = vars
                .iter()
                .find(|(n, _)| n == &v.name)
                .unwrap_or_else(|| panic!("planner must cover variable '{}'", v.name));
            format!(".{} = {}", c_ident(&v.name), var_value_literal_c(&val.1))
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!("(vars_t){{ {fields} }}")
}

/// Render a concrete input case as a C initializer expression.
fn input_case_expr_c(variants: &FsmVariants, case: &InputCase) -> Option<String> {
    let v = variants.input_variant(&case.group, &case.variant)?;
    if v.fields.is_empty() {
        return Some(format!(
            "{{ .kind = INPUT_{} }}",
            v.unique_name.to_uppercase()
        ));
    }
    if v.fields.len() != case.fields.len() {
        return None;
    }
    let pl = format!("payload.input_{}", v.unique_name.to_lowercase());
    let mut fields = vec![format!(".kind = INPUT_{}", v.unique_name.to_uppercase())];
    for ((fname, _), (cname, val)) in v.fields.iter().zip(case.fields.iter()) {
        if fname != cname {
            return None;
        }
        fields.push(format!(
            ".{}.{} = {}",
            pl,
            c_ident(fname),
            var_value_literal_c(val)
        ));
    }
    Some(format!("{{ {} }}", fields.join(", ")))
}

/// Resolve an output case to its concrete variant. A bare `emit Group;`
/// resolves to the group's single variant - multi-variant groups are rejected
/// by production codegen, so this cannot fail for a generated spec.
fn resolve_output_variant_c<'a>(
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

/// Render the outputs assertion for a taken case: variant + every statically
/// known field (the documented planner fallback when an arg is not evaluable).
fn render_outputs_assert_c(
    variants: &FsmVariants,
    outputs: &[OutputCase],
) -> Result<String, Vec<String>> {
    if outputs.is_empty() {
        return Ok("CHECK(last->n_outputs == 0, \"output count\");".to_string());
    }
    let mut out = String::new();
    let _ = writeln!(
        out,
        "CHECK(last->n_outputs == {}, \"output count\");",
        outputs.len()
    );
    for (i, o) in outputs.iter().enumerate() {
        let v = resolve_output_variant_c(variants, o)
            .ok_or_else(|| vec![format!("output {}: unresolvable variant", o.group)])?;
        let name = v.unique_name.to_lowercase();
        let _ = writeln!(out, "if (last->n_outputs > {i}) {{");
        let _ = writeln!(
            out,
            "CHECK(last->outputs[{i}].kind == OUTPUT_{}, \"output {i} variant\");",
            v.unique_name.to_uppercase()
        );
        for (j, (fname, _)) in v.fields.iter().enumerate() {
            if let Some(val) = o.args.get(j).and_then(|a| a.as_ref()) {
                let _ = writeln!(
                    out,
                    "CHECK(last->outputs[{i}].payload.output_{name}.{f} == {lit}, \"output {i} field {f}\");",
                    f = c_ident(fname),
                    lit = var_value_literal_c(val)
                );
            }
        }
        let _ = writeln!(out, "}}");
    }
    Ok(out)
}

/// Render the `same_sig` computation for a not-taken case: 1 iff the recorded
/// landing equals the transition's own signature (state + full outputs). The
/// planner guarantees observability at the expression level, so when values
/// are not statically known we refuse to emit a toothless test (strict).
fn render_signature_compare_c(
    variants: &FsmVariants,
    sig_state: &str,
    sig_outputs: &[OutputCase],
) -> Result<String, Vec<String>> {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "int same_sig = (last->to == STATE_{});",
        c_ident(sig_state).to_uppercase()
    );
    let _ = writeln!(
        out,
        "if (same_sig && last->n_outputs != {}) {{ same_sig = 0; }}",
        sig_outputs.len()
    );
    for (i, o) in sig_outputs.iter().enumerate() {
        let v = resolve_output_variant_c(variants, o)
            .ok_or_else(|| vec![format!("output {}: unresolvable variant", o.group)])?;
        if v.fields.len() != o.args.len() || o.args.iter().any(|a| a.is_none()) {
            return Err(vec![
                "not-taken signature has un-evaluable output args; refusing to emit a toothless test"
                    .to_string(),
            ]);
        }
        let name = v.unique_name.to_lowercase();
        let mut conds = vec![format!(
            "last->outputs[{i}].kind != OUTPUT_{}",
            v.unique_name.to_uppercase()
        )];
        for ((fname, _), val) in v.fields.iter().zip(o.args.iter()) {
            conds.push(format!(
                "last->outputs[{i}].payload.output_{name}.{f} != {lit}",
                f = c_ident(fname),
                lit = var_value_literal_c(val.as_ref().unwrap())
            ));
        }
        let guard = if i == 0 { "" } else { "same_sig && " };
        let _ = writeln!(
            out,
            "if ({guard}({conds})) {{ same_sig = 0; }}",
            conds = conds.join(" || ")
        );
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// Per-transition tests
// ---------------------------------------------------------------------------

/// Render one per-transition unit test (taken or not-taken case).
fn render_transition_test_c(
    module: &Module,
    variants: &FsmVariants,
    c: &TransitionTestCase,
) -> Result<String, Vec<String>> {
    let input = input_case_expr_c(variants, &c.input).ok_or_else(|| {
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
    let vars = vars_initializer_c(module, &c.vars);
    let from = c_ident(&c.from_state).to_uppercase();
    match &c.expectation {
        TransitionExpectation::Taken {
            next_state,
            outputs,
        } => {
            let outs = render_outputs_assert_c(variants, outputs)?;
            Ok(format!(
                "static void {base}(void) {{\n\
                 \x20   fsm_t f; fsm_new(&f);\n\
                 \x20   f.state = STATE_{from};\n\
                 \x20   f.vars = {vars};\n\
                 \x20   input_t in = {input};\n\
                 \x20   fsm_step(&f, in, cont_handler, 0);\n\
                 \x20   CHECK(f.trace.len >= 1, \"transition #{id} should fire\");\n\
                 \x20   if (f.trace.len >= 1) {{\n\
                 \x20       const trace_step_t* last = &f.trace.steps[f.trace.len - 1];\n\
                 \x20       CHECK(last->to == STATE_{next}, \"next state\");\n\
                 {outs}\n\
                 \x20   }}\n\
                 }}\n",
                id = c.transition_id,
                next = c_ident(next_state).to_uppercase(),
            ))
        }
        TransitionExpectation::NotTaken { signature, .. } => {
            let (sig_state, sig_outputs) = signature;
            let sig_cmp = render_signature_compare_c(variants, sig_state, sig_outputs)?;
            Ok(format!(
                "static void {base}_not_taken(void) {{\n\
                 \x20   fsm_t f; fsm_new(&f);\n\
                 \x20   f.state = STATE_{from};\n\
                 \x20   f.vars = {vars};\n\
                 \x20   input_t in = {input};\n\
                 \x20   step_result_t r = fsm_step(&f, in, cont_handler, 0);\n\
                 \x20   if (r.status == RFS_UNEXPECTED_EVENT) {{\n\
                 \x20       CHECK(f.state == STATE_{from}, \"no progress expected\");\n\
                 \x20       CHECK(f.trace.len == 0, \"no step may be recorded\");\n\
                 \x20   }} else {{\n\
                 \x20       CHECK(f.trace.len >= 1, \"a step was recorded\");\n\
                 \x20       if (f.trace.len >= 1) {{\n\
                 \x20           const trace_step_t* last = &f.trace.steps[f.trace.len - 1];\n\
                 {sig_cmp}\n\
                 \x20           CHECK(!same_sig, \"transition #{id} fired although its guard is false under this configuration\");\n\
                 \x20       }}\n\
                 \x20   }}\n\
                 }}\n",
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
fn render_invariant_test_c(
    module: &Module,
    variants: &FsmVariants,
    inv: &InvariantCase,
    k: usize,
) -> Result<String, Vec<String>> {
    let label = snake(&inv.label);
    let prefix = format!("rfs_inv_{label}");
    let (input_pats, output_pats) =
        crate::backends::c_fsm::collect_liv_signal_patterns(&inv.signal);
    let mut out = String::new();
    for (i, p) in input_pats.iter().enumerate() {
        out.push_str(
            &crate::backends::c_fsm::input_is_sample_helper_c(&prefix, i, p, variants).ok_or_else(
                || {
                    vec![format!(
                        "invariant '{}': unresolvable input pattern",
                        inv.label
                    )]
                },
            )?,
        );
    }
    for (i, p) in output_pats.iter().enumerate() {
        out.push_str(
            &crate::backends::c_fsm::output_has_sample_helper_c(&prefix, i, p, variants)
                .ok_or_else(|| {
                    vec![format!(
                        "invariant '{}': unresolvable output pattern",
                        inv.label
                    )]
                })?,
        );
    }
    let sig = crate::backends::c_fsm::render_signal_expr_c(
        module,
        &inv.signal,
        &prefix,
        &input_pats,
        &output_pats,
    )
    .map_err(|e| vec![format!("invariant '{}': {e}", inv.label)])?;
    let _ = writeln!(
        out,
        "static bool rfs_inv_check_{label}(const rfs_sample_t* s) {{ return {sig}; }}"
    );
    let bound = match inv.bound {
        None => "-1",
        Some(n) => &n.to_string(),
    };
    Ok(format!(
        "{out}\n\
         static void invariant_{label}(void) {{\n\
         \x20   fsm_t f; fsm_new(&f);\n\
         \x20   const long long BOUND = {bound}; /* -1 = unbounded */\n\
         \x20   rfs_sample_t s0 = {{ 0, 0, 0, f.state, f.state, f.vars }};\n\
         \x20   if (!rfs_inv_check_{label}(&s0)) {{ printf(\"invariant '{label}' violated at sample 0\\n\"); CHECK(0, \"invariant {label} violated at sample 0\"); }}\n\
         \x20   for (int i = 0; i < {k}; i++) {{\n\
         \x20       int progressed = 0;\n\
         \x20       for (int c = 0; c < N_ALL_INPUTS; c++) {{\n\
         \x20           fsm_t t = clone_fsm(&f);\n\
         \x20           step_result_t r = fsm_step(&t, ALL_INPUTS[c], stop_handler, 0);\n\
         \x20           if (r.status == RFS_OK) {{ f = t; progressed = 1; break; }}\n\
         \x20       }}\n\
         \x20       if (!progressed) break;\n\
         \x20   }}\n\
         \x20   for (int i = 0; i < f.trace.len; i++) {{\n\
         \x20       long long idx = (long long)i + 1;\n\
         \x20       if (BOUND < 0 || idx < BOUND) {{\n\
         \x20           const trace_step_t* st = &f.trace.steps[i];\n\
         \x20           rfs_sample_t s = {{ &st->input, st->outputs, st->n_outputs, st->from, st->to, st->vars }};\n\
         \x20           if (!rfs_inv_check_{label}(&s)) {{ printf(\"invariant '{label}' violated at sample %lld\\n\", idx); CHECK(0, \"invariant {label} violated\"); }}\n\
         \x20       }}\n\
         \x20   }}\n\
         }}\n")
    )
}

// ---------------------------------------------------------------------------
// Property-monitor debug tests
// ---------------------------------------------------------------------------

/// Render the monitor-violating-trace test: drive the planned trace and assert
/// the sticky property-violation record holds the target label (the planner
/// guarantees no other monitored property records first).
fn render_monitor_test_c(
    module: &Module,
    variants: &FsmVariants,
    mv: &MonitorViolationCase,
) -> Result<String, Vec<String>> {
    let name = format!("monitor_{}_fires_on_violation", snake(&mv.label));
    if mv.step_input.is_none() {
        // The initial configuration already violates: fsm_new records it at
        // sample 0.
        return Ok(format!(
            "static void {name}(void) {{\n\
             \x20   fsm_t f; fsm_new(&f);\n\
             \x20   CHECK(f.prop_label[0] != 0 && strcmp(f.prop_label, \"{label}\") == 0, \"the initial configuration must record property '{label}' first\");\n\
             }}\n",
            label = mv.label,
        ));
    }
    let input = input_case_expr_c(variants, mv.step_input.as_ref().unwrap())
        .ok_or_else(|| vec![format!("property {}: unresolvable step input", mv.label)])?;
    let vars = vars_initializer_c(module, &mv.vars);
    Ok(format!(
        "static void {name}(void) {{\n\
         \x20   fsm_t f; fsm_new(&f);\n\
         \x20   f.state = STATE_{state};\n\
         \x20   f.vars = {vars};\n\
         \x20   input_t in = {input};\n\
         \x20   fsm_step(&f, in, cont_handler, 0);\n\
         \x20   CHECK(f.prop_label[0] != 0 && strcmp(f.prop_label, \"{label}\") == 0, \"the violating drive must record property '{label}' first\");\n\
         }}\n",
        state = c_ident(&mv.state).to_uppercase(),
        label = mv.label,
    ))
}

// ---------------------------------------------------------------------------
// Random-trace test
// ---------------------------------------------------------------------------

/// Render the deterministic random-trace test: drive the precomputed LCG input
/// sequence (seed + indices fixed at plan time - same constants as the fuzz
/// wire format) and print deterministic coverage counts.
fn render_random_trace_test_c(module: &Module, rt: &RandomTraceCase) -> String {
    let n_states = module
        .enums
        .iter()
        .find(|e| e.name == "State")
        .map(|e| e.variants.len())
        .unwrap_or(1)
        .max(1);
    let drive = if rt.input_indices.is_empty() {
        "    // no inputs declared; nothing to drive\n".to_string()
    } else {
        format!(
            "static const int RT_IDX[] = {{ {} }};\n",
            rt.input_indices
                .iter()
                .map(|i| i.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ) + "\n"
    };
    let loop_body = if rt.input_indices.is_empty() {
        String::new()
    } else {
        format!(
            "    for (int i = 0; i < {k}; i++) {{\n\
             \x20       input_t in = ALL_INPUTS[RT_IDX[i]];\n\
             \x20       step_result_t r = fsm_step(&f, in, stop_handler, 0);\n\
             \x20       if (r.status == RFS_OK) ok++;\n\
             \x20       else if (r.status == RFS_UNEXPECTED_EVENT) unexpected++;\n\
             \x20       else if (r.status == RFS_ASSUMPTION_VIOLATION) assumption++;\n\
             \x20       else property++;\n\
             \x20   }}\n",
            k = rt.steps
        )
    };
    format!(
        "{drive}\n\
         static void random_trace_seed{seed}_steps{k}(void) {{\n\
         \x20   fsm_t f; fsm_new(&f);\n\
         \x20   state_t initial = f.state;\n\
         \x20   int ok = 0, unexpected = 0, assumption = 0, property = 0;\n\
         {loop_body}\
         \x20   int seen[{ns}]; int n_seen = 0;\n\
         \x20   seen[n_seen++] = initial;\n\
         \x20   for (int i = 0; i < f.trace.len; i++) {{\n\
         \x20       state_t s = f.trace.steps[i].to;\n\
         \x20       int found = 0;\n\
         \x20       for (int j = 0; j < n_seen; j++) if (seen[j] == s) {{ found = 1; break; }}\n\
         \x20       if (!found && n_seen < {ns}) seen[n_seen++] = s;\n\
         \x20   }}\n\
         \x20   printf(\"random_trace seed={seed} steps={k} ok=%d unexpected=%d assumption=%d property=%d states_visited=%d\\n\", ok, unexpected, assumption, property, n_seen);\n\
         }}\n",
        seed = rt.seed,
        k = rt.steps,
        ns = n_states,
    )
}

// ---------------------------------------------------------------------------
// libFuzzer fuzz-target skeleton (separate file)
// ---------------------------------------------------------------------------

/// Number of little-endian bytes a `bit<W>` payload field decodes from.
fn payload_byte_count(width: u32) -> usize {
    (width as usize).div_ceil(8)
}

/// Emit the `fuzz_target.c` skeleton: an `LLVMFuzzerTestOneInput` that decodes
/// inputs per the planner's strategy and loops them over `fsm_step`. Depends
/// on a libFuzzer-enabled clang build - hence a separate file (the main test
/// artifact stays plain `gcc -Wall`). Building it is task 12's job.
pub fn emit_fuzz_target_c(
    module: &Module,
    variants: &FsmVariants,
    spec_hash: &str,
) -> Result<String, Vec<String>> {
    let decode = build_fuzz_decode(variants);
    let n_variants = decode.variants.len();
    let mut out = String::new();
    // Line 1 is the v1.0 unified artifact header (Phase 6, D-P6-3) - same
    // text as the main test artifact's line 1 (`//` is a valid C comment).
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
        "// This file is intentionally SEPARATE from the main test artifact:"
    );
    let _ = writeln!(
        out,
        "// building it needs a libFuzzer-enabled clang, which the plain"
    );
    let _ = writeln!(out, "// `gcc -Wall` test artifact must not require.");
    let _ = writeln!(out, "//");
    let _ = writeln!(out, "// Wiring (see the Phase 5 task 12 example):");
    let _ = writeln!(out, "//   1. Generate the FSM source next to this file:");
    let _ = writeln!(
        out,
        "//        rfs codegen c production <spec> --out rfs_generated_fsm.c"
    );
    let _ = writeln!(
        out,
        "//      (the `test` command output works too, but it carries a main())."
    );
    let _ = writeln!(
        out,
        "//   2. Adjust the #include below if you renamed the generated file."
    );
    let _ = writeln!(out, "//   3. Build with clang + libFuzzer:");
    let _ = writeln!(
        out,
        "//        clang -O1 -fsanitize=fuzzer,address -o fuzz_target fuzz_target.c"
    );
    let _ = writeln!(out, "//   4. Run:");
    let _ = writeln!(
        out,
        "//        ./fuzz_target            (add corpus dirs / -runs=N as needed)"
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
    let _ = writeln!(out, "#include <stdint.h>");
    let _ = writeln!(out, "#include <stddef.h>");
    let _ = writeln!(
        out,
        "// ADJUST to your wiring: the RFS-generated FSM source (production"
    );
    let _ = writeln!(out, "// command output) must be included here.");
    let _ = writeln!(out, "#include \"rfs_generated_fsm.c\"");
    let _ = writeln!(out);

    if n_variants == 0 {
        let _ = writeln!(
            out,
            "int LLVMFuzzerTestOneInput(const uint8_t* data, size_t size) {{"
        );
        let _ = writeln!(out, "    (void)data; (void)size;");
        let _ = writeln!(
            out,
            "    // The module declares no inputs; there is nothing to fuzz."
        );
        let _ = writeln!(out, "    return 0;");
        let _ = writeln!(out, "}}");
        return Ok(out);
    }

    let _ = writeln!(
        out,
        "static rfs_policy_t fuzz_stop_handler(void* user_data, const char* label) {{ (void)user_data; (void)label; return RFS_STOP; }}"
    );
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "int LLVMFuzzerTestOneInput(const uint8_t* data, size_t size) {{"
    );
    let _ = writeln!(out, "    fsm_t fsm;");
    let _ = writeln!(out, "    fsm_new(&fsm);");
    let _ = writeln!(out, "    size_t pos = 0;");
    let _ = writeln!(out, "    while (pos < size) {{");
    let _ = writeln!(
        out,
        "        int vi = (int)(data[pos] % {n});",
        n = n_variants
    );
    let _ = writeln!(out, "        pos += 1;");
    let _ = writeln!(out, "        input_t input = {{0}};");
    // Single-exit decode (Phase 5 task 16 close-out fix): a truncated field
    // run or an unknown variant sets `done` and breaks out of the loop so the
    // trailing fsm_free always runs - mirroring the Rust skeleton's break +
    // scope-drop, which keeps per-input allocations from accumulating.
    let _ = writeln!(out, "        int done = 0;");
    let _ = writeln!(out, "        switch (vi) {{");
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
        let _ = writeln!(out, "        case {vi}: {{");
        let _ = writeln!(
            out,
            "            input.kind = INPUT_{};",
            info.unique_name.to_uppercase()
        );
        if info.fields.is_empty() {
            let _ = writeln!(out, "            break;");
            let _ = writeln!(out, "        }}");
            continue;
        }
        if budget > 0 {
            let _ = writeln!(
                out,
                "            if (pos + {budget} > size) {{ done = 1; break; }}"
            );
        }
        for ((fname, strat), (_, fty)) in d.fields.iter().zip(info.fields.iter()) {
            let bind = c_ident(fname);
            match strat {
                FuzzFieldStrategy::ByteNonZero => {
                    let _ = writeln!(
                        out,
                        "            input.payload.input_{u}.{bind} = data[pos] != 0; pos += 1;",
                        u = info.unique_name.to_lowercase()
                    );
                }
                FuzzFieldStrategy::PayloadBytes { width } => {
                    let ty = bit_c_type(*width).map_err(|e| vec![format!("fuzz decode: {e}")])?;
                    let nbytes = payload_byte_count(*width);
                    // Accumulate little-endian bytes into the target type.
                    let mut acc = format!("(({ty})data[pos])");
                    for i in 1..nbytes {
                        acc = format!("({acc} | ((({ty})data[pos + {i}]) << {}))", 8 * i);
                    }
                    let masked = if *width >= 64 {
                        acc
                    } else {
                        format!("({acc}) & 0x{:X}", (1u64 << width) - 1)
                    };
                    let _ = writeln!(
                        out,
                        "            input.payload.input_{u}.{bind} = {masked}; pos += {nbytes};",
                        u = info.unique_name.to_lowercase()
                    );
                }
                FuzzFieldStrategy::Representative => match fty {
                    Type::Enum { name: enum_name } => {
                        let rep = module
                            .enums
                            .iter()
                            .find(|e| e.name.as_str() == enum_name)
                            .and_then(|e| e.variants.first())
                            .map(|v| {
                                format!(
                                    "{}_{}",
                                    c_ident(enum_name).to_uppercase(),
                                    c_ident(v).to_uppercase()
                                )
                            })
                            .unwrap_or_else(|| "0".to_string());
                        let _ = writeln!(
                            out,
                            "            // {fname}: constant representative (enum)."
                        );
                        let _ = writeln!(
                            out,
                            "            input.payload.input_{u}.{bind} = {rep};",
                            u = info.unique_name.to_lowercase()
                        );
                    }
                    _ => unreachable!("representative strategy only for enum fields"),
                },
            }
        }
        let _ = writeln!(out, "            break;");
        let _ = writeln!(out, "        }}");
    }
    let _ = writeln!(out, "        default: done = 1;");
    let _ = writeln!(out, "        }}");
    let _ = writeln!(out, "        if (done) break;");
    let _ = writeln!(
        out,
        "        (void)fsm_step(&fsm, input, fuzz_stop_handler, 0);"
    );
    let _ = writeln!(out, "    }}");
    let _ = writeln!(out, "    fsm_free(&fsm);");
    let _ = writeln!(out, "    return 0;");
    let _ = writeln!(out, "}}");
    Ok(out)
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Entry point for the `test` command: production FSM code + unit tests +
/// the separate libFuzzer fuzz-target skeleton (extra file).
/// Property monitors are always included (D-P3-6), so a spec that violates its
/// own property fails its generated tests.
pub fn generate_unit_tests(
    module: &Module,
    options: &BackendOptions,
) -> Result<CodegenOutput, Vec<String>> {
    if options.no_std {
        return Err(vec![
            crate::codegen::NO_STD_PRODUCTION_ONLY_ERROR.to_string()
        ]);
    }
    let prod = crate::backends::c_fsm::generate_core(
        module,
        options,
        crate::codegen::PropertyMonitorMode::Always,
    )?;
    let variants = codegen_fsm::build_variants(module);
    let plan = codegen_test::build_test_plan(&variants, module, options.steps);

    let candidates: Vec<String> = variants
        .inputs
        .iter()
        .map(|v| rep_input_c(v, module))
        .collect();
    let n_all = candidates.len().max(1);
    let cand_list = if candidates.is_empty() {
        "{{ 0 }}".to_string()
    } else {
        format!("{{ {} }}", candidates.join(", "))
    };

    let init_state = module
        .initial_state
        .clone()
        .unwrap_or_else(|| "__None__".into());
    let k = plan.steps;

    // `cont_handler` is only emitted when at least one test uses it (modules
    // without transitions/monitor drives must stay warning-free under -Wall).
    let neg_uses_cont = plan.has_on_error || plan.definite_unexpected;
    let need_cont = !plan.transitions.is_empty()
        || plan
            .monitor_violations
            .iter()
            .any(|m| m.step_input.is_some())
        || neg_uses_cont;
    let cont_handler_def = if need_cont {
        "static rfs_policy_t cont_handler(void* user_data, const char* label) { (void)user_data; (void)label; return RFS_CONTINUE; }\n"
    } else {
        ""
    };
    let harness = format!(
        "\n// ---- generated unit tests (plain-assert harness) ----\n\
         static int rfs_test_failures = 0;\n\
         #define CHECK(cond, msg) do {{ if (!(cond)) {{ rfs_test_failures++; printf(\"FAIL: %s\\n\", msg); }} }} while (0)\n\
         static rfs_policy_t stop_handler(void* user_data, const char* label) {{ (void)user_data; (void)label; return RFS_STOP; }}\n\
         {cont_handler_def}\
         static fsm_t clone_fsm(const fsm_t* f) {{\n\
             fsm_t t = *f;\n\
             if (f->trace.steps != 0) {{\n\
                 t.trace.cap = f->trace.cap;\n\
                 t.trace.steps = (trace_step_t*)malloc((size_t)t.trace.cap * sizeof(trace_step_t));\n\
                 memcpy(t.trace.steps, f->trace.steps, (size_t)f->trace.len * sizeof(trace_step_t));\n\
             }} else {{ t.trace.steps = 0; }}\n\
             return t;\n\
         }}\n\
         static const input_t ALL_INPUTS[{n_all}] = {cand_list};\n\
         #define N_ALL_INPUTS (sizeof(ALL_INPUTS) / sizeof(ALL_INPUTS[0]))\n\
         #define EXPECT_PROGRESS ({expect_progress})\n",
        n_all = n_all,
        cand_list = cand_list,
        expect_progress = plan.expect_progress,
        cont_handler_def = cont_handler_def,
    );

    let initial_state_block = format!(
        "static void initial_state_is_correct(void) {{\n\
         \x20   fsm_t f; fsm_new(&f);\n\
         \x20   CHECK(f.state == STATE_{}, \"initial state\");\n\
         }}\n",
        c_ident(&init_state).to_uppercase()
    );

    let positive = format!(
        "static void positive_sequence_runs(void) {{\n\
         \x20   fsm_t f; fsm_new(&f);\n\
         \x20   int steps = 0;\n\
         \x20   for (int i = 0; i < {k}; i++) {{\n\
         \x20       int progressed = 0;\n\
         \x20       for (int c = 0; c < N_ALL_INPUTS; c++) {{\n\
         \x20           fsm_t t = clone_fsm(&f);\n\
         \x20           step_result_t r = fsm_step(&t, ALL_INPUTS[c], stop_handler, 0);\n\
         \x20           if (r.status == RFS_OK) {{ f = t; steps++; progressed = 1; break; }}\n\
         \x20       }}\n\
         \x20       if (!progressed) break;\n\
         \x20   }}\n\
         \x20   CHECK(steps > 0 || !EXPECT_PROGRESS || N_ALL_INPUTS == 0, \"positive sequence made no progress\");\n\
         }}\n"
    );

    let negative = render_negative_c(&plan, &variants, module, k);

    // Extended plan classes (Phase 5 task 4/6) - additive, after the
    // module-level tests.
    let mut extended = String::new();
    let mut test_names: Vec<String> = vec![
        "initial_state_is_correct".to_string(),
        "positive_sequence_runs".to_string(),
        "negative_unexpected_input_search".to_string(),
    ];
    for c in &plan.transitions {
        extended.push_str(&render_transition_test_c(module, &variants, c)?);
        let vinfo = variants
            .input_variant(&c.input.group, &c.input.variant)
            .expect("planner input variant resolves");
        let base = format!(
            "t{}_{}_on_{}",
            c.transition_id,
            snake(&c.from_state),
            snake(&vinfo.unique_name)
        );
        test_names.push(match &c.expectation {
            TransitionExpectation::Taken { .. } => base,
            TransitionExpectation::NotTaken { .. } => format!("{base}_not_taken"),
        });
    }
    for inv in &plan.invariants {
        extended.push_str(&render_invariant_test_c(module, &variants, inv, k)?);
        test_names.push(format!("invariant_{}", snake(&inv.label)));
    }
    for mv in &plan.monitor_violations {
        extended.push_str(&render_monitor_test_c(module, &variants, mv)?);
        test_names.push(format!("monitor_{}_fires_on_violation", snake(&mv.label)));
    }
    let rt_name = format!(
        "random_trace_seed{}_steps{}",
        plan.random_trace.seed, plan.steps
    );
    extended.push_str(&render_random_trace_test_c(module, &plan.random_trace));
    test_names.push(rt_name);

    let table_entries: Vec<String> = test_names
        .iter()
        .map(|n| format!("    {{\"{n}\", {n}}},"))
        .collect();

    let closing = "int main(void) {\n\
                   \x20   static const struct { const char* name; void (*fn)(void); } tests[] = {\n\
                   {table}\n\
                   \x20   };\n\
                   \x20   for (size_t i = 0; i < sizeof(tests) / sizeof(tests[0]); i++) {\n\
                   \x20       int before = rfs_test_failures;\n\
                   \x20       tests[i].fn();\n\
                   \x20       printf(\"%s %s\\n\", rfs_test_failures == before ? \"PASS\" : \"FAIL\", tests[i].name);\n\
                   \x20   }\n\
                   \x20   if (rfs_test_failures == 0) { printf(\"all generated C tests passed\\n\"); return 0; }\n\
                   \x20   printf(\"%d generated C test(s) failed\\n\", rfs_test_failures);\n\
                   \x20   return 1;\n\
                   }\n";

    let mut code = prod.code;
    code.push_str(&harness);
    code.push_str(&initial_state_block);
    code.push_str(&positive);
    code.push_str(&negative);
    code.push_str(&extended);
    code.push_str(&closing.replace("{table}", &table_entries.join("\n")));

    // The fuzz skeleton is a SEPARATE file: it needs a libFuzzer-enabled
    // clang build, which the dependency-free test artifact must not pull in.
    let mut result = CodegenOutput {
        code,
        warnings: prod.warnings,
        extra_files: Vec::new(),
    };
    result.extra_files.push(ExtraFile {
        name: "fuzz_target.c".to_string(),
        content: emit_fuzz_target_c(module, &variants, &options.spec_hash)?,
    });
    Ok(result)
}

fn render_negative_c(
    plan: &codegen_test::TestPlan,
    variants: &FsmVariants,
    module: &Module,
    k: usize,
) -> String {
    let definite = codegen_test::has_definite_unexpected(variants, module);
    if plan.has_on_error {
        let oe = module.on_error.as_ref().unwrap();
        let assert_line = if definite {
            "CHECK(on_error_found, \"on_error fallback was never reached\");".to_string()
        } else {
            "// No statically obvious unexpected representative input.".to_string()
        };
        format!(
            "static void negative_unexpected_input_search(void) {{\n\
             \x20   fsm_t f; fsm_new(&f);\n\
             \x20   int on_error_found = 0;\n\
             \x20   for (int i = 0; i < {k}; i++) {{\n\
             \x20       for (int c = 0; c < N_ALL_INPUTS; c++) {{\n\
             \x20           fsm_t t = clone_fsm(&f);\n\
             \x20           step_result_t r = fsm_step(&t, ALL_INPUTS[c], cont_handler, 0);\n\
             \x20           if (r.status == RFS_UNEXPECTED_EVENT) break;\n\
             \x20           if (t.state == STATE_{oe_state}) {{ on_error_found = 1; break; }}\n\
             \x20       }}\n\
             \x20       if (on_error_found) break;\n\
             \x20       int progressed = 0;\n\
             \x20       for (int c = 0; c < N_ALL_INPUTS; c++) {{\n\
             \x20           fsm_t t = clone_fsm(&f);\n\
             \x20           step_result_t r = fsm_step(&t, ALL_INPUTS[c], cont_handler, 0);\n\
             \x20           if (r.status == RFS_OK) {{ f = t; progressed = 1; break; }}\n\
             \x20       }}\n\
             \x20       if (!progressed) break;\n\
             \x20   }}\n\
             \x20   {assert_line}\n\
             }}\n",
            oe_state = c_ident(&oe.next_state).to_uppercase()
        )
    } else if plan.definite_unexpected {
        format!(
            "static void negative_unexpected_input_search(void) {{\n\
             \x20   fsm_t f; fsm_new(&f);\n\
             \x20   int found = 0;\n\
             \x20   for (int i = 0; i < {k}; i++) {{\n\
             \x20       for (int c = 0; c < N_ALL_INPUTS; c++) {{\n\
             \x20           fsm_t t = clone_fsm(&f);\n\
             \x20           step_result_t r = fsm_step(&t, ALL_INPUTS[c], cont_handler, 0);\n\
             \x20           if (r.status == RFS_UNEXPECTED_EVENT) {{ found = 1; break; }}\n\
             \x20       }}\n\
             \x20       if (found) break;\n\
             \x20       int progressed = 0;\n\
             \x20       for (int c = 0; c < N_ALL_INPUTS; c++) {{\n\
             \x20           fsm_t t = clone_fsm(&f);\n\
             \x20           step_result_t r = fsm_step(&t, ALL_INPUTS[c], cont_handler, 0);\n\
             \x20           if (r.status == RFS_OK) {{ f = t; progressed = 1; break; }}\n\
             \x20       }}\n\
             \x20       if (!progressed) break;\n\
             \x20   }}\n\
             \x20   CHECK(found, \"expected an unexpected-event failure\");\n\
             }}\n"
        )
    } else {
        "static void negative_unexpected_input_search(void) {\n\
         \x20   // No statically obvious unexpected representative input.\n\
         }\n"
        .to_string()
    }
}
