//! Language-agnostic PSL-to-monitor compiler (Phase 3).
//!
//! Compiles formulas from the Phase 3 property subset into a [`MonitorPlan`]:
//! a tree of monitor nodes that every backend renders with identical
//! semantics (backend parity principle, `docs/roadmap/INTRO.md`). The plan is
//! the single source of truth: Rust and C codegen render the same tree, so
//! both backends step monitors identically on the same input sequence.
//!
//! Semantics are pinned by D-P3-3 (`docs/roadmap/PHASE_3_PLAN.md`):
//! - samples carry post-step values; sample 0 is the initial configuration
//!   (no input/outputs, so `input_is`/`output_has` are false there);
//! - a formula is evaluated over the trace starting at position 0: temporal
//!   operators expand over positions, a bare signal asserts at its position;
//! - an obligation triggered at sample i requires its body to hold at some
//!   sample j with i <= j <= i + N - 1 (N >= 1); the violation is confirmed
//!   at sample i + N - 1 when the body still fails;
//! - `bounded_eventually[0]` means "the body must hold at the trigger sample
//!   itself" (violation confirmed at the trigger sample);
//! - `always[N] p`: p must hold at samples 0..N-1;
//! - `next p`: p must hold one sample later (vacuous when the trace ends).
//!
//! Monitor-compilability restrictions (R1-R4, enforced by
//! [`compile_formula`]): formulas outside the subset still parse and validate;
//! codegen in every backend emits the same "outside monitor subset; skipped"
//! warning and generates no monitor for them.

use crate::ir::{EventPattern, Expr, Formula, OutputPattern};

/// A per-sample boolean signal: a temporal-operator-free subformula.
// Note: no Eq/Hash derive - the IR types embedded here only
// implement PartialEq; structural equality is what backends/tests need.
#[derive(Debug, Clone, PartialEq)]
pub enum SignalExpr {
    Atom(SignalAtom),
    Not(Box<SignalExpr>),
    And(Box<SignalExpr>, Box<SignalExpr>),
    Or(Box<SignalExpr>, Box<SignalExpr>),
}

/// One atomic per-sample condition.
// Note: no Eq/Hash derive - the IR types embedded here only implement
// PartialEq; structural equality is what backends/tests need.
#[derive(Debug, Clone, PartialEq)]
pub enum SignalAtom {
    /// A boolean RFS expression over variables and the post-step state
    /// (`state == S`, variable comparisons, ...).
    VarExpr(Expr),
    InputIs(EventPattern),
    OutputHas(OutputPattern),
    /// The constant `true` signal (trigger of bare obligations). Renders as
    /// a plain boolean literal in every backend.
    True,
}

/// Anchor range: the samples at which a property/obligation is evaluated.
/// `to = None` means unbounded (all samples from `from` onward).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnchorRange {
    pub from: u64,
    pub to: Option<u64>,
}

impl AnchorRange {
    /// All samples from 0 onward (top-level evaluation).
    pub fn all() -> Self {
        Self { from: 0, to: None }
    }

    /// Whether sample `s` lies in the range.
    pub fn contains(&self, s: u64) -> bool {
        s >= self.from && self.to.is_none_or(|t| s <= t)
    }

    /// Shift the whole range by +1 (the effect of `next`).
    fn shifted(&self) -> Self {
        Self {
            from: self.from + 1,
            to: self.to.map(|t| t + 1),
        }
    }
}

/// A compiled monitor node. Every node exposes a sticky `failed` flag; the
/// property is violated when the root node fails (first failing sample wins).
#[derive(Debug, Clone, PartialEq)]
pub enum MonNode {
    /// `antecedent(i) -> consequent(i + 1)` for every anchor i in `range`:
    /// checked at sample s = i + 1 with a one-sample delay register holding
    /// the antecedent evaluated at s - 1. The register starts false, so the
    /// first checkable sample is vacuous (D-P3-3). Stateless apart from the
    /// register; no trigger step (per-sample failure semantics).
    DelayedCheck {
        antecedent: SignalExpr,
        consequent: SignalExpr,
        range: AnchorRange,
    },
    /// `expr` must hold at every sample in `range`. Violation at the first
    /// failing sample inside the range. Stateless: evaluated against the
    /// current sample index on every step.
    RangeCheck {
        expr: SignalExpr,
        range: AnchorRange,
    },
    /// For every anchor i in `range` where `trigger(i)` holds, `body` must
    /// hold at some sample j with i <= j <= i + bound - 1 (bound >= 1); for
    /// bound == 0 the body must hold at i itself. The violation is confirmed
    /// at sample i + bound - 1 (or i for bound == 0) when the body never held
    /// in the window. State: the D-P3-3 pending model (`pending`,
    /// `remaining`, sticky `trigger_step`) - a re-trigger while pending is
    /// ignored, which is safe because the pending obligation has the earliest
    /// deadline and the monitor stops at the first confirmed violation.
    Obligation {
        bound: u64,
        trigger: SignalExpr,
        body: SignalExpr,
        range: AnchorRange,
    },
    /// Formula conjunction: fails when ANY child has failed.
    And(Vec<MonNode>),
    /// Formula disjunction: fails when ALL children have failed (each child's
    /// failure is witnessed at a definite sample, so the conjunction of
    /// failures is confirmed when the last child fails).
    Or(Vec<MonNode>),
}

/// A fully compiled monitor for one property or assumption.
#[derive(Debug, Clone, PartialEq)]
pub struct MonitorPlan {
    pub label: String,
    pub is_assumption: bool,
    pub root: MonNode,
}

/// Compile a formula into a monitor plan.
///
/// `Err(msg)` means the formula is outside the monitor subset (R1-R4): no
/// monitor is generated and codegen warns (identical text in every backend).
pub fn compile_formula(
    label: &str,
    is_assumption: bool,
    f: &Formula,
) -> Result<MonitorPlan, String> {
    let root = compile_check(f, AnchorRange::all(), false)?;
    Ok(MonitorPlan {
        label: label.to_string(),
        is_assumption,
        root,
    })
}

/// `true` constant signal (used for bare obligations).
fn true_signal() -> SignalExpr {
    SignalExpr::Atom(SignalAtom::True)
}

/// Compile `f` as a check anchored at `range`: the formula must hold at every
/// position in `range`. `in_temporal` is true when an Always/Next/BoundedAlways
/// ancestor exists (affects self-anchoring of bounded constructs, R2/R3).
fn compile_check(f: &Formula, range: AnchorRange, in_temporal: bool) -> Result<MonNode, String> {
    match f {
        Formula::BoundedEventually { bound, inner } => {
            let body = compile_signal(inner).ok_or_else(|| {
                "bounded_eventually body must be temporal-operator-free".to_string()
            })?;
            let anchor = if in_temporal {
                range
            } else {
                AnchorRange {
                    from: 0,
                    to: Some(0),
                }
            };
            Ok(MonNode::Obligation {
                bound: *bound,
                trigger: true_signal(),
                body,
                range: anchor,
            })
        }
        Formula::Implies { left, right } => match right.as_ref() {
            // `trigger -> bounded_eventually[N] body`: the implication
            // antecedent is the obligation trigger (R2).
            Formula::BoundedEventually { bound, inner } => {
                let trigger = compile_signal(left).ok_or_else(|| {
                    "implication antecedent must be temporal-operator-free".to_string()
                })?;
                let body = compile_signal(inner).ok_or_else(|| {
                    "bounded_eventually body must be temporal-operator-free".to_string()
                })?;
                let anchor = if in_temporal {
                    range
                } else {
                    AnchorRange {
                        from: 0,
                        to: Some(0),
                    }
                };
                Ok(MonNode::Obligation {
                    bound: *bound,
                    trigger,
                    body,
                    range: anchor,
                })
            }
            Formula::Next { inner } => {
                // `p -> next q`: at anchor i, q must hold at i + 1 when p
                // holds at i - a one-sample delay check over the shifted range.
                let antecedent = compile_signal(left).ok_or_else(|| {
                    "implication antecedent must be temporal-operator-free".to_string()
                })?;
                let consequent = compile_signal(inner)
                    .ok_or_else(|| "next body must be temporal-operator-free".to_string())?;
                Ok(MonNode::DelayedCheck {
                    antecedent,
                    consequent,
                    range: range.shifted(),
                })
            }
            _ => {
                // Signal-level implication (both sides temporal-operator-free):
                // a per-sample check `!l || r`.
                let l = compile_signal(left).ok_or_else(|| {
                    "implication antecedent must be temporal-operator-free".to_string()
                })?;
                let r = compile_signal(right)
                    .ok_or_else(|| "implication consequent must be temporal-operator-free or bounded_eventually".to_string())?;
                Ok(MonNode::RangeCheck {
                    expr: SignalExpr::Or(Box::new(SignalExpr::Not(Box::new(l))), Box::new(r)),
                    range,
                })
            }
        },
        Formula::Always { inner } => compile_check(inner, range, true),
        Formula::BoundedAlways { bound, inner } => {
            // R3: `always[N]` only at top level or under boolean connectives -
            // never under another temporal operator.
            if in_temporal {
                return Err("always[N] is not allowed under a temporal operator".into());
            }
            let n = *bound;
            if n < 1 {
                return Err("always[N] requires N >= 1".into());
            }
            compile_check(
                inner,
                AnchorRange {
                    from: 0,
                    to: Some(n - 1),
                },
                true,
            )
        }
        Formula::Next { inner } => {
            // `next psi` at position i is `psi` at position i+1: for signal
            // children this shifts the anchor range by +1 (R4).
            let s = compile_signal(inner)
                .ok_or_else(|| "next requires a temporal-operator-free body".to_string())?;
            Ok(MonNode::RangeCheck {
                expr: s,
                range: range.shifted(),
            })
        }
        Formula::And { left, right } => Ok(MonNode::And(vec![
            compile_check(left, range, in_temporal)?,
            compile_check(right, range, in_temporal)?,
        ])),
        Formula::Or { left, right } => Ok(MonNode::Or(vec![
            compile_check(left, range, in_temporal)?,
            compile_check(right, range, in_temporal)?,
        ])),
        Formula::Not { inner } => {
            // R1: negation of a temporal formula is not monitorable (it would
            // be unbounded liveness). Only signal-level negation compiles.
            let s = compile_signal(inner).ok_or_else(|| {
                "negation of a temporal formula is outside the monitor subset".to_string()
            })?;
            Ok(MonNode::RangeCheck {
                expr: SignalExpr::Not(Box::new(s)),
                range,
            })
        }
        _ => {
            // Bare signal (Expr/InputIs/OutputHas): asserts at its position.
            let s = compile_signal(f)
                .ok_or_else(|| "formula is outside the monitor subset".to_string())?;
            Ok(MonNode::RangeCheck { expr: s, range })
        }
    }
}

/// Compile a temporal-operator-free subformula into a per-sample signal.
/// Returns `None` (not `Err`) when the formula contains temporal operators -
/// callers decide whether that is an error or a different shape.
fn compile_signal(f: &Formula) -> Option<SignalExpr> {
    match f {
        Formula::Expr { expr } => Some(SignalExpr::Atom(SignalAtom::VarExpr(expr.clone()))),
        Formula::InputIs { pattern } => {
            Some(SignalExpr::Atom(SignalAtom::InputIs(pattern.clone())))
        }
        Formula::OutputHas { pattern } => {
            Some(SignalExpr::Atom(SignalAtom::OutputHas(pattern.clone())))
        }
        Formula::Not { inner } => Some(SignalExpr::Not(Box::new(compile_signal(inner)?))),
        Formula::And { left, right } => Some(SignalExpr::And(
            Box::new(compile_signal(left)?),
            Box::new(compile_signal(right)?),
        )),
        Formula::Or { left, right } => Some(SignalExpr::Or(
            Box::new(compile_signal(left)?),
            Box::new(compile_signal(right)?),
        )),
        Formula::Implies { left, right } => Some(SignalExpr::Or(
            Box::new(SignalExpr::Not(Box::new(compile_signal(left)?))),
            Box::new(compile_signal(right)?),
        )),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Reference interpreter (tests only)
// ---------------------------------------------------------------------------

/// Runtime state of one monitor node, mirroring what backends generate.
///
/// The obligation state uses the pending model from D-P3-3 (the exact
/// algorithm both backends render), so the reference interpreter pins the same
/// observable behavior (violation sample + trigger step) as generated code.
#[derive(Debug, Clone)]
pub enum MonState {
    DelayedCheck {
        prev_antecedent: bool,
        failed: bool,
    },
    RangeCheck {
        failed: bool,
    },
    Obligation {
        bound: u64,
        pending: bool,
        remaining: u64,
        failed: bool,
        trigger_step: i64,
    },
    And {
        children: Vec<MonState>,
        failed: bool,
        trigger_step: i64,
    },
    Or {
        children: Vec<MonState>,
        failed: bool,
        trigger_step: i64,
    },
}

impl MonState {
    pub fn failed(&self) -> bool {
        match self {
            MonState::DelayedCheck { failed, .. } | MonState::RangeCheck { failed } => *failed,
            MonState::Obligation { failed, .. }
            | MonState::And { failed, .. }
            | MonState::Or { failed, .. } => *failed,
        }
    }

    pub fn trigger_step(&self) -> i64 {
        match self {
            MonState::DelayedCheck { .. } | MonState::RangeCheck { .. } => -1,
            MonState::Obligation { trigger_step, .. }
            | MonState::And { trigger_step, .. }
            | MonState::Or { trigger_step, .. } => *trigger_step,
        }
    }
}

/// Build the initial state tree for a plan (before sample 0).
pub fn initial_state(root: &MonNode) -> MonState {
    match root {
        MonNode::DelayedCheck { .. } => MonState::DelayedCheck {
            prev_antecedent: false,
            failed: false,
        },
        MonNode::RangeCheck { .. } => MonState::RangeCheck { failed: false },
        MonNode::Obligation { bound, .. } => MonState::Obligation {
            bound: *bound,
            pending: false,
            remaining: 0,
            failed: false,
            trigger_step: -1,
        },
        MonNode::And(children) => MonState::And {
            children: children.iter().map(initial_state).collect(),
            failed: false,
            trigger_step: -1,
        },
        MonNode::Or(children) => MonState::Or {
            children: children.iter().map(initial_state).collect(),
            failed: false,
            trigger_step: -1,
        },
    }
}

/// One reference monitor instance over a plan.
pub struct ReferenceMonitor {
    pub plan: MonitorPlan,
    state: MonState,
}

impl ReferenceMonitor {
    pub fn new(plan: &MonitorPlan) -> Self {
        let state = initial_state(&plan.root);
        Self {
            plan: plan.clone(),
            state,
        }
    }

    /// Step one sample. `eval` maps a signal to its value at this sample.
    /// Returns true when the monitor's failure is NEWLY confirmed here.
    pub fn step(&mut self, s: usize, eval: &impl Fn(&SignalExpr) -> bool) -> bool {
        let before = self.state.failed();
        step_node(&mut self.state, &self.plan.root, s as u64, eval);
        self.state.failed() && !before
    }
}

/// Step one node on sample `s`. Returns the node's CURRENT-SAMPLE failure
/// status ("now failing"): a per-sample check fails while its expression
/// holds false; an obligation fails only once its deadline has been confirmed
/// (terminal); And/Or combine their children's current statuses. The sticky
/// `failed` flag is set when `now` is true and stays set - it is what the
/// root reports as the property violation. Using per-sample (not sticky)
/// child status for Or/And composition is essential: disjuncts may fail at
/// different samples and recover, so a disjunction fails only while ALL of
/// its children fail at the SAME sample.
fn step_node(
    node: &mut MonState,
    spec: &MonNode,
    s: u64,
    eval: &impl Fn(&SignalExpr) -> bool,
) -> bool {
    match (node, spec) {
        (MonState::RangeCheck { failed }, MonNode::RangeCheck { expr, range }) => {
            let now = range.contains(s) && !eval(expr);
            if now {
                *failed = true;
            }
            now
        }
        (
            MonState::DelayedCheck {
                prev_antecedent,
                failed,
            },
            MonNode::DelayedCheck {
                antecedent,
                consequent,
                range,
            },
        ) => {
            if !*failed && range.contains(s) && *prev_antecedent && !eval(consequent) {
                *failed = true;
            }
            *prev_antecedent = eval(antecedent);
            *failed
        }
        (
            MonState::Obligation {
                bound,
                pending,
                remaining,
                failed,
                trigger_step,
            },
            MonNode::Obligation {
                bound: n,
                trigger,
                body,
                range,
            },
        ) => {
            debug_assert_eq!(*bound, *n);
            if !*failed {
                let a = range.contains(s) && eval(trigger);
                let b = eval(body);
                if *n == 0 {
                    // bound == 0: the body must hold at the trigger sample itself.
                    if a && !b {
                        *failed = true;
                        *trigger_step = s as i64;
                    }
                } else {
                    // D-P3-3 pending model (the algorithm backends render).
                    if !*pending && a {
                        *pending = true;
                        *remaining = *n;
                        *trigger_step = s as i64;
                    }
                    if *pending {
                        if b {
                            *pending = false;
                        } else {
                            *remaining -= 1;
                            if *remaining == 0 {
                                *failed = true;
                            }
                        }
                    }
                }
            }
            // Obligation failures are terminal: once confirmed, the node is
            // failing at every later sample.
            *failed
        }
        (
            MonState::And {
                children,
                failed,
                trigger_step,
            },
            MonNode::And(specs),
        ) => {
            let news: Vec<bool> = children
                .iter_mut()
                .zip(specs.iter())
                .map(|(c, sp)| step_node(c, sp, s, eval))
                .collect();
            let now = news.iter().any(|n| *n);
            if !*failed && now {
                *failed = true;
                *trigger_step = min_trigger_at(&news, children);
            }
            now
        }
        (
            MonState::Or {
                children,
                failed,
                trigger_step,
            },
            MonNode::Or(specs),
        ) => {
            let news: Vec<bool> = children
                .iter_mut()
                .zip(specs.iter())
                .map(|(c, sp)| step_node(c, sp, s, eval))
                .collect();
            let now = !news.is_empty() && news.iter().all(|n| *n);
            if !*failed && now {
                *failed = true;
                *trigger_step = min_trigger_at(&news, children);
            }
            now
        }
        _ => unreachable!("state/spec mismatch in reference interpreter"),
    }
}

/// Earliest trigger step among the CURRENTLY failing children (-1 when none
/// of them carries one, e.g. a pure per-sample check failure).
fn min_trigger_at(news: &[bool], children: &[MonState]) -> i64 {
    news.iter()
        .zip(children.iter())
        .filter(|(n, _)| **n)
        .map(|(_, c)| c.trigger_step())
        .filter(|t| *t >= 0)
        .min()
        .unwrap_or(-1)
}

// ---------------------------------------------------------------------------
// Tests: plan structure (R1-R4) + pinned monitor semantics (D-P3-3)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::BinOp;
    use crate::parser::parse_source;

    /// Parse a formula body inside a fixed test module.
    fn formula(src: &str) -> Formula {
        let full = format!(
            "module m {{ enum State {{ A, B }} initial A event Recv(Fin) output Send(Ack) prop p : {} ; }}",
            src
        );
        parse_source(&full)
            .expect("test source should parse")
            .modules
            .pop()
            .unwrap()
            .properties
            .pop()
            .unwrap()
            .formula
    }

    fn plan(src: &str) -> MonitorPlan {
        compile_formula("p", false, &formula(src)).expect("should compile")
    }

    /// Evaluate a signal against per-sample state/input/output sequences.
    fn eval_at(sig: &SignalExpr, s: usize, states: &[&str], fins: &[bool], acks: &[bool]) -> bool {
        match sig {
            SignalExpr::Atom(SignalAtom::True) => true,
            SignalExpr::Atom(SignalAtom::VarExpr(Expr::Binary {
                op: BinOp::Eq,
                left,
                right,
            })) => {
                if let (Expr::Path { path }, Expr::Path { path: rp }) =
                    (left.as_ref(), right.as_ref())
                {
                    if path == &["state".to_string()] && rp.len() == 1 {
                        return states[s] == rp[0];
                    }
                }
                panic!("unexpected expression atom in test");
            }
            SignalExpr::Atom(SignalAtom::VarExpr(_)) => {
                panic!("unexpected expression atom in test")
            }
            SignalExpr::Atom(SignalAtom::InputIs(p)) => {
                assert_eq!(p.group, "Recv");
                fins[s]
            }
            SignalExpr::Atom(SignalAtom::OutputHas(p)) => {
                assert_eq!(p.group, "Send");
                acks[s]
            }
            SignalExpr::Not(inner) => !eval_at(inner, s, states, fins, acks),
            SignalExpr::And(l, r) => {
                eval_at(l, s, states, fins, acks) && eval_at(r, s, states, fins, acks)
            }
            SignalExpr::Or(l, r) => {
                eval_at(l, s, states, fins, acks) || eval_at(r, s, states, fins, acks)
            }
        }
    }

    /// Run the reference monitor over `n_samples` samples; return
    /// (violation sample, trigger step) or None.
    fn run(
        p: &MonitorPlan,
        n_samples: usize,
        states: &[&str],
        fins: &[bool],
        acks: &[bool],
    ) -> Option<(usize, i64)> {
        let mut m = ReferenceMonitor::new(p);
        for s in 0..n_samples {
            if m.step(s, &|sig| eval_at(sig, s, states, fins, acks)) {
                return Some((s, m.state.trigger_step()));
            }
        }
        None
    }

    fn no(n: usize) -> Vec<bool> {
        vec![false; n]
    }

    // -- Plan structure (R1-R4) ------------------------------------------------

    #[test]
    fn plan_structure_bare_signal_is_invariant() {
        // Bare signals are invariants (anchored at the full range): sample 0
        // has no input/outputs, so a one-shot reading would make I/O atoms
        // vacuously false and assumptions like `assume a : input_is(Tick)`
        // prune every trace.
        match plan("state == A").root {
            MonNode::RangeCheck { ref range, .. } => assert_eq!(*range, AnchorRange::all()),
            other => panic!("expected RangeCheck, got {:?}", other),
        }
    }

    #[test]
    fn plan_structure_mixed_conjunction_is_in_subset() {
        // Task 23: formula-level `&&` mixing a state comparison and an I/O
        // atom (previously unparseable, now Formula::And) compiles as a
        // per-sample signal conjunction under R1-R4.
        let p = plan("always (state == A && output_has(Send(Ack)))");
        match &p.root {
            MonNode::And(children) => {
                assert_eq!(
                    children.len(),
                    2,
                    "expected two conjuncts, got {:?}",
                    children
                );
                for c in children {
                    match c {
                        MonNode::RangeCheck { ref range, .. } => {
                            assert_eq!(*range, AnchorRange::all())
                        }
                        other => panic!("expected RangeCheck conjunct, got {:?}", other),
                    }
                }
            }
            other => panic!("expected And of per-sample checks, got {:?}", other),
        }
    }

    #[test]
    fn plan_structure_always_and_bounded_always_ranges() {
        match plan("always (state == A)").root {
            MonNode::RangeCheck { ref range, .. } => {
                assert_eq!(*range, AnchorRange::all())
            }
            other => panic!("expected RangeCheck, got {:?}", other),
        }
        match plan("always[5] (state == A)").root {
            MonNode::RangeCheck { ref range, .. } => {
                assert_eq!(
                    *range,
                    AnchorRange {
                        from: 0,
                        to: Some(4)
                    }
                )
            }
            other => panic!("expected RangeCheck, got {:?}", other),
        }
    }

    #[test]
    fn plan_structure_next_shifts_range_by_one() {
        match plan("next (state == A)").root {
            MonNode::RangeCheck { ref range, .. } => {
                // `next p` at every position i is `p` at i+1: the invariant
                // range [0, inf) shifts to [1, inf).
                assert_eq!(*range, AnchorRange { from: 1, to: None })
            }
            other => panic!("expected RangeCheck, got {:?}", other),
        }
        match plan("always (next (state == A))").root {
            MonNode::RangeCheck { ref range, .. } => {
                assert_eq!(*range, AnchorRange { from: 1, to: None })
            }
            other => panic!("expected RangeCheck, got {:?}", other),
        }
    }

    #[test]
    fn plan_structure_obligation_anchors() {
        // Top-level bare be[N]: one-shot anchor at sample 0.
        match plan("bounded_eventually[3] (output_has(Send(Ack)))").root {
            MonNode::Obligation {
                bound, ref range, ..
            } => {
                assert_eq!(bound, 3);
                assert_eq!(
                    *range,
                    AnchorRange {
                        from: 0,
                        to: Some(0)
                    }
                );
            }
            other => panic!("expected Obligation, got {:?}", other),
        }
        // Top-level implication: trigger = antecedent, one-shot anchor.
        match plan("input_is(Recv(Fin)) -> bounded_eventually[3] (output_has(Send(Ack)))").root {
            MonNode::Obligation {
                bound, ref range, ..
            } => {
                assert_eq!(bound, 3);
                assert_eq!(
                    *range,
                    AnchorRange {
                        from: 0,
                        to: Some(0)
                    }
                );
            }
            other => panic!("expected Obligation, got {:?}", other),
        }
        // Under always: recurring (unbounded anchor range).
        match plan("always (input_is(Recv(Fin)) -> bounded_eventually[3] (output_has(Send(Ack))))")
            .root
        {
            MonNode::Obligation {
                bound, ref range, ..
            } => {
                assert_eq!(bound, 3);
                assert_eq!(*range, AnchorRange::all());
            }
            other => panic!("expected Obligation, got {:?}", other),
        }
        // Under always[N]: anchor window [0, N-1].
        match plan(
            "always[4] (input_is(Recv(Fin)) -> bounded_eventually[3] (output_has(Send(Ack))))",
        )
        .root
        {
            MonNode::Obligation {
                bound, ref range, ..
            } => {
                assert_eq!(bound, 3);
                assert_eq!(
                    *range,
                    AnchorRange {
                        from: 0,
                        to: Some(3)
                    }
                );
            }
            other => panic!("expected Obligation, got {:?}", other),
        }
    }

    #[test]
    fn plan_structure_rejects_outside_subset() {
        // R1: no bounded_eventually under not.
        assert!(plan_err("!bounded_eventually[3] (output_has(Send(Ack)))"));
        assert!(plan_err(
            "always (!bounded_eventually[3] (output_has(Send(Ack))))"
        ));
        // R2: be body must be temporal-operator-free.
        assert!(plan_err("bounded_eventually[3] (always (state == A))"));
        // R3: always[N] never under a temporal operator.
        assert!(plan_err("always (always[2] (state == A))"));
        assert!(plan_err("next (always[2] (state == A))"));
        // R4: next requires a safety body.
        assert!(plan_err(
            "next (bounded_eventually[3] (output_has(Send(Ack))))"
        ));
        // Implication with temporal consequent other than be[N].
        assert!(plan_err("state == A -> always (state == B)"));
    }

    fn plan_err(src: &str) -> bool {
        compile_formula("p", false, &formula(src)).is_err()
    }

    // -- Pinned semantics (D-P3-3) --------------------------------------------

    const FIN_ACK: &str =
        "always (input_is(Recv(Fin)) -> bounded_eventually[3] (output_has(Send(Ack))))";

    #[test]
    fn fin_ack_violation_confirmed_at_exact_deadline() {
        let p = plan(FIN_ACK);
        // Fin at sample 2, no Ack anywhere: window [2..4], violation at 4.
        let fins = vec![false, false, true, false, false, false];
        assert_eq!(run(&p, 6, &no_states(6), &fins, &no(6)), Some((4, 2)));
    }

    #[test]
    fn fin_ack_same_step_satisfaction() {
        let p = plan(FIN_ACK);
        // Fin and Ack at the same sample: satisfied immediately.
        let fins = vec![false, false, true, false, false];
        let acks = vec![false, false, true, false, false];
        assert_eq!(run(&p, 5, &no_states(5), &fins, &acks), None);
    }

    #[test]
    fn fin_ack_deadline_sample_satisfaction() {
        let p = plan(FIN_ACK);
        // Fin at 2, Ack exactly at the deadline (4): no violation.
        let fins = vec![false, false, true, false, false, false];
        let acks = vec![false, false, false, false, true, false];
        assert_eq!(run(&p, 6, &no_states(6), &fins, &acks), None);
    }

    #[test]
    fn fin_ack_one_sample_past_deadline_is_violation() {
        let p = plan(FIN_ACK);
        // Fin at 2, Ack only at 5 (one past deadline 4): violation at 4.
        let fins = vec![false, false, true, false, false, false];
        let acks = vec![false, false, false, false, false, true];
        assert_eq!(run(&p, 6, &no_states(6), &fins, &acks), Some((4, 2)));
    }

    #[test]
    fn periodic_bounded_eventually() {
        // always(be[3] ack): an obligation starts at EVERY sample.
        let p = plan("always (bounded_eventually[3] (output_has(Send(Ack))))");
        // Ack only at sample 1: obligations from 0 and 1 are saved by it;
        // the obligation from 2 needs ack in [2..4] -> violation at 4,
        // triggered at step 2.
        let acks = vec![false, true, false, false, false, false];
        assert_eq!(run(&p, 6, &no_states(6), &no(6), &acks), Some((4, 2)));
    }

    #[test]
    fn bounded_always_window() {
        let p = plan("always[5] (state == A)");
        // Window samples 0..4: state B at sample 3 -> violation at 3.
        let states = vec!["A", "A", "A", "B", "A"];
        assert_eq!(run(&p, 5, &states, &no(5), &no(5)), Some((3, -1)));
        // State B only at sample 5 (outside the window): no violation.
        let states = vec!["A", "A", "A", "A", "A", "B"];
        assert_eq!(run(&p, 6, &states, &no(6), &no(6)), None);
    }

    #[test]
    fn next_delay_is_vacuous_at_sample_zero() {
        let p = plan("next (state == A)");
        // `next` asserts at sample 1 only: state B at 1 -> violation at 1.
        let states = vec!["A", "B"];
        assert_eq!(run(&p, 2, &states, &no(2), &no(2)), Some((1, -1)));
        // A trace with only the initial sample never reaches sample 1.
        let states = vec!["A"];
        assert_eq!(run(&p, 1, &states, &no(1), &no(1)), None);
    }

    #[test]
    fn bounded_eventually_zero_is_same_step() {
        // be[0]: the body must hold at the trigger sample itself.
        let p =
            plan("always (input_is(Recv(Fin)) -> bounded_eventually[0] (output_has(Send(Ack))))");
        let fins = vec![false, false, true, false];
        // Ack not at the trigger sample (only later): violation AT sample 2.
        let acks = vec![false, false, false, true];
        assert_eq!(run(&p, 4, &no_states(4), &fins, &acks), Some((2, 2)));
        // Ack at the trigger sample: satisfied.
        let acks = vec![false, false, true, false];
        assert_eq!(run(&p, 4, &no_states(4), &fins, &acks), None);
    }

    #[test]
    fn disjunction_of_obligations_fails_when_all_fail() {
        // be[3] ack || be[3] fin: violated only when NEITHER holds in [0..2].
        let p = plan("bounded_eventually[3] (output_has(Send(Ack))) || bounded_eventually[3] (input_is(Recv(Fin)))");
        // Neither in the window: both one-shot obligations (triggered at
        // sample 0) fail -> violation at 2 with trigger step 0.
        assert_eq!(run(&p, 3, &no_states(3), &no(3), &no(3)), Some((2, 0)));
        // Fin at 1 saves the disjunction.
        let fins = vec![false, true, false];
        assert_eq!(run(&p, 3, &no_states(3), &fins, &no(3)), None);
    }

    #[test]
    fn disjunction_of_invariants_children_fail_at_different_samples() {
        // fin || ack as invariants: each child fails on the samples where it
        // does not hold, but they never fail at the SAME sample -> no
        // violation. (Pins the per-sample Or semantics: sticky child flags
        // would falsely report a violation at sample 1.)
        let p = plan("input_is(Recv(Fin)) || output_has(Send(Ack))");
        let fins = vec![false, true, false];
        let acks = vec![true, false, true];
        assert_eq!(run(&p, 3, &no_states(3), &fins, &acks), None);
        // Both fail at sample 0 -> violation confirmed there.
        assert_eq!(run(&p, 1, &no_states(1), &[false], &[false]), Some((0, -1)));
    }

    #[test]
    fn implication_next_uses_delay_register() {
        // fin -> next ack: the antecedent at sample i must be paired with the
        // consequent at i+1 (one-sample delay register, vacuous at sample 0).
        let p = plan("input_is(Recv(Fin)) -> next (output_has(Send(Ack)))");
        // Fin at 1, Ack at 2: satisfied.
        let fins = vec![false, true, false];
        let acks = vec![false, false, true];
        assert_eq!(run(&p, 3, &no_states(3), &fins, &acks), None);
        // Fin at 1, no Ack at 2: violation confirmed at sample 2.
        let fins = vec![false, true, false];
        let acks = vec![false, false, false];
        assert_eq!(run(&p, 3, &no_states(3), &fins, &acks), Some((2, -1)));
    }

    #[test]
    fn always_implication_next_under_bounded_window() {
        // always[3](fin -> next ack): positions 0..2, checks at samples 1..3.
        let p = plan("always[3] (input_is(Recv(Fin)) -> next (output_has(Send(Ack))))");
        // Fin at position 2: the check lands exactly at the window edge (s3).
        let fins = vec![false, false, true, false];
        let acks = vec![false, false, false, true];
        assert_eq!(run(&p, 4, &no_states(4), &fins, &acks), None);
        // No Ack at s3: violation confirmed there.
        let fins = vec![false, false, true, false, false];
        let acks = vec![false, false, false, false, false];
        assert_eq!(run(&p, 5, &no_states(5), &fins, &acks), Some((3, -1)));
    }

    #[test]
    fn and_under_or_recovers_per_sample() {
        // ((state == A) && fin) || ack: at sample 1 the And child fails
        // (state B) but the Ack disjunct holds; at sample 0 the And holds
        // while Ack fails. No simultaneous all-children failure -> no
        // violation, even though every child is sticky-failed by sample 2.
        let p = plan("((state == A) && input_is(Recv(Fin))) || output_has(Send(Ack))");
        let states = vec!["A", "B"];
        let fins = vec![true, false];
        let acks = vec![false, true];
        assert_eq!(run(&p, 2, &states, &fins, &acks), None);
    }

    #[test]
    fn conjunction_fails_when_any_child_fails() {
        // always(state == A) && be[2] ack: violated when either fails.
        let p = plan("always (state == A) && bounded_eventually[2] (output_has(Send(Ack)))");
        // State B at sample 1 -> violation at 1 even though ack holds at 0.
        let states = vec!["A", "B"];
        let acks = vec![true, false];
        assert_eq!(run(&p, 2, &states, &no(2), &acks), Some((1, -1)));
    }

    fn no_states(n: usize) -> Vec<&'static str> {
        vec!["A"; n]
    }
}
