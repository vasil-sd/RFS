//! Rust backend: model-checker executable generation.
//!
//! The generated binary embeds the production FSM core (no unit tests) plus
//! two runtimes sharing the same monitors and trace format:
//!
//! - **exact mode** (default, Phase 4): a level-synchronous BFS over the
//!   reachable configurations with a visited set and parent pointers. The
//!   input enumeration, configuration packing, and successor order come from
//!   the codegen-time plan (`crate::mc_plan`, D-P4-1..3) embedded as tables,
//!   so the Rust and C engines are identical by construction. Statuses and
//!   the machine-readable summary line follow `crate::codegen_mc` (D-P4-4);
//!   exit codes: 0 pass-complete, 1 fail, 2 unknown-state-limit.
//! - **fuzz mode** (`--mode fuzz`, Phase 3): random bounded runs with forced
//!   violation injection; behavior unchanged from Phase 3.
//!
//! The generated binary is self-contained: it re-parses its own CLI flags and
//! cannot depend on the RFS toolchain crate.

use crate::codegen::{BackendOptions, CodegenOutput, McMode};
use crate::ir::Module;
use crate::mc_plan::{McPlan, ValueDomain};

/// Entry point for the `mc` command of the Rust backend.
pub fn generate_model_checker(
    module: &Module,
    options: &BackendOptions,
) -> Result<CodegenOutput, Vec<String>> {
    // SMT bounded mode (task 11): a standalone z3-based binary - no FSM core,
    // no exact/fuzz runtimes. The spec section is rendered from the shared
    // plan (smt_plan/smt_emit) so both backends build identical constraints.
    if options.mc_mode == McMode::SmtBounded {
        let plan = McPlan::compute(module, &options.assumptions);
        let spec = crate::smt_plan::build_smt_spec(module, &plan).map_err(|e| vec![e])?;
        let code = crate::smt_emit::emit_rust_spec(&spec)
            + &SMT_ENGINE_RUST.replace("{BOUND}", &options.mc_smt_bound.to_string());
        return Ok(CodegenOutput {
            code,
            warnings: Vec::new(),
        });
    }
    // The model checker reuses the production FSM core without unit tests.
    // Property monitors are always included (D-P3-6).
    let fsm_options = BackendOptions {
        assumptions: options.assumptions.clone(),
        steps: 0,
        with_property_monitors: false,
        ..BackendOptions::default()
    };
    let plan = McPlan::compute(module, &options.assumptions);
    let fsm = crate::backends::rust_fsm::generate_core(
        module,
        &fsm_options,
        false,
        crate::codegen::PropertyMonitorMode::Always,
    )?;
    let mut out = fsm.code;
    let (exact, liv_warnings) = exact_codegen(module, &plan, options).map_err(|e| vec![e])?;
    out.push_str(&exact);
    out.push_str(FUZZ_RUNTIME);
    out.push_str(MAIN_DISPATCH);
    Ok(CodegenOutput {
        code: out,
        warnings: fsm.warnings.into_iter().chain(liv_warnings).collect(),
    })
}

/// Render the exact-mode runtime from the plan (D-P4-1..6). Returns the code
/// plus liveness decomposition diagnostics (codegen warnings, D-P4-9).
fn exact_codegen(
    module: &Module,
    plan: &McPlan,
    options: &BackendOptions,
) -> Result<(String, Vec<String>), String> {
    use crate::backends::rust::{rust_ident, snake};

    let mut s = String::new();
    // Codegen-time defaults for the runtime flags (D-P4-7).
    let mode_const = if options.mc_mode == McMode::Fuzz {
        1
    } else {
        0
    };
    s.push_str(&format!("const DEFAULT_MODE: u8 = {};\n", mode_const));
    s.push_str(&format!(
        "const DEFAULT_MAX_STATES: usize = {};\n",
        options.mc_max_states
    ));
    s.push_str(&format!(
        "const DEFAULT_NONDET: bool = {};\n",
        options.mc_nondet
    ));
    s.push_str(&format!(
        "const DEFAULT_TIMEOUT_SEC: u64 = {};\n",
        options.mc_timeout_sec
    ));
    s.push_str(&format!(
        "const DEFAULT_THREADS: usize = {};\n",
        options.mc_threads
    ));
    s.push_str(&format!(
        "const CONFIG_TOTAL_BYTES: usize = {};\n",
        plan.encoding.total_bytes
    ));
    s.push_str(&format!(
        "const MONITORS_BITS: usize = {};\n",
        plan.encoding.monitors_bits
    ));

    // all_inputs(): canonical enumeration (D-P4-2) - variants in declaration
    // order, fields in declaration order, values ascending per the plan.
    s.push_str("fn all_inputs() -> Vec<Input> {\n    let mut out = Vec::new();\n");
    for vs in &plan.abstraction.variants {
        if vs.fields.is_empty() {
            s.push_str(&format!(
                "    out.push(Input::{});\n",
                rust_ident(&vs.unique_name)
            ));
            continue;
        }
        let mut concrete_idx: Vec<usize> = Vec::new();
        for (i, f) in vs.fields.iter().enumerate() {
            if f.concrete {
                let k = concrete_idx.len();
                s.push_str(&format!(
                    "    for _c{} in {} {{\n",
                    k,
                    domain_iter(f.domain.as_ref().unwrap())
                ));
                concrete_idx.push(i);
            }
        }
        let field_exprs: Vec<String> = vs
            .fields
            .iter()
            .enumerate()
            .map(|(i, f)| {
                if f.concrete {
                    let k = concrete_idx.iter().position(|&x| x == i).unwrap();
                    format!("{}: _c{}", rust_ident(&f.name), k)
                } else {
                    format!(
                        "{}: {}",
                        rust_ident(&f.name),
                        rep_expr(f.representative.as_ref().unwrap())
                    )
                }
            })
            .collect();
        s.push_str(&format!(
            "    out.push(Input::{} {{ {} }});\n",
            rust_ident(&vs.unique_name),
            field_exprs.join(", ")
        ));
        for _ in 0..concrete_idx.len() {
            s.push_str("    }\n");
        }
    }
    s.push_str("    out\n}\n");
    // put_bits + config_bytes (plan layout, D-P4-2).
    s.push_str(
        "fn put_bits(buf: &mut [u8], offset: usize, value: u64, width: u32) {\n    for i in 0..width {\n        if (value >> i) & 1 == 1 {\n            let b = offset + i as usize;\n            buf[b / 8] |= 1 << (b % 8);\n        }\n    }\n}\n",
    );
    s.push_str(
        "fn config_bytes(state: State, vars: &Vars, prop: &PropMonitors, assume: &AssumeMonitors) -> Vec<u8> {\n    let mut buf = vec![0u8; CONFIG_TOTAL_BYTES];\n",
    );
    let state_variants: Vec<String> = module
        .enums
        .iter()
        .find(|e| e.name == "State")
        .map(|e| e.variants.clone())
        .unwrap_or_default();
    if !state_variants.is_empty() {
        let arms: Vec<String> = state_variants
            .iter()
            .enumerate()
            .map(|(i, v)| format!("State::{} => {}u64", rust_ident(v), i))
            .collect();
        s.push_str(&format!(
            "    let s = match state {{ {} }};\n    put_bits(&mut buf, 0, s, {});\n",
            arms.join(", "),
            plan.encoding.state_bits
        ));
    } else {
        s.push_str("    put_bits(&mut buf, 0, 0u64, 0);\n");
    }
    for slot in &plan.encoding.var_slots {
        let ve = var_value_expr(slot, module);
        s.push_str(&format!(
            "    put_bits(&mut buf, {}, {}, {});\n",
            slot.offset_bits, ve, slot.width_bits
        ));
    }
    for m in &plan.encoding.monitors {
        let agg = if m.is_assumption { "assume" } else { "prop" };
        for n in &m.nodes {
            let base = format!("{}.mon_{}", agg, snake(&m.label));
            let nb = m.offset_bits;
            if n.pending_bits > 0 {
                s.push_str(&format!(
                    "    put_bits(&mut buf, {}, if {}.n{}_pending {{ 1u64 }} else {{ 0u64 }}, {});\n",
                    nb + n.offset_bits, base, n.node_id, n.pending_bits
                ));
            }
            if n.remaining_bits > 0 {
                s.push_str(&format!(
                    "    put_bits(&mut buf, {}, {}.n{}_remaining as u64, {});\n",
                    nb + n.offset_bits + n.pending_bits,
                    base,
                    n.node_id,
                    n.remaining_bits
                ));
            }
            if n.antecedent_bits > 0 {
                s.push_str(&format!(
                    "    put_bits(&mut buf, {}, if {}.n{}_prev_antecedent {{ 1u64 }} else {{ 0u64 }}, {});\n",
                    nb + n.offset_bits + n.pending_bits + n.remaining_bits, base, n.node_id, n.antecedent_bits
                ));
            }
        }
    }
    s.push_str("    buf\n}\n");

    // mc_step_all: every enabled transition's effect in firing order (D-P4-11).
    s.push_str(&crate::backends::rust_fsm::render_mc_step_all(module)?);

    // The BFS engine (static; see D-P4-2..6).
    s.push_str(EXACT_ENGINE);

    // Liveness (Phase 4 task 6, D-P4-9/10/11): obligation decomposition,
    // per-leaf sample evaluators, the static engine, and the property table.
    let liv = crate::mc_plan::LivenessDecomposition::compute(module);
    let warnings: Vec<String> = liv.diagnostics.clone();
    s.push_str(LIV_ENGINE);
    let mut prop_fns: Vec<String> = Vec::new();
    let mut prop_names: Vec<String> = Vec::new();
    for (pi, spec_opt) in liv.properties.iter().enumerate() {
        let Some(spec) = spec_opt else { continue };
        // Flatten the skeleton: leaves in first-appearance order.
        let mut leaves: Vec<crate::mc_plan::SkeletonLeaf> = Vec::new();
        let skel_expr = flatten_skeleton(&spec.skeleton, &mut leaves);
        let mut leaf_entries: Vec<String> = Vec::new();
        for (li, leaf) in leaves.iter().enumerate() {
            match leaf {
                crate::mc_plan::SkeletonLeaf::Safety(f) => {
                    let name = format!("liv_p{pi}_l{li}");
                    s.push_str(&crate::backends::rust_fsm::render_sample_formula_evaluator(
                        module, &name, f,
                    )?);
                    leaf_entries.push(format!(
                        "LivLeaf {{ kind: 0, eval_a: {name}, eval_b: None }}"
                    ));
                }
                crate::mc_plan::SkeletonLeaf::Obligation(oi) => match &spec.obligations[*oi] {
                    crate::mc_plan::LivenessObligation::Eventually { body } => {
                        let name = format!("liv_p{pi}_l{li}");
                        s.push_str(&crate::backends::rust_fsm::render_sample_formula_evaluator(
                            module, &name, body,
                        )?);
                        leaf_entries.push(format!(
                            "LivLeaf {{ kind: 1, eval_a: {name}, eval_b: None }}"
                        ));
                    }
                    crate::mc_plan::LivenessObligation::Until { left, right } => {
                        let na = format!("liv_p{pi}_l{li}");
                        let nb = format!("liv_p{pi}_l{li}b");
                        s.push_str(&crate::backends::rust_fsm::render_sample_formula_evaluator(
                            module, &na, left,
                        )?);
                        s.push_str(&crate::backends::rust_fsm::render_sample_formula_evaluator(
                            module, &nb, right,
                        )?);
                        leaf_entries.push(format!(
                            "LivLeaf {{ kind: 2, eval_a: {na}, eval_b: Some({nb}) }}"
                        ));
                    }
                },
            }
        }
        s.push_str(&format!(
            "fn liv_p{pi}_skel(t: &[bool]) -> bool {{ {} }}\n",
            skel_expr
        ));
        let formula_text = crate::printer::print_formula(&module.properties[pi].formula);
        let label_lit = format!("{:?}", spec.label);
        let formula_lit = format!("{:?}", formula_text);
        prop_names.push(format!("liv_prop_{pi}"));
        prop_fns.push(format!(
            "fn liv_prop_{pi}() -> LivProperty {{ LivProperty {{ label: {label_lit}, formula: {formula_lit}, leaves: vec![{}], skeleton: liv_p{pi}_skel }} }}\n",
            leaf_entries.join(", ")
        ));
    }
    for f in &prop_fns {
        s.push_str(f);
    }
    s.push_str(&format!(
        "const LIV_PROPS: [fn() -> LivProperty; {}] = [{}];\n",
        prop_names.len(),
        prop_names.join(", ")
    ));

    s.push_str(EXACT_RUN);
    Ok((s, warnings))
}

/// Re-index the skeleton leaves in first-appearance order and render the
/// boolean expression over `t[0..L)` (per-leaf satisfaction flags).
fn flatten_skeleton(
    sk: &crate::mc_plan::Skeleton,
    leaves: &mut Vec<crate::mc_plan::SkeletonLeaf>,
) -> String {
    match sk {
        crate::mc_plan::Skeleton::And(l, r) => format!(
            "({} && {})",
            flatten_skeleton(l, leaves),
            flatten_skeleton(r, leaves)
        ),
        crate::mc_plan::Skeleton::Or(l, r) => format!(
            "({} || {})",
            flatten_skeleton(l, leaves),
            flatten_skeleton(r, leaves)
        ),
        crate::mc_plan::Skeleton::Implies(l, r) => format!(
            "(!({}) || {})",
            flatten_skeleton(l, leaves),
            flatten_skeleton(r, leaves)
        ),
        crate::mc_plan::Skeleton::Not(x) => format!("!({})", flatten_skeleton(x, leaves)),
        crate::mc_plan::Skeleton::Leaf(leaf) => {
            let i = leaves.len();
            leaves.push(leaf.clone());
            format!("t[{}]", i)
        }
    }
}

/// Canonical iterator expression for a concrete field domain (D-P4-2).
fn domain_iter(d: &ValueDomain) -> String {
    use crate::backends::rust::rust_ident;
    match d {
        ValueDomain::Bool => "[false, true]".into(),
        ValueDomain::Int { max } => format!("0..={max}"),
        ValueDomain::Enum { variants } => {
            let vs: Vec<String> = variants.iter().map(|v| rust_ident(v)).collect();
            format!("[{}]", vs.join(", "))
        }
    }
}

/// Representative value expression for an abstracted field.
fn rep_expr(v: &crate::codegen_fsm::RepValue) -> String {
    use crate::backends::rust::rust_ident;
    match v {
        crate::codegen_fsm::RepValue::Bool(b) => b.to_string(),
        crate::codegen_fsm::RepValue::Int(i) => i.to_string(),
        crate::codegen_fsm::RepValue::EnumVariant(path) => path
            .iter()
            .map(|s| rust_ident(s))
            .collect::<Vec<_>>()
            .join("::"),
        crate::codegen_fsm::RepValue::EnumEmpty(name) => format!("{}::__Empty__", rust_ident(name)),
        crate::codegen_fsm::RepValue::EnumUnknown(name) => {
            format!("{}::__Unknown__", rust_ident(name))
        }
    }
}

/// Value expression for one variable slot in `config_bytes`.
fn var_value_expr(slot: &crate::mc_plan::VarSlot, module: &Module) -> String {
    use crate::backends::rust::rust_ident;
    let ty = module
        .vars
        .iter()
        .find(|v| v.name == slot.name)
        .map(|v| v.ty.clone())
        .unwrap_or(crate::ir::Type::Bool);
    match ty {
        crate::ir::Type::Bool => format!(
            "if vars.{} {{ 1u64 }} else {{ 0u64 }}",
            rust_ident(&slot.name)
        ),
        crate::ir::Type::Bit { .. } => format!("vars.{} as u64", rust_ident(&slot.name)),
        crate::ir::Type::Enum { name } => {
            let variants: Vec<String> = module
                .enums
                .iter()
                .find(|e| e.name == name)
                .map(|e| e.variants.clone())
                .unwrap_or_default();
            if variants.is_empty() {
                format!("vars.{} as u64", rust_ident(&slot.name))
            } else {
                let arms: Vec<String> = variants
                    .iter()
                    .enumerate()
                    .map(|(i, v)| format!("{}::{} => {}u64", rust_ident(&name), rust_ident(v), i))
                    .collect();
                format!(
                    "match vars.{} {{ {} }}",
                    rust_ident(&slot.name),
                    arms.join(", ")
                )
            }
        }
    }
}
/// The exact-mode BFS engine (generated verbatim into every MC binary).
const EXACT_ENGINE: &str = r#"
#[derive(Clone)]
struct Node { state: State, vars: Vars, prop: PropMonitors, assume: AssumeMonitors }

enum StepOutcome {
    Success(Node),
    Pruned,
    Violation(String, usize, Option<usize>),
}

/// Apply one enabled transition (or on_error fallback) to a configuration and
/// step the monitors at the correct sample index (depth + 1).
fn step_successor(node: &Node, depth: usize, input: Input, eff: &McEffect, active_assumes: &[String]) -> StepOutcome {
    let mut prop = node.prop.clone();
    let mut assume = node.assume.clone();
    let sample = Sample { input: Some(input), outputs: eff.outputs.clone(), from_state: node.state.clone(), to_state: eff.next_state.clone(), vars: eff.next_vars.clone() };
    let idx = depth + 1;
    if assume.step(&sample, idx, active_assumes).is_some() { return StepOutcome::Pruned; }
    if let Some(pv) = prop.step(&sample, idx) { return StepOutcome::Violation(pv.label, pv.step, pv.trigger_step); }
    StepOutcome::Success(Node { state: eff.next_state.clone(), vars: eff.next_vars.clone(), prop, assume })
}

/// Steps from the root to configuration `cid`: (child config id, input index, transition id).
fn path_to(parent: &Vec<Option<(usize, usize, usize)>>, cid: usize) -> Vec<(usize, usize, usize)> {
    let mut v = Vec::new();
    let mut c = cid;
    while let Some((p, i, t)) = parent[c] { v.push((c, i, t)); c = p; }
    v.reverse();
    v
}

/// Replay a path on a fresh Fsm to rebuild the Phase 3 trace format.
fn replay(path: &[(usize, usize, usize)], nondet: bool, active_assumes: &[String]) -> Fsm {
    let inputs = all_inputs();
    let mut fsm = Fsm::new();
    fsm.active_assumes = active_assumes.to_vec();
    for (_cid, iidx, tid) in path {
        let input = inputs[*iidx];
        let effects = mc_step_all(fsm.state.clone(), &fsm.vars, input);
        let eff = if *tid == usize::MAX || !nondet { &effects[0] } else { effects.iter().find(|e| e.transition_id == *tid).unwrap() };
        let sample = Sample { input: Some(input), outputs: eff.outputs.clone(), from_state: fsm.state.clone(), to_state: eff.next_state.clone(), vars: eff.next_vars.clone() };
        fsm.trace.steps.push(TraceStep { input, outputs: eff.outputs.clone(), from_state: sample.from_state.clone(), to_state: sample.to_state.clone(), vars: sample.vars.clone() });
        let idx = fsm.trace.steps.len();
        let _ = fsm.assume_monitors.step(&sample, idx, &fsm.active_assumes);
        if let Some(pv) = fsm.prop_monitors.step(&sample, idx) { if fsm.property_violation.is_none() { fsm.property_violation = Some(pv); } }
        fsm.state = eff.next_state.clone();
        fsm.vars = eff.next_vars.clone();
    }
    fsm
}

/// The machine-readable summary line (D-P4-4), exact format.
fn exact_summary_line(status: &str, states: usize, transitions: usize, max_depth: usize, fail: Option<(&str, &str, usize)>) -> String {
    let mut s = format!("model-check summary: status={} states={} transitions={} max_depth={}", status, states, transitions, max_depth);
    if let Some((r, l, st)) = fail { s.push_str(&format!(" reason={} label={} step={}", r, l, st)); }
    s
}
"#;
// ---------------------------------------------------------------------------
// Liveness engine (Phase 4 task 6; D-P4-10/11). Runs on the reachable graph
// after exact BFS completes (or is truncated by --max-states / timeout).
//
// Per property: a boolean skeleton over per-leaf progress automata. Each leaf
// has a small deterministic automaton whose state is eventually constant on
// every infinite path; the property is violated iff some reachable product
// cycle realizes a limit vector that falsifies the skeleton (verified against
// brute-force lasso enumeration - see PHASE_4_TASK_6_STATUS.md).
//
// Labels are EDGE-based: position 0 uses the initial-configuration sample,
// position k uses the sample of step k, so input_is/output_has atoms inside
// obligation bodies depend on the edge taken (the same configuration reached
// via different inputs can carry different labels).
// ---------------------------------------------------------------------------
const LIV_ENGINE: &str = r#"
struct LivGraph<'a> {
    nodes: &'a Vec<Node>,
    /// Per config: (input index, transition id, target config) in discovery order.
    adj: &'a Vec<Vec<(usize, usize, usize)>>,
    /// Config fully expanded (matters for the nondet fairness filter when the
    /// graph is truncated by --max-states / timeout).
    complete: &'a [bool],
    inputs: &'a [Input],
    nondet: bool,
}

#[derive(Clone, Copy)]
struct LivLeaf {
    kind: u8, // 0 safety, 1 eventually, 2 until
    eval_a: fn(&Sample) -> bool,
    eval_b: Option<fn(&Sample) -> bool>,
}

struct LivProperty {
    label: &'static str,
    formula: &'static str,
    leaves: Vec<LivLeaf>,
    skeleton: fn(&[bool]) -> bool,
}

struct LivWitness {
    /// Lasso edges in order: the first `cycle_start` are the prefix (from
    /// config 0); the rest form a cycle back to the config at position
    /// `cycle_start`. Each edge: (source config id, input index, transition id).
    lasso: Vec<(usize, usize, usize)>,
    cycle_start: usize,
    /// Indices of the leaves falsified on the witness path.
    falsified: Vec<usize>,
}

/// Label bits of one step: bit 2k = leaf k atom A (S / P / P), bit 2k+1 =
/// leaf k atom B (the `until` right operand). Cached per edge position.
fn liv_bits_at(graph: &LivGraph, cache: &mut Vec<Vec<Option<u32>>>, c: usize, pos: usize, prop: &LivProperty) -> u32 {
    if let Some(b) = cache[c][pos] { return b; }
    let (iidx, tid, _v) = graph.adj[c][pos];
    let node = &graph.nodes[c];
    let effects = mc_step_all(node.state.clone(), &node.vars, graph.inputs[iidx]);
    let eff = if tid == usize::MAX || !graph.nondet {
        &effects[0]
    } else {
        effects.iter().find(|e| e.transition_id == tid).unwrap()
    };
    let sample = Sample {
        input: Some(graph.inputs[iidx]),
        outputs: eff.outputs.clone(),
        from_state: node.state.clone(),
        to_state: eff.next_state.clone(),
        vars: eff.next_vars.clone(),
    };
    let mut bits = 0u32;
    for (k, leaf) in prop.leaves.iter().enumerate() {
        if (leaf.eval_a)(&sample) { bits |= 1 << (2 * k); }
        if let Some(b) = leaf.eval_b { if b(&sample) { bits |= 1 << (2 * k + 1); } }
    }
    cache[c][pos] = Some(bits);
    bits
}

/// Mixed-radix transition of the per-leaf progress automata on label bits.
/// Leaf states: safety 0=ok/1=bad; eventually 0=wait/1=done;
/// until 0=ok/1=sat/2=bad. All transitions are one-way, so every state
/// sequence on an infinite path is eventually constant (the limit vector).
fn liv_delta(q: usize, places: &[usize], radices: &[usize], kinds: &[u8], bits: u32) -> usize {
    let mut out = 0usize;
    for (k, &r) in radices.iter().enumerate() {
        let s = (q / places[k]) % r;
        let a = (bits >> (2 * k)) & 1 == 1;
        let b = (bits >> (2 * k + 1)) & 1 == 1;
        let ns = match kinds[k] {
            0 => if s == 1 || !a { 1 } else { 0 },
            1 => if s == 1 || a { 1 } else { 0 },
            _ => match s {
                2 => 2,
                1 => 1,
                _ => if b { 1 } else if !a { 2 } else { 0 },
            },
        };
        out = out * r + ns;
    }
    out
}

/// A leaf is satisfied on an infinite path iff its limit state is in the good set.
fn liv_good(kind: u8, s: usize) -> bool {
    match kind {
        0 => s == 0, // safety: never entered bad
        1 => s == 1, // eventually: done reached
        _ => s != 2, // until: ok (P-forever branch) or sat
    }
}

/// Analyze one property on the graph. Returns a lasso witness iff violated.
fn liv_analyze(graph: &LivGraph, prop: &LivProperty) -> Option<LivWitness> {
    let num_leaves = prop.leaves.len();
    if num_leaves == 0 { return None; }
    let radices: Vec<usize> = prop.leaves.iter().map(|l| if l.kind == 2 { 3 } else { 2 }).collect();
    let mut places = vec![1usize; num_leaves];
    for k in (0..num_leaves - 1).rev() { places[k] = places[k + 1] * radices[k + 1]; }
    let qcount: usize = radices.iter().product();
    let kinds: Vec<u8> = prop.leaves.iter().map(|l| l.kind).collect();

    // Position-0 label (initial configuration sample, as in Fsm::new).
    let n0 = &graph.nodes[0];
    let init_sample = Sample { input: None, outputs: Vec::new(), from_state: n0.state.clone(), to_state: n0.state.clone(), vars: n0.vars.clone() };
    let mut init_bits = 0u32;
    for (k, leaf) in prop.leaves.iter().enumerate() {
        if (leaf.eval_a)(&init_sample) { init_bits |= 1 << (2 * k); }
        if let Some(b) = leaf.eval_b { if b(&init_sample) { init_bits |= 1 << (2 * k + 1); } }
    }

    let mut label_cache: Vec<Vec<Option<u32>>> = graph.adj.iter().map(|es| vec![None; es.len()]).collect();

    // Product BFS from (q0, config 0): reachable[q] = configs reachable with
    // per-leaf automaton states encoding q.
    let start_q = liv_delta(0, &places, &radices, &kinds, init_bits);
    let mut reachable: Vec<Option<std::collections::HashSet<usize>>> = vec![None; qcount];
    let mut queue: Vec<(usize, usize)> = vec![(start_q, 0)];
    reachable[start_q].get_or_insert_with(std::collections::HashSet::new).insert(0);
    while let Some((q, c)) = queue.pop() {
        for pos in 0..graph.adj[c].len() {
            let v = graph.adj[c][pos].2;
            let bits = liv_bits_at(graph, &mut label_cache, c, pos, prop);
            let q2 = liv_delta(q, &places, &radices, &kinds, bits);
            if !reachable[q2].get_or_insert_with(std::collections::HashSet::new).contains(&v) {
                reachable[q2].as_mut().unwrap().insert(v);
                queue.push((q2, v));
            }
        }
    }

    // Per limit vector q: stay-edge subgraph on R(q). In nondet mode a bad
    // cycle must be STRONGLY FAIR (D-P4-11): every enabled successor of a
    // cycle state is taken on the cycle, i.e. each cycle state has exactly
    // one explored successor (and was fully expanded when truncated).
    for q in 0..qcount {
        let Some(reached) = &reachable[q] else { continue };
        if reached.is_empty() { continue; }
        let starts: Vec<usize> = reached.iter().copied().collect();
        let mut stay_succ: std::collections::HashMap<usize, Vec<(usize, usize)>> = std::collections::HashMap::new();
        let mut self_loop: std::collections::HashSet<usize> = std::collections::HashSet::new();
        for &c in &starts {
            if graph.nondet && !(graph.adj[c].len() == 1 && graph.complete[c]) { continue; }
            for pos in 0..graph.adj[c].len() {
                let v = graph.adj[c][pos].2;
                if !reached.contains(&v) { continue; }
                let bits = liv_bits_at(graph, &mut label_cache, c, pos, prop);
                if liv_delta(q, &places, &radices, &kinds, bits) != q { continue; }
                stay_succ.entry(c).or_default().push((v, pos));
                if v == c { self_loop.insert(c); }
            }
        }
        // SCCs of the stay graph (Kosaraju, iterative).
        let mut rev: std::collections::HashMap<usize, Vec<usize>> = std::collections::HashMap::new();
        for (c, vs) in &stay_succ { for &(v, _pos) in vs { rev.entry(v).or_default().push(*c); } }
        let mut seen: std::collections::HashSet<usize> = std::collections::HashSet::new();
        let mut fin_order: Vec<usize> = Vec::new();
        for start in &starts {
            if !seen.insert(*start) { continue; }
            let mut stack: Vec<(usize, usize)> = vec![(*start, 0)];
            loop {
                let (c, next_pos) = *stack.last().unwrap();
                let succs_len = stay_succ.get(&c).map(|v| v.len()).unwrap_or(0);
                let mut descended = false;
                let mut i = next_pos;
                while i < succs_len {
                    let v = stay_succ[&c][i].0;
                    if seen.insert(v) {
                        stack.last_mut().unwrap().1 = i + 1;
                        stack.push((v, 0));
                        descended = true;
                        break;
                    }
                    i += 1;
                }
                if descended { continue; }
                stack.pop();
                fin_order.push(c);
                if stack.is_empty() { break; }
            }
        }
        let mut comp: std::collections::HashMap<usize, usize> = std::collections::HashMap::new();
        let mut comp_size: Vec<usize> = Vec::new();
        for &c in fin_order.iter().rev() {
            if comp.contains_key(&c) { continue; }
            let cid = comp_size.len();
            comp.insert(c, cid);
            comp_size.push(1);
            let mut members: Vec<usize> = vec![c];
            let mut mi = 0;
            while mi < members.len() {
                let u = members[mi]; mi += 1;
                if let Some(preds) = rev.get(&u) {
                    for &w in preds {
                        if !comp.contains_key(&w) { comp.insert(w, cid); comp_size[cid] += 1; members.push(w); }
                    }
                }
            }
        }
        let mut cyclic: Vec<usize> = Vec::new();
        for &c in &starts {
            if comp_size[comp[&c]] > 1 || self_loop.contains(&c) { cyclic.push(c); }
        }
        if cyclic.is_empty() { continue; }
        // The limit vector of any path looping inside this SCC is exactly q.
        let t: Vec<bool> = (0..num_leaves).map(|k| liv_good(kinds[k], (q / places[k]) % radices[k])).collect();
        if (prop.skeleton)(&t) { continue; }
        let cstar = *cyclic.iter().min().unwrap();
        let scc_members: std::collections::HashSet<usize> = starts.iter().copied().filter(|c| comp[c] == comp[&cstar]).collect();
        return Some(liv_witness(graph, prop, &places, &radices, &kinds, init_bits, q, cstar, t, &stay_succ, &scc_members));
    }
    None
}

/// Reconstruct the lasso: a product path from (start_q, 0) to (q, cstar)
/// plus a stay-edge cycle through cstar.
fn liv_witness(
    graph: &LivGraph,
    prop: &LivProperty,
    places: &[usize],
    radices: &[usize],
    kinds: &[u8],
    init_bits: u32,
    q: usize,
    cstar: usize,
    t: Vec<bool>,
    stay_succ: &std::collections::HashMap<usize, Vec<(usize, usize)>>,
    scc_members: &std::collections::HashSet<usize>,
) -> LivWitness {
    let start_q = liv_delta(0, places, radices, kinds, init_bits);
    let mut label_cache: Vec<Vec<Option<u32>>> = graph.adj.iter().map(|es| vec![None; es.len()]).collect();

    // Prefix: DFS in the product from (start_q, 0) to (q, cstar).
    let mut visited: std::collections::HashSet<(usize, usize)> = std::collections::HashSet::new();
    let mut path: Vec<(usize, usize)> = Vec::new(); // (src config, edge pos)
    let mut stack: Vec<(usize, usize, usize)> = vec![(start_q, 0, 0)]; // (q, c, next edge pos)
    visited.insert((start_q, 0));
    loop {
        let top = *stack.last().unwrap();
        if top.0 == q && top.1 == cstar { break; }
        let (cq, cc, next_pos) = top;
        let mut descended = false;
        let mut p = next_pos;
        while p < graph.adj[cc].len() {
            let v = graph.adj[cc][p].2;
            let bits = liv_bits_at(graph, &mut label_cache, cc, p, prop);
            let q2 = liv_delta(cq, places, radices, kinds, bits);
            if visited.insert((q2, v)) {
                stack.last_mut().unwrap().2 = p + 1;
                path.push((cc, p));
                stack.push((q2, v, 0));
                descended = true;
                break;
            }
            p += 1;
        }
        if descended { continue; }
        stack.pop();
        if !stack.is_empty() { path.pop(); }
        // The target is reachable (product BFS proved it), so the stack
        // cannot empty before the break above.
    }

    // Cycle: DFS from cstar over stay-successors (inside its SCC) back to cstar.
    let mut cyc: Vec<(usize, usize)> = Vec::new();
    let mut on_path: std::collections::HashSet<usize> = std::collections::HashSet::new();
    on_path.insert(cstar);
    let mut stack2: Vec<(usize, usize)> = vec![(cstar, 0)];
    loop {
        let (cc, next_i) = *stack2.last().unwrap();
        let succs = stay_succ.get(&cc).cloned().unwrap_or_default();
        let mut descended = false;
        let mut i = next_i;
        while i < succs.len() {
            let (v, epos) = succs[i];
            if !scc_members.contains(&v) { i += 1; continue; }
            if v == cstar {
                cyc.push((cc, epos));
                // Convert (config, edge pos) steps to (config, input idx,
                // transition id) edges for replay.
                let mut lasso: Vec<(usize, usize, usize)> = Vec::with_capacity(path.len() + cyc.len());
                for (c, pos) in path.iter().chain(cyc.iter()) {
                    let (iidx, tid, _v) = graph.adj[*c][*pos];
                    lasso.push((*c, iidx, tid));
                }
                let falsified: Vec<usize> = (0..prop.leaves.len()).filter(|&k| !t[k]).collect();
                return LivWitness { lasso, cycle_start: path.len(), falsified };
            }
            if on_path.insert(v) {
                stack2.last_mut().unwrap().1 = i + 1;
                cyc.push((cc, epos));
                stack2.push((v, 0));
                descended = true;
                break;
            }
            i += 1;
        }
        if descended { continue; }
        let (pc, _ni) = stack2.pop().unwrap();
        on_path.remove(&pc);
        if !stack2.is_empty() { cyc.pop(); }
        // cstar lies on a stay cycle inside its SCC, so the search cannot fail.
    }
}

/// Human-facing explanation of which leaves are falsified on the witness.
fn liv_note(graph: &LivGraph, prop: &LivProperty, w: &LivWitness) -> String {
    let sample_at = |pos: usize| -> Sample {
        if pos == 0 {
            let n0 = &graph.nodes[0];
            Sample { input: None, outputs: Vec::new(), from_state: n0.state.clone(), to_state: n0.state.clone(), vars: n0.vars.clone() }
        } else {
            let (c, iidx, tid) = w.lasso[pos - 1];
            let node = &graph.nodes[c];
            let effects = mc_step_all(node.state.clone(), &node.vars, graph.inputs[iidx]);
            let eff = if tid == usize::MAX || !graph.nondet {
                &effects[0]
            } else {
                effects.iter().find(|e| e.transition_id == tid).unwrap()
            };
            Sample { input: Some(graph.inputs[iidx]), outputs: eff.outputs.clone(), from_state: node.state.clone(), to_state: eff.next_state.clone(), vars: eff.next_vars.clone() }
        }
    };
    let mut parts: Vec<String> = Vec::new();
    for &k in &w.falsified {
        let leaf = &prop.leaves[k];
        match leaf.kind {
            0 => {
                // bad is absorbing and constant on the cycle, so the first
                // falsifying position lies inside prefix + one cycle iteration.
                let p = (0..=w.lasso.len()).find(|&pos| !(leaf.eval_a)(&sample_at(pos))).unwrap();
                parts.push(format!("safety subformula false at step {}", p));
            }
            1 => parts.push("eventually body never holds on the path".to_string()),
            _ => {
                let b = leaf.eval_b.unwrap();
                let mut seen_q = false;
                let mut p = None;
                for pos in 0..=w.lasso.len() {
                    let s = sample_at(pos);
                    if !(leaf.eval_a)(&s) && !(seen_q || b(&s)) { p = Some(pos); break; }
                    seen_q |= b(&s);
                }
                parts.push(format!("until left fails at step {} without right holding", p.unwrap()));
            }
        }
    }
    parts.join("; ")
}
"#;
/// The exact-mode entry point (generated verbatim into every MC binary).
const EXACT_RUN: &str = r#"
/// One successor outcome of a config expansion (task 10B): computed in
/// parallel (phase A), committed sequentially in canonical order (phase B).
enum LevelOutcome {
    /// Input with no matching transition and no on_error (D-P4-5 path end).
    Unexpected(usize),
    /// Successor pruned by an active assumption.
    Pruned,
    /// Property violation at this successor.
    Violation { iidx: usize, tid: usize, label: String, step: usize, trigger: Option<usize> },
    /// Valid successor configuration.
    Success { iidx: usize, tid: usize, node: Node },
}

/// Expands one config over all inputs x effects. Pure (no shared state), so
/// phase A may call it from multiple threads on disjoint configs.
fn expand_config(
    cid: usize,
    nodes: &Vec<Node>,
    depth: &Vec<usize>,
    inputs: &[Input],
    nondet: bool,
    active_assumes: &[String],
) -> Vec<LevelOutcome> {
    let node = nodes[cid].clone();
    let d = depth[cid];
    let mut outs = Vec::new();
    for (iidx, input) in inputs.iter().enumerate() {
        let effects = mc_step_all(node.state.clone(), &node.vars, *input);
        if effects.is_empty() {
            outs.push(LevelOutcome::Unexpected(iidx));
            continue;
        }
        let sel: Vec<&McEffect> = if nondet { effects.iter().collect() } else { vec![&effects[0]] };
        for eff in sel {
            match step_successor(&node, d, *input, eff, active_assumes) {
                StepOutcome::Pruned => outs.push(LevelOutcome::Pruned),
                StepOutcome::Violation(label, step, trigger) => {
                    outs.push(LevelOutcome::Violation { iidx, tid: eff.transition_id, label, step, trigger });
                }
                StepOutcome::Success(n) => outs.push(LevelOutcome::Success { iidx, tid: eff.transition_id, node: n }),
            }
        }
    }
    outs
}

fn run_exact() -> ! {
    // Runtime flags (codegen-time defaults, D-P4-7).
    let args: Vec<String> = std::env::args().collect();
    let mut max_states = DEFAULT_MAX_STATES;
    let mut nondet = DEFAULT_NONDET;
    let mut timeout_sec = DEFAULT_TIMEOUT_SEC;
    let mut threads = DEFAULT_THREADS;
    let mut fast = false;
    let mut assumes_arg = "all".to_string();
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--max-states" => { i += 1; max_states = args.get(i).and_then(|x| x.parse().ok()).unwrap_or(DEFAULT_MAX_STATES); }
            "--nondet" => { nondet = true; }
            "--timeout-sec" => { i += 1; timeout_sec = args.get(i).and_then(|x| x.parse().ok()).unwrap_or(DEFAULT_TIMEOUT_SEC); }
            "--threads" => { i += 1; threads = args.get(i).and_then(|x| x.parse().ok()).unwrap_or(DEFAULT_THREADS); }
            "--fast" => { fast = true; }
            "--assumes" => { i += 1; assumes_arg = args.get(i).cloned().unwrap_or_else(|| "all".into()); }
            _ => {}
        }
        i += 1;
    }
    let active_assumes = parse_assume_selection(&assumes_arg);
    let start = std::time::Instant::now();

    // Initial configuration (Fsm::new steps sample 0; an assumption violated
    // at sample 0 panics, as in Phase 3).
    let mut init_fsm = Fsm::new();
    init_fsm.active_assumes = active_assumes.clone();
    if let Some(pv) = &init_fsm.property_violation {
        println!("FAIL");
        println!("{}", exact_summary_line("fail", 1, 0, 0, Some(("property-violation", &pv.label, pv.step))));
        println!("property violation: {}", pv.label);
        if let Some(t) = pv.trigger_step { println!("  trigger step: {} (initial configuration)", t); }
        println!("  violation step: {} (initial configuration)", pv.step);
        print_trace(&init_fsm.trace);
        std::process::exit(1);
    }
    let node0 = Node { state: init_fsm.state.clone(), vars: init_fsm.vars.clone(), prop: init_fsm.prop_monitors.clone(), assume: init_fsm.assume_monitors.clone() };

    // Level-synchronous BFS (D-P4-2): canonical input order, successors in
    // firing order; visited set checked at enqueue time; parent pointers for
    // counterexample reconstruction.
    let inputs = all_inputs();
    let mut visited: std::collections::HashMap<Vec<u8>, usize> = std::collections::HashMap::new();
    let mut nodes: Vec<Node> = vec![node0];
    let mut parent: Vec<Option<(usize, usize, usize)>> = vec![None];
    let mut depth: Vec<usize> = vec![0];
    // Explored graph edges (liveness analysis, D-P4-10): per config,
    // (input index, transition id, target config) in discovery order.
    let mut adj: Vec<Vec<(usize, usize, usize)>> = vec![Vec::new()];
    let mut complete: Vec<bool> = vec![false];
    visited.insert(config_bytes(nodes[0].state.clone(), &nodes[0].vars, &nodes[0].prop, &nodes[0].assume), 0);

    let mut frontier: Vec<usize> = vec![0];
    let mut max_depth = 0usize;
    let mut transitions = 0usize;
    let mut unexpected = 0usize;
    let mut first_unexpected: Option<(usize, usize)> = None;
    // status: 0 running, 1 pass-complete, 2 state limit, 3 timeout, 4 fail
    let mut status = 0u8;
    let mut fail_wire = "";
    let mut fail_label = String::new();
    let mut fail_step = 0usize;
    let mut fail_path: Vec<(usize, usize, usize)> = Vec::new();
    let mut is_deadlock = false;

    // Parallel level expansion (task 10B): with --threads N > 1, phase A
    // computes every successor outcome of the current level in parallel
    // (workers own disjoint frontier chunks; no shared mutable state), and
    // phase B commits them SEQUENTIALLY in canonical order - visited insert
    // at enqueue, parent/depth/adj pushes, first violation, deadlock check -
    // exactly as the single-threaded engine does. Results are therefore a
    // pure function of the spec: byte-identical to --threads 1 for any N.
    // --fast (non-parity): phase A stops scheduling work once any worker
    // finds a violation, and the first found violation is reported.
    'outer: while !frontier.is_empty() {
        if timeout_sec > 0 && start.elapsed().as_secs() >= timeout_sec { status = 3; break; }
        let mut next_frontier: Vec<usize> = Vec::new();
        let n_threads = threads.min(frontier.len());

        // Phase A: expand the level - sequentially (n_threads == 1) or in
        // parallel over disjoint frontier chunks. Outcomes per config are in
        // canonical position order (input x effect).
        let abort = std::sync::atomic::AtomicBool::new(false);
        let outcomes: Vec<Vec<LevelOutcome>> = if n_threads <= 1 {
            frontier.iter().map(|&cid| expand_config(cid, &nodes, &depth, &inputs, nondet, &active_assumes)).collect()
        } else {
            // Shared read-only state as pre-bound references: each worker
            // closure captures only Copy values (references + primitives), so
            // no engine data is moved and loop variables are never shared.
            let nodes_ref = &nodes;
            let depth_ref = &depth;
            let inputs_ref = &inputs;
            let assumes_ref = &active_assumes;
            let frontier_ref = &frontier;
            let abort_ref = &abort;
            std::thread::scope(|s| {
                let mut handles = Vec::new();
                let chunk = (frontier.len() + n_threads - 1) / n_threads;
                for w in 0..n_threads {
                    let lo = w * chunk;
                    if lo >= frontier.len() { break; }
                    let range = (lo, ((w + 1) * chunk).min(frontier.len()));
                    handles.push(s.spawn(move || {
                        frontier_ref[range.0..range.1].iter().map(|&cid| {
                            if fast && abort_ref.load(std::sync::atomic::Ordering::Relaxed) { return Vec::new(); }
                            let outs = expand_config(cid, nodes_ref, depth_ref, inputs_ref, nondet, assumes_ref);
                            if fast && outs.iter().any(|o| matches!(o, LevelOutcome::Violation { .. })) {
                                abort_ref.store(true, std::sync::atomic::Ordering::Relaxed);
                            }
                            outs
                        }).collect::<Vec<_>>()
                    }));
                }
                // Flatten the per-worker chunks (in frontier order) into one
                // outcome vec per config.
                handles
                    .into_iter()
                    .map(|h| h.join().unwrap())
                    .collect::<Vec<_>>()
                    .into_iter()
                    .flatten()
                    .collect::<Vec<_>>()
            })
        };

        // --fast early abort (non-parity): report the first violation found.
        // Which configs were expanded before the abort is race-dependent, so
        // states/transitions/label may vary between runs (never a false pass).
        if fast && n_threads > 1 && abort.load(std::sync::atomic::Ordering::Relaxed) {
            'scan: for (j, &cid) in frontier.iter().enumerate() {
                for out in &outcomes[j] {
                    if let LevelOutcome::Violation { iidx, tid, label, step, .. } = out {
                        status = 4; fail_wire = "property-violation"; fail_label = label.clone(); fail_step = *step;
                        let mut p = path_to(&parent, cid);
                        p.push((cid, *iidx, *tid));
                        fail_path = p;
                        break 'scan;
                    }
                }
            }
            // Unreachable: the flag is only set after a Violation outcome is
            // stored. If it ever triggers, report unknown (never a false pass).
            if status != 4 { status = 2; }
            break 'outer;
        }

        // Phase B: sequential canonical commit - identical to the pre-task-10B
        // single-threaded order for every observable counter and pointer.
        'level: for (j, &cid) in frontier.iter().enumerate() {
            let d = depth[cid];
            let mut any_successor = false;
            for out in &outcomes[j] {
                match out {
                    LevelOutcome::Unexpected(iidx) => {
                        // Input matches no transition and there is no on_error:
                        // this path terminates (D-P4-5); the configuration itself
                        // may still have other successors. (No transition was
                        // evaluated, so `transitions` is untouched - as before.)
                        unexpected += 1;
                        if first_unexpected.is_none() { first_unexpected = Some((cid, *iidx)); }
                    }
                    LevelOutcome::Pruned => {
                        transitions += 1;
                    }
                    LevelOutcome::Violation { iidx, tid, label, step, .. } => {
                        transitions += 1;
                        status = 4; fail_wire = "property-violation"; fail_label = label.clone(); fail_step = *step;
                        let mut p = path_to(&parent, cid);
                        p.push((cid, *iidx, *tid));
                        fail_path = p;
                        break 'level;
                    }
                    LevelOutcome::Success { iidx, tid, node } => {
                        transitions += 1;
                        any_successor = true;
                        let bytes = config_bytes(node.state.clone(), &node.vars, &node.prop, &node.assume);
                        let tcid = match visited.get(&bytes).copied() {
                            Some(x) => x,
                            None => {
                                if nodes.len() >= max_states { status = 2; break 'level; }
                                visited.insert(bytes, nodes.len());
                                parent.push(Some((cid, *iidx, *tid)));
                                depth.push(d + 1);
                                if d + 1 > max_depth { max_depth = d + 1; }
                                adj.push(Vec::new());
                                complete.push(false);
                                nodes.push(node.clone());
                                next_frontier.push(nodes.len() - 1);
                                nodes.len() - 1
                            }
                        };
                        // Graph edge: recorded for every Success outcome,
                        // including edges to already-visited configs.
                        adj[cid].push((*iidx, *tid, tcid));
                    }
                }
            }
            if status == 4 || status == 2 { break 'outer; }
            complete[cid] = true;
            if !any_successor {
                // Deadlock (D-P4-5): no successor at all - every input was
                // unexpected or pruned by an active assumption.
                status = 4; fail_wire = "deadlock"; fail_label = "-".into(); fail_step = d;
                fail_path = path_to(&parent, cid);
                is_deadlock = true;
                break 'outer;
            }
        }
        frontier = next_frontier;
    }
    if status == 0 { status = 1; }

    // Liveness analysis (D-P4-10/11): runs only when BFS ended without a
    // safety failure or deadlock. On a truncated graph (state limit / timeout)
    // this is a TRUNCATED analysis: a witness found inside the explored part
    // is sound (all its edges are real transitions), otherwise the UNKNOWN
    // status stands - never a false pass-complete.
    let graph = LivGraph { nodes: &nodes, adj: &adj, complete: &complete, inputs: &inputs, nondet };
    let mut liv_result: Option<(usize, LivWitness)> = None;
    if status != 4 && !LIV_PROPS.is_empty() {
        for (pi, mk) in LIV_PROPS.iter().enumerate() {
            if let Some(w) = liv_analyze(&graph, &mk()) {
                liv_result = Some((pi, w));
                break;
            }
        }
        if let Some((pi, w)) = &liv_result {
            status = 4;
            fail_wire = "liveness-violation";
            fail_label = LIV_PROPS[*pi]().label.to_string();
            fail_step = w.cycle_start;
            fail_path = w.lasso.clone();
        }
    }

    // Report (D-P4-4): human line, pinned machine line, statistics.
    let states = nodes.len();
    let mem_est = states * (CONFIG_TOTAL_BYTES + 32);
    match status {
        1 => println!("PASS (complete)"),
        4 => println!("FAIL"),
        _ => println!("UNKNOWN (state limit reached{})", if status == 3 { format!(", timeout after {}s", start.elapsed().as_secs()) } else { String::new() }),
    }
    let fail = if status == 4 { Some((fail_wire, fail_label.as_str(), fail_step)) } else { None };
    let st_wire = match status { 1 => "pass-complete", 4 => "fail", _ => "unknown-state-limit" };
    println!("{}", exact_summary_line(st_wire, states, transitions, max_depth, fail));
    println!("  unexpected terminations: {}", unexpected);
    if let Some((cid, iidx)) = first_unexpected { println!("  first unexpected: config {} input index {}", cid, iidx); }
    // Statistics block (task 10): identical field set and format in both
    // backends - monitor size is the encoded bit width of all property +
    // assumption monitors (deterministic; feeds CONFIG_TOTAL_BYTES).
    println!("  monitor size: {} bits", MONITORS_BITS);
    println!("  memory estimate: {} bytes", mem_est);

    if status == 4 {
        // Counterexample reconstruction (Phase 3 trace format).
        let fsm = replay(&fail_path, nondet, &active_assumes);
        if let Some((pi, w)) = &liv_result {
            let prop = LIV_PROPS[*pi]();
            println!("liveness violation: {}", fail_label);
            println!("  formula: {}", prop.formula);
            println!("  violation step: {}", fail_step);
            println!("  cycle length: {}", w.lasso.len() - w.cycle_start);
            println!("  note: {}", liv_note(&graph, &prop, w));
            print_trace(&fsm.trace);
            std::process::exit(1);
        }
        if is_deadlock {
            println!("deadlock: configuration at depth {} has no enabled transition", fail_step);
        } else {
            let pv = fsm.property_violation.as_ref().expect("violation recorded");
            println!("property violation: {}", pv.label);
            if let Some(t) = pv.trigger_step {
                println!("  trigger step: {}", t);
                println!("  vars at trigger step: {}", vars_at(&fsm.trace, t));
            }
            println!("  violation step: {}", pv.step);
            println!("  vars at violation step: {}", vars_at(&fsm.trace, pv.step));
        }
        print_trace(&fsm.trace);
        std::process::exit(1);
    }
    std::process::exit(if status == 1 { 0 } else { 2 });
}
"#;

/// Mode dispatch (D-P4-7): `--mode fuzz` selects the Phase 3 runtime.
const MAIN_DISPATCH: &str = r#"
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut mode = DEFAULT_MODE;
    let mut i = 1;
    while i < args.len() {
        if args[i] == "--mode" {
            i += 1;
            mode = match args.get(i).map(|s| s.as_str()) { Some("fuzz") => 1, _ => 0 };
        }
        i += 1;
    }
    if mode == 1 { run_fuzz(); } else { run_exact(); }
}
"#;
/// The model-checker fuzz runtime (Phase 3, unchanged except `main` ->
/// `run_fuzz`).
const FUZZ_RUNTIME: &str = r#"

struct Rng(u64);
impl Rng {
    fn new(seed: u64) -> Self { Self(seed) }
    fn next_f64(&mut self) -> f64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((self.0 >> 11) as f64) / ((u64::MAX >> 11) as f64)
    }
    fn chance(&mut self, p: f64) -> bool { self.next_f64() < p.clamp(0.0, 1.0) }
    fn pick_index(&mut self, len: usize) -> Option<usize> { if len == 0 { None } else { Some((self.next_f64() * len as f64) as usize % len) } }
}

struct ViolationSpec { label: String, prob: f64, percent: f64 }

fn parse_violation_spec(s: &str) -> ViolationSpec {
    let mut parts = s.splitn(3, ':');
    let label = parts.next().unwrap_or("").to_string();
    let prob = parts.next().and_then(|x| x.parse::<f64>().ok()).unwrap_or(1.0);
    let percent = parts.next().map(|x| x.trim_end_matches('%').parse::<f64>().unwrap_or(100.0)).unwrap_or(100.0);
    ViolationSpec { label, prob, percent }
}

fn parse_assume_selection(s: &str) -> Vec<String> {
    match s {
        "all" => ASSUME_LABELS.iter().map(|x| x.to_string()).collect(),
        "none" => Vec::new(),
        other => other.split(',').map(|x| x.trim().to_string()).filter(|x| !x.is_empty() && ASSUME_LABELS.contains(&x.as_str())).collect(),
    }
}

fn print_trace(trace: &Trace) {
    println!("counterexample trace:");
    println!("  initial state={:?} vars={:?}", trace.initial_state, trace.initial_vars);
    for (i, step) in trace.steps.iter().enumerate() {
        println!("  step {} input={:?} outputs={:?} from={:?} to={:?} vars={:?}", i, step.input, step.outputs, step.from_state, step.to_state, step.vars);
    }
}

/// Variable values at sample `pos` (0 = initial configuration).
fn vars_at(trace: &Trace, pos: usize) -> String {
    if pos == 0 { format!("{:?}", trace.initial_vars) } else { format!("{:?}", trace.steps[pos - 1].vars) }
}

/// Print the property-specific counterexample (which property failed, which
/// step triggered the obligation, when the violation was confirmed, relevant
/// variable values, full input/output sequence) and exit.
fn report_property_violation(fsm: &Fsm, run: usize) -> ! {
    let pv = fsm.property_violation.as_ref().expect("violation recorded");
    println!("run {} property violation: {}", run, pv.label);
    if let Some(t) = pv.trigger_step {
        println!("  trigger step: {}", t);
        println!("  vars at trigger step: {}", vars_at(&fsm.trace, t));
    }
    println!("  violation step: {}", pv.step);
    println!("  vars at violation step: {}", vars_at(&fsm.trace, pv.step));
    print_trace(&fsm.trace);
    std::process::exit(1);
}

fn run_fuzz() {
    let args: Vec<String> = std::env::args().collect();
    let mut bound = 5usize;
    let mut runs = 20usize;
    let mut seed = 12345u64;
    let mut assumes_arg = "all".to_string();
    let mut violate_arg: Vec<String> = Vec::new();
    let mut unexpected_prob = 0.0f64;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--bound" => { i += 1; bound = args.get(i).and_then(|x| x.parse().ok()).unwrap_or(5); }
            "--runs" => { i += 1; runs = args.get(i).and_then(|x| x.parse().ok()).unwrap_or(20); }
            "--seed" => { i += 1; seed = args.get(i).and_then(|x| x.parse().ok()).unwrap_or(12345); }
            "--assumes" => { i += 1; assumes_arg = args.get(i).cloned().unwrap_or_else(|| "all".into()); }
            "--violate" => { i += 1; if let Some(x) = args.get(i) { violate_arg.extend(x.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty())); } }
            "--unexpected-prob" => { i += 1; unexpected_prob = args.get(i).and_then(|x| x.parse().ok()).unwrap_or(0.0); }
            _ => {}
        }
        i += 1;
    }
    let specs: Vec<ViolationSpec> = violate_arg.iter().map(|s| parse_violation_spec(s)).collect();
    let active_assumes = parse_assume_selection(&assumes_arg);
    let mut rng = Rng::new(seed);
    // Property violations terminate the checker immediately (exit 1), so
    // this counter is always 0 in the summary; the field stays for a stable
    // summary format.
    let property_violations = 0usize;
    let mut unexpected_count = 0usize;
    let mut assumption_stops = 0usize;
    let mut completed = 0usize;

    for run in 0..runs {
        let violating: Vec<bool> = specs.iter().map(|s| rng.chance(s.prob)).collect();
        let target_fraction: Vec<f64> = specs.iter().map(|s| s.percent / 100.0).collect();
        let mut fsm = Fsm::new();
        fsm.active_assumes = active_assumes.clone();
        // A property can already be violated at sample 0 (the initial
        // configuration, e.g. an invariant on the initial state).
        if fsm.property_violation.is_some() { report_property_violation(&fsm, run); }
        let mut status = String::from("completed");
        for _step in 0..bound {
            // Try forced violation first (assumption or property labels).
            let mut forced = None;
            for (idx, spec) in specs.iter().enumerate() {
                if violating[idx] && rng.chance(target_fraction[idx]) {
                    if let Some(input) = ALL_INPUTS.iter().find(|input| {
                        let mut trial = fsm.clone();
                        let mut handler = StopAssumptionHandler;
                        match trial.step_with_handler(**input, &mut handler) {
                            Err(StepError::AssumptionViolation { label }) => label == spec.label,
                            Err(StepError::PropertyViolation { label, .. }) => label == spec.label,
                            _ => false,
                        }
                    }) {
                        forced = Some(*input);
                        break;
                    }
                }
            }
            if let Some(input) = forced {
                let mut handler = StopAssumptionHandler;
                match fsm.step_with_handler(input, &mut handler) {
                    Err(StepError::AssumptionViolation { label }) => { status = format!("assumption-violated:{}", label); assumption_stops += 1; break; }
                    _ => {}
                }
                if fsm.property_violation.is_some() { report_property_violation(&fsm, run); }
                continue;
            }

            // Optional unexpected injection.
            if rng.chance(unexpected_prob) {
                if let Some(idx) = rng.pick_index(ALL_INPUTS.len()) {
                    let input = ALL_INPUTS[idx];
                    let mut handler = ContinueAssumptionHandler;
                    if matches!(fsm.step_with_handler(input, &mut handler), Err(StepError::UnexpectedEvent)) {
                        status = String::from("unexpected-event");
                        unexpected_count += 1;
                        break;
                    }
                }
                if fsm.property_violation.is_some() { report_property_violation(&fsm, run); }
                continue;
            }

            // Default: choose an allowed representative input. Inputs that
            // confirm a property violation are allowed on purpose: the model
            // checker wants to find violations, not avoid them.
            let mut allowed: Vec<Input> = Vec::new();
            for input in ALL_INPUTS {
                let mut trial = fsm.clone();
                let mut handler = StopAssumptionHandler;
                match trial.step_with_handler(*input, &mut handler) {
                    Ok(_) => allowed.push(*input),
                    Err(StepError::PropertyViolation { .. }) => allowed.push(*input),
                    _ => {}
                }
            }
            if allowed.is_empty() { status = String::from("stuck"); break; }
            if let Some(idx) = rng.pick_index(allowed.len()) {
                let input = allowed[idx];
                let mut handler = StopAssumptionHandler;
                match fsm.step_with_handler(input, &mut handler) {
                    Ok(_) => {}
                    Err(StepError::UnexpectedEvent) => { status = String::from("unexpected-event"); unexpected_count += 1; break; }
                    Err(StepError::AssumptionViolation { label }) => { status = format!("assumption-violated:{}", label); assumption_stops += 1; break; }
                    Err(StepError::PropertyViolation { .. }) => {}
                }
            }
            if fsm.property_violation.is_some() { report_property_violation(&fsm, run); }
        }
        if status == "completed" { completed += 1; }
    }

    println!("model-check summary: runs={} bound={} completed={} unexpected={} assumption_stops={} property_violations={}", runs, bound, completed, unexpected_count, assumption_stops, property_violations);
}
"#;

/// SMT bounded-mode runtime (task 11). `{BOUND}` is replaced with the
/// codegen-time default (`--smt-bound`). The generated file = spec section
/// (smt_emit) + this engine. Parity-critical conventions: step variables are
/// z3 constants with fixed names declared in a fixed order; constraints are
/// asserted in a fixed order; queries run in a fixed order (safety properties
/// in declaration order, then liveness properties in declaration order). The
/// C backend mirrors all of this exactly.
const SMT_ENGINE_RUST: &str = r#"
// ===== SMT bounded-mode runtime (task 11) =====
const DEFAULT_BOUND: usize = {BOUND};

use z3::{Config, Model, SatResult, Solver, with_z3_config};

fn smt_read_bv(m: &Model, e: &BV) -> u64 {
    m.eval(e, true).expect("model evaluation").as_u64().expect("numeral value")
}

fn smt_read_bool(m: &Model, e: &Bool) -> bool {
    m.eval(e, true).expect("model evaluation").as_bool().expect("bool value")
}

/// Two-argument boolean connectives (z3 0.21 exposes variadic associated
/// functions; this wrapper keeps the rendered call sites uniform and maps to
/// Z3_mk_and/Z3_mk_or with two args - identical to the C backend).
fn b_and(a: &Bool, b: &Bool) -> Bool { Bool::and(&[a, b]) }
fn b_or(a: &Bool, b: &Bool) -> Bool { Bool::or(&[a, b]) }

/// Witness trace (pinned format - identical in both backends).
fn smt_print_trace(k: usize, m: &Model, s_vars: &[BV], v_vars: &[Vec<BV>], ev_vars: Option<&[BV]>, fld_vars: &[Vec<BV>]) {
    println!("witness trace (solver model; bound k={k}):");
    for i in 0..k {
        let si = smt_read_bv(m, &s_vars[i]) as usize;
        let mut vs = String::new();
        for j in 0..VAR_NAMES.len() {
            if j > 0 { vs.push_str(", "); }
            let val = smt_read_bv(m, &v_vars[i][j]);
            if VAR_IS_BOOL[j] {
                vs.push_str(&format!("{}={}", VAR_NAMES[j], if val != 0 { "true" } else { "false" }));
            } else {
                vs.push_str(&format!("{}={}", VAR_NAMES[j], val));
            }
        }
        let ev = if N_CLASSES > 1 {
            let ci = smt_read_bv(m, &ev_vars.expect("event vars")[i]) as usize;
            let mut fs = String::new();
            for g in 0..FIELD_POS.len() {
                if FIELD_POS[g].0 != ci { continue; }
                if !fs.is_empty() { fs.push_str(", "); }
                let val = smt_read_bv(m, &fld_vars[i][g]);
                if FIELD_IS_BOOL[g] {
                    fs.push_str(&format!("{}={}", FIELD_NAMES[g], if val != 0 { "true" } else { "false" }));
                } else {
                    fs.push_str(&format!("{}={}", FIELD_NAMES[g], val));
                }
            }
            format!("event={}({fs})", CLASS_NAMES[ci])
        } else {
            "event=-".to_string()
        };
        println!("  step {i}: state={} vars={{{vs}}} {ev}", STATES[si]);
    }
}

fn run_smt() -> i32 {
    // Runtime flags.
    let args: Vec<String> = std::env::args().collect();
    let mut bound = DEFAULT_BOUND;
    let mut ai = 1;
    while ai < args.len() {
        if args[ai] == "--bound" {
            ai += 1;
            bound = args.get(ai).and_then(|x| x.parse().ok()).unwrap_or(DEFAULT_BOUND);
        }
        ai += 1;
    }
    let k = bound.max(1);
    let cfg = Config::new();
    with_z3_config(&cfg, || {
        // ---- Step variables (fixed names/order - parity-critical) ----
        let s_vars: Vec<BV> = (0..k).map(|i| BV::new_const(format!("s_{i}"), STATE_BITS)).collect();
        let v_vars: Vec<Vec<BV>> = (0..k)
            .map(|i| VAR_WIDTHS.iter().enumerate().map(|(j, &w)| BV::new_const(format!("v_{i}_{j}"), w)).collect())
            .collect();
        let ev_vars: Option<Vec<BV>> = if N_CLASSES > 1 {
            Some((0..k).map(|i| BV::new_const(format!("ev_{i}"), CLS_BITS)).collect())
        } else {
            None
        };
        let fld_vars: Vec<Vec<BV>> = (0..k)
            .map(|i| FIELD_WIDTHS.iter().enumerate().map(|(g, &w)| BV::new_const(format!("fld_{i}_{g}"), w)).collect())
            .collect();

        // ---- Base constraints: initial configuration + k-step unrolling ----
        let mut base: Vec<Bool> = vec![s_vars[0].eq(&BV::from_u64(INITIAL_STATE, STATE_BITS))];
        for j in 0..VAR_WIDTHS.len() {
            base.push(v_vars[0][j].eq(&BV::from_u64(VAR_INITS[j], VAR_WIDTHS[j])));
        }
        let guards = guard_fns();
        let asgns = asgn_fns();
        let mut alive: Vec<Bool> = vec![Bool::from_bool(true)];
        for i in 0..k - 1 {
            // enabled_t = state match ∧ class match ∧ guard (firing order).
            let mut enableds: Vec<Bool> = Vec::new();
            for t in 0..TRANS.len() {
                let mut en = s_vars[i].eq(&BV::from_u64(TRANS[t].0 as u64, STATE_BITS));
                if N_CLASSES > 1 {
                    en = b_and(&en, &ev_vars.as_ref().unwrap()[i].eq(&BV::from_u64(TRANS[t].2 as u64, CLS_BITS)));
                }
                en = b_and(&en, &guards[t](i));
                enableds.push(en);
            }
            // fire_t = enabled_t ∧ no earlier transition enabled (first match).
            let mut fires: Vec<Bool> = Vec::new();
            let mut prefix_or: Option<Bool> = None;
            for t in 0..TRANS.len() {
                let fire = match &prefix_or {
                    Some(p) => b_and(&p.not(), &enableds[t]),
                    None => b_and(&enableds[t], &Bool::from_bool(true)),
                };
                // The and-true keeps the node fresh (no move out of the vec);
                // z3 rewrites it to `en` - the C backend does the same op.
                let new_prefix = match prefix_or {
                    Some(p) => b_or(&p, &enableds[t]),
                    None => b_and(&enableds[t], &Bool::from_bool(true)),
                };
                fires.push(fire);
                prefix_or = Some(new_prefix);
            }
            let any_fire = if fires.is_empty() {
                Bool::from_bool(false)
            } else {
                let mut acc: Option<Bool> = None;
                for f in &fires {
                    acc = Some(match acc {
                        Some(a) => b_or(&a, f),
                        None => b_and(f, &Bool::from_bool(true)),
                    });
                }
                acc.expect("non-empty fires")
            };
            alive.push(b_and(&alive[i], &any_fire));
            if TRANS.is_empty() {
                // No transitions: deadlocked from step 0; state/vars frozen.
                base.push(s_vars[i].eq(&s_vars[i + 1]));
                for j in 0..VAR_WIDTHS.len() {
                    base.push(v_vars[i][j].eq(&v_vars[i + 1][j]));
                }
            } else {
                // Next state: ITE chain over firing transitions (last first).
                let mut chain: BV = BV::from_u64(TRANS[TRANS.len() - 1].1 as u64, STATE_BITS);
                for t in (0..TRANS.len() - 1).rev() {
                    chain = fires[t].ite(&BV::from_u64(TRANS[t].1 as u64, STATE_BITS), &chain);
                }
                base.push(any_fire.ite(&chain, &s_vars[i]).eq(&s_vars[i + 1]));
                // Next vars: per-var ITE chains (unchanged when unassigned).
                for j in 0..VAR_WIDTHS.len() {
                    let mut acc: Option<BV> = None;
                    for t in (0..TRANS.len()).rev() {
                        if ASGN_MAP[t][j] {
                            let e = asgns[t][j](i);
                            acc = Some(match acc {
                                Some(a) => fires[t].ite(&e, &a),
                                None => fires[t].ite(&e, &v_vars[i][j]),
                            });
                        } else if let Some(a) = acc {
                            acc = Some(fires[t].ite(&v_vars[i][j], &a));
                        }
                    }
                    let next_v = match acc {
                        Some(a) => any_fire.ite(&a, &v_vars[i][j]),
                        None => any_fire.ite(&v_vars[i][j], &v_vars[i][j]),
                    };
                    base.push(next_v.eq(&v_vars[i + 1][j]));
                }
            }
        }

        // ---- Queries (fixed order: safety, then liveness; declaration order) ----
        let props = prop_fns();
        for p in 0..PROP_KINDS.len() {
            if PROP_KINDS[p] != 0 { continue; }
            // viol_p = ∃i<k: alive_i ∧ ¬phi_p(i).
            let mut viol: Option<Bool> = None;
            for i in 0..k {
                let phi = props[p](i);
                let bad = b_and(&alive[i], &phi.not());
                viol = Some(match viol { Some(v) => b_or(&v, &bad), None => bad });
            }
            let solver = Solver::new();
            for b in &base { solver.assert(b); }
            solver.assert(viol.expect("property formula"));
            match solver.check() {
                SatResult::Sat => {
                    let m = solver.get_model().expect("model");
                    let mut vstep = 0usize;
                    for i in 0..k {
                        if smt_read_bool(&m, &alive[i]) && !smt_read_bool(&m, &props[p](i)) {
                            vstep = i;
                            break;
                        }
                    }
                    println!("FAIL");
                    println!("model-check summary: status=fail mode=smt-bounded bound={k} reason=property-violation label={} step={vstep}", PROP_NAMES[p]);
                    println!("property violation: {}", PROP_NAMES[p]);
                    println!("  violation step: {vstep}");
                    smt_print_trace(k, &m, &s_vars, &v_vars, ev_vars.as_deref(), &fld_vars);
                    return 1;
                }
                SatResult::Unsat => {}
                SatResult::Unknown => {
                    println!("UNKNOWN (solver inconclusive within bound {k})");
                    println!("model-check summary: status=unknown mode=smt-bounded bound={k}");
                    return 2;
                }
            }
        }
        for p in 0..PROP_KINDS.len() {
            if PROP_KINDS[p] != 1 { continue; }
            // never_p = phi_p holds at no alive step within the bound.
            let mut never: Option<Bool> = None;
            for i in 0..k {
                let phi = props[p](i);
                let n = b_and(&alive[i], &phi).not();
                never = Some(match never { Some(v) => b_and(&v, &n), None => n });
            }
            // cycle = ∃a<b: identical (state, vars) - an infinite path.
            let mut cyc: Option<Bool> = None;
            for a in 0..k {
                for b in (a + 1)..k {
                    let mut eq = s_vars[a].eq(&s_vars[b]);
                    for j in 0..VAR_WIDTHS.len() {
                        eq = b_and(&eq, &v_vars[a][j].eq(&v_vars[b][j]));
                    }
                    cyc = Some(match cyc { Some(c) => b_or(&c, &eq), None => eq });
                }
            }
            if let Some(c) = cyc {
                let solver = Solver::new();
                for b in &base { solver.assert(b); }
                solver.assert(never.expect("liveness formula"));
                solver.assert(&c);
                match solver.check() {
                    SatResult::Sat => {
                        let m = solver.get_model().expect("model");
                        // Find the cycle pair from the model (first in order).
                        let cfgs: Vec<(u64, Vec<u64>)> = (0..k)
                            .map(|i| {
                                let s = smt_read_bv(&m, &s_vars[i]);
                                let vs: Vec<u64> = (0..VAR_WIDTHS.len()).map(|j| smt_read_bv(&m, &v_vars[i][j])).collect();
                                (s, vs)
                            })
                            .collect();
                        let mut pair = (0usize, 1usize);
                        'outer: for a in 0..k {
                            for b in (a + 1)..k {
                                if cfgs[a] == cfgs[b] { pair = (a, b); break 'outer; }
                            }
                        }
                        println!("FAIL");
                        println!("model-check summary: status=fail mode=smt-bounded bound={k} reason=liveness-violation label={} step={}", PROP_NAMES[p], pair.0);
                        println!("liveness violation: {}", PROP_NAMES[p]);
                        println!("  violation step: {}", pair.0);
                        println!("  cycle length: {}", pair.1 - pair.0);
                        smt_print_trace(k, &m, &s_vars, &v_vars, ev_vars.as_deref(), &fld_vars);
                        return 1;
                    }
                    SatResult::Unsat => {}
                    SatResult::Unknown => {
                        println!("UNKNOWN (solver inconclusive within bound {k})");
                        println!("model-check summary: status=unknown mode=smt-bounded bound={k}");
                        return 2;
                    }
                }
            }
        }
        // No violation found within the bound (incomplete - no claim).
        println!("PASS WITHIN BOUND {k} (incomplete - no completeness claim)");
        println!("model-check summary: status=pass-within-bound mode=smt-bounded bound={k}");
        0
    })
}

fn main() {
    std::process::exit(run_smt());
}
"#;
