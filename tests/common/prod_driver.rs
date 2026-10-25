//! Shared production-FSM driver infrastructure for the Phase 6 task 4
//! differential suites (4.3 prod parity, 4.4 monitor references, 4.5 PSL
//! translation).
//!
//! For one spec this module generates TWO thin drivers with an IDENTICAL
//! stdout contract - a Rust driver (the generated production artifact plus
//! per-spec glue + `main`, concatenated into one file) and a C driver
//! (`#include` of the generated .c artifact plus per-spec glue + `main`) -
//! plus a deterministic input script. Both drivers read the same script from
//! stdin (one input per line: `Variant|f0,f1,...` or bare `Variant`) and
//! print, byte-identically:
//!
//! ```text
//! init st={State} vars={v0},{v1},...        (vars = "-" when the module has none)
//! S{i} in={In} out={Outs} st={From}->{To} vars={...}   (committed step; out = "-" when empty)
//! S{i} in={In} UNEXPECTED                   (no transition, no on_error)
//! S{i} in={In} out={Outs} ASSUME {label} st={From}->{To} vars={...}
//!                                          (assumption stop - Stop policy
//!                                           handler; the step is RECORDED in
//!                                           the trace but NOT applied to
//!                                           state/vars)
//! S{i} in={In} out={Outs} PROP {label} step={n} st={From}->{To} vars={...}
//!                                          (first property violation; same
//!                                           record-but-not-apply semantics)
//! M{idx} OK | M{idx} FAIL {label} step={n} trig={t|-1}  (probe: fresh monitors over the committed trace)
//! FIRST none | FIRST {label} step={n} trig={t|-1}       (the FSM's own recorded first violation)
//! ```
//!
//! Raw-value conventions (engine-renderer-independent, so both drivers agree
//! by construction): states = bare variant names; variables/fields in
//! declaration order as raw integers (bool 0/1, bit<N> decimal, enum =
//! variant index); outputs `;`-separated.

use std::path::{Path, PathBuf};
use std::process::Command;

/// gcc location (same assumption as the other suites).
pub const GCC: &str = "/usr/bin/gcc";

/// rustc override (same convention as backend_parity_test).
fn rustc() -> String {
    std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_string())
}

/// One flattened input/output variant of a module (declaration order).
#[derive(Debug, Clone)]
pub struct FlatVariant {
    /// Collision-free name (single-variant same-name group: bare; else
    /// `Group_Variant`) - the same names the artifacts use.
    pub name: String,
    /// The constructor group this variant belongs to (read by the PSL
    /// translation suite's pattern matcher).
    #[allow(dead_code)]
    pub group: String,
    /// The variant's own name (without the group prefix) (read by the PSL
    /// translation suite's pattern matcher).
    #[allow(dead_code)]
    pub variant_name: String,
    /// (field name, field type) in declaration order.
    pub fields: Vec<(String, rfs::Type)>,
}

/// Flatten constructor groups into variants (the backend naming convention).
pub fn flatten_groups(groups: &[rfs::ConstructorGroup]) -> Vec<FlatVariant> {
    let mut out = Vec::new();
    let mut used: std::collections::HashSet<String> = std::collections::HashSet::new();
    for group in groups {
        for v in &group.variants {
            let (vname, fields) = match v {
                rfs::ConstructorVariant::Unit { name } => (name.clone(), vec![]),
                rfs::ConstructorVariant::Struct { name, fields } => (
                    name.clone(),
                    fields
                        .iter()
                        .map(|f| (f.name.clone(), f.ty.clone()))
                        .collect(),
                ),
            };
            let base = if group.name == vname && group.variants.len() == 1 {
                group.name.clone()
            } else {
                format!("{}_{}", group.name, vname)
            };
            let mut unique = base.clone();
            let mut n = 2;
            while !used.insert(unique.clone()) {
                unique = format!("{base}{n}");
                n += 1;
            }
            out.push(FlatVariant {
                name: unique,
                group: group.name.clone(),
                variant_name: vname,
                fields,
            });
        }
    }
    out
}

/// The module's input variants (flattened events).
pub fn flattened_inputs(module: &rfs::Module) -> Vec<FlatVariant> {
    flatten_groups(&module.events)
}

/// The module's output variants (flattened outputs).
pub fn flattened_outputs(module: &rfs::Module) -> Vec<FlatVariant> {
    flatten_groups(&module.outputs)
}

/// State variant names (the `State` enum).
pub fn state_variants(module: &rfs::Module) -> Vec<String> {
    module
        .enums
        .iter()
        .find(|e| e.name == "State")
        .map(|e| e.variants.clone())
        .unwrap_or_default()
}

/// Rust integer type for a bit width (same mapping as the Rust backend).
pub fn bit_rust_type(w: u32) -> &'static str {
    match w {
        0..=8 => "u8",
        9..=16 => "u16",
        17..=32 => "u32",
        33..=64 => "u64",
        _ => "u128",
    }
}

/// C integer type for a bit width (same mapping as the C backend).
pub fn bit_c_type(w: u32) -> &'static str {
    match w {
        0..=8 => "uint8_t",
        9..=16 => "uint16_t",
        17..=32 => "uint32_t",
        _ => "uint64_t",
    }
}

/// C type of a payload/variable field.
fn c_field_ty(ty: &rfs::Type) -> String {
    match ty {
        rfs::Type::Bool => "bool".to_string(),
        rfs::Type::Bit { width } => bit_c_type(*width).to_string(),
        rfs::Type::Enum { name } => format!("{name}_t"),
    }
}

/// Deterministic input script (the LCG lineage of task 3 / the fuzz wire
/// format): one line per step, variant picked by the RNG, field values drawn
/// from each field's domain. The SAME bytes are fed to both drivers.
pub fn input_script(module: &rfs::Module, seed: u64, steps: usize) -> String {
    let variants = flattened_inputs(module);
    if variants.is_empty() {
        return String::new();
    }
    let mut rng = rfs::specgen::Rng::new(seed);
    let mut out = String::new();
    for _ in 0..steps {
        let v = rng.pick(&variants).expect("non-empty variant list");
        let fields: Vec<String> = v
            .fields
            .iter()
            .map(|(_, ty)| match ty {
                rfs::Type::Bool => (rng.next_u64() % 2).to_string(),
                rfs::Type::Bit { width } => {
                    let span = if *width >= 64 {
                        u64::MAX
                    } else {
                        1u64 << width
                    };
                    (rng.next_u64() % span).to_string()
                }
                rfs::Type::Enum { name } => {
                    let n = module
                        .enums
                        .iter()
                        .find(|e| e.name == *name)
                        .map(|e| e.variants.len())
                        .unwrap_or(1);
                    (rng.next_u64() % n as u64).to_string()
                }
            })
            .collect();
        if fields.is_empty() {
            out.push_str(&format!("{}\n", v.name));
        } else {
            out.push_str(&format!("{}|{}\n", v.name, fields.join(",")));
        }
    }
    out
}

/// Per-enum variant-index helper name (shared by the Rust and C glue).
fn enum_helper_name(enum_name: &str) -> String {
    format!("rfs_pp_enum_idx_{enum_name}")
}

/// The Rust driver source: the production artifact verbatim (its inner
/// `#![allow]` stays valid at file top) + generated glue + `main`.
///
/// NOTE: all glue text is built by concatenation (no format! over templates
/// containing braces) so the emitted Rust compiles as-is.
pub fn rust_driver_source(module: &rfs::Module, artifact: &str) -> String {
    let mut g = String::new();
    g.push_str("use std::io::BufRead;\n");

    // --- enum variant-index helpers. ---
    for en in &module.enums {
        let arms: Vec<String> = en
            .variants
            .iter()
            .enumerate()
            .map(|(i, v)| format!("{}::{} => {},", en.name, v, i))
            .collect();
        g.push_str(&format!(
            "fn {}(v: &{}) -> u64 {{ match v {{ \n{}\n    }} }}\n",
            enum_helper_name(&en.name),
            en.name,
            arms.join(" ")
        ));
    }

    // --- input parser. ---
    let mut arms = String::new();
    for v in flattened_inputs(module) {
        if v.fields.is_empty() {
            arms.push_str(&format!("    \"{}\" => Input::{},\n", v.name, v.name));
        } else {
            let fields: Vec<String> = v
                .fields
                .iter()
                .enumerate()
                .map(|(i, (fname, ty))| match ty {
                    rfs::Type::Bool => format!("{fname}: (f[{i}] != 0)"),
                    rfs::Type::Bit { width } => {
                        format!("{fname}: f[{i}] as {}", bit_rust_type(*width))
                    }
                    rfs::Type::Enum { name } => format!("{fname}: f[{i}] as u64 as {name}"),
                })
                .collect();
            arms.push_str(&format!(
                "    \"{}\" => Input::{0} {{ {1} }},\n",
                v.name,
                fields.join(", ")
            ));
        }
    }
    g.push_str("fn rfs_pp_parse_input(name: &str, f: &[u64]) -> Input {\n");
    g.push_str(&format!("    match name {{\n{arms}"));
    g.push_str("        _ => panic!(\"unknown input variant {}\", name),\n    }\n}\n");

    // --- raw variable renderer (N placeholders, N args - no nested format). ---
    if module.vars.is_empty() {
        g.push_str("fn rfs_pp_render_vars(v: &Vars) -> String {\n    let _ = v;\n    \"-\".to_string()\n}\n");
    } else {
        let var_exprs: Vec<String> = module
            .vars
            .iter()
            .map(|v| match &v.ty {
                rfs::Type::Enum { name } => format!("{}(&v.{})", enum_helper_name(name), v.name),
                rfs::Type::Bool => format!("v.{} as u64", v.name),
                rfs::Type::Bit { .. } => format!("v.{}", v.name),
            })
            .collect();
        // Comma-joined (no spaces) - the C driver's rendering convention.
        let inner = std::iter::repeat_n("{}", var_exprs.len())
            .collect::<Vec<_>>()
            .join(",");
        g.push_str("fn rfs_pp_render_vars(v: &Vars) -> String { format!(\"");
        g.push_str(&inner);
        g.push_str("\", ");
        g.push_str(&var_exprs.join(", "));
        g.push_str(") }");
        g.push('\n');
    }

    // --- raw output renderer. ---
    let mut out_arms = String::new();
    for v in flattened_outputs(module) {
        if v.fields.is_empty() {
            out_arms.push_str(&format!(
                "    Output::{0} => \"{1}\".to_string(),\n",
                v.name, v.name
            ));
        } else {
            let pat_fields: Vec<String> = v.fields.iter().map(|(f, _)| f.clone()).collect();
            let exprs: Vec<String> = v
                .fields
                .iter()
                .map(|(fname, ty)| match ty {
                    rfs::Type::Enum { name } => format!("{}(&{fname})", enum_helper_name(name)),
                    rfs::Type::Bool => format!("{fname} as u64"),
                    rfs::Type::Bit { .. } => fname.clone(),
                })
                .collect();
            let inner = format!("{}|{}", v.name, "{}".repeat(exprs.len()));
            out_arms.push_str(&format!(
                "    Output::{0} {{ {1} }} => format!(\"{2}\", {3}),\n",
                v.name,
                pat_fields.join(", "),
                inner,
                exprs.join(", ")
            ));
        }
    }
    g.push_str("fn rfs_pp_render_output(o: &Output) -> String {\n");
    g.push_str(&format!("    match o {{\n{out_arms}"));
    g.push_str("        _ => panic!(\"unreachable output\"),\n    }\n}\n");

    // --- main. ---
    // `StepError::PropertyViolation` exists in every artifact (the enum is
    // unconditional), so the arm must exist even for property-less modules.

    let mut m = String::new();
    m.push_str("fn main() {\n");
    m.push_str("    let mut fsm = Fsm::new();\n");
    m.push_str(
        "    println!(\"init st={:?} vars={}\", fsm.state, rfs_pp_render_vars(&fsm.vars));\n",
    );
    m.push_str("    let mut handler = StopAssumptionHandler;\n");
    m.push_str("    let stdin = std::io::stdin();\n");
    m.push_str("    let mut i = 0usize;\n");
    m.push_str("    for line in stdin.lock().lines() {\n");
    m.push_str("        let line = line.expect(\"read stdin\");\n");
    m.push_str("        if line.trim().is_empty() { continue; }\n");
    m.push_str("        i += 1;\n");
    m.push_str("        let (name, fields) = rfs_pp_parse_input_line(&line);\n");
    m.push_str("        let input = rfs_pp_parse_input(name, &fields);\n");
    m.push_str("        let in_s = render_input_line(name, &fields);\n");
    m.push_str("        match fsm.step_with_handler(input, &mut handler) {\n");
    m.push_str("            Ok(_) => {\n");
    m.push_str("                let last = fsm.trace.steps.last().expect(\"step committed\");\n");
    m.push_str(
        "                let outs_s = if last.outputs.is_empty() { \"-\".to_string() } else {\n",
    );
    m.push_str("                    last.outputs.iter().map(rfs_pp_render_output).collect::<Vec<_>>().join(\";\")\n");
    m.push_str("                };\n");
    m.push_str("                println!(\"S{i} in={in_s} out={outs_s} st={:?}->{:?} vars={}\", last.from_state, last.to_state, rfs_pp_render_vars(&last.vars));\n");
    m.push_str("            }\n");
    m.push_str("            Err(e) => {\n");
    m.push_str("                match e {\n");
    m.push_str("                    StepError::UnexpectedEvent => println!(\"S{i} in={in_s} UNEXPECTED\"),\n");
    // ASSUME/PROP stops are recorded in the trace (commit_step pushes
    // before checking) but not applied - echo the recorded sample so the
    // line is parseable as a trace sample.
    m.push_str("                    StepError::AssumptionViolation { label } => { let last = fsm.trace.steps.last().expect(\"step recorded\"); let outs_s = if last.outputs.is_empty() { \"-\".to_string() } else { last.outputs.iter().map(rfs_pp_render_output).collect::<Vec<_>>().join(\";\") }; println!(\"S{i} in={in_s} out={outs_s} ASSUME {label} st={:?}->{:?} vars={}\", last.from_state, last.to_state, rfs_pp_render_vars(&last.vars)); }\n");
    m.push_str("                    StepError::PropertyViolation { label, step } => { let last = fsm.trace.steps.last().expect(\"step recorded\"); let outs_s = if last.outputs.is_empty() { \"-\".to_string() } else { last.outputs.iter().map(rfs_pp_render_output).collect::<Vec<_>>().join(\";\") }; println!(\"S{i} in={in_s} out={outs_s} PROP {label} step={step} st={:?}->{:?} vars={}\", last.from_state, last.to_state, rfs_pp_render_vars(&last.vars)); }\n");
    m.push_str("                }\n");
    m.push_str("            }\n");
    m.push_str("        }\n");
    m.push_str("    }\n");
    // Probe section: fresh property monitors over the committed trace.
    m.push_str("    {\n");
    m.push_str("        let mut mon = PropMonitors::new();\n");
    m.push_str("        let s0 = Sample { input: None, outputs: Vec::new(), from_state: fsm.trace.initial_state, to_state: fsm.trace.initial_state, vars: fsm.trace.initial_vars.clone() };\n");
    m.push_str("        match mon.step(&s0, 0) {\n");
    m.push_str("            Some(pv) => println!(\"M0 FAIL {} step={} trig={}\", pv.label, pv.step, trig_str(pv.trigger_step)),\n");
    m.push_str("            None => println!(\"M0 OK\"),\n");
    m.push_str("        }\n");
    m.push_str("        for (k, st) in fsm.trace.steps.iter().enumerate() {\n");
    m.push_str("            let s = Sample { input: Some(st.input.clone()), outputs: st.outputs.clone(), from_state: st.from_state, to_state: st.to_state, vars: st.vars.clone() };\n");
    m.push_str("            match mon.step(&s, k + 1) {\n");
    m.push_str("                Some(pv) => println!(\"M{} FAIL {} step={} trig={}\", k + 1, pv.label, pv.step, trig_str(pv.trigger_step)),\n");
    m.push_str("                None => println!(\"M{} OK\", k + 1),\n");
    m.push_str("            }\n");
    m.push_str("        }\n");
    m.push_str("    }\n");
    m.push_str("    if let Some(pv) = &fsm.property_violation {\n");
    m.push_str("        println!(\"FIRST {} step={} trig={}\", pv.label, pv.step, trig_str(pv.trigger_step));\n");
    m.push_str("    } else {\n");
    m.push_str("        println!(\"FIRST none\");\n");
    m.push_str("    }\n");
    m.push_str("}\n");
    m.push_str("fn trig_str(t: Option<usize>) -> String { t.map(|x| x.to_string()).unwrap_or_else(|| \"-1\".to_string()) }\n");
    m.push_str("fn rfs_pp_parse_input_line(line: &str) -> (&str, Vec<u64>) {\n");
    m.push_str("    let line = line.trim();\n");
    m.push_str("    match line.split_once('|') {\n");
    m.push_str("        Some((name, rest)) => (name, rest.split(',').map(|s| s.parse::<u64>().expect(\"field value\")).collect()),\n");
    m.push_str("        None => (line, Vec::new()),\n");
    m.push_str("    }\n");
    m.push_str("}\n");
    m.push_str("fn render_input_line(name: &str, fields: &[u64]) -> String {\n");
    m.push_str("    if fields.is_empty() { name.to_string() } else { format!(\"{}|{}\", name, fields.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(\",\")) }\n");
    m.push_str("}\n");
    g.push_str(&m);

    let mut out = artifact.to_string();
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out.push('\n');
    out.push_str(&g);
    out
}

/// The C driver source: `#include` of the production artifact + generated
/// glue + `main`. `artifact_file` is the file name (no directory) of the
/// artifact, written next to this driver.
/// The C kind-enum constant for a flattened variant (`INPUT_TICK`).
fn c_kind_name(prefix: &str, unique: &str) -> String {
    format!("{prefix}_{}", unique.to_uppercase())
}

/// The C payload-union member name for a flattened variant (`input_tick`).
/// Payload union member name. Same convention as the C backend (c_fsm.rs):
/// `{prefix}_` + lowercased unique name, underscores preserved.
fn c_payload_member(prefix: &str, unique: &str) -> String {
    format!("{prefix}_{}", unique.to_lowercase())
}

fn c_union_member(unique: &str) -> String {
    c_payload_member("input", unique)
}

pub fn c_driver_source(module: &rfs::Module, artifact_file: &str) -> String {
    let mut g = String::new();
    g.push_str(&format!("#include \"{artifact_file}\"\n"));
    g.push_str("#include <stdio.h>\n#include <stdlib.h>\n#include <string.h>\n");

    // --- input parser. ---
    let mut arms = String::new();
    for v in flattened_inputs(module) {
        if v.fields.is_empty() {
            arms.push_str(&format!(
                "    if (strcmp(name, \"{}\") == 0) return (input_t){{ .kind = {} }};\n",
                v.name,
                c_kind_name("INPUT", &v.name)
            ));
        } else {
            let fields: Vec<String> = v
                .fields
                .iter()
                .enumerate()
                .map(|(i, (_, ty))| format!("({})f[{i}]", c_field_ty(ty)))
                .collect();
            arms.push_str(&format!(
                "    if (strcmp(name, \"{}\") == 0) return (input_t){{ .kind = {}, .payload.{} = {{ {} }} }};\n",
                v.name,
                c_kind_name("INPUT", &v.name),
                c_union_member(&v.name),
                fields.join(", ")
            ));
        }
    }
    g.push_str("static input_t rfs_pp_parse_input(const char* name, const uint64_t* f) {\n");
    g.push_str(&arms);
    g.push_str("    (void)f; fprintf(stderr, \"unknown input variant %s\\n\", name); abort();\n");
    g.push_str("}\n");

    // --- state name renderer. ---
    let mut st_arms = String::new();
    for (i, s) in state_variants(module).iter().enumerate() {
        st_arms.push_str(&format!(
            "    case STATE_{}: return \"{}\";\n",
            s.to_uppercase(),
            s
        ));
        let _ = i;
    }
    g.push_str("static const char* rfs_pp_state_name(state_t s) {\n    switch (s) {\n");
    g.push_str(&st_arms);
    g.push_str("    default: return \"?\";\n    }\n}\n");

    // --- raw variable renderer. ---
    if module.vars.is_empty() {
        g.push_str("static void rfs_pp_render_vars(const vars_t* v, char* buf, size_t n) { (void)v; snprintf(buf, n, \"-\"); }\n");
    } else {
        let exprs: Vec<String> = module
            .vars
            .iter()
            .map(|v| format!("(unsigned long long)v->{}", v.name))
            .collect();
        let fmt = std::iter::repeat_n("%llu", exprs.len())
            .collect::<Vec<_>>()
            .join(",");
        g.push_str("static void rfs_pp_render_vars(const vars_t* v, char* buf, size_t n) {\n");
        g.push_str(&format!(
            "    snprintf(buf, n, \"{fmt}\", {});\n",
            exprs.join(", ")
        ));
        g.push_str("}\n");
    }

    // --- raw output renderer. ---
    let mut out_arms = String::new();
    for v in flattened_outputs(module) {
        if v.fields.is_empty() {
            out_arms.push_str(&format!(
                "        case {}: snprintf(buf, n, \"{}\"); break;\n",
                c_kind_name("OUTPUT", &v.name),
                v.name
            ));
        } else {
            let exprs: Vec<String> = v
                .fields
                .iter()
                .map(|(fname, _)| {
                    format!(
                        "(unsigned long long)o->payload.{}.{}",
                        c_payload_member("output", &v.name),
                        fname
                    )
                })
                .collect();
            let inner = format!("{}|{}", v.name, "%llu".repeat(exprs.len()));
            out_arms.push_str(&format!(
                "        case {}: snprintf(buf, n, \"{inner}\", {}); break;\n",
                c_kind_name("OUTPUT", &v.name),
                exprs.join(", ")
            ));
        }
    }
    g.push_str("static void rfs_pp_render_output(const output_t* o, char* buf, size_t n) {\n");
    g.push_str("    switch (o->kind) {\n");
    g.push_str(&out_arms);
    g.push_str("        default: snprintf(buf, n, \"?\"); break;\n    }\n}\n");

    // --- stop handler + main. ---
    let mut m = String::new();
    m.push_str("static rfs_policy_t rfs_pp_stop_handler(void* u, const char* label) { (void)u; (void)label; return RFS_STOP; }\n");
    m.push_str("int main(void) {\n");
    m.push_str("    fsm_t f; fsm_new(&f);\n");
    m.push_str("    char vb[4096]; rfs_pp_render_vars(&f.vars, vb, sizeof vb);\n");
    m.push_str("    printf(\"init st=%s vars=%s\\n\", rfs_pp_state_name(f.state), vb);\n");
    m.push_str("    char line[8192];\n");
    m.push_str("    int i = 0;\n");
    m.push_str("    while (fgets(line, sizeof line, stdin)) {\n");
    m.push_str("        size_t len = strlen(line); while (len > 0 && (line[len-1] == '\\n' || line[len-1] == '\\r')) line[--len] = 0;\n");
    m.push_str("        if (len == 0) continue;\n");
    m.push_str("        i++;\n");
    m.push_str("        char* bar = strchr(line, '|');\n");
    m.push_str("        const char* name = line; uint64_t fields[64]; int nf = 0;\n");
    m.push_str("        if (bar) { *bar = 0; char* p = bar + 1; char* tok = strtok(p, \",\"); while (tok && nf < 64) { fields[nf++] = strtoull(tok, 0, 10); tok = strtok(0, \",\"); } }\n");
    m.push_str("        input_t in = rfs_pp_parse_input(name, fields);\n");
    m.push_str("        step_result_t r = fsm_step(&f, in, rfs_pp_stop_handler, 0);\n");
    // Stream the input echo directly (no intermediate buffer: -Wformat-truncation).
    m.push_str("        printf(\"S%d in=\", i); if (bar) { printf(\"%s|\", name); for (int k = 0; k < nf; k++) printf(\"%llu%s\", (unsigned long long)fields[k], k + 1 < nf ? \",\" : \"\"); } else { printf(\"%s\", name); }\n");
    m.push_str("        if (r.status == RFS_OK) {\n");
    m.push_str("            const trace_step_t* last = &f.trace.steps[f.trace.len - 1];\n");
    m.push_str("            char vb2[4096]; rfs_pp_render_vars(&last->vars, vb2, sizeof vb2);\n");
    m.push_str("            if (last->n_outputs == 0) { printf(\" out=- st=%s->%s vars=%s\\n\", rfs_pp_state_name(last->from), rfs_pp_state_name(last->to), vb2); } else { printf(\" out=\"); for (int k = 0; k < last->n_outputs; k++) { char tmp[512]; rfs_pp_render_output(&last->outputs[k], tmp, sizeof tmp); if (k > 0) printf(\";\"); printf(\"%s\", tmp); } printf(\" st=%s->%s vars=%s\\n\", rfs_pp_state_name(last->from), rfs_pp_state_name(last->to), vb2); }\n");
    m.push_str("        } else if (r.status == RFS_UNEXPECTED_EVENT) {\n");
    m.push_str("            printf(\" UNEXPECTED\\n\");\n");
    // ASSUME/PROP stops are recorded in the trace (commit pushes before
    // checking) but not applied - echo the recorded sample so the line is
    // parseable as a trace sample.
    m.push_str("        } else if (r.status == RFS_ASSUMPTION_VIOLATION) {\n");
    m.push_str("            const trace_step_t* plast = &f.trace.steps[f.trace.len - 1]; char pvb[4096]; rfs_pp_render_vars(&plast->vars, pvb, sizeof pvb); printf(\" out=\"); if (plast->n_outputs == 0) { printf(\"-\"); } else { for (int k = 0; k < plast->n_outputs; k++) { char tmp[512]; rfs_pp_render_output(&plast->outputs[k], tmp, sizeof tmp); if (k > 0) printf(\";\"); printf(\"%s\", tmp); } } printf(\" ASSUME %s st=%s->%s vars=%s\\n\", r.assume_label, rfs_pp_state_name(plast->from), rfs_pp_state_name(plast->to), pvb);\n");
    m.push_str("        } else {\n");
    m.push_str("            const trace_step_t* plast = &f.trace.steps[f.trace.len - 1]; char pvb[4096]; rfs_pp_render_vars(&plast->vars, pvb, sizeof pvb); printf(\" out=\"); if (plast->n_outputs == 0) { printf(\"-\"); } else { for (int k = 0; k < plast->n_outputs; k++) { char tmp[512]; rfs_pp_render_output(&plast->outputs[k], tmp, sizeof tmp); if (k > 0) printf(\";\"); printf(\"%s\", tmp); } } printf(\" PROP %s step=%d st=%s->%s vars=%s\\n\", r.prop_label, r.prop_step, rfs_pp_state_name(plast->from), rfs_pp_state_name(plast->to), pvb);\n");
    m.push_str("        }\n");
    m.push_str("    }\n");
    // Probe section: fresh property monitors over the committed trace.
    m.push_str("    prop_monitors_t mon; prop_monitors_new(&mon);\n");
    m.push_str("    rfs_property_violation_t pv;\n");
    m.push_str("    rfs_sample_t s0 = { 0, 0, 0, f.trace.initial_state, f.trace.initial_state, f.trace.initial_vars };\n");
    m.push_str("    if (prop_monitors_step(&mon, &s0, 0, &pv)) printf(\"M0 FAIL %s step=%d trig=%d\\n\", pv.label, pv.step, pv.trigger_step); else printf(\"M0 OK\\n\");\n");
    m.push_str("    for (int k = 0; k < f.trace.len; k++) {\n");
    m.push_str("        const trace_step_t* st = &f.trace.steps[k];\n");
    m.push_str("        rfs_sample_t s = { &st->input, st->outputs, st->n_outputs, st->from, st->to, st->vars };\n");
    m.push_str("        if (prop_monitors_step(&mon, &s, k + 1, &pv)) printf(\"M%d FAIL %s step=%d trig=%d\\n\", k + 1, pv.label, pv.step, pv.trigger_step); else printf(\"M%d OK\\n\", k + 1);\n");
    m.push_str("    }\n");
    m.push_str("    if (f.prop_label[0] != 0) printf(\"FIRST %s step=%d trig=%d\\n\", f.prop_label, f.prop_step, f.prop_trigger_step); else printf(\"FIRST none\\n\");\n");
    m.push_str("    fsm_free(&f);\n");
    m.push_str("    return 0;\n");
    m.push_str("}\n");
    g.push_str(&m);
    g
}

/// Generated production artifacts (property monitors ON) for one module. The
/// C driver `#include`s the artifact file, so only the Rust source is kept in
/// memory.
pub struct ProductionArtifacts {
    pub rust_path: PathBuf,
    pub c_path: PathBuf,
    pub rust_code: String,
}

/// Generate the production artifacts (property monitors ON) for `module` into
/// `<scratch_root>/<tag>/` and return their paths + contents. The input
/// drivers ([`build_drivers`]) build on top of this; the monitor-reference
/// suite (4.4) uses it to drive the generated monitors over raw sample lines.
pub fn production_artifacts(
    tag: &str,
    module: &rfs::Module,
    scratch_root: &Path,
) -> ProductionArtifacts {
    let dir = scratch_root.join(tag);
    std::fs::create_dir_all(&dir).expect("create artifact dir");

    let ir = rfs::Ir {
        ir_version: rfs::IR_VERSION,
        modules: vec![module.clone()],
    };
    let options = rfs::codegen::BackendOptions {
        with_property_monitors: true,
        ..rfs::codegen::BackendOptions::default()
    };
    let rust_code = rfs::codegen::CodegenCommandKind::Production
        .run(
            rfs::codegen::get_backend("rust").expect("rust backend"),
            &ir,
            &options,
        )
        .expect("rust production codegen")
        .code;
    let c_code = rfs::codegen::CodegenCommandKind::Production
        .run(
            rfs::codegen::get_backend("c").expect("c backend"),
            &ir,
            &options,
        )
        .expect("c production codegen")
        .code;

    let rust_art = dir.join(format!("{tag}_prod.rs"));
    let c_art = dir.join(format!("{tag}_prod.c"));
    std::fs::write(&rust_art, &rust_code).expect("write rust artifact");
    std::fs::write(&c_art, &c_code).expect("write c artifact");
    ProductionArtifacts {
        rust_path: rust_art,
        c_path: c_art,
        rust_code,
    }
}

/// Build both drivers for one spec: generate the production artifacts
/// (property monitors ON), write the driver sources next to them, compile.
pub fn build_drivers(tag: &str, module: &rfs::Module, scratch_root: &Path) -> (PathBuf, PathBuf) {
    let art = production_artifacts(tag, module, scratch_root);
    let dir = art.rust_path.parent().unwrap().to_path_buf();
    let rust_drv_src = rust_driver_source(module, &art.rust_code);

    let rust_drv = dir.join(format!("{tag}_driver.rs"));
    std::fs::write(&rust_drv, rust_drv_src).expect("write rust driver");
    let c_drv = dir.join(format!("{tag}_driver.c"));
    std::fs::write(
        &c_drv,
        c_driver_source(module, &art.c_path.file_name().unwrap().to_string_lossy()),
    )
    .expect("write c driver");

    let rust_bin = dir.join(format!("{tag}_drv_rs"));
    let out = Command::new(rustc())
        .args(["--edition", "2021"])
        .arg(&rust_drv)
        .arg("-o")
        .arg(&rust_bin)
        .output()
        .expect("run rustc on rust driver");
    assert!(
        out.status.success(),
        "rustc failed on {}:\n{}",
        rust_drv.display(),
        String::from_utf8_lossy(&out.stderr)
    );

    let c_bin = dir.join(format!("{tag}_drv_c"));
    let out = Command::new(GCC)
        .arg("-Wall")
        .arg(&c_drv)
        .arg("-o")
        .arg(&c_bin)
        .output()
        .expect("run gcc on c driver");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "gcc failed on {}:\n{stderr}",
        c_drv.display()
    );
    assert!(
        !stderr.lines().any(|l| l.contains("warning:")),
        "gcc -Wall emitted warnings on {} (generated C must be warning-free):\n{stderr}",
        c_drv.display()
    );

    (rust_bin, c_bin)
}

// ---------------------------------------------------------------------------
// Sample-level drivers (Phase 6 task 4, action 4 / subtask 4.4): drive the
// GENERATED property monitors over raw sample lines instead of FSM inputs.
// One line per sample: `State|v0,v1,...` (declaration-order var values; bare
// `State` when the module has no vars). stdout contract:
//
// ```text
// M{k} OK | M{k} FAIL {label} step={k} trig={t|-1}
// FIRST none | FIRST {label} step={n} trig={t|-1}
// ```
//
// `FIRST` is the first monitor confirmation (earliest sample, declaration
// order on ties) - exactly what the FSM's sticky record would hold. Sample k
// carries from_state = sample k-1's state (the initial state for k = 0) and
// empty input/outputs; bodies that read those atoms are out of scope for the
// specs this machinery serves (43/44/45 monitorable properties).

// Not every consumer of this module uses the sample-level drivers - allow
// dead code so each including test crate stays warning-free.
#[allow(dead_code)]
pub fn rust_sample_driver_source(module: &rfs::Module, artifact: &str) -> String {
    let states = state_variants(module);
    let init = module
        .initial_state
        .as_deref()
        .unwrap_or_else(|| states.first().expect("stateful module"));
    let mut arms = String::new();
    for s in &states {
        arms.push_str(&format!("\"{s}\" => State::{s},\n"));
    }
    let vars_init = if module.vars.is_empty() {
        "let vars = Vars;".to_string()
    } else {
        let fields: Vec<String> = module
            .vars
            .iter()
            .enumerate()
            .map(|(i, v)| match &v.ty {
                rfs::Type::Bool => format!("{}: (f[{i}] != 0)", v.name),
                rfs::Type::Bit { width } => format!("{}: f[{i}] as {}", v.name, bit_rust_type(*width)),
                rfs::Type::Enum { .. } => panic!("enum-typed vars are not supported by the sample-level drivers (specs 43/44/45 use none)"),
            })
            .collect();
        format!("let vars = Vars {{ {} }};", fields.join(", "))
    };

    let mut g = String::new();
    g.push_str("use std::io::BufRead;\n");
    g.push_str(&format!("fn main() {{\n    let mut mon = PropMonitors::new();\n    let mut first: Option<PropertyViolation> = None;\n    let stdin = std::io::stdin();\n    let mut prev = State::{init};\n    let mut k = 0usize;\n"));
    g.push_str("    for raw in stdin.lock().lines() {\n        let line = raw.expect(\"read sample line\").trim().to_string();\n        if line.is_empty() { continue; }\n");
    g.push_str("        let (name, rest) = match line.split_once('|') { Some((a, b)) => (a, b), None => (line.as_str(), \"\") };\n");
    g.push_str(&format!("        let st = match name {{\n{arms}            _ => panic!(\"unknown state {{}}\", name),\n        }};\n"));
    g.push_str("        let f: Vec<u64> = if rest.is_empty() { Vec::new() } else { rest.split(',').map(|s| s.parse::<u64>().expect(\"field value\")).collect() };\n");
    g.push_str(&format!("        {vars_init}\n"));
    g.push_str("        let sample = Sample { input: None, outputs: Vec::new(), from_state: prev, to_state: st, vars };\n");
    g.push_str("        match mon.step(&sample, k) {\n");
    g.push_str("            Some(pv) => { println!(\"M{} FAIL {} step={} trig={}\", k, pv.label, pv.step, pv.trigger_step.map(|x| x.to_string()).unwrap_or_else(|| \"-1\".to_string())); if first.is_none() { first = Some(pv); } }\n");
    g.push_str("            None => println!(\"M{} OK\", k),\n        }\n");
    g.push_str("        prev = st;\n        k += 1;\n    }\n");
    g.push_str("    match &first {\n        Some(pv) => println!(\"FIRST {} step={} trig={}\", pv.label, pv.step, pv.trigger_step.map(|x| x.to_string()).unwrap_or_else(|| \"-1\".to_string())),\n        None => println!(\"FIRST none\"),\n    }\n}\n");

    let mut out = artifact.to_string();
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out.push('\n');
    out.push_str(&g);
    out
}

#[allow(dead_code)]
pub fn c_sample_driver_source(module: &rfs::Module, artifact_file: &str) -> String {
    let states = state_variants(module);
    let init = module
        .initial_state
        .as_deref()
        .unwrap_or_else(|| states.first().expect("stateful module"));
    let mut arms = String::new();
    for s in &states {
        arms.push_str(&format!(
            "if (!strcmp(n, \"{s}\")) return STATE_{};\n",
            s.to_uppercase()
        ));
    }
    let vars_init = if module.vars.is_empty() {
        "vars_t v; memset(&v, 0, sizeof v);".to_string()
    } else {
        let fields: Vec<String> = module
            .vars
            .iter()
            .enumerate()
            .map(|(i, v)| match &v.ty {
                rfs::Type::Bool => format!("(bool)(f[{i}] != 0)"),
                rfs::Type::Bit { width } => format!("({})f[{i}]", bit_c_type(*width)),
                rfs::Type::Enum { .. } => panic!("enum-typed vars are not supported by the sample-level drivers (specs 43/44/45 use none)"),
            })
            .collect();
        format!("vars_t v = {{ {} }};", fields.join(", "))
    };

    let mut g = String::new();
    g.push_str(&format!("#include \"{artifact_file}\"\n"));
    g.push_str("static state_t mr_parse_state(const char* n) {\n");
    g.push_str(&arms);
    g.push_str("    fprintf(stderr, \"unknown state %s\\n\", n); abort();\n}");
    g.push_str("int main(void) {\n");
    g.push_str("    prop_monitors_t mon; prop_monitors_new(&mon);\n");
    g.push_str("    rfs_property_violation_t first; int first_set = 0;\n");
    g.push_str(&format!(
        "    state_t prev = STATE_{};\n",
        init.to_uppercase()
    ));
    g.push_str("    char line[8192];\n    int k = 0;\n");
    g.push_str("    while (fgets(line, sizeof line, stdin)) {\n");
    g.push_str("        size_t len = strlen(line); while (len > 0 && (line[len-1] == '\\n' || line[len-1] == '\\r')) line[--len] = 0;\n");
    g.push_str("        if (len == 0) continue;\n");
    g.push_str("        char* bar = strchr(line, '|');\n");
    g.push_str("        unsigned long long f[64]; int nf = 0;\n");
    g.push_str("        if (bar) { *bar = 0; char* p = bar + 1; char* tok = strtok(p, \",\"); while (tok && nf < 64) { f[nf++] = strtoull(tok, 0, 10); tok = strtok(0, \",\"); } }\n");
    g.push_str(&format!("        {vars_init}\n"));
    g.push_str("        state_t st = mr_parse_state(line);\n");
    g.push_str("        rfs_sample_t s = { 0, 0, 0, prev, st, v };\n");
    g.push_str("        rfs_property_violation_t pv;\n");
    g.push_str("        if (prop_monitors_step(&mon, &s, k, &pv)) {\n");
    g.push_str("            printf(\"M%d FAIL %s step=%d trig=%d\\n\", k, pv.label, pv.step, pv.trigger_step);\n");
    g.push_str("            if (!first_set) { first = pv; first_set = 1; }\n");
    g.push_str("        } else {\n            printf(\"M%d OK\\n\", k);\n        }\n");
    g.push_str("        prev = st; k++;\n    }\n");
    g.push_str("    if (first_set) printf(\"FIRST %s step=%d trig=%d\\n\", first.label, first.step, first.trigger_step);\n");
    g.push_str("    else printf(\"FIRST none\\n\");\n    return 0;\n}\n");
    g
}

/// Build both SAMPLE-level drivers for one spec (generated monitors over raw
/// sample lines). Returns (rust bin, c bin).
#[allow(dead_code)]
pub fn build_sample_drivers(
    tag: &str,
    module: &rfs::Module,
    scratch_root: &Path,
) -> (PathBuf, PathBuf) {
    let art = production_artifacts(tag, module, scratch_root);
    let dir = art.rust_path.parent().unwrap().to_path_buf();

    let rust_drv = dir.join(format!("{tag}_sdrv.rs"));
    std::fs::write(&rust_drv, rust_sample_driver_source(module, &art.rust_code))
        .expect("write rust sample driver");
    let rust_bin = dir.join(format!("{tag}_sdrv_rs"));
    let out = Command::new(rustc())
        .args(["--edition", "2021"])
        .arg(&rust_drv)
        .arg("-o")
        .arg(&rust_bin)
        .output()
        .expect("run rustc on rust sample driver");
    assert!(
        out.status.success(),
        "rustc failed on {}:\n{}",
        rust_drv.display(),
        String::from_utf8_lossy(&out.stderr)
    );

    let c_drv = dir.join(format!("{tag}_sdrv.c"));
    std::fs::write(
        &c_drv,
        c_sample_driver_source(module, &art.c_path.file_name().unwrap().to_string_lossy()),
    )
    .expect("write c sample driver");
    let c_bin = dir.join(format!("{tag}_sdrv_c"));
    let out = Command::new(GCC)
        .arg("-Wall")
        .arg(&c_drv)
        .arg("-o")
        .arg(&c_bin)
        .output()
        .expect("run gcc on c sample driver");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "gcc failed on {}:\n{stderr}",
        c_drv.display()
    );
    assert!(
        !stderr.lines().any(|l| l.contains("warning:")),
        "gcc -Wall emitted warnings on {} (generated C must be warning-free):\n{stderr}",
        c_drv.display()
    );

    (rust_bin, c_bin)
}

/// Run one driver with the script on stdin. Returns (exit code, stdout).
pub fn run_driver(bin: &Path, script: &str) -> (i32, String) {
    let mut child = Command::new(bin)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("run driver {:?}: {e}", bin));
    use std::io::Write;
    child
        .stdin
        .as_mut()
        .expect("stdin piped")
        .write_all(script.as_bytes())
        .expect("write script");
    drop(child.stdin.take());
    let out = child.wait_with_output().expect("wait for driver");
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).to_string(),
    )
}
