//! Rust backend: bounded model-checker executable generation.
//!
//! The generated binary embeds the production FSM core (no unit tests) plus a
//! runtime that performs random bounded runs and checks every property label
//! against the recorded trace after each run. The per-step input-selection
//! strategy follows the phase order defined in `crate::codegen_mc`
//! (`McPhase::ForcedAssumptionViolation`, then `McPhase::UnexpectedInjection`,
//! then `McPhase::AllowedRandomInput`), and per-run outcomes use the
//! `codegen_mc::McStatus` taxonomy. The generated binary is self-contained: it
//! re-parses its own CLI flags (see `codegen_mc::McConfig`) and cannot depend
//! on the RFS toolchain crate.

use crate::codegen::{BackendOptions, CodegenOutput};
use crate::ir::Module;

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
    };
    let fsm = crate::backends::rust_fsm::generate_core(
        module,
        &fsm_options,
        false,
        crate::codegen::PropertyMonitorMode::Always,
    )?;
    let mut out = fsm.code;
    out.push_str(MC_RUNTIME);
    Ok(CodegenOutput {
        code: out,
        warnings: fsm.warnings,
    })
}

/// The model-checker runtime appended to the generated FSM core.
const MC_RUNTIME: &str = r#"
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

fn main() {
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
