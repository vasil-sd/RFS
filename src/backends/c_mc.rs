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
    code.push_str(&exact_codegen_c(module, &plan, options).map_err(|e| vec![e])?);
    code.push_str(&fuzz_runtime(module));
    code.push_str(MAIN_DISPATCH_C);
    Ok(CodegenOutput {
        code,
        warnings: prod.warnings,
    })
}

/// `print_vars` + `print_trace`, shared by the exact and fuzz runtimes.
fn shared_printers(module: &Module) -> String {
    // Generated vars printer for counterexample output (type-aware).
    let vars_print = if module.vars.is_empty() {
        "static void print_vars(const vars_t* v) { (void)v; printf(\"vars()\\n\"); }\n".to_string()
    } else {
        let parts: Vec<String> = module
            .vars
            .iter()
            .enumerate()
            .map(|(i, v)| {
                let n = c_ident(&v.name);
                let sep = if i == 0 { "" } else { " " };
                match &v.ty {
                    Type::Bool => format!(
                        "printf(\"%s%s=%s\", \"{}\", \"{}\", v->{} ? \"true\" : \"false\");",
                        sep, n, n
                    ),
                    Type::Bit { .. } => format!(
                        "printf(\"%s%s=%llu\", \"{}\", \"{}\", (unsigned long long)v->{});",
                        sep, n, n
                    ),
                    Type::Enum { .. } => format!(
                        "printf(\"%s%s=%d\", \"{}\", \"{}\", (int)v->{});",
                        sep, n, n
                    ),
                }
            })
            .collect();
        format!(
            "static void print_vars(const vars_t* v) {{\n    printf(\"vars(\");\n{parts}\n    printf(\")\\n\");\n}}\n",
            parts = parts.join("\n    ")
        )
    };
    let trace_print = r#"static void print_trace(const rfs_trace_t* trace) {
    printf("counterexample trace:");
    printf("\n  initial state=%d vars_raw", trace->initial_state);
    for (int i = 0; i < trace->len; i++) {
        const trace_step_t* s = &trace->steps[i];
        printf("\n  step %d input_kind=%d from=%d to=%d n_outputs=%d", i, s->input.kind, s->from, s->to, s->n_outputs);
    }
}
"#;
    format!("{vars_print}{trace_print}")
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

/// Render the exact-mode C runtime from the plan (D-P4-1..6).
fn exact_codegen_c(
    module: &Module,
    plan: &McPlan,
    options: &BackendOptions,
) -> Result<String, String> {
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
    s.push_str(EXACT_RUN_C);
    Ok(s)
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

// Visited set: sorted vector of config byte strings + binary search. The
// behavior must not depend on this container (D-P4-2); task 10 may switch it.
static int mc_visited_find(const uint8_t* visited, int n_visited, const uint8_t* key) {
    int lo = 0, hi = n_visited;
    while (lo < hi) {
        int mid = (lo + hi) / 2;
        int c = memcmp(visited + (size_t)mid * CONFIG_TOTAL_BYTES, key, CONFIG_TOTAL_BYTES);
        if (c == 0) return mid;
        if (c < 0) lo = mid + 1; else hi = mid;
    }
    return -1;
}
static void mc_visited_insert(uint8_t* visited, int* n_visited, const uint8_t* key) {
    int lo = 0, hi = *n_visited;
    while (lo < hi) {
        int mid = (lo + hi) / 2;
        int c = memcmp(visited + (size_t)mid * CONFIG_TOTAL_BYTES, key, CONFIG_TOTAL_BYTES);
        if (c == 0) return;
        if (c < 0) lo = mid + 1; else hi = mid;
    }
    memmove(visited + (size_t)(lo + 1) * CONFIG_TOTAL_BYTES, visited + (size_t)lo * CONFIG_TOTAL_BYTES, (size_t)(*n_visited - lo) * CONFIG_TOTAL_BYTES);
    memcpy(visited + (size_t)lo * CONFIG_TOTAL_BYTES, key, CONFIG_TOTAL_BYTES);
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
        printf("  vars at trigger step: "); print_vars(&tv);
    }
    printf("  violation step: %d\n", f->prop_step);
    vars_t vv = trace_vars(&f->trace, f->prop_step);
    printf("  vars at violation step: "); print_vars(&vv);
    print_trace(&f->trace);
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
        mc_report_property(&init_fsm);
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
    if (!nodes || !parent_id || !parent_input || !parent_trans || !depth_arr || !visited) {
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
                    if (mc_visited_find(visited, n_visited, key) < 0) {
                        if (n_nodes >= max_states) { status = 2; goto done; }
                        mc_visited_insert(visited, &n_visited, key);
                        parent_id[n_nodes] = cid;
                        parent_input[n_nodes] = iidx;
                        parent_trans[n_nodes] = eff->transition_id;
                        depth_arr[n_nodes] = d + 1;
                        if (d + 1 > max_depth) max_depth = d + 1;
                        nodes[n_nodes++] = succ;
                    }
                }
            }
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
        if (is_deadlock) {
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
        printf("  vars at trigger step: "); print_vars(&f->trace.steps[f->prop_trigger_step - 1].vars);
    }}
    printf("  violation step: %d\n", f->prop_step);
    const vars_t* vv = (f->prop_step == 0) ? &f->trace.initial_vars : &f->trace.steps[f->prop_step - 1].vars;
    printf("  vars at violation step: "); print_vars(vv);
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
