//! Shared SMT bounded-mode spec (task 11): a backend-neutral encoding of a
//! module for k-step unrolling over z3 bitvectors. Both backends render this
//! SAME structure, so the constraint construction is identical in Rust and C
//! generated binaries (parity by construction, D-P4-12).
//!
//! v1 scope (documented limitations, rejected at codegen with shared errors):
//! - assumptions and on_error blocks ARE supported (task 3C): every assumption
//!   is encoded as a per-sample base constraint (exact-mode-faithful successor
//!   pruning, SEMANTICS 7.x) and the on_error block as a catch-all lowest-
//!   priority transition from any state (SEMANTICS 3.6); expensive assumption
//!   forms (top-level eventually/bounded_eventually/until, nested temporals)
//!   are carried but asserted only with `--full-assumptions` (omitted with a
//!   per-assumption warning otherwise);
//! - properties: top-level `always` / `always[N]` / `eventually` bodies may use
//!   expr/not/and/or/implies plus `input_is`, `output_has` and `next` (the
//!   DEFAULT class - checked without any flag); top-level `bounded_eventually[N]`
//!   / `until` and any temporal operator nested under another temporal operator
//!   form the FULL class - converted and carried in the spec, but checked only
//!   when the engine runs with `--full-properties` (omitted with a per-property
//!   warning otherwise);
//! - variables: bit<N>, bool and enum-typed (task 3D) - an enum value is its
//!   variant's DECLARATION-ORDER INDEX in a `bit<ceil_log2(N)>` word, exactly
//!   as exact mode encodes it (`ref_mc_exact.rs`); witness display prints the
//!   variant NAME, not the index;
//! - concrete payload fields: bit<N>/bool with N <= 64, and enum-typed (task
//!   3D) with the same index encoding; abstracted enum fields fold to their
//!   representative's index (the first declared variant);
//! - transitions: a bare group pattern fires on ANY of the group's variants
//!   (`SmtTrans.classes` carries them all, task 3D); such a transition has no
//!   payload bindings (validator rule), so its guard/assignments/emits convert
//!   identically for every class.
//!
//! Docs: dev:mc-engine-overview, dev:mc-engine-smt, dev:algorithms-mc-engine, llm:mc-engine

use crate::lang::ir::{Expr, Formula, Module};
use crate::mc::plan::{McPlan, ValueDomain};

/// Expression node over (state, vars, payload fields) with references already
/// resolved to indices. Mirrors the IR expression language.
#[derive(Debug, Clone)]
pub enum Se {
    /// Literal. `is_bool` distinguishes a bool literal (z3 Bool in boolean
    /// contexts) from an integer literal (IntLit, lifted by parent context).
    Lit {
        value: u64,
        is_bool: bool,
    },
    /// Current state (BV of `state_bits`).
    State,
    /// Variable index into [`SmtSpec::vars`].
    Var(usize),
    /// Concrete payload field: index into the CURRENT input class's concrete
    /// fields (see [`SmtTrans::class`]), with the field's bit width (1 for
    /// bool) and bool-ness captured at conversion time - both are
    /// class-relative, so they cannot be recovered from the spec alone.
    Field {
        idx: usize,
        width: u32,
        is_bool: bool,
    },
    Not(Box<Se>),
    BitNot(Box<Se>),
    Neg(Box<Se>),
    Bin(Sb, Box<Se>, Box<Se>),
}

/// Binary operators (subset of the IR `BinOp`, same names).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sb {
    Plus,
    Minus,
    Star,
    ShiftLeft,
    ShiftRight,
    Eq,
    NotEq,
    Lt,
    Le,
    Gt,
    Ge,
    BitOr,
    BitXor,
    BitAnd,
    LogicOr,
    LogicAnd,
}

/// Property formula (boolean layer) over configuration expressions.
#[derive(Debug, Clone)]
pub enum Sf {
    Se(Se),
    Not(Box<Sf>),
    And(Box<Sf>, Box<Sf>),
    Or(Box<Sf>, Box<Sf>),
    Implies(Box<Sf>, Box<Sf>),
    /// input_is(Pattern): the input consumed at this step matches the pattern.
    /// Pre-resolved into arms, one per matching input class: (class index,
    /// literal field constraints as (GLOBAL field index, value, is_bool)). An
    /// empty arm list means no class can match - the atom is always false. The
    /// engine evaluates it over the step's fire predicates (the fired
    /// transition's class IS the consumed input), so it is automatically false
    /// at the final unrolled step (no input there). `id` indexes
    /// [`SmtSpec::input_atoms`] (assigned in formula walk order).
    InputIs {
        id: usize,
        arms: Vec<(usize, Vec<(usize, u64, bool)>)>,
    },
    /// output_has(Pattern): at least one output emitted by the transition
    /// firing at this step matches. `constraints` are positionally aligned with
    /// the matched output declaration's fields: (field position, value,
    /// is_bool); wildcard and bind pattern fields constrain nothing. Evaluated
    /// over the step's fire predicates (false at the final unrolled step).
    /// `id` indexes [`SmtSpec::output_atoms`].
    OutputHas {
        id: usize,
        group: String,
        variant: Option<String>,
        constraints: Vec<(usize, u64, bool)>,
    },
    /// next(f): f holds one step later. The engine evaluates the body at step
    /// i+1; properties containing `next` are checked only at steps where the
    /// shifted horizon exists (trailing anchors dropped - sound, no false
    /// FAILs).
    Next(Box<Sf>),
    /// bounded_eventually[N](f) - FULL class (checked only with
    /// --full-properties): f must hold within N steps of the anchor.
    BoundedEventually {
        bound: usize,
        inner: Box<Sf>,
    },
    /// p until q - FULL class (checked only with --full-properties): weak
    /// until.
    Until {
        left: Box<Sf>,
        right: Box<Sf>,
    },
    /// always[N](f) / eventually(f) nested under another temporal operator -
    /// FULL class (checked only with --full-properties). Top-level always[N]
    /// is the DEFAULT class and is carried as [`SmtPropKind::Always`] with a
    /// bound, not as this node.
    BoundedAlways {
        bound: usize,
        inner: Box<Sf>,
    },
    Eventually(Box<Sf>),
}

#[derive(Debug, Clone)]
pub struct SmtVar {
    pub name: String,
    /// Bit width (1 for bool vars; `ceil_log2(variant_count)` for enum vars).
    pub width: u32,
    /// True when the IR type is `bool` (vs `bit<1>`) - witness display only
    /// (true/false vs 0/1); both encode as 1-bit z3 values.
    pub is_bool: bool,
    /// Initial value (literal inits only; an enum var's init is its variant's
    /// declaration-order index).
    pub init: u64,
    /// True when the IR type is an enum (task 3D): the value is a variant
    /// index, and witness display prints `enum_variants[value]`, not the raw
    /// index.
    pub is_enum: bool,
    /// The enum's variants in declaration order (empty for non-enum vars) -
    /// witness display only.
    pub enum_variants: Vec<String>,
}

/// One canonical input class = one event variant. Concrete fields become free
/// z3 variables per unrolled step; abstracted fields are substituted as their
/// representative literal during expression conversion.
#[derive(Debug, Clone)]
pub struct SmtClass {
    /// Display name for counterexample output (the plan's unique_name).
    pub name: String,
    /// Concrete fields only: (name, bit width; 1 for bool, is_bool - witness
    /// display only: true/false vs 0/1, enum_variants - the field's enum
    /// variants in declaration order, empty for non-enum fields; an enum
    /// field's value is a variant index, task 3D).
    pub fields: Vec<(String, u32, bool, Vec<String>)>,
}

/// One output emitted by a transition: group + variant name + argument
/// expressions converted in the transition's context (positionally aligned
/// with the output declaration's fields). Carried so `output_has` properties
/// can be evaluated over the step's fire predicates.
#[derive(Debug, Clone)]
pub struct SmtEmit {
    pub group: String,
    pub variant: Option<String>,
    pub args: Vec<Se>,
}

/// The module's `on_error` fallback (SEMANTICS 3.6): a catch-all, lowest-
/// priority transition from ANY state - no pattern bindings, no variable
/// assignments (all vars carry over unchanged). Encoded as a dedicated field
/// (NOT a [`SmtTrans`] with a fake state index) so the three engines and both
/// emitters handle it uniformly and the real-transition firing order is
/// untouched. `on_error_fire(i) = NOT any_fire_real(i)`; its emits feed the
/// step's output_has atoms exactly like a real transition's emits.
#[derive(Debug, Clone)]
pub struct SmtOnError {
    pub next_state: usize,
    pub emits: Vec<SmtEmit>,
}

#[derive(Debug, Clone)]
pub struct SmtTrans {
    /// State index into [`SmtSpec::states`] this transition fires from.
    pub state: usize,
    pub next_state: usize,
    /// Input classes (event variant) this transition fires on (task 3D):
    /// exactly one for a named-variant pattern, ALL of the group's variants
    /// for a bare group pattern (multi-variant wildcard). The engine renders
    /// `enabled_t(i)` as the OR over these classes of (state-match AND
    /// `ev_i == c` AND guard).
    pub classes: Vec<usize>,
    /// Guard over (state, vars, payload fields); None = always enabled. A
    /// bare-group-pattern transition carries no payload bindings (validator
    /// rule - `validate_event_pattern` introduces none), so ONE conversion
    /// serves every class in `classes`.
    pub guard: Option<Se>,
    /// Variable updates: (var index, expression). Vars absent here are
    /// unchanged by the transition.
    pub assignments: Vec<(usize, Se)>,
    /// Outputs emitted when this transition fires (see [`SmtEmit`]).
    pub emits: Vec<SmtEmit>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmtPropKind {
    Always,
    Eventually,
    /// Top-level bounded_eventually[N] - FULL class.
    BoundedEventually(usize),
    /// Top-level until - FULL class.
    Until,
}

#[derive(Debug, Clone)]
pub struct SmtProp {
    pub name: String,
    pub kind: SmtPropKind,
    /// Anchor-range clip for top-level `always[N]` (the DEFAULT class): the
    /// property is checked at steps 0..min(N, k). None = unbounded always.
    pub bound: Option<usize>,
    /// FULL class: top-level bounded_eventually/until, or a temporal operator
    /// nested under another temporal operator. Checked only when the engine
    /// runs with `--full-properties`; omitted with a per-property warning
    /// otherwise.
    pub full: bool,
    /// Maximum nesting depth of `next` in the formula (0 = none). Properties
    /// are checked only at steps i with i + depth < k (the shifted horizon
    /// must exist - dropping trailing anchors is sound, no false FAILs).
    pub next_depth: usize,
    pub formula: Sf,
}

/// One assumption converted for per-sample base-constraint assertion
/// (task 3C, D3). Exact mode compiles each selected assumption to a monitor
/// and prunes any successor whose input violates it (SEMANTICS 7.5); the SMT
/// encoding expresses the same pruning as per-step constraints: only inputs
/// satisfying every active assumption are allowed.
#[derive(Debug, Clone)]
pub struct SmtAssumption {
    pub name: String,
    /// The converted formula with any top-level `always`/`always[N]` DROPPED
    /// (the per-sample assertion loop IS the universal quantification over
    /// samples). Other top forms are carried as their [`Sf`] nodes
    /// (`Eventually` / `BoundedEventually` / `Until`) and asserted at every
    /// sample from which their horizon-truncated expansion is evaluated.
    pub formula: Sf,
    /// `always[N]` clip: assert only at samples 0..N. None = all samples.
    pub bound: Option<usize>,
    /// Deepest `next` nesting in the body: the per-sample assertion is
    /// evaluated at sample i + depth, so anchors within `next_depth` of the
    /// horizon end have no shifted horizon and are dropped (same clipping as
    /// properties - without it a `next`-bearing assumption would read
    /// `alive[k]` out of bounds at the last sample).
    pub next_depth: usize,
    /// FULL class: top-level eventually/bounded_eventually/until, or a
    /// temporal operator nested under another (O(k*N) formula growth). Asserted
    /// only when the engine runs with `--full-assumptions`; omitted with a
    /// per-assumption warning otherwise. A top-level `always`/`always[N]`
    /// input-guard body is DEFAULT.
    pub full: bool,
}

/// The complete backend-neutral SMT spec.
#[derive(Debug, Clone)]
pub struct SmtSpec {
    /// State names in declaration order (the module's `State` enum).
    pub states: Vec<String>,
    pub initial_state: usize,
    /// Bit width of the state encoding: max(1, ceil_log2(states.len())).
    pub state_bits: u32,
    pub vars: Vec<SmtVar>,
    pub classes: Vec<SmtClass>,
    /// Transitions in FIRING order (priority desc, declaration order for ties)
    /// - the same order the exact engine uses for first-match selection.
    pub transitions: Vec<SmtTrans>,
    pub properties: Vec<SmtProp>,
    /// The module's on_error fallback (None = an unmatched input kills the
    /// path, as before task 3C).
    pub on_error: Option<SmtOnError>,
    /// ALL of the module's assumptions, in declaration order (selection via
    /// `--assumes` happens at RUNTIME in every engine - the same pattern exact
    /// mode uses, where generated binaries carry every assumption's monitor
    /// and activate a subset per run).
    pub assumptions: Vec<SmtAssumption>,
    /// input_is atom definitions, indexed by `Sf::InputIs.id` (formula walk
    /// order over all properties, then all assumptions). Each: (class index,
    /// literal field constraints as (GLOBAL field index, value, is_bool)).
    pub input_atoms: Vec<Vec<(usize, Vec<(usize, u64, bool)>)>>,
    /// output_has atom definitions, indexed by `Sf::OutputHas.id`.
    pub output_atoms: Vec<(String, Option<String>, Vec<(usize, u64, bool)>)>,
}

/// Context for converting one expression: name resolution follows the
/// validator's order exactly (state -> payload bindings -> vars -> state
/// variants).
struct ExprCtx {
    var_idx: std::collections::HashMap<String, usize>,
    /// Renamed event parameter: pattern binding name -> declared payload field
    /// name (patterns match positionally, exactly as the exact-mode engines
    /// do). Resolved at the top of expression name lookup, so the width/domain
    /// lookup below still uses the declared name.
    bind_alias: std::collections::HashMap<String, String>,
    /// Payload field name -> Some(idx into the class's concrete fields) or
    /// None (abstracted; the caller substitutes the representative literal).
    field: std::collections::HashMap<String, Option<usize>>,
    state_variant_idx: std::collections::HashMap<String, usize>,
    /// Enum name -> variants in declaration order (task 3D): resolves an enum
    /// path literal (`Color::Red`) to the variant's index, exactly as exact
    /// mode does at configuration build time.
    enums: std::collections::HashMap<String, Vec<String>>,
    /// Input-class view for input_is resolution: (class index, group, variant
    /// name, per-declared-field concrete GLOBAL field index or None, per-
    /// declared-field representative value as u64 or None). Empty outside
    /// property conversion.
    input_classes: Vec<(usize, String, String, Vec<Option<usize>>, Vec<Option<u64>>)>,
}

/// ceil(log2(n)), minimum 1 (bit width of an index space of size n).
pub fn ceil_log2_pub(n: usize) -> u32 {
    if n <= 1 {
        return 1;
    }
    usize::BITS - (n - 1).leading_zeros()
}

fn ceil_log2(n: usize) -> u32 {
    ceil_log2_pub(n)
}

/// Build the shared SMT spec from a validated module + its MC plan. Returns a
/// shared error string for unsupported constructs (identical in both
/// backends, so codegen behavior is parity-safe).
pub fn build_smt_spec(module: &Module, plan: &McPlan) -> Result<SmtSpec, String> {
    // States: the module's `State` enum, declaration order.
    let state_enum = module
        .enums
        .iter()
        .find(|e| e.name == "State")
        .ok_or("module has no State enum")?;
    let states: Vec<String> = state_enum.variants.clone();
    let initial_state = match &module.initial_state {
        Some(name) => states
            .iter()
            .position(|s| s == name)
            .ok_or_else(|| format!("unknown initial state '{name}'"))?,
        None => 0,
    };

    // Enum table (task 3D): name -> variants in declaration order. An enum
    // value encodes as its variant's DECLARATION-ORDER INDEX - the same
    // encoding exact mode uses at configuration build time
    // (`ref_mc_exact.rs`: `Type::Enum { name } => ceil_log2(variant_count)`,
    // `Val::Enum(name)` replaced by the variant index), so a spec that PASSes
    // under exact mode encodes to the SAME boolean facts under smt-bounded.
    let enums: std::collections::HashMap<String, Vec<String>> = module
        .enums
        .iter()
        .map(|e| (e.name.clone(), e.variants.clone()))
        .collect();

    // Vars: bit<N>/bool/enum (task 3D).
    let mut vars: Vec<SmtVar> = Vec::new();
    let mut var_idx: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for v in &module.vars {
        let is_bool = matches!(v.ty, crate::lang::ir::Type::Bool);
        let (is_enum, enum_variants) = match &v.ty {
            crate::lang::ir::Type::Enum { name } => {
                let variants = enums.get(name).cloned().ok_or_else(|| {
                    format!("var '{}' references unknown enum '{}'", v.name, name)
                })?;
                (true, variants)
            }
            _ => (false, Vec::new()),
        };
        let width = if is_enum {
            ceil_log2(enum_variants.len())
        } else {
            match &v.ty {
                crate::lang::ir::Type::Bool => 1u32,
                crate::lang::ir::Type::Bit { width } => *width,
                _ => unreachable!("enum handled above"),
            }
        };
        // Literal inits only (v1): no var/state references. An enum var's init
        // is a variant path - `Color::Red` (qualified) or, for a State-typed
        // var, the bare state-variant name (both type-check per resolve_path)
        // - and encodes as the variant's declaration-order index.
        let init = match &v.init {
            Expr::Int { value } => *value,
            Expr::Bool { value } => u64::from(*value),
            Expr::Path { path } if is_enum => {
                let (enum_name, variant) = if path.len() == 1 {
                    // Bare name: legal only for a State-typed var (a state
                    // variant); the enum is the var's own type.
                    (
                        match &v.ty {
                            crate::lang::ir::Type::Enum { name } => name.clone(),
                            _ => unreachable!("is_enum"),
                        },
                        path[0].clone(),
                    )
                } else {
                    (path[0].clone(), path[1].clone())
                };
                let variants = enums.get(&enum_name).unwrap_or(&enum_variants);
                variants.iter().position(|s| s == &variant).ok_or_else(|| {
                    format!(
                        "initial value of var '{}' references unknown variant '{}'",
                        v.name, variant
                    )
                })? as u64
            }
            other => {
                return Err(format!(
                    "smt-bounded mode requires literal initial values (var '{}' has {:?})",
                    v.name, other
                ))
            }
        };
        if init >= (1u64 << width.min(64)) {
            return Err(format!(
                "initial value of var '{}' does not fit in {} bits",
                v.name, width
            ));
        }
        var_idx.insert(v.name.clone(), vars.len());
        vars.push(SmtVar {
            name: v.name.clone(),
            width,
            is_bool,
            init,
            is_enum,
            enum_variants,
        });
    }

    // Input classes from the plan's abstraction table.
    let mut classes: Vec<SmtClass> = Vec::new();
    for vi in &plan.abstraction.variants {
        let mut fields: Vec<(String, u32, bool, Vec<String>)> = Vec::new();
        for f in &vi.fields {
            if !f.concrete {
                continue; // abstracted: substituted as a literal during conversion
            }
            let is_bool = matches!(f.domain, Some(ValueDomain::Bool));
            let (width, enum_variants) = match &f.domain {
                Some(ValueDomain::Bool) => (1u32, Vec::new()),
                Some(ValueDomain::Int { max }) => {
                    if *max == u64::MAX {
                        return Err(format!(
                            "smt-bounded mode does not support concrete payload fields wider than 64 bits (field '{}')",
                            f.name
                        ));
                    }
                    // Int domains are always 2^N-1 by construction (plan.rs::value_domain
                    // emits them only from bit<N> types), so no power-of-two test is
                    // needed - the dead check was removed in task 3F (V1_R_R_AUDIT.md S3).
                    let w = ceil_log2((*max as usize) + 1);
                    (w, Vec::new())
                }
                // Enum-typed concrete field (task 3D): the value is the
                // variant's declaration-order index in ceil_log2(N) bits.
                Some(ValueDomain::Enum { variants, .. }) => {
                    (ceil_log2(variants.len()), variants.clone())
                }
                None => unreachable!("concrete field has a domain"),
            };
            fields.push((f.name.clone(), width, is_bool, enum_variants));
        }
        classes.push(SmtClass {
            name: vi.unique_name.clone(),
            fields,
        });
    }

    // State-variant names for expression resolution.
    let mut state_variant_idx: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    for (i, s) in states.iter().enumerate() {
        state_variant_idx.insert(s.clone(), i);
    }

    // Input-class view for input_is resolution: per class, the group, variant
    // name, each declared field's concrete GLOBAL index (None if abstracted)
    // and its representative value (bools as 0/1).
    let mut input_classes: Vec<(usize, String, String, Vec<Option<usize>>, Vec<Option<u64>>)> =
        Vec::new();
    {
        let mut base = 0usize;
        for (ci, vi) in plan.abstraction.variants.iter().enumerate() {
            let mut concrete: Vec<Option<usize>> = Vec::new();
            let mut reps: Vec<Option<u64>> = Vec::new();
            for f in &vi.fields {
                if f.concrete {
                    concrete.push(Some(base));
                    base += 1;
                } else {
                    concrete.push(None);
                }
                reps.push(match &f.representative {
                    Some(crate::codegen::fsm::RepValue::Bool(b)) => Some(u64::from(*b)),
                    Some(crate::codegen::fsm::RepValue::Int(v)) => Some(*v),
                    // Abstracted enum field (task 3D): the representative is
                    // the FIRST declared variant - fold it to its index (0),
                    // exactly as concrete enum values encode.
                    Some(crate::codegen::fsm::RepValue::EnumVariant(path)) => {
                        f.domain.as_ref().and_then(|d| match d {
                            ValueDomain::Enum { variants, .. } => variants
                                .iter()
                                .position(|s| s == &path[path.len() - 1])
                                .map(|i| i as u64),
                            _ => None,
                        })
                    }
                    _ => None,
                });
            }
            input_classes.push((ci, vi.group.clone(), vi.variant.clone(), concrete, reps));
        }
    }

    // Transitions in firing order (priority desc, stable).
    let mut order: Vec<usize> = (0..module.transitions.len()).collect();
    order.sort_by_key(|&i| std::cmp::Reverse(module.transitions[i].priority.unwrap_or(0)));
    let mut transitions: Vec<SmtTrans> = Vec::new();
    for &ti in &order {
        let t = &module.transitions[ti];
        let state = states
            .iter()
            .position(|s| *s == t.state)
            .ok_or_else(|| format!("transition {} references unknown state '{}'", t.id, t.state))?;
        let next_state = states
            .iter()
            .position(|s| *s == t.next_state)
            .ok_or_else(|| {
                format!(
                    "transition {} references unknown next state '{}'",
                    t.id, t.next_state
                )
            })?;
        // Match the transition's pattern to its input classes (task 3D). A
        // bare group pattern (variant: None) fires on ANY of the group's
        // variants - all of them go into `classes` (the multi-variant
        // wildcard; exact mode's input_matches_variant returns true for every
        // variant of the group). A named-variant pattern yields exactly one.
        let t_classes: Vec<usize> = if t.pattern.variant.is_none() {
            plan.abstraction
                .variants
                .iter()
                .enumerate()
                .filter(|(_, v)| v.group == t.pattern.group)
                .map(|(i, _)| i)
                .collect()
        } else {
            vec![plan
                .abstraction
                .variants
                .iter()
                .position(|v| {
                    v.group == t.pattern.group
                        && variant_name(&t.pattern.variant).as_deref() == Some(v.variant.as_str())
                })
                .ok_or_else(|| format!("transition {} has no matching input class", t.id))?]
        };
        if t_classes.is_empty() {
            return Err(format!(
                "transition {} matches no input class (group '{}')",
                t.id, t.pattern.group
            ));
        }
        // The single-class case is the common one; multi-class transitions
        // carry no payload references (see below), so t_classes[0] is a valid
        // field-resolution context for every expression of this transition.
        let class = t_classes[0];

        // Conversion context for this transition's payload fields. Exact mode
        // introduces bindings ONLY for a struct pattern matching the input's
        // variant (pattern_bindings returns empty maps otherwise - bare group
        // patterns and unit variants), so the field map + bind aliases are
        // built only in that case: a bare-group-pattern guard/assignment/emit
        // can reference state, vars and literals only, exactly as the
        // validator's binding-free environment allows. Declared field names
        // resolve directly; the pattern's BINDING names (which may rename a
        // field, e.g. `R(k)` for declared field `rkey`) are aliased to the
        // declared name at the same POSITION - patterns match positionally,
        // exactly as the exact-mode engines do.
        let mut field: std::collections::HashMap<String, Option<usize>> =
            std::collections::HashMap::new();
        let mut bind_alias: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();
        if let Some(crate::lang::ir::PatternVariant::Struct { fields, .. }) = &t.pattern.variant {
            for f in &plan.abstraction.variants[class].fields {
                field.insert(
                    f.name.clone(),
                    if f.concrete {
                        Some(fi_concrete(&classes[class], &f.name))
                    } else {
                        None
                    },
                );
            }
            for (pos, pf) in fields.iter().enumerate() {
                if let crate::lang::ir::PatternExpr::Bind { name } = &pf.pattern {
                    if let Some(declared) = plan.abstraction.variants[class]
                        .fields
                        .get(pos)
                        .map(|f| f.name.clone())
                    {
                        bind_alias.insert(name.clone(), declared);
                    }
                }
            }
        }
        let ctx = ExprCtx {
            var_idx: var_idx.clone(),
            field,
            state_variant_idx: state_variant_idx.clone(),
            bind_alias,
            input_classes: Vec::new(),
            enums: enums.clone(),
        };

        let guard = match &t.guard {
            Some(g) => Some(convert_expr(g, &ctx, &plan.abstraction.variants[class])?),
            None => None,
        };
        let mut assignments: Vec<(usize, Se)> = Vec::new();
        for a in &t.assignments {
            let vi = var_idx
                .get(&a.var)
                .copied()
                .ok_or_else(|| format!("transition {} assigns unknown var '{}'", t.id, a.var))?;
            assignments.push((
                vi,
                convert_expr(&a.value, &ctx, &plan.abstraction.variants[class])?,
            ));
        }
        // Emitted outputs: argument expressions converted in this transition's
        // context (an enum-path argument resolves to the variant's index,
        // task 3D).
        let mut emits: Vec<SmtEmit> = Vec::new();
        for o in &t.emits {
            let args = o
                .args
                .iter()
                .map(|a| convert_expr(a, &ctx, &plan.abstraction.variants[class]))
                .collect::<Result<Vec<_>, _>>()?;
            emits.push(SmtEmit {
                group: o.group.clone(),
                variant: o.variant.clone(),
                args,
            });
        }
        transitions.push(SmtTrans {
            state,
            next_state,
            classes: t_classes,
            guard,
            assignments,
            emits,
        });
    }

    // on_error fallback (task 3C, D4): a catch-all lowest-priority transition
    // from ANY state - no pattern bindings, no variable assignments. Emit
    // argument expressions convert in a no-binding context (an enum-path
    // argument still hits the shared enum rejection until task 3D).
    let on_error = match &module.on_error {
        Some(oe) => {
            let next_state = states
                .iter()
                .position(|s| s == &oe.next_state)
                .ok_or_else(|| format!("on_error targets unknown state '{}'", oe.next_state))?;
            // No input class: an empty variant spec (field references in emit
            // arguments are impossible - on_error carries no pattern bindings).
            let empty_variant = crate::mc::plan::VariantInputSpec {
                group: String::new(),
                variant: String::new(),
                unique_name: String::new(),
                fields: Vec::new(),
            };
            let ctx = ExprCtx {
                var_idx: var_idx.clone(),
                field: std::collections::HashMap::new(),
                state_variant_idx: state_variant_idx.clone(),
                bind_alias: std::collections::HashMap::new(),
                input_classes: Vec::new(),
                enums: enums.clone(),
            };
            let emits = oe
                .emits
                .iter()
                .map(|o| -> Result<SmtEmit, String> {
                    let args = o
                        .args
                        .iter()
                        .map(|a| convert_expr(a, &ctx, &empty_variant))
                        .collect::<Result<Vec<_>, _>>()?;
                    Ok(SmtEmit {
                        group: o.group.clone(),
                        variant: o.variant.clone(),
                        args,
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;
            Some(SmtOnError { next_state, emits })
        }
        None => None,
    };

    // Properties: the top forms always, always[N] and eventually are the
    // DEFAULT class (checked without any flag); top-level
    // bounded_eventually/until and any
    // temporal operator nested under another temporal operator form the FULL
    // class (converted here, checked only with --full-properties). Task 3G: a
    // top form that is none of those five (a bare boolean formula, a top-level
    // next, ...) is desugared to always(...) BEFORE the kind match - exact mode
    // anchors such formulas at Range::all() (checked at every sample; a
    // top-level next shifts by one), so smt-bounded reproduces the same
    // verdicts with zero formula-size change for the common cases.
    let mut properties: Vec<SmtProp> = Vec::new();
    for p in &module.properties {
        // Task 3G normalization: the five accepted top forms pass through
        // unchanged (every existing pin stays byte-identical); anything else is
        // wrapped in Always, which the kind match below takes as a plain always
        // (bound None) and whose inner is converted as a nested node by
        // convert_formula.
        let top = match &p.formula {
            Formula::Always { .. }
            | Formula::BoundedAlways { .. }
            | Formula::Eventually { .. }
            | Formula::BoundedEventually { .. }
            | Formula::Until { .. } => p.formula.clone(),
            other => Formula::Always {
                inner: Box::new(other.clone()),
            },
        };
        let (kind, bound) = match &top {
            Formula::Always { .. } => (SmtPropKind::Always, None),
            Formula::BoundedAlways { bound, .. } => (SmtPropKind::Always, Some(*bound as usize)),
            Formula::Eventually { .. } => (SmtPropKind::Eventually, None),
            Formula::BoundedEventually { bound, .. } => {
                (SmtPropKind::BoundedEventually(*bound as usize), None)
            }
            Formula::Until { .. } => (SmtPropKind::Until, None),
            _ => unreachable!("top form normalized above"),
        };
        // Property context: no payload bindings (input_is/output_has atoms
        // carry their own pre-resolved data).
        let ctx = ExprCtx {
            var_idx: var_idx.clone(),
            field: std::collections::HashMap::new(),
            state_variant_idx: state_variant_idx.clone(),
            bind_alias: std::collections::HashMap::new(),
            input_classes: input_classes.clone(),
            enums: enums.clone(),
        };
        let formula = match &top {
            Formula::Always { inner }
            | Formula::BoundedAlways { inner, .. }
            | Formula::Eventually { inner }
            | Formula::BoundedEventually { inner, .. } => convert_formula(inner, &ctx)?,
            Formula::Until { left, right } => Sf::Until {
                left: Box::new(convert_formula(left, &ctx)?),
                right: Box::new(convert_formula(right, &ctx)?),
            },
            _ => unreachable!("top form matched above"),
        };
        properties.push(SmtProp {
            name: p.name.clone(),
            kind,
            bound,
            full: classify_full(&top),
            next_depth: next_depth(&top),
            formula,
        });
    }

    // Assumptions (task 3C, D3): every assumption of the module is converted
    // with the SAME convert_formula path as properties (input_is/output_has/
    // expr/guards all work) and carried for per-sample base-constraint
    // assertion. Selection (`--assumes all|none|labels`) happens at RUNTIME in
    // every engine - the same pattern exact mode uses, where generated
    // binaries carry every assumption's monitor and activate a subset per run.
    // Assumption context: no payload bindings (input_is/output_has atoms carry
    // their own pre-resolved data) - identical to the property context.
    let assume_ctx = ExprCtx {
        var_idx: var_idx.clone(),
        field: std::collections::HashMap::new(),
        state_variant_idx: state_variant_idx.clone(),
        bind_alias: std::collections::HashMap::new(),
        input_classes: input_classes.clone(),
        enums: enums.clone(),
    };
    let mut assumptions: Vec<SmtAssumption> = module
        .assumptions
        .iter()
        .map(|a| convert_assumption(a, &assume_ctx))
        .collect::<Result<Vec<_>, _>>()?;

    // Atom ids: walk every property formula, then every assumption formula,
    // in pre-order and number the input_is/output_has occurrences; collect
    // their definitions for the engines' per-step precomputation.
    let mut input_atoms: Vec<Vec<(usize, Vec<(usize, u64, bool)>)>> = Vec::new();
    let mut output_atoms: Vec<(String, Option<String>, Vec<(usize, u64, bool)>)> = Vec::new();
    for prop in &mut properties {
        assign_atom_ids(&mut prop.formula, &mut input_atoms, &mut output_atoms);
    }
    for a in &mut assumptions {
        assign_atom_ids(&mut a.formula, &mut input_atoms, &mut output_atoms);
    }

    let state_bits = ceil_log2(states.len());
    Ok(SmtSpec {
        states,
        initial_state,
        state_bits,
        vars,
        classes,
        transitions,
        properties,
        on_error,
        assumptions,
        input_atoms,
        output_atoms,
    })
}

/// Convert one assumption formula for per-sample assertion (task 3C, D3).
/// A top-level `always`/`always[N]` is DROPPED - the per-sample assertion loop
/// IS the universal quantification over samples (exact mode steps the
/// assumption monitor at every sample, SEMANTICS 7.3/7.5). Other top forms are
/// carried as their [`Sf`] nodes and asserted at every sample from which their
/// horizon-truncated expansion is evaluated. Task 3G: a top form that is none
/// of the listed ones (a bare boolean formula, a top-level `next`, ...) is
/// likewise asserted per-sample as its converted [`Sf`] node - a bare boolean
/// means "holds at every sample", a top-level `next` shifts by one, both
/// matching exact mode's Range::all() anchoring. FULL class: top-level
/// eventually/bounded_eventually/until (the liveness forms) or a temporal
/// operator nested under another - asserted only with `--full-assumptions`;
/// the classification still comes from `nested_temporal` on the original
/// formula, so expensive shapes stay behind the flag.
fn convert_assumption(
    a: &crate::lang::ir::Property,
    ctx: &ExprCtx,
) -> Result<SmtAssumption, String> {
    let (formula, bound, full) = match &a.formula {
        Formula::Always { inner } => (
            convert_formula(inner, ctx)?,
            None,
            nested_temporal(&a.formula),
        ),
        Formula::BoundedAlways { bound: n, inner } => (
            convert_formula(inner, ctx)?,
            Some(*n as usize),
            nested_temporal(&a.formula),
        ),
        Formula::Eventually { inner } => (
            Sf::Eventually(Box::new(convert_formula(inner, ctx)?)),
            None,
            true,
        ),
        Formula::BoundedEventually { bound: n, inner } => (
            Sf::BoundedEventually {
                bound: *n as usize,
                inner: Box::new(convert_formula(inner, ctx)?),
            },
            None,
            true,
        ),
        Formula::Until { left, right } => (
            Sf::Until {
                left: Box::new(convert_formula(left, ctx)?),
                right: Box::new(convert_formula(right, ctx)?),
            },
            None,
            true,
        ),
        // Task 3G: any other top form is asserted per-sample as its converted
        // Sf node - the Always arm's shape verbatim, with nested_temporal on
        // the ORIGINAL (unwrapped) formula so classification is not fooled by
        // a wrapper.
        other => (
            convert_formula(other, ctx)?,
            None,
            nested_temporal(&a.formula),
        ),
    };
    Ok(SmtAssumption {
        name: a.name.clone(),
        formula,
        bound,
        next_depth: next_depth(&a.formula),
        full,
    })
}

fn variant_name(v: &Option<crate::lang::ir::PatternVariant>) -> Option<String> {
    match v {
        Some(crate::lang::ir::PatternVariant::Unit { name }) => Some(name.clone()),
        Some(crate::lang::ir::PatternVariant::Struct { name, .. }) => Some(name.clone()),
        None => None,
    }
}

/// Index of a concrete field within the class's concrete-fields list.
fn fi_concrete(class: &SmtClass, name: &str) -> usize {
    class
        .fields
        .iter()
        .position(|(n, _, _, _)| n == name)
        .expect("concrete field listed")
}

fn convert_expr(
    e: &Expr,
    ctx: &ExprCtx,
    variant: &crate::mc::plan::VariantInputSpec,
) -> Result<Se, String> {
    match e {
        Expr::Int { value } => Ok(Se::Lit {
            value: *value,
            is_bool: false,
        }),
        Expr::Bool { value } => Ok(Se::Lit {
            value: u64::from(*value),
            is_bool: true,
        }),
        Expr::Path { path } => {
            if path.len() > 1 {
                // Enum path literal (task 3D): `Color::Red` resolves to the
                // variant's DECLARATION-ORDER INDEX - the same value exact
                // mode substitutes for Val::Enum at configuration build time.
                // The enum type is fixed by context (the validator already
                // checked the path against the module's enums).
                let variants = ctx
                    .enums
                    .get(&path[0])
                    .ok_or_else(|| format!("unknown enum type '{}' in expression", path[0]))?;
                return variants
                    .iter()
                    .position(|s| s == &path[1])
                    .map(|i| Se::Lit {
                        value: i as u64,
                        is_bool: false,
                    })
                    .ok_or_else(|| format!("unknown variant '{}' in enum '{}'", path[1], path[0]));
            }
            let name = &path[0];
            if name == "state" {
                return Ok(Se::State);
            }
            // Renamed event parameter: the pattern bound this name to a
            // declared payload field positionally - resolve the alias first so
            // the width/domain lookup below uses the declared name.
            let name = ctx.bind_alias.get(name).map(|s| s.as_str()).unwrap_or(name);
            if let Some(f) = ctx.field.get(name) {
                match f {
                    Some(idx) => {
                        let fs = variant
                            .fields
                            .iter()
                            .find(|fs| fs.name == *name)
                            .expect("field listed in class");
                        let (width, is_b) = match &fs.domain {
                            Some(crate::mc::plan::ValueDomain::Bool) => (1u32, true),
                            Some(crate::mc::plan::ValueDomain::Int { max }) => {
                                (ceil_log2((*max as usize) + 1), false)
                            }
                            // Enum-typed concrete field (task 3D): the value
                            // is the variant's declaration-order index.
                            Some(crate::mc::plan::ValueDomain::Enum { variants, .. }) => {
                                (ceil_log2(variants.len()), false)
                            }
                            _ => unreachable!("concrete field domain"),
                        };
                        return Ok(Se::Field {
                            idx: *idx,
                            width,
                            is_bool: is_b,
                        });
                    }
                    None => {
                        // Abstracted field: substitute the representative literal.
                        let spec = variant
                            .fields
                            .iter()
                            .find(|fs| fs.name == *name)
                            .ok_or_else(|| format!("unknown payload field '{name}'"))?;
                        return match &spec.representative {
                            Some(crate::codegen::fsm::RepValue::Bool(b)) => {
                                Ok(Se::Lit { value: u64::from(*b), is_bool: true })
                            }
                            Some(crate::codegen::fsm::RepValue::Int(v)) => {
                                Ok(Se::Lit { value: *v, is_bool: false })
                            }
                            other => Err(format!(
                                "smt-bounded mode cannot substitute representative {:?} for field '{name}'",
                                other.as_ref().map(|_| "enum")
                            )),
                        };
                    }
                }
            }
            if let Some(vi) = ctx.var_idx.get(name) {
                return Ok(Se::Var(*vi));
            }
            if let Some(si) = ctx.state_variant_idx.get(name) {
                return Ok(Se::Lit {
                    value: *si as u64,
                    is_bool: false,
                });
            }
            Err(format!("unknown identifier '{name}' in expression"))
        }
        Expr::Unary { op, expr } => {
            let inner = convert_expr(expr, ctx, variant)?;
            Ok(match op {
                crate::lang::ir::UnaryOp::Not => Se::Not(Box::new(inner)),
                crate::lang::ir::UnaryOp::BitNot => Se::BitNot(Box::new(inner)),
                crate::lang::ir::UnaryOp::Neg => Se::Neg(Box::new(inner)),
            })
        }
        Expr::Binary { op, left, right } => {
            let l = convert_expr(left, ctx, variant)?;
            let r = convert_expr(right, ctx, variant)?;
            let sb = match op {
                crate::lang::ir::BinOp::Plus => Sb::Plus,
                crate::lang::ir::BinOp::Minus => Sb::Minus,
                crate::lang::ir::BinOp::Star => Sb::Star,
                crate::lang::ir::BinOp::ShiftLeft => Sb::ShiftLeft,
                crate::lang::ir::BinOp::ShiftRight => Sb::ShiftRight,
                crate::lang::ir::BinOp::Eq => Sb::Eq,
                crate::lang::ir::BinOp::NotEq => Sb::NotEq,
                crate::lang::ir::BinOp::Lt => Sb::Lt,
                crate::lang::ir::BinOp::Le => Sb::Le,
                crate::lang::ir::BinOp::Gt => Sb::Gt,
                crate::lang::ir::BinOp::Ge => Sb::Ge,
                crate::lang::ir::BinOp::BitOr => Sb::BitOr,
                crate::lang::ir::BinOp::BitXor => Sb::BitXor,
                crate::lang::ir::BinOp::BitAnd => Sb::BitAnd,
                crate::lang::ir::BinOp::LogicOr => Sb::LogicOr,
                crate::lang::ir::BinOp::LogicAnd => Sb::LogicAnd,
            };
            Ok(Se::Bin(sb, Box::new(l), Box::new(r)))
        }
    }
}

fn convert_formula(f: &Formula, ctx: &ExprCtx) -> Result<Sf, String> {
    match f {
        Formula::Expr { expr } => {
            // No payload context in properties: use an empty variant view.
            let variant = crate::mc::plan::VariantInputSpec {
                group: String::new(),
                variant: String::new(),
                unique_name: String::new(),
                fields: Vec::new(),
            };
            Ok(Sf::Se(convert_expr(expr, ctx, &variant)?))
        }
        Formula::Not { inner } => Ok(Sf::Not(Box::new(convert_formula(inner, ctx)?))),
        Formula::And { left, right } => Ok(Sf::And(
            Box::new(convert_formula(left, ctx)?),
            Box::new(convert_formula(right, ctx)?),
        )),
        Formula::Or { left, right } => Ok(Sf::Or(
            Box::new(convert_formula(left, ctx)?),
            Box::new(convert_formula(right, ctx)?),
        )),
        Formula::Implies { left, right } => Ok(Sf::Implies(
            Box::new(convert_formula(left, ctx)?),
            Box::new(convert_formula(right, ctx)?),
        )),
        Formula::InputIs { pattern } => {
            // Resolved against the abstraction table: one arm per matching
            // input class with its literal field constraints in GLOBAL field
            // indices. Abstracted fields fold against their representative at
            // conversion time (the abstraction fixes them to that value).
            let arms = resolve_input_is(pattern, ctx)?;
            Ok(Sf::InputIs { id: 0, arms })
        }
        Formula::OutputHas { pattern } => {
            let mut constraints: Vec<(usize, u64, bool)> = Vec::new();
            if let Some(crate::lang::ir::PatternVariant::Struct { fields, .. }) = &pattern.variant {
                for (pos, pf) in fields.iter().enumerate() {
                    match &pf.pattern {
                        crate::lang::ir::PatternExpr::Wildcard
                        | crate::lang::ir::PatternExpr::Bind { .. } => {}
                        crate::lang::ir::PatternExpr::Int { value } => {
                            constraints.push((pos, *value, false))
                        }
                        crate::lang::ir::PatternExpr::Bool { value } => {
                            constraints.push((pos, u64::from(*value), true))
                        }
                        // Enum-variant output pattern (task 3D): the emitted
                        // enum argument encodes as its variant's index, so the
                        // constraint is an index equality - the same (pos,
                        // value, is_bool) triple, value = the index.
                        crate::lang::ir::PatternExpr::EnumVariant { path } => {
                            let variants = ctx.enums.get(&path[0]).ok_or_else(|| {
                                format!("unknown enum type '{}' in output pattern", path[0])
                            })?;
                            let idx =
                                variants.iter().position(|s| s == &path[1]).ok_or_else(|| {
                                    format!("unknown variant '{}' in enum '{}'", path[1], path[0])
                                })?;
                            constraints.push((pos, idx as u64, false))
                        }
                    }
                }
            }
            Ok(Sf::OutputHas {
                id: 0,
                group: pattern.group.clone(),
                variant: variant_name(&pattern.variant),
                constraints,
            })
        }
        Formula::Next { inner } => Ok(Sf::Next(Box::new(convert_formula(inner, ctx)?))),
        Formula::BoundedEventually { bound, inner } => Ok(Sf::BoundedEventually {
            bound: *bound as usize,
            inner: Box::new(convert_formula(inner, ctx)?),
        }),
        Formula::Until { left, right } => Ok(Sf::Until {
            left: Box::new(convert_formula(left, ctx)?),
            right: Box::new(convert_formula(right, ctx)?),
        }),
        // Nested under another temporal operator (top-level forms are handled
        // by the caller) - FULL class.
        Formula::Always { inner } => Ok(Sf::BoundedAlways {
            bound: usize::MAX,
            inner: Box::new(convert_formula(inner, ctx)?),
        }),
        Formula::BoundedAlways { bound, inner } => Ok(Sf::BoundedAlways {
            bound: *bound as usize,
            inner: Box::new(convert_formula(inner, ctx)?),
        }),
        Formula::Eventually { inner } => Ok(Sf::Eventually(Box::new(convert_formula(inner, ctx)?))),
    }
}

/// Resolve an input_is pattern against the abstraction table into per-class
/// arms. `ctx.input_classes` carries the (class index, group, variant name,
/// per-declared-field concreteness + representative) view built once per spec.
fn resolve_input_is(
    pattern: &crate::lang::ir::EventPattern,
    ctx: &ExprCtx,
) -> Result<Vec<(usize, Vec<(usize, u64, bool)>)>, String> {
    let mut arms: Vec<(usize, Vec<(usize, u64, bool)>)> = Vec::new();
    for (ci, group, variant, concrete, reps) in &ctx.input_classes {
        if group != &pattern.group {
            continue;
        }
        let variant_ok = match &pattern.variant {
            None => true,
            Some(crate::lang::ir::PatternVariant::Unit { name }) => variant == name,
            Some(crate::lang::ir::PatternVariant::Struct { name, fields }) => {
                variant == name && concrete.len() == fields.len()
            }
        };
        if !variant_ok {
            continue;
        }
        let mut constraints: Vec<(usize, u64, bool)> = Vec::new();
        let mut impossible = false;
        if let Some(crate::lang::ir::PatternVariant::Struct { fields, .. }) = &pattern.variant {
            for (pos, pf) in fields.iter().enumerate() {
                match &pf.pattern {
                    crate::lang::ir::PatternExpr::Wildcard
                    | crate::lang::ir::PatternExpr::Bind { .. } => {}
                    crate::lang::ir::PatternExpr::Int { value } => {
                        if let Some(g) = concrete[pos] {
                            constraints.push((g, *value, false));
                        } else {
                            // Abstracted field: fold against the representative.
                            match &reps[pos] {
                                Some(v) if *v == *value => {}
                                _ => impossible = true,
                            }
                        }
                    }
                    crate::lang::ir::PatternExpr::Bool { value } => {
                        if let Some(g) = concrete[pos] {
                            constraints.push((g, u64::from(*value), true));
                        } else {
                            match &reps[pos] {
                                Some(v) if *v == u64::from(*value) => {}
                                _ => impossible = true,
                            }
                        }
                    }
                    // Enum-variant input pattern (task 3D): the field encodes
                    // as its variant's index - a concrete field gets an index
                    // constraint; an abstracted field folds against its
                    // representative's index (the first declared variant),
                    // exactly like the Int/Bool folding above.
                    crate::lang::ir::PatternExpr::EnumVariant { path } => {
                        let variants = ctx.enums.get(&path[0]).ok_or_else(|| {
                            format!("unknown enum type '{}' in input pattern", path[0])
                        })?;
                        let idx = variants.iter().position(|s| s == &path[1]).ok_or_else(|| {
                            format!("unknown variant '{}' in enum '{}'", path[1], path[0])
                        })? as u64;
                        if let Some(g) = concrete[pos] {
                            constraints.push((g, idx, false));
                        } else {
                            match &reps[pos] {
                                Some(v) if *v == idx => {}
                                _ => impossible = true,
                            }
                        }
                    }
                }
            }
        }
        if !impossible {
            arms.push((*ci, constraints));
        }
    }
    Ok(arms)
}

/// FULL class iff the top form is bounded_eventually/until, or a temporal
/// operator (other than `next`) occurs strictly inside another temporal
/// operator. `next` alone is never full (a pure step shift).
fn classify_full(top: &Formula) -> bool {
    match top {
        // Top-level until is FULL (horizon-truncated expansion). Top-level
        // bounded_eventually[N] is DEFAULT class: the engine checks it as a
        // one-shot with the pending rule (no formula blowup).
        Formula::Until { .. } => true,
        Formula::BoundedEventually { .. } => false,
        _ => nested_temporal(top),
    }
}

/// True iff a temporal operator (other than `next`) occurs strictly inside
/// another temporal operator of `f`.
fn nested_temporal(f: &Formula) -> bool {
    match f {
        Formula::Always { inner }
        | Formula::BoundedAlways { inner, .. }
        | Formula::Eventually { inner }
        | Formula::BoundedEventually { inner, .. } => contains_temporal(inner),
        Formula::Until { left, right } => contains_temporal(left) || contains_temporal(right),
        // A temporal under `next` is still nested (the shift moves the whole
        // temporal structure).
        Formula::Next { inner } => contains_temporal(inner),
        _ => false,
    }
}

fn contains_temporal(f: &Formula) -> bool {
    match f {
        Formula::Always { .. }
        | Formula::BoundedAlways { .. }
        | Formula::Eventually { .. }
        | Formula::BoundedEventually { .. }
        | Formula::Until { .. } => true,
        Formula::Next { inner } => contains_temporal(inner),
        Formula::Not { inner } => contains_temporal(inner),
        Formula::And { left, right }
        | Formula::Or { left, right }
        | Formula::Implies { left, right } => contains_temporal(left) || contains_temporal(right),
        Formula::Expr { .. } | Formula::InputIs { .. } | Formula::OutputHas { .. } => false,
    }
}

/// Maximum nesting depth of `next` in the formula (0 = none). Every temporal
/// wrapper is descended through explicitly - a `next` under `always`/`eventually`
/// still shifts its evaluation one step and must be clipped at the horizon.
fn next_depth(f: &Formula) -> usize {
    match f {
        Formula::Next { inner } => 1 + next_depth(inner),
        Formula::Not { inner } => next_depth(inner),
        Formula::And { left, right }
        | Formula::Or { left, right }
        | Formula::Implies { left, right } => next_depth(left).max(next_depth(right)),
        Formula::Always { inner }
        | Formula::BoundedAlways { inner, .. }
        | Formula::BoundedEventually { inner, .. }
        | Formula::Eventually { inner } => next_depth(inner),
        Formula::Until { left, right } => next_depth(left).max(next_depth(right)),
        _ => 0,
    }
}

/// Number the input_is/output_has occurrences of a formula in pre-order and
/// append their definitions to the atom lists (ids are the list indices).
fn assign_atom_ids(
    sf: &mut Sf,
    input_atoms: &mut Vec<Vec<(usize, Vec<(usize, u64, bool)>)>>,
    output_atoms: &mut Vec<(String, Option<String>, Vec<(usize, u64, bool)>)>,
) {
    match sf {
        Sf::InputIs { id, arms } => {
            *id = input_atoms.len();
            input_atoms.push(std::mem::take(arms));
        }
        Sf::OutputHas {
            id,
            group,
            variant,
            constraints,
        } => {
            *id = output_atoms.len();
            output_atoms.push((group.clone(), variant.clone(), std::mem::take(constraints)));
        }
        Sf::Not(inner) | Sf::Next(inner) | Sf::Eventually(inner) => {
            assign_atom_ids(inner, input_atoms, output_atoms)
        }
        Sf::BoundedEventually { inner, .. } | Sf::BoundedAlways { inner, .. } => {
            assign_atom_ids(inner, input_atoms, output_atoms)
        }
        Sf::And(l, r) | Sf::Or(l, r) | Sf::Implies(l, r) | Sf::Until { left: l, right: r } => {
            assign_atom_ids(l, input_atoms, output_atoms);
            assign_atom_ids(r, input_atoms, output_atoms);
        }
        Sf::Se(_) => {}
    }
}

/// Type of an [`Se`] node after name resolution. Mirrors
/// `validator::ExprType` with Enum already resolved to concrete widths;
/// `IntLit` is kept distinct (as in the validator) so binary-op rules can lift
/// it to a sibling's width.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeType {
    Bool,
    Bit(u32),
    IntLit,
}

impl SeType {
    /// Bit width used for z3 sort selection (bool = 1 bit; IntLit defaults to
    /// 8 - only valid when no parent context applies).
    pub fn width(self) -> u32 {
        match self {
            SeType::Bool => 1,
            SeType::Bit(w) => w,
            SeType::IntLit => 8,
        }
    }
}

/// Bottom-up type of an expression node, computed with EXACTLY the validator's
/// rules (`type_expr`/`binary_type`/`bit_op_type`): integer literals default
/// to `Bit(8)` and are lifted by their parent per the binary-op rules. The
/// spec is already validated, so this never fails; both emitters use it to
/// pick z3 constructors (Bool vs bitvector) and to mask literals to the right
/// width.
pub fn se_type(se: &Se, spec: &SmtSpec) -> SeType {
    match se {
        Se::Lit { is_bool: true, .. } => SeType::Bool,
        Se::Lit { is_bool: false, .. } => SeType::IntLit,
        Se::State => SeType::Bit(spec.state_bits),
        Se::Var(i) => {
            if spec.vars[*i].is_bool {
                SeType::Bool
            } else {
                SeType::Bit(spec.vars[*i].width)
            }
        }
        Se::Field { width, is_bool, .. } => {
            if *is_bool {
                SeType::Bool
            } else {
                SeType::Bit(*width)
            }
        }
        Se::Not(_) => SeType::Bool,
        Se::BitNot(inner) | Se::Neg(inner) => match se_type(inner, spec) {
            // IntLit under bit_not/neg defaults to Bit(8) (validator rule).
            SeType::IntLit => SeType::Bit(8),
            t @ SeType::Bit(_) => t,
            _ => SeType::Bit(8),
        },
        Se::Bin(op, l, r) => {
            let lt = se_type(l, spec);
            let rt = se_type(r, spec);
            match op {
                Sb::LogicAnd | Sb::LogicOr => SeType::Bool,
                Sb::Eq | Sb::NotEq | Sb::Lt | Sb::Le | Sb::Gt | Sb::Ge => SeType::Bool,
                Sb::Plus | Sb::Minus | Sb::Star | Sb::BitXor => bit_op_type(lt, rt),
                Sb::ShiftLeft | Sb::ShiftRight => match lt {
                    // Width comes from the LEFT operand; the right may be any
                    // width (validator rule).
                    SeType::IntLit => SeType::Bit(8),
                    t @ SeType::Bit(_) => t,
                    _ => SeType::Bit(8),
                },
                Sb::BitAnd | Sb::BitOr => {
                    if lt == SeType::Bool && rt == SeType::Bool {
                        SeType::Bool
                    } else {
                        bit_op_type(lt, rt)
                    }
                }
            }
        }
    }
}

/// Validator `bit_op_type`: (Bit(a),Bit(b)) requires a==b; IntLit lifts to the
/// other operand's width; (IntLit,IntLit) -> Bit(8).
fn bit_op_type(lt: SeType, rt: SeType) -> SeType {
    match (lt, rt) {
        (SeType::Bit(a), SeType::Bit(b)) => {
            debug_assert_eq!(a, b, "validated spec: equal widths");
            SeType::Bit(a)
        }
        (SeType::IntLit, SeType::Bit(b)) | (SeType::Bit(b), SeType::IntLit) => SeType::Bit(b),
        (SeType::IntLit, SeType::IntLit) => SeType::Bit(8),
        // Bool operands are rejected by the validator for these ops; a
        // validated spec never reaches this arm.
        _ => SeType::Bit(8),
    }
}
