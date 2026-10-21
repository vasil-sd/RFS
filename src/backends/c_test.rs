//! C backend: unit-test file generation (plain-assert harness, no framework).
//!
//! The `test` command emits the production FSM source plus a `main()` that
//! runs the same planned checks as the Rust backend's generated tests
//! (`codegen_test::TestPlan`): initial state, positive bounded sequence, and
//! either an unexpected-input search or an on_error landing check. Failures
//! are counted (not aborting) so one run reports every broken check.

use crate::backends::c::c_ident;
use crate::codegen_fsm::{self, FsmVariants, VariantInfo};
use crate::ir::*;

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

pub fn generate_unit_tests(
    module: &Module,
    options: &crate::codegen::BackendOptions,
) -> Result<crate::codegen::CodegenOutput, Vec<String>> {
    if options.no_std {
        return Err(vec![
            crate::codegen::NO_STD_PRODUCTION_ONLY_ERROR.to_string()
        ]);
    }
    // The generated tests reuse the production FSM core. Property monitors
    // are always included (D-P3-6), so a spec that violates its own property
    // fails its generated tests.
    let prod = crate::backends::c_fsm::generate_core(
        module,
        options,
        crate::codegen::PropertyMonitorMode::Always,
    )?;
    let variants = codegen_fsm::build_variants(module);
    let plan = crate::codegen_test::build_test_plan(&variants, module, options.steps);

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

    let harness = format!(
        "\n// ---- generated unit tests (plain-assert harness) ----\n\
         static int rfs_test_failures = 0;\n\
         #define CHECK(cond, msg) do {{ if (!(cond)) {{ rfs_test_failures++; printf(\"FAIL: %s\\n\", msg); }} }} while (0)\n\
         static rfs_policy_t stop_handler(void* user_data, const char* label) {{ (void)user_data; (void)label; return RFS_STOP; }}\n\
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
    );

    let positive = if candidates.is_empty() {
        "    // no inputs declared; nothing to drive\n".to_string()
    } else {
        format!(
            "    {{\n        fsm_t f; fsm_new(&f);\n        int steps = 0;\n        for (int i = 0; i < {k}; i++) {{\n            int progressed = 0;\n            for (int c = 0; c < N_ALL_INPUTS; c++) {{\n                fsm_t t = clone_fsm(&f);\n                step_result_t r = fsm_step(&t, ALL_INPUTS[c], stop_handler, 0);\n                if (r.status == RFS_OK) {{ f = t; steps++; progressed = 1; break; }}\n            }}\n            if (!progressed) break;\n        }}\n        CHECK(steps > 0 || !EXPECT_PROGRESS, \"positive sequence made no progress\");\n    }}\n"
        )
    };

    let initial_state_block = format!(
        "    {{\n        fsm_t f; fsm_new(&f);\n        CHECK(f.state == STATE_{}, \"initial state\");\n    }}\n",
        c_ident(&init_state).to_uppercase()
    );

    let negative = render_negative(&plan, &variants, module, k);

    let closing = "    if (rfs_test_failures == 0) { printf(\"all generated C tests passed\\n\"); return 0; }\n    printf(\"%d generated C test(s) failed\\n\", rfs_test_failures);\n    return 1;\n}\n";

    let mut code = prod.code;
    code.push_str(&harness);
    code.push_str("int main(void) {\n");
    code.push_str(&initial_state_block);
    code.push_str(&positive);
    code.push_str(&negative);
    code.push_str(closing);

    Ok(crate::codegen::CodegenOutput {
        code,
        warnings: prod.warnings,
        extra_files: Vec::new(),
    })
}

fn render_negative(
    plan: &crate::codegen_test::TestPlan,
    variants: &FsmVariants,
    module: &Module,
    k: usize,
) -> String {
    let definite = crate::codegen_test::has_definite_unexpected(variants, module);
    if plan.has_on_error {
        let oe = module.on_error.as_ref().unwrap();
        let assert_line = if definite {
            "CHECK(on_error_found, \"on_error fallback was never reached\");".to_string()
        } else {
            "// No statically obvious unexpected representative input.".to_string()
        };
        format!(
            "    {{\n        fsm_t f; fsm_new(&f);\n        int on_error_found = 0;\n        for (int i = 0; i < {k}; i++) {{\n            for (int c = 0; c < N_ALL_INPUTS; c++) {{\n                fsm_t t = clone_fsm(&f);\n                step_result_t r = fsm_step(&t, ALL_INPUTS[c], stop_handler, 0);\n                if (r.status == RFS_UNEXPECTED_EVENT) break;\n                if (t.state == STATE_{oe_state}) {{ on_error_found = 1; break; }}\n            }}\n            if (on_error_found) break;\n            int progressed = 0;\n            for (int c = 0; c < N_ALL_INPUTS; c++) {{\n                fsm_t t = clone_fsm(&f);\n                step_result_t r = fsm_step(&t, ALL_INPUTS[c], stop_handler, 0);\n                if (r.status == RFS_OK) {{ f = t; progressed = 1; break; }}\n            }}\n            if (!progressed) break;\n        }}\n        {assert_line}\n    }}\n",
            oe_state = c_ident(&oe.next_state).to_uppercase()
        )
    } else if plan.definite_unexpected {
        format!(
            "    {{\n        fsm_t f; fsm_new(&f);\n        int found = 0;\n        for (int i = 0; i < {k}; i++) {{\n            for (int c = 0; c < N_ALL_INPUTS; c++) {{\n                fsm_t t = clone_fsm(&f);\n                step_result_t r = fsm_step(&t, ALL_INPUTS[c], stop_handler, 0);\n                if (r.status == RFS_UNEXPECTED_EVENT) {{ found = 1; break; }}\n            }}\n            if (found) break;\n            int progressed = 0;\n            for (int c = 0; c < N_ALL_INPUTS; c++) {{\n                fsm_t t = clone_fsm(&f);\n                step_result_t r = fsm_step(&t, ALL_INPUTS[c], stop_handler, 0);\n                if (r.status == RFS_OK) {{ f = t; progressed = 1; break; }}\n            }}\n            if (!progressed) break;\n        }}\n        CHECK(found, \"expected an unexpected-event failure\");\n    }}\n"
        )
    } else {
        "    // No statically obvious unexpected representative input.\n".to_string()
    }
}
