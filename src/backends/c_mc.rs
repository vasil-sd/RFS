//! C backend: model-checker executable generation.
//!
//! The generated program has the **same runtime wire format** as the Rust
//! model checker (see `crate::codegen_mc`):
//!
//! - **exact mode** (default, Phase 4): a level-synchronous BFS over the
//!   reachable configurations. The input enumeration and configuration
//!   packing come from the codegen-time plan (`crate::mc_plan`, D-P4-1..3)
//!   embedded as C constants, so the Rust and C engines are identical by
//!   construction. The visited set is a SORTED VECTOR + BINARY SEARCH (a
//!   deliberate simple choice; behavior must not depend on the container -
//!   task 10 may switch it to open addressing with benchmark data).
//! - **fuzz mode** (`--mode fuzz`, Phase 3): random bounded runs with forced
//!   violation injection; behavior unchanged from Phase 3.
//!
//! Hard requirement: the generated C compiles with plain `gcc -Wall` without
//! warnings.

use crate::backends::c::{c_ident, int_c_literal};
use crate::codegen::{BackendOptions, CodegenOutput, McMode};
use crate::codegen_fsm::{self, VariantInfo};
use crate::ir::*;
use crate::mc_plan::{McPlan, ValueDomain};

/// Entry point for the `mc` command of the C backend.
pub fn generate_model_checker(
    module: &Module,
    options: &BackendOptions,
) -> Result<CodegenOutput, Vec<String>> {
    if options.no_std {
        return Err(vec![
            crate::codegen::NO_STD_PRODUCTION_ONLY_ERROR.to_string()
        ]);
    }
    // The ProtocolFsm impl belongs to the production/test surface (Phase 5
    // task 11); the model checker keeps its own runtime.
    if options.with_protocol_fsm {
        return Err(vec![crate::codegen::PROTOCOL_FSM_MC_ERROR.to_string()]);
    }
    // SMT bounded mode (task 11): a standalone libz3-based binary - no FSM
    // core, no exact/fuzz runtimes. The spec section is rendered from the
    // shared plan (smt_plan/smt_emit) so both backends build identical
    // constraints (parity by construction, D-P4-12).
    if options.mc_mode == McMode::SmtBounded {
        let plan = McPlan::compute(module, &options.assumptions);
        let spec = crate::smt_plan::build_smt_spec(module, &plan).map_err(|e| vec![e])?;
        let code = crate::smt_emit::emit_c_spec(&spec)
            + &SMT_ENGINE_C.replace("{BOUND}", &options.mc_smt_bound.to_string());
        return Ok(CodegenOutput {
            code,
            warnings: Vec::new(),
            extra_files: Vec::new(),
        });
    }
    // The model checker reuses the production FSM core without unit tests.
    // Property monitors are always included (D-P3-6).
    let prod = crate::backends::c_fsm::generate_core(
        module,
        options,
        crate::codegen::PropertyMonitorMode::Always,
    )?;
    let plan = McPlan::compute(module, &options.assumptions);
    let mut code = prod.code;
    // Shared trace/vars printers (used by both runtimes; defined once).
    code.push_str(&shared_printers(module));
    let (exact, liv_warnings) = exact_codegen_c(module, &plan, options).map_err(|e| vec![e])?;
    code.push_str(&exact);
    code.push_str(&fuzz_runtime(module));
    code.push_str(MAIN_DISPATCH_C);
    Ok(CodegenOutput {
        code,
        warnings: prod.warnings.into_iter().chain(liv_warnings).collect(),
        extra_files: Vec::new(),
    })
}

/// Trace/vars printers shared by the exact and fuzz runtimes. Generates C
/// functions that reproduce byte-for-byte the Rust MC binary's Debug-based
/// counterexample format (D-P4-4): state/input/output variant names,
/// `Vars { ... }` and `[...]` output lists - so FAIL traces are identical
/// across backends.
fn shared_printers(module: &Module) -> String {
    let variants = codegen_fsm::build_variants(module);

    // Enum types referenced by vars fields or input/output payload fields -
    // only those get a name-lookup function (an unused static would trip
    // gcc -Wall). `State` is covered by the dedicated lookup below.
    let mut used_enums: Vec<String> = Vec::new();
    for v in &module.vars {
        if let Type::Enum { name } = &v.ty {
            if name != "State" && !used_enums.contains(name) {
                used_enums.push(name.clone());
            }
        }
    }
    for list in [&variants.inputs, &variants.outputs] {
        for vi in list {
            for (_, fty) in &vi.fields {
                if let Type::Enum { name } = fty {
                    if name != "State" && !used_enums.contains(name) {
                        used_enums.push(name.clone());
                    }
                }
            }
        }
    }

    let mut out = String::new();
    for ename in &used_enums {
        let e = module
            .enums
            .iter()
            .find(|e| e.name == *ename)
            .unwrap_or_else(|| panic!("enum {ename} referenced by a field is not declared"));
        let arms: Vec<String> = e
            .variants
            .iter()
            .map(|v| {
                format!(
                    "case {}_{}: return \"{}\";",
                    c_ident(ename).to_uppercase(),
                    c_ident(v).to_uppercase(),
                    crate::backends::rust::rust_ident(v)
                )
            })
            .collect();
        let tname = format!("{}_t", c_ident(ename));
        out.push_str(&format!(
            "static const char* {c}_name_c({t} v) {{ switch (v) {{ {arms} default: return \"?\"; }} }}\n",
            c = c_ident(ename),
            t = tname,
            arms = arms.join(" ")
        ));
    }

    // State name lookup (Rust Debug names).
    if let Some(se) = module.enums.iter().find(|e| e.name == "State") {
        let arms: Vec<String> = se
            .variants
            .iter()
            .map(|v| {
                format!(
                    "case STATE_{}: return \"{}\";",
                    c_ident(v).to_uppercase(),
                    crate::backends::rust::rust_ident(v)
                )
            })
            .collect();
        out.push_str(&format!(
            "static const char* state_name_c(state_t s) {{ switch (s) {{ {} default: return \"?\"; }} }}\n",
            arms.join(" ")
        ));
    } else {
        out.push_str("static const char* state_name_c(state_t s) { (void)s; return \"?\"; }\n");
    }

    // `Vars { f: v, ... }` - Rust derived Debug, no trailing newline.
    if module.vars.is_empty() {
        out.push_str(
            "static void print_vars_debug(const vars_t* v) { (void)v; printf(\"Vars\"); }\n",
        );
    } else {
        let parts: Vec<String> = module
            .vars
            .iter()
            .enumerate()
            .map(|(i, v)| {
                let sep = if i == 0 { "" } else { ", " };
                format!(
                    "printf(\"{sep}{name}: \"); {val}",
                    name = crate::backends::rust::rust_ident(&v.name),
                    val = print_value_c(&v.ty, &format!("v->{}", c_ident(&v.name)))
                )
            })
            .collect();
        out.push_str(&format!(
            "static void print_vars_debug(const vars_t* v) {{ printf(\"Vars {{ \"); {parts} printf(\" }}\"); }}\n",
            parts = parts.join(" ")
        ));
    }

    // Input Debug: unit variant name or `Name { f: v, ... }`.
    let iarms: Vec<String> = variants
        .inputs
        .iter()
        .map(|v| {
            let kind = format!("INPUT_{}", v.unique_name.to_uppercase());
            if v.fields.is_empty() {
                format!(
                    "case {}: printf(\"{}\"); break;",
                    kind,
                    crate::backends::rust::rust_ident(&v.unique_name)
                )
            } else {
                let member = format!("input_{}", v.unique_name.to_lowercase());
                let parts: Vec<String> = v
                    .fields
                    .iter()
                    .enumerate()
                    .map(|(i, (fname, fty))| {
                        let sep = if i == 0 { "" } else { ", " };
                        format!(
                            "printf(\"{sep}{name}: \"); {val}",
                            name = crate::backends::rust::rust_ident(fname),
                            val = print_value_c(
                                fty,
                                &format!("in->payload.{member}.{cf}", cf = c_ident(fname))
                            )
                        )
                    })
                    .collect();
                format!(
                    "case {}: printf(\"{} {{ \"); {parts} printf(\" }}\"); break;",
                    kind,
                    crate::backends::rust::rust_ident(&v.unique_name),
                    parts = parts.join(" ")
                )
            }
        })
        .collect();
    out.push_str(&format!(
        "static void print_input_debug(const input_t* in) {{ switch (in->kind) {{ {arms} default: printf(\"?\"); break; }} }}\n",
        arms = iarms.join(" ")
    ));

    // Output list Debug: `[X, Y]`.
    let oarms: Vec<String> = variants
        .outputs
        .iter()
        .map(|v| {
            let kind = format!("OUTPUT_{}", v.unique_name.to_uppercase());
            if v.fields.is_empty() {
                format!(
                    "case {}: printf(\"{}\"); break;",
                    kind,
                    crate::backends::rust::rust_ident(&v.unique_name)
                )
            } else {
                let member = format!("output_{}", v.unique_name.to_lowercase());
                let parts: Vec<String> = v
                    .fields
                    .iter()
                    .enumerate()
                    .map(|(i, (fname, fty))| {
                        let sep = if i == 0 { "" } else { ", " };
                        format!(
                            "printf(\"{sep}{name}: \"); {val}",
                            name = crate::backends::rust::rust_ident(fname),
                            val = print_value_c(
                                fty,
                                &format!("outs[i].payload.{member}.{cf}", cf = c_ident(fname))
                            )
                        )
                    })
                    .collect();
                format!(
                    "case {}: printf(\"{} {{ \"); {parts} printf(\" }}\"); break;",
                    kind,
                    crate::backends::rust::rust_ident(&v.unique_name),
                    parts = parts.join(" ")
                )
            }
        })
        .collect();
    out.push_str(&format!(
        "static void print_outputs_debug(const output_t* outs, int n) {{ int i; printf(\"[\"); for (i = 0; i < n; i++) {{ if (i > 0) {{ printf(\", \"); }} switch (outs[i].kind) {{ {arms} default: printf(\"?\"); break; }} }} printf(\"]\"); }}\n",
        arms = oarms.join(" ")
    ));

    // The trace itself - byte-identical to the Rust MC binary.
    out.push_str(
        r#"static void print_trace(const rfs_trace_t* trace) {
printf("counterexample trace:\n");
printf("  initial state=%s vars=", state_name_c(trace->initial_state));
print_vars_debug(&trace->initial_vars);
printf("\n");
for (int i = 0; i < trace->len; i++) {
    const trace_step_t* s = &trace->steps[i];
    printf("  step %d input=", i);
    print_input_debug(&s->input);
    printf(" outputs=");
    print_outputs_debug(s->outputs, s->n_outputs);
    printf(" from=%s to=%s vars=", state_name_c(s->from), state_name_c(s->to));
    print_vars_debug(&s->vars);
    printf("\n");
}
}
"#,
    );

    out
}

/// C statement printing the Rust Debug rendering of `expr` (type `ty`).
fn print_value_c(ty: &Type, expr: &str) -> String {
    match ty {
        Type::Bool => format!("printf(\"%s\", {} ? \"true\" : \"false\");", expr),
        Type::Bit { .. } => format!("printf(\"%llu\", (unsigned long long){});", expr),
        Type::Enum { name } => format!("printf(\"%s\", {}_name_c({}));", c_ident(name), expr),
    }
}

/// Canonical iterator bounds for a concrete field domain (D-P4-2). Returns
/// (loop variable type is always uint64_t) the condition and the value
/// expression for loop counter `_c{k}`.
fn domain_loop(d: &ValueDomain, k: usize) -> (String, String) {
    match d {
        ValueDomain::Bool => (format!("_c{k} < 2"), format!("(_c{k} != 0)")),
        ValueDomain::Int { max } => (
            format!("_c{k} <= {}", int_c_literal(*max, None)),
            format!("_c{k}"),
        ),
        ValueDomain::Enum { variants, .. } => (
            format!("_c{k} < {}", variants.len()),
            format!("_dom_{k}[(size_t)_c{k}]"),
        ),
    }
}

/// Representative value expression for an abstracted field (C).
fn rep_value_c(v: &crate::codegen_fsm::RepValue) -> Result<String, String> {
    use crate::codegen_fsm::RepValue;
    match v {
        RepValue::Bool(b) => Ok(b.to_string()),
        RepValue::Int(i) => Ok(int_c_literal(*i, None)),
        RepValue::EnumVariant(path) => {
            let base = c_ident(&path[0]);
            let rest = path[1..]
                .iter()
                .map(|x| c_ident(x).to_uppercase())
                .collect::<Vec<_>>()
                .join("_");
            Ok(format!("{}_{}", base.to_uppercase(), rest))
        }
        other => Err(format!(
            "representative value {other:?} has no C rendering (empty/unknown enum)"
        )),
    }
}

/// Size of a concrete field's domain (for N_TOTAL_INPUTS).
fn domain_size(d: &ValueDomain) -> usize {
    match d {
        ValueDomain::Bool => 2,
        ValueDomain::Int { max } => (*max as usize).saturating_add(1),
        ValueDomain::Enum { variants, .. } => variants.len(),
    }
}

/// Representative `input_t` initializers (same defaults as the C tests).
fn rep_inputs(module: &Module) -> Vec<String> {
    let variants = codegen_fsm::build_variants(module);
    variants
        .inputs
        .iter()
        .map(|v| rep_input_c(v, module))
        .collect()
}

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

/// Render the exact-mode C runtime from the plan (D-P4-1..6). Returns the
/// code plus liveness decomposition diagnostics (codegen warnings, D-P4-9).
fn exact_codegen_c(
    module: &Module,
    plan: &McPlan,
    options: &BackendOptions,
) -> Result<(String, Vec<String>), String> {
    let mut s = String::new();
    s.push_str(
        "// ---- exact reachable-state engine (Phase 4; same wire format as the Rust MC) ----\n",
    );
    s.push_str("#include <time.h>\n#include <pthread.h>\n#include <stdatomic.h>\n");
    // Codegen-time defaults for the runtime flags (D-P4-7).
    let mode_const = if options.mc_mode == McMode::Fuzz {
        1
    } else {
        0
    };
    s.push_str(&format!("#define DEFAULT_MODE {}\n", mode_const));
    s.push_str(&format!(
        "#define DEFAULT_MAX_STATES {}\n",
        options.mc_max_states
    ));
    s.push_str(&format!(
        "#define DEFAULT_NONDET {}\n",
        options.mc_nondet as i32
    ));
    s.push_str(&format!(
        "#define DEFAULT_TIMEOUT_SEC {}\n",
        options.mc_timeout_sec
    ));
    s.push_str(&format!("#define DEFAULT_THREADS {}\n", options.mc_threads));
    // Counterexample report format (Phase 5 task 10): 0 = text (default),
    // 1 = json. Runtime-overridable with --counterexample-format.
    s.push_str(&format!(
        "#define DEFAULT_COUNTEREXAMPLE_FORMAT {}\n",
        if options.counterexample_format == crate::codegen_mc::CounterexampleFormat::Json {
            1
        } else {
            0
        }
    ));
    s.push_str(&format!(
        "#define CONFIG_TOTAL_BYTES {}\n",
        plan.encoding.total_bytes
    ));
    s.push_str(&format!(
        "#define MONITORS_BITS {}\n",
        plan.encoding.monitors_bits
    ));
    s.push_str(&format!(
        "#define STATE_BITS {}\n",
        plan.encoding.state_bits
    ));

    // all_inputs(): canonical enumeration (D-P4-2).
    let total_inputs: usize = plan
        .abstraction
        .variants
        .iter()
        .map(|vs| {
            vs.fields
                .iter()
                .filter(|f| f.concrete)
                .map(|f| domain_size(f.domain.as_ref().unwrap()))
                .product::<usize>()
                .max(1)
        })
        .sum();
    s.push_str(&format!("#define N_TOTAL_INPUTS {}\n", total_inputs.max(1)));
    // Max effects per (state, input): transitions of one state + on_error.
    let mut per_state: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for t in &module.transitions {
        *per_state.entry(t.state.as_str()).or_insert(0) += 1;
    }
    let max_effects = per_state.values().max().copied().unwrap_or(0)
        + if module.on_error.is_some() { 1 } else { 0 };
    s.push_str(&format!("#define MC_MAX_EFFECTS {}\n", max_effects.max(1)));

    s.push_str("static int all_inputs(input_t* out) {\n    int n = 0;\n");
    for vs in &plan.abstraction.variants {
        if vs.fields.is_empty() {
            s.push_str(&format!(
                "    out[n].kind = INPUT_{}; n++;\n",
                vs.unique_name.to_uppercase()
            ));
            continue;
        }
        let pl = format!("out[n].payload.input_{}", vs.unique_name.to_lowercase());
        // Field types from the original variant info (for casts/domain arrays).
        let variants = codegen_fsm::build_variants(module);
        let vinfo = variants
            .inputs
            .iter()
            .find(|v| v.unique_name == vs.unique_name)
            .ok_or_else(|| format!("variant {} not found", vs.unique_name))?;
        let mut concrete_idx: Vec<usize> = Vec::new();
        for (i, f) in vs.fields.iter().enumerate() {
            if f.concrete {
                if let ValueDomain::Enum { variants, .. } = f.domain.as_ref().unwrap() {
                    let ety = crate::backends::c::c_type(&vinfo.fields[i].1)
                        .map_err(|e| e.to_string())?;
                    let vals: Vec<String> = variants
                        .iter()
                        .map(|v| {
                            format!(
                                "{}_{}",
                                c_ident(ety.trim_end_matches("_t")).to_uppercase(),
                                c_ident(v).to_uppercase()
                            )
                        })
                        .collect();
                    s.push_str(&format!(
                        "    static const {} _dom_{}[] = {{ {} }};\n",
                        ety,
                        concrete_idx.len(),
                        vals.join(", ")
                    ));
                }
                concrete_idx.push(i);
            }
        }
        // Nested loops over concrete fields.
        let mut loop_stack: Vec<(String, String)> = Vec::new();
        for (i, f) in vs.fields.iter().enumerate() {
            if f.concrete {
                let k = loop_stack.len();
                let (cond, _val) = domain_loop(f.domain.as_ref().unwrap(), k);
                s.push_str(&format!(
                    "    for (uint64_t _c{k} = 0; {cond}; _c{k}++) {{\n"
                ));
                loop_stack.push((i.to_string(), String::new()));
            }
        }
        // Push statement.
        s.push_str(&format!(
            "    out[n].kind = INPUT_{};\n",
            vs.unique_name.to_uppercase()
        ));
        let mut k = 0usize;
        for (i, f) in vs.fields.iter().enumerate() {
            let cast = crate::backends::c::c_type(&vinfo.fields[i].1).map_err(|e| e.to_string())?;
            if f.concrete {
                let (_cond, val) = domain_loop(f.domain.as_ref().unwrap(), k);
                s.push_str(&format!(
                    "    {}.{} = ({}){};\n",
                    pl,
                    c_ident(&f.name),
                    cast,
                    val
                ));
                k += 1;
            } else {
                let rv = rep_value_c(f.representative.as_ref().unwrap())?;
                s.push_str(&format!(
                    "    {}.{} = ({}){};\n",
                    pl,
                    c_ident(&f.name),
                    cast,
                    rv
                ));
            }
        }
        s.push_str("    n++;\n");
        for _ in 0..loop_stack.len() {
            s.push_str("    }\n");
        }
    }
    s.push_str("    return n;\n}\n");

    // put_bits + config_bytes (plan layout, D-P4-2).
    s.push_str(
        "static void put_bits(uint8_t* buf, int offset, uint64_t value, int width) {\n    for (int i = 0; i < width; i++) {\n        if ((value >> i) & 1ULL) {\n            int b = offset + i;\n            buf[b / 8] |= (uint8_t)(1u << (b % 8));\n        }\n    }\n}\n",
    );
    s.push_str(
        "static void config_bytes(state_t state, const vars_t* vars, const prop_monitors_t* prop, const assume_monitors_t* assume, uint8_t* buf) {\n    (void)vars; (void)prop; (void)assume;\n    memset(buf, 0, CONFIG_TOTAL_BYTES);\n",
    );
    s.push_str("    put_bits(buf, 0, (uint64_t)state, STATE_BITS);\n");
    for slot in &plan.encoding.var_slots {
        let ve = var_value_c(slot, module);
        s.push_str(&format!(
            "    put_bits(buf, {}, {}, {});\n",
            slot.offset_bits, ve, slot.width_bits
        ));
    }
    for m in &plan.encoding.monitors {
        let agg = if m.is_assumption { "assume" } else { "prop" };
        for n in &m.nodes {
            let base = format!("{}->mon_{}", agg, crate::backends::rust::snake(&m.label));
            let nb = m.offset_bits;
            if n.pending_bits > 0 {
                s.push_str(&format!(
                    "    put_bits(buf, {}, {}.n{}_pending ? 1ULL : 0ULL, {});\n",
                    nb + n.offset_bits,
                    base,
                    n.node_id,
                    n.pending_bits
                ));
            }
            if n.remaining_bits > 0 {
                s.push_str(&format!(
                    "    put_bits(buf, {}, (uint64_t)({}.n{}_remaining), {});\n",
                    nb + n.offset_bits + n.pending_bits,
                    base,
                    n.node_id,
                    n.remaining_bits
                ));
            }
            if n.antecedent_bits > 0 {
                s.push_str(&format!(
                    "    put_bits(buf, {}, {}.n{}_prev_antecedent ? 1ULL : 0ULL, {});\n",
                    nb + n.offset_bits + n.pending_bits + n.remaining_bits,
                    base,
                    n.node_id,
                    n.antecedent_bits
                ));
            }
        }
    }
    s.push_str("}\n");

    // mc_step_all: every enabled transition's effect in firing order (D-P4-11).
    s.push_str(&crate::backends::c_fsm::render_mc_step_all_c(module)?);

    // The BFS engine (static; see D-P4-2..6).
    s.push_str(EXACT_ENGINE_C);

    // JSON counterexample report (Phase 5 task 10): embedded port of the
    // toolchain writer + canonical document builders, then the
    // codegen-specific input/output/vars renderers for this module.
    s.push_str(JSON_WRITER_C);
    s.push_str(&json_renderers_c(module));

    // Liveness (Phase 4 task 7, D-P4-9/10/11): C mirror of the Rust
    // LIV_ENGINE - per-leaf progress automata + product limit-vector analysis
    // on the explored graph, rendered from the same decomposition.
    let liv = crate::mc_plan::LivenessDecomposition::compute(module);
    let warnings: Vec<String> = liv.diagnostics.clone();
    s.push_str(LIV_ENGINE_C);
    let mut prop_defs: Vec<(usize, String)> = Vec::new();
    for (pi, spec_opt) in liv.properties.iter().enumerate() {
        let Some(spec) = spec_opt else { continue };
        // Flatten the skeleton: leaves in first-appearance order.
        let mut leaves: Vec<crate::mc_plan::SkeletonLeaf> = Vec::new();
        let skel_expr = flatten_skeleton_c(&spec.skeleton, &mut leaves);
        let mut leaf_entries: Vec<String> = Vec::new();
        for (li, leaf) in leaves.iter().enumerate() {
            match leaf {
                crate::mc_plan::SkeletonLeaf::Safety(f) => {
                    let name = format!("liv_p{pi}_l{li}");
                    s.push_str(&crate::backends::c_fsm::render_liv_sample_evaluator_c(
                        module, &name, f,
                    )?);
                    leaf_entries.push(format!("{{ 0, {name}, 0 }}"));
                }
                crate::mc_plan::SkeletonLeaf::Obligation(oi) => match &spec.obligations[*oi] {
                    crate::mc_plan::LivenessObligation::Eventually { body } => {
                        let name = format!("liv_p{pi}_l{li}");
                        s.push_str(&crate::backends::c_fsm::render_liv_sample_evaluator_c(
                            module, &name, body,
                        )?);
                        leaf_entries.push(format!("{{ 1, {name}, 0 }}"));
                    }
                    crate::mc_plan::LivenessObligation::Until { left, right } => {
                        let na = format!("liv_p{pi}_l{li}");
                        let nb = format!("liv_p{pi}_l{li}b");
                        s.push_str(&crate::backends::c_fsm::render_liv_sample_evaluator_c(
                            module, &na, left,
                        )?);
                        s.push_str(&crate::backends::c_fsm::render_liv_sample_evaluator_c(
                            module, &nb, right,
                        )?);
                        leaf_entries.push(format!("{{ 2, {na}, {nb} }}"));
                    }
                },
            }
        }
        s.push_str(&format!(
            "static int liv_p{pi}_skel(const unsigned char* t) {{ return {skel_expr}; }}\n"
        ));
        let formula_text = crate::printer::print_formula(&module.properties[pi].formula);
        // A liveness-bearing property always has >= 1 leaf; the dummy entry
        // keeps the C array valid in the (unreachable) zero-leaf case and is
        // never dereferenced (liv_analyze returns early for n_leaves == 0).
        let entries = if leaf_entries.is_empty() {
            "{ 0, 0, 0 }".to_string()
        } else {
            leaf_entries.join(", ")
        };
        prop_defs.push((
            pi,
            format!(
                "static const liv_leaf_t LIV_P{pi}_LEAVES[{}] = {{ {} }};\n\
                 static const liv_property_t LIV_PROP_{pi} = {{ {}, {}, LIV_P{pi}_LEAVES, {}, liv_p{pi}_skel }};",
                leaves.len().max(1),
                entries,
                c_string_literal(&spec.label),
                c_string_literal(&formula_text),
                leaves.len()
            ),
        ));
    }
    if prop_defs.is_empty() {
        // No liveness properties: the table still needs a valid definition
        // (the loop below never touches it when N_LIV_PROPS == 0).
        s.push_str(
            "static const liv_leaf_t LIV_P_DUMMY_LEAVES[1] = { { 0, 0, 0 } };\n\
             static const liv_property_t LIV_PROP_DUMMY = { \"-\", \"\", LIV_P_DUMMY_LEAVES, 0, 0 };\n\
             static const liv_property_t* const LIV_PROPS[1] = { &LIV_PROP_DUMMY };\n\
             #define N_LIV_PROPS 0\n",
        );
    } else {
        for (_pi, d) in &prop_defs {
            s.push_str(d);
            s.push('\n');
        }
        let names: Vec<String> = prop_defs
            .iter()
            .map(|(pi, _)| format!("&LIV_PROP_{pi}"))
            .collect();
        s.push_str(&format!(
            "static const liv_property_t* const LIV_PROPS[{}] = {{ {} }};\n#define N_LIV_PROPS {}\n",
            prop_defs.len(),
            names.join(", "),
            prop_defs.len()
        ));
    }

    s.push_str(EXACT_RUN_C);
    Ok((s, warnings))
}

/// Codegen-specific JSON renderers for this module's input/output/vars types
/// (Phase 5 task 10). Names use `rust_ident` - the same name source as the
/// text report printers - so JSON and text carry identical data.
fn json_renderers_c(module: &Module) -> String {
    let variants = codegen_fsm::build_variants(module);
    let mut s = String::new();

    for (kw, arg, list) in [
        ("input", "in", &variants.inputs),
        ("output", "o", &variants.outputs),
    ] {
        let fn_name = if kw == "input" {
            "mc_json_input"
        } else {
            "mc_json_output"
        };
        if list.is_empty() {
            s.push_str(&format!(
                "static void {fn_name}(mc_json_buf_t* b, const {kw}_t* {arg}) {{ (void){arg}; mjb_str(b, \"{{\\\"variant\\\":\\\"?\\\",\\\"fields\\\":{{}}}}\"); }}\n"
            ));
            continue;
        }
        let arms: Vec<String> = list
            .iter()
            .map(|vi| {
                let kind = format!("{}_{}", kw.to_uppercase(), vi.unique_name.to_uppercase());
                if vi.fields.is_empty() {
                    format!(
                        "        case {kind}: mjb_str(b, \"\\\"{}\\\",\\\"fields\\\":{{}}\"); break;",
                        crate::backends::rust::rust_ident(&vi.unique_name)
                    )
                } else {
                    let member = format!("{kw}_{}", vi.unique_name.to_lowercase());
                    let parts: Vec<String> = vi
                        .fields
                        .iter()
                        .enumerate()
                        .map(|(i, (fname, fty))| {
                            let sep = if i == 0 { "" } else { "," };
                            let expr = format!("{arg}->payload.{member}.{cf}", cf = c_ident(fname));
                            format!(
                                "mjb_str(b, \"{}\\\"{}\\\":\"); {}",
                                sep,
                                crate::backends::rust::rust_ident(fname),
                                json_value_c(fty, &expr)
                            )
                        })
                        .collect();
                    format!(
                        "        case {kind}: mjb_str(b, \"\\\"{}\\\",\\\"fields\\\":{{\"); {} mjb_char(b, '}}'); break;",
                        crate::backends::rust::rust_ident(&vi.unique_name),
                        parts.join(" ")
                    )
                }
            })
            .collect();
        s.push_str(&format!(
            "static void {fn_name}(mc_json_buf_t* b, const {kw}_t* {arg}) {{\n    mjb_str(b, \"{{\\\"variant\\\":\");\n    switch ({arg}->kind) {{\n{arms}\n        default: mjb_str(b, \"\\\"?\\\"\"); break;\n    }}\n    mjb_char(b, '}}');\n}}\n",
            arms = arms.join("\n")
        ));
    }

    // mc_json_vars: declaration order.
    if module.vars.is_empty() {
        s.push_str(
            "static void mc_json_vars(mc_json_buf_t* b, const vars_t* v) { (void)v; mjb_str(b, \"{}\"); }\n",
        );
    } else {
        let parts: Vec<String> = module
            .vars
            .iter()
            .enumerate()
            .map(|(i, v)| {
                let sep = if i == 0 {
                    String::new()
                } else {
                    "mjb_char(b, ','); ".to_string()
                };
                format!(
                    "{sep}mjb_str(b, \"\\\"{}\\\":\"); {}",
                    crate::backends::rust::rust_ident(&v.name),
                    json_value_c(&v.ty, &format!("v->{}", c_ident(&v.name)))
                )
            })
            .collect();
        s.push_str(&format!(
            "static void mc_json_vars(mc_json_buf_t* b, const vars_t* v) {{\n    mjb_char(b, '{{'); {} mjb_char(b, '}}');\n}}\n",
            parts.join(" ")
        ));
    }
    s
}

/// C statement appending the JSON rendering of `expr` (type `ty`): bool ->
/// true/false, bit<N> -> unsigned long long decimal (the same reduction the
/// text printers use), enum -> variant name via the shared lookup function.
fn json_value_c(ty: &Type, expr: &str) -> String {
    match ty {
        Type::Bool => format!("mjb_str(b, {expr} ? \"true\" : \"false\");"),
        Type::Bit { .. } => format!("mjb_int(b, (unsigned long long){expr});"),
        Type::Enum { name } => format!("mjb_escape(b, {}_name_c({expr}));", c_ident(name)),
    }
}

/// Re-index the skeleton leaves in first-appearance order and render the
/// boolean expression over `t[0..L)` (per-leaf satisfaction flags) - C mirror
/// of rust_mc's `flatten_skeleton`.
fn flatten_skeleton_c(
    sk: &crate::mc_plan::Skeleton,
    leaves: &mut Vec<crate::mc_plan::SkeletonLeaf>,
) -> String {
    match sk {
        crate::mc_plan::Skeleton::And(l, r) => format!(
            "({}) && ({})",
            flatten_skeleton_c(l, leaves),
            flatten_skeleton_c(r, leaves)
        ),
        crate::mc_plan::Skeleton::Or(l, r) => format!(
            "({}) || ({})",
            flatten_skeleton_c(l, leaves),
            flatten_skeleton_c(r, leaves)
        ),
        crate::mc_plan::Skeleton::Implies(l, r) => format!(
            "(!({}) || {})",
            flatten_skeleton_c(l, leaves),
            flatten_skeleton_c(r, leaves)
        ),
        crate::mc_plan::Skeleton::Not(x) => format!("!({})", flatten_skeleton_c(x, leaves)),
        crate::mc_plan::Skeleton::Leaf(leaf) => {
            let i = leaves.len();
            leaves.push(leaf.clone());
            format!("t[{}]", i)
        }
    }
}

/// Escape a Rust string as a C string literal.
fn c_string_literal(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\x{:02x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Value expression for one variable slot in `config_bytes` (C).
fn var_value_c(slot: &crate::mc_plan::VarSlot, module: &Module) -> String {
    let ty = module
        .vars
        .iter()
        .find(|v| v.name == slot.name)
        .map(|v| v.ty.clone())
        .unwrap_or(Type::Bool);
    match ty {
        Type::Bool => format!("vars->{} ? 1ULL : 0ULL", c_ident(&slot.name)),
        Type::Bit { .. } => format!("(uint64_t)(vars->{})", c_ident(&slot.name)),
        Type::Enum { .. } => format!("(uint64_t)(vars->{})", c_ident(&slot.name)),
    }
}
/// The exact-mode BFS engine (generated verbatim into every C MC binary).
const EXACT_ENGINE_C: &str = r#"
typedef struct { state_t state; vars_t vars; prop_monitors_t prop; assume_monitors_t assume; } mc_node_t;

static rfs_policy_t mc_stop_handler(void* user_data, const char* label) { (void)user_data; (void)label; return RFS_STOP; }

/// Apply one enabled transition (or on_error fallback) to a configuration and
/// step the monitors at the correct sample index (depth + 1).
/// Returns: 0 = success, 1 = pruned by an active assumption, 2 = property violation.
static int mc_step_successor(const mc_node_t* node, int depth, const input_t* in, const mc_effect_t* eff,
                             const char** active_assumes, int n_active,
                             mc_node_t* out_node, rfs_property_violation_t* pv) {
    *out_node = *node;
    rfs_sample_t sample;
    sample.input = in;
    sample.outputs = eff->outputs;
    sample.n_outputs = eff->n_outputs;
    sample.from_state = node->state;
    sample.to_state = eff->next_state;
    sample.vars = eff->next_vars;
    int idx = depth + 1;
    const char* al = assume_monitors_step(&out_node->assume, &sample, idx, active_assumes, n_active);
    if (al) return 1;
    rfs_property_violation_t pv2;
    if (prop_monitors_step(&out_node->prop, &sample, idx, &pv2)) { *pv = pv2; return 2; }
    out_node->state = eff->next_state;
    out_node->vars = eff->next_vars;
    return 0;
}

// Parallel level expansion (task 10B): one successor outcome per evaluated
// (input, effect) pair - computed in phase A (possibly on worker threads),
// committed sequentially in canonical order in phase B. kind: 0 unexpected,
// 1 pruned, 2 violation, 3 success.
typedef struct {
    int kind;
    int iidx;
    int tid;
    int step;
    char label[64];
    mc_node_t node; // successor (kind 3)
} mc_outcome_t;

typedef struct {
    mc_outcome_t* v;
    int n, cap;
} mc_outcomes_t;

static void mc_outcomes_push(mc_outcomes_t* o, const mc_outcome_t* x) {
    if (o->n == o->cap) {
        int ncap = o->cap ? o->cap * 2 : 8;
        mc_outcome_t* nv = (mc_outcome_t*)realloc(o->v, (size_t)ncap * sizeof(mc_outcome_t));
        if (!nv) { fprintf(stderr, "out of memory\n"); exit(2); }
        o->v = nv;
        o->cap = ncap;
    }
    o->v[o->n++] = *x;
}

/// Expands configs [lo, hi) of the current level into per-config outcome
/// lists (indexed by cid - level_start). Pure with respect to the shared
/// engine state, so workers may run it in parallel on disjoint ranges.
static void mc_expand_range(
    const mc_node_t* nodes, const int* depth_arr, input_t* inputs, int n_inputs,
    int nondet, const char** active, int n_active,
    int level_start, int lo, int hi,
    mc_outcomes_t* outs, atomic_int* abort_flag, int fast) {
    for (int cid = lo; cid < hi; cid++) {
        if (fast && atomic_load_explicit(abort_flag, memory_order_relaxed)) continue;
        mc_node_t node = nodes[cid];
        int d = depth_arr[cid];
        int found_violation = 0;
        for (int iidx = 0; iidx < n_inputs; iidx++) {
            input_t in = inputs[iidx];
            mc_effect_t effs[MC_MAX_EFFECTS];
            int n_eff = mc_step_all(node.state, &node.vars, in, effs);
            if (n_eff == 0) {
                mc_outcome_t x; memset(&x, 0, sizeof(x));
                x.kind = 0; x.iidx = iidx;
                mc_outcomes_push(&outs[cid - level_start], &x);
                continue;
            }
            int n_sel = nondet ? n_eff : 1;
            for (int e = 0; e < n_sel; e++) {
                const mc_effect_t* eff = &effs[e];
                mc_node_t succ;
                rfs_property_violation_t pv;
                int r = mc_step_successor(&node, d, &in, eff, active, n_active, &succ, &pv);
                mc_outcome_t x; memset(&x, 0, sizeof(x));
                x.iidx = iidx; x.tid = eff->transition_id;
                if (r == 1) {
                    x.kind = 1;
                } else if (r == 2) {
                    x.kind = 2; x.step = pv.step;
                    strncpy(x.label, pv.label, sizeof(x.label) - 1);
                    found_violation = 1;
                } else {
                    x.kind = 3; x.node = succ;
                }
                mc_outcomes_push(&outs[cid - level_start], &x);
            }
        }
        if (fast && found_violation) atomic_store_explicit(abort_flag, 1, memory_order_relaxed);
    }
}

typedef struct {
    const mc_node_t* nodes;
    const int* depth_arr;
    input_t* inputs;
    int n_inputs;
    int nondet;
    const char** active;
    int n_active;
    int level_start, lo, hi;
    mc_outcomes_t* outs;
    atomic_int* abort_flag;
    int fast;
} mc_worker_arg_t;

static void* mc_expand_worker(void* p) {
    mc_worker_arg_t* a = (mc_worker_arg_t*)p;
    mc_expand_range(a->nodes, a->depth_arr, a->inputs, a->n_inputs, a->nondet,
                    a->active, a->n_active, a->level_start, a->lo, a->hi,
                    a->outs, a->abort_flag, a->fast);
    return 0;
}

// Visited set: sorted vector of config byte strings + binary search, with a
// parallel array mapping each entry to its node index (needed by the liveness
// graph edge recording). The behavior must not depend on this container
// (D-P4-2); task 10 may switch it.
static int mc_visited_find(const uint8_t* visited, const int* visited_idx, int n_visited, const uint8_t* key) {
    int lo = 0, hi = n_visited;
    while (lo < hi) {
        int mid = (lo + hi) / 2;
        int c = memcmp(visited + (size_t)mid * CONFIG_TOTAL_BYTES, key, CONFIG_TOTAL_BYTES);
        if (c == 0) return visited_idx[mid];
        if (c < 0) lo = mid + 1; else hi = mid;
    }
    return -1;
}
static void mc_visited_insert(uint8_t* visited, int* visited_idx, int* n_visited, const uint8_t* key, int node_idx) {
    int lo = 0, hi = *n_visited;
    while (lo < hi) {
        int mid = (lo + hi) / 2;
        int c = memcmp(visited + (size_t)mid * CONFIG_TOTAL_BYTES, key, CONFIG_TOTAL_BYTES);
        if (c == 0) return;
        if (c < 0) lo = mid + 1; else hi = mid;
    }
    memmove(visited + (size_t)(lo + 1) * CONFIG_TOTAL_BYTES, visited + (size_t)lo * CONFIG_TOTAL_BYTES, (size_t)(*n_visited - lo) * CONFIG_TOTAL_BYTES);
    memmove(visited_idx + lo + 1, visited_idx + lo, (size_t)(*n_visited - lo) * sizeof(int));
    memcpy(visited + (size_t)lo * CONFIG_TOTAL_BYTES, key, CONFIG_TOTAL_BYTES);
    visited_idx[lo] = node_idx;
    (*n_visited)++;
}

/// Fill the first depth_arr[cid] entries of the path arrays with the steps
/// from the root to configuration `cid`: (child config id, input index, transition id).
static void mc_fill_path(const int* parent_id, const int* parent_input, const int* parent_trans, const int* depth_arr, int cid, int* p_cid, int* p_input, int* p_trans) {
    int n = depth_arr[cid];
    int k = n;
    int c = cid;
    while (c > 0) {
        k--;
        p_cid[k] = c;
        p_input[k] = parent_input[c];
        p_trans[k] = parent_trans[c];
        c = parent_id[c];
    }
}

/// Replay a path on a fresh Fsm to rebuild the Phase 3 trace format.
static void mc_replay(const int* p_cid, const int* p_input, const int* p_trans, int n_steps, int nondet, const char** active, int n_active, const input_t* inputs, fsm_t* f) {
    (void)p_cid;
    fsm_new(f);
    f->active_assumes = active;
    f->n_active_assumes = n_active;
    for (int i = 0; i < n_steps; i++) {
        input_t in = inputs[p_input[i]];
        mc_effect_t effs[MC_MAX_EFFECTS];
        int n_eff = mc_step_all(f->state, &f->vars, in, effs);
        const mc_effect_t* eff = &effs[0];
        if (p_trans[i] == -1) {
            for (int e = 0; e < n_eff; e++) if (effs[e].transition_id == -1) { eff = &effs[e]; break; }
        } else if (nondet) {
            for (int e = 0; e < n_eff; e++) if (effs[e].transition_id == p_trans[i]) { eff = &effs[e]; break; }
        }
        rfs_commit(f, &in, eff->outputs, eff->n_outputs, eff->next_state, &eff->next_vars, mc_stop_handler, 0);
    }
}

/// Print the property-specific counterexample (exact mode; same fields as the
/// Rust model checker) without exiting.
static void mc_report_property(const fsm_t* f) {
    printf("property violation: %s\n", f->prop_label);
    if (f->prop_trigger_step >= 0) {
        printf("  trigger step: %d\n", f->prop_trigger_step);
        vars_t tv = trace_vars(&f->trace, f->prop_trigger_step);
        printf("  vars at trigger step: "); print_vars_debug(&tv); printf("\n");
    }
    printf("  violation step: %d\n", f->prop_step);
    vars_t vv = trace_vars(&f->trace, f->prop_step);
    printf("  vars at violation step: "); print_vars_debug(&vv); printf("\n");
    print_trace(&f->trace);
}
"#;
// The liveness engine (Phase 4 task 7; generated verbatim into every C MC
// binary). Mirror of the Rust LIV_ENGINE: per-obligation bad-set analysis on
// the explored graph via per-leaf progress automata + product limit vectors
// (D-P4-10/11). Behavior must not depend on the containers below (D-P4-2);
// witness selection is canonical - properties in declaration order, limit
// vectors ascending, smallest cyclic config id, DFS over edges in discovery
// order - so Rust and C produce identical status + counterexample fields.
const LIV_ENGINE_C: &str = r#"
typedef struct { int input_idx; int trans_id; int target; } mc_edge_t;
typedef struct { int v; int pos; } liv_stay_edge_t;

typedef struct {
    const mc_node_t* nodes;
    int n_nodes;
    const mc_edge_t* edges;        // flat edge storage (per-config discovery order)
    const int* adj_start;          // per-config first-edge offset, n_nodes+1 entries
    const unsigned char* complete; // per-config fully expanded
    const input_t* inputs;
    int n_inputs;
    int nondet;
} liv_graph_t;

typedef int (*liv_eval_fn)(const rfs_sample_t* s);

typedef struct { int kind; liv_eval_fn eval_a; liv_eval_fn eval_b; } liv_leaf_t;
// kind: 0 safety, 1 eventually, 2 until (eval_b = NULL unless until)

typedef struct {
    const char* label;
    const char* formula;
    const liv_leaf_t* leaves;
    int n_leaves;
    int (*skeleton)(const unsigned char* t);
} liv_property_t;

typedef struct {
    int* lasso_src;     // per edge: source config id
    int* lasso_input;   // per edge: input index
    int* lasso_trans;   // per edge: transition id (-1 = on_error)
    int n_lasso;
    int cycle_start;    // first edge index of the cycle (= prefix length)
    unsigned char* falsified; // per leaf: 1 = falsified on the witness path
} liv_witness_t;

static void* liv_malloc(size_t n) {
    void* p = malloc(n ? n : 1);
    if (!p) { fprintf(stderr, "out of memory\n"); exit(2); }
    return p;
}
static void* liv_calloc(size_t n, size_t sz) {
    void* p = calloc(n ? n : 1, sz ? sz : 1);
    if (!p) { fprintf(stderr, "out of memory\n"); exit(2); }
    return p;
}

static inline int liv_bit_at(const unsigned char* bits, int i) { return (bits[i >> 3] >> (i & 7)) & 1; }
static inline void liv_bit_set(unsigned char* bits, int i) { bits[i >> 3] |= (unsigned char)(1u << (i & 7)); }

/// Label bits of one step: bit 2k = leaf k atom A (S / P / P), bit 2k+1 =
/// leaf k atom B (the `until` right operand). Cached per edge position.
static uint32_t liv_bits_at(const liv_graph_t* g, uint32_t* cache, unsigned char* done, int c, int pos, const liv_property_t* prop) {
    size_t ei = (size_t)g->adj_start[c] + (size_t)pos;
    if (done[ei]) return cache[ei];
    const mc_edge_t* e = &g->edges[ei];
    const mc_node_t* node = &g->nodes[c];
    const input_t* in = &g->inputs[e->input_idx];
    mc_effect_t effs[MC_MAX_EFFECTS];
    int n_eff = mc_step_all(node->state, &node->vars, *in, effs);
    const mc_effect_t* eff = (n_eff > 0) ? &effs[0] : 0;
    if (eff && e->trans_id != -1 && g->nondet) {
        for (int i = 0; i < n_eff; i++) if (effs[i].transition_id == e->trans_id) { eff = &effs[i]; break; }
    }
    rfs_sample_t sample;
    sample.input = in;
    sample.outputs = eff ? eff->outputs : 0;
    sample.n_outputs = eff ? eff->n_outputs : 0;
    sample.from_state = node->state;
    sample.to_state = eff ? eff->next_state : node->state;
    sample.vars = eff ? eff->next_vars : node->vars;
    uint32_t bits = 0;
    for (int k = 0; k < prop->n_leaves; k++) {
        const liv_leaf_t* leaf = &prop->leaves[k];
        if (leaf->eval_a(&sample)) bits |= 1u << (2 * k);
        if (leaf->eval_b && leaf->eval_b(&sample)) bits |= 1u << (2 * k + 1);
    }
    cache[ei] = bits;
    done[ei] = 1;
    return bits;
}

/// Mixed-radix transition of the per-leaf progress automata on label bits.
/// Leaf states: safety 0=ok/1=bad; eventually 0=wait/1=done;
/// until 0=ok/1=sat/2=bad. All transitions are one-way, so every state
/// sequence on an infinite path is eventually constant (the limit vector).
static int liv_delta(int q, int n, const int* places, const int* radices, const unsigned char* kinds, uint32_t bits) {
    int out = 0;
    for (int k = 0; k < n; k++) {
        int r = radices[k];
        int s = (q / places[k]) % r;
        int a = ((bits >> (2 * k)) & 1u) != 0;
        int b = ((bits >> (2 * k + 1)) & 1u) != 0;
        int ns;
        if (kinds[k] == 0) {
            ns = (s == 1 || !a) ? 1 : 0;
        } else if (kinds[k] == 1) {
            ns = (s == 1 || a) ? 1 : 0;
        } else {
            if (s == 2) ns = 2;
            else if (s == 1) ns = 1;
            else ns = b ? 1 : (!a ? 2 : 0);
        }
        out = out * r + ns;
    }
    return out;
}

/// A leaf is satisfied on an infinite path iff its limit state is in the good set.
static int liv_good(int kind, int s) {
    if (kind == 0) return s == 0; // safety: never entered bad
    if (kind == 1) return s == 1; // eventually: done reached
    return s != 2;                // until: ok (P-forever branch) or sat
}

/// Sample at witness position `pos`: 0 = initial configuration, p > 0 = the
/// effect of lasso edge p-1.
static void liv_sample_at(const liv_graph_t* g, const liv_witness_t* w, int pos, rfs_sample_t* out) {
    if (pos == 0) {
        out->input = 0;
        out->outputs = 0;
        out->n_outputs = 0;
        out->from_state = g->nodes[0].state;
        out->to_state = g->nodes[0].state;
        out->vars = g->nodes[0].vars;
        return;
    }
    int c = w->lasso_src[pos - 1];
    const mc_node_t* node = &g->nodes[c];
    const input_t* in = &g->inputs[w->lasso_input[pos - 1]];
    int tid = w->lasso_trans[pos - 1];
    mc_effect_t effs[MC_MAX_EFFECTS];
    int n_eff = mc_step_all(node->state, &node->vars, *in, effs);
    const mc_effect_t* eff = (n_eff > 0) ? &effs[0] : 0;
    if (eff && tid != -1 && g->nondet) {
        for (int i = 0; i < n_eff; i++) if (effs[i].transition_id == tid) { eff = &effs[i]; break; }
    }
    out->input = in;
    out->outputs = eff ? eff->outputs : 0;
    out->n_outputs = eff ? eff->n_outputs : 0;
    out->from_state = node->state;
    out->to_state = eff ? eff->next_state : node->state;
    out->vars = eff ? eff->next_vars : node->vars;
}

/// Reconstruct the lasso: a product path from (start_q, 0) to (q, cstar)
/// plus a stay-edge cycle through cstar. Returns 1 on success.
static int liv_build_lasso(
    const liv_graph_t* g, const liv_property_t* prop, uint32_t* cache, unsigned char* done,
    int n_leaves, const int* places, const int* radices, const unsigned char* kinds,
    uint32_t init_bits, int q, int cstar, const unsigned char* t,
    const int* stay_start, const liv_stay_edge_t* stay_edges, const int* comp,
    liv_witness_t* out)
{
    int n_nodes = g->n_nodes;
    int start_q = liv_delta(0, n_leaves, places, radices, kinds, init_bits);
    size_t n_words = ((size_t)n_nodes + 7) / 8;
    int qcount = 1;
    for (int k = 0; k < n_leaves; k++) qcount *= radices[k];

    // Prefix: DFS in the product from (start_q, 0) to (q, cstar).
    unsigned char* dfs_bits = (unsigned char*)liv_calloc((size_t)qcount * n_words, 1);
    size_t pth_cap = 64, pth_n = 0;
    int* pth_c = (int*)liv_malloc(pth_cap * sizeof(int));
    int* pth_pos = (int*)liv_malloc(pth_cap * sizeof(int));
    size_t stk_cap = 64, stk_n = 0;
    int* stk_q = (int*)liv_malloc(stk_cap * sizeof(int));
    int* stk_c = (int*)liv_malloc(stk_cap * sizeof(int));
    int* stk_p = (int*)liv_malloc(stk_cap * sizeof(int));
    liv_bit_set(dfs_bits + (size_t)start_q * n_words, 0);
    stk_q[0] = start_q; stk_c[0] = 0; stk_p[0] = 0; stk_n = 1;
    for (;;) {
        if (stk_q[stk_n - 1] == q && stk_c[stk_n - 1] == cstar) break;
        int cq = stk_q[stk_n - 1];
        int cc = stk_c[stk_n - 1];
        int len = g->adj_start[cc + 1] - g->adj_start[cc];
        int descended = 0;
        int p = stk_p[stk_n - 1];
        while (p < len) {
            int v = g->edges[g->adj_start[cc] + p].target;
            uint32_t bits = liv_bits_at(g, cache, done, cc, p, prop);
            int q2 = liv_delta(cq, n_leaves, places, radices, kinds, bits);
            unsigned char* dq = dfs_bits + (size_t)q2 * n_words;
            if (!liv_bit_at(dq, v)) {
                liv_bit_set(dq, v);
                stk_p[stk_n - 1] = p + 1;
                if (pth_n == pth_cap) {
                    pth_cap *= 2;
                    pth_c = realloc(pth_c, pth_cap * sizeof(int));
                    pth_pos = realloc(pth_pos, pth_cap * sizeof(int));
                    if (!pth_c || !pth_pos) { fprintf(stderr, "out of memory\n"); exit(2); }
                }
                pth_c[pth_n] = cc; pth_pos[pth_n] = p; pth_n++;
                if (stk_n == stk_cap) {
                    stk_cap *= 2;
                    stk_q = realloc(stk_q, stk_cap * sizeof(int));
                    stk_c = realloc(stk_c, stk_cap * sizeof(int));
                    stk_p = realloc(stk_p, stk_cap * sizeof(int));
                    if (!stk_q || !stk_c || !stk_p) { fprintf(stderr, "out of memory\n"); exit(2); }
                }
                stk_q[stk_n] = q2; stk_c[stk_n] = v; stk_p[stk_n] = 0; stk_n++;
                descended = 1;
                break;
            }
            p++;
        }
        if (descended) continue;
        stk_n--;
        if (stk_n > 0) pth_n--;
        if (stk_n == 0) {
            // Unreachable: the target is reachable (product BFS proved it).
            free(dfs_bits);
            free(pth_c); free(pth_pos); free(stk_q); free(stk_c); free(stk_p);
            return 0;
        }
    }

    // Cycle: DFS from cstar over stay-successors (inside its SCC) back to cstar.
    unsigned char* on_path = (unsigned char*)liv_calloc((size_t)n_nodes, 1);
    on_path[cstar] = 1;
    size_t cyc_cap = 64, cyc_n = 0;
    int* cyc_c = (int*)liv_malloc(cyc_cap * sizeof(int));
    int* cyc_pos = (int*)liv_malloc(cyc_cap * sizeof(int));
    size_t cstk_cap = 64, cstk_n = 0;
    int* cstk_c = (int*)liv_malloc(cstk_cap * sizeof(int));
    int* cstk_i = (int*)liv_malloc(cstk_cap * sizeof(int));
    cstk_c[0] = cstar; cstk_i[0] = 0; cstk_n = 1;
    for (;;) {
        int cc = cstk_c[cstk_n - 1];
        int succs_len = stay_start[cc + 1] - stay_start[cc];
        int descended = 0;
        int i = cstk_i[cstk_n - 1];
        while (i < succs_len) {
            const liv_stay_edge_t* se = &stay_edges[stay_start[cc] + i];
            if (comp[se->v] != comp[cstar]) { i++; continue; } // outside cstar's SCC
            if (se->v == cstar) {
                if (cyc_n == cyc_cap) {
                    cyc_cap *= 2;
                    cyc_c = realloc(cyc_c, cyc_cap * sizeof(int));
                    cyc_pos = realloc(cyc_pos, cyc_cap * sizeof(int));
                    if (!cyc_c || !cyc_pos) { fprintf(stderr, "out of memory\n"); exit(2); }
                }
                cyc_c[cyc_n] = cc; cyc_pos[cyc_n] = se->pos; cyc_n++;
                // Convert (config, edge pos) steps to (config, input idx,
                // transition id) edges for replay.
                int n_lasso = (int)(pth_n + cyc_n);
                out->lasso_src = (int*)liv_malloc((size_t)n_lasso * sizeof(int));
                out->lasso_input = (int*)liv_malloc((size_t)n_lasso * sizeof(int));
                out->lasso_trans = (int*)liv_malloc((size_t)n_lasso * sizeof(int));
                for (int k2 = 0; k2 < n_lasso; k2++) {
                    int c, pos;
                    if (k2 < (int)pth_n) { c = pth_c[k2]; pos = pth_pos[k2]; }
                    else { c = cyc_c[k2 - (int)pth_n]; pos = cyc_pos[k2 - (int)pth_n]; }
                    const mc_edge_t* e = &g->edges[g->adj_start[c] + pos];
                    out->lasso_src[k2] = c;
                    out->lasso_input[k2] = e->input_idx;
                    out->lasso_trans[k2] = e->trans_id;
                }
                out->n_lasso = n_lasso;
                out->cycle_start = (int)pth_n;
                out->falsified = (unsigned char*)liv_malloc((size_t)n_leaves);
                for (int k2 = 0; k2 < n_leaves; k2++) out->falsified[k2] = !t[k2];
                free(dfs_bits); free(on_path);
                free(pth_c); free(pth_pos); free(stk_q); free(stk_c); free(stk_p);
                free(cyc_c); free(cyc_pos); free(cstk_c); free(cstk_i);
                return 1;
            }
            if (!on_path[se->v]) {
                on_path[se->v] = 1;
                cstk_i[cstk_n - 1] = i + 1;
                if (cyc_n == cyc_cap) {
                    cyc_cap *= 2;
                    cyc_c = realloc(cyc_c, cyc_cap * sizeof(int));
                    cyc_pos = realloc(cyc_pos, cyc_cap * sizeof(int));
                    if (!cyc_c || !cyc_pos) { fprintf(stderr, "out of memory\n"); exit(2); }
                }
                cyc_c[cyc_n] = cc; cyc_pos[cyc_n] = se->pos; cyc_n++;
                if (cstk_n == cstk_cap) {
                    cstk_cap *= 2;
                    cstk_c = realloc(cstk_c, cstk_cap * sizeof(int));
                    cstk_i = realloc(cstk_i, cstk_cap * sizeof(int));
                    if (!cstk_c || !cstk_i) { fprintf(stderr, "out of memory\n"); exit(2); }
                }
                cstk_c[cstk_n] = se->v; cstk_i[cstk_n] = 0; cstk_n++;
                descended = 1;
                break;
            }
            i++;
        }
        if (descended) continue;
        int pc = cstk_c[--cstk_n];
        on_path[pc] = 0;
        if (cstk_n > 0) cyc_n--;
        // cstar lies on a stay cycle inside its SCC, so the search cannot fail.
    }
}

/// Analyze one property on the graph. Returns 1 and fills `out` with a lasso
/// witness iff violated. `cache`/`done` are per-edge label caches (size =
/// total number of explored edges; may be NULL when there are none).
static int liv_analyze(const liv_graph_t* g, const liv_property_t* prop, uint32_t* cache, unsigned char* done, liv_witness_t* out) {
    int n_nodes = g->n_nodes;
    int n_leaves = prop->n_leaves;
    if (n_leaves == 0) return 0;
    int* radices = (int*)liv_malloc((size_t)n_leaves * sizeof(int));
    int* places = (int*)liv_malloc((size_t)n_leaves * sizeof(int));
    unsigned char* kinds = (unsigned char*)liv_malloc((size_t)n_leaves);
    int qcount = 1;
    for (int k = 0; k < n_leaves; k++) {
        kinds[k] = (unsigned char)prop->leaves[k].kind;
        radices[k] = (kinds[k] == 2) ? 3 : 2;
        qcount *= radices[k];
    }
    places[n_leaves - 1] = 1;
    for (int k = n_leaves - 2; k >= 0; k--) places[k] = places[k + 1] * radices[k + 1];

    // Position-0 label (initial configuration sample, as in fsm_new).
    rfs_sample_t s0;
    s0.input = 0;
    s0.outputs = 0;
    s0.n_outputs = 0;
    s0.from_state = g->nodes[0].state;
    s0.to_state = g->nodes[0].state;
    s0.vars = g->nodes[0].vars;
    uint32_t init_bits = 0;
    for (int k = 0; k < n_leaves; k++) {
        if (prop->leaves[k].eval_a(&s0)) init_bits |= 1u << (2 * k);
        if (prop->leaves[k].eval_b && prop->leaves[k].eval_b(&s0)) init_bits |= 1u << (2 * k + 1);
    }

    // Product BFS from (start_q, config 0): reach[q] = configs reachable with
    // per-leaf automaton states encoding q. The worklist order does not affect
    // the reachability fixed point.
    size_t n_words = ((size_t)n_nodes + 7) / 8;
    unsigned char* reach = (unsigned char*)liv_calloc((size_t)qcount * n_words, 1);
    int start_q = liv_delta(0, n_leaves, places, radices, kinds, init_bits);
    size_t wq_cap = 64, wq_head = 0, wq_tail = 0;
    int* wq_q = (int*)liv_malloc(wq_cap * sizeof(int));
    int* wq_c = (int*)liv_malloc(wq_cap * sizeof(int));
    liv_bit_set(reach + (size_t)start_q * n_words, 0);
    wq_q[0] = start_q; wq_c[0] = 0; wq_tail = 1;
    while (wq_head < wq_tail) {
        int q = wq_q[wq_head];
        int c = wq_c[wq_head];
        wq_head++;
        int len = g->adj_start[c + 1] - g->adj_start[c];
        for (int pos = 0; pos < len; pos++) {
            int v = g->edges[g->adj_start[c] + pos].target;
            uint32_t bits = liv_bits_at(g, cache, done, c, pos, prop);
            int q2 = liv_delta(q, n_leaves, places, radices, kinds, bits);
            unsigned char* rq2 = reach + (size_t)q2 * n_words;
            if (!liv_bit_at(rq2, v)) {
                liv_bit_set(rq2, v);
                if (wq_tail == wq_cap) {
                    wq_cap *= 2;
                    wq_q = realloc(wq_q, wq_cap * sizeof(int));
                    wq_c = realloc(wq_c, wq_cap * sizeof(int));
                    if (!wq_q || !wq_c) { fprintf(stderr, "out of memory\n"); exit(2); }
                }
                wq_q[wq_tail] = q2; wq_c[wq_tail] = v; wq_tail++;
            }
        }
    }

    int violated = 0;
    for (int q = 0; q < qcount && !violated; q++) {
        unsigned char* rq = reach + (size_t)q * n_words;
        int any = 0;
        for (size_t w = 0; w < n_words && !any; w++) any = (rq[w] != 0);
        if (!any) continue;

        // Stay-edge subgraph on R(q): edges whose label keeps the limit vector.
        // In nondet mode a bad cycle must be STRONGLY FAIR (D-P4-11): every
        // enabled successor of a cycle state is taken on the cycle, i.e. each
        // cycle state has exactly one explored successor (and was fully
        // expanded when truncated).
        int* stay_count = (int*)liv_calloc((size_t)n_nodes, sizeof(int));
        unsigned char* self_loop = (unsigned char*)liv_calloc((size_t)n_nodes, 1);
        for (int c = 0; c < n_nodes; c++) {
            if (!liv_bit_at(rq, c)) continue;
            int len = g->adj_start[c + 1] - g->adj_start[c];
            if (g->nondet && !(len == 1 && g->complete[c])) continue;
            for (int pos = 0; pos < len; pos++) {
                int v = g->edges[g->adj_start[c] + pos].target;
                if (!liv_bit_at(rq, v)) continue;
                uint32_t bits = liv_bits_at(g, cache, done, c, pos, prop);
                if (liv_delta(q, n_leaves, places, radices, kinds, bits) != q) continue;
                stay_count[c]++;
                if (v == c) self_loop[c] = 1;
            }
        }
        int* stay_start = (int*)liv_malloc(((size_t)n_nodes + 1) * sizeof(int));
        int total_stay = 0;
        for (int c = 0; c < n_nodes; c++) { stay_start[c] = total_stay; total_stay += stay_count[c]; }
        stay_start[n_nodes] = total_stay;
        liv_stay_edge_t* stay_edges = (liv_stay_edge_t*)liv_malloc((size_t)(total_stay ? total_stay : 1) * sizeof(liv_stay_edge_t));
        int* scur = (int*)liv_calloc((size_t)n_nodes, sizeof(int));
        for (int c = 0; c < n_nodes; c++) {
            if (!liv_bit_at(rq, c)) continue;
            int len = g->adj_start[c + 1] - g->adj_start[c];
            if (g->nondet && !(len == 1 && g->complete[c])) continue;
            for (int pos = 0; pos < len; pos++) {
                int v = g->edges[g->adj_start[c] + pos].target;
                if (!liv_bit_at(rq, v)) continue;
                uint32_t bits = liv_bits_at(g, cache, done, c, pos, prop);
                if (liv_delta(q, n_leaves, places, radices, kinds, bits) != q) continue;
                stay_edges[stay_start[c] + scur[c]].v = v;
                stay_edges[stay_start[c] + scur[c]].pos = pos;
                scur[c]++;
            }
        }
        free(scur);

        // SCCs of the stay graph (Kosaraju, iterative). The SCC partition is
        // independent of the DFS start order, so ascending config ids here
        // reproduce the Rust engine's results.
        int* rev_count = (int*)liv_calloc((size_t)n_nodes, sizeof(int));
        for (int c = 0; c < n_nodes; c++)
            for (int i = 0; i < stay_count[c]; i++) rev_count[stay_edges[stay_start[c] + i].v]++;
        int* rev_start = (int*)liv_malloc(((size_t)n_nodes + 1) * sizeof(int));
        int total_rev = 0;
        for (int c = 0; c < n_nodes; c++) { rev_start[c] = total_rev; total_rev += rev_count[c]; }
        rev_start[n_nodes] = total_rev;
        int* rev_edges = (int*)liv_malloc((size_t)(total_rev ? total_rev : 1) * sizeof(int));
        int* rc = (int*)liv_calloc((size_t)n_nodes, sizeof(int));
        for (int c = 0; c < n_nodes; c++)
            for (int i = 0; i < stay_count[c]; i++) {
                int v = stay_edges[stay_start[c] + i].v;
                rev_edges[rev_start[v] + rc[v]++] = c;
            }
        free(rc);

        unsigned char* seen = (unsigned char*)liv_calloc((size_t)n_nodes, 1);
        int* fin_order = (int*)liv_malloc((size_t)(n_nodes ? n_nodes : 1) * sizeof(int));
        int n_fin = 0;
        for (int start = 0; start < n_nodes; start++) {
            if (!liv_bit_at(rq, start) || seen[start]) continue;
            size_t st_cap = 16, st_n = 0;
            int* st_c = (int*)liv_malloc(st_cap * sizeof(int));
            int* st_i = (int*)liv_malloc(st_cap * sizeof(int));
            seen[start] = 1;
            st_c[0] = start; st_i[0] = 0; st_n = 1;
            while (st_n > 0) {
                int c = st_c[st_n - 1];
                int i = st_i[st_n - 1];
                int succs_len = stay_count[c];
                int descended = 0;
                while (i < succs_len) {
                    int v = stay_edges[stay_start[c] + i].v;
                    if (!seen[v]) {
                        seen[v] = 1;
                        st_i[st_n - 1] = i + 1;
                        if (st_n == st_cap) {
                            st_cap *= 2;
                            st_c = realloc(st_c, st_cap * sizeof(int));
                            st_i = realloc(st_i, st_cap * sizeof(int));
                            if (!st_c || !st_i) { fprintf(stderr, "out of memory\n"); exit(2); }
                        }
                        st_c[st_n] = v; st_i[st_n] = 0; st_n++;
                        descended = 1;
                        break;
                    }
                    i++;
                }
                if (descended) continue;
                st_n--;
                fin_order[n_fin++] = c;
            }
            free(st_c); free(st_i);
        }

        int* comp = (int*)liv_malloc((size_t)(n_nodes ? n_nodes : 1) * sizeof(int));
        for (int c = 0; c < n_nodes; c++) comp[c] = -1;
        int* comp_size = (int*)liv_calloc((size_t)(n_nodes ? n_nodes : 1), sizeof(int));
        int n_comp = 0;
        for (int i = n_fin - 1; i >= 0; i--) {
            int c = fin_order[i];
            if (comp[c] >= 0) continue;
            int cid = n_comp++;
            comp_size[cid] = 0;
            size_t bq_cap = 16, bq_head = 0, bq_tail = 0;
            int* bq = (int*)liv_malloc(bq_cap * sizeof(int));
            comp[c] = cid; comp_size[cid]++;
            bq[bq_tail++] = c;
            while (bq_head < bq_tail) {
                int u = bq[bq_head++];
                for (int r = rev_start[u]; r < rev_start[u + 1]; r++) {
                    int wnode = rev_edges[r];
                    if (comp[wnode] >= 0) continue;
                    comp[wnode] = cid; comp_size[cid]++;
                    if (bq_tail == bq_cap) {
                        bq_cap *= 2;
                        bq = realloc(bq, bq_cap * sizeof(int));
                        if (!bq) { fprintf(stderr, "out of memory\n"); exit(2); }
                    }
                    bq[bq_tail++] = wnode;
                }
            }
            free(bq);
        }
        (void)n_comp;

        // Cyclic configs: SCC size > 1 or a stay self-loop. The smallest id is
        // the canonical cycle anchor (parity with the Rust engine).
        int cstar = -1;
        for (int c = 0; c < n_nodes; c++) {
            if (!liv_bit_at(rq, c)) continue;
            if (comp_size[comp[c]] > 1 || self_loop[c]) { cstar = c; break; }
        }

        int ok = 0;
        if (cstar >= 0) {
            // The limit vector of any path looping inside this SCC is exactly q.
            unsigned char* t = (unsigned char*)liv_malloc((size_t)n_leaves);
            for (int k = 0; k < n_leaves; k++) t[k] = (unsigned char)liv_good(kinds[k], (q / places[k]) % radices[k]);
            if (!prop->skeleton(t)) {
                ok = liv_build_lasso(g, prop, cache, done, n_leaves, places, radices, kinds, init_bits, q, cstar, t, stay_start, stay_edges, comp, out);
            }
            free(t);
        }

        free(stay_count); free(self_loop); free(stay_start); free(stay_edges);
        free(rev_count); free(rev_start); free(rev_edges);
        free(seen); free(fin_order); free(comp); free(comp_size);
        if (ok) violated = 1;
    }

    free(wq_q); free(wq_c); free(reach);
    free(radices); free(places); free(kinds);
    return violated;
}

/// Human-facing explanation of which leaves are falsified on the witness.
static void liv_note(const liv_graph_t* g, const liv_property_t* prop, const liv_witness_t* w, char* buf, size_t buf_size) {
    size_t off = 0;
    int first_part = 1;
    for (int k = 0; k < prop->n_leaves; k++) {
        if (!w->falsified[k]) continue;
        const liv_leaf_t* leaf = &prop->leaves[k];
        char part[160];
        if (leaf->kind == 0) {
            // bad is absorbing and constant on the cycle, so the first
            // falsifying position lies inside prefix + one cycle iteration.
            int p = -1;
            for (int pos = 0; pos <= w->n_lasso && p < 0; pos++) {
                rfs_sample_t s;
                liv_sample_at(g, w, pos, &s);
                if (!leaf->eval_a(&s)) p = pos;
            }
            if (p < 0) p = w->n_lasso;
            snprintf(part, sizeof(part), "safety subformula false at step %d", p);
        } else if (leaf->kind == 1) {
            snprintf(part, sizeof(part), "eventually body never holds on the path");
        } else {
            int seen_q = 0;
            int p = -1;
            for (int pos = 0; pos <= w->n_lasso && p < 0; pos++) {
                rfs_sample_t s;
                liv_sample_at(g, w, pos, &s);
                int a = leaf->eval_a(&s) ? 1 : 0;
                int b = (leaf->eval_b && leaf->eval_b(&s)) ? 1 : 0;
                if (!a && !(seen_q || b)) p = pos;
                seen_q |= b;
            }
            if (p < 0) p = w->n_lasso;
            snprintf(part, sizeof(part), "until left fails at step %d without right holding", p);
        }
        if (!first_part && off + 2 < buf_size) {
            off += (size_t)snprintf(buf + off, buf_size - off, "; ");
        }
        first_part = 0;
        if (off < buf_size) {
            off += (size_t)snprintf(buf + off, buf_size - off, "%s", part);
        }
    }
    if (buf_size > 0 && off >= buf_size) off = buf_size - 1;
    buf[off] = 0;
}
"#;
/// Embedded JSON writer + canonical report document builders (Phase 5
/// task 10). A port of the toolchain's `src/json_rw.rs` compact writer
/// (same escaping contract), appending to a growing buffer. Integers are
/// unsigned long long: every value in this document is non-negative and
/// bit<64> fields reach 2^64-1. Key ordering is the byte-parity contract
/// (see `codegen_mc.rs` in the toolchain). The generated binary stays
/// dependency-free by construction.
const JSON_WRITER_C: &str = r#"
typedef struct { char* data; size_t len; size_t cap; } mc_json_buf_t;

static void mjb_reserve(mc_json_buf_t* b, size_t extra) {
    if (b->len + extra + 1 > b->cap) {
        size_t ncap = b->cap ? b->cap : 64;
        while (ncap < b->len + extra + 1) ncap *= 2;
        char* nd = (char*)realloc(b->data, ncap);
        if (!nd) { fprintf(stderr, "out of memory\n"); exit(2); }
        b->data = nd;
        b->cap = ncap;
    }
}
static void mjb_push(mc_json_buf_t* b, const char* s, size_t n) {
    mjb_reserve(b, n);
    memcpy(b->data + b->len, s, n);
    b->len += n;
}
static void mjb_char(mc_json_buf_t* b, char c) { mjb_push(b, &c, 1); }
static void mjb_str(mc_json_buf_t* b, const char* s) { mjb_push(b, s, strlen(s)); }

/// Escape a string exactly per the toolchain writer's contract: `"` `\\`
/// `\\b` `\\t` `\\n` `\\f` `\\r`; other control chars U+0000..=U+001F as
/// \\u00XX (lowercase hex); everything else raw bytes (UTF-8 passes through).
static void mjb_escape(mc_json_buf_t* b, const char* s) {
    mjb_char(b, '"');
    for (const unsigned char* p = (const unsigned char*)s; *p; p++) {
        switch (*p) {
            case '"': mjb_str(b, "\\\""); break;
            case '\\': mjb_str(b, "\\\\"); break;
            case 0x08: mjb_str(b, "\\b"); break;
            case 0x09: mjb_str(b, "\\t"); break;
            case 0x0A: mjb_str(b, "\\n"); break;
            case 0x0C: mjb_str(b, "\\f"); break;
            case 0x0D: mjb_str(b, "\\r"); break;
            default:
                if (*p < 0x20) { char tmp[8]; snprintf(tmp, sizeof(tmp), "\\u%04x", (unsigned)*p); mjb_str(b, tmp); }
                else mjb_char(b, (char)*p);
        }
    }
    mjb_char(b, '"');
}
static void mjb_int(mc_json_buf_t* b, unsigned long long n) {
    char tmp[32];
    snprintf(tmp, sizeof(tmp), "%llu", n);
    mjb_str(b, tmp);
}
/// NUL-terminate (the buffer is not kept terminated while growing).
static void mjb_done(mc_json_buf_t* b) { mjb_reserve(b, 0); b->data[b->len] = 0; }

// Forward declarations: the codegen-specific renderers are emitted after
// this block (they need the module's input/output/vars types).
static void mc_json_input(mc_json_buf_t* b, const input_t* in);
static void mc_json_output(mc_json_buf_t* b, const output_t* o);
static void mc_json_vars(mc_json_buf_t* b, const vars_t* v);

/// The canonical report document head - key order is the byte-parity
/// contract (see codegen_mc.rs in the toolchain): status, metrics
/// (states, transitions, max_depth), [reason, label, step on fail],
/// statistics (unexpected_terminations, monitor_size_bits,
/// memory_estimate_bytes). Leaves the top-level object OPEN: the caller
/// appends `,"counterexample":{...}` (fail only) and closes with
/// mjb_char(b, '}').
static void mc_json_doc_head(
    mc_json_buf_t* b, const char* status,
    unsigned long long states, unsigned long long transitions, unsigned long long max_depth,
    int has_fail, const char* fail_reason, const char* fail_label, unsigned long long fail_step,
    unsigned long long unexpected_terminations, unsigned long long monitor_size_bits,
    unsigned long long memory_estimate_bytes) {
    mjb_str(b, "{\"status\":"); mjb_escape(b, status);
    mjb_str(b, ",\"metrics\":{\"states\":"); mjb_int(b, states);
    mjb_str(b, ",\"transitions\":"); mjb_int(b, transitions);
    mjb_str(b, ",\"max_depth\":"); mjb_int(b, max_depth);
    mjb_char(b, '}');
    if (has_fail) {
        mjb_str(b, ",\"reason\":"); mjb_escape(b, fail_reason);
        mjb_str(b, ",\"label\":"); mjb_escape(b, fail_label);
        mjb_str(b, ",\"step\":"); mjb_int(b, fail_step);
    }
    mjb_str(b, ",\"statistics\":{\"unexpected_terminations\":"); mjb_int(b, unexpected_terminations);
    mjb_str(b, ",\"monitor_size_bits\":"); mjb_int(b, monitor_size_bits);
    mjb_str(b, ",\"memory_estimate_bytes\":"); mjb_int(b, memory_estimate_bytes);
    mjb_char(b, '}');
}

/// One trace step: {\"input\":...,\"outputs\":[...],\"from_state\":...,\"to_state\":...,\"vars\":...}.
static void mc_json_step(mc_json_buf_t* b, const trace_step_t* s) {
    mjb_str(b, "{\"input\":");
    mc_json_input(b, &s->input);
    mjb_str(b, ",\"outputs\":[");
    for (int i = 0; i < s->n_outputs; i++) {
        if (i > 0) mjb_char(b, ',');
        mc_json_output(b, &s->outputs[i]);
    }
    mjb_str(b, "],\"from_state\":");
    mjb_escape(b, state_name_c(s->from));
    mjb_str(b, ",\"to_state\":");
    mjb_escape(b, state_name_c(s->to));
    mjb_str(b, ",\"vars\":");
    mc_json_vars(b, &s->vars);
    mjb_char(b, '}');
}

/// Counterexample kind \"trace\": the full trace (property violation, incl.
/// sample-0).
static void ce_trace_json(mc_json_buf_t* b, const rfs_trace_t* trace) {
    mjb_str(b, "{\"kind\":\"trace\",\"initial_state\":");
    mjb_escape(b, state_name_c(trace->initial_state));
    mjb_str(b, ",\"initial_vars\":");
    mc_json_vars(b, &trace->initial_vars);
    mjb_str(b, ",\"steps\":[");
    for (int i = 0; i < trace->len; i++) {
        if (i > 0) mjb_char(b, ',');
        mc_json_step(b, &trace->steps[i]);
    }
    mjb_str(b, "]}");
}

/// Counterexample kind \"lasso\": prefix + one cycle iteration (liveness).
static void ce_lasso_json(mc_json_buf_t* b, const rfs_trace_t* trace, int cycle_start) {
    mjb_str(b, "{\"kind\":\"lasso\",\"initial_state\":");
    mjb_escape(b, state_name_c(trace->initial_state));
    mjb_str(b, ",\"initial_vars\":");
    mc_json_vars(b, &trace->initial_vars);
    mjb_str(b, ",\"prefix\":[");
    for (int i = 0; i < cycle_start; i++) {
        if (i > 0) mjb_char(b, ',');
        mc_json_step(b, &trace->steps[i]);
    }
    mjb_str(b, "],\"cycle\":[");
    for (int i = cycle_start; i < trace->len; i++) {
        if (i > cycle_start) mjb_char(b, ',');
        mc_json_step(b, &trace->steps[i]);
    }
    mjb_str(b, "],\"cycle_length\":");
    mjb_int(b, (unsigned long long)(trace->len - cycle_start));
    mjb_char(b, '}');
}

/// Counterexample kind \"deadlock-config\": the terminal configuration.
static void ce_deadlock_json(mc_json_buf_t* b, const rfs_trace_t* trace, int depth) {
    const state_t st = trace->len == 0 ? trace->initial_state : trace->steps[trace->len - 1].to;
    const vars_t* vv = trace->len == 0 ? &trace->initial_vars : &trace->steps[trace->len - 1].vars;
    mjb_str(b, "{\"kind\":\"deadlock-config\",\"depth\":");
    mjb_int(b, (unsigned long long)depth);
    mjb_str(b, ",\"state\":");
    mjb_escape(b, state_name_c(st));
    mjb_str(b, ",\"vars\":");
    mc_json_vars(b, vv);
    mjb_char(b, '}');
}
"#;

/// The exact-mode entry point (generated verbatim into every C MC binary).
const EXACT_RUN_C: &str = r#"
static long mc_elapsed_sec(const struct timespec* t0) {
    struct timespec tn;
    clock_gettime(CLOCK_MONOTONIC, &tn);
    return (long)(tn.tv_sec - t0->tv_sec);
}

static int run_exact(int argc, char** argv) {
    // Runtime flags (codegen-time defaults, D-P4-7).
    int max_states = DEFAULT_MAX_STATES;
    int nondet = DEFAULT_NONDET;
    long timeout_sec = DEFAULT_TIMEOUT_SEC;
    int threads = DEFAULT_THREADS;
    int fast = 0;
    int counterexample_format = DEFAULT_COUNTEREXAMPLE_FORMAT;
    const char* assumes_arg = "all";
    for (int i = 1; i < argc; i++) {
        if (strcmp(argv[i], "--max-states") == 0 && i + 1 < argc) max_states = atoi(argv[++i]);
        else if (strcmp(argv[i], "--nondet") == 0) nondet = 1;
        else if (strcmp(argv[i], "--timeout-sec") == 0 && i + 1 < argc) timeout_sec = atol(argv[++i]);
        else if (strcmp(argv[i], "--threads") == 0 && i + 1 < argc) threads = atoi(argv[++i]);
        else if (strcmp(argv[i], "--fast") == 0) fast = 1;
        else if (strcmp(argv[i], "--counterexample-format") == 0 && i + 1 < argc) counterexample_format = strcmp(argv[++i], "json") == 0 ? 1 : 0;
        else if (strcmp(argv[i], "--assumes") == 0 && i + 1 < argc) assumes_arg = argv[++i];
    }
    if (threads < 1) threads = 1;

    // Active assumptions: "all", "none", or a comma-separated label list.
    char assume_buf[512];
    const char* active[64];
    int n_active = 0;
    if (strcmp(assumes_arg, "all") == 0) {
        for (int i = 0; i < N_ASSUME_LABELS; i++) active[n_active++] = ASSUME_LABELS[i];
    } else if (strcmp(assumes_arg, "none") == 0) {
        n_active = 0;
    } else {
        strncpy(assume_buf, assumes_arg, sizeof(assume_buf) - 1); assume_buf[sizeof(assume_buf) - 1] = 0;
        for (char* tok = strtok(assume_buf, ","); tok; tok = strtok(0, ",")) {
            for (int i = 0; i < N_ASSUME_LABELS; i++) {
                if (strcmp(ASSUME_LABELS[i], tok) == 0) active[n_active++] = ASSUME_LABELS[i];
            }
        }
    }

    struct timespec t0;
    clock_gettime(CLOCK_MONOTONIC, &t0);

    // Initial configuration (fsm_new steps sample 0; an assumption violated at
    // sample 0 aborts, as in Phase 3).
    fsm_t init_fsm;
    fsm_new(&init_fsm);
    init_fsm.active_assumes = active;
    init_fsm.n_active_assumes = n_active;
    if (init_fsm.prop_label[0] != 0) {
        if (counterexample_format == 1) {
            // JSON report (Phase 5 task 10): one compact document, same data
            // as the text report; key order is the byte-parity contract.
            mc_json_buf_t jb = {0};
            mc_json_doc_head(&jb, "fail", 1, 0, 0, 1, "property-violation", init_fsm.prop_label,
                             (unsigned long long)init_fsm.prop_step, 0, MONITORS_BITS,
                             (unsigned long long)(CONFIG_TOTAL_BYTES + 32));
            mjb_str(&jb, ",\"counterexample\":");
            ce_trace_json(&jb, &init_fsm.trace);
            mjb_char(&jb, '}');
            mjb_done(&jb);
            printf("%s\n", jb.data);
            exit(1);
        }
        printf("FAIL\n");
        printf("model-check summary: status=fail states=1 transitions=0 max_depth=0 reason=property-violation label=%s step=%d\n", init_fsm.prop_label, init_fsm.prop_step);
        // Same dedicated sample-0 report as the Rust exact runtime.
        printf("property violation: %s\n", init_fsm.prop_label);
        if (init_fsm.prop_trigger_step >= 0) {
            printf("  trigger step: %d (initial configuration)\n", init_fsm.prop_trigger_step);
        }
        printf("  violation step: %d (initial configuration)\n", init_fsm.prop_step);
        print_trace(&init_fsm.trace);
        exit(1);
    }

    static input_t inputs[N_TOTAL_INPUTS];
    int n_inputs = all_inputs(inputs);

    // BFS storage (bounded by max_states).
    mc_node_t* nodes = (mc_node_t*)malloc((size_t)max_states * sizeof(mc_node_t));
    int* parent_id = (int*)malloc((size_t)max_states * sizeof(int));
    int* parent_input = (int*)malloc((size_t)max_states * sizeof(int));
    int* parent_trans = (int*)malloc((size_t)max_states * sizeof(int));
    int* depth_arr = (int*)malloc((size_t)max_states * sizeof(int));
    uint8_t* visited = (uint8_t*)malloc((size_t)max_states * CONFIG_TOTAL_BYTES);
    int* visited_idx = (int*)malloc((size_t)max_states * sizeof(int));
    // Explored graph edges (liveness analysis, D-P4-10): per config,
    // (input index, transition id, target config) in discovery order.
    int* adj_count = (int*)calloc((size_t)max_states, sizeof(int));
    unsigned char* complete = (unsigned char*)calloc((size_t)max_states, 1);
    size_t adj_cap = 64;
    int adj_n = 0;
    mc_edge_t* adj_edges = (mc_edge_t*)malloc(adj_cap * sizeof(mc_edge_t));
    if (!nodes || !parent_id || !parent_input || !parent_trans || !depth_arr || !visited ||
        !visited_idx || !adj_count || !complete || !adj_edges) {
        fprintf(stderr, "out of memory\n");
        exit(2);
    }

    nodes[0].state = init_fsm.state;
    nodes[0].vars = init_fsm.vars;
    nodes[0].prop = init_fsm.prop_monitors;
    nodes[0].assume = init_fsm.assume_monitors;
    int n_nodes = 1;
    uint8_t key0[CONFIG_TOTAL_BYTES];
    config_bytes(nodes[0].state, &nodes[0].vars, &nodes[0].prop, &nodes[0].assume, key0);
    memcpy(visited, key0, CONFIG_TOTAL_BYTES);
    visited_idx[0] = 0;
    int n_visited = 1;

    // Level-synchronous BFS (D-P4-2): canonical input order, successors in
    // firing order; visited set checked at enqueue time; parent pointers for
    // counterexample reconstruction.
    int level_start = 0, level_end = 1;
    int max_depth = 0, transitions = 0, unexpected = 0;
    int first_unexp_cid = -1, first_unexp_idx = -1;
    // status: 0 running, 1 pass-complete, 2 state limit, 3 timeout, 4 fail
    int status = 0;
    const char* fail_wire = "";
    char fail_label[64] = "-";
    int fail_step = 0;
    int n_fail_steps = 0;
    int* fail_cid = 0;
    int* fail_input = 0;
    int* fail_trans = 0;
    int is_deadlock = 0;

    // Parallel level expansion (task 10B): with --threads N > 1, phase A
    // computes every successor outcome of the current level in parallel
    // (workers own disjoint config ranges; no shared mutable state), and
    // phase B commits them SEQUENTIALLY in canonical order - visited insert
    // at enqueue, parent/depth/adj pushes, first violation, deadlock check -
    // exactly as the single-threaded engine does. Results are therefore a
    // pure function of the spec: byte-identical to --threads 1 for any N.
    // --fast (non-parity): phase A stops scheduling work once any worker
    // finds a violation, and the first found violation is reported.
    while (level_start < level_end && status == 0) {
        if (timeout_sec > 0 && mc_elapsed_sec(&t0) >= timeout_sec) { status = 3; break; }
        int next_start = n_nodes;
        int n_level = level_end - level_start;
        int n_threads = threads < n_level ? threads : n_level;

        // Phase A: expand the level. Per-config outcome lists in canonical
        // position order (input x effect).
        mc_outcomes_t* outs = (mc_outcomes_t*)calloc((size_t)n_level, sizeof(mc_outcomes_t));
        if (!outs) { fprintf(stderr, "out of memory\n"); exit(2); }
        atomic_int abort_flag = 0;
        if (n_threads <= 1) {
            mc_expand_range(nodes, depth_arr, inputs, n_inputs, nondet, active, n_active,
                            level_start, level_start, level_end, outs, &abort_flag, fast);
        } else {
            int nw = n_threads < 64 ? n_threads : 64;
            pthread_t tids[64];
            mc_worker_arg_t* args = (mc_worker_arg_t*)malloc((size_t)nw * sizeof(mc_worker_arg_t));
            if (!args) { fprintf(stderr, "out of memory\n"); exit(2); }
            int n_spawned = 0;
            int chunk = (n_level + nw - 1) / nw;
            for (int w = 0; w < nw; w++) {
                int lo = level_start + w * chunk;
                if (lo >= level_end) break;
                int hi = level_start + (w + 1) * chunk;
                if (hi > level_end) hi = level_end;
                args[w].nodes = nodes; args[w].depth_arr = depth_arr;
                args[w].inputs = inputs; args[w].n_inputs = n_inputs;
                args[w].nondet = nondet; args[w].active = active; args[w].n_active = n_active;
                args[w].level_start = level_start; args[w].lo = lo; args[w].hi = hi;
                args[w].outs = outs; args[w].abort_flag = &abort_flag; args[w].fast = fast;
                if (pthread_create(&tids[n_spawned], 0, mc_expand_worker, &args[w]) == 0) {
                    n_spawned++;
                } else {
                    // Fallback: run this range inline (results stay canonical).
                    mc_expand_range(nodes, depth_arr, inputs, n_inputs, nondet, active, n_active,
                                    level_start, lo, hi, outs, &abort_flag, fast);
                }
            }
            for (int w = 0; w < n_spawned; w++) pthread_join(tids[w], 0);
            free(args);
        }

        // --fast early abort (non-parity): report the first violation found.
        // Which configs were expanded before the abort is race-dependent, so
        // states/transitions/label may vary between runs (never a false pass).
        if (fast && n_threads > 1 && atomic_load_explicit(&abort_flag, memory_order_relaxed)) {
            int found = 0;
            for (int cid = level_start; cid < level_end && !found; cid++) {
                mc_outcomes_t* o = &outs[cid - level_start];
                for (int k = 0; k < o->n; k++) {
                    if (o->v[k].kind != 2) continue;
                    status = 4;
                    fail_wire = "property-violation";
                    strncpy(fail_label, o->v[k].label, sizeof(fail_label) - 1);
                    fail_label[sizeof(fail_label) - 1] = 0;
                    fail_step = o->v[k].step;
                    n_fail_steps = depth_arr[cid] + 1;
                    fail_cid = (int*)malloc((size_t)n_fail_steps * sizeof(int));
                    fail_input = (int*)malloc((size_t)n_fail_steps * sizeof(int));
                    fail_trans = (int*)malloc((size_t)n_fail_steps * sizeof(int));
                    mc_fill_path(parent_id, parent_input, parent_trans, depth_arr, cid, fail_cid, fail_input, fail_trans);
                    fail_cid[depth_arr[cid]] = cid;
                    fail_input[depth_arr[cid]] = o->v[k].iidx;
                    fail_trans[depth_arr[cid]] = o->v[k].tid;
                    found = 1;
                    break;
                }
            }
            // Unreachable: the flag is only set after a Violation outcome is
            // stored. If it ever triggers, report unknown (never a false pass).
            if (!found) status = 2;
            for (int k2 = 0; k2 < n_level; k2++) free(outs[k2].v);
            free(outs);
            goto done;
        }

        // Phase B: sequential canonical commit - identical to the pre-task-10B
        // single-threaded order for every observable counter and pointer.
        for (int cid = level_start; cid < level_end && status == 0; cid++) {
            int d = depth_arr[cid];
            int any_successor = 0;
            mc_outcomes_t* o = &outs[cid - level_start];
            for (int k = 0; k < o->n; k++) {
                if (o->v[k].kind == 0) {
                    // Input matches no transition and there is no on_error:
                    // this path terminates (D-P4-5); the configuration itself
                    // may still have other successors. (No transition was
                    // evaluated, so `transitions` is untouched - as before.)
                    unexpected++;
                    if (first_unexp_cid < 0) { first_unexp_cid = cid; first_unexp_idx = o->v[k].iidx; }
                    continue;
                }
                transitions++;
                if (o->v[k].kind == 1) continue; // pruned by an active assumption
                if (o->v[k].kind == 2) {
                    status = 4;
                    fail_wire = "property-violation";
                    strncpy(fail_label, o->v[k].label, sizeof(fail_label) - 1);
                    fail_label[sizeof(fail_label) - 1] = 0;
                    fail_step = o->v[k].step;
                    n_fail_steps = d + 1;
                    fail_cid = (int*)malloc((size_t)n_fail_steps * sizeof(int));
                    fail_input = (int*)malloc((size_t)n_fail_steps * sizeof(int));
                    fail_trans = (int*)malloc((size_t)n_fail_steps * sizeof(int));
                    mc_fill_path(parent_id, parent_input, parent_trans, depth_arr, cid, fail_cid, fail_input, fail_trans);
                    fail_cid[d] = cid;
                    fail_input[d] = o->v[k].iidx;
                    fail_trans[d] = o->v[k].tid;
                    goto free_outs;
                }
                any_successor = 1;
                mc_node_t succ = o->v[k].node;
                uint8_t key[CONFIG_TOTAL_BYTES];
                config_bytes(succ.state, &succ.vars, &succ.prop, &succ.assume, key);
                int tcid = mc_visited_find(visited, visited_idx, n_visited, key);
                if (tcid < 0) {
                    if (n_nodes >= max_states) { status = 2; goto free_outs; }
                    mc_visited_insert(visited, visited_idx, &n_visited, key, n_nodes);
                    parent_id[n_nodes] = cid;
                    parent_input[n_nodes] = o->v[k].iidx;
                    parent_trans[n_nodes] = o->v[k].tid;
                    depth_arr[n_nodes] = d + 1;
                    if (d + 1 > max_depth) max_depth = d + 1;
                    nodes[n_nodes++] = succ;
                    tcid = n_nodes - 1;
                }
                // Graph edge: recorded for every Success outcome, including
                // edges to already-visited configs (mirror of the Rust engine).
                if ((size_t)adj_n == adj_cap) {
                    size_t ncap = adj_cap * 2;
                    mc_edge_t* ne = (mc_edge_t*)realloc(adj_edges, ncap * sizeof(mc_edge_t));
                    if (!ne) { fprintf(stderr, "out of memory\n"); exit(2); }
                    adj_edges = ne;
                    adj_cap = ncap;
                }
                adj_edges[adj_n].input_idx = o->v[k].iidx;
                adj_edges[adj_n].trans_id = o->v[k].tid;
                adj_edges[adj_n].target = tcid;
                adj_count[cid]++;
                adj_n++;
            }
            complete[cid] = 1;
            if (!any_successor) {
                // Deadlock (D-P4-5): no successor at all - every input was
                // unexpected or pruned by an active assumption.
                status = 4;
                fail_wire = "deadlock";
                fail_label[0] = '-';
                fail_label[1] = 0;
                fail_step = d;
                n_fail_steps = d;
                fail_cid = (int*)malloc((size_t)(n_fail_steps ? n_fail_steps : 1) * sizeof(int));
                fail_input = (int*)malloc((size_t)(n_fail_steps ? n_fail_steps : 1) * sizeof(int));
                fail_trans = (int*)malloc((size_t)(n_fail_steps ? n_fail_steps : 1) * sizeof(int));
                mc_fill_path(parent_id, parent_input, parent_trans, depth_arr, cid, fail_cid, fail_input, fail_trans);
                is_deadlock = 1;
                goto free_outs;
            }
        }
free_outs:
        for (int k2 = 0; k2 < n_level; k2++) free(outs[k2].v);
        free(outs);
        level_start = next_start;
        level_end = n_nodes;
    }
done:
    if (status == 0) status = 1;

    // Liveness analysis (D-P4-10/11): runs only when BFS ended without a
    // safety failure or deadlock. On a truncated graph (state limit / timeout)
    // this is a TRUNCATED analysis: a witness found inside the explored part
    // is sound (all its edges are real transitions), otherwise the UNKNOWN
    // status stands - never a false pass-complete.
    liv_graph_t graph;
    memset(&graph, 0, sizeof(graph));
    int* adj_start = 0;
    int is_liveness = 0;
    int liv_pi = -1;
    liv_witness_t kept_w;
    memset(&kept_w, 0, sizeof(kept_w));
    if (status != 4 && N_LIV_PROPS > 0) {
        adj_start = (int*)liv_malloc(((size_t)n_nodes + 1) * sizeof(int));
        adj_start[0] = 0;
        for (int c = 0; c < n_nodes; c++) adj_start[c + 1] = adj_start[c] + adj_count[c];
        graph.nodes = nodes;
        graph.n_nodes = n_nodes;
        graph.edges = adj_edges;
        graph.adj_start = adj_start;
        graph.complete = complete;
        graph.inputs = inputs;
        graph.n_inputs = n_inputs;
        graph.nondet = nondet;
        // The label cache is per-property: bit positions in the 32-bit label
        // depend on that property's leaves (bit 2k/2k+1 of leaf k), so a cache
        // shared across properties would mislabel edges. Allocate once per
        // property, free before the next.
        size_t total_edges = (size_t)adj_start[n_nodes];
        for (int pi = 0; pi < N_LIV_PROPS && !is_liveness; pi++) {
            uint32_t* edge_bits = 0;
            unsigned char* edge_done = 0;
            if (total_edges > 0) {
                edge_bits = (uint32_t*)calloc(total_edges, sizeof(uint32_t));
                edge_done = (unsigned char*)calloc(total_edges, 1);
                if (!edge_bits || !edge_done) { fprintf(stderr, "out of memory\n"); exit(2); }
            }
            int violated = liv_analyze(&graph, LIV_PROPS[pi], edge_bits, edge_done, &kept_w);
            free(edge_bits);
            free(edge_done);
            if (violated) {
                is_liveness = 1;
                liv_pi = pi;
                status = 4;
                fail_wire = "liveness-violation";
                strncpy(fail_label, LIV_PROPS[pi]->label, sizeof(fail_label) - 1);
                fail_label[sizeof(fail_label) - 1] = 0;
                fail_step = kept_w.cycle_start;
                n_fail_steps = kept_w.n_lasso;
                fail_cid = kept_w.lasso_src;
                fail_input = kept_w.lasso_input;
                fail_trans = kept_w.lasso_trans;
            }
        }
    }

    // Report (D-P4-4): human line, pinned machine line, statistics.
    size_t mem_est = (size_t)n_nodes * ((size_t)CONFIG_TOTAL_BYTES + 32);
    if (counterexample_format == 1) {
        // JSON report (Phase 5 task 10): one compact document carrying the
        // same data as the text report below; key order is the byte-parity
        // contract (see codegen_mc.rs in the toolchain).
        mc_json_buf_t jb = {0};
        const char* st_wire = status == 1 ? "pass-complete" : (status == 4 ? "fail" : "unknown-state-limit");
        int has_fail = status == 4;
        mc_json_doc_head(&jb, st_wire, (unsigned long long)n_nodes, (unsigned long long)transitions,
                         (unsigned long long)max_depth, has_fail, fail_wire, fail_label,
                         (unsigned long long)fail_step, (unsigned long long)unexpected,
                         MONITORS_BITS, (unsigned long long)mem_est);
        if (has_fail) {
            fsm_t cfsm;
            mc_replay(fail_cid, fail_input, fail_trans, n_fail_steps, nondet, active, n_active, inputs, &cfsm);
            mjb_str(&jb, ",\"counterexample\":");
            if (is_liveness) ce_lasso_json(&jb, &cfsm.trace, fail_step);
            else if (is_deadlock) ce_deadlock_json(&jb, &cfsm.trace, fail_step);
            else ce_trace_json(&jb, &cfsm.trace);
        }
        mjb_char(&jb, '}');
        mjb_done(&jb);
        printf("%s\n", jb.data);
        exit(status == 1 ? 0 : (status == 4 ? 1 : 2));
    }
    if (status == 1) printf("PASS (complete)\n");
    else if (status == 4) printf("FAIL\n");
    else if (status == 3) printf("UNKNOWN (state limit reached, timeout after %lds)\n", mc_elapsed_sec(&t0));
    else printf("UNKNOWN (state limit reached)\n");
    printf("model-check summary: status=%s states=%d transitions=%d max_depth=%d",
           status == 1 ? "pass-complete" : (status == 4 ? "fail" : "unknown-state-limit"),
           n_nodes, transitions, max_depth);
    if (status == 4) printf(" reason=%s label=%s step=%d", fail_wire, fail_label, fail_step);
    printf("\n");
    printf("  unexpected terminations: %d\n", unexpected);
    if (first_unexp_cid >= 0) printf("  first unexpected: config %d input index %d\n", first_unexp_cid, first_unexp_idx);
    // Statistics block (task 10): identical field set and format in both
    // backends - monitor size is the encoded bit width of all property +
    // assumption monitors (deterministic; feeds CONFIG_TOTAL_BYTES).
    printf("  monitor size: %u bits\n", MONITORS_BITS);
    printf("  memory estimate: %llu bytes\n", (unsigned long long)mem_est);

    if (status == 4) {
        // Counterexample reconstruction (Phase 3 trace format).
        fsm_t cfsm;
        mc_replay(fail_cid, fail_input, fail_trans, n_fail_steps, nondet, active, n_active, inputs, &cfsm);
        if (is_liveness) {
            printf("liveness violation: %s\n", fail_label);
            printf("  formula: %s\n", LIV_PROPS[liv_pi]->formula);
            printf("  violation step: %d\n", fail_step);
            printf("  cycle length: %d\n", n_fail_steps - fail_step);
            char note_buf[1024];
            liv_note(&graph, LIV_PROPS[liv_pi], &kept_w, note_buf, sizeof(note_buf));
            printf("  note: %s\n", note_buf);
            print_trace(&cfsm.trace);
        } else if (is_deadlock) {
            printf("deadlock: configuration at depth %d has no enabled transition\n", fail_step);
            print_trace(&cfsm.trace);
        } else {
            mc_report_property(&cfsm);
        }
        exit(1);
    }
    return status == 1 ? 0 : 2;
}
"#;

/// Mode dispatch (D-P4-7): `--mode fuzz` selects the Phase 3 runtime.
const MAIN_DISPATCH_C: &str = r#"
int main(int argc, char** argv) {
    int mode = DEFAULT_MODE;
    for (int i = 1; i < argc; i++) {
        if (strcmp(argv[i], "--mode") == 0 && i + 1 < argc) {
            i++;
            mode = strcmp(argv[i], "fuzz") == 0 ? 1 : 0;
        }
    }
    if (mode == 1) return run_fuzz(argc, argv);
    return run_exact(argc, argv);
}
"#;
/// The Phase 3 fuzz runtime (unchanged except `main` -> `run_fuzz`; the
/// shared printers moved to [`shared_printers`]).
fn fuzz_runtime(module: &Module) -> String {
    let candidates = rep_inputs(module);
    let n_all = candidates.len().max(1);
    let cand_list = if candidates.is_empty() {
        "{{ 0 }}".to_string()
    } else {
        format!("{{ {} }}", candidates.join(", "))
    };
    format!(
        r#"
// ---- generated bounded model checker (same wire format as the Rust MC) ----
static fsm_t clone_fsm(const fsm_t* f) {{
    fsm_t t = *f;
    if (f->trace.steps != 0) {{
        t.trace.cap = f->trace.cap;
        t.trace.steps = (trace_step_t*)malloc((size_t)t.trace.cap * sizeof(trace_step_t));
        memcpy(t.trace.steps, f->trace.steps, (size_t)f->trace.len * sizeof(trace_step_t));
    }} else {{ t.trace.steps = 0; }}
    return t;
}}
static rfs_policy_t stop_handler(void* user_data, const char* label) {{ (void)user_data; (void)label; return RFS_STOP; }}
static rfs_policy_t cont_handler(void* user_data, const char* label) {{ (void)user_data; (void)label; return RFS_CONTINUE; }}

static uint64_t MC_SEED;
// Same LCG as the Rust model checker: 6364136223846793005 / 1442695040888963407.
static double mc_next_f64(void) {{
    MC_SEED = MC_SEED * 6364136223846793005ULL + 1442695040888963407ULL;
    return (double)(MC_SEED >> 11) / (double)(((uint64_t)-1) >> 11);
}}
static int mc_chance(double p) {{ double lo = p < 0.0 ? 0.0 : (p > 1.0 ? 1.0 : p); return mc_next_f64() < lo; }}
static int mc_pick_index(int len) {{ if (len <= 0) return -1; return (int)(mc_next_f64() * (double)len) % len; }}

typedef struct {{ const char* label; double prob; double percent; }} violation_spec_t;
static void parse_violation_spec(char* s, violation_spec_t* out) {{
    // `s` is a mutable buffer: truncate the label at the first ':' so it
    // matches the Rust parser (label:prob:percent%).
    out->label = s; out->prob = 1.0; out->percent = 100.0;
    char* c1 = (char*)strchr(s, ':');
    if (c1) {{
        *c1 = 0;
        out->prob = strtod(c1 + 1, 0);
        const char* c2 = strchr(c1 + 1, ':');
        if (c2) out->percent = strtod(c2 + 1, 0);
    }}
}}

/// Print the property-specific counterexample (which property failed, which
/// step triggered the obligation, when the violation was confirmed, relevant
/// variable values, full input/output sequence) and exit.
static void report_property_violation(const fsm_t* f, int run) {{
    printf("run %d property violation: %s\n", run, f->prop_label);
    if (f->prop_trigger_step >= 0) {{
        printf("  trigger step: %d\n", f->prop_trigger_step);
        printf("  vars at trigger step: "); print_vars_debug(&f->trace.steps[f->prop_trigger_step - 1].vars); printf("\n");
    }}
    printf("  violation step: %d\n", f->prop_step);
    const vars_t* vv = (f->prop_step == 0) ? &f->trace.initial_vars : &f->trace.steps[f->prop_step - 1].vars;
    printf("  vars at violation step: "); print_vars_debug(vv); printf("\n");
    print_trace(&f->trace);
    exit(1);
}}

static int run_fuzz(int argc, char** argv) {{
    int bound = 5;
    int runs = 20;
    uint64_t seed = 12345ULL;
    const char* assumes_arg = "all";
    violation_spec_t specs[16];
    int n_specs = 0;
    double unexpected_prob = 0.0;
    for (int i = 1; i < argc; i++) {{
        if (strcmp(argv[i], "--bound") == 0 && i + 1 < argc) bound = atoi(argv[++i]);
        else if (strcmp(argv[i], "--runs") == 0 && i + 1 < argc) runs = atoi(argv[++i]);
        else if (strcmp(argv[i], "--seed") == 0 && i + 1 < argc) seed = strtoull(argv[++i], 0, 10);
        else if (strcmp(argv[i], "--assumes") == 0 && i + 1 < argc) assumes_arg = argv[++i];
        else if (strcmp(argv[i], "--violate") == 0 && i + 1 < argc) {{
            // comma-separated label:prob:percent% entries
            char buf[512]; strncpy(buf, argv[++i], sizeof(buf) - 1); buf[sizeof(buf) - 1] = 0;
            for (char* tok = strtok(buf, ","); tok && n_specs < 16; tok = strtok(0, ",")) {{
                parse_violation_spec(tok, &specs[n_specs++]); // tok is a mutable buffer
            }}
        }}
        else if (strcmp(argv[i], "--unexpected-prob") == 0 && i + 1 < argc) unexpected_prob = strtod(argv[++i], 0);
    }}

    // Active assumptions: "all", "none", or a comma-separated label list.
    char assume_buf[512];
    if (strcmp(assumes_arg, "all") != 0 && strcmp(assumes_arg, "none") != 0) {{
        strncpy(assume_buf, assumes_arg, sizeof(assume_buf) - 1); assume_buf[sizeof(assume_buf) - 1] = 0;
    }}
    int n_active = 0;
    const char* active[64];
    if (strcmp(assumes_arg, "all") == 0) {{
        for (int i = 0; i < N_ASSUME_LABELS; i++) active[n_active++] = ASSUME_LABELS[i];
    }} else if (strcmp(assumes_arg, "none") == 0) {{
        n_active = 0;
    }} else {{
        for (char* tok = strtok(assume_buf, ","); tok; tok = strtok(0, ",")) {{
            for (int i = 0; i < N_ASSUME_LABELS; i++) {{
                if (strcmp(ASSUME_LABELS[i], tok) == 0) active[n_active++] = ASSUME_LABELS[i];
            }}
        }}
    }}

    static const input_t ALL_INPUTS[{n_all}] = {cand_list};
    #define N_ALL_INPUTS (sizeof(ALL_INPUTS) / sizeof(ALL_INPUTS[0]))

    MC_SEED = seed;
    // Property violations terminate the checker immediately (exit 1), so
    // this counter is always 0 in the summary; the field stays for a stable
    // summary format.
    int property_violations = 0, unexpected_count = 0, assumption_stops = 0, completed = 0;

    for (int run = 0; run < runs; run++) {{
        int violating[16]; double target_fraction[16];
        for (int i = 0; i < n_specs; i++) {{ violating[i] = mc_chance(specs[i].prob); target_fraction[i] = specs[i].percent / 100.0; }}

        fsm_t fsm; fsm_new(&fsm);
        // Restrict active assumptions to the CLI selection (parity with the
        // Rust model checker; the sample-0 check in fsm_new ran with all).
        fsm.active_assumes = active;
        fsm.n_active_assumes = n_active;
        // A property can already be violated at sample 0 (the initial
        // configuration, e.g. an invariant on the initial state).
        if (fsm.prop_label[0] != 0) report_property_violation(&fsm, run);
        const char* status = "completed";

        for (int step = 0; step < bound; step++) {{
            // Phase 1: forced assumption violation.
            int forced = -1;
            for (int i = 0; i < n_specs && forced < 0; i++) {{
                if (!violating[i] || !mc_chance(target_fraction[i])) continue;
                for (int c = 0; c < N_ALL_INPUTS; c++) {{
                    fsm_t t = clone_fsm(&fsm);
                    step_result_t r = fsm_step(&t, ALL_INPUTS[c], stop_handler, 0);
                    if ((r.status == RFS_ASSUMPTION_VIOLATION && strcmp(r.assume_label, specs[i].label) == 0)
                        || (r.status == RFS_PROPERTY_VIOLATION && strcmp(r.prop_label, specs[i].label) == 0)) {{ forced = c; break; }}
                }}
            }}
            if (forced >= 0) {{
                step_result_t r = fsm_step(&fsm, ALL_INPUTS[forced], stop_handler, 0);
                if (r.status == RFS_ASSUMPTION_VIOLATION) {{ status = "assumption-violated"; assumption_stops++; break; }}
                if (fsm.prop_label[0] != 0) report_property_violation(&fsm, run);
                continue;
            }}

            // Phase 2: optional unexpected injection.
            if (mc_chance(unexpected_prob)) {{
                int idx = mc_pick_index(N_ALL_INPUTS);
                if (idx >= 0) {{
                    step_result_t r = fsm_step(&fsm, ALL_INPUTS[idx], cont_handler, 0);
                    if (r.status == RFS_UNEXPECTED_EVENT) {{ status = "unexpected-event"; unexpected_count++; break; }}
                }}
                if (fsm.prop_label[0] != 0) report_property_violation(&fsm, run);
                continue;
            }}

            // Phase 3: allowed representative input.
            int allowed[256]; int n_allowed = 0;
            for (int c = 0; c < N_ALL_INPUTS && n_allowed < 256; c++) {{
                fsm_t t = clone_fsm(&fsm);
                step_result_t r = fsm_step(&t, ALL_INPUTS[c], stop_handler, 0);
                // Inputs that confirm a property violation are allowed on
                // purpose: the model checker wants to find violations.
                if (r.status == RFS_OK || r.status == RFS_PROPERTY_VIOLATION) allowed[n_allowed++] = c;
            }}
            if (n_allowed == 0) {{ status = "stuck"; break; }}
            int idx = mc_pick_index(n_allowed);
            step_result_t r = fsm_step(&fsm, ALL_INPUTS[allowed[idx]], stop_handler, 0);
            if (r.status == RFS_UNEXPECTED_EVENT) {{ status = "unexpected-event"; unexpected_count++; break; }}
            if (r.status == RFS_ASSUMPTION_VIOLATION) {{ status = "assumption-violated"; assumption_stops++; break; }}
            if (fsm.prop_label[0] != 0) report_property_violation(&fsm, run);
        }}

        if (strcmp(status, "completed") == 0) completed++;
    }}

    printf("model-check summary: runs=%d bound=%d completed=%d unexpected=%d assumption_stops=%d property_violations=%d\n", runs, bound, completed, unexpected_count, assumption_stops, property_violations);
    return 0;
}}
"#,
        n_all = n_all,
        cand_list = cand_list
    )
}

const SMT_ENGINE_C: &str = r#"

// ===== SMT bounded-mode runtime (task 11) =====
// C mirror of SMT_ENGINE_RUST: same step-variable names, same constraint
// construction order, same z3 operations, same output format and exit codes.
#define DEFAULT_BOUND {BOUND}u

static uint64_t smt_read_bv(Z3_context c, Z3_model m, Z3_ast e) {
    Z3_ast v = 0;
    if (!Z3_model_eval(c, m, e, true, &v)) { fprintf(stderr, "error: model evaluation failed\n"); exit(2); }
    uint64_t u = 0;
    if (!Z3_get_numeral_uint64(c, v, &u)) { fprintf(stderr, "error: non-numeral model value\n"); exit(2); }
    return u;
}

static int smt_read_bool(Z3_context c, Z3_model m, Z3_ast e) {
    Z3_ast v = 0;
    if (!Z3_model_eval(c, m, e, true, &v)) { fprintf(stderr, "error: model evaluation failed\n"); exit(2); }
    Z3_lbool b = Z3_get_bool_value(c, v);
    if (b != Z3_L_TRUE && b != Z3_L_FALSE) { fprintf(stderr, "error: non-boolean model value\n"); exit(2); }
    return b == Z3_L_TRUE;
}

/// Witness trace (pinned format - identical in both backends).
static void smt_print_trace(Z3_context c, unsigned k, Z3_model m, const Z3_ast *s_vars, const Z3_ast *v_vars, const Z3_ast *ev_vars, const Z3_ast *fld_vars) {
    printf("witness trace (solver model; bound k=%u):\n", k);
    for (unsigned i = 0; i < k; i++) {
        unsigned si = (unsigned)smt_read_bv(c, m, s_vars[i]);
        printf("  step %u: state=%s vars={", i, STATES[si]);
        for (unsigned j = 0; j < N_VARS; j++) {
            if (j > 0) printf(", ");
            uint64_t val = smt_read_bv(c, m, v_vars[(size_t)i * N_VARS + j]);
            if (VAR_IS_BOOL[j]) printf("%s=%s", VAR_NAMES[j], val != 0 ? "true" : "false");
            else printf("%s=%llu", VAR_NAMES[j], (unsigned long long)val);
        }
        printf("} ");
        if (N_CLASSES > 1) {
            unsigned ci = (unsigned)smt_read_bv(c, m, ev_vars[i]);
            int first = 1;
            printf("event=%s(", CLASS_NAMES[ci]);
            for (unsigned g = 0; g < N_FIELDS; g++) {
                if (FIELD_POS[g].c != ci) continue;
                if (!first) printf(", ");
                first = 0;
                uint64_t val = smt_read_bv(c, m, fld_vars[(size_t)i * N_FIELDS + g]);
                if (FIELD_IS_BOOL[g]) printf("%s=%s", FIELD_NAMES[g], val != 0 ? "true" : "false");
                else printf("%s=%llu", FIELD_NAMES[g], (unsigned long long)val);
            }
            printf(")");
        } else {
            printf("event=-");
        }
        printf("\n");
    }
}

static int run_smt(int argc, char **argv) {
    // Runtime flags.
    unsigned bound = DEFAULT_BOUND;
    for (int ai = 1; ai < argc; ai++) {
        if (strcmp(argv[ai], "--bound") == 0 && ai + 1 < argc) {
            char *end = 0;
            unsigned long v = strtoul(argv[ai + 1], &end, 10);
            if (end != argv[ai + 1] && *end == '\0') bound = (unsigned)v;
        }
    }
    unsigned k = bound > 1u ? bound : 1u;

    Z3_context c = Z3_mk_context(0);

    // ---- Step variables (fixed names/order - parity-critical) ----
    Z3_ast *s_vars = malloc(k * sizeof(Z3_ast));
    Z3_ast *v_vars = malloc((size_t)k * (N_VARS ? N_VARS : 1) * sizeof(Z3_ast));
    Z3_ast *ev_vars = N_CLASSES > 1 ? malloc(k * sizeof(Z3_ast)) : 0;
    Z3_ast *fld_vars = malloc((size_t)k * (N_FIELDS ? N_FIELDS : 1) * sizeof(Z3_ast));
    char name[64];
    for (unsigned i = 0; i < k; i++) {
        snprintf(name, sizeof name, "s_%u", i);
        s_vars[i] = Z3_mk_fresh_const(c, name, Z3_mk_bv_sort(c, STATE_BITS));
        for (unsigned j = 0; j < N_VARS; j++) {
            snprintf(name, sizeof name, "v_%u_%u", i, j);
            v_vars[(size_t)i * N_VARS + j] = Z3_mk_fresh_const(c, name, Z3_mk_bv_sort(c, VAR_WIDTHS[j]));
        }
        if (N_CLASSES > 1) {
            snprintf(name, sizeof name, "ev_%u", i);
            ev_vars[i] = Z3_mk_fresh_const(c, name, Z3_mk_bv_sort(c, CLS_BITS));
        }
        for (unsigned g = 0; g < N_FIELDS; g++) {
            snprintf(name, sizeof name, "fld_%u_%u", i, g);
            fld_vars[(size_t)i * N_FIELDS + g] = Z3_mk_fresh_const(c, name, Z3_mk_bv_sort(c, FIELD_WIDTHS[g]));
        }
    }

    // ---- Base constraints: initial configuration + k-step unrolling ----
    size_t base_n = 0, base_cap = 0;
    Z3_ast *base = 0;
#define BASE_PUSH(a) do { \
        if (base_n == base_cap) { \
            base_cap = base_cap ? base_cap * 2 : 16; \
            base = realloc(base, base_cap * sizeof(Z3_ast)); \
        } \
        base[base_n++] = (a); \
    } while (0)

    BASE_PUSH(Z3_mk_eq(c, s_vars[0], z3_num(c, STATE_BITS, INITIAL_STATE)));
    for (unsigned j = 0; j < N_VARS; j++) {
        BASE_PUSH(Z3_mk_eq(c, v_vars[j], z3_num(c, VAR_WIDTHS[j], VAR_INITS[j])));
    }

    Z3_ast *alive = malloc(k * sizeof(Z3_ast));
    alive[0] = Z3_mk_true(c);
    int ret = 0;
    for (unsigned i = 0; i + 1 < k; i++) {
        const Z3_ast *vi = &v_vars[(size_t)i * N_VARS];
        const Z3_ast *fldi = &fld_vars[(size_t)i * N_FIELDS];
        // enabled_t = state match ∧ class match ∧ guard (firing order).
        Z3_ast *enableds = malloc((N_TRANS ? N_TRANS : 1) * sizeof(Z3_ast));
        for (unsigned t = 0; t < N_TRANS; t++) {
            Z3_ast en = Z3_mk_eq(c, s_vars[i], z3_num(c, STATE_BITS, TRANS[t].state));
            if (N_CLASSES > 1) {
                Z3_ast pair[2] = { en, Z3_mk_eq(c, ev_vars[i], z3_num(c, CLS_BITS, TRANS[t].cls)) };
                en = Z3_mk_and(c, 2, pair);
            }
            Z3_ast pair2[2] = { en, GUARD_FNS[t](c, s_vars[i], vi, fldi) };
            enableds[t] = Z3_mk_and(c, 2, pair2);
        }
        // fire_t = enabled_t ∧ no earlier transition enabled (first match).
        Z3_ast *fires = malloc((N_TRANS ? N_TRANS : 1) * sizeof(Z3_ast));
        Z3_ast prefix = 0;
        for (unsigned t = 0; t < N_TRANS; t++) {
            if (prefix) {
                Z3_ast pair[2] = { Z3_mk_not(c, prefix), enableds[t] };
                fires[t] = Z3_mk_and(c, 2, pair);
            } else {
                Z3_ast pair[2] = { enableds[t], Z3_mk_true(c) };
                fires[t] = Z3_mk_and(c, 2, pair);
            }
            // The and-true keeps the node fresh (no aliasing of enableds);
            // z3 rewrites it to `en` - the Rust backend does the same op.
            if (prefix) {
                Z3_ast pair[2] = { prefix, enableds[t] };
                prefix = Z3_mk_or(c, 2, pair);
            } else {
                Z3_ast pair[2] = { enableds[t], Z3_mk_true(c) };
                prefix = Z3_mk_and(c, 2, pair);
            }
        }
        Z3_ast any_fire;
        if (N_TRANS == 0) {
            any_fire = Z3_mk_false(c);
        } else {
            Z3_ast pair[2] = { fires[0], Z3_mk_true(c) };
            any_fire = Z3_mk_and(c, 2, pair);
            for (unsigned t = 1; t < N_TRANS; t++) {
                Z3_ast pair[2] = { any_fire, fires[t] };
                any_fire = Z3_mk_or(c, 2, pair);
            }
        }
        Z3_ast apair[2] = { alive[i], any_fire };
        alive[i + 1] = Z3_mk_and(c, 2, apair);
        if (N_TRANS == 0) {
            // No transitions: deadlocked from step 0; state/vars frozen.
            BASE_PUSH(Z3_mk_eq(c, s_vars[i], s_vars[i + 1]));
            for (unsigned j = 0; j < N_VARS; j++) {
                BASE_PUSH(Z3_mk_eq(c, v_vars[(size_t)i * N_VARS + j], v_vars[(size_t)(i + 1) * N_VARS + j]));
            }
        } else {
            // Next state: ITE chain over firing transitions (last first).
            Z3_ast chain = z3_num(c, STATE_BITS, TRANS[N_TRANS - 1].next_state);
            for (long u = (long)N_TRANS - 2; u >= 0; u--) {
                chain = Z3_mk_ite(c, fires[u], z3_num(c, STATE_BITS, TRANS[u].next_state), chain);
            }
            BASE_PUSH(Z3_mk_eq(c, Z3_mk_ite(c, any_fire, chain, s_vars[i]), s_vars[i + 1]));
            // Next vars: per-var ITE chains (unchanged when unassigned).
            for (unsigned j = 0; j < N_VARS; j++) {
                Z3_ast acc = 0;
                for (long t = (long)N_TRANS - 1; t >= 0; t--) {
                    if (ASGN_MAP[t][j]) {
                        Z3_ast e = ASGN_FNS[t][j](c, s_vars[i], vi, fldi);
                        acc = Z3_mk_ite(c, fires[t], e, acc ? acc : v_vars[(size_t)i * N_VARS + j]);
                    } else if (acc) {
                        acc = Z3_mk_ite(c, fires[t], v_vars[(size_t)i * N_VARS + j], acc);
                    }
                }
                Z3_ast next_v;
                if (acc) {
                    next_v = Z3_mk_ite(c, any_fire, acc, v_vars[(size_t)i * N_VARS + j]);
                } else {
                    next_v = Z3_mk_ite(c, any_fire, v_vars[(size_t)i * N_VARS + j], v_vars[(size_t)i * N_VARS + j]);
                }
                BASE_PUSH(Z3_mk_eq(c, next_v, v_vars[(size_t)(i + 1) * N_VARS + j]));
            }
        }
        free(enableds);
        free(fires);
    }

    // ---- Queries (fixed order: safety, then liveness; declaration order) ----
    for (unsigned p = 0; p < N_PROPS; p++) {
        if (PROP_KINDS[p] != 0) continue;
        // viol_p = ∃i<k: alive_i ∧ ¬phi_p(i).
        Z3_ast viol = 0;
        for (unsigned i = 0; i < k; i++) {
            Z3_ast phi = PROP_FNS[p](c, s_vars[i], &v_vars[(size_t)i * N_VARS], &fld_vars[(size_t)i * N_FIELDS]);
            Z3_ast pair[2] = { alive[i], Z3_mk_not(c, phi) };
            Z3_ast bad = Z3_mk_and(c, 2, pair);
            if (viol) {
                Z3_ast opair[2] = { viol, bad };
                viol = Z3_mk_or(c, 2, opair);
            } else {
                viol = bad;
            }
        }
        Z3_solver solver = Z3_mk_solver(c);
        for (size_t bi = 0; bi < base_n; bi++) Z3_solver_assert(c, solver, base[bi]);
        Z3_solver_assert(c, solver, viol);
        Z3_lbool r = Z3_solver_check(c, solver);
        if (r == Z3_L_TRUE) {
            Z3_model m = Z3_solver_get_model(c, solver);
            unsigned vstep = 0;
            for (unsigned i = 0; i < k; i++) {
                Z3_ast phi = PROP_FNS[p](c, s_vars[i], &v_vars[(size_t)i * N_VARS], &fld_vars[(size_t)i * N_FIELDS]);
                if (smt_read_bool(c, m, alive[i]) && !smt_read_bool(c, m, phi)) { vstep = i; break; }
            }
            printf("FAIL\n");
            printf("model-check summary: status=fail mode=smt-bounded bound=%u reason=property-violation label=%s step=%u\n", k, PROP_NAMES[p], vstep);
            printf("property violation: %s\n", PROP_NAMES[p]);
            printf("  violation step: %u\n", vstep);
            smt_print_trace(c, k, m, s_vars, v_vars, ev_vars, fld_vars);
            ret = 1; goto done;
        } else if (r == Z3_L_UNDEF) {
            printf("UNKNOWN (solver inconclusive within bound %u)\n", k);
            printf("model-check summary: status=unknown mode=smt-bounded bound=%u\n", k);
            ret = 2; goto done;
        }
    }
    for (unsigned p = 0; p < N_PROPS; p++) {
        if (PROP_KINDS[p] != 1) continue;
        // never_p = phi_p holds at no alive step within the bound.
        Z3_ast never = 0;
        for (unsigned i = 0; i < k; i++) {
            Z3_ast phi = PROP_FNS[p](c, s_vars[i], &v_vars[(size_t)i * N_VARS], &fld_vars[(size_t)i * N_FIELDS]);
            Z3_ast pair[2] = { alive[i], phi };
            Z3_ast n = Z3_mk_not(c, Z3_mk_and(c, 2, pair));
            if (never) {
                Z3_ast apair[2] = { never, n };
                never = Z3_mk_and(c, 2, apair);
            } else {
                never = n;
            }
        }
        // cycle = ∃a<b: identical (state, vars) - an infinite path.
        Z3_ast cyc = 0;
        for (unsigned a = 0; a < k; a++) {
            for (unsigned b = a + 1; b < k; b++) {
                Z3_ast eq = Z3_mk_eq(c, s_vars[a], s_vars[b]);
                for (unsigned j = 0; j < N_VARS; j++) {
                    Z3_ast pair[2] = { eq, Z3_mk_eq(c, v_vars[(size_t)a * N_VARS + j], v_vars[(size_t)b * N_VARS + j]) };
                    eq = Z3_mk_and(c, 2, pair);
                }
                if (cyc) {
                    Z3_ast opair[2] = { cyc, eq };
                    cyc = Z3_mk_or(c, 2, opair);
                } else {
                    cyc = eq;
                }
            }
        }
        if (cyc) {
            Z3_solver solver = Z3_mk_solver(c);
            for (size_t bi = 0; bi < base_n; bi++) Z3_solver_assert(c, solver, base[bi]);
            Z3_solver_assert(c, solver, never);
            Z3_solver_assert(c, solver, cyc);
            Z3_lbool r = Z3_solver_check(c, solver);
            if (r == Z3_L_TRUE) {
                Z3_model m = Z3_solver_get_model(c, solver);
                // Find the cycle pair from the model (first in order).
                unsigned pa = 0, pb = 1; int found = 0;
                for (unsigned a = 0; a < k && !found; a++) {
                    for (unsigned b = a + 1; b < k; b++) {
                        int same = smt_read_bv(c, m, s_vars[a]) == smt_read_bv(c, m, s_vars[b]);
                        for (unsigned j = 0; j < N_VARS && same; j++) {
                            same = smt_read_bv(c, m, v_vars[(size_t)a * N_VARS + j]) == smt_read_bv(c, m, v_vars[(size_t)b * N_VARS + j]);
                        }
                        if (same) { pa = a; pb = b; found = 1; break; }
                    }
                }
                printf("FAIL\n");
                printf("model-check summary: status=fail mode=smt-bounded bound=%u reason=liveness-violation label=%s step=%u\n", k, PROP_NAMES[p], pa);
                printf("liveness violation: %s\n", PROP_NAMES[p]);
                printf("  violation step: %u\n", pa);
                printf("  cycle length: %u\n", pb - pa);
                smt_print_trace(c, k, m, s_vars, v_vars, ev_vars, fld_vars);
                ret = 1; goto done;
            } else if (r == Z3_L_UNDEF) {
                printf("UNKNOWN (solver inconclusive within bound %u)\n", k);
                printf("model-check summary: status=unknown mode=smt-bounded bound=%u\n", k);
                ret = 2; goto done;
            }
        }
    }
    // No violation found within the bound (incomplete - no claim).
    printf("PASS WITHIN BOUND %u (incomplete - no completeness claim)\n", k);
    printf("model-check summary: status=pass-within-bound mode=smt-bounded bound=%u\n", k);

done:
    free(s_vars);
    free(v_vars);
    if (ev_vars) free(ev_vars);
    free(fld_vars);
    free(base);
    free(alive);
    Z3_del_context(c);
    return ret;
}

int main(int argc, char **argv) {
    return run_smt(argc, argv);
}
"#;
