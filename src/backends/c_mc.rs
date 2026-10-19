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
        ValueDomain::Enum { variants } => (
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
        ValueDomain::Enum { variants } => variants.len(),
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
    s.push_str("#include <time.h>\n");
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
    s.push_str(&format!(
        "#define CONFIG_TOTAL_BYTES {}\n",
        plan.encoding.total_bytes
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
                if let ValueDomain::Enum { variants } = f.domain.as_ref().unwrap() {
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
    const char* assumes_arg = "all";
    for (int i = 1; i < argc; i++) {
        if (strcmp(argv[i], "--max-states") == 0 && i + 1 < argc) max_states = atoi(argv[++i]);
        else if (strcmp(argv[i], "--nondet") == 0) nondet = 1;
        else if (strcmp(argv[i], "--timeout-sec") == 0 && i + 1 < argc) timeout_sec = atol(argv[++i]);
        else if (strcmp(argv[i], "--assumes") == 0 && i + 1 < argc) assumes_arg = argv[++i];
    }

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

    while (level_start < level_end && status == 0) {
        if (timeout_sec > 0 && mc_elapsed_sec(&t0) >= timeout_sec) { status = 3; break; }
        int next_start = n_nodes;
        for (int cid = level_start; cid < level_end; cid++) {
            mc_node_t node = nodes[cid];
            int d = depth_arr[cid];
            int any_successor = 0;
            for (int iidx = 0; iidx < n_inputs; iidx++) {
                input_t in = inputs[iidx];
                mc_effect_t effs[MC_MAX_EFFECTS];
                int n_eff = mc_step_all(node.state, &node.vars, in, effs);
                if (n_eff == 0) {
                    // Input matches no transition and there is no on_error:
                    // this path terminates (D-P4-5); the configuration itself
                    // may still have other successors.
                    unexpected++;
                    if (first_unexp_cid < 0) { first_unexp_cid = cid; first_unexp_idx = iidx; }
                    continue;
                }
                int n_sel = nondet ? n_eff : 1;
                for (int e = 0; e < n_sel; e++) {
                    const mc_effect_t* eff = &effs[e];
                    transitions++;
                    mc_node_t succ;
                    rfs_property_violation_t pv;
                    int r = mc_step_successor(&node, d, &in, eff, active, n_active, &succ, &pv);
                    if (r == 1) continue; // pruned by an active assumption
                    if (r == 2) {
                        status = 4;
                        fail_wire = "property-violation";
                        strncpy(fail_label, pv.label, sizeof(fail_label) - 1);
                        fail_label[sizeof(fail_label) - 1] = 0;
                        fail_step = pv.step;
                        n_fail_steps = d + 1;
                        fail_cid = (int*)malloc((size_t)n_fail_steps * sizeof(int));
                        fail_input = (int*)malloc((size_t)n_fail_steps * sizeof(int));
                        fail_trans = (int*)malloc((size_t)n_fail_steps * sizeof(int));
                        mc_fill_path(parent_id, parent_input, parent_trans, depth_arr, cid, fail_cid, fail_input, fail_trans);
                        fail_cid[d] = cid;
                        fail_input[d] = iidx;
                        fail_trans[d] = eff->transition_id;
                        goto done;
                    }
                    any_successor = 1;
                    uint8_t key[CONFIG_TOTAL_BYTES];
                    config_bytes(succ.state, &succ.vars, &succ.prop, &succ.assume, key);
                    int tcid = mc_visited_find(visited, visited_idx, n_visited, key);
                    if (tcid < 0) {
                        if (n_nodes >= max_states) { status = 2; goto done; }
                        mc_visited_insert(visited, visited_idx, &n_visited, key, n_nodes);
                        parent_id[n_nodes] = cid;
                        parent_input[n_nodes] = iidx;
                        parent_trans[n_nodes] = eff->transition_id;
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
                    adj_edges[adj_n].input_idx = iidx;
                    adj_edges[adj_n].trans_id = eff->transition_id;
                    adj_edges[adj_n].target = tcid;
                    adj_count[cid]++;
                    adj_n++;
                }
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
                goto done;
            }
        }
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
