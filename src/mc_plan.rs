//! Codegen-time plan for the exact model-checking engine (Phase 4, D-P4-1..3).
//!
//! Computed ONCE in the Rust tool from the validated IR and embedded as
//! constants/tables in both generated MC binaries, so the Rust and C engines
//! are identical by construction (mirrors the Phase 3 `MonitorPlan` pattern):
//!
//! - **Input abstraction** (D-P4-3): which payload fields of each event variant
//!   must be enumerated concretely vs collapsed to a single representative
//!   value. A field is concrete iff it is referenced by any transition guard,
//!   output payload expression, variable update expression, or
//!   property/assumption formula (`input_is` patterns with field values count
//!   as references); otherwise it is abstracted.
//! - **Configuration encoding** (D-P4-2): the bit layout of a configuration =
//!   (state index, packed variables, monitor transition states). The visited
//!   set hashes this buffer, so both engines must pack identically.
//! - **Traversal spec** (D-P4-2): canonical input order (events in declaration
//!   order, payload fields in declaration order, values ascending) and the
//!   per-state transition firing order used by both det mode (first match
//!   fires) and nondet mode (all matches are successors, in this order).
//!
//! The plan is a pure function of the module (+ assumption selection): no
//! environment reads, no randomness. It is not consumed by any backend until
//! Phase 4 task 2; nothing here changes generated output.

use std::collections::HashSet;

use crate::codegen::AssumptionSelection;
use crate::codegen_fsm::{build_variants, representative_values, RepValue, VariantInfo};
use crate::ir::*;
use crate::monitor::{compile_formula, MonNode, MonitorPlan};

// ---------------------------------------------------------------------------
// Input abstraction (D-P4-3)
// ---------------------------------------------------------------------------

/// Canonical enumeration domain of one CONCRETE payload field. Value order is
/// ascending: bool `false < true`; `bit<N>` `0..=max`; enum in declaration
/// order.
#[derive(Debug, Clone, PartialEq)]
pub enum ValueDomain {
    /// `bool`: false then true.
    Bool,
    /// `bit<N>`: 0..=max ascending (max = 2^N - 1; saturated to u64::MAX for
    /// N >= 64, which exact MC cannot enumerate anyway).
    Int { max: u64 },
    /// Enum type: declared variants in declaration order.
    Enum { variants: Vec<String> },
}

/// Per-field enumeration spec of one event variant (declaration order).
#[derive(Debug, Clone, PartialEq)]
pub struct FieldSpec {
    pub name: String,
    /// `true`: enumerate the full domain (`domain`); `false`: use the single
    /// representative value (`representative`).
    pub concrete: bool,
    /// Full canonical domain (concrete fields only).
    pub domain: Option<ValueDomain>,
    /// Representative value (abstracted fields only).
    pub representative: Option<RepValue>,
}

/// Input enumeration spec of one event variant.
#[derive(Debug, Clone, PartialEq)]
pub struct VariantInputSpec {
    pub group: String,
    pub variant: String,
    /// Collision-free name of the flattened input constructor (the name used
    /// in the generated `Input` enum).
    pub unique_name: String,
    /// Per declared field, in declaration order.
    pub fields: Vec<FieldSpec>,
}

/// The input-abstraction table: one entry per event variant, in the same
/// declaration order as `build_variants`.
#[derive(Debug, Clone, PartialEq)]
pub struct InputAbstraction {
    pub variants: Vec<VariantInputSpec>,
}

// ---------------------------------------------------------------------------
// Configuration encoding (D-P4-2)
// ---------------------------------------------------------------------------

/// Bit slot of one module variable inside a configuration (declaration order).
#[derive(Debug, Clone, PartialEq)]
pub struct VarSlot {
    pub name: String,
    /// Bit offset within the configuration buffer (variables start right after
    /// the state index).
    pub offset_bits: u32,
    /// Number of bits storing the value.
    pub width_bits: u32,
}

/// Persistent transition state of one monitor node inside the packed
/// configuration. Only nodes that carry persistent state appear here (node ids
/// are preorder and match the backends' `n{id}` naming).
///
/// The sticky `failed`/`trigger_step` flags are deliberately NOT part of the
/// configuration: every configuration enqueued by the exact engine has no
/// confirmed failure yet (a safety failure terminates the search with a
/// counterexample before enqueue), so the flags are invariant (false/-1) on
/// all live configurations and carry no information.
#[derive(Debug, Clone, PartialEq)]
pub struct MonitorNodeState {
    /// Node id (preorder over the monitor tree).
    pub node_id: usize,
    /// Bit offset within the monitor's state block.
    pub offset_bits: u32,
    /// 1 bit for the pending flag (`Obligation` with bound >= 1 only).
    pub pending_bits: u32,
    /// ceil(log2(bound+1)) bits for the remaining counter (`Obligation` with
    /// bound >= 1 only; values 0..=bound).
    pub remaining_bits: u32,
    /// 1 bit for the antecedent register (`DelayedCheck` only).
    pub antecedent_bits: u32,
}

impl MonitorNodeState {
    /// Total bits of this node's persistent state.
    pub fn total_bits(&self) -> u32 {
        self.pending_bits + self.remaining_bits + self.antecedent_bits
    }
}

/// Packed transition state of one monitor (properties in declaration order,
/// then selected assumptions in declaration order - the aggregate stepping
/// order).
#[derive(Debug, Clone, PartialEq)]
pub struct MonitorStateLayout {
    pub label: String,
    pub is_assumption: bool,
    /// Absolute bit offset of this monitor's block within the configuration
    /// (assigned by `config_encoding`).
    pub offset_bits: u32,
    /// Total bits of this monitor's transition state (0 for monitors made of
    /// pure per-sample checks).
    pub total_bits: u32,
    /// Per-node state slots in preorder (only nodes with persistent state).
    pub nodes: Vec<MonitorNodeState>,
}

/// The configuration encoding: config = (state index, packed variables,
/// monitor transition states), bit-packed little-endian into `total_bytes`.
#[derive(Debug, Clone, PartialEq)]
pub struct ConfigEncoding {
    /// Number of states (variants of the `State` enum).
    pub state_count: usize,
    /// Bits for the state index (0 when state_count <= 1).
    pub state_bits: u32,
    /// Per-variable slots in declaration order.
    pub var_slots: Vec<VarSlot>,
    /// Total bits of all variables.
    pub vars_bits: u32,
    /// Monitor layouts in aggregate stepping order.
    pub monitors: Vec<MonitorStateLayout>,
    /// Total bits of all monitor transition states.
    pub monitors_bits: u32,
    /// `state_bits + vars_bits + monitors_bits` (before byte padding).
    pub total_bits: u32,
    /// `total_bits` rounded up to whole bytes - the buffer size both engines
    /// hash for the visited set.
    pub total_bytes: usize,
}

// ---------------------------------------------------------------------------
// Traversal spec (D-P4-2)
// ---------------------------------------------------------------------------

/// Per-state transition firing order: effective priority (`priority N`
/// descending, declaration order for ties) - the canonical successor order
/// used by det mode (first match fires) and nondet mode (all matches are
/// successors, in this order).
#[derive(Debug, Clone, PartialEq)]
pub struct StateTransitionOrder {
    pub state: String,
    /// Transition ids in firing order.
    pub transitions: Vec<usize>,
}

/// Traversal spec shared by both engines.
#[derive(Debug, Clone, PartialEq)]
pub struct TraversalSpec {
    /// Per event variant (same order as `InputAbstraction.variants`): the
    /// canonical value domain per field in declaration order. `None` marks an
    /// abstracted field (single representative value, see `FieldSpec`).
    pub value_domains: Vec<Vec<Option<ValueDomain>>>,
    /// Per state (declaration order of the `State` enum): transitions in
    /// effective firing order.
    pub state_orders: Vec<StateTransitionOrder>,
    /// All assumption labels in declaration order (the engine's known-labels
    /// list for `--assumes` selection; D-P4-6).
    pub assumption_labels: Vec<String>,
}

// ---------------------------------------------------------------------------
// Top-level plan
// ---------------------------------------------------------------------------

/// The complete codegen-time MC plan (D-P4-1): computed once from the
/// validated IR; both backends render it verbatim, so the engines are
/// identical by construction.
#[derive(Debug, Clone, PartialEq)]
pub struct McPlan {
    pub abstraction: InputAbstraction,
    pub encoding: ConfigEncoding,
    pub traversal: TraversalSpec,
}

impl McPlan {
    /// Compute the plan for a validated module. `assumptions` selects which
    /// assumptions are active (mirrors the codegen `--assumes` option); only
    /// selected assumptions contribute monitor state to the encoding.
    pub fn compute(module: &Module, assumptions: &AssumptionSelection) -> McPlan {
        let variants = build_variants(module);
        let referenced = referenced_fields(module, &variants.inputs);
        let abstraction = InputAbstraction {
            variants: variants
                .inputs
                .iter()
                .enumerate()
                .map(|(i, v)| {
                    let reps = representative_values(v, module);
                    VariantInputSpec {
                        group: v.group.clone(),
                        variant: v.variant.clone(),
                        unique_name: v.unique_name.clone(),
                        fields: v
                            .fields
                            .iter()
                            .enumerate()
                            .map(|(fi, (name, ty))| {
                                let concrete = referenced[i].contains(&fi);
                                FieldSpec {
                                    name: name.clone(),
                                    concrete,
                                    domain: concrete.then(|| value_domain(ty, module)),
                                    representative: (!concrete).then(|| reps[fi].1.clone()),
                                }
                            })
                            .collect(),
                    }
                })
                .collect(),
        };
        let mut monitors = monitor_layouts(module, assumptions);
        let encoding = config_encoding(module, &mut monitors);
        let traversal = traversal_spec(module, &abstraction);
        McPlan {
            abstraction,
            encoding,
            traversal,
        }
    }
}

/// Smallest k with 2^k >= n (0 for n <= 1).
fn ceil_log2(n: u64) -> u32 {
    if n <= 1 {
        return 0;
    }
    let floor = 63 - n.leading_zeros();
    if n & (n - 1) == 0 {
        floor
    } else {
        floor + 1
    }
}

/// Bit width of a type when packed into a configuration.
fn type_bits(ty: &Type, module: &Module) -> u32 {
    match ty {
        Type::Bool => 1,
        Type::Bit { width } => *width,
        Type::Enum { name } => {
            let n = module
                .enums
                .iter()
                .find(|e| e.name == *name)
                .map(|e| e.variants.len())
                .unwrap_or(0) as u64;
            ceil_log2(n)
        }
    }
}

/// Canonical value domain of a payload field type.
fn value_domain(ty: &Type, module: &Module) -> ValueDomain {
    match ty {
        Type::Bool => ValueDomain::Bool,
        Type::Bit { width } => {
            let max = if *width >= 64 {
                u64::MAX
            } else {
                (1u64 << width) - 1
            };
            ValueDomain::Int { max }
        }
        Type::Enum { name } => ValueDomain::Enum {
            variants: module
                .enums
                .iter()
                .find(|e| e.name == *name)
                .map(|e| e.variants.clone())
                .unwrap_or_default(),
        },
    }
}

// ---------------------------------------------------------------------------
// Input-abstraction analysis (D-P4-3)
// ---------------------------------------------------------------------------

/// Per event variant: the set of field positions referenced by guards, output
/// payload expressions, variable update expressions, or property/assumption
/// formulas. A field outside this set is abstracted.
fn referenced_fields(module: &Module, inputs: &[VariantInfo]) -> Vec<HashSet<usize>> {
    let mut refd: Vec<HashSet<usize>> = inputs.iter().map(|_| HashSet::new()).collect();

    // (a) Transitions: a bound field is referenced iff its binding name is
    // used in the guard, an emit argument, or an assignment value.
    for t in &module.transitions {
        let Some(PatternVariant::Struct { name, fields }) = &t.pattern.variant else {
            continue; // group-level / unit patterns bind no payload fields
        };
        let Some(vi) = inputs
            .iter()
            .position(|v| v.group == t.pattern.group && v.variant == *name)
        else {
            continue;
        };
        let mut bindings: Vec<(usize, String)> = Vec::new();
        for (i, pf) in fields.iter().enumerate() {
            if let PatternExpr::Bind { name } = &pf.pattern {
                bindings.push((i, name.clone()));
            }
        }
        if bindings.is_empty() {
            continue;
        }
        let used = used_identifiers(t);
        for (i, bname) in bindings {
            if i < inputs[vi].fields.len() && used.contains(&bname) {
                refd[vi].insert(i);
            }
        }
    }

    // (b) Properties + assumptions: `input_is` patterns with CONSTRAINING field
    // patterns (literals / enum-variant matches). Wildcards and binds constrain
    // nothing, so they do not count as references.
    let mut formulas: Vec<&Formula> = module.properties.iter().map(|p| &p.formula).collect();
    formulas.extend(module.assumptions.iter().map(|a| &a.formula));
    for f in formulas {
        collect_input_is_refs(f, inputs, &mut refd);
    }

    refd
}

/// Single-segment path identifiers used in a transition's guard, emit args,
/// and assignment values (the contexts where pattern bindings are in scope).
fn used_identifiers(t: &Transition) -> HashSet<String> {
    let mut out = HashSet::new();
    if let Some(g) = &t.guard {
        collect_paths(g, &mut out);
    }
    for e in &t.emits {
        for a in &e.args {
            collect_paths(a, &mut out);
        }
    }
    for a in &t.assignments {
        collect_paths(&a.value, &mut out);
    }
    out
}

fn collect_paths(e: &Expr, out: &mut HashSet<String>) {
    match e {
        Expr::Path { path } if path.len() == 1 => {
            out.insert(path[0].clone());
        }
        Expr::Unary { expr, .. } => collect_paths(expr, out),
        Expr::Binary { left, right, .. } => {
            collect_paths(left, out);
            collect_paths(right, out);
        }
        _ => {}
    }
}

/// Walk a formula and mark fields constrained by `input_is` patterns.
fn collect_input_is_refs(f: &Formula, inputs: &[VariantInfo], refd: &mut [HashSet<usize>]) {
    match f {
        Formula::InputIs { pattern } => {
            if let Some(PatternVariant::Struct { name, fields }) = &pattern.variant {
                if let Some(vi) = inputs
                    .iter()
                    .position(|v| v.group == pattern.group && v.variant == *name)
                {
                    for (i, pf) in fields.iter().enumerate() {
                        let constraining = matches!(
                            pf.pattern,
                            PatternExpr::Int { .. }
                                | PatternExpr::Bool { .. }
                                | PatternExpr::EnumVariant { .. }
                        );
                        if constraining && i < inputs[vi].fields.len() {
                            refd[vi].insert(i);
                        }
                    }
                }
            }
        }
        Formula::Not { inner }
        | Formula::Next { inner }
        | Formula::Always { inner }
        | Formula::BoundedAlways { inner, .. } => collect_input_is_refs(inner, inputs, refd),
        Formula::BoundedEventually { inner, .. } => collect_input_is_refs(inner, inputs, refd),
        Formula::And { left, right }
        | Formula::Or { left, right }
        | Formula::Implies { left, right } => {
            collect_input_is_refs(left, inputs, refd);
            collect_input_is_refs(right, inputs, refd);
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Monitor state layout
// ---------------------------------------------------------------------------

/// Monitor layouts for the MC binary: properties (declaration order) then
/// SELECTED assumptions (declaration order) - the aggregate stepping order.
/// Formulas outside the monitor subset are skipped (no monitor, no state);
/// backends emit the identical skip warning themselves.
fn monitor_layouts(module: &Module, assumptions: &AssumptionSelection) -> Vec<MonitorStateLayout> {
    let mut out = Vec::new();
    for p in &module.properties {
        if let Ok(plan) = compile_formula(&p.name, false, &p.formula) {
            out.push(layout_for(&plan));
        }
    }
    for a in &module.assumptions {
        let selected = matches!(assumptions, AssumptionSelection::All)
            || matches!(assumptions, AssumptionSelection::Labels(l) if l.iter().any(|x| x == &a.name));
        if !selected {
            continue;
        }
        if let Ok(plan) = compile_formula(&a.name, true, &a.formula) {
            out.push(layout_for(&plan));
        }
    }
    out
}

fn layout_for(plan: &MonitorPlan) -> MonitorStateLayout {
    let mut nodes = Vec::new();
    let mut next_id = 0usize;
    let mut offset = 0u32;
    walk_node(&plan.root, &mut next_id, &mut offset, &mut nodes);
    MonitorStateLayout {
        label: plan.label.clone(),
        is_assumption: plan.is_assumption,
        offset_bits: 0,
        total_bits: offset,
        nodes,
    }
}

/// Preorder walk assigning node ids (matching the backends' `n{id}` naming)
/// and recording the persistent state of each node.
fn walk_node(
    node: &MonNode,
    next_id: &mut usize,
    offset: &mut u32,
    out: &mut Vec<MonitorNodeState>,
) {
    let id = *next_id;
    *next_id += 1;
    match node {
        MonNode::Obligation { bound, .. } if *bound >= 1 => {
            // pending flag + remaining counter (values 0..=bound).
            let remaining_bits = ceil_log2(bound.saturating_add(1));
            out.push(MonitorNodeState {
                node_id: id,
                offset_bits: *offset,
                pending_bits: 1,
                remaining_bits,
                antecedent_bits: 0,
            });
            *offset += 1 + remaining_bits;
        }
        MonNode::DelayedCheck { .. } => {
            // one-bit antecedent register.
            out.push(MonitorNodeState {
                node_id: id,
                offset_bits: *offset,
                pending_bits: 0,
                remaining_bits: 0,
                antecedent_bits: 1,
            });
            *offset += 1;
        }
        // RangeCheck / And / Or / Obligation{bound == 0}: no persistent state.
        _ => {}
    }
    if let MonNode::And(children) | MonNode::Or(children) = node {
        for c in children {
            walk_node(c, next_id, offset, out);
        }
    }
}

// ---------------------------------------------------------------------------
// Configuration encoding + traversal spec
// ---------------------------------------------------------------------------

fn config_encoding(module: &Module, monitors: &mut [MonitorStateLayout]) -> ConfigEncoding {
    let state_count = module
        .enums
        .iter()
        .find(|e| e.name == "State")
        .map(|e| e.variants.len())
        .unwrap_or(0);
    let state_bits = ceil_log2(state_count as u64);

    let mut var_slots = Vec::new();
    let mut offset = state_bits;
    for v in &module.vars {
        let width = type_bits(&v.ty, module);
        var_slots.push(VarSlot {
            name: v.name.clone(),
            offset_bits: offset,
            width_bits: width,
        });
        offset += width;
    }
    let vars_bits = offset - state_bits;
    let mut mon_offset = state_bits + vars_bits;
    for m in monitors.iter_mut() {
        m.offset_bits = mon_offset;
        mon_offset += m.total_bits;
    }
    let monitors_bits: u32 = monitors.iter().map(|m| m.total_bits).sum();
    let total_bits = state_bits + vars_bits + monitors_bits;
    let total_bytes = (total_bits as usize).div_ceil(8);
    ConfigEncoding {
        state_count,
        state_bits,
        var_slots,
        vars_bits,
        monitors: monitors.to_vec(),
        monitors_bits,
        total_bits,
        total_bytes,
    }
}

fn traversal_spec(module: &Module, abstraction: &InputAbstraction) -> TraversalSpec {
    let value_domains = abstraction
        .variants
        .iter()
        .map(|vs| vs.fields.iter().map(|f| f.domain.clone()).collect())
        .collect();

    let state_names: Vec<String> = module
        .enums
        .iter()
        .find(|e| e.name == "State")
        .map(|e| e.variants.clone())
        .unwrap_or_default();
    let state_orders = state_names
        .iter()
        .map(|s| {
            // Effective priority: `priority N` descending, declaration order
            // for ties (stable sort) - the firing order of the generated FSM.
            let mut ts: Vec<&Transition> = module
                .transitions
                .iter()
                .filter(|t| t.state == *s)
                .collect();
            ts.sort_by_key(|t| std::cmp::Reverse(t.priority.unwrap_or(0)));
            StateTransitionOrder {
                state: s.clone(),
                transitions: ts.iter().map(|t| t.id).collect(),
            }
        })
        .collect();

    let assumption_labels = module.assumptions.iter().map(|a| a.name.clone()).collect();
    TraversalSpec {
        value_domains,
        state_orders,
        assumption_labels,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_source;

    fn module(src: &str) -> Module {
        parse_source(src)
            .expect("parse")
            .modules
            .pop()
            .expect("module")
    }

    /// Spec exercising every reference kind on variant `V` of group `E`:
    /// - `x`: used in a guard (concrete);
    /// - `y`: used in an emit argument (concrete);
    /// - `z`: used in an assignment value (concrete);
    /// - `w`: constrained by a property `input_is` literal (concrete);
    /// - `u`: constrained by an assumption `input_is` literal (concrete);
    /// - `free`: never referenced (abstracted).
    const REF_SPEC: &str = "\
        module m {
          enum State { A, B }
          initial A
          event E(V(x: bit<8>, y: bool, z: bit<4>, w: bit<2>, u: bit<2>, free: bit<3>))
          output O(P(y: bool))
          var counter: bit<4> init 0
          transition A on E(V(x, y, z, _, _, _)) where x > 0 {
            next B;
            emit O(P(y));
            set counter = z;
          }
          prop uses_w : always (input_is(E(V(_, _, _, 1, _, _)))) ;
          assume env_u : always (!input_is(E(V(_, _, _, _, 0, _)))) ;
        }";

    fn field<'a>(plan: &'a McPlan, variant: &str, name: &str) -> &'a FieldSpec {
        let vs = plan
            .abstraction
            .variants
            .iter()
            .find(|v| v.variant == variant)
            .expect("variant");
        vs.fields.iter().find(|f| f.name == name).expect("field")
    }

    #[test]
    fn each_reference_kind_keeps_field_concrete() {
        let plan = McPlan::compute(&module(REF_SPEC), &AssumptionSelection::All);
        assert!(field(&plan, "V", "x").concrete, "guard reference");
        assert!(field(&plan, "V", "y").concrete, "emit-arg reference");
        assert!(field(&plan, "V", "z").concrete, "assignment reference");
        assert!(field(&plan, "V", "w").concrete, "property input_is literal");
        assert!(
            field(&plan, "V", "u").concrete,
            "assumption input_is literal"
        );
    }

    #[test]
    fn unreferenced_field_is_abstracted_with_representative() {
        let plan = McPlan::compute(&module(REF_SPEC), &AssumptionSelection::All);
        let f = field(&plan, "V", "free");
        assert!(!f.concrete);
        assert_eq!(f.representative, Some(RepValue::Int(0)));
        assert!(f.domain.is_none());
        // Concrete fields carry their canonical domain instead.
        let x = field(&plan, "V", "x");
        assert_eq!(x.domain, Some(ValueDomain::Int { max: 255 }));
        assert!(x.representative.is_none());
    }

    #[test]
    fn bound_but_unused_field_is_abstracted() {
        // Binding a field does NOT reference it; only USING the binding does.
        let m = module(
            "module m { enum State { A } initial A event E(V(x: bit<8>)) \
             transition A on E(V(x)) { next A; } }",
        );
        let plan = McPlan::compute(&m, &AssumptionSelection::All);
        assert!(!field(&plan, "V", "x").concrete);
    }

    #[test]
    fn wildcard_and_bind_in_input_is_do_not_reference() {
        // `input_is` with wildcards/binds constrains nothing.
        let m = module(
            "module m { enum State { A } initial A event E(V(x: bit<8>)) \
             prop p : always (input_is(E(V(_)))) ; }",
        );
        let plan = McPlan::compute(&m, &AssumptionSelection::All);
        assert!(!field(&plan, "V", "x").concrete);
    }

    #[test]
    fn encoding_layout_is_deterministic_and_exact() {
        let m = module(REF_SPEC);
        let a = McPlan::compute(&m, &AssumptionSelection::All);
        let b = McPlan::compute(&m, &AssumptionSelection::All);
        assert_eq!(a, b, "plan must be byte-deterministic for a fixed IR");

        let e = &a.encoding;
        // 2 states -> 1 bit; counter bit<4> -> 4 bits.
        assert_eq!(e.state_count, 2);
        assert_eq!(e.state_bits, 1);
        assert_eq!(e.vars_bits, 4);
        assert_eq!(e.var_slots[0].name, "counter");
        // Var slots start after the state index (1 bit).
        assert_eq!(e.var_slots[0].offset_bits, 1);
        assert_eq!(e.var_slots[0].width_bits, 4);

        // Monitors: `uses_w` = always(input_is(...)) and `env_u` =
        // always(!input_is(...)) are both pure per-sample RangeChecks -> no
        // persistent state (0 bits each).
        assert_eq!(e.monitors.len(), 2);
        // Monitor blocks start after state + vars (1 + 4 = 5 bits).
        assert_eq!(e.monitors[0].offset_bits, 5);
        assert_eq!(e.monitors[0].label, "uses_w");
        assert!(!e.monitors[0].is_assumption);
        assert_eq!(e.monitors[0].total_bits, 0);
        assert!(e.monitors[0].nodes.is_empty());
        assert_eq!(e.monitors[1].label, "env_u");
        assert!(e.monitors[1].is_assumption);
        assert_eq!(e.monitors[1].total_bits, 0);

        // total = 1 (state) + 4 (vars) + 0 (monitors) = 5 bits -> 1 byte.
        assert_eq!(e.total_bits, 5);
        assert_eq!(e.total_bytes, 1);
    }

    #[test]
    fn obligation_and_delayedcheck_state_bits() {
        let m = module(
            "module m { enum State { A } initial A event E \
             prop o : always (input_is(E) -> bounded_eventually[5] (state == A)) ; \
             prop d : always (input_is(E) -> next (state == A)) ; }",
        );
        let plan = McPlan::compute(&m, &AssumptionSelection::All);
        // o: `p -> bounded_eventually[5] q` compiles to a single root
        // Obligation node (id 0): pending 1 + remaining ceil(log2(6)) = 3 bits.
        assert_eq!(plan.encoding.monitors[0].total_bits, 4);
        let nodes = &plan.encoding.monitors[0].nodes;
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].node_id, 0);
        assert_eq!(nodes[0].pending_bits, 1);
        assert_eq!(nodes[0].remaining_bits, 3);
        // d: `p -> next q` compiles to a single root DelayedCheck (id 0):
        // antecedent register = 1 bit.
        assert_eq!(plan.encoding.monitors[1].total_bits, 1);
        let nodes = &plan.encoding.monitors[1].nodes;
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].node_id, 0);
        assert_eq!(nodes[0].antecedent_bits, 1);
    }

    #[test]
    fn assumption_selection_affects_encoding() {
        let m = module(REF_SPEC);
        let all = McPlan::compute(&m, &AssumptionSelection::All);
        let none = McPlan::compute(&m, &AssumptionSelection::None);
        assert_eq!(all.encoding.monitors.len(), 2);
        assert_eq!(none.encoding.monitors.len(), 1, "no assumption monitors");
        let labels = McPlan::compute(&m, &AssumptionSelection::Labels(vec!["env_u".into()]));
        assert_eq!(labels.encoding.monitors.len(), 2);
    }

    #[test]
    fn canonical_value_domains_and_order() {
        // All three fields are referenced by the guard, so all are concrete.
        let m = module(
            "module m { enum State { A } initial A \
             enum Mode { M1, M2, M3 } \
             event E(V(b: bool, n: bit<4>, mode: Mode)) \
             event F \
             transition A on E(V(b, n, mode)) where b && n > 0 && mode == Mode::M1 { next A; } }",
        );
        let plan = McPlan::compute(&m, &AssumptionSelection::All);
        // Variant order: declaration order (E.V before F).
        assert_eq!(plan.abstraction.variants[0].variant, "V");
        assert_eq!(plan.abstraction.variants[1].variant, "F");
        let domains = &plan.traversal.value_domains[0];
        assert_eq!(domains[0], Some(ValueDomain::Bool));
        assert_eq!(domains[1], Some(ValueDomain::Int { max: 15 }));
        assert_eq!(
            domains[2],
            Some(ValueDomain::Enum {
                variants: vec!["M1".into(), "M2".into(), "M3".into()]
            })
        );
        // Unit variant has no fields.
        assert!(plan.traversal.value_domains[1].is_empty());
    }

    #[test]
    fn state_transition_firing_order() {
        let m = module(
            "module m { enum State { A, B } initial A event E \
             transition A on E { next B; } \
             transition A on E where true { next A; priority 5; } \
             transition B on E { next A; } }",
        );
        let plan = McPlan::compute(&m, &AssumptionSelection::All);
        // State order: declaration order (A then B).
        assert_eq!(plan.traversal.state_orders[0].state, "A");
        // Priority 5 fires first; the default-priority transition second.
        let ids = &plan.traversal.state_orders[0].transitions;
        assert_eq!(ids.len(), 2);
        assert_eq!(ids[0], 1);
        assert_eq!(ids[1], 0);
        assert_eq!(plan.traversal.state_orders[1].transitions, vec![2]);
    }

    #[test]
    fn assumption_labels_in_declaration_order() {
        let m = module(REF_SPEC);
        let plan = McPlan::compute(&m, &AssumptionSelection::All);
        assert_eq!(plan.traversal.assumption_labels, vec!["env_u"]);
    }
}
