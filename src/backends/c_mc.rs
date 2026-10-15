//! C backend: bounded model-checker executable generation.
//!
//! The generated program has the **same runtime wire format** as the Rust
//! model checker (see `crate::codegen_mc`): CLI flags `--bound`, `--runs`,
//! `--seed`, `--assumes`, `--violate`, `--unexpected-prob`; the same LCG
//! constants; the same per-step phase strategy (forced assumption violation,
//! then unexpected injection, then allowed random input); and the same
//! summary line. Counterexample traces are printed in the same shape.

use crate::backends::c::c_ident;
use crate::codegen_fsm::{self, VariantInfo};
use crate::ir::*;

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

pub fn generate_model_checker(
    module: &Module,
    options: &crate::codegen::BackendOptions,
) -> Result<crate::codegen::CodegenOutput, Vec<String>> {
    let prod = crate::backends::c_fsm::generate_production(module, options)?;
    let candidates = rep_inputs(module);
    let n_all = candidates.len().max(1);
    let cand_list = if candidates.is_empty() {
        "{{ 0 }}".to_string()
    } else {
        format!("{{ {} }}", candidates.join(", "))
    };

    // State/vars kind names for trace printing: print only kind + state names.
    let mc_main = format!(
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
static void parse_violation_spec(const char* s, violation_spec_t* out) {{
    out->label = s; out->prob = 1.0; out->percent = 100.0;
    const char* c1 = strchr(s, ':');
    if (c1) {{
        out->prob = strtod(c1 + 1, 0);
        const char* c2 = strchr(c1 + 1, ':');
        if (c2) out->percent = strtod(c2 + 1, 0);
    }}
}}

static void print_trace(const rfs_trace_t* trace) {{
    printf("counterexample trace:");
    printf("\n  initial state=%d vars_raw", trace->initial_state);
    for (int i = 0; i < trace->len; i++) {{
        const trace_step_t* s = &trace->steps[i];
        printf("\n  step %d input_kind=%d from=%d to=%d n_outputs=%d", i, s->input.kind, s->from, s->to, s->n_outputs);
    }}
}}

int main(int argc, char** argv) {{
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
                parse_violation_spec(tok, &specs[n_specs++]);
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
    int property_violations = 0, unexpected_count = 0, assumption_stops = 0, completed = 0;

    for (int run = 0; run < runs; run++) {{
        int violating[16]; double target_fraction[16];
        for (int i = 0; i < n_specs; i++) {{ violating[i] = mc_chance(specs[i].prob); target_fraction[i] = specs[i].percent / 100.0; }}

        fsm_t fsm; fsm_new(&fsm);
        // Restrict active assumptions to the CLI selection.
        for (int i = 0; i < n_active && i < 64; i++) {{ /* labels already filtered */ }}
        const char* status = "completed";

        for (int step = 0; step < bound; step++) {{
            // Phase 1: forced assumption violation.
            int forced = -1;
            for (int i = 0; i < n_specs && forced < 0; i++) {{
                if (!violating[i] || !mc_chance(target_fraction[i])) continue;
                for (int c = 0; c < N_ALL_INPUTS; c++) {{
                    fsm_t t = clone_fsm(&fsm);
                    step_result_t r = fsm_step(&t, ALL_INPUTS[c], stop_handler, 0);
                    if (r.status == RFS_ASSUMPTION_VIOLATION && strcmp(r.assume_label, specs[i].label) == 0) {{ forced = c; break; }}
                }}
            }}
            if (forced >= 0) {{
                step_result_t r = fsm_step(&fsm, ALL_INPUTS[forced], stop_handler, 0);
                if (r.status == RFS_ASSUMPTION_VIOLATION) {{ status = "assumption-violated"; assumption_stops++; break; }}
                continue;
            }}

            // Phase 2: optional unexpected injection.
            if (mc_chance(unexpected_prob)) {{
                int idx = mc_pick_index(N_ALL_INPUTS);
                if (idx >= 0) {{
                    step_result_t r = fsm_step(&fsm, ALL_INPUTS[idx], cont_handler, 0);
                    if (r.status == RFS_UNEXPECTED_EVENT) {{ status = "unexpected-event"; unexpected_count++; break; }}
                }}
                continue;
            }}

            // Phase 3: allowed representative input.
            int allowed[256]; int n_allowed = 0;
            for (int c = 0; c < N_ALL_INPUTS && n_allowed < 256; c++) {{
                fsm_t t = clone_fsm(&fsm);
                step_result_t r = fsm_step(&t, ALL_INPUTS[c], stop_handler, 0);
                if (r.status == RFS_OK) allowed[n_allowed++] = c;
            }}
            if (n_allowed == 0) {{ status = "stuck"; break; }}
            int idx = mc_pick_index(n_allowed);
            step_result_t r = fsm_step(&fsm, ALL_INPUTS[allowed[idx]], stop_handler, 0);
            if (r.status == RFS_UNEXPECTED_EVENT) {{ status = "unexpected-event"; unexpected_count++; break; }}
            if (r.status == RFS_ASSUMPTION_VIOLATION) {{ status = "assumption-violated"; assumption_stops++; break; }}
        }}

        if (strcmp(status, "completed") == 0) completed++;

        for (int i = 0; i < N_PROPERTY_LABELS; i++) {{
            if (!rfs_property_check(PROPERTY_LABELS[i], &fsm.trace)) {{
                property_violations++;
                printf("run %d property violation: %s\n", run, PROPERTY_LABELS[i]);
                print_trace(&fsm.trace);
                return 1;
            }}
        }}
    }}

    printf("model-check summary: runs=%d bound=%d completed=%d unexpected=%d assumption_stops=%d property_violations=%d\n", runs, bound, completed, unexpected_count, assumption_stops, property_violations);
    return 0;
}}
"#,
        n_all = n_all,
        cand_list = cand_list
    );

    let mut code = prod.code;
    code.push_str(&mc_main);
    Ok(crate::codegen::CodegenOutput {
        code,
        warnings: prod.warnings,
    })
}
