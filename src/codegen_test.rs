//! Target-language-agnostic planning for FSM unit-test generation.
//!
//! Shared by all codegen backends (see `CODEGEN.md`). The planner decides
//! *what* to test; backends render the actual test code from the plan data
//! (the parity principle: one plan, rendered identically in Rust and C).
//!
//! The plan covers (Phase 5 task 4):
//!
//! - **module-level tests** (pre-existing): positive drive (`expect_progress`),
//!   negative unexpected-input probe (`definite_unexpected`);
//! - **per-transition unit tests**: one case per transition, driven from the
//!   transition's source state with a representative input matching its
//!   pattern and concrete variable assignments computed STATICALLY for the
//!   guard (taken case / not-taken case; documented fallbacks in `notes`);
//! - **invariant tests**: each in-subset SAFETY property (`always [N]` of a
//!   liveness-free per-sample atom) is re-checked at every step of the
//!   positive drive trace; liveness properties are excluded (the model
//!   checker's job);
//! - **property-monitor debug tests**: for `always [N] (p)` where `p` is a
//!   boolean atom over state/vars, an injected violating configuration +
//!   step that must fire the monitor; other shapes are skipped with a note;
//! - **random-trace test**: deterministic LCG input sequence (SAME constants
//!   and top-53-bit double mapping as the fuzz wire format) with a fixed
//!   seed, precomputed at plan time;
//! - **fuzz skeleton decode plan**: per event variant, the field decode
//!   strategy consumed by the libfuzzer_sys / libFuzzer renderers.
//!
//! The planner is static and language-agnostic: it emits data (cases,
//! expected outcomes, labels), never code. Where a case cannot be decided
//! statically, `TestPlan.notes` records the documented fallback.

use crate::codegen_fsm::FsmVariants;
use crate::ir::*;
use crate::monitor::{self, SignalAtom, SignalExpr};
use crate::validator::{self, Env, ExprType};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Plan data types
// ---------------------------------------------------------------------------

/// The plan for a generated unit-test suite.
#[derive(Debug, Clone)]
pub struct TestPlan {
    /// Maximum example-run length (K steps).
    pub steps: usize,
    /// Whether the positive run is expected to make at least one step
    /// (true when the module declares transitions).
    pub expect_progress: bool,
    /// Whether some input variant is statically not handled by any transition;
    /// when true, the negative test asserts that an unexpected-event failure
    /// (or, with `on_error`, a fallback landing) is found.
    pub definite_unexpected: bool,
    /// Whether the module declares an `on_error` fallback; when true, steps
    /// never fail with `UnexpectedEvent`, so the negative test asserts that
    /// the fallback state is reached instead.
    pub has_on_error: bool,
    /// Per-transition unit-test cases (transitions in declaration order; a
    /// guarded transition may contribute up to two cases).
    pub transitions: Vec<TransitionTestCase>,
    /// Invariant checks re-evaluated on the positive drive trace (safety
    /// properties only - see [`build_test_plan`]).
    pub invariants: Vec<InvariantCase>,
    /// Injected violating traces for property monitors.
    pub monitor_violations: Vec<MonitorViolationCase>,
    /// Deterministic random-trace configuration with the precomputed input
    /// sequence (same LCG as the fuzz wire format).
    pub random_trace: RandomTraceCase,
    /// Fuzz-skeleton decode plan (one entry per input variant).
    pub fuzz_decode: FuzzDecodePlan,
    /// Documented static-analysis fallbacks (skips and their reasons).
    pub notes: Vec<PlannerNote>,
}

/// A concrete value of a variable or field, in canonical form (backends map
/// it to their own literal syntax).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VarValue {
    Bool(bool),
    /// An unsigned integer within the declared bit width.
    Int(u64),
    /// An enum value as a full path `[EnumName, Variant]`.
    Enum(Vec<String>),
}

/// A concrete input matching one event variant (backends render it as a
/// single constructor call).
#[derive(Debug, Clone, PartialEq)]
pub struct InputCase {
    pub group: String,
    /// The variant this input is constructed from.
    pub variant: String,
    /// Concrete value for each field of the variant, in declaration order.
    pub fields: Vec<(String, VarValue)>,
}

/// A concrete emitted output (args evaluated against the case's assignment).
#[derive(Debug, Clone, PartialEq)]
pub struct OutputCase {
    pub group: String,
    /// The output variant name, or `None` for a bare `emit Group;`.
    pub variant: Option<String>,
    /// Arg values in declaration order; `None` where an arg expression could
    /// not be evaluated statically (a note is emitted; the renderer asserts
    /// only what is known).
    pub args: Vec<Option<VarValue>>,
}

/// The expected outcome of driving one per-transition test case.
#[derive(Debug, Clone, PartialEq)]
pub enum TransitionExpectation {
    /// The transition fires under this case: assert next state + outputs.
    Taken {
        next_state: String,
        outputs: Vec<OutputCase>,
    },
    /// This transition must NOT fire under the installed configuration.
    NotTaken {
        /// No other transition in the same state can match this input: the
        /// step fails with `UnexpectedEvent`, or takes `on_error` when present.
        only_candidate: bool,
        /// The on_error fallback expectation (when `only_candidate` and the
        /// module declares one), evaluated under the case's assignment.
        on_error: Option<OnCase>,
        /// This transition's own signature (next state + its outputs): when
        /// other transitions may match, the observed outcome must differ from
        /// it (the not-taken decision is only planned when that is statically
        /// guaranteed to be observable).
        signature: (String, Vec<OutputCase>),
    },
}

/// The expected landing of an `on_error` fallback.
#[derive(Debug, Clone, PartialEq)]
pub struct OnCase {
    pub next_state: String,
    pub outputs: Vec<OutputCase>,
}

/// One per-transition unit-test case.
#[derive(Debug, Clone, PartialEq)]
pub struct TransitionTestCase {
    /// Index into `Module.transitions` (declaration order).
    pub transition_id: usize,
    /// The state the test installs before stepping.
    pub from_state: String,
    /// The representative input matching the transition's pattern.
    pub input: InputCase,
    /// The COMPLETE concrete variable configuration to install (every module
    /// variable; searched values for guard-relevant ones, representatives
    /// for the rest).
    pub vars: Vec<(String, VarValue)>,
    pub expectation: TransitionExpectation,
}

/// An invariant re-checked at every sample of the positive drive trace.
#[derive(Debug, Clone, PartialEq)]
pub struct InvariantCase {
    /// The property label (for failure messages).
    pub label: String,
    /// `None` for `always (p)`; `Some(N)` for `always[N] (p)` - check samples
    /// 0..N-1 only.
    pub bound: Option<u64>,
    /// The per-sample atom to re-check (state/vars expression).
    pub signal: SignalExpr,
}

/// An injected violating trace for a property monitor.
#[derive(Debug, Clone, PartialEq)]
pub struct MonitorViolationCase {
    /// The property label the monitor must record.
    pub label: String,
    /// `None` for `always (p)`; `Some(N)` for `always[N] (p)`.
    pub bound: Option<u64>,
    /// The atom that evaluates FALSE in the post-step (or initial)
    /// configuration - what the monitor sees.
    pub atom: SignalExpr,
    /// Install this pre-step configuration before driving.
    pub state: String,
    pub vars: Vec<(String, VarValue)>,
    /// The input to step with (fires a transition whose post-step
    /// configuration violates the atom). `None` when the INITIAL
    /// configuration already violates - the monitor records it at sample 0
    /// of `Fsm::new()`, so no step is needed.
    pub step_input: Option<InputCase>,
    /// Post-step state where the atom evaluates false.
    pub post_state: String,
    /// Post-step variable configuration (all module variables).
    pub post_vars: Vec<(String, VarValue)>,
}

/// Deterministic random-trace test configuration.
#[derive(Debug, Clone, PartialEq)]
pub struct RandomTraceCase {
    /// The fixed seed (see [`RANDOM_TRACE_SEED`]).
    pub seed: u64,
    /// Number of drive steps (K = `--steps`).
    pub steps: usize,
    /// The precomputed input sequence: indices into the module's flattened
    /// input variants (declaration order - the same table the generated
    /// `ALL_INPUTS` uses). Empty when the module has no inputs.
    pub input_indices: Vec<usize>,
}

/// Decode plan for one fuzz skeleton (consumed by the tasks 5/6 renderers).
#[derive(Debug, Clone, PartialEq)]
pub struct FuzzDecodePlan {
    /// One entry per input variant, declaration order. The renderer selects
    /// the variant as `first_byte % variants.len()` and decodes the fields
    /// that follow.
    pub variants: Vec<FuzzVariantDecode>,
}

/// Per-variant field decode strategies.
#[derive(Debug, Clone, PartialEq)]
pub struct FuzzVariantDecode {
    /// The flattened variant name (pre-mangling; backends apply their own
    /// identifier rules).
    pub unique_name: String,
    /// One strategy per field, declaration order.
    pub fields: Vec<(String, FuzzFieldStrategy)>,
}

/// How one fuzzed field is decoded from the fuzzer byte stream (v1
/// placeholder strategies; renderers may refine within these semantics).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FuzzFieldStrategy {
    /// Decode one byte; the value is `byte != 0`.
    ByteNonZero,
    /// Decode `ceil(width / 8)` bytes little-endian, masked to `width` bits
    /// (payload decode).
    PayloadBytes { width: u32 },
    /// Constant representative value (first declared enum variant).
    Representative,
}

/// A documented static-analysis fallback recorded by the planner.
#[derive(Debug, Clone, PartialEq)]
pub struct PlannerNote {
    /// What the note is about ("transition #3", "property p").
    pub subject: String,
    /// The fallback explanation (renderers may surface it as a test comment).
    pub message: String,
}

/// The fixed seed for the deterministic random-trace test.
pub const RANDOM_TRACE_SEED: u64 = 12345;

// ---------------------------------------------------------------------------
// Planner entry point
// ---------------------------------------------------------------------------

/// Build a test plan for a module given its flattened variants.
pub fn build_test_plan(variants: &FsmVariants, module: &Module, steps: usize) -> TestPlan {
    let mut notes: Vec<PlannerNote> = Vec::new();
    TestPlan {
        steps,
        expect_progress: !module.transitions.is_empty(),
        definite_unexpected: has_definite_unexpected(variants, module),
        has_on_error: module.on_error.is_some(),
        transitions: plan_transitions(variants, module, &mut notes),
        invariants: select_invariants(module),
        monitor_violations: plan_monitor_violations(variants, module, &mut notes),
        random_trace: build_random_trace(variants, module, steps),
        fuzz_decode: build_fuzz_decode(variants),
        notes,
    }
}

/// True when at least one input variant is never matched by any transition
/// pattern. Such a variant's representative value is guaranteed to produce an
/// `UnexpectedEvent` error on some reachable state (in particular, the test
/// driver will find it while exploring).
pub fn has_definite_unexpected(variants: &FsmVariants, module: &Module) -> bool {
    if variants.inputs.is_empty() {
        return false;
    }
    let mut used = std::collections::HashSet::new();
    for t in &module.transitions {
        match &t.pattern.variant {
            None => {
                for v in variants
                    .inputs
                    .iter()
                    .filter(|v| v.group == t.pattern.group)
                {
                    used.insert(v.unique_name.clone());
                }
            }
            Some(crate::ir::PatternVariant::Unit { name })
            | Some(crate::ir::PatternVariant::Struct { name, .. }) => {
                if let Some(v) = variants.input_variant(&t.pattern.group, name) {
                    used.insert(v.unique_name.clone());
                }
            }
        }
    }
    variants
        .inputs
        .iter()
        .any(|v| !used.contains(&v.unique_name))
}

/// True when the module has inputs but no transitions at all: every input is
/// unexpected from the initial state (a deadlock-style negative case).
pub fn is_deadlocked_initial(module: &Module, variants: &FsmVariants) -> bool {
    !variants.inputs.is_empty() && module.transitions.is_empty()
}

// ---------------------------------------------------------------------------
// Per-transition cases
// ---------------------------------------------------------------------------

/// Cap on the number of concrete assignments evaluated per static search
/// (guard cases, monitor-violation drives). The enumeration order is
/// deterministic, so the cap only trims the tail of the search space.
const MAX_STATIC_ASSIGNMENTS: usize = 4096;

/// A concrete input plus the pattern-binding types it creates (needed to
/// build a validator `Env` for guard evaluation).
struct ConcreteInput {
    case: InputCase,
    /// `(binding name, declared field type)` in pattern-field order.
    bindings: Vec<(String, Type)>,
}

/// Plan one (or two) test case(s) per transition, in declaration order.
fn plan_transitions(
    variants: &FsmVariants,
    module: &Module,
    notes: &mut Vec<PlannerNote>,
) -> Vec<TransitionTestCase> {
    let mut cases = Vec::new();
    for (idx, t) in module.transitions.iter().enumerate() {
        let Some(input) = concrete_input_for_pattern(&t.pattern, variants, module) else {
            notes.push(PlannerNote {
                subject: format!("transition #{idx}"),
                message: "pattern does not name a concrete input variant; no case planned".into(),
            });
            continue;
        };

        // Siblings: other transitions in the same state whose pattern can
        // match THIS concrete input (same group; no variant restriction or
        // the same variant name).
        let mut siblings: Vec<usize> = Vec::new();
        for (j, s) in module.transitions.iter().enumerate() {
            if j == idx || s.state != t.state {
                continue;
            }
            if pattern_can_match_variant(&s.pattern, &input.case.group, &input.case.variant) {
                siblings.push(j);
            }
        }

        // Firing order within the state: descending effective priority, then
        // source order (stable sort - mirrors the generated fsm_step).
        let mut firing: Vec<usize> = siblings.clone();
        firing.push(idx);
        firing.sort_by_key(|&k| std::cmp::Reverse(module.transitions[k].priority.unwrap_or(0)));
        let earlier: Vec<usize> = firing.iter().copied().take_while(|&k| k != idx).collect();

        // A sibling that fires before t and is UNGUARDED shadows t for this
        // input: t can never fire, so no taken case is possible.
        let shadow_by = earlier
            .iter()
            .find(|&&s| module.transitions[s].guard.is_none());

        // Variables whose candidate values matter: every variable referenced
        // by t's guard or any sibling's guard (module declaration order).
        let mut guard_vars: Vec<String> = Vec::new();
        for v in &module.vars {
            let refs = std::iter::once(t.guard.as_ref())
                .chain(
                    siblings
                        .iter()
                        .map(|&s| module.transitions[s].guard.as_ref()),
                )
                .any(|g| g.is_some_and(|e| expr_refs_var(e, &v.name)));
            if refs {
                guard_vars.push(v.name.clone());
            }
        }

        let binding_vals: HashMap<String, VarValue> = input
            .case
            .fields
            .iter()
            .map(|(f, v)| (f.clone(), v.clone()))
            .collect();

        let mut taken_vars: Option<Vec<(String, VarValue)>> = None;
        let mut not_taken_vars: Option<Vec<(String, VarValue)>> = None;

        if shadow_by.is_none() {
            for assignment in enumerate_assignments(module, &guard_vars) {
                let env = eval_env(
                    module,
                    &t.state,
                    &assignment,
                    &input.bindings,
                    &binding_vals,
                );
                let t_guard_true = match &t.guard {
                    Some(g) => eval_bool(g, &env),
                    None => Some(true),
                };
                // Earlier siblings must be disabled for t to fire.
                let earlier_disabled = earlier.iter().all(|&s| {
                    let g = module.transitions[s]
                        .guard
                        .as_ref()
                        .expect("unguarded earlier sibling checked as shadow");
                    eval_bool(g, &env) == Some(false)
                });

                if taken_vars.is_none() && t_guard_true == Some(true) && earlier_disabled {
                    taken_vars = Some(complete_configuration(module, assignment.clone()));
                }
                // Not-taken: t's guard false (t must be guarded) AND the
                // decision is statically observable.
                if not_taken_vars.is_none()
                    && t_guard_true == Some(false)
                    && not_taken_observable(module, &siblings, idx, &env)
                {
                    not_taken_vars = Some(complete_configuration(module, assignment));
                }
                if taken_vars.is_some() && not_taken_vars.is_some() {
                    break;
                }
            }
        }

        let subject = format!("transition #{idx}");
        if let Some(&s) = shadow_by {
            notes.push(PlannerNote {
                subject: subject.clone(),
                message: format!(
                    "shadowed by unguarded earlier-firing transition #{s}; it always takes this input first"
                ),
            });
        }

        if let Some(vars) = taken_vars {
            cases.push(TransitionTestCase {
                transition_id: idx,
                from_state: t.state.clone(),
                input: input.case.clone(),
                vars: vars.clone(),
                expectation: TransitionExpectation::Taken {
                    next_state: t.next_state.clone(),
                    outputs: eval_outputs(
                        &t.emits,
                        &t.state,
                        &vars,
                        &input.bindings,
                        &binding_vals,
                        module,
                    ),
                },
            });
        } else if t.guard.is_some() && shadow_by.is_none() {
            notes.push(PlannerNote {
                subject: subject.clone(),
                message: "guard not statically satisfiable by representative values (with higher-priority siblings disabled); only the not-taken case is planned"
                    .into(),
            });
        }

        if let Some(vars) = not_taken_vars {
            let on_error = if siblings.is_empty() {
                module.on_error.as_ref().map(|oe| OnCase {
                    next_state: oe.next_state.clone(),
                    outputs: eval_outputs(
                        &oe.emits,
                        &t.state,
                        &vars,
                        &input.bindings,
                        &binding_vals,
                        module,
                    ),
                })
            } else {
                None
            };
            let signature = (
                t.next_state.clone(),
                eval_outputs(
                    &t.emits,
                    &t.state,
                    &vars,
                    &input.bindings,
                    &binding_vals,
                    module,
                ),
            );
            cases.push(TransitionTestCase {
                transition_id: idx,
                from_state: t.state.clone(),
                input: input.case.clone(),
                vars,
                expectation: TransitionExpectation::NotTaken {
                    only_candidate: siblings.is_empty(),
                    on_error,
                    signature,
                },
            });
        } else if t.guard.is_some() && shadow_by.is_none() {
            notes.push(PlannerNote {
                subject,
                message: "not-taken case not statically observable (a matching sibling can fire with an identical outcome signature)".into(),
            });
        }
    }
    cases
}

/// True when any transition firing BEFORE `t_idx` (descending effective
/// priority, then source order - mirroring the generated `fsm_step`) can match
/// the concrete input and fires under `env` (unguarded = always fires). The
/// monitor-violation search must skip such candidates: the planned drive would
/// take the sibling, not `t`.
fn earlier_sibling_fires(
    module: &Module,
    t_idx: usize,
    input_case: &InputCase,
    env: &EvalEnv,
) -> bool {
    let t = &module.transitions[t_idx];
    let mut firing: Vec<usize> = (0..module.transitions.len())
        .filter(|&j| j != t_idx && module.transitions[j].state == t.state)
        .filter(|&j| {
            pattern_can_match_variant(
                &module.transitions[j].pattern,
                &input_case.group,
                &input_case.variant,
            )
        })
        .collect();
    firing.push(t_idx);
    firing.sort_by_key(|&k| std::cmp::Reverse(module.transitions[k].priority.unwrap_or(0)));
    let earlier = firing
        .iter()
        .copied()
        .take_while(|&k| k != t_idx)
        .collect::<Vec<_>>();
    earlier.iter().any(|&s| match &module.transitions[s].guard {
        Some(g) => eval_bool(g, env) == Some(true),
        None => true,
    })
}

/// True when `pattern` can match a concrete input of variant `variant` in
/// group `group`: same group, and either no variant restriction or exactly
/// that variant.
fn pattern_can_match_variant(pattern: &EventPattern, group: &str, variant: &str) -> bool {
    pattern.group == group
        && match &pattern.variant {
            None => true,
            Some(PatternVariant::Unit { name }) | Some(PatternVariant::Struct { name, .. }) => {
                name == variant
            }
        }
}

/// The representative concrete input matching a transition pattern: the named
/// variant (or the group's first variant for whole-group patterns), with
/// literal pattern fields kept and wildcard/bound fields at their
/// representative values.
fn concrete_input_for_pattern(
    pattern: &EventPattern,
    variants: &FsmVariants,
    module: &Module,
) -> Option<ConcreteInput> {
    let (vname, pfields) = match &pattern.variant {
        None => {
            let v = variants.inputs.iter().find(|v| v.group == pattern.group)?;
            (v.variant.clone(), None::<&[PatternField]>)
        }
        Some(PatternVariant::Unit { name }) => (name.clone(), None),
        Some(PatternVariant::Struct { name, fields }) => (name.clone(), Some(fields.as_slice())),
    };
    let group_decl = module.events.iter().find(|g| g.name == pattern.group)?;
    let decl = group_decl.variants.iter().find(|v| match v {
        ConstructorVariant::Unit { name } => *name == vname,
        ConstructorVariant::Struct { name, .. } => *name == vname,
    })?;
    let decl_fields: Vec<FieldDecl> = match decl {
        ConstructorVariant::Unit { .. } => Vec::new(),
        ConstructorVariant::Struct { fields, .. } => fields.clone(),
    };
    let mut case_fields = Vec::new();
    let mut bindings = Vec::new();
    for (i, df) in decl_fields.iter().enumerate() {
        let pf = pfields.and_then(|pf| pf.get(i));
        let val = match pf {
            Some(PatternField {
                pattern: PatternExpr::Int { value },
                ..
            }) => VarValue::Int(*value),
            Some(PatternField {
                pattern: PatternExpr::Bool { value },
                ..
            }) => VarValue::Bool(*value),
            Some(PatternField {
                pattern: PatternExpr::EnumVariant { path },
                ..
            }) => VarValue::Enum(path.clone()),
            // Wildcard / Bind / whole-group: representative value.
            _ => representative_var_value(&df.ty, module)?,
        };
        case_fields.push((df.name.clone(), val));
        if let Some(PatternField { name: Some(b), .. }) = pf {
            bindings.push((b.clone(), df.ty.clone()));
        }
    }
    Some(ConcreteInput {
        case: InputCase {
            group: pattern.group.clone(),
            variant: vname,
            fields: case_fields,
        },
        bindings,
    })
}

/// The not-taken decision is observable when no matching sibling can fire
/// with an outcome identical to t's signature under `env` (and, when nothing
/// fires at all, the on_error landing differs from it too).
fn not_taken_observable(module: &Module, siblings: &[usize], idx: usize, env: &EvalEnv) -> bool {
    let t = &module.transitions[idx];
    for &s in siblings {
        let s_t = &module.transitions[s];
        let fires = match &s_t.guard {
            Some(g) => eval_bool(g, env) == Some(true),
            None => true,
        };
        if !fires {
            continue;
        }
        if s_t.next_state != t.next_state || s_t.emits != t.emits {
            continue; // fires with a different signature - the difference is observable
        }
        return false; // fires identically - indistinguishable from t firing
    }
    if let Some(oe) = &module.on_error {
        if oe.next_state == t.next_state && oe.emits == t.emits {
            return false; // the fallback lands exactly where t would
        }
    }
    true
}

// ---------------------------------------------------------------------------
// Invariant selection
// ---------------------------------------------------------------------------

/// Select the in-subset SAFETY properties for invariant re-checking:
/// `always (p)` / `always[N] (p)` where `p` is a liveness-free per-sample
/// signal. Liveness-bearing properties (`bounded_eventually`, `eventually`,
/// `until`) are excluded - they belong to the model checker.
fn select_invariants(module: &Module) -> Vec<InvariantCase> {
    let mut out = Vec::new();
    for p in &module.properties {
        let (bound, inner) = match &p.formula {
            Formula::Always { inner } => (None, inner),
            Formula::BoundedAlways { bound, inner } => (Some(*bound), inner),
            _ => continue,
        };
        if has_liveness_op(inner) {
            continue;
        }
        match monitor::formula_to_signal(inner) {
            Some(signal) => out.push(InvariantCase {
                label: p.name.clone(),
                bound,
                signal,
            }),
            None => continue, // temporal operators inside (e.g. `next`) - not a per-sample invariant
        }
    }
    out
}

/// True when the formula contains a liveness operator anywhere
/// (`bounded_eventually`, `eventually`, `until`). Mirrors the Phase 4 MC
/// classification (`mc_plan::contains_liveness`), which is the superset of
/// `codegen_fsm::contains_liveness`.
fn has_liveness_op(f: &Formula) -> bool {
    match f {
        Formula::BoundedEventually { .. } | Formula::Eventually { .. } | Formula::Until { .. } => {
            true
        }
        Formula::Not { inner }
        | Formula::Always { inner }
        | Formula::Next { inner }
        | Formula::BoundedAlways { inner, .. } => has_liveness_op(inner),
        Formula::And { left, right }
        | Formula::Or { left, right }
        | Formula::Implies { left, right } => has_liveness_op(left) || has_liveness_op(right),
        _ => false,
    }
}

// ---------------------------------------------------------------------------
// Monitor-violating traces
// ---------------------------------------------------------------------------

/// Plan injected violating traces for properties of the shape
/// `always [N] (p)` where `p` is a boolean atom over state/vars (the monitor
/// compiles to a stateless range check, so directly installing a violating
/// configuration is sound). Other shapes are skipped with a note.
fn plan_monitor_violations(
    variants: &FsmVariants,
    module: &Module,
    notes: &mut Vec<PlannerNote>,
) -> Vec<MonitorViolationCase> {
    let mut out = Vec::new();
    for p in &module.properties {
        let (bound, inner) = match &p.formula {
            Formula::Always { inner } => (None, inner),
            Formula::BoundedAlways { bound, inner } => (Some(*bound), inner),
            _ => {
                notes.push(PlannerNote {
                    subject: format!("property {}", p.name),
                    message: "not of the form always [N] (atom); no monitor-violating trace planned (covered by the MC suites)".into(),
                });
                continue;
            }
        };
        let Some(signal) = monitor::formula_to_signal(inner) else {
            notes.push(PlannerNote {
                subject: format!("property {}", p.name),
                message: "body contains temporal operators; no monitor-violating trace planned"
                    .into(),
            });
            continue;
        };
        let SignalExpr::Atom(SignalAtom::VarExpr(atom)) = signal else {
            notes.push(PlannerNote {
                subject: format!("property {}", p.name),
                message: "atom is not a state/vars expression (input_is/output_has kinds are not planned in v1); no monitor-violating trace planned".into(),
            });
            continue;
        };

        // Case 1: the initial configuration already violates.
        let init_vars = init_var_values(module);
        let init_env = eval_env(
            module,
            module.initial_state.as_deref().unwrap_or(""),
            &init_vars,
            &[],
            &HashMap::new(),
        );
        if eval_expr(&atom, &init_env) == Some(VarValue::Bool(false)) {
            out.push(MonitorViolationCase {
                label: p.name.clone(),
                bound,
                atom: SignalExpr::Atom(SignalAtom::VarExpr(atom.clone())),
                state: module.initial_state.clone().unwrap_or_default(),
                vars: init_vars.clone(),
                step_input: None,
                post_state: module.initial_state.clone().unwrap_or_default(),
                post_vars: init_vars,
            });
            continue;
        }

        // Case 2: search (state, assignment) x outgoing transition for a
        // one-step drive whose POST-STEP configuration violates the atom.
        let state_variants = module
            .enums
            .iter()
            .find(|e| e.name == "State")
            .map(|e| e.variants.clone())
            .unwrap_or_default();
        let mut found = None;
        'states: for st in &state_variants {
            // Atom-referenced variables get candidate values; the rest stay
            // at their representatives.
            let atom_vars: Vec<String> = module
                .vars
                .iter()
                .filter(|v| expr_refs_var(&atom, &v.name))
                .map(|v| v.name.clone())
                .collect();
            for assignment in enumerate_assignments(module, &atom_vars) {
                for (t_idx, t) in module
                    .transitions
                    .iter()
                    .enumerate()
                    .filter(|(_, t)| t.state == *st)
                {
                    let Some(input) = concrete_input_for_pattern(&t.pattern, variants, module)
                    else {
                        continue;
                    };
                    let step_binding_vals: HashMap<String, VarValue> = input
                        .case
                        .fields
                        .iter()
                        .map(|(f, v)| (f.clone(), v.clone()))
                        .collect();
                    let env_step =
                        eval_env(module, st, &assignment, &input.bindings, &step_binding_vals);
                    // The planned drive must actually take THIS transition: a
                    // higher-firing sibling (priority desc, source order -
                    // mirroring the generated fsm_step) that matches the input
                    // and whose guard holds would fire first, so t's post-step
                    // configuration is not the observed one.
                    if earlier_sibling_fires(module, t_idx, &input.case, &env_step) {
                        continue;
                    }
                    let guard_ok = match &t.guard {
                        Some(g) => eval_bool(g, &env_step) == Some(true),
                        None => true,
                    };
                    if !guard_ok {
                        continue;
                    }
                    let post_vars = apply_assignments(module, t, &assignment);
                    let env_post =
                        eval_env(module, &t.next_state, &post_vars, &[], &HashMap::new());
                    if eval_expr(&atom, &env_post) == Some(VarValue::Bool(false)) {
                        found = Some(MonitorViolationCase {
                            label: p.name.clone(),
                            bound,
                            atom: SignalExpr::Atom(SignalAtom::VarExpr(atom.clone())),
                            state: st.clone(),
                            vars: complete_configuration(module, assignment),
                            step_input: Some(input.case),
                            post_state: t.next_state.clone(),
                            post_vars,
                        });
                        break 'states;
                    }
                }
            }
        }
        match found {
            Some(c) => out.push(c),
            None => notes.push(PlannerNote {
                subject: format!("property {}", p.name),
                message: "no statically found violating configuration (search capped); no monitor-violating trace planned".into(),
            }),
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Random trace (deterministic LCG, same constants as the fuzz wire format)
// ---------------------------------------------------------------------------

/// The shared LCG: `state = state * 6364136223846793005 + 1442695040888963407`
/// (wrapping), with the top-53-bit double mapping used by the generated fuzz
/// runtimes (`Rng::next_f64` in both backends).
#[derive(Debug, Clone)]
pub struct Lcg(u64);

impl Lcg {
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }
    /// Same top-53-bit double mapping as the fuzz wire format.
    pub fn next_f64(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.0 >> 11) as f64) / ((u64::MAX >> 11) as f64)
    }
    /// Same index selection as the fuzz wire format (`Rng::pick_index`).
    pub fn pick_index(&mut self, len: usize) -> Option<usize> {
        if len == 0 {
            None
        } else {
            Some((self.next_f64() * len as f64) as usize % len)
        }
    }
}

/// Build the deterministic random-trace case: fixed seed + K steps, with the
/// input sequence precomputed at plan time (byte-pinned by unit tests).
fn build_random_trace(variants: &FsmVariants, _module: &Module, steps: usize) -> RandomTraceCase {
    let mut rng = Lcg::new(RANDOM_TRACE_SEED);
    let input_indices = (0..steps)
        .filter_map(|_| rng.pick_index(variants.inputs.len()))
        .collect();
    RandomTraceCase {
        seed: RANDOM_TRACE_SEED,
        steps,
        input_indices,
    }
}

// ---------------------------------------------------------------------------
// Fuzz decode plan
// ---------------------------------------------------------------------------

/// Build the per-variant field decode strategies (v1 placeholders consumed by
/// the libfuzzer_sys / libFuzzer renderers): bool -> one nonzero byte;
/// bit<N> -> ceil(N/8) little-endian bytes masked to N bits; enum -> constant
/// representative value.
pub(crate) fn build_fuzz_decode(variants: &FsmVariants) -> FuzzDecodePlan {
    FuzzDecodePlan {
        variants: variants
            .inputs
            .iter()
            .map(|v| FuzzVariantDecode {
                unique_name: v.unique_name.clone(),
                fields: v
                    .fields
                    .iter()
                    .map(|(name, ty)| match ty {
                        Type::Bool => (name.clone(), FuzzFieldStrategy::ByteNonZero),
                        Type::Bit { width } => (
                            name.clone(),
                            FuzzFieldStrategy::PayloadBytes { width: *width },
                        ),
                        Type::Enum { .. } => (name.clone(), FuzzFieldStrategy::Representative),
                    })
                    .collect(),
            })
            .collect(),
    }
}

// ---------------------------------------------------------------------------
// Static expression evaluation (mirrors the backend lowering exactly)
// ---------------------------------------------------------------------------

/// Concrete values of all module variables at the initial configuration.
fn init_var_values(module: &Module) -> Vec<(String, VarValue)> {
    let mut env_vals: HashMap<String, VarValue> = HashMap::new();
    // Initializers may reference earlier variables (declaration order).
    for v in &module.vars {
        let env = EvalEnv {
            module,
            state: String::new(),
            vals: env_vals.clone(),
            venv: Env::for_module(module),
        };
        let val = eval_expr(&v.init, &env)
            .or_else(|| representative_var_value(&v.ty, module))
            .unwrap_or(VarValue::Bool(false));
        env_vals.insert(v.name.clone(), val);
    }
    module
        .vars
        .iter()
        .map(|v| (v.name.clone(), env_vals[&v.name].clone()))
        .collect()
}

/// A concrete evaluation environment: module + state name + variable/binding
/// values + a validator `Env` with matching bindings (for width typing).
#[derive(Clone)]
struct EvalEnv<'a> {
    module: &'a Module,
    state: String,
    vals: HashMap<String, VarValue>,
    venv: Env,
}

impl std::fmt::Debug for EvalEnv<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EvalEnv")
            .field("state", &self.state)
            .field("vals", &self.vals)
            .finish()
    }
}

/// Build an evaluation environment for a configuration: `assignment` values
/// for the listed variables (representatives for the rest), pattern bindings
/// with their declared types, and the concrete binding values.
fn eval_env<'a>(
    module: &'a Module,
    state: &str,
    assignment: &[(String, VarValue)],
    bindings: &[(String, Type)],
    binding_vals: &HashMap<String, VarValue>,
) -> EvalEnv<'a> {
    let mut vals = HashMap::new();
    for v in &module.vars {
        vals.insert(
            v.name.clone(),
            representative_var_value(&v.ty, module).unwrap_or(VarValue::Bool(false)),
        );
    }
    for (n, v) in assignment {
        vals.insert(n.clone(), v.clone());
    }
    let mut venv = Env::for_module(module);
    for (n, ty) in bindings {
        if !venv.bindings.contains_key(n) {
            venv = venv.with_binding(n.clone(), validator::type_to_expr_type(ty));
        }
        if let Some(v) = binding_vals.get(n) {
            vals.insert(n.clone(), v.clone());
        }
    }
    EvalEnv {
        module,
        state: state.to_string(),
        vals,
        venv,
    }
}

/// The representative concrete value of a type (bool -> false, bit<N> -> 0,
/// enum -> first declared variant).
fn representative_var_value(ty: &Type, module: &Module) -> Option<VarValue> {
    match ty {
        Type::Bool => Some(VarValue::Bool(false)),
        Type::Bit { .. } => Some(VarValue::Int(0)),
        Type::Enum { name } => module
            .enums
            .iter()
            .find(|e| e.name == *name)
            .and_then(|e| e.variants.first())
            .map(|v| VarValue::Enum(vec![name.clone(), v.clone()])),
    }
}

/// Extend an assignment to a COMPLETE configuration: every module variable in
/// declaration order, keeping the assigned values and filling the rest with
/// their representative values - EXACTLY what `eval_env` assumes for
/// unassigned variables (the installed runtime configuration must match the
/// static analysis).
fn complete_configuration(
    module: &Module,
    assignment: Vec<(String, VarValue)>,
) -> Vec<(String, VarValue)> {
    module
        .vars
        .iter()
        .map(|v| {
            let val = assignment
                .iter()
                .find(|(n, _)| n == &v.name)
                .map(|(_, v)| v.clone())
                .or_else(|| representative_var_value(&v.ty, module))
                .unwrap_or(VarValue::Bool(false));
            (v.name.clone(), val)
        })
        .collect()
}

/// Candidate concrete values for one variable type, in deterministic order:
/// bool -> [false, true]; bit<N> -> [0, 1, 2^N-1] (deduplicated); enum -> all
/// declared variants.
fn candidate_values(ty: &Type, module: &Module) -> Vec<VarValue> {
    match ty {
        Type::Bool => vec![VarValue::Bool(false), VarValue::Bool(true)],
        Type::Bit { width } => {
            let max = if *width >= 64 {
                u64::MAX
            } else {
                (1u64 << width) - 1
            };
            let mut v = vec![VarValue::Int(0)];
            if max != 0 {
                v.push(VarValue::Int(1));
            }
            if max != 0 && max != 1 {
                v.push(VarValue::Int(max));
            }
            v
        }
        Type::Enum { name } => module
            .enums
            .iter()
            .find(|e| e.name == *name)
            .map(|e| {
                e.variants
                    .iter()
                    .map(|v| VarValue::Enum(vec![name.clone(), v.clone()]))
                    .collect()
            })
            .unwrap_or_default(),
    }
}

/// Enumerate concrete assignments over `var_names` (module declaration order)
/// as the lexicographic product of their candidate values - first variable is
/// the slowest counter - capped at [`MAX_STATIC_ASSIGNMENTS`] total.
fn enumerate_assignments<'a>(
    module: &'a Module,
    var_names: &'a [String],
) -> impl Iterator<Item = Vec<(String, VarValue)>> + 'a {
    let lists: Vec<Vec<VarValue>> = var_names
        .iter()
        .map(|n| {
            let ty = module
                .vars
                .iter()
                .find(|v| &v.name == n)
                .map(|v| v.ty.clone())
                .unwrap_or(Type::Bool);
            candidate_values(&ty, module)
        })
        .collect();
    let total: usize = lists.iter().map(|l| l.len()).product();
    let cap = total.min(MAX_STATIC_ASSIGNMENTS);
    (0..cap).map(move |i| {
        // Mixed-radix decode with the LAST variable as the fastest counter.
        let mut rem = i;
        let mut digits = vec![0usize; var_names.len()];
        for k in (0..var_names.len()).rev() {
            digits[k] = rem % lists[k].len();
            rem /= lists[k].len();
        }
        var_names
            .iter()
            .enumerate()
            .zip(digits.iter())
            .map(|((k, n), &d)| (n.clone(), lists[k][d].clone()))
            .collect()
    })
}

/// True when the expression references module variable `name` (a single-segment
/// path that is a declared variable).
fn expr_refs_var(e: &Expr, name: &str) -> bool {
    match e {
        Expr::Path { path } => path.len() == 1 && path[0] == name,
        Expr::Unary { expr, .. } => expr_refs_var(expr, name),
        Expr::Binary { left, right, .. } => expr_refs_var(left, name) || expr_refs_var(right, name),
        _ => false,
    }
}

/// Evaluate an expression to a concrete value. Mirrors the backend lowering
/// EXACTLY (u128 wrapping arithmetic masked to the expression's type width;
/// shifts saturate to 0 when the amount reaches the width; bool bitwise ops
/// are logical). Returns `None` where no concrete interpretation exists.
/// Both operands as booleans (None when either is not a bool).
fn bools(l: &VarValue, r: &VarValue) -> Option<(bool, bool)> {
    match (l, r) {
        (VarValue::Bool(a), VarValue::Bool(b)) => Some((*a, *b)),
        _ => None,
    }
}

/// Both operands as integers (None when either is not an int).
fn ints(l: &VarValue, r: &VarValue) -> Option<(u64, u64)> {
    match (l, r) {
        (VarValue::Int(a), VarValue::Int(b)) => Some((*a, *b)),
        _ => None,
    }
}

/// Structural equality of two concrete values.
fn values_equal(l: &VarValue, r: &VarValue) -> bool {
    match (l, r) {
        (VarValue::Bool(a), VarValue::Bool(b)) => a == b,
        (VarValue::Int(a), VarValue::Int(b)) => a == b,
        (VarValue::Enum(a), VarValue::Enum(b)) => a == b,
        _ => false,
    }
}

fn eval_expr(e: &Expr, env: &EvalEnv) -> Option<VarValue> {
    use BinOp::*;
    match e {
        Expr::Int { value } => Some(VarValue::Int(*value)),
        Expr::Bool { value } => Some(VarValue::Bool(*value)),
        Expr::Path { path } => {
            if path.len() == 1 {
                let name = &path[0];
                if name == "state" {
                    return Some(VarValue::Enum(vec!["State".into(), env.state.clone()]));
                }
                if let Some(v) = env.vals.get(name) {
                    return Some(v.clone());
                }
                if is_state_variant(env.module, name) {
                    return Some(VarValue::Enum(vec!["State".into(), name.clone()]));
                }
                None
            } else {
                Some(VarValue::Enum(path.clone()))
            }
        }
        Expr::Unary { op, expr } => {
            let inner = eval_expr(expr, env)?;
            match op {
                crate::ir::UnaryOp::Not => match inner {
                    VarValue::Bool(b) => Some(VarValue::Bool(!b)),
                    _ => None,
                },
                crate::ir::UnaryOp::BitNot => match inner {
                    VarValue::Int(x) => {
                        let w = width_of(expr, env)?;
                        Some(VarValue::Int(((!(x as u128)) & mask(w)) as u64))
                    }
                    _ => None,
                },
                crate::ir::UnaryOp::Neg => match inner {
                    VarValue::Int(x) => {
                        let w = width_of(expr, env)?;
                        Some(VarValue::Int(
                            (0u128.wrapping_sub(x as u128) & mask(w)) as u64,
                        ))
                    }
                    _ => None,
                },
            }
        }
        Expr::Binary { op, left, right } => {
            let l = eval_expr(left, env)?;
            let r = eval_expr(right, env)?;
            match op {
                LogicAnd => bools(&l, &r).map(|(a, b)| VarValue::Bool(a && b)),
                LogicOr => bools(&l, &r).map(|(a, b)| VarValue::Bool(a || b)),
                Eq => Some(VarValue::Bool(values_equal(&l, &r))),
                NotEq => Some(VarValue::Bool(!values_equal(&l, &r))),
                Lt | Le | Gt | Ge => ints(&l, &r).map(|(a, b)| {
                    let c = match op {
                        Lt => a < b,
                        Le => a <= b,
                        Gt => a > b,
                        _ => a >= b,
                    };
                    VarValue::Bool(c)
                }),
                Plus | Minus | Star => ints(&l, &r).and_then(|(a, b)| {
                    let w = width_of(e, env)?;
                    let (a, b) = (a as u128, b as u128);
                    let x = match op {
                        Plus => a.wrapping_add(b),
                        Minus => a.wrapping_sub(b),
                        _ => a.wrapping_mul(b),
                    };
                    Some(VarValue::Int((x & mask(w)) as u64))
                }),
                ShiftLeft | ShiftRight => ints(&l, &r).and_then(|(a, b)| {
                    let w = width_of(left, env)?; // width of the LEFT operand
                    if (b as u32) >= w {
                        Some(VarValue::Int(0))
                    } else {
                        let a = a as u128;
                        let x = match op {
                            ShiftLeft => a << (b as u32),
                            _ => a >> (b as u32),
                        };
                        Some(VarValue::Int((x & mask(w)) as u64))
                    }
                }),
                BitAnd | BitOr | BitXor => {
                    if is_bool_expr(e, env) {
                        return bools(&l, &r).map(|(a, b)| {
                            VarValue::Bool(match op {
                                BitAnd => a && b,
                                BitOr => a || b,
                                _ => a != b,
                            })
                        });
                    }
                    ints(&l, &r).and_then(|(a, b)| {
                        let w = width_of(e, env)?;
                        let (a, b) = (a as u128, b as u128);
                        let x = match op {
                            BitAnd => a & b,
                            BitOr => a | b,
                            _ => a ^ b,
                        };
                        Some(VarValue::Int((x & mask(w)) as u64))
                    })
                }
            }
        }
    }
}

fn is_state_variant(module: &Module, name: &str) -> bool {
    module
        .enums
        .iter()
        .find(|e| e.name == "State")
        .map(|e| e.variants.iter().any(|v| v == name))
        .unwrap_or(false)
}

/// The bit width an arithmetic/bitwise result is masked to - exactly the
/// backend's `width_from_type` with no expected context: the expression's
/// validator type (Bit(w)), or 8 for integer literals.
fn width_of(e: &Expr, env: &EvalEnv) -> Option<u32> {
    match validator::type_expr(e, &env.venv).ok()? {
        ExprType::Bit(w) => Some(w),
        ExprType::IntLit => Some(8),
        _ => None,
    }
}

fn is_bool_expr(e: &Expr, env: &EvalEnv) -> bool {
    matches!(
        validator::type_expr(e, &env.venv).ok(),
        Some(ExprType::Bool)
    )
}

/// Evaluate a guard/atom expression and require a boolean result.
fn eval_bool(e: &Expr, env: &EvalEnv) -> Option<bool> {
    match eval_expr(e, env) {
        Some(VarValue::Bool(b)) => Some(b),
        _ => None,
    }
}

/// Bit mask for a width (mirrors `mask_expr`): all ones, u128-wide.
fn mask(w: u32) -> u128 {
    if w >= 128 {
        u128::MAX
    } else {
        (1u128 << w) - 1
    }
}

/// Apply a transition's assignments SIMULTANEOUSLY to a pre-step assignment
/// (all values computed from the pre-step variables, like the generated
/// temporary-next-variable lowering). Unassigned variables carry over.
fn apply_assignments(
    module: &Module,
    t: &Transition,
    pre_vars: &[(String, VarValue)],
) -> Vec<(String, VarValue)> {
    let pre_env = eval_env(module, "", pre_vars, &[], &HashMap::new());
    // Evaluate every assignment against the PRE-step values first.
    let computed: Vec<Option<VarValue>> = t
        .assignments
        .iter()
        .map(|a| eval_expr(&a.value, &pre_env))
        .collect();
    let mut out: Vec<(String, VarValue)> = pre_vars.to_vec();
    for (a, val) in t.assignments.iter().zip(computed.iter()) {
        if let Some(v) = val.clone() {
            match out.iter_mut().find(|(n, _)| n == &a.var) {
                Some(slot) => slot.1 = v,
                None => out.push((a.var.clone(), v)),
            }
        }
    }
    // Ensure every module variable is present (declaration order).
    module
        .vars
        .iter()
        .map(|v| {
            let val = out
                .iter()
                .find(|(n, _)| n == &v.name)
                .map(|(_, v)| v.clone())
                .or_else(|| representative_var_value(&v.ty, module))
                .unwrap_or(VarValue::Bool(false));
            (v.name.clone(), val)
        })
        .collect()
}

/// Evaluate emitted outputs against a concrete configuration (args that do
/// not evaluate become `None`).
fn eval_outputs(
    emits: &[OutputExpr],
    state: &str,
    vars: &[(String, VarValue)],
    bindings: &[(String, Type)],
    binding_vals: &HashMap<String, VarValue>,
    module: &Module,
) -> Vec<OutputCase> {
    let env = eval_env(module, state, vars, bindings, binding_vals);
    emits
        .iter()
        .map(|oe| OutputCase {
            group: oe.group.clone(),
            variant: oe.variant.clone(),
            args: oe.args.iter().map(|a| eval_expr(a, &env)).collect(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codegen_fsm::build_variants;
    use crate::monitor::{ReferenceMonitor, SignalAtom as SA, SignalExpr as SE};
    use crate::parse_source;

    fn variants(src: &str) -> (Module, FsmVariants) {
        let m = parse_source(src)
            .expect("parse")
            .modules
            .pop()
            .expect("module");
        (m.clone(), build_variants(&m))
    }

    fn module_file(path: &str) -> Module {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let src = std::fs::read_to_string(root.join(path)).expect("read example");
        parse_source(&src)
            .expect("parse")
            .modules
            .pop()
            .expect("module")
    }

    // -- pre-existing module-level flags ------------------------------------

    #[test]
    fn plan_flags_deadlocked_module() {
        let (m, v) = variants("module m { enum State { A } initial A event Go }");
        let plan = build_test_plan(&v, &m, 4);
        assert!(plan.definite_unexpected);
        assert!(!plan.expect_progress);
        assert!(is_deadlocked_initial(&m, &v));
    }

    #[test]
    fn plan_flags_fully_covered_module() {
        let (m, v) = variants(
            "module m { enum State { A, B } initial A event Go transition A on Go { next B; } }",
        );
        let plan = build_test_plan(&v, &m, 4);
        assert!(!plan.definite_unexpected);
        assert!(plan.expect_progress);
        assert!(!is_deadlocked_initial(&m, &v));
    }

    #[test]
    fn partial_coverage_is_definite_unexpected() {
        let (m, v) = variants(
            "module m { enum State { A, B } initial A event Recv(Syn) event Recv(Ack(x: bit<4>)) transition A on Recv(Syn) { next B; } }",
        );
        let plan = build_test_plan(&v, &m, 4);
        assert!(plan.definite_unexpected); // Recv_Ack is never matched
    }

    // -- LCG determinism pinning --------------------------------------------

    #[test]
    fn lcg_pinned_sequences_match_fuzz_wire_format() {
        // Two input variants (e.g. example 43: Tick, Ack(Seq)): seed 12345, K=8.
        let mut rng = Lcg::new(RANDOM_TRACE_SEED);
        let seq: Vec<usize> = (0..8).map(|_| rng.pick_index(2).unwrap()).collect();
        assert_eq!(seq, vec![0, 0, 1, 1, 0, 1, 1, 0]);

        // Five input variants: seed 12345, K=10.
        let mut rng = Lcg::new(RANDOM_TRACE_SEED);
        let seq: Vec<usize> = (0..10).map(|_| rng.pick_index(5).unwrap()).collect();
        assert_eq!(seq, vec![0, 1, 4, 4, 1, 2, 3, 1, 4, 0]);

        // Empty input set: no sequence.
        let mut rng = Lcg::new(RANDOM_TRACE_SEED);
        assert!(rng.pick_index(0).is_none());
    }

    #[test]
    fn random_trace_case_is_deterministic_and_pinned() {
        let (m, v) = variants(
            "module m { enum State { A } initial A event E1 event E2(X(f: bit<4>)) transition A on E1 { next A; } }",
        );
        let plan = build_test_plan(&v, &m, 8);
        assert_eq!(plan.random_trace.seed, RANDOM_TRACE_SEED);
        assert_eq!(plan.random_trace.steps, 8);
        // Two variants -> the pinned len-2 sequence.
        assert_eq!(
            plan.random_trace.input_indices,
            vec![0usize, 0, 1, 1, 0, 1, 1, 0]
        );

        // No inputs at all: empty sequence.
        let (m2, v2) = variants("module m { enum State { A } initial A }");
        let plan2 = build_test_plan(&v2, &m2, 4);
        assert!(plan2.random_trace.input_indices.is_empty());
    }

    // -- per-transition cases ------------------------------------------------

    #[test]
    fn unguarded_transition_gets_single_taken_case() {
        let (m, v) = variants(
            "module m { enum State { A, B } initial A event Go transition A on Go { next B; emit Out(X); } }",
        );
        let plan = build_test_plan(&v, &m, 4);
        assert_eq!(plan.transitions.len(), 1);
        let c = &plan.transitions[0];
        assert_eq!(c.transition_id, 0);
        assert_eq!(c.from_state, "A");
        assert_eq!(c.input.variant, "Go");
        match &c.expectation {
            TransitionExpectation::Taken {
                next_state,
                outputs,
            } => {
                assert_eq!(next_state, "B");
                assert_eq!(outputs.len(), 1);
                assert_eq!(outputs[0].group, "Out");
                assert_eq!(outputs[0].variant.as_deref(), Some("X"));
            }
            other => panic!("expected taken, got {other:?}"),
        }
    }

    #[test]
    fn guarded_transition_pins_both_cases_with_concrete_values() {
        let (m, v) = variants(
            "module m { enum State { A, B } initial A var x: bit<4> init 0 event Go transition A on Go where x < 8 { next B; emit Out(X(x)); } }",
        );
        let plan = build_test_plan(&v, &m, 4);
        // Taken case: first candidate with x < 8 true is x = 0.
        let taken = plan
            .transitions
            .iter()
            .find(|c| matches!(&c.expectation, TransitionExpectation::Taken { .. }));
        let not_taken = plan
            .transitions
            .iter()
            .find(|c| matches!(&c.expectation, TransitionExpectation::NotTaken { .. }));
        assert!(taken.is_some() && not_taken.is_some(), "plan: {plan:?}");
        let taken = taken.unwrap();
        assert_eq!(taken.vars, vec![("x".into(), VarValue::Int(0))]);
        match &taken.expectation {
            TransitionExpectation::Taken {
                next_state,
                outputs,
            } => {
                assert_eq!(next_state, "B");
                // emit Out(X(x)) with x = 0.
                assert_eq!(outputs[0].args, vec![Some(VarValue::Int(0))]);
            }
            other => panic!("expected taken, got {other:?}"),
        }
        // Not-taken case: first candidate with x < 8 false is x = 15 (2^4-1).
        let not_taken = not_taken.unwrap();
        assert_eq!(not_taken.vars, vec![("x".into(), VarValue::Int(15))]);
        match &not_taken.expectation {
            TransitionExpectation::NotTaken {
                only_candidate,
                on_error,
                signature,
            } => {
                assert!(*only_candidate);
                assert!(on_error.is_none());
                assert_eq!(signature.0, "B");
            }
            other => panic!("expected not-taken, got {other:?}"),
        }
    }

    #[test]
    fn unsatisfiable_guard_by_candidates_gets_note_and_not_taken_only() {
        // x == 8 is unreachable with bit<4> candidates [0, 1, 15].
        let (m, v) = variants(
            "module m { enum State { A, B } initial A var x: bit<4> init 0 event Go transition A on Go where x == 8 { next B; } }",
        );
        let plan = build_test_plan(&v, &m, 4);
        assert!(
            !plan
                .transitions
                .iter()
                .any(|c| matches!(&c.expectation, TransitionExpectation::Taken { .. })),
            "no taken case expected: {plan:?}"
        );
        assert!(plan
            .transitions
            .iter()
            .any(|c| matches!(&c.expectation, TransitionExpectation::NotTaken { .. })));
        assert!(
            plan.notes.iter().any(|n| n.subject == "transition #0"
                && n.message.contains("not statically satisfiable")),
            "notes: {:?}",
            plan.notes
        );
    }

    #[test]
    fn shadowed_transition_is_noted_and_gets_no_cases() {
        // Unguarded sibling first in source order always takes the input.
        let (m, v) = variants(
            "module m { enum State { A } initial A var x: bit<4> init 0 event Go transition A on Go { next A; } transition A on Go where x == 5 { next A; } }",
        );
        let plan = build_test_plan(&v, &m, 4);
        // Transition #0 (unguarded): taken. Transition #1: shadowed - no cases.
        assert_eq!(plan.transitions.len(), 1);
        assert_eq!(plan.transitions[0].transition_id, 0);
        assert!(
            plan.notes
                .iter()
                .any(|n| n.subject == "transition #1" && n.message.contains("shadowed")),
            "notes: {:?}",
            plan.notes
        );
    }

    #[test]
    fn higher_priority_sibling_must_be_disabled_for_taken_case() {
        // t1 (priority 2) fires before t0 for the same input; t0's taken case
        // needs an assignment where t1's guard is false.
        let (m, v) = variants(
            "module m { enum State { A, B } initial A var x: bit<4> init 0 event Go \
             transition A on Go where x == 1 { priority 2; next B; } \
             transition A on Go where x == 15 { next B; } }",
        );
        let plan = build_test_plan(&v, &m, 4);
        // t0 (x == 1, priority 2): taken case x = 1.
        let taken0 = plan
            .transitions
            .iter()
            .find(|c| {
                c.transition_id == 0
                    && matches!(&c.expectation, TransitionExpectation::Taken { .. })
            })
            .expect("t0 taken case");
        assert_eq!(taken0.vars, vec![("x".into(), VarValue::Int(1))]);
        // t1 (x == 15, no priority): taken case must use x = 15 - the only
        // candidate where t0's higher-priority guard is false.
        let taken1 = plan
            .transitions
            .iter()
            .find(|c| {
                c.transition_id == 1
                    && matches!(&c.expectation, TransitionExpectation::Taken { .. })
            })
            .expect("t1 taken case");
        assert_eq!(taken1.vars, vec![("x".into(), VarValue::Int(15))]);
    }

    #[test]
    fn pattern_literals_flow_into_input_and_guard_evaluation() {
        // Pattern binds a literal field; the guard references it.
        let (m, v) = variants(
            "module m { enum State { A, B } initial A event Recv(P(n: bit<4>)) transition A on Recv(P(n: 5)) where n == 5 { next B; } }",
        );
        let plan = build_test_plan(&v, &m, 4);
        assert_eq!(plan.transitions.len(), 1);
        let c = &plan.transitions[0];
        assert_eq!(c.input.fields, vec![("n".into(), VarValue::Int(5))]);
        assert!(matches!(
            &c.expectation,
            TransitionExpectation::Taken { .. }
        ));
    }

    #[test]
    fn example_43_plans_every_transition() {
        let m = module_file("examples/valid/43_sliding_window.rfs");
        let v = build_variants(&m);
        let plan = build_test_plan(&v, &m, 8);
        // All 8 transitions appear (taken or not-taken).
        let mut ids: Vec<usize> = plan.transitions.iter().map(|c| c.transition_id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids, vec![0usize, 1, 2, 3, 4, 5, 6, 7]);
        // Guarded transitions #2/#3/#4 each have a not-taken case; #2 has NO
        // taken case (x == 8 unreachable by candidates) with the note.
        assert!(plan.transitions.iter().any(|c| c.transition_id == 2
            && matches!(&c.expectation, TransitionExpectation::NotTaken { .. })));
        assert!(!plan.transitions.iter().any(|c| c.transition_id == 2
            && matches!(&c.expectation, TransitionExpectation::Taken { .. })));
        for id in [3usize, 4] {
            assert!(
                plan.transitions.iter().any(|c| c.transition_id == id
                    && matches!(&c.expectation, TransitionExpectation::Taken { .. })),
                "taken case for #{id}"
            );
            assert!(
                plan.transitions.iter().any(|c| c.transition_id == id
                    && matches!(&c.expectation, TransitionExpectation::NotTaken { .. })),
                "not-taken case for #{id}"
            );
        }
        // Unguarded transitions have exactly one (taken) case.
        for id in [0usize, 1, 5, 6, 7] {
            let cases: Vec<&TransitionTestCase> = plan
                .transitions
                .iter()
                .filter(|c| c.transition_id == id)
                .collect();
            assert_eq!(cases.len(), 1, "transition #{id}");
            assert!(matches!(
                &cases[0].expectation,
                TransitionExpectation::Taken { .. }
            ));
        }
        // The #2 satisfiability note is recorded.
        assert!(plan
            .notes
            .iter()
            .any(|n| n.subject == "transition #2"
                && n.message.contains("not statically satisfiable")));
    }

    // -- invariant selection ---------------------------------------------------

    #[test]
    fn invariants_select_safety_only_on_example_43() {
        let m = module_file("examples/valid/43_sliding_window.rfs");
        let v = build_variants(&m);
        let plan = build_test_plan(&v, &m, 8);
        // window_bound is the only safety property; `terminates` (eventually)
        // and `bounded_until_done` (until) are liveness - excluded. The
        // assumption env_tick is not a property at all.
        let labels: Vec<&str> = plan.invariants.iter().map(|i| i.label.as_str()).collect();
        assert_eq!(labels, vec!["window_bound"]);
        assert_eq!(plan.invariants[0].bound, None);
    }

    #[test]
    fn invariants_cover_bounded_always_and_skip_liveness() {
        let (m, v) = variants(
            "module m { enum State { A } initial A var x: bit<4> init 0 event E \
             prop s1 : always (x <= 3) ; \
             prop s2 : always[5] (state == A) ; \
             prop l1 : always (bounded_eventually[3] (x > 0)) ; \
             prop l2 : eventually (x > 0) ; \
             prop l3 : (x <= 3) until (x > 0) ; \
             transition A on E { next A; } }",
        );
        let plan = build_test_plan(&v, &m, 4);
        let labels: Vec<&str> = plan.invariants.iter().map(|i| i.label.as_str()).collect();
        assert_eq!(labels, vec!["s1", "s2"]);
        assert_eq!(plan.invariants[0].bound, None);
        assert_eq!(plan.invariants[1].bound, Some(5));
    }

    // -- monitor-violating traces ----------------------------------------------

    #[test]
    fn monitor_violation_at_initial_configuration_needs_no_step() {
        let (m, v) = variants(
            "module m { enum State { A, B } initial A var x: bit<4> init 7 event Go \
             transition A on Go { next B; set x = 0; } prop p : always (x <= 5); }",
        );
        let plan = build_test_plan(&v, &m, 4);
        assert_eq!(plan.monitor_violations.len(), 1);
        let c = &plan.monitor_violations[0];
        assert_eq!(c.label, "p");
        assert_eq!(c.state, "A");
        assert_eq!(c.vars, vec![("x".into(), VarValue::Int(7))]);
        assert!(c.step_input.is_none());
        assert_eq!(c.post_state, "A");
        assert_eq!(c.post_vars, vec![("x".into(), VarValue::Int(7))]);
    }

    #[test]
    fn monitor_violation_one_step_drive_pins_post_configuration() {
        let (m, v) = variants(
            "module m { enum State { A, B } initial A var x: bit<4> init 0 event Go \
             transition A on Go { next B; set x = 9; } prop p : always (x <= 5); }",
        );
        let plan = build_test_plan(&v, &m, 4);
        assert_eq!(plan.monitor_violations.len(), 1);
        let c = &plan.monitor_violations[0];
        assert_eq!(c.state, "A");
        assert_eq!(c.vars, vec![("x".into(), VarValue::Int(0))]);
        assert!(c.step_input.is_some()); // Go
        assert_eq!(c.post_state, "B");
        assert_eq!(c.post_vars, vec![("x".into(), VarValue::Int(9))]);
    }

    #[test]
    fn monitor_violation_non_atom_shapes_are_skipped_with_notes() {
        let (m, v) = variants(
            "module m { enum State { A, B } initial A event Go \
             prop i : always (input_is(Go)) ; \
             prop o : always (Go -> bounded_eventually[2] (state == B)) ; \
             transition A on Go { next B; } }",
        );
        let plan = build_test_plan(&v, &m, 4);
        assert!(plan.monitor_violations.is_empty());
        assert!(plan.notes.iter().any(
            |n| n.subject == "property i" && n.message.contains("not a state/vars expression")
        ));
        assert!(plan
            .notes
            .iter()
            .any(|n| n.subject == "property o" && n.message.contains("temporal operators")));
    }

    /// Exit-criterion check: the planned violating trace fires EXACTLY the
    /// target property's monitor in the Phase 3 reference interpreter - at
    /// the expected sample and not before.
    #[test]
    fn planned_violating_traces_fire_the_reference_monitor() {
        let specs = [
            "module m { enum State { A, B } initial A var x: bit<4> init 7 event Go transition A on Go { next B; set x = 0; } prop p : always (x <= 5); }",
            "module m { enum State { A, B } initial A var x: bit<4> init 0 event Go transition A on Go { next B; set x = 9; } prop p : always (x <= 5); }",
        ];
        for src in specs {
            let (m, v) = variants(src);
            let plan = build_test_plan(&v, &m, 4);
            assert_eq!(plan.monitor_violations.len(), 1, "{src}");
            let case = &plan.monitor_violations[0];
            let (Formula::Always { inner } | Formula::BoundedAlways { inner, .. }) =
                &m.properties[0].formula
            else {
                panic!("test spec must use always");
            };
            let SE::Atom(SA::VarExpr(_atom)) = monitor::formula_to_signal(inner).expect("atom")
            else {
                panic!("test spec must be an atom");
            };
            let mref = &m;
            // Samples: 0 = initial configuration; 1 = post-step (when a step
            // is planned). The reference monitor steps over them in order.
            let init_vars = init_var_values(mref);
            let expected_fire = if case.step_input.is_none() {
                Some(0)
            } else {
                Some(1)
            };
            let mut mon = ReferenceMonitor::new(
                &monitor::compile_formula(&case.label, false, inner).expect("compiles"),
            );
            let mut fired_at = None;
            for (s, (state, vars)) in std::iter::once((
                0usize,
                (mref.initial_state.as_deref().unwrap(), init_vars.as_slice()),
            ))
            .chain(case.step_input.as_ref().map(|_| {
                (
                    1usize,
                    (case.post_state.as_str(), case.post_vars.as_slice()),
                )
            })) {
                let env = eval_env(mref, state, vars, &[], &HashMap::new());
                let newly = mon.step(s, &|sig| {
                    matches!(sig, SE::Atom(SA::VarExpr(a)) if matches!(eval_expr(a, &env), Some(VarValue::Bool(true))))
                });
                if newly && fired_at.is_none() {
                    fired_at = Some(s);
                }
            }
            assert_eq!(fired_at, expected_fire, "trace for: {src}");
        }
    }

    #[test]
    fn example_43_monitor_violation_uses_bit_wraparound() {
        let m = module_file("examples/valid/43_sliding_window.rfs");
        let v = build_variants(&m);
        let plan = build_test_plan(&v, &m, 8);
        // window_bound (next_seq - acked <= 4) is violated by the first
        // candidate drive: from Idle with next_seq=0, acked=1 a Tick wraps
        // 0-1 to 15 in bit<4> at Active.
        assert_eq!(plan.monitor_violations.len(), 1);
        let c = &plan.monitor_violations[0];
        assert_eq!(c.label, "window_bound");
        assert_eq!(c.state, "Idle");
        assert_eq!(
            c.vars,
            vec![
                ("next_seq".into(), VarValue::Int(0)),
                ("acked".into(), VarValue::Int(1))
            ]
        );
        assert_eq!(c.post_state, "Active");
        // The liveness properties are noted, not planned.
        assert!(
            plan.notes
                .iter()
                .filter(|n| n.message.contains("not of the form"))
                .count()
                >= 2
        );
    }

    // -- fuzz decode plan --------------------------------------------------------

    #[test]
    fn fuzz_decode_covers_every_variant_with_field_strategies() {
        let (m, v) = variants(
            "module m { enum State { A } initial A enum Mode { M1, M2 } \
             event Tick \
             event Msg(P(b: bool, n: bit<13>, mode: Mode)) \
             transition A on Tick { next A; } }",
        );
        let plan = build_test_plan(&v, &m, 4);
        assert_eq!(plan.fuzz_decode.variants.len(), 2);
        assert_eq!(plan.fuzz_decode.variants[0].unique_name, "Tick");
        assert!(plan.fuzz_decode.variants[0].fields.is_empty());
        let p = &plan.fuzz_decode.variants[1];
        assert_eq!(p.unique_name, "Msg_P");
        assert_eq!(
            p.fields,
            vec![
                ("b".into(), FuzzFieldStrategy::ByteNonZero),
                ("n".into(), FuzzFieldStrategy::PayloadBytes { width: 13 }),
                ("mode".into(), FuzzFieldStrategy::Representative),
            ]
        );
    }

    #[test]
    fn example_43_fuzz_decode_pins_payload_widths() {
        let m = module_file("examples/valid/43_sliding_window.rfs");
        let v = build_variants(&m);
        let plan = build_test_plan(&v, &m, 8);
        assert_eq!(plan.fuzz_decode.variants.len(), 2);
        assert_eq!(plan.fuzz_decode.variants[0].unique_name, "Tick");
        assert_eq!(
            plan.fuzz_decode.variants[1].fields,
            vec![
                ("seq".into(), FuzzFieldStrategy::PayloadBytes { width: 4 }),
                (
                    "checksum".into(),
                    FuzzFieldStrategy::PayloadBytes { width: 32 }
                ),
            ]
        );
    }

    // -- corpus robustness ---------------------------------------------------------

    #[test]
    fn planner_runs_on_all_valid_examples() {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let files: Vec<std::path::PathBuf> =
            crate::collect_rfs_files(root.join("examples/valid")).expect("examples");
        assert_eq!(files.len(), 47, "corpus size");
        for file in &files {
            let src = std::fs::read_to_string(file).expect("read spec");
            let ir = parse_source(&src).expect("parse spec");
            let m = &ir.modules[0];
            let v = build_variants(m);
            let plan = build_test_plan(&v, m, 4);
            // Structural sanity: every case references a real transition; the
            // random trace stays in bounds; fuzz decode covers all variants.
            for c in &plan.transitions {
                assert!(c.transition_id < m.transitions.len(), "{}", file.display());
                assert_eq!(c.from_state, m.transitions[c.transition_id].state);
            }
            let n_inputs = v.inputs.len();
            for idx in &plan.random_trace.input_indices {
                assert!(
                    *idx < n_inputs,
                    "{}: random index out of bounds",
                    file.display()
                );
            }
            assert_eq!(
                plan.fuzz_decode.variants.len(),
                n_inputs,
                "{}",
                file.display()
            );
        }
    }

    // -- static evaluator sanity ---------------------------------------------------

    #[test]
    fn evaluator_mirrors_backend_width_semantics() {
        let (m, _v) = variants(
            "module m { enum State { A } initial A var x: bit<4> init 0 event E transition A on E { next A; } }",
        );
        // 15 + 1 wraps to 0 in bit<4>.
        let env = eval_env(&m, "A", &[], &[], &HashMap::new());
        let _ = env; // x is a variable: evaluate via assignment instead
        let env = eval_env(
            &m,
            "A",
            &[("x".into(), VarValue::Int(15))],
            &[],
            &HashMap::new(),
        );
        let e = crate::ir::Expr::Binary {
            op: BinOp::Plus,
            left: Box::new(Expr::Path {
                path: vec!["x".into()],
            }),
            right: Box::new(Expr::Int { value: 1 }),
        };
        assert_eq!(eval_expr(&e, &env), Some(VarValue::Int(0)));
        // Shifts saturate to 0 when the amount reaches the width.
        let e = crate::ir::Expr::Binary {
            op: BinOp::ShiftLeft,
            left: Box::new(Expr::Path {
                path: vec!["x".into()],
            }),
            right: Box::new(Expr::Int { value: 4 }),
        };
        assert_eq!(eval_expr(&e, &env), Some(VarValue::Int(0)));
        // State comparison.
        let e = crate::ir::Expr::Binary {
            op: BinOp::Eq,
            left: Box::new(Expr::Path {
                path: vec!["state".into()],
            }),
            right: Box::new(Expr::Path {
                path: vec!["A".into()],
            }),
        };
        assert_eq!(eval_expr(&e, &env), Some(VarValue::Bool(true)));
    }

    // -- Monitor-violation plan guarantees (Phase 5 task 5) -------------------

    /// The transition that actually fires for a concrete input in `state`
    /// under `vars`: firing order is descending effective priority, then
    /// source order - mirroring the generated `fsm_step`. `None` when nothing
    /// matches (or every matching guard fails).
    fn firing_transition<'a>(
        module: &'a Module,
        variants: &FsmVariants,
        state: &str,
        input_case: &InputCase,
        vars: &[(String, VarValue)],
    ) -> Option<&'a Transition> {
        let info = variants.input_variant(&input_case.group, &input_case.variant)?;
        let bindings: Vec<(String, Type)> = info
            .fields
            .iter()
            .map(|(n, t)| (n.clone(), t.clone()))
            .collect();
        let binding_vals: HashMap<String, VarValue> = input_case
            .fields
            .iter()
            .map(|(f, v)| (f.clone(), v.clone()))
            .collect();
        let mut idxs: Vec<usize> = (0..module.transitions.len())
            .filter(|&j| module.transitions[j].state == state)
            .filter(|&j| {
                pattern_can_match_variant(
                    &module.transitions[j].pattern,
                    &input_case.group,
                    &input_case.variant,
                )
            })
            .collect();
        idxs.sort_by_key(|&k| std::cmp::Reverse(module.transitions[k].priority.unwrap_or(0)));
        for &j in &idxs {
            let t = &module.transitions[j];
            let env = eval_env(module, state, vars, &bindings, &binding_vals);
            let fires = match &t.guard {
                Some(g) => eval_bool(g, &env) == Some(true),
                None => true,
            };
            if fires {
                return Some(t);
            }
        }
        None
    }

    /// The planned violating traces must satisfy the two properties the
    /// generated `monitor_<label>_fires_on_violation` tests rely on:
    ///
    /// 1. EXECUTABLE - the one-step drive actually takes the transition whose
    ///    post-step configuration is recorded in the case (no higher-firing
    ///    sibling matching the input fires first; firing order = priority desc,
    ///    source order).
    /// 2. TARGET FIRST - over the trace samples [initial config, post-step],
    ///    no other MONITORED property newly fails earlier, or at the same
    ///    sample with an earlier declaration order: the sticky
    ///    `property_violation` record must hold the target label.
    #[test]
    fn planned_monitor_violations_are_executable_and_record_target_first() {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let files: Vec<std::path::PathBuf> =
            crate::collect_rfs_files(root.join("examples/valid")).expect("examples");
        assert_eq!(files.len(), 47, "corpus size");
        // Recursive signal evaluator for the reference monitors (VarExpr atoms
        // only - the planned traces carry no input/output samples beyond the
        // step input, which is not part of a state/vars atom).
        fn eval_sig(sig: &SignalExpr, env: &EvalEnv) -> bool {
            match sig {
                SignalExpr::Atom(SignalAtom::True) => true,
                SignalExpr::Atom(SignalAtom::VarExpr(a)) => {
                    matches!(eval_expr(a, env), Some(VarValue::Bool(true)))
                }
                SignalExpr::Not(x) => !eval_sig(x, env),
                SignalExpr::And(l, r) => eval_sig(l, env) && eval_sig(r, env),
                SignalExpr::Or(l, r) => eval_sig(l, env) || eval_sig(r, env),
                _ => false,
            }
        }
        for file in &files {
            let src = std::fs::read_to_string(file).expect("read spec");
            let ir = parse_source(&src).expect("parse spec");
            let m = &ir.modules[0];
            let v = build_variants(m);
            let plan = build_test_plan(&v, m, 4);
            for case in &plan.monitor_violations {
                // (1) The drive takes the transition recorded in the case.
                if let Some(input) = &case.step_input {
                    let winner = firing_transition(m, &v, case.state.as_str(), input, &case.vars)
                        .unwrap_or_else(|| {
                            panic!(
                                "{}: no transition fires for the planned input of '{}'",
                                file.display(),
                                case.label
                            )
                        });
                    assert_eq!(
                        winner.next_state,
                        case.post_state,
                        "{}: case={}",
                        file.display(),
                        case.label
                    );
                    assert_eq!(
                        apply_assignments(m, winner, &case.vars),
                        case.post_vars,
                        "{}: case={}",
                        file.display(),
                        case.label
                    );
                }
                // (2) The target label is recorded first.
                let compiled: Vec<(usize, crate::monitor::MonitorPlan)> = m
                    .properties
                    .iter()
                    .enumerate()
                    .filter_map(|(i, p)| {
                        monitor::compile_formula(&p.name, false, &p.formula)
                            .ok()
                            .map(|pl| (i, pl))
                    })
                    .collect();
                let init_vars = init_var_values(m);
                let samples: Vec<(String, Vec<(String, VarValue)>)> = std::iter::once((
                    m.initial_state.clone().unwrap_or_default(),
                    init_vars.clone(),
                ))
                .chain(
                    case.step_input
                        .as_ref()
                        .map(|_| (case.post_state.clone(), case.post_vars.clone())),
                )
                .collect();
                let mut mons: Vec<crate::monitor::ReferenceMonitor> = compiled
                    .iter()
                    .map(|(_, pl)| crate::monitor::ReferenceMonitor::new(pl))
                    .collect();
                let mut recorded: Option<String> = None;
                for (s, (state, vars)) in samples.iter().enumerate() {
                    let env = eval_env(m, state, vars, &[], &HashMap::new());
                    for (i, mon) in mons.iter_mut().enumerate() {
                        if mon.step(s, &|sig| eval_sig(sig, &env)) && recorded.is_none() {
                            recorded = Some(m.properties[compiled[i].0].name.clone());
                        }
                    }
                }
                assert_eq!(
                    recorded.as_deref(),
                    Some(case.label.as_str()),
                    "{}: case={} recorded={:?}",
                    file.display(),
                    case.label,
                    recorded
                );
            }
        }
    }
}
