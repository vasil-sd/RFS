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
    out.push_str(&exact_codegen(module, &plan, options).map_err(|e| vec![e])?);
    out.push_str(FUZZ_RUNTIME);
    out.push_str(MAIN_DISPATCH);
    Ok(CodegenOutput {
        code: out,
        warnings: fsm.warnings,
    })
}

/// Render the exact-mode runtime from the plan (D-P4-1..6).
fn exact_codegen(
    module: &Module,
    plan: &McPlan,
    options: &BackendOptions,
) -> Result<String, String> {
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
        "const CONFIG_TOTAL_BYTES: usize = {};\n",
        plan.encoding.total_bytes
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
    s.push_str(EXACT_RUN);
    Ok(s)
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
/// The exact-mode entry point (generated verbatim into every MC binary).
const EXACT_RUN: &str = r#"
fn run_exact() -> ! {
    // Runtime flags (codegen-time defaults, D-P4-7).
    let args: Vec<String> = std::env::args().collect();
    let mut max_states = DEFAULT_MAX_STATES;
    let mut nondet = DEFAULT_NONDET;
    let mut timeout_sec = DEFAULT_TIMEOUT_SEC;
    let mut assumes_arg = "all".to_string();
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--max-states" => { i += 1; max_states = args.get(i).and_then(|x| x.parse().ok()).unwrap_or(DEFAULT_MAX_STATES); }
            "--nondet" => { nondet = true; }
            "--timeout-sec" => { i += 1; timeout_sec = args.get(i).and_then(|x| x.parse().ok()).unwrap_or(DEFAULT_TIMEOUT_SEC); }
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

    'outer: while !frontier.is_empty() {
        if timeout_sec > 0 && start.elapsed().as_secs() >= timeout_sec { status = 3; break; }
        let mut next_frontier: Vec<usize> = Vec::new();
        for &cid in &frontier {
            let node = nodes[cid].clone();
            let d = depth[cid];
            let mut any_successor = false;
            for (iidx, input) in inputs.iter().enumerate() {
                let effects = mc_step_all(node.state.clone(), &node.vars, *input);
                if effects.is_empty() {
                    // Input matches no transition and there is no on_error:
                    // this path terminates (D-P4-5); the configuration itself
                    // may still have other successors.
                    unexpected += 1;
                    if first_unexpected.is_none() { first_unexpected = Some((cid, iidx)); }
                    continue;
                }
                let sel: Vec<&McEffect> = if nondet { effects.iter().collect() } else { vec![&effects[0]] };
                for eff in sel {
                    transitions += 1;
                    match step_successor(&node, d, *input, eff, &active_assumes) {
                        StepOutcome::Pruned => {}
                        StepOutcome::Violation(label, step, _trigger) => {
                            status = 4; fail_wire = "property-violation"; fail_label = label; fail_step = step;
                            let mut p = path_to(&parent, cid);
                            p.push((cid, iidx, eff.transition_id));
                            fail_path = p;
                            break 'outer;
                        }
                        StepOutcome::Success(n) => {
                            any_successor = true;
                            let bytes = config_bytes(n.state.clone(), &n.vars, &n.prop, &n.assume);
                            if !visited.contains_key(&bytes) {
                                if nodes.len() >= max_states { status = 2; break 'outer; }
                                visited.insert(bytes, nodes.len());
                                parent.push(Some((cid, iidx, eff.transition_id)));
                                depth.push(d + 1);
                                if d + 1 > max_depth { max_depth = d + 1; }
                                nodes.push(n);
                                next_frontier.push(nodes.len() - 1);
                            }
                        }
                    }
                }
            }
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
    println!("  memory estimate: {} bytes", mem_est);

    if status == 4 {
        // Counterexample reconstruction (Phase 3 trace format).
        let fsm = replay(&fail_path, nondet, &active_assumes);
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
