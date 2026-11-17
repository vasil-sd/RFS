//! Exact-mode engine of the reference model checker (Phase 16 task 1, D-P16-1).
//!
//! Moved verbatim from `ref_mc.rs` (pure relocation - zero behavior change):
//! the level-synchronous BFS over structured configuration keys, path/trace
//! reconstruction, the SCC-based liveness analysis (product graph + witnesses),
//! and the minimal-violation-label search. The common core it builds on
//! (values/typing, evaluator, signals, monitors, representative inputs,
//! effects, liveness decomposition, printing) stays in `ref_mc.rs`; the
//! counterexample verification there imports back the liveness helpers it
//! re-derives (`state_name`, `leaf_signals`, `leaf_bits`, `prod_delta`,
//! `leaf_good`, `skel_eval`).
//!
//! Independence scope (D-P16-2): this file imports nothing from within `mc/`
//! beyond the common core in `ref_mc.rs` - no `mc::plan`, `monitor`,
//! `codegen::fsm`, `printer`, or backend modules. Enforced by the
//! `independence_import_audit` test in `ref_mc.rs` (reads all `ref_mc_*.rs`).
//! Docs: dev:mc-engine-exact, dev:mc-engine-liveness, dev:algorithms-mc-engine, dev:references-model-checking, llm:mc-engine

use std::collections::{HashMap, HashSet};
use std::time::Instant;

use super::ref_mc::{
    all_inputs, bool_of, ceil_log2, compile_formula, compile_signal, contains_liveness,
    decompose_properties, eval_expr, eval_sig, flatten, input_group, input_matches_variant,
    match_effect, mon_bits, mon_init, mon_key, mon_step, output_variant_idx, pattern_bindings,
    state_index, state_variants, var_map, CeData, Effect, EvalCtx, FlatVariant, InputVal, IoRec,
    Leaf, LivProp, Mon, MonSt, OutputVal, RefConfig, RefResult, Sample, Sig, Skel, StepRec, Val,
};
use crate::codegen::mc_concepts::{parse_assume_selection, ExactStatus, FailReason};
use crate::lang::ir::*;

/// One BFS node: a configuration with its monitor runtime states.
#[derive(Clone)]
struct BNode {
    state: usize,
    vars: Vec<Val>,
    /// Monitor states in aggregate order (properties, then selected
    /// assumptions).
    mons: Vec<MonSt>,
}

/// Structured configuration key (no bit-packing - deliberate).
#[derive(Hash, Eq, PartialEq)]
struct CfgKey {
    state: usize,
    vars: Vec<u64>,
    /// Per monitor: the persistent projection (one u64 per persistent node).
    mons: Vec<Vec<u64>>,
}

/// Encode one value for the configuration key. Variable entries are typed by
/// position, so encoding is injective per variable: enums encode as their
/// variant index, bools as 0/1, bits as the value.
fn val_key(v: &Val) -> u64 {
    match v {
        Val::Bool(b) => *b as u64,
        Val::Bit(x) => *x,
        Val::Enum(_) => 0, // replaced below with the variant index
    }
}

fn bnode_key(n: &BNode, module: &Module, mon_specs: &[&Mon]) -> CfgKey {
    let vars = n
        .vars
        .iter()
        .zip(module.vars.iter())
        .map(|(val, vd)| match (val, &vd.ty) {
            (Val::Enum(name), Type::Enum { name: en }) => module
                .enums
                .iter()
                .find(|e| e.name == *en)
                .and_then(|e| e.variants.iter().position(|v| v == name))
                .unwrap_or(0) as u64,
            _ => val_key(val),
        })
        .collect();
    CfgKey {
        state: n.state,
        vars,
        mons: n
            .mons
            .iter()
            .zip(mon_specs.iter())
            .map(|(st, sp)| mon_key(st, sp))
            .collect(),
    }
}

/// ALL matching transition effects for `input` at state `state_idx`, in
/// firing order (effective priority descending, declaration order for ties) -
/// the `--nondet` successor strategy fires every one of them. The `on_error`
/// fallback is included ONLY when no transition matched; an empty result means
/// unexpected input (no match and no on_error). For a validated spec this is
/// exactly `match_effect`'s result as a one-element vec in the deterministic
/// case, so the two strategies share one matching semantics.
pub(crate) fn match_effects_all(
    module: &Module,
    state_idx: usize,
    vars: &[Val],
    input: &InputVal,
) -> Vec<Effect> {
    let states = state_variants(module).to_vec();
    let Some(state_name) = states.get(state_idx).cloned() else {
        return Vec::new();
    };
    let Some(group) = input_group(module, input) else {
        return Vec::new(); // internal invariant (validated spec)
    };

    // Firing order for this state (own computation of section 3.4).
    let mut ts: Vec<&Transition> = module
        .transitions
        .iter()
        .filter(|t| t.state == state_name)
        .collect();
    ts.sort_by_key(|t| std::cmp::Reverse(t.priority.unwrap_or(0)));

    let mut effects: Vec<Effect> = Vec::new();
    for t in &ts {
        if t.pattern.group != group {
            continue;
        }
        if !input_matches_variant(module, t, input) {
            continue;
        }
        let (binds, bind_types) = pattern_bindings(module, t, input).unwrap_or_default();
        let ctx = EvalCtx {
            module,
            state: Val::Enum(state_name.clone()),
            vars: var_map(module, vars),
            binds,
            bind_types,
        };
        if let Some(g) = &t.guard {
            // Validated specs always evaluate; an internal failure means "no
            // match" (the generated engines would not compile such a spec).
            if eval_expr(g, &ctx).ok().and_then(|v| bool_of(&v).ok()) == Some(false) {
                continue;
            }
        }
        // Emitted outputs (declaration order).
        let mut outputs: Vec<OutputVal> = Vec::new();
        let mut ok = true;
        for o in &t.emits {
            let args: Vec<Val> = match o
                .args
                .iter()
                .map(|a| eval_expr(a, &ctx))
                .collect::<Result<Vec<_>, _>>()
            {
                Ok(v) => v,
                Err(_) => {
                    ok = false;
                    break; // internal invariant (validated spec)
                }
            };
            match output_variant_idx(module, o) {
                Some(idx) => outputs.push(OutputVal {
                    variant: idx,
                    fields: args,
                }),
                None => {
                    ok = false;
                    break;
                }
            };
        }
        if !ok {
            continue; // skip this transition on internal failure
        }
        // Simultaneous updates: all RHS on OLD values.
        let mut next_vars = vars.to_vec();
        for a in &t.assignments {
            match eval_expr(&a.value, &ctx).ok() {
                Some(v) => {
                    if let Some(pos) = module.vars.iter().position(|x| x.name == a.var) {
                        next_vars[pos] = v;
                    }
                }
                None => {
                    ok = false;
                    break;
                }
            };
        }
        if !ok {
            continue;
        }
        let Some(next_state) = state_index(module, &t.next_state) else {
            continue;
        };
        effects.push(Effect {
            tid: t.id,
            next_state,
            next_vars,
            outputs,
        });
    }

    // on_error fallback: only when NO transition matched. No pattern bindings,
    // variables carry over unchanged.
    if effects.is_empty() {
        if let Some(oe) = &module.on_error {
            let ctx = EvalCtx {
                module,
                state: Val::Enum(state_name.clone()),
                vars: var_map(module, vars),
                binds: HashMap::new(),
                bind_types: HashMap::new(),
            };
            let mut outputs: Vec<OutputVal> = Vec::new();
            let mut ok = true;
            for o in &oe.emits {
                let args: Vec<Val> = match o
                    .args
                    .iter()
                    .map(|a| eval_expr(a, &ctx))
                    .collect::<Result<Vec<_>, _>>()
                {
                    Ok(v) => v,
                    Err(_) => {
                        ok = false;
                        break;
                    }
                };
                match output_variant_idx(module, o) {
                    Some(idx) => outputs.push(OutputVal {
                        variant: idx,
                        fields: args,
                    }),
                    None => {
                        ok = false;
                        break;
                    }
                };
            }
            if ok {
                if let Some(next_state) = state_index(module, &oe.next_state) {
                    effects.push(Effect {
                        tid: usize::MAX,
                        next_state,
                        next_vars: vars.to_vec(),
                        outputs,
                    });
                }
            }
        }
    }
    effects
}

/// Run the brute-force reference model checker on ONE validated module.
///
/// Mirrors the generated engines' DEFAULT exact mode (deterministic
/// first-match firing, seedless exhaustive level-synchronous BFS). Returns
/// `Err` for internal invariant failures (e.g. an initializer that references
/// another variable - such specs do not produce working generated binaries
/// either) and for an assumption already violated at the initial
/// configuration (the generated binaries panic there; a CLI tool reports a
/// clean error instead, exit code 1).
pub(crate) fn run(module: &Module, cfg: &RefConfig) -> Result<RefResult, String> {
    let t0 = Instant::now();

    // --- Monitors + liveness decomposition (own compilers). ---
    let mut skip_warnings = Vec::new();
    let mut prop_mons: Vec<(String, Mon)> = Vec::new();
    for p in &module.properties {
        if contains_liveness(&p.formula) {
            continue; // liveness properties are checked post-BFS, not monitored
        }
        match compile_formula(&p.formula) {
            Ok(mon) => prop_mons.push((p.name.clone(), mon)),
            Err(_) => skip_warnings.push(format!(
                "property '{}' is outside the monitor subset; skipped",
                p.name
            )),
        }
    }
    let known: Vec<&str> = module.assumptions.iter().map(|a| a.name.as_str()).collect();
    let selected = parse_assume_selection(&cfg.assume_arg, &known);
    let mut assume_mons: Vec<(String, Mon)> = Vec::new();
    for a in &module.assumptions {
        if !selected.iter().any(|x| x == &a.name) {
            continue;
        }
        match compile_formula(&a.formula) {
            Ok(mon) => assume_mons.push((a.name.clone(), mon)),
            Err(_) => skip_warnings.push(format!(
                "assume '{}' is outside the monitor subset; skipped",
                a.name
            )),
        }
    }
    let (liv_props_opt, liv_diagnostics) = decompose_properties(module);
    for d in &liv_diagnostics {
        skip_warnings.push(d.clone());
    }
    let liv_props: Vec<LivProp> = liv_props_opt.into_iter().flatten().collect();

    // --- Configuration encoding widths (documented formulas, SEMANTICS 9.2)
    //     - used for the `monitor size` + `memory estimate` statistics only.
    let state_count = state_variants(module).len();
    let state_bits = ceil_log2(state_count as u64);
    let vars_bits: u32 = module
        .vars
        .iter()
        .map(|v| match &v.ty {
            Type::Bool => 1,
            Type::Bit { width } => *width,
            Type::Enum { name } => ceil_log2(
                module
                    .enums
                    .iter()
                    .find(|e| e.name == *name)
                    .map(|e| e.variants.len() as u64)
                    .unwrap_or(0),
            ),
        })
        .sum();
    let monitors_bits: u32 = prop_mons
        .iter()
        .chain(assume_mons.iter())
        .map(|(_, m)| mon_bits(m))
        .sum();
    let total_bytes = ((state_bits + vars_bits + monitors_bits) as usize).div_ceil(8);

    // --- Initial configuration. ---
    let states = state_variants(module).to_vec();
    let q0 = module
        .initial_state
        .as_ref()
        .and_then(|s| states.iter().position(|x| x == s))
        .ok_or_else(|| "module has no initial state".to_string())?;
    let mut init_vars: Vec<Val> = Vec::new();
    for v in &module.vars {
        let ctx = EvalCtx {
            module,
            state: Val::Enum(states[q0].clone()),
            vars: HashMap::new(),
            binds: HashMap::new(),
            bind_types: HashMap::new(),
        };
        match eval_expr(&v.init, &ctx) {
            Ok(val) => init_vars.push(val),
            Err(_) => {
                return Err(format!(
                    "variable '{}' initializer is not a constant expression (unsupported)",
                    v.name
                ))
            }
        }
    }

    let all_mons: Vec<(String, Mon)> = prop_mons
        .iter()
        .cloned()
        .chain(assume_mons.iter().cloned())
        .collect();
    let n_props = all_mons.len() - assume_mons.len();
    let mon_specs: Vec<&Mon> = all_mons.iter().map(|(_, m)| m).collect();

    // Monitor states of the initial configuration; step sample 0.
    let mut mons: Vec<MonSt> = mon_specs.iter().copied().map(mon_init).collect();
    let sample0 = Sample {
        input: None,
        outputs: Vec::new(),
        to_state: q0,
        vars: init_vars.clone(),
    };
    // Assumptions first (the generated constructors panic on a sample-0
    // assumption violation; here: a clean error).
    for (i, (label, spec)) in all_mons.iter().enumerate().skip(n_props) {
        if mon_step(&mut mons[i], spec, 0, &sample0, module)? {
            return Err(format!(
                "assumption '{label}' violated at initial configuration (sample 0)"
            ));
        }
    }
    for (i, (label, spec)) in all_mons.iter().enumerate().take(n_props) {
        if mon_step(&mut mons[i], spec, 0, &sample0, module)? {
            // Sample-0 property violation: FAIL with the initial trace only.
            return Ok(RefResult {
                status: ExactStatus::Fail,
                states: 1,
                transitions: 0,
                max_depth: 0,
                fail: Some((FailReason::PropertyViolation, label.clone(), 0)),
                unexpected_terminations: 0,
                deadlock_mons: None,
                first_unexpected: None,
                monitor_size_bits: monitors_bits as usize,
                memory_estimate_bytes: total_bytes + 32,
                timeout_secs: None,
                skip_warnings,
                initial_state: states[q0].clone(),
                initial_vars: init_vars,
                ce: Some(CeData::Trace {
                    trigger_step: {
                        let t = mons[i].trigger_step();
                        (t >= 0).then_some(t as usize)
                    },
                    steps: Vec::new(),
                }),
            });
        }
    }

    // --- Level-synchronous BFS (SEMANTICS 9.5). ---
    let inputs = all_inputs(module);
    let in_variants = flatten(&module.events);
    let out_variants = flatten(&module.outputs);

    let node0 = BNode {
        state: q0,
        vars: init_vars.clone(),
        mons,
    };
    let mut visited: HashMap<CfgKey, usize> = HashMap::new();
    let mut nodes: Vec<BNode> = vec![node0];
    let mut parent: Vec<Option<(usize, usize, usize)>> = vec![None]; // (parent cid, iidx, tid)
    let mut depth: Vec<usize> = vec![0];
    let mut adj: Vec<Vec<(usize, usize, usize)>> = vec![Vec::new()]; // (iidx, tid, target)
    let mut complete: Vec<bool> = vec![false];
    visited.insert(bnode_key(&nodes[0], module, &mon_specs), 0);

    let mut frontier: Vec<usize> = vec![0];
    let mut max_depth = 0usize;
    let mut transitions = 0usize;
    let mut unexpected = 0usize;
    let mut first_unexpected: Option<(usize, usize)> = None;
    #[derive(PartialEq)]
    enum St {
        Running,
        Pass,
        StateLimit,
        Timeout,
        Fail,
    }
    let mut status = St::Running;
    let mut fail: Option<(FailReason, String, usize)> = None;
    let mut fail_path: Vec<(usize, usize, usize)> = Vec::new();
    let mut fail_trigger: Option<usize> = None;
    // Persistent monitor projection of the deadlocked terminal configuration
    // (SEMANTICS 8.4 deadlock depends on assumption-pruning state).
    let mut deadlock_mons: Option<Vec<Vec<u64>>> = None;

    while !frontier.is_empty() && status == St::Running {
        let mut next_frontier: Vec<usize> = Vec::new();
        'level: for &cid in &frontier {
            // Fine-grained deadline check (per configuration): ONE BFS level
            // can exceed the wall-clock budget on its own (e.g.
            // 19_guard_complex: one level runs for minutes), so checking only
            // between levels would let a single run blow past --timeout-sec.
            if cfg.timeout_sec > 0 && t0.elapsed().as_secs() >= cfg.timeout_sec {
                status = St::Timeout;
                break 'level;
            }
            let d = depth[cid];
            let node = nodes[cid].clone();
            let mut any_successor = false;
            for (iidx, input) in inputs.iter().enumerate() {
                // Successor strategy (mirrors the generated engines): deterministic
                // fires the FIRST matching effect only; --nondet fires EVERY matching
                // effect as its own successor. Empty = unexpected input.
                let effects: Vec<Effect> = if cfg.nondet {
                    match_effects_all(module, node.state, &node.vars, input)
                } else {
                    match_effect(module, node.state, &node.vars, input)
                        .into_iter()
                        .collect()
                };
                if effects.is_empty() {
                    // Unexpected input: this path terminates (SEMANTICS 8.3).
                    unexpected += 1;
                    if first_unexpected.is_none() {
                        first_unexpected = Some((cid, iidx));
                    }
                    continue;
                }
                for eff in effects {
                    transitions += 1;
                    let sample = Sample {
                        input: Some(input.clone()),
                        outputs: eff.outputs.clone(),
                        to_state: eff.next_state,
                        vars: eff.next_vars.clone(),
                    };
                    // Step the monitors at sample d + 1 (assumptions first: a
                    // pruned successor is never reported as a property violation).
                    let mut mons = node.mons.clone();
                    let mut pruned = false;
                    for (mi, (_, spec)) in all_mons.iter().enumerate().skip(n_props) {
                        if mon_step(&mut mons[mi], spec, (d + 1) as u64, &sample, module)? {
                            pruned = true;
                            break;
                        }
                    }
                    let mut violation: Option<(String, usize, Option<usize>)> = None;
                    if !pruned {
                        for (mi, (label, spec)) in all_mons.iter().enumerate().take(n_props) {
                            if mon_step(&mut mons[mi], spec, (d + 1) as u64, &sample, module)? {
                                violation = Some((label.clone(), d + 1, {
                                    let t = mons[mi].trigger_step();
                                    (t >= 0).then_some(t as usize)
                                }));
                                break;
                            }
                        }
                    }
                    if let Some((label, step, trigger)) = violation {
                        fail = Some((FailReason::PropertyViolation, label, step));
                        status = St::Fail;
                        fail_trigger = trigger;
                        let mut p = path_to(&parent, cid);
                        p.push((cid, iidx, eff.tid));
                        fail_path = p;
                        break 'level;
                    }
                    if pruned {
                        continue;
                    }
                    any_successor = true;
                    let next = BNode {
                        state: eff.next_state,
                        vars: eff.next_vars,
                        mons,
                    };
                    let key = bnode_key(&next, module, &mon_specs);
                    let tcid = match visited.get(&key).copied() {
                        Some(x) => x,
                        None => {
                            if nodes.len() >= cfg.max_states {
                                status = St::StateLimit;
                                break 'level;
                            }
                            visited.insert(key, nodes.len());
                            parent.push(Some((cid, iidx, eff.tid)));
                            depth.push(d + 1);
                            if d + 1 > max_depth {
                                max_depth = d + 1;
                            }
                            adj.push(Vec::new());
                            complete.push(false);
                            nodes.push(next);
                            next_frontier.push(nodes.len() - 1);
                            nodes.len() - 1
                        }
                    };
                    // Graph edge: recorded for every success (incl. to visited).
                    adj[cid].push((iidx, eff.tid, tcid));
                }
            }
            if status != St::Running {
                break;
            }
            complete[cid] = true;
            if !any_successor {
                // Deadlock (SEMANTICS 8.4): no successor at all.
                fail = Some((FailReason::Deadlock, "-".into(), d));
                status = St::Fail;
                fail_path = path_to(&parent, cid);
                deadlock_mons = Some(
                    nodes[cid]
                        .mons
                        .iter()
                        .zip(mon_specs.iter())
                        .map(|(st, sp)| mon_key(st, sp))
                        .collect(),
                );
                break;
            }
        }
        frontier = next_frontier;
    }
    if status == St::Running {
        status = St::Pass;
    }

    // --- Liveness analysis (own algorithm) on the completed or truncated
    //     graph - only when no safety failure / deadlock was found. A witness
    //     inside a truncated subgraph is SOUND (all its edges are real).
    let mut liv_result: Option<(usize, LivWitness)> = None;
    if status != St::Fail && !liv_props.is_empty() {
        for (pi, prop) in liv_props.iter().enumerate() {
            if let Some(w) = liv_analyze(module, &nodes, &adj, &complete, &inputs, prop, cfg.nondet)
            {
                liv_result = Some((pi, w));
                break;
            }
        }
        if let Some((pi, w)) = &liv_result {
            status = St::Fail;
            fail = Some((
                FailReason::LivenessViolation,
                liv_props[*pi].label.clone(),
                w.cycle_start,
            ));
            fail_path = w.lasso.clone(); // (src config, iidx, tid) edges
        }
    }

    // --- Assemble the result + counterexample data. ---
    let states_n = nodes.len();
    let mem_est = states_n * (total_bytes + 32);
    let status_out = match status {
        St::Pass => ExactStatus::PassComplete,
        St::Fail => ExactStatus::Fail,
        _ => ExactStatus::UnknownStateLimit,
    };
    let timeout_secs = matches!(status, St::Timeout).then(|| t0.elapsed().as_secs());

    let ce = match &fail {
        Some((FailReason::PropertyViolation, _, _)) => Some(CeData::Trace {
            trigger_step: fail_trigger,
            steps: trace_steps(
                module,
                &nodes,
                &inputs,
                &in_variants,
                &out_variants,
                &fail_path,
            ),
        }),
        Some((FailReason::Deadlock, _, d)) => Some(CeData::Deadlock {
            depth: *d,
            steps: trace_steps(
                module,
                &nodes,
                &inputs,
                &in_variants,
                &out_variants,
                &fail_path,
            ),
        }),
        Some((FailReason::LivenessViolation, _, _)) => {
            let (pi, w) = liv_result.as_ref().expect("liveness witness present");
            let prop = &liv_props[*pi];
            let steps = trace_steps(
                module,
                &nodes,
                &inputs,
                &in_variants,
                &out_variants,
                &w.lasso,
            );
            let note = liv_note(
                module,
                &nodes,
                &inputs,
                &in_variants,
                &out_variants,
                prop,
                w,
            );
            Some(CeData::Lasso {
                formula_text: prop.formula_text.clone(),
                cycle_start: w.cycle_start,
                note,
                steps,
            })
        }
        None => None,
    };

    Ok(RefResult {
        status: status_out,
        states: states_n,
        transitions,
        max_depth,
        fail,
        unexpected_terminations: unexpected,
        first_unexpected,
        deadlock_mons,
        monitor_size_bits: monitors_bits as usize,
        memory_estimate_bytes: mem_est,
        timeout_secs,
        skip_warnings,
        initial_state: states[q0].clone(),
        initial_vars: init_vars,
        ce,
    })
}

/// Steps from the root to configuration `cid`: (child config id, input idx,
/// transition id).
fn path_to(parent: &[Option<(usize, usize, usize)>], cid: usize) -> Vec<(usize, usize, usize)> {
    // Entries are (SOURCE config id, input index, transition id).
    let mut v = Vec::new();
    let mut c = cid;
    while let Some((p, i, t)) = parent[c] {
        v.push((p, i, t));
        c = p;
    }
    v.reverse();
    v
}

pub(crate) fn state_name(module: &Module, idx: usize) -> &str {
    &state_variants(module)[idx]
}

fn io_in(in_variants: &[FlatVariant], variant_idx: usize, input: &InputVal) -> IoRec {
    let vi = &in_variants[variant_idx];
    IoRec {
        variant: vi.unique_name.clone(),
        fields: vi
            .fields
            .iter()
            .map(|(n, _)| n.clone())
            .zip(input.fields.iter().cloned())
            .collect(),
    }
}

fn io_out(out_variants: &[FlatVariant], o: &OutputVal) -> IoRec {
    let vi = &out_variants[o.variant];
    IoRec {
        variant: vi.unique_name.clone(),
        fields: vi
            .fields
            .iter()
            .map(|(n, _)| n.clone())
            .zip(o.fields.iter().cloned())
            .collect(),
    }
}

/// Rebuild the trace steps of a path (each edge re-evaluated on its source
/// configuration - deterministic first-match, so effects are reproducible).
fn trace_steps(
    module: &Module,
    nodes: &[BNode],
    inputs: &[InputVal],
    in_variants: &[FlatVariant],
    out_variants: &[FlatVariant],
    path: &[(usize, usize, usize)],
) -> Vec<StepRec> {
    path.iter()
        .map(|(cid, iidx, _tid)| {
            let node = &nodes[*cid];
            let input = &inputs[*iidx];
            let eff = match_effect(module, node.state, &node.vars, input)
                .expect("deterministic replay: the recorded edge must still fire");
            StepRec {
                input: io_in(in_variants, input.variant, input),
                outputs: eff
                    .outputs
                    .iter()
                    .map(|o| io_out(out_variants, o))
                    .collect(),
                from_state: state_name(module, node.state).to_string(),
                to_state: state_name(module, eff.next_state).to_string(),
                vars: eff.next_vars,
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Liveness analysis (own algorithm - SCC-based, per SEMANTICS.md section 6)
// ---------------------------------------------------------------------------

/// A liveness counterexample: `lasso` edges in order; the first
/// `cycle_start` are the prefix (from config 0), the rest form a cycle back
/// to the config at position `cycle_start`.
#[derive(Debug, Clone)]
pub struct LivWitness {
    pub lasso: Vec<(usize, usize, usize)>,
    pub cycle_start: usize,
}

/// Per-sample leaf atoms of one liveness property (safety leaves +
/// obligation bodies as signals).
pub(crate) struct LeafSig {
    pub(crate) kind: u8, // 0 safety, 1 eventually, 2 until
    pub(crate) a: Sig,
    pub(crate) b: Option<Sig>,
}

pub(crate) fn leaf_signals(prop: &LivProp) -> Vec<LeafSig> {
    prop.leaves
        .iter()
        .map(|l| match l {
            Leaf::Safety(f) => LeafSig {
                kind: 0,
                a: compile_signal(f).expect("decomposition guarantees boolean-over-atoms"),
                b: None,
            },
            Leaf::Eventually(body) => LeafSig {
                kind: 1,
                a: compile_signal(body).expect("decomposition guarantees boolean-over-atoms"),
                b: None,
            },
            Leaf::Until { left, right } => LeafSig {
                kind: 2,
                a: compile_signal(left).expect("decomposition guarantees boolean-over-atoms"),
                b: Some(
                    compile_signal(right).expect("decomposition guarantees boolean-over-atoms"),
                ),
            },
        })
        .collect()
}

/// Evaluate the skeleton over per-leaf satisfaction flags.
pub(crate) fn skel_eval(s: &Skel, t: &[bool]) -> bool {
    match s {
        Skel::And(l, r) => skel_eval(l, t) && skel_eval(r, t),
        Skel::Or(l, r) => skel_eval(l, t) || skel_eval(r, t),
        Skel::Implies(l, r) => !skel_eval(l, t) || skel_eval(r, t),
        Skel::Not(i) => !skel_eval(i, t),
        Skel::Leaf(i) => t[*i],
    }
}

/// Analyze one liveness property on the explored graph. Returns a lasso
/// witness iff violated (SEMANTICS.md section 6.4/6.6).
fn liv_analyze(
    module: &Module,
    nodes: &[BNode],
    adj: &[Vec<(usize, usize, usize)>],
    complete: &[bool],
    inputs: &[InputVal],
    prop: &LivProp,
    nondet: bool,
) -> Option<LivWitness> {
    let leaves = leaf_signals(prop);
    let n = leaves.len();
    if n == 0 {
        return None;
    }
    let radices: Vec<usize> = leaves
        .iter()
        .map(|l| if l.kind == 2 { 3 } else { 2 })
        .collect();
    let mut places = vec![1usize; n];
    for k in (0..n - 1).rev() {
        places[k] = places[k + 1] * radices[k + 1];
    }
    let qcount: usize = radices.iter().product();

    // --- Position graph: position 0 = initial sample; one position per
    //     explored edge (edge labels are EDGE-based, SEMANTICS 6.5). ---
    let n_edges: usize = adj.iter().map(|e| e.len()).sum();
    let n_pos = 1 + n_edges;
    let mut pos_succ: Vec<Vec<usize>> = vec![Vec::new(); n_pos];
    // Position id of edge (cid, i) = 1 + sum_{c < cid} adj[c].len() + i.
    let mut prefix: Vec<usize> = vec![0usize; adj.len() + 1];
    for (c, es) in adj.iter().enumerate() {
        prefix[c + 1] = prefix[c] + es.len();
    }
    let pos_of_edge = |cid: usize, i: usize| 1 + prefix[cid] + i;
    // Inverse mapping: position -> (config id, adj index) of its edge.
    let mut pos_edge: Vec<Option<(usize, usize)>> = vec![None; n_pos];
    for cid in 0..adj.len() {
        for (i, _) in adj[cid].iter().enumerate() {
            pos_edge[pos_of_edge(cid, i)] = Some((cid, i));
        }
    }
    // Successors of position 0: outgoing edges of config 0. Successors of an
    // edge-position (u -> v): the outgoing edges of v.
    for (i, _) in adj[0].iter().enumerate() {
        pos_succ[0].push(pos_of_edge(0, i));
    }
    for cid in 0..adj.len() {
        for (i, &(_, _, v)) in adj[cid].iter().enumerate() {
            let p = pos_of_edge(cid, i);
            for (j, _) in adj[v].iter().enumerate() {
                pos_succ[p].push(pos_of_edge(v, j));
            }
        }
    }

    // --- Per-position label bits. ---
    let init_sample = Sample {
        input: None,
        outputs: Vec::new(),
        to_state: nodes[0].state,
        vars: nodes[0].vars.clone(),
    };
    let mut bits_at: Vec<u32> = vec![0u32; n_pos];
    bits_at[0] = leaf_bits(&leaves, &init_sample, module).ok()?;
    for cid in 0..adj.len() {
        let node = &nodes[cid];
        for (i, &(_iidx, _tid, _v)) in adj[cid].iter().enumerate() {
            let input = inputs[_iidx].clone();
            let eff = match_effect(module, node.state, &node.vars, &input)?;
            let sample = Sample {
                input: Some(input),
                outputs: eff.outputs.clone(),
                to_state: eff.next_state,
                vars: eff.next_vars.clone(),
            };
            bits_at[pos_of_edge(cid, i)] = leaf_bits(&leaves, &sample, module).ok()?;
        }
    }

    // --- Product reachability (BFS with parent pointers). ---
    let start_q = prod_delta(0, &places, &radices, &leaves, bits_at[0]);
    let mut pair_id: HashMap<(usize, usize), usize> = HashMap::new();
    let mut pair_pos: Vec<usize> = Vec::new(); // pair id -> position
    let mut pair_q: Vec<usize> = Vec::new();
    let mut prod_adj: Vec<Vec<usize>> = Vec::new();
    let mut parent_pair: Vec<Option<usize>> = Vec::new();
    let start_pid = 0usize;
    pair_id.insert((0, start_q), start_pid);
    pair_pos.push(0);
    pair_q.push(start_q);
    prod_adj.push(Vec::new());
    parent_pair.push(None);
    let mut queue: Vec<usize> = vec![start_pid];
    let mut qi = 0usize;
    while qi < queue.len() {
        let pid = queue[qi];
        qi += 1;
        let p = pair_pos[pid];
        let q = pair_q[pid];
        for &p2 in &pos_succ[p] {
            let q2 = prod_delta(q, &places, &radices, &leaves, bits_at[p2]);
            let key = (p2, q2);
            if let Some(&pid2) = pair_id.get(&key) {
                prod_adj[pid].push(pid2);
            } else {
                let npid = pair_pos.len();
                pair_id.insert(key, npid);
                pair_pos.push(p2);
                pair_q.push(q2);
                prod_adj.push(Vec::new());
                parent_pair.push(Some(pid));
                queue.push(npid);
                prod_adj[pid].push(npid);
            }
        }
    }

    // --- SCCs of the reachable product graph (iterative Tarjan). ---
    let npairs = pair_pos.len();
    let mut index_of = vec![usize::MAX; npairs];
    let mut lowlink = vec![0usize; npairs];
    let mut on_stack = vec![false; npairs];
    let mut scc_id = vec![usize::MAX; npairs];
    let mut counter = 0usize;
    let mut stack: Vec<usize> = Vec::new();
    let mut sccs: Vec<Vec<usize>> = Vec::new();
    for root in 0..npairs {
        if index_of[root] != usize::MAX {
            continue;
        }
        // Iterative DFS: (node, next-child-index).
        let mut work: Vec<(usize, usize)> = vec![(root, 0)];
        index_of[root] = counter;
        lowlink[root] = counter;
        counter += 1;
        stack.push(root);
        on_stack[root] = true;
        while let Some(&(node, ci)) = work.last() {
            if ci < prod_adj[node].len() {
                let w = prod_adj[node][ci];
                work.last_mut().unwrap().1 = ci + 1;
                if index_of[w] == usize::MAX {
                    index_of[w] = counter;
                    lowlink[w] = counter;
                    counter += 1;
                    stack.push(w);
                    on_stack[w] = true;
                    work.push((w, 0));
                } else if on_stack[w] {
                    lowlink[node] = lowlink[node].min(index_of[w]);
                }
            } else {
                work.pop();
                if let Some(&(parent_node, _)) = work.last() {
                    lowlink[parent_node] = lowlink[parent_node].min(lowlink[node]);
                }
                if lowlink[node] == index_of[node] {
                    let mut comp = Vec::new();
                    loop {
                        let w = stack.pop().unwrap();
                        on_stack[w] = false;
                        scc_id[w] = sccs.len();
                        comp.push(w);
                        if w == node {
                            break;
                        }
                    }
                    sccs.push(comp);
                }
            }
        }
    }

    // --- On-cycle pairs: non-trivial SCC members + self-loop singletons. ---
    let mut on_cycle = vec![false; npairs];
    for comp in &sccs {
        if comp.len() > 1 {
            for &p in comp {
                on_cycle[p] = true;
            }
        } else {
            let p = comp[0];
            if prod_adj[p].contains(&p) {
                on_cycle[p] = true;
            }
        }
    }

    // Strong-fairness eligibility (D-P4-11, nondet mode only): a config can sit
    // on a bad cycle only if it has EXACTLY ONE explored successor AND was fully
    // expanded - every enabled successor of a cycle state must be taken on the
    // cycle, so a branching or truncated config cannot be one. Position 0 (the
    // initial sample) maps to config 0. Deterministic mode: everything eligible.
    let eligible_cfg = |cid: usize| -> bool { !nondet || (adj[cid].len() == 1 && complete[cid]) };
    let eligible_pair = |pid: usize| -> bool {
        let p = pair_pos[pid];
        let sc = if p == 0 {
            0
        } else {
            pos_edge[p].expect("pair sits on an edge position").0
        };
        eligible_cfg(sc)
    };

    // --- Find a falsifying limit vector (q ascending, discovery order). ---
    let kinds: Vec<u8> = leaves.iter().map(|l| l.kind).collect();
    for q in 0..qcount {
        let t: Vec<bool> = (0..n)
            .map(|k| leaf_good(kinds[k], (q / places[k]) % radices[k]))
            .collect();
        if skel_eval(&prop.skeleton, &t) {
            continue; // satisfied for this limit vector
        }
        let start = pair_id.get(&(0, start_q)).copied().unwrap_or(usize::MAX);
        // nondet: try EVERY strongly-fair-eligible on-cycle pair at this q - the
        // first may sit in a mixed SCC whose cycle passes through ineligible
        // configs (the gated DFS cannot close it), so one candidate is not enough.
        let candidates: Vec<usize> = (start..npairs)
            .filter(|&pid| on_cycle[pid] && pair_q[pid] == q && eligible_pair(pid))
            .collect();
        for wpid in candidates {
            // Prefix: product path from the start pair to wpid (parent pointers).
            // Pairs P0..Pa with Pa = wpid; the edge taken by a product transition
            // X -> Y is the edge at position pos(Y), so the lasso's PREFIX edges
            // are the positions of P1..P_{a-1} (wpid's own edge starts the cycle).
            let mut prefix_pids: Vec<usize> = vec![wpid];
            let mut cur = wpid;
            while let Some(pp) = parent_pair[cur] {
                prefix_pids.push(pp);
                cur = pp;
            }
            prefix_pids.reverse();

            let mut lasso: Vec<(usize, usize, usize)> = Vec::new();
            for &pid in prefix_pids.iter().skip(1) {
                if pid == wpid {
                    continue; // wpid's edge belongs to the cycle (below)
                }
                let (sc, si) =
                    pos_edge[pair_pos[pid]].expect("prefix pair sits on an edge position");
                let (iidx, tid, _t) = adj[sc][si];
                lasso.push((sc, iidx, tid));
            }
            let cycle_start = lasso.len();

            // Cycle edges in order: wpid's own edge p_w=(u_w -> v_w) FIRST (after
            // the prefix we sit at u_w), then one edge per DFS descent. The
            // closing transition back to wpid re-takes p_w, so it is not recorded
            // again.
            {
                let (sc, si) = pos_edge[pair_pos[wpid]].expect("edge position");
                let (iidx, tid, _t) = adj[sc][si];
                lasso.push((sc, iidx, tid));
            }
            // DFS for the closing path back to wpid. GLOBAL visited pruning
            // (not just the current path): every reachable node is explored once,
            // so the search is O(V+E) - path-only pruning re-explores diamond
            // subgraphs exponentially and spins forever on small graphs (Phase 6
            // task 4 finding, specgen seed 900_006). Edges to wpid are checked
            // before the visited test, so any cycle through wpid is still found.
            let mut rec: Vec<usize> = Vec::new();
            let mut visited: HashSet<usize> = HashSet::new();
            visited.insert(wpid);
            let mut dfs: Vec<(usize, usize)> = vec![(wpid, 0)];
            let mut found = false;
            while let Some(&(at, ci)) = dfs.last() {
                if ci < prod_adj[at].len() {
                    let w = prod_adj[at][ci];
                    dfs.last_mut().unwrap().1 = ci + 1;
                    if w == wpid {
                        // Closing transition re-takes p_w - already pushed first.
                        found = true;
                        break;
                    }
                    // nondet: the cycle may only pass through strongly-fair-eligible
                    // configs (checked AFTER the closing edge, so any cycle through
                    // wpid is still found).
                    if !eligible_pair(w) {
                        continue;
                    }
                    if visited.insert(w) {
                        rec.push(pair_pos[w]);
                        dfs.push((w, 0));
                    }
                } else {
                    let (_node, _) = dfs.pop().unwrap();
                    if !dfs.is_empty() {
                        rec.pop();
                    }
                }
            }
            if !found {
                // nondet: this candidate's cycle passes through ineligible
                // configs - try the next candidate at the same q.
                continue;
            }
            for &p in &rec {
                let (sc, si) = pos_edge[p].expect("cycle pair sits on an edge position");
                let (iidx, tid, _t) = adj[sc][si];
                lasso.push((sc, iidx, tid));
            }
            return Some(LivWitness { lasso, cycle_start });
        }
    }
    None
}

/// Mixed-radix transition of the per-leaf progress automata on label bits.
/// Leaf states: safety 0=ok/1=bad; eventually 0=wait/1=done;
/// until 0=ok/1=sat/2=bad. All transitions are one-way, so every state
/// sequence on an infinite path is eventually constant (the limit vector).
pub(crate) fn prod_delta(
    q: usize,
    places: &[usize],
    radices: &[usize],
    leaves: &[LeafSig],
    bits: u32,
) -> usize {
    let mut out = 0usize;
    for (k, &r) in radices.iter().enumerate() {
        let s = (q / places[k]) % r;
        let a = (bits >> (2 * k)) & 1 == 1;
        let b = (bits >> (2 * k + 1)) & 1 == 1;
        let ns = match leaves[k].kind {
            0 => {
                if s == 1 || !a {
                    1
                } else {
                    0
                }
            }
            1 => {
                if s == 1 || a {
                    1
                } else {
                    0
                }
            }
            _ => match s {
                2 => 2,
                1 => 1,
                _ => {
                    if b {
                        1
                    } else if !a {
                        2
                    } else {
                        0
                    }
                }
            },
        };
        out = out * r + ns;
    }
    out
}

/// A leaf is satisfied on an infinite path iff its limit state is in the good
/// set.
pub(crate) fn leaf_good(kind: u8, s: usize) -> bool {
    match kind {
        0 => s == 0, // safety: never entered bad
        1 => s == 1, // eventually: done reached
        _ => s != 2, // until: ok (P-forever branch) or sat
    }
}

/// Label bits of one sample for the property's leaves: bit 2k = atom A,
/// bit 2k+1 = atom B (`until` right operand).
pub(crate) fn leaf_bits(leaves: &[LeafSig], s: &Sample, module: &Module) -> Result<u32, String> {
    let mut bits = 0u32;
    for (k, leaf) in leaves.iter().enumerate() {
        if eval_sig(&leaf.a, s, module)? {
            bits |= 1 << (2 * k);
        }
        if let Some(b) = &leaf.b {
            if eval_sig(b, s, module)? {
                bits |= 1 << (2 * k + 1);
            }
        }
    }
    Ok(bits)
}

/// Human-facing explanation of which leaves are falsified on the witness.
fn liv_note(
    module: &Module,
    nodes: &[BNode],
    inputs: &[InputVal],
    _in_variants: &[FlatVariant],
    _out_variants: &[FlatVariant],
    prop: &LivProp,
    w: &LivWitness,
) -> String {
    let leaves = leaf_signals(prop);
    let sample_at = |pos: usize| -> Result<Sample, String> {
        if pos == 0 {
            Ok(Sample {
                input: None,
                outputs: Vec::new(),
                to_state: nodes[0].state,
                vars: nodes[0].vars.clone(),
            })
        } else {
            let (cid, iidx, _tid) = w.lasso[pos - 1];
            let node = &nodes[cid];
            let input = inputs[iidx].clone();
            let eff = match_effect(module, node.state, &node.vars, &input)
                .ok_or_else(|| "lasso replay failed".to_string())?;
            Ok(Sample {
                input: Some(input),
                outputs: eff.outputs.clone(),
                to_state: eff.next_state,
                vars: eff.next_vars.clone(),
            })
        }
    };
    // Falsified leaves: those whose limit state is not good on the witness.
    let kinds: Vec<u8> = leaves.iter().map(|l| l.kind).collect();
    let radices: Vec<usize> = leaves
        .iter()
        .map(|l| if l.kind == 2 { 3 } else { 2 })
        .collect();
    let mut places = vec![1usize; leaves.len()];
    for k in (0..leaves.len() - 1).rev() {
        places[k] = places[k + 1] * radices[k + 1];
    }
    // Determine the limit vector from one cycle position (constant on the
    // cycle): run the automata over prefix + cycle and read the final states.
    let mut q = 0usize;
    q = prod_delta(
        q,
        &places,
        &radices,
        &leaves,
        leaf_bits(&leaves, &sample_at(0).unwrap(), module).unwrap(),
    );
    for pos in 1..=w.lasso.len() {
        let s = sample_at(pos).expect("valid lasso position");
        q = prod_delta(
            q,
            &places,
            &radices,
            &leaves,
            leaf_bits(&leaves, &s, module).expect("labels"),
        );
    }
    let falsified: Vec<usize> = (0..leaves.len())
        .filter(|&k| !leaf_good(kinds[k], (q / places[k]) % radices[k]))
        .collect();
    let mut parts: Vec<String> = Vec::new();
    for &k in &falsified {
        let leaf = &leaves[k];
        match leaf.kind {
            0 => {
                let p = (0..=w.lasso.len())
                    .find(|&pos| !eval_sig(&leaf.a, &sample_at(pos).unwrap(), module).unwrap())
                    .expect("safety leaf falsified on the witness");
                parts.push(format!("safety subformula false at step {p}"));
            }
            1 => parts.push("eventually body never holds on the path".to_string()),
            _ => {
                let b = leaf.b.as_ref().expect("until has a right operand");
                let mut seen_q = false;
                let mut p = None;
                for pos in 0..=w.lasso.len() {
                    let s = sample_at(pos).unwrap();
                    if !eval_sig(&leaf.a, &s, module).unwrap()
                        && !(seen_q || eval_sig(b, &s, module).unwrap())
                    {
                        p = Some(pos);
                        break;
                    }
                    seen_q |= eval_sig(b, &s, module).unwrap();
                }
                parts.push(format!(
                    "until left fails at step {} without right holding",
                    p.unwrap()
                ));
            }
        }
    }
    parts.join("; ")
}

// ---------------------------------------------------------------------------
// Minimal-violation labels (order-independent ground truth for task 4.1)
// ---------------------------------------------------------------------------

/// The sorted set of property labels violated at the GLOBAL MINIMAL depth,
/// independent of traversal order:
///
/// - safety violations count at their confirmation sample;
/// - liveness witnesses count at their cycle start position;
/// - deadlocks (label `-`) are excluded - they are covered by status
///   agreement in the differential suites.
///
/// The search is complete up to the first violation found in canonical order
/// (deeper violations cannot be shallower), so the set is exact for the
/// explored graph; with a truncating `max_states` it covers everything the
/// generated engines would have explored before stopping (sound: every label
/// returned has a real counterexample).
pub fn minimal_violation_labels(module: &Module, cfg: &RefConfig) -> Result<Vec<String>, String> {
    // Phase 1: canonical run to find the first violation in traversal order.
    let first = run(module, cfg)?;
    let Some((_, _, first_step)) = first.fail else {
        return Ok(Vec::new()); // pass-complete or unknown: no violations found
    };
    // Phase 2: re-run BFS but keep processing levels until level
    // `first_step - 1` is fully expanded, collecting every violation
    // confirmed at the minimal step.
    let mut labels: HashSet<String> = HashSet::new();
    collect_min_labels(module, cfg, first_step, &mut labels)?;

    // Liveness: witnesses from phase-1's graph (its cycle starts are exact
    // for the explored subgraph); include any at depth == the global min.
    let mut all: Vec<String> = labels.into_iter().collect();
    if let Some((FailReason::LivenessViolation, label, step)) = first.fail {
        if step <= first_step {
            all.push(label);
        }
    }
    // If the canonical run reported a liveness violation at `first_step` but
    // phase 2 found no safety tie, the liveness label still belongs.
    let mut seen: HashSet<String> = HashSet::new();
    let mut out: Vec<String> = all.into_iter().filter(|l| seen.insert(l.clone())).collect();
    out.sort();
    Ok(out)
}

/// BFS that does NOT stop at the first violation; collects labels of all
/// violations confirmed at step `target` (and records whether any shallower
/// violation exists - impossible by construction of `target`).
fn collect_min_labels(
    module: &Module,
    cfg: &RefConfig,
    target: usize,
    out: &mut HashSet<String>,
) -> Result<(), String> {
    let known: Vec<&str> = module.assumptions.iter().map(|a| a.name.as_str()).collect();
    let selected = parse_assume_selection(&cfg.assume_arg, &known);
    let mut prop_mons: Vec<(String, Mon)> = Vec::new();
    for p in &module.properties {
        if contains_liveness(&p.formula) {
            continue;
        }
        if let Ok(mon) = compile_formula(&p.formula) {
            prop_mons.push((p.name.clone(), mon));
        }
    }
    let mut assume_mons: Vec<(String, Mon)> = Vec::new();
    for a in &module.assumptions {
        if !selected.iter().any(|x| x == &a.name) {
            continue;
        }
        if let Ok(mon) = compile_formula(&a.formula) {
            assume_mons.push((a.name.clone(), mon));
        }
    }
    let all_mons: Vec<(String, Mon)> = prop_mons
        .iter()
        .cloned()
        .chain(assume_mons.iter().cloned())
        .collect();
    let n_props = prop_mons.len();
    let mon_specs: Vec<&Mon> = all_mons.iter().map(|(_, m)| m).collect();

    let states = state_variants(module).to_vec();
    let q0 = module
        .initial_state
        .as_ref()
        .and_then(|s| states.iter().position(|x| x == s))
        .ok_or_else(|| "module has no initial state".to_string())?;
    let mut init_vars: Vec<Val> = Vec::new();
    for v in &module.vars {
        let ctx = EvalCtx {
            module,
            state: Val::Enum(states[q0].clone()),
            vars: HashMap::new(),
            binds: HashMap::new(),
            bind_types: HashMap::new(),
        };
        init_vars.push(eval_expr(&v.init, &ctx)?);
    }

    let inputs = all_inputs(module);
    // Step the monitors at sample 0 (as `run` does): countdowns and one-shot
    // obligations must start from the initial configuration. No NEW failure
    // can occur here when target > 0 (run() would have reported it).
    let sample0 = Sample {
        input: None,
        outputs: Vec::new(),
        to_state: q0,
        vars: init_vars.clone(),
    };
    let mut mons0: Vec<MonSt> = mon_specs.iter().copied().map(mon_init).collect();
    let mut failed_at_0: Vec<bool> = Vec::new();
    for (i, (_, spec)) in all_mons.iter().enumerate() {
        failed_at_0.push(mon_step(&mut mons0[i], spec, 0, &sample0, module)?);
    }
    let node0 = BNode {
        state: q0,
        vars: init_vars,
        mons: mons0,
    };
    // A sample-0 violation is the minimal one: collect EVERY monitorable
    // property whose monitor fails at sample 0 (run() reports only the first
    // in declaration order). Selected assumptions cannot be failing here -
    // run() would have returned Err for a sample-0 assumption violation.
    if target == 0 {
        for ((label, _), failed) in prop_mons.iter().zip(failed_at_0.iter()) {
            if *failed {
                out.insert(label.clone());
            }
        }
        return Ok(());
    }

    let mut visited: HashMap<CfgKey, usize> = HashMap::new();
    let mut nodes: Vec<BNode> = vec![node0];
    visited.insert(bnode_key(&nodes[0], module, &mon_specs), 0);
    let mut frontier: Vec<usize> = vec![0];
    let mut depth_of: Vec<usize> = vec![0];

    while !frontier.is_empty() {
        let level_depth = depth_of[frontier[0]];
        if level_depth + 1 > target {
            break; // all further violations are deeper than `target`
        }
        let mut next_frontier: Vec<usize> = Vec::new();
        for &cid in &frontier {
            let node = nodes[cid].clone();
            for input in inputs.iter() {
                // Same successor strategy as run(): deterministic fires the first
                // match only; --nondet fires every matching effect.
                let effects: Vec<Effect> = if cfg.nondet {
                    match_effects_all(module, node.state, &node.vars, input)
                } else {
                    match_effect(module, node.state, &node.vars, input)
                        .into_iter()
                        .collect()
                };
                for eff in effects {
                    let sample = Sample {
                        input: Some(input.clone()),
                        outputs: eff.outputs.clone(),
                        to_state: eff.next_state,
                        vars: eff.next_vars.clone(),
                    };
                    let mut mons = node.mons.clone();
                    let mut pruned = false;
                    for (mi, (_, spec)) in all_mons.iter().enumerate().skip(n_props) {
                        if mon_step(
                            &mut mons[mi],
                            spec,
                            (level_depth + 1) as u64,
                            &sample,
                            module,
                        )? {
                            pruned = true;
                            break;
                        }
                    }
                    if !pruned {
                        for (mi, (label, spec)) in all_mons.iter().enumerate().take(n_props) {
                            if mon_step(
                                &mut mons[mi],
                                spec,
                                (level_depth + 1) as u64,
                                &sample,
                                module,
                            )? && level_depth + 1 == target
                            {
                                out.insert(label.clone());
                            }
                        }
                    }
                    let next = BNode {
                        state: eff.next_state,
                        vars: eff.next_vars,
                        mons,
                    };
                    let key = bnode_key(&next, module, &mon_specs);
                    if !visited.contains_key(&key) && nodes.len() < cfg.max_states {
                        visited.insert(key, nodes.len());
                        depth_of.push(level_depth + 1);
                        next_frontier.push(nodes.len());
                        nodes.push(next);
                    } else if !visited.contains_key(&key) {
                        // State cap reached: deeper exploration is impossible; the
                        // target level was already fully processed above.
                        return Ok(());
                    }
                }
            }
        }
        frontier = next_frontier;
    }
    Ok(())
}

// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mc::ref_mc::{render_text, to_json_report};
    use crate::parse_source;

    fn module(src: &str) -> Module {
        parse_source(src)
            .expect("parse")
            .modules
            .pop()
            .expect("module")
    }

    fn cfg() -> RefConfig {
        RefConfig::default()
    }

    // 1. Passing spec: A <-> B on Go.
    #[test]
    fn pass_spec_is_pass_complete() {
        let m = module(
            "module m { enum State { A, B } initial A event Go \
             transition A on Go { next B; } transition B on Go { next A; } }",
        );
        let r = run(&m, &cfg()).expect("run");
        assert_eq!(r.status, ExactStatus::PassComplete);
        assert_eq!(r.states, 2);
        assert_eq!(r.transitions, 2);
        assert_eq!(r.max_depth, 1);
        assert_eq!(r.unexpected_terminations, 0);
        assert!(r.fail.is_none());
        assert!(r.ce.is_none());
    }

    // 2. Safety FAIL with known label + step.
    #[test]
    fn safety_fail_reports_label_and_step() {
        let m = module(
            "module m { enum State { Good, Bad } initial Good \
             event Stay event ToBad event ToGood \
             transition Good on Stay { next Good; } \
             transition Good on ToBad { next Bad; } \
             transition Bad on ToGood { next Good; } \
             prop no_bad : always (state != Bad) ; }",
        );
        let r = run(&m, &cfg()).expect("run");
        assert_eq!(r.status, ExactStatus::Fail);
        assert_eq!(
            r.fail,
            Some((FailReason::PropertyViolation, "no_bad".into(), 1))
        );
        // Canonical order: Stay first (self-loop), then ToBad -> violation at step 1.
        let CeData::Trace {
            steps,
            trigger_step,
        } = r.ce.as_ref().unwrap()
        else {
            panic!("expected trace CE");
        };
        assert_eq!(*trigger_step, None);
        assert_eq!(steps.len(), 1);
        assert_eq!(steps[0].from_state, "Good");
        assert_eq!(steps[0].to_state, "Bad");
        assert_eq!(steps[0].input.variant, "ToBad");
    }

    // 3. Liveness FAIL with a lasso (eventually over an unreachable state).
    #[test]
    fn liveness_fail_reports_lasso() {
        let m = module(
            "module m { enum State { A, B, C } initial A event Go \
             transition A on Go { next B; } transition B on Go { next B; } \
             prop reaches_c : eventually (state == C) ; }",
        );
        let r = run(&m, &cfg()).expect("run");
        assert_eq!(r.status, ExactStatus::Fail);
        assert_eq!(
            r.fail,
            Some((FailReason::LivenessViolation, "reaches_c".into(), 1))
        );
        let CeData::Lasso {
            cycle_start,
            steps,
            note,
            formula_text,
        } = r.ce.as_ref().unwrap()
        else {
            panic!("expected lasso CE");
        };
        assert_eq!(*cycle_start, 1);
        assert_eq!(steps.len(), 2); // prefix A->B, cycle B->B
        assert_eq!(formula_text, "eventually (state == C)");
        assert_eq!(note, "eventually body never holds on the path");
    }

    // 4. Deadlock: terminal state with no outgoing transitions.
    #[test]
    fn deadlock_reports_terminal_configuration() {
        let m = module(
            "module m { enum State { A, T } initial A event Go \
             transition A on Go { next T; } }",
        );
        let r = run(&m, &cfg()).expect("run");
        assert_eq!(r.status, ExactStatus::Fail);
        assert_eq!(r.fail, Some((FailReason::Deadlock, "-".into(), 1)));
        let CeData::Deadlock { depth, steps } = r.ce.as_ref().unwrap() else {
            panic!("expected deadlock CE");
        };
        assert_eq!(*depth, 1);
        assert_eq!(steps.len(), 1);
        assert_eq!(steps[0].to_state, "T");
    }

    // 5. State-limit truncation: UnknownStateLimit, never a completeness claim.
    #[test]
    fn state_limit_is_unknown() {
        let m = module(
            "module m { enum State { A, B, C } initial A event Go \
             transition A on Go { next B; } transition B on Go { next C; } \
             transition C on Go { next C; } }",
        );
        let mut c = cfg();
        c.max_states = 1;
        let r = run(&m, &c).expect("run");
        assert_eq!(r.status, ExactStatus::UnknownStateLimit);
        assert_eq!(r.states, 1);
        assert_eq!(r.transitions, 1); // the counted success that hit the cap
        assert!(r.fail.is_none());

        let mut c = cfg();
        c.max_states = 2;
        let r = run(&m, &c).expect("run");
        assert_eq!(r.status, ExactStatus::UnknownStateLimit);
        assert_eq!(r.states, 2);
    }

    // Sample-0 property violation.
    #[test]
    fn sample_zero_violation() {
        let m = module(
            "module m { enum State { A, B } initial B event Go \
             transition B on Go { next A; } \
             prop starts_a : always (state == A) ; }",
        );
        let r = run(&m, &cfg()).expect("run");
        assert_eq!(r.status, ExactStatus::Fail);
        assert_eq!(
            r.fail,
            Some((FailReason::PropertyViolation, "starts_a".into(), 0))
        );
        assert_eq!(r.states, 1);
        let CeData::Trace { steps, .. } = r.ce.as_ref().unwrap() else {
            panic!("expected trace CE");
        };
        assert!(steps.is_empty());
    }

    // Assumption pruning: the violating input is not explored.
    #[test]
    fn assumption_prunes_successor() {
        // A -Go-> B, B -Go-> B; the assumption forbids two consecutive Gos,
        // so B's self-loop successor is pruned on every visit.
        let m = module(
            "module m { enum State { A, B } initial A event Go \
             transition A on Go { next B; } transition B on Go { next B; } \
             assume single_go : always (input_is(Go) -> next (!input_is(Go))) ; }",
        );
        let r = run(&m, &cfg()).expect("run");
        // B's Go successor is pruned at every visit; B has no other input with
        // a successor (Go is the only event) -> deadlock at depth 1.
        assert_eq!(r.status, ExactStatus::Fail);
        assert_eq!(r.fail, Some((FailReason::Deadlock, "-".into(), 1)));

        // With assumptions off, B's self-loop is explored: pass-complete.
        let mut c = cfg();
        c.assume_arg = "none".into();
        let r = run(&m, &c).expect("run");
        assert_eq!(r.status, ExactStatus::PassComplete);
    }

    // Bounded-eventually obligation: deadline semantics (N=2, one-shot at 0).
    #[test]
    fn bounded_eventually_deadline() {
        // Top-level bounded_eventually[2]: the body must hold within samples
        // 0..=1. Sent is first emitted at step 2, so the countdown expires:
        // sample 0 triggers (remaining 2 -> 1), sample 1 decrements to 0 and
        // confirms the violation - before the emit can clear it.
        let m = module(
            "module m { enum State { A, B } initial A event Go \
             transition A on Go { next B; } \
             transition B on Go { next B; emit Sent; } \
             output Sent \
             prop sent_soon : bounded_eventually[2] (output_has(Sent)) ; }",
        );
        let r = run(&m, &cfg()).expect("run");
        assert_eq!(r.status, ExactStatus::Fail);
        assert_eq!(
            r.fail,
            Some((FailReason::PropertyViolation, "sent_soon".into(), 1))
        );
        // The violating successor is never enqueued.
        assert_eq!(r.states, 1);
        assert_eq!(r.transitions, 1);
        let CeData::Trace {
            trigger_step,
            steps,
        } = r.ce.as_ref().unwrap()
        else {
            panic!("expected trace CE");
        };
        assert_eq!(*trigger_step, Some(0)); // the one-shot obligation started at sample 0
        assert_eq!(steps.len(), 1);
    }

    // Unexpected inputs terminate paths but do not fail the model.
    #[test]
    fn unexpected_inputs_are_not_failures() {
        let m = module(
            "module m { enum State { A, B } initial A event Go event Other \
             transition A on Go { next B; } transition B on Go { next A; } }",
        );
        let r = run(&m, &cfg()).expect("run");
        assert_eq!(r.status, ExactStatus::PassComplete);
        assert_eq!(r.unexpected_terminations, 2); // Other at A and at B
        assert_eq!(r.first_unexpected, Some((0, 1)));
    }

    // --- Byte-pinned reports (text + JSON) for the five outcomes above. ---

    #[test]
    fn pinned_pass_text_and_json() {
        let m = module(
            "module m { enum State { A, B } initial A event Go \
             transition A on Go { next B; } transition B on Go { next A; } }",
        );
        let r = run(&m, &cfg()).expect("run");
        assert_eq!(
            render_text(&r, &m),
            "PASS (complete)\n\
             model-check summary: status=pass-complete states=2 transitions=2 max_depth=1\n\
             \x20 unexpected terminations: 0\n\
             \x20 monitor size: 0 bits\n\
             \x20 memory estimate: 66 bytes\n"
        );
        let doc = to_json_report(&r, &m).to_json_string();
        assert_eq!(
            doc,
            "{\"status\":\"pass-complete\",\"metrics\":{\"states\":2,\"transitions\":2,\"max_depth\":1},\"statistics\":{\"unexpected_terminations\":0,\"monitor_size_bits\":0,\"memory_estimate_bytes\":66,\"peak_rss_kb\":0}}"
        );
    }

    #[test]
    fn pinned_safety_fail_text_and_json() {
        let m = module(
            "module m { enum State { Good, Bad } initial Good \
             event Stay event ToBad event ToGood \
             transition Good on Stay { next Good; } \
             transition Good on ToBad { next Bad; } \
             transition Bad on ToGood { next Good; } \
             prop no_bad : always (state != Bad) ; }",
        );
        let r = run(&m, &cfg()).expect("run");
        assert_eq!(
            render_text(&r, &m),
            "FAIL\n\
             model-check summary: status=fail states=1 transitions=2 max_depth=0 reason=property-violation label=no_bad step=1\n\
             \x20 unexpected terminations: 0\n\
             \x20 monitor size: 0 bits\n\
             \x20 memory estimate: 33 bytes\n\
             property violation: no_bad\n\
             \x20 violation step: 1\n\
             \x20 vars at violation step: Vars\n\
             counterexample trace:\n\
             \x20 initial state=State::Good vars=Vars\n\
             \x20 step 0 input=Input::ToBad outputs=[] from=State::Good to=State::Bad vars=Vars\n"
        );
        let doc = to_json_report(&r, &m).to_json_string();
        assert_eq!(
            doc,
            "{\"status\":\"fail\",\"metrics\":{\"states\":1,\"transitions\":2,\"max_depth\":0},\"reason\":\"property-violation\",\"label\":\"no_bad\",\"step\":1,\"statistics\":{\"unexpected_terminations\":0,\"monitor_size_bits\":0,\"memory_estimate_bytes\":33,\"peak_rss_kb\":0},\"counterexample\":{\"kind\":\"trace\",\"initial_state\":\"State::Good\",\"initial_vars\":{},\"steps\":[{\"input\":{\"variant\":\"ToBad\",\"fields\":{}},\"outputs\":[],\"from_state\":\"State::Good\",\"to_state\":\"State::Bad\",\"vars\":{}}],\"ce_steps\":1}}"
        );
    }

    #[test]
    fn pinned_deadlock_text() {
        let m = module(
            "module m { enum State { A, T } initial A event Go \
             transition A on Go { next T; } }",
        );
        let r = run(&m, &cfg()).expect("run");
        assert_eq!(
            render_text(&r, &m),
            "FAIL\n\
             model-check summary: status=fail states=2 transitions=1 max_depth=1 reason=deadlock label=- step=1\n\
             \x20 unexpected terminations: 1\n\
             \x20 first unexpected: config 1 input index 0\n\
             \x20 monitor size: 0 bits\n\
             \x20 memory estimate: 66 bytes\n\
             deadlock: configuration at depth 1 has no enabled transition\n\
             counterexample trace:\n\
             \x20 initial state=State::A vars=Vars\n\
             \x20 step 0 input=Input::Go outputs=[] from=State::A to=State::T vars=Vars\n"
        );
    }

    #[test]
    fn pinned_liveness_text() {
        let m = module(
            "module m { enum State { A, B, C } initial A event Go \
             transition A on Go { next B; } transition B on Go { next B; } \
             prop reaches_c : eventually (state == C) ; }",
        );
        let r = run(&m, &cfg()).expect("run");
        assert_eq!(
            render_text(&r, &m),
            "FAIL\n\
             model-check summary: status=fail states=2 transitions=2 max_depth=1 reason=liveness-violation label=reaches_c step=1\n\
             \x20 unexpected terminations: 0\n\
             \x20 monitor size: 0 bits\n\
             \x20 memory estimate: 66 bytes\n\
             liveness violation: reaches_c\n\
             \x20 formula: eventually (state == C)\n\
             \x20 violation step: 1\n\
             \x20 cycle length: 1\n\
             \x20 note: eventually body never holds on the path\n\
             counterexample trace:\n\
             \x20 initial state=State::A vars=Vars\n\
             \x20 step 0 input=Input::Go outputs=[] from=State::A to=State::B vars=Vars\n\
             \x20 step 1 input=Input::Go outputs=[] from=State::B to=State::B vars=Vars\n"
        );
    }

    #[test]
    fn minimal_labels_unique_and_tied() {
        // Unique minimal violation.
        let m = module(
            "module m { enum State { Good, Bad } initial Good \
             event Stay event ToBad event ToGood \
             transition Good on Stay { next Good; } \
             transition Good on ToBad { next Bad; } \
             transition Bad on ToGood { next Good; } \
             prop no_bad : always (state != Bad) ; }",
        );
        let labels = minimal_violation_labels(&m, &cfg()).expect("labels");
        assert_eq!(labels, vec!["no_bad".to_string()]);

        // No violations at all.
        let m2 = module(
            "module m { enum State { A, B } initial A event Go \
             transition A on Go { next B; } transition B on Go { next A; } }",
        );
        assert!(minimal_violation_labels(&m2, &cfg())
            .expect("labels")
            .is_empty());
    }

    // Timeout granularity: a SINGLE BFS level may exceed the wall-clock budget
    // (19_guard_complex: one level runs for minutes). The deadline check must
    // therefore fire mid-level, not only between levels - otherwise
    // --timeout-sec is silently ignored on such specs.
    #[test]
    fn timeout_fires_mid_level_on_giant_level() {
        // 1 state x bit<12> var; the event payload is referenced (assignment),
        // so it contributes its full domain: 4096 representative inputs. Level
        // 1 = 4095 configs x 4096 inputs (~1.7e7 transition steps) - one level
        // that takes several seconds on any host.
        let m = module(
            "module m { enum State { S } initial S \
             event E(P(p: bit<12>)) var x: bit<12> init 0 \
             transition S on E(P(p)) { next S; set x = p; } }",
        );
        let c = RefConfig {
            timeout_sec: 3,
            ..RefConfig::default()
        };
        let t0 = Instant::now();
        let r = run(&m, &c).expect("run");
        let elapsed = t0.elapsed().as_secs();
        assert_eq!(r.status, ExactStatus::UnknownStateLimit);
        assert!(r.timeout_secs.is_some(), "must record the timeout");
        assert!(
            elapsed <= 5,
            "timeout should fire shortly after the 3s budget (took {elapsed}s)"
        );
    }

    // Regression (Phase 6 task 4, correction 4a): a violation confirmed at
    // SAMPLE 0 must still yield the full minimal-label set - every
    // monitorable property failing at sample 0, not an empty set.
    #[test]
    fn minimal_labels_sample_zero_violation() {
        let m = module(
            "module m { enum State { S } initial S \
             event Stay \
             transition S on Stay { next S; } \
             prop p1 : true && false ; \
             prop p2 : always (false) ; }",
        );
        // run() reports the FIRST failing property in declaration order.
        let r = run(&m, &cfg()).expect("run");
        assert_eq!(
            r.fail.as_ref(),
            Some(&(FailReason::PropertyViolation, "p1".into(), 0))
        );
        // The minimal set carries BOTH labels (sorted).
        let labels = minimal_violation_labels(&m, &cfg()).expect("labels");
        assert_eq!(labels, vec!["p1".to_string(), "p2".to_string()]);
    }

    // --- Phase 16 task 2: --nondet successor strategy + liveness fairness. ---

    fn nondet_cfg() -> RefConfig {
        RefConfig {
            nondet: true,
            ..Default::default()
        }
    }

    /// Two transitions with the same pattern but different priorities BOTH match
    /// one input; match_effects_all returns both (firing order), match_effect only
    /// the first.
    #[test]
    fn match_effects_all_returns_every_matching_transition() {
        let m = module(
            "module m { enum State { A } initial A event Go var x: bit<2> init 0 \
             transition A on Go { next A; set x = 1; priority 1; } \
             transition A on Go { next A; set x = 2; priority 0; } }",
        );
        let go_idx = flatten(&m.events)
            .iter()
            .position(|v| v.unique_name == "Go")
            .expect("Go variant");
        let inputs = all_inputs(&m);
        let inp = &inputs[go_idx];
        let all = match_effects_all(&m, 0, &[Val::Bit(0)], inp);
        assert_eq!(all.len(), 2, "both matching transitions are effects");
        // Firing order: priority 1 (x=1) before priority 0 (x=2).
        assert_eq!(all[0].next_vars, vec![Val::Bit(1)]);
        assert_eq!(all[1].next_vars, vec![Val::Bit(2)]);
        // Deterministic first-match is the head of the same enumeration.
        let first = match_effect(&m, 0, &[Val::Bit(0)], inp).expect("a match");
        assert_eq!(first.next_vars, all[0].next_vars);
    }

    /// on_error is an effect ONLY when no transition matched; with no on_error an
    /// unmatched input yields an empty enumeration (unexpected).
    #[test]
    fn match_effects_all_on_error_fallback_and_empty() {
        let with_oe = module(
            "module m { enum State { A } initial A \
             event Go \
             event Other \
             transition A on Go { next A; } on_error { next A; } }",
        );
        let other_idx = flatten(&with_oe.events)
            .iter()
            .position(|v| v.unique_name == "Other")
            .expect("Other variant");
        let inputs = all_inputs(&with_oe);
        let other = &inputs[other_idx];
        let effs = match_effects_all(&with_oe, 0, &[], other);
        assert_eq!(effs.len(), 1, "on_error is the sole effect");
        assert_eq!(effs[0].tid, usize::MAX, "on_error carries the sentinel tid");

        let no_oe = module(
            "module m { enum State { A } initial A \
             event Go \
             event Other \
             transition A on Go { next A; } }",
        );
        let other2_idx = flatten(&no_oe.events)
            .iter()
            .position(|v| v.unique_name == "Other")
            .expect("Other variant");
        let inputs = all_inputs(&no_oe);
        let other2 = &inputs[other2_idx];
        assert!(
            match_effects_all(&no_oe, 0, &[], other2).is_empty(),
            "no match, no on_error"
        );
    }

    /// Nondet fires EVERY enabled transition as its own successor, so it reaches
    /// states the deterministic first-match strategy never explores.
    #[test]
    fn nondet_explores_every_enabled_successor() {
        let m = module(
            "module m { enum State { A, B, C } initial A event Go \
             transition A on Go { next B; priority 1; } \
             transition A on Go { next C; priority 0; } \
             transition B on Go { next B; } \
             transition C on Go { next C; } }",
        );
        let det = run(&m, &cfg()).expect("deterministic run");
        assert_eq!(det.status, ExactStatus::PassComplete);
        assert_eq!(det.states, 2, "deterministic reaches only A and B");
        let nd = run(&m, &nondet_cfg()).expect("nondet run");
        assert_eq!(nd.status, ExactStatus::PassComplete);
        assert_eq!(nd.states, 3, "nondet also reaches C via the second effect");
    }

    /// The D-P4-11 strong-fairness guard: a cycle through a config that has MORE
    /// THAN ONE explored successor is not a valid nondet bad cycle. Deterministic
    /// FAILs the liveness property (the A<->B cycle never emits Done); nondet
    /// PASSes because A branches to C (which emits Done), so A<->B is not strongly
    /// fair and must be excluded.
    #[test]
    fn nondet_liveness_requires_strongly_fair_cycles() {
        let m = module(
            "module m { enum State { A, B, C } initial A event Go \
             transition A on Go { next B; priority 1; } \
             transition A on Go { next C; priority 0; } \
             transition B on Go { next A; } \
             transition C on Go { next C; } \
             prop reaches_c : eventually (state == C) ; }",
        );
        let det = run(&m, &cfg()).expect("deterministic run");
        assert!(
            matches!(
                det.fail.as_ref(),
                Some((FailReason::LivenessViolation, label, _)) if label == "reaches_c"
            ),
            "deterministic: the A<->B cycle never reaches C (got {:?})",
            det.fail
        );
        let nd = run(&m, &nondet_cfg()).expect("nondet run");
        assert_eq!(
            nd.status,
            ExactStatus::PassComplete,
            "nondet: A branches to C, so the A<->B cycle is not strongly fair"
        );
    }
}
