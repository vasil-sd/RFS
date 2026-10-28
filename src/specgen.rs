//! Seeded random spec generator + incremental mutation (Phase 6 task 3, D-P6-11).
//!
//! Test/reference infrastructure (public lib module, like `monitor` and
//! `ref_mc`). Deterministic from a u64 seed using the project's shared LCG
//! lineage - the SAME constants as `codegen_test::Lcg`, so all seeded machinery
//! in the toolchain shares one RNG family. Generated specs are rendered through
//! `crate::printer` (guaranteeing parseability and exercising the printer) and
//! every generated/mutated module is validator-checked.
//! Docs: dev:cli-commands-overview, llm:cli-commands

use std::collections::HashSet;

use crate::ir::{
    Assignment, BinOp, ConstructorGroup, ConstructorVariant, EnumDecl, EventPattern, Expr,
    FieldDecl, Formula, Ir, Module, OnError, OutputExpr, OutputPattern, PatternExpr, PatternField,
    PatternVariant, Property, Transition, Type, UnaryOp, VarDecl, IR_VERSION,
};
use crate::validator;

// ---------------------------------------------------------------------------
// RNG (shared LCG lineage)
// ---------------------------------------------------------------------------

/// The project's seeded RNG: `state = state * 6364136223846793005 +
/// 1442695040888963407` (wrapping) - the same LCG family and top-53-bit double
/// mapping as `codegen_test::Lcg` / the generated fuzz runtimes.
#[derive(Debug, Clone)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn step(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0
    }

    pub fn next_u64(&mut self) -> u64 {
        self.step()
    }

    /// Same top-53-bit double mapping as the fuzz wire format.
    pub fn next_f64(&mut self) -> f64 {
        ((self.step() >> 11) as f64) / ((u64::MAX >> 11) as f64)
    }

    /// Uniform integer in `[lo, hi)` (panics on an empty range).
    pub fn range(&mut self, lo: usize, hi: usize) -> usize {
        assert!(hi > lo, "empty range");
        lo + (self.next_u64() % (hi - lo) as u64) as usize
    }

    /// A random element (`None` for an empty slice).
    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> Option<&'a T> {
        if items.is_empty() {
            None
        } else {
            Some(&items[self.range(0, items.len())])
        }
    }

    /// True with probability `p`.
    pub fn chance(&mut self, p: f64) -> bool {
        self.next_f64() < p
    }
}

// ---------------------------------------------------------------------------
// Bounds
// ---------------------------------------------------------------------------

/// Size caps for generated specs. `gen-spec` exposes most of them as flags;
/// `max_outputs` stays an internal knob (set by the presets).
#[derive(Debug, Clone)]
pub struct Bounds {
    pub max_states: usize,
    pub max_events: usize,
    pub max_outputs: usize,
    pub max_vars: usize,
    pub max_transitions: usize,
    /// Properties + assumptions combined.
    pub max_props: usize,
}

impl Bounds {
    /// `small`: <= 6 states - reachable sets stay tiny (MC fuzzing).
    pub fn small() -> Self {
        Self {
            max_states: 6,
            max_events: 4,
            max_outputs: 3,
            max_vars: 3,
            max_transitions: 12,
            max_props: 3,
        }
    }

    /// `medium`: benchmark-like.
    pub fn medium() -> Self {
        Self {
            max_states: 10,
            max_events: 6,
            max_outputs: 4,
            max_vars: 5,
            max_transitions: 30,
            max_props: 5,
        }
    }
}

impl Default for Bounds {
    fn default() -> Self {
        Self::small()
    }
}

// ---------------------------------------------------------------------------
// Shared construction helpers (used by both generation and mutation)
// ---------------------------------------------------------------------------

/// A random value type over the module's user enums.
pub(crate) fn random_type(rng: &mut Rng, enum_names: &[String]) -> Type {
    let mut choices = vec![Type::Bool];
    for w in 2..=8u32 {
        if rng.chance(0.5) {
            choices.push(Type::Bit { width: w });
        }
    }
    for e in enum_names {
        choices.push(Type::Enum { name: e.clone() });
    }
    choices
        .get(rng.range(0, choices.len()))
        .cloned()
        .unwrap_or(Type::Bool)
}

/// A typed constant expression (always valid as a var initializer).
pub(crate) fn const_expr(
    rng: &mut Rng,
    ty: &Type,
    enum_variants: &[(String, Vec<String>)],
) -> Expr {
    match ty {
        Type::Bool => Expr::Bool {
            value: rng.chance(0.5),
        },
        Type::Bit { width } => {
            let max = 1u64 << (*width).min(32);
            Expr::Int {
                value: (rng.next_u64() % max) / 2,
            }
        }
        Type::Enum { name } => {
            let variants = enum_variants
                .iter()
                .find(|(n, _)| n == name)
                .map(|(_, v)| v.clone())
                .unwrap_or_default();
            let v = variants
                .get(rng.range(0, variants.len().max(1)))
                .cloned()
                .unwrap_or_else(|| "V0".to_string());
            Expr::Path {
                path: vec![name.clone(), v],
            }
        }
    }
}

/// A random constructor group (event or output) with 1-3 variants.
pub(crate) fn random_constructor_group(
    rng: &mut Rng,
    name: &str,
    enum_names: &[String],
) -> ConstructorGroup {
    let n = rng.range(1, 4);
    let mut variants = Vec::new();
    for j in 0..n {
        if rng.chance(0.5) {
            variants.push(ConstructorVariant::Unit {
                name: format!("U{j}"),
            });
        } else {
            let nf = rng.range(0, 3);
            let fields: Vec<FieldDecl> = (0..nf)
                .map(|f| FieldDecl {
                    name: format!("f{f}"),
                    ty: random_type(rng, enum_names),
                })
                .collect();
            variants.push(ConstructorVariant::Struct {
                name: format!("S{j}"),
                fields,
            });
        }
    }
    ConstructorGroup {
        name: name.to_string(),
        variants,
    }
}

/// A typed expression over the given environment. `enum_variants` maps each
/// user enum to its variant names (for constant fallbacks).
pub(crate) fn expr_of_type(
    rng: &mut Rng,
    ty: &Type,
    enum_variants: &[(String, Vec<String>)],
    vars: &[(String, Type)],
    binds: &[(String, Type)],
) -> Expr {
    let sources: Vec<(String, Type)> = {
        let mut v = Vec::new();
        if matches!(ty, Type::Enum { name } if name == "State") {
            v.push((
                "state".to_string(),
                Type::Enum {
                    name: "State".into(),
                },
            ));
        }
        for (n, t) in vars.iter().chain(binds.iter()) {
            if std::mem::discriminant(t) == std::mem::discriminant(ty)
                && match (t, ty) {
                    (Type::Bit { width: a }, Type::Bit { width: b }) => a == b,
                    (Type::Enum { name: a }, Type::Enum { name: b }) => a == b,
                    _ => true,
                }
            {
                v.push((n.clone(), t.clone()));
            }
        }
        v
    };
    if !sources.is_empty() && rng.chance(0.6) {
        return Expr::Path {
            path: vec![sources[rng.range(0, sources.len())].0.clone()],
        };
    }
    match ty {
        Type::Bool => Expr::Bool {
            value: rng.chance(0.5),
        },
        other => const_expr(rng, other, enum_variants),
    }
}

/// One boolean atom over the environment (never returns None in practice).
fn bool_atom(
    rng: &mut Rng,
    ty_hint: Option<&Type>,
    states: &[String],
    enum_variants: &[(String, Vec<String>)],
    vars: &[(String, Type)],
    binds: &[(String, Type)],
) -> Option<Expr> {
    let mut seen_types: Vec<Type> = Vec::new();
    let sources_of = |ty: &Type| -> Vec<(String, Type)> {
        let mut v = Vec::new();
        if matches!(ty, Type::Enum { name } if name == "State") {
            v.push((
                "state".to_string(),
                Type::Enum {
                    name: "State".into(),
                },
            ));
        }
        for (n, t) in vars.iter().chain(binds.iter()) {
            if std::mem::discriminant(t) == std::mem::discriminant(ty)
                && match (t, ty) {
                    (Type::Bit { width: a }, Type::Bit { width: b }) => a == b,
                    (Type::Enum { name: a }, Type::Enum { name: b }) => a == b,
                    _ => true,
                }
            {
                v.push((n.clone(), t.clone()));
            }
        }
        v
    };

    // (a) a bare boolean source.
    if let Some((name, _)) = sources_of(&Type::Bool).into_iter().next() {
        if !seen_types.contains(&Type::Bool) {
            seen_types.push(Type::Bool.clone());
            let e = Expr::Path { path: vec![name] };
            return Some(if rng.chance(0.3) {
                Expr::Unary {
                    op: UnaryOp::Not,
                    expr: Box::new(e),
                }
            } else {
                e
            });
        }
    }

    // (b) compare the state against a random variant.
    if !states.is_empty() && rng.chance(0.4) {
        let v = states[rng.range(0, states.len())].clone();
        let op = if rng.chance(0.5) {
            BinOp::Eq
        } else {
            BinOp::NotEq
        };
        return Some(Expr::Binary {
            op,
            left: Box::new(Expr::Path {
                path: vec!["state".into()],
            }),
            right: Box::new(Expr::Path { path: vec![v] }),
        });
    }

    // (c) compare two same-typed sources (bit or enum).
    for ty in ty_hint.iter().copied().chain(std::iter::once(&Type::Bool)) {
        let srcs = sources_of(ty);
        if srcs.len() >= 2 && !seen_types.contains(ty) {
            seen_types.push(ty.clone());
            let i = rng.range(0, srcs.len());
            let j = (i + 1) % srcs.len();
            let ops = [BinOp::Eq, BinOp::NotEq];
            return Some(Expr::Binary {
                op: ops[rng.range(0, 2)],
                left: Box::new(Expr::Path {
                    path: vec![srcs[i].0.clone()],
                }),
                right: Box::new(Expr::Path {
                    path: vec![srcs[j].0.clone()],
                }),
            });
        }
    }

    // (d) one source vs a typed constant.
    for ty in ty_hint.iter().copied() {
        let srcs = sources_of(ty);
        if !srcs.is_empty() && !seen_types.contains(ty) {
            seen_types.push(ty.clone());
            let name = srcs[rng.range(0, srcs.len())].0.clone();
            let c = const_expr(rng, ty, enum_variants);
            let ops: &[BinOp] = match ty {
                Type::Bit { .. } => &[BinOp::Lt, BinOp::Gt, BinOp::Le, BinOp::Ge, BinOp::Eq],
                _ => &[BinOp::Eq, BinOp::NotEq],
            };
            return Some(Expr::Binary {
                op: ops[rng.range(0, ops.len())],
                left: Box::new(Expr::Path { path: vec![name] }),
                right: Box::new(c),
            });
        }
    }
    None
}

/// A random bool-typed expression (1-3 atoms joined by &&/||).
pub(crate) fn expr_bool(
    rng: &mut Rng,
    states: &[String],
    enum_variants: &[(String, Vec<String>)],
    vars: &[(String, Type)],
    binds: &[(String, Type)],
) -> Expr {
    let mut atoms = Vec::new();
    for _ in 0..rng.range(1, 4) {
        if let Some(a) = bool_atom(rng, None, states, enum_variants, vars, binds) {
            atoms.push(a);
        }
    }
    if atoms.is_empty() {
        return Expr::Bool { value: true };
    }
    let mut e = atoms.remove(0);
    for a in atoms {
        let op = if rng.chance(0.5) {
            BinOp::LogicAnd
        } else {
            BinOp::LogicOr
        };
        e = Expr::Binary {
            op,
            left: Box::new(e),
            right: Box::new(a),
        };
    }
    e
}

/// A random transition from `src` to `dst`. Guards/emits/assignments are built
/// type-safe over the pattern's bindings; priorities stay unique (the caller
/// owns the used set).
pub(crate) fn random_transition(
    rng: &mut Rng,
    module: &Module,
    src: &str,
    dst: &str,
    used_priorities: &mut HashSet<u32>,
) -> Transition {
    let group = module.events[rng.range(0, module.events.len())].clone();
    let variant = group.variants[rng.range(0, group.variants.len())].clone();
    let (pattern, binds): (PatternVariant, Vec<(String, Type)>) = match &variant {
        ConstructorVariant::Unit { name } => (PatternVariant::Unit { name: name.clone() }, vec![]),
        ConstructorVariant::Struct { fields, .. } => {
            let mut bs = Vec::new();
            let pf: Vec<PatternField> = fields
                .iter()
                .enumerate()
                .map(|(fi, f)| {
                    if rng.chance(0.8) {
                        let b = format!("b{}_{}", module.transitions.len(), fi);
                        bs.push((b.clone(), f.ty.clone()));
                        PatternField {
                            name: Some(b.clone()),
                            pattern: PatternExpr::Bind { name: b },
                        }
                    } else {
                        PatternField {
                            name: None,
                            pattern: PatternExpr::Wildcard,
                        }
                    }
                })
                .collect();
            (
                PatternVariant::Struct {
                    name: variant_struct_name(&variant).to_string(),
                    fields: pf,
                },
                bs,
            )
        }
    };

    let states = module
        .enums
        .iter()
        .find(|e| e.name == "State")
        .map(|e| e.variants.clone())
        .unwrap_or_default();
    let enum_variants: Vec<(String, Vec<String>)> = module
        .enums
        .iter()
        .filter(|e| e.name != "State")
        .map(|e| (e.name.clone(), e.variants.clone()))
        .collect();
    let vars: Vec<(String, Type)> = module
        .vars
        .iter()
        .map(|v| (v.name.clone(), v.ty.clone()))
        .collect();

    let guard = if rng.chance(0.5) {
        Some(expr_bool(rng, &states, &enum_variants, &vars, &binds))
    } else {
        None
    };

    let mut emits: Vec<OutputExpr> = Vec::new();
    for _ in 0..rng.range(0, 3) {
        if module.outputs.is_empty() {
            break;
        }
        let og = &module.outputs[rng.range(0, module.outputs.len())];
        let ov = &og.variants[rng.range(0, og.variants.len())];
        let args: Vec<Expr> = match ov {
            ConstructorVariant::Unit { .. } => vec![],
            ConstructorVariant::Struct { fields, .. } => fields
                .iter()
                .map(|f| expr_of_type(rng, &f.ty, &enum_variants, &vars, &binds))
                .collect(),
        };
        emits.push(OutputExpr {
            group: og.name.clone(),
            variant: Some(variant_struct_name(ov).to_string()),
            args,
        });
    }

    // Distinct vars only - the grammar rejects a var assigned twice in one
    // transition body (parser `set` action).
    let mut assignments: Vec<Assignment> = Vec::new();
    let n_assign = rng.range(0, 3).min(module.vars.len());
    if n_assign > 0 {
        let mut pool: Vec<usize> = (0..module.vars.len()).collect();
        for _ in 0..n_assign {
            let vi = pool.swap_remove(rng.range(0, pool.len()));
            let v = &module.vars[vi];
            assignments.push(Assignment {
                var: v.name.clone(),
                value: expr_of_type(rng, &v.ty, &enum_variants, &vars, &binds),
            });
        }
    }

    // Shadowing rule (validator): two transitions of the same state with an
    // identical pattern must not both lack priorities when their guards are
    // equal. If a no-priority sibling already carries this pattern, force a
    // unique priority on the new transition (mixed pairs are always legal).
    let has_noprio_sibling = module.transitions.iter().any(|t| {
        t.state == src
            && t.pattern.group == group.name
            && t.pattern.variant.as_ref() == Some(&pattern)
            && t.priority.is_none()
    });
    let priority = if has_noprio_sibling || rng.chance(0.3) {
        let mut p = rng.range(1, 101) as u32;
        while used_priorities.contains(&p) {
            p += 1;
        }
        used_priorities.insert(p);
        Some(p)
    } else {
        None
    };

    Transition {
        id: module.transitions.len(),
        state: src.to_string(),
        pattern: EventPattern {
            group: group.name,
            variant: Some(pattern),
        },
        guard,
        next_state: dst.to_string(),
        emits,
        assignments,
        priority,
    }
}

/// The constructor variant name (unit or struct).
fn variant_struct_name(v: &ConstructorVariant) -> &str {
    match v {
        ConstructorVariant::Unit { name } => name,
        ConstructorVariant::Struct { name, .. } => name,
    }
}

/// A random well-typed formula from the implemented subset (the templates the
/// reference MC and the monitors both understand).
pub(crate) fn random_formula(rng: &mut Rng, module: &Module) -> Formula {
    let states = module
        .enums
        .iter()
        .find(|e| e.name == "State")
        .map(|e| e.variants.clone())
        .unwrap_or_default();
    let enum_variants: Vec<(String, Vec<String>)> = module
        .enums
        .iter()
        .filter(|e| e.name != "State")
        .map(|e| (e.name.clone(), e.variants.clone()))
        .collect();

    match rng.range(0, 6) {
        // always (state ==|!= X)
        0 => Formula::Always {
            inner: Box::new(Formula::Expr {
                expr: Expr::Binary {
                    op: if rng.chance(0.5) {
                        BinOp::Eq
                    } else {
                        BinOp::NotEq
                    },
                    left: Box::new(Expr::Path {
                        path: vec!["state".into()],
                    }),
                    right: Box::new(Expr::Path {
                        path: vec![states[rng.range(0, states.len())].clone()],
                    }),
                },
            }),
        },
        // eventually (input_is(...))
        1 => Formula::Eventually {
            inner: Box::new(Formula::InputIs {
                pattern: random_unit_event(rng, module).unwrap_or_else(|| EventPattern {
                    group: module
                        .events
                        .first()
                        .map(|e| e.name.clone())
                        .unwrap_or_default(),
                    variant: None,
                }),
            }),
        },
        // bounded_eventually[N] (input_is(...))
        2 => Formula::BoundedEventually {
            bound: rng.range(1, 6) as u64,
            inner: Box::new(Formula::InputIs {
                pattern: random_unit_event(rng, module).unwrap_or_else(|| EventPattern {
                    group: module
                        .events
                        .first()
                        .map(|e| e.name.clone())
                        .unwrap_or_default(),
                    variant: None,
                }),
            }),
        },
        // always (var atom)
        3 => Formula::Always {
            inner: Box::new(Formula::Expr {
                expr: random_var_atom(rng, module, &enum_variants)
                    .unwrap_or(Expr::Bool { value: true }),
            }),
        },
        // (atom &&||| atom)
        4 => {
            let l =
                random_var_atom(rng, module, &enum_variants).unwrap_or(Expr::Bool { value: true });
            let r =
                random_var_atom(rng, module, &enum_variants).unwrap_or(Expr::Bool { value: false });
            Formula::And {
                left: Box::new(Formula::Expr { expr: l }),
                right: Box::new(Formula::Expr { expr: r }),
            }
        }
        // input_is(...) -> bounded_eventually[N] (output_has(...))
        _ => Formula::Implies {
            left: Box::new(Formula::InputIs {
                pattern: random_unit_event(rng, module).unwrap_or_else(|| EventPattern {
                    group: module
                        .events
                        .first()
                        .map(|e| e.name.clone())
                        .unwrap_or_default(),
                    variant: None,
                }),
            }),
            right: Box::new(Formula::BoundedEventually {
                bound: rng.range(1, 6) as u64,
                inner: Box::new(Formula::OutputHas {
                    pattern: random_unit_output(rng, module).unwrap_or_else(|| OutputPattern {
                        group: module
                            .outputs
                            .first()
                            .map(|o| o.name.clone())
                            .unwrap_or_default(),
                        variant: None,
                    }),
                }),
            }),
        },
    }
}

/// A unit-variant event pattern (formula patterns never bind, Phase 1); falls
/// back to an all-wildcard struct pattern when the group has no units.
fn random_unit_event(rng: &mut Rng, m: &Module) -> Option<EventPattern> {
    if m.events.is_empty() {
        return None;
    }
    let g = &m.events[rng.range(0, m.events.len())];
    let units: Vec<&ConstructorVariant> = g
        .variants
        .iter()
        .filter(|v| matches!(v, ConstructorVariant::Unit { .. }))
        .collect();
    if !units.is_empty() {
        return Some(EventPattern {
            group: g.name.clone(),
            variant: Some(PatternVariant::Unit {
                name: variant_struct_name(units[rng.range(0, units.len())]).to_string(),
            }),
        });
    }
    let s = &g.variants[rng.range(0, g.variants.len())];
    if let ConstructorVariant::Struct { fields, .. } = s {
        let pf: Vec<PatternField> = fields
            .iter()
            .map(|_| PatternField {
                name: None,
                pattern: PatternExpr::Wildcard,
            })
            .collect();
        return Some(EventPattern {
            group: g.name.clone(),
            variant: Some(PatternVariant::Struct {
                name: variant_struct_name(s).to_string(),
                fields: pf,
            }),
        });
    }
    None
}

/// The output counterpart of [`random_unit_event`].
fn random_unit_output(rng: &mut Rng, m: &Module) -> Option<OutputPattern> {
    if m.outputs.is_empty() {
        return None;
    }
    let g = &m.outputs[rng.range(0, m.outputs.len())];
    let units: Vec<&ConstructorVariant> = g
        .variants
        .iter()
        .filter(|v| matches!(v, ConstructorVariant::Unit { .. }))
        .collect();
    if !units.is_empty() {
        return Some(OutputPattern {
            group: g.name.clone(),
            variant: Some(PatternVariant::Unit {
                name: variant_struct_name(units[rng.range(0, units.len())]).to_string(),
            }),
        });
    }
    let s = &g.variants[rng.range(0, g.variants.len())];
    if let ConstructorVariant::Struct { fields, .. } = s {
        let pf: Vec<PatternField> = fields
            .iter()
            .map(|_| PatternField {
                name: None,
                pattern: PatternExpr::Wildcard,
            })
            .collect();
        return Some(OutputPattern {
            group: g.name.clone(),
            variant: Some(PatternVariant::Struct {
                name: variant_struct_name(s).to_string(),
                fields: pf,
            }),
        });
    }
    None
}

/// A typed boolean atom over one module variable (bool / bit / enum).
fn random_var_atom(
    rng: &mut Rng,
    m: &Module,
    enum_variants: &[(String, Vec<String>)],
) -> Option<Expr> {
    if m.vars.is_empty() {
        return None;
    }
    let v = &m.vars[rng.range(0, m.vars.len())];
    match &v.ty {
        Type::Bool => Some(if rng.chance(0.3) {
            Expr::Unary {
                op: UnaryOp::Not,
                expr: Box::new(Expr::Path {
                    path: vec![v.name.clone()],
                }),
            }
        } else {
            Expr::Path {
                path: vec![v.name.clone()],
            }
        }),
        Type::Bit { .. } => {
            let ops = [BinOp::Lt, BinOp::Gt, BinOp::Eq];
            Some(Expr::Binary {
                op: ops[rng.range(0, 3)],
                left: Box::new(Expr::Path {
                    path: vec![v.name.clone()],
                }),
                right: Box::new(const_expr(rng, &v.ty, enum_variants)),
            })
        }
        Type::Enum { .. } => Some(Expr::Binary {
            op: if rng.chance(0.5) {
                BinOp::Eq
            } else {
                BinOp::NotEq
            },
            left: Box::new(Expr::Path {
                path: vec![v.name.clone()],
            }),
            right: Box::new(const_expr(rng, &v.ty, enum_variants)),
        }),
    }
}

// ---------------------------------------------------------------------------
// SpecGen (random generation)
// ---------------------------------------------------------------------------

/// A deterministic random spec builder. `generate` always yields a valid,
/// parseable module (validated before return; a violation is a generator bug).
pub struct SpecGen {
    rng: Rng,
    bounds: Bounds,
}

impl SpecGen {
    pub fn new(seed: u64, bounds: Bounds) -> Self {
        Self {
            rng: Rng::new(seed),
            bounds,
        }
    }

    /// The configured caps (tests / diagnostics).
    pub fn bounds(&self) -> &Bounds {
        &self.bounds
    }

    /// Generate one random single-module spec.
    pub fn generate(&mut self) -> Ir {
        let module = self.build_module();
        // Defense in depth: construction is validity-by-design; a failure here
        // is a generator bug, not bad luck.
        let errors = validator::validate_module(&module);
        assert!(
            errors.is_empty(),
            "generator produced an invalid spec: {errors:?}"
        );
        Ir {
            ir_version: IR_VERSION,
            modules: vec![module],
        }
    }

    fn build_module(&mut self) -> Module {
        let b = &self.bounds;
        // States: letters A.. (caps keep us far below 26).
        let n_states = if b.max_states < 2 {
            1
        } else {
            self.rng.range(2, b.max_states + 1)
        };
        let states: Vec<String> = (0..n_states)
            .map(|i| format!("{}", (b'A' + i as u8) as char))
            .collect();

        // Optional user enum (for var/field types).
        let mut enum_names: Vec<String> = Vec::new();
        let mut enums: Vec<EnumDecl> = vec![EnumDecl {
            name: "State".into(),
            variants: states.clone(),
        }];
        if self.rng.chance(0.5) {
            let ename = "T0".to_string();
            let nv = self.rng.range(2, 5);
            enums.push(EnumDecl {
                name: ename.clone(),
                variants: (0..nv).map(|i| format!("V{i}")).collect(),
            });
            enum_names.push(ename);
        }
        let enum_variants: Vec<(String, Vec<String>)> = enums
            .iter()
            .filter(|e| e.name != "State")
            .map(|e| (e.name.clone(), e.variants.clone()))
            .collect();

        // Events / outputs.
        let n_events = self.rng.range(1, b.max_events + 1);
        let events: Vec<ConstructorGroup> = (0..n_events)
            .map(|i| random_constructor_group(&mut self.rng, &format!("E{i}"), &enum_names))
            .collect();
        // At least one output group: the formula templates (bounded_eventually
        // of an output) need a formable output_has pattern.
        let n_outputs = self.rng.range(1, b.max_outputs + 1);
        let outputs: Vec<ConstructorGroup> = (0..n_outputs)
            .map(|i| random_constructor_group(&mut self.rng, &format!("O{i}"), &enum_names))
            .collect();

        // Vars.
        let n_vars = self.rng.range(0, b.max_vars + 1);
        let vars: Vec<VarDecl> = (0..n_vars)
            .map(|i| {
                let ty = random_type(&mut self.rng, &enum_names);
                VarDecl {
                    name: format!("v{i}"),
                    init: const_expr(&mut self.rng, &ty, &enum_variants),
                    ty,
                }
            })
            .collect();

        // Build the module incrementally so transitions/formulas can reference
        // the real declarations.
        let mut module = Module {
            name: "gen".into(),
            version: None,
            on_error: None,
            enums,
            initial_state: Some(states[0].clone()),
            events,
            outputs,
            vars,
            transitions: Vec::new(),
            properties: Vec::new(),
            assumptions: Vec::new(),
        };

        // Transitions: a spanning tree from the initial state (reachability by
        // induction), then random extras up to the cap.
        let mut used_priorities: HashSet<u32> = HashSet::new();
        for i in 1..states.len() {
            let src = states[self.rng.range(0, i)].clone();
            module.transitions.push(random_transition(
                &mut self.rng,
                &module,
                &src,
                &states[i],
                &mut used_priorities,
            ));
        }
        while module.transitions.len() < b.max_transitions {
            let src = states[self.rng.range(0, states.len())].clone();
            let dst = states[self.rng.range(0, states.len())].clone();
            module.transitions.push(random_transition(
                &mut self.rng,
                &module,
                &src,
                &dst,
                &mut used_priorities,
            ));
        }
        for (i, t) in module.transitions.iter_mut().enumerate() {
            t.id = i;
        }

        // on_error (30%): next state + a unit-output emit (no bindings in scope).
        if self.rng.chance(0.3) {
            let units: Vec<&ConstructorGroup> = module
                .outputs
                .iter()
                .filter(|g| {
                    g.variants
                        .iter()
                        .any(|v| matches!(v, ConstructorVariant::Unit { .. }))
                })
                .collect();
            let mut emits = Vec::new();
            if let Some(g) = self.rng.pick(&units) {
                let u = g
                    .variants
                    .iter()
                    .find(|v| matches!(v, ConstructorVariant::Unit { .. }))
                    .unwrap();
                emits.push(OutputExpr {
                    group: g.name.clone(),
                    variant: Some(variant_struct_name(u).to_string()),
                    args: Vec::new(),
                });
            }
            module.on_error = Some(OnError {
                next_state: states[self.rng.range(0, states.len())].clone(),
                emits,
            });
        }

        // Properties / assumptions (~2:1 split), generated against the finished
        // module so every referenced declaration exists.
        let n_props = if b.max_props == 0 {
            0
        } else {
            self.rng.range(1, b.max_props + 1)
        };
        let n_assume = n_props / 3;
        let props: Vec<Property> = (0..n_props - n_assume)
            .map(|i| Property {
                name: format!("p{i}"),
                formula: random_formula(&mut self.rng, &module),
            })
            .collect();
        let assumes: Vec<Property> = (0..n_assume)
            .map(|i| Property {
                name: format!("a{i}"),
                formula: random_formula(&mut self.rng, &module),
            })
            .collect();
        module.properties = props;
        module.assumptions = assumes;

        module
    }
}

// ---------------------------------------------------------------------------
// Incremental mutation (subtask 3.1b, D-P6-11)
// ---------------------------------------------------------------------------

/// The mutation operations (recorded by [`mutate_traced`] for coverage tests).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MutOp {
    AddState,
    RemoveState,
    RenameState,
    AddEvent,
    RemoveEvent,
    RenameEvent,
    AddOutput,
    RemoveOutput,
    RenameOutput,
    AddTransition,
    RemoveTransition,
    TweakGuard,
    TweakPriority,
    TweakOnError,
    TweakPayloadWidth,
    AddProperty,
    RemoveProperty,
}

const ALL_OPS: [MutOp; 17] = [
    MutOp::AddState,
    MutOp::RemoveState,
    MutOp::RenameState,
    MutOp::AddEvent,
    MutOp::RemoveEvent,
    MutOp::RenameEvent,
    MutOp::AddOutput,
    MutOp::RemoveOutput,
    MutOp::RenameOutput,
    MutOp::AddTransition,
    MutOp::RemoveTransition,
    MutOp::TweakGuard,
    MutOp::TweakPriority,
    MutOp::TweakOnError,
    MutOp::TweakPayloadWidth,
    MutOp::AddProperty,
    MutOp::RemoveProperty,
];

/// Does the formula reference state variant `name` (bare single-segment path)?
fn formula_refs_state(f: &Formula, name: &str) -> bool {
    match f {
        Formula::Expr { expr } => expr_refs_path(expr, name),
        Formula::Not { inner }
        | Formula::Always { inner }
        | Formula::Next { inner }
        | Formula::BoundedAlways { inner, .. }
        | Formula::BoundedEventually { inner, .. }
        | Formula::Eventually { inner } => formula_refs_state(inner, name),
        Formula::And { left, right }
        | Formula::Or { left, right }
        | Formula::Implies { left, right }
        | Formula::Until { left, right } => {
            formula_refs_state(left, name) || formula_refs_state(right, name)
        }
        Formula::InputIs { .. } | Formula::OutputHas { .. } => false,
    }
}

fn expr_refs_path(e: &Expr, name: &str) -> bool {
    match e {
        Expr::Path { path } => path.len() == 1 && path[0] == name && name != "state",
        Expr::Unary { expr, .. } => expr_refs_path(expr, name),
        Expr::Binary { left, right, .. } => {
            expr_refs_path(left, name) || expr_refs_path(right, name)
        }
        _ => false,
    }
}

/// Rewrite bare state-variant references `old` -> `new` in a formula.
fn rewrite_formula_state(f: &mut Formula, old: &str, new: &str) {
    match f {
        Formula::Expr { expr } => rewrite_expr_path(expr, old, new),
        Formula::Not { inner }
        | Formula::Always { inner }
        | Formula::Next { inner }
        | Formula::BoundedAlways { inner, .. }
        | Formula::BoundedEventually { inner, .. }
        | Formula::Eventually { inner } => rewrite_formula_state(inner, old, new),
        Formula::And { left, right }
        | Formula::Or { left, right }
        | Formula::Implies { left, right }
        | Formula::Until { left, right } => {
            rewrite_formula_state(left, old, new);
            rewrite_formula_state(right, old, new);
        }
        Formula::InputIs { .. } | Formula::OutputHas { .. } => {}
    }
}

fn rewrite_expr_path(e: &mut Expr, old: &str, new: &str) {
    match e {
        Expr::Path { path } if path.len() == 1 && path[0] == old => *path = vec![new.to_string()],
        Expr::Unary { expr, .. } => rewrite_expr_path(expr, old, new),
        Expr::Binary { left, right, .. } => {
            rewrite_expr_path(left, old, new);
            rewrite_expr_path(right, old, new);
        }
        _ => {}
    }
}

/// Does the formula reference event group `g` (via input_is)?
fn formula_refs_event(f: &Formula, g: &str) -> bool {
    match f {
        Formula::InputIs { pattern } => pattern.group == g,
        Formula::Not { inner }
        | Formula::Always { inner }
        | Formula::Next { inner }
        | Formula::BoundedAlways { inner, .. }
        | Formula::BoundedEventually { inner, .. }
        | Formula::Eventually { inner } => formula_refs_event(inner, g),
        Formula::And { left, right }
        | Formula::Or { left, right }
        | Formula::Implies { left, right }
        | Formula::Until { left, right } => {
            formula_refs_event(left, g) || formula_refs_event(right, g)
        }
        _ => false,
    }
}

fn rewrite_formula_event(f: &mut Formula, old: &str, new: &str) {
    match f {
        Formula::InputIs { pattern } if pattern.group == old => pattern.group = new.to_string(),
        Formula::Not { inner }
        | Formula::Always { inner }
        | Formula::Next { inner }
        | Formula::BoundedAlways { inner, .. }
        | Formula::BoundedEventually { inner, .. }
        | Formula::Eventually { inner } => rewrite_formula_event(inner, old, new),
        Formula::And { left, right }
        | Formula::Or { left, right }
        | Formula::Implies { left, right }
        | Formula::Until { left, right } => {
            rewrite_formula_event(left, old, new);
            rewrite_formula_event(right, old, new);
        }
        _ => {}
    }
}

/// Does the formula reference output group `g` (via output_has)?
fn formula_refs_output(f: &Formula, g: &str) -> bool {
    match f {
        Formula::OutputHas { pattern } => pattern.group == g,
        Formula::Not { inner }
        | Formula::Always { inner }
        | Formula::Next { inner }
        | Formula::BoundedAlways { inner, .. }
        | Formula::BoundedEventually { inner, .. }
        | Formula::Eventually { inner } => formula_refs_output(inner, g),
        Formula::And { left, right }
        | Formula::Or { left, right }
        | Formula::Implies { left, right }
        | Formula::Until { left, right } => {
            formula_refs_output(left, g) || formula_refs_output(right, g)
        }
        _ => false,
    }
}

fn rewrite_formula_output(f: &mut Formula, old: &str, new: &str) {
    match f {
        Formula::OutputHas { pattern } if pattern.group == old => pattern.group = new.to_string(),
        Formula::Not { inner }
        | Formula::Always { inner }
        | Formula::Next { inner }
        | Formula::BoundedAlways { inner, .. }
        | Formula::BoundedEventually { inner, .. }
        | Formula::Eventually { inner } => rewrite_formula_output(inner, old, new),
        Formula::And { left, right }
        | Formula::Or { left, right }
        | Formula::Implies { left, right }
        | Formula::Until { left, right } => {
            rewrite_formula_output(left, old, new);
            rewrite_formula_output(right, old, new);
        }
        _ => {}
    }
}

/// The typing environment for guard/assignment expressions: state variants,
/// user enums (name -> variants), module vars and pattern bindings.
struct Env {
    states: Vec<String>,
    enum_variants: Vec<(String, Vec<String>)>,
    vars: Vec<(String, Type)>,
    binds: Vec<(String, Type)>,
}

/// The State enum's variants (empty if the module has none).
fn state_variants(module: &Module) -> &[String] {
    module
        .enums
        .iter()
        .find(|e| e.name == "State")
        .map(|e| e.variants.as_slice())
        .unwrap_or_default()
}

/// User-defined enums (everything except the reserved `State`) with their
/// variant names - the same list the formula generators consume.
fn user_enum_variants(module: &Module) -> Vec<(String, Vec<String>)> {
    module
        .enums
        .iter()
        .filter(|e| e.name != "State")
        .map(|e| (e.name.clone(), e.variants.clone()))
        .collect()
}

/// The expression environment of one transition's pattern (bindings + types).
fn transition_env(module: &Module, t: &Transition) -> Env {
    let states = state_variants(module).to_vec();
    let enum_variants = user_enum_variants(module);
    let vars: Vec<(String, Type)> = module
        .vars
        .iter()
        .map(|v| (v.name.clone(), v.ty.clone()))
        .collect();
    let mut binds: Vec<(String, Type)> = Vec::new();
    if let Some(PatternVariant::Struct { fields, .. }) = &t.pattern.variant {
        let decl: &[FieldDecl] = module
            .events
            .iter()
            .find(|g| g.name == t.pattern.group)
            .and_then(|g| {
                g.variants.iter().find_map(|v| match v {
                    ConstructorVariant::Struct { fields, .. } => Some(fields.as_slice()),
                    _ => None,
                })
            })
            .unwrap_or(&[]);
        for (pf, fv) in fields.iter().zip(decl) {
            if let PatternExpr::Bind { name } = &pf.pattern {
                binds.push((name.clone(), fv.ty.clone()));
            }
        }
    }
    Env {
        states,
        enum_variants,
        vars,
        binds,
    }
}

// ---------------------------------------------------------------------------
// Mutation operations (one candidate each; validity checked by the caller)
// ---------------------------------------------------------------------------

/// A fresh identifier not present in `taken`: `prefix` + smallest unused index.
/// Deterministic (no RNG draw), so mutation sequences stay reproducible.
fn fresh_name(taken: &[String], prefix: &str) -> String {
    let mut i = 0;
    loop {
        let n = format!("{prefix}{i}");
        if !taken.contains(&n) {
            return n;
        }
        i += 1;
    }
}

fn state_names(module: &Module) -> Vec<String> {
    state_variants(module).to_vec()
}

fn event_group_names(module: &Module) -> Vec<String> {
    module.events.iter().map(|g| g.name.clone()).collect()
}

fn output_group_names(module: &Module) -> Vec<String> {
    module.outputs.iter().map(|g| g.name.clone()).collect()
}

/// Every event variant name (groups are renamed atomically, but variant names
/// must stay unique across the module for codegen).
fn all_event_variant_names(module: &Module) -> Vec<String> {
    module
        .events
        .iter()
        .flat_map(|g| {
            g.variants
                .iter()
                .map(|v| variant_struct_name(v).to_string())
        })
        .collect()
}

fn all_output_variant_names(module: &Module) -> Vec<String> {
    module
        .outputs
        .iter()
        .flat_map(|g| {
            g.variants
                .iter()
                .map(|v| variant_struct_name(v).to_string())
        })
        .collect()
}

fn any_formula_refs_state(module: &Module, name: &str) -> bool {
    module
        .properties
        .iter()
        .chain(module.assumptions.iter())
        .any(|p| formula_refs_state(&p.formula, name))
}

fn rewrite_all_formulas_state(module: &mut Module, old: &str, new: &str) {
    for p in module
        .properties
        .iter_mut()
        .chain(module.assumptions.iter_mut())
    {
        rewrite_formula_state(&mut p.formula, old, new);
    }
}

fn any_formula_refs_event(module: &Module, g: &str) -> bool {
    module
        .properties
        .iter()
        .chain(module.assumptions.iter())
        .any(|p| formula_refs_event(&p.formula, g))
}

fn rewrite_all_formulas_event(module: &mut Module, old: &str, new: &str) {
    for p in module
        .properties
        .iter_mut()
        .chain(module.assumptions.iter_mut())
    {
        rewrite_formula_event(&mut p.formula, old, new);
    }
}

fn any_formula_refs_output(module: &Module, g: &str) -> bool {
    module
        .properties
        .iter()
        .chain(module.assumptions.iter())
        .any(|p| formula_refs_output(&p.formula, g))
}

fn rewrite_all_formulas_output(module: &mut Module, old: &str, new: &str) {
    for p in module
        .properties
        .iter_mut()
        .chain(module.assumptions.iter_mut())
    {
        rewrite_formula_output(&mut p.formula, old, new);
    }
}

/// Renumber transition ids sequentially (ids are positional metadata).
fn renumber_transitions(module: &mut Module) {
    for (i, t) in module.transitions.iter_mut().enumerate() {
        t.id = i;
    }
}

/// Apply ONE candidate mutation to `module` IN PLACE. Returns false when the
/// operation is infeasible on this module shape (nothing was changed); the
/// caller validates the result and rolls back on failure.
fn apply_op(module: &mut Module, op: MutOp, rng: &mut Rng) -> bool {
    match op {
        // ---- states -------------------------------------------------------
        MutOp::AddState => {
            let se = match module.enums.iter_mut().find(|e| e.name == "State") {
                Some(e) => e,
                None => return false,
            };
            let name = fresh_name(&se.variants, "S");
            se.variants.push(name.clone());
            // Keep the new state reachable: one extra transition into it.
            if !module.events.is_empty() {
                let states = state_names(module);
                let src = states[rng.range(0, states.len())].clone();
                let mut used: HashSet<u32> = module
                    .transitions
                    .iter()
                    .filter(|t| t.state == src)
                    .filter_map(|t| t.priority)
                    .collect();
                module
                    .transitions
                    .push(random_transition(rng, module, &src, &name, &mut used));
                renumber_transitions(module);
            }
            true
        }
        MutOp::RemoveState => {
            let initial = module.initial_state.clone();
            let variants = state_names(module);
            if variants.len() <= 1 {
                return false;
            }
            // Only states referenced by nothing may go (validator re-checks).
            let free: Vec<String> = variants
                .into_iter()
                .filter(|s| Some(s) != initial.as_ref())
                .filter(|s| {
                    !module
                        .transitions
                        .iter()
                        .any(|t| &t.state == s || &t.next_state == s)
                })
                .filter(|s| {
                    module
                        .on_error
                        .as_ref()
                        .map(|o| &o.next_state != s)
                        .unwrap_or(true)
                })
                .filter(|s| !any_formula_refs_state(module, s))
                .collect();
            if free.is_empty() {
                return false;
            }
            let name = free[rng.range(0, free.len())].clone();
            if let Some(se) = module.enums.iter_mut().find(|e| e.name == "State") {
                se.variants.retain(|v| v != &name);
            }
            true
        }
        MutOp::RenameState => {
            let states = state_names(module);
            if states.is_empty() {
                return false;
            }
            let old = states[rng.range(0, states.len())].clone();
            let new = fresh_name(&states, "RN");
            if let Some(se) = module.enums.iter_mut().find(|e| e.name == "State") {
                *se.variants.iter_mut().find(|v| **v == old).unwrap() = new.clone();
            }
            if module.initial_state.as_deref() == Some(old.as_str()) {
                module.initial_state = Some(new.clone());
            }
            for t in module.transitions.iter_mut() {
                if t.state == old {
                    t.state = new.clone();
                }
                if t.next_state == old {
                    t.next_state = new.clone();
                }
            }
            if let Some(o) = module.on_error.as_mut() {
                if o.next_state == old {
                    o.next_state = new.clone();
                }
            }
            rewrite_all_formulas_state(module, &old, &new);
            true
        }

        // ---- events -------------------------------------------------------
        MutOp::AddEvent => {
            let name = fresh_name(&event_group_names(module), "E");
            let vname = fresh_name(&all_event_variant_names(module), "EV");
            module.events.push(ConstructorGroup {
                name,
                variants: vec![ConstructorVariant::Unit { name: vname }],
            });
            true
        }
        MutOp::RemoveEvent => {
            if module.events.len() <= 1 {
                return false;
            }
            let free: Vec<String> = event_group_names(module)
                .into_iter()
                .filter(|g| !module.transitions.iter().any(|t| t.pattern.group == *g))
                .filter(|g| !any_formula_refs_event(module, g))
                .collect();
            if free.is_empty() {
                return false;
            }
            let name = free[rng.range(0, free.len())].clone();
            module.events.retain(|g| g.name != name);
            true
        }
        MutOp::RenameEvent => {
            let names = event_group_names(module);
            if names.is_empty() {
                return false;
            }
            let old = names[rng.range(0, names.len())].clone();
            let new = fresh_name(&names, "ER");
            if let Some(g) = module.events.iter_mut().find(|g| g.name == old) {
                g.name = new.clone();
            }
            for t in module.transitions.iter_mut() {
                if t.pattern.group == old {
                    t.pattern.group = new.clone();
                }
            }
            rewrite_all_formulas_event(module, &old, &new);
            true
        }

        // ---- outputs ------------------------------------------------------
        MutOp::AddOutput => {
            let name = fresh_name(&output_group_names(module), "O");
            let vname = fresh_name(&all_output_variant_names(module), "OV");
            module.outputs.push(ConstructorGroup {
                name,
                variants: vec![ConstructorVariant::Unit { name: vname }],
            });
            true
        }
        MutOp::RemoveOutput => {
            if module.outputs.len() <= 1 {
                return false;
            }
            let free: Vec<String> = output_group_names(module)
                .into_iter()
                .filter(|g| {
                    !module
                        .transitions
                        .iter()
                        .any(|t| t.emits.iter().any(|e| e.group == *g))
                })
                .filter(|g| {
                    module
                        .on_error
                        .as_ref()
                        .map(|o| !o.emits.iter().any(|e| e.group == *g))
                        .unwrap_or(true)
                })
                .filter(|g| !any_formula_refs_output(module, g))
                .collect();
            if free.is_empty() {
                return false;
            }
            let name = free[rng.range(0, free.len())].clone();
            module.outputs.retain(|g| g.name != name);
            true
        }
        MutOp::RenameOutput => {
            let names = output_group_names(module);
            if names.is_empty() {
                return false;
            }
            let old = names[rng.range(0, names.len())].clone();
            let new = fresh_name(&names, "OR");
            if let Some(g) = module.outputs.iter_mut().find(|g| g.name == old) {
                g.name = new.clone();
            }
            for t in module.transitions.iter_mut() {
                for e in t.emits.iter_mut() {
                    if e.group == old {
                        e.group = new.clone();
                    }
                }
            }
            if let Some(o) = module.on_error.as_mut() {
                for e in o.emits.iter_mut() {
                    if e.group == old {
                        e.group = new.clone();
                    }
                }
            }
            rewrite_all_formulas_output(module, &old, &new);
            true
        }

        // ---- transitions --------------------------------------------------
        MutOp::AddTransition => {
            let states = state_names(module);
            if states.is_empty() || module.events.is_empty() {
                return false;
            }
            let src = states[rng.range(0, states.len())].clone();
            let dst = states[rng.range(0, states.len())].clone();
            let mut used: HashSet<u32> = module
                .transitions
                .iter()
                .filter(|t| t.state == src)
                .filter_map(|t| t.priority)
                .collect();
            module
                .transitions
                .push(random_transition(rng, module, &src, &dst, &mut used));
            renumber_transitions(module);
            true
        }
        MutOp::RemoveTransition => {
            if module.transitions.is_empty() {
                return false;
            }
            module
                .transitions
                .remove(rng.range(0, module.transitions.len()));
            renumber_transitions(module);
            true
        }

        // ---- tweaks -------------------------------------------------------
        MutOp::TweakGuard => {
            if module.transitions.is_empty() {
                return false;
            }
            let i = rng.range(0, module.transitions.len());
            let env = transition_env(module, &module.transitions[i]);
            module.transitions[i].guard = Some(expr_bool(
                rng,
                &env.states,
                &env.enum_variants,
                &env.vars,
                &env.binds,
            ));
            true
        }
        MutOp::TweakPriority => {
            if module.transitions.is_empty() {
                return false;
            }
            let i = rng.range(0, module.transitions.len());
            let src = module.transitions[i].state.clone();
            let used: HashSet<u32> = module
                .transitions
                .iter()
                .enumerate()
                .filter(|(j, _)| *j != i)
                .filter(|(_, t)| t.state == src)
                .filter_map(|(_, t)| t.priority)
                .collect();
            // A priority distinct from every sibling's (validator: duplicates
            // on the same pattern are rejected).
            let mut p = rng.range(0, 8) as u32;
            while used.contains(&p) {
                p += 1;
            }
            module.transitions[i].priority = Some(p);
            true
        }
        MutOp::TweakOnError => match &module.on_error {
            None => {
                let states = state_names(module);
                if states.is_empty() {
                    return false;
                }
                let mut emits = Vec::new();
                // At most one unit-output emit (no bindings in scope).
                if !module.outputs.is_empty() && rng.chance(0.5) {
                    let g = &module.outputs[rng.range(0, module.outputs.len())];
                    if let Some(u) = g
                        .variants
                        .iter()
                        .find(|v| matches!(v, ConstructorVariant::Unit { .. }))
                    {
                        emits.push(OutputExpr {
                            group: g.name.clone(),
                            variant: Some(variant_struct_name(u).to_string()),
                            args: vec![],
                        });
                    }
                }
                module.on_error = Some(OnError {
                    next_state: states[rng.range(0, states.len())].clone(),
                    emits,
                });
                true
            }
            Some(_) => {
                if rng.chance(0.5) {
                    let states = state_names(module);
                    if states.is_empty() {
                        return false;
                    }
                    module.on_error.as_mut().unwrap().next_state =
                        states[rng.range(0, states.len())].clone();
                    true
                } else {
                    module.on_error = None;
                    true
                }
            }
        },
        MutOp::TweakPayloadWidth => {
            // Collect (group-slot, variant-index, field-index) of every bit<N>
            // payload field across events and outputs.
            let mut spots: Vec<(bool, usize, usize, usize)> = Vec::new();
            for (is_out, groups) in [(false, &module.events), (true, &module.outputs)] {
                for (gi, g) in groups.iter().enumerate() {
                    for (vi, v) in g.variants.iter().enumerate() {
                        if let ConstructorVariant::Struct { fields, .. } = v {
                            for (fi, f) in fields.iter().enumerate() {
                                if matches!(f.ty, Type::Bit { .. }) {
                                    spots.push((is_out, gi, vi, fi));
                                }
                            }
                        }
                    }
                }
            }
            if spots.is_empty() {
                return false;
            }
            let (is_out, gi, vi, fi) = spots[rng.range(0, spots.len())];
            let groups = if is_out {
                &mut module.outputs
            } else {
                &mut module.events
            };
            let fields = match &mut groups[gi].variants[vi] {
                ConstructorVariant::Struct { fields, .. } => fields,
                _ => unreachable!("spot collected from a struct variant"),
            };
            // New width in 1..=64 (the validator's literal range), different
            // from the current one.
            let cur = match &fields[fi].ty {
                Type::Bit { width } => *width,
                _ => unreachable!(),
            };
            let mut w = rng.range(1, 65) as u32;
            if w == cur {
                w = (w % 64) + 1;
            }
            fields[fi].ty = Type::Bit { width: w };
            true
        }

        // ---- properties ---------------------------------------------------
        MutOp::AddProperty => {
            let taken: Vec<String> = module
                .properties
                .iter()
                .chain(module.assumptions.iter())
                .map(|p| p.name.clone())
                .collect();
            let name = fresh_name(&taken, "p");
            module.properties.push(Property {
                name,
                formula: random_formula(rng, module),
            });
            true
        }
        MutOp::RemoveProperty => {
            let n = module.properties.len() + module.assumptions.len();
            if n == 0 {
                return false;
            }
            let pick = rng.range(0, n);
            if pick < module.properties.len() {
                module.properties.remove(pick);
            } else {
                module.assumptions.remove(pick - module.properties.len());
            }
            true
        }
    }
}

// ---------------------------------------------------------------------------
// Public mutation API (D-P6-11)
// ---------------------------------------------------------------------------

/// Bounded redraw attempts per step; on exhaustion the step is SKIPPED
/// (documented behavior: a shape where no candidate of the drawn op passes
/// validation simply does not consume one of the K steps).
const MAX_RETRIES_PER_STEP: usize = 100;

/// Apply K seeded mutation steps to the FIRST module of `base_ir` (other
/// modules pass through unchanged - RFS modules are independent). Returns the
/// mutated IR plus the list of ops actually applied (steps that exhausted all
/// redraw attempts are skipped and absent from the list, so the list length is
/// <= K). `k = 0` is the identity: the result renders byte-identically to the
/// input. Requires a VALID base module (the CLI validates before calling).
pub fn mutate_traced(base_ir: &Ir, rng: &mut Rng, k: usize) -> (Ir, Vec<MutOp>) {
    let mut ir = base_ir.clone();
    if ir.modules.is_empty() {
        return (ir, Vec::new());
    }
    let mut applied: Vec<MutOp> = Vec::new();
    for _ in 0..k {
        let mut committed = false;
        for _attempt in 0..MAX_RETRIES_PER_STEP {
            let op = ALL_OPS[rng.range(0, ALL_OPS.len())];
            let mut cand = ir.modules[0].clone();
            if apply_op(&mut cand, op, rng) && validator::validate_module(&cand).is_empty() {
                ir.modules[0] = cand;
                applied.push(op);
                committed = true;
                break;
            }
        }
        if !committed {
            // All 100 candidates rejected - skip this step and continue.
        }
    }
    (ir, applied)
}

/// [`mutate_traced`] without the op trace.
pub fn mutate(base_ir: &Ir, rng: &mut Rng, k: usize) -> Ir {
    mutate_traced(base_ir, rng, k).0
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser;

    /// Render + re-parse + validate one generated spec (all three must hold).
    fn roundtrip_gen(gen: &mut SpecGen) -> String {
        let ir = gen.generate();
        let src = crate::printer::print_ir(&ir);
        let reparsed = parser::parse_source(&src).expect("generated source must parse");
        validator::validate_ir(&reparsed).expect("generated spec must validate");
        src
    }

    #[test]
    fn seed_determinism_same_seed_byte_identical() {
        for seed in [0u64, 1, 7, 42, 12345, u64::MAX] {
            let a = roundtrip_gen(&mut SpecGen::new(seed, Bounds::small()));
            let b = roundtrip_gen(&mut SpecGen::new(seed, Bounds::small()));
            assert_eq!(a, b, "seed {seed} must render byte-identically");
        }
    }

    #[test]
    fn different_seeds_differ() {
        let a = roundtrip_gen(&mut SpecGen::new(1, Bounds::small()));
        let b = roundtrip_gen(&mut SpecGen::new(2, Bounds::small()));
        assert_ne!(a, b, "adjacent seeds should produce different specs");
    }

    #[test]
    fn generated_specs_parse_validate_and_respect_bounds() {
        // 50 seeds x both presets (exit criterion: 50+ generated specs).
        for preset in [&Bounds::small(), &Bounds::medium()] {
            for seed in 0..50u64 {
                let mut gen = SpecGen::new(seed, (*preset).clone());
                let ir = gen.generate();
                let src = crate::printer::print_ir(&ir);
                let reparsed = parser::parse_source(&src)
                    .unwrap_or_else(|e| panic!("seed {seed} failed to parse: {e:?}"));
                validator::validate_ir(&reparsed)
                    .unwrap_or_else(|e| panic!("seed {seed} invalid: {e:?}"));
                let m = &reparsed.modules[0];
                let states = state_variants(m).len();
                assert!(
                    states <= preset.max_states,
                    "seed {seed}: {states} states > cap"
                );
                assert!(!m.events.is_empty() && m.events.len() <= preset.max_events);
                assert!(m.vars.len() <= preset.max_vars);
                assert!(m.transitions.len() <= preset.max_transitions);
                assert!(m.properties.len() + m.assumptions.len() <= preset.max_props);
            }
        }
    }

    #[test]
    fn mutated_k_zero_is_identity() {
        let base = roundtrip_gen(&mut SpecGen::new(3, Bounds::small()));
        let ir = parser::parse_source(&base).unwrap();
        for seed in [0u64, 5, 99] {
            let mut rng = Rng::new(seed);
            let out = mutate(&ir, &mut rng, 0);
            assert_eq!(
                crate::printer::print_ir(&out),
                base,
                "k=0 must render byte-identically to the input"
            );
        }
    }

    #[test]
    fn mutation_per_step_validity_k_up_to_20() {
        // Per-step validity invariant: every intermediate result of a k-step
        // mutation sequence validates (mutate only commits validated steps, so
        // the final result must validate for every k in 1..=20).
        let base_src = roundtrip_gen(&mut SpecGen::new(11, Bounds::small()));
        let base = parser::parse_source(&base_src).unwrap();
        for seed in [0u64, 1, 2] {
            for k in 1..=20usize {
                let mut rng = Rng::new(seed);
                let (out, applied) = mutate_traced(&base, &mut rng, k);
                validator::validate_ir(&out)
                    .unwrap_or_else(|e| panic!("seed {seed} k={k} invalid: {e:?}"));
                assert!(applied.len() <= k);
            }
        }
    }

    #[test]
    fn mutation_determinism_same_seed_byte_identical() {
        let base_src = roundtrip_gen(&mut SpecGen::new(13, Bounds::small()));
        let base = parser::parse_source(&base_src).unwrap();
        for seed in [0u64, 7, 2024] {
            let a = {
                let mut rng = Rng::new(seed);
                crate::printer::print_ir(&mutate(&base, &mut rng, 15))
            };
            let b = {
                let mut rng = Rng::new(seed);
                crate::printer::print_ir(&mutate(&base, &mut rng, 15))
            };
            assert_eq!(a, b, "mutation must be deterministic from the seed");
        }
    }

    #[test]
    fn mutation_coverage_exercises_adds_and_removes() {
        // Over many seeds the applied ops must include BOTH add and remove
        // operations (guards against a degenerate single-op RNG path).
        let base_src = roundtrip_gen(&mut SpecGen::new(21, Bounds::medium()));
        let base = parser::parse_source(&base_src).unwrap();
        let mut seen: std::collections::HashSet<MutOp> = std::collections::HashSet::new();
        for seed in 0..30u64 {
            let mut rng = Rng::new(seed);
            let (_, applied) = mutate_traced(&base, &mut rng, 10);
            for op in applied {
                seen.insert(op);
            }
        }
        let adds: Vec<MutOp> = seen
            .iter()
            .copied()
            .filter(|o| {
                matches!(
                    o,
                    MutOp::AddState
                        | MutOp::AddEvent
                        | MutOp::AddOutput
                        | MutOp::AddTransition
                        | MutOp::AddProperty
                )
            })
            .collect();
        let removes: Vec<MutOp> = seen
            .iter()
            .copied()
            .filter(|o| {
                matches!(
                    o,
                    MutOp::RemoveState
                        | MutOp::RemoveEvent
                        | MutOp::RemoveOutput
                        | MutOp::RemoveTransition
                        | MutOp::RemoveProperty
                )
            })
            .collect();
        assert!(!adds.is_empty(), "no add op was ever applied: {seen:?}");
        assert!(
            !removes.is_empty(),
            "no remove op was ever applied: {seen:?}"
        );
    }

    #[test]
    fn mutation_grows_state_count_over_seeds() {
        // Over a batch of seeds x long sequences AddState must land at least
        // once (the base has fewer states than the small cap).
        let base_src = roundtrip_gen(&mut SpecGen::new(29, Bounds::small()));
        let base = parser::parse_source(&base_src).unwrap();
        let n0 = state_variants(&base.modules[0]).len();
        let mut max_n = 0;
        for seed in 0..12u64 {
            let mut rng = Rng::new(seed);
            let (out, applied) = mutate_traced(&base, &mut rng, 60);
            validator::validate_ir(&out).expect("mutated spec must validate");
            assert!(applied.contains(&MutOp::AddState));
            max_n = max_n.max(state_variants(&out.modules[0]).len());
        }
        assert!(max_n > n0, "state count never grew (n0={n0}, max={max_n})");
    }

    /// RemoveState semantics: removes exactly the unreferenced state and is
    /// infeasible (returns false, module untouched) when every state carries
    /// a reference.
    #[test]
    fn apply_op_remove_state_deterministic() {
        // C is referenced by nothing - removable.
        let src = "module m {\n  enum State { A, B, C }\n  initial A\n  event E(U)\n  output O(V)\n  transition A on E(U) { next B; }\n  prop p : always (state != B) ;\n}\n";
        let mut ir = parser::parse_source(src).unwrap();
        let mut rng = Rng::new(1);
        assert!(apply_op(&mut ir.modules[0], MutOp::RemoveState, &mut rng));
        validator::validate_ir(&ir).expect("post-removal spec must validate");
        assert_eq!(
            state_variants(&ir.modules[0]),
            vec!["A".to_string(), "B".to_string()]
        );

        // Every state referenced (initial + transition targets) - infeasible.
        let src = "module m {\n  enum State { A, B, C }\n  initial A\n  event E(U)\n  output O(V)\n  transition A on E(U) { next B; }\n  transition B on E(U) { next C; }\n}\n";
        let mut ir = parser::parse_source(src).unwrap();
        let before = crate::printer::print_ir(&ir);
        let mut rng = Rng::new(1);
        assert!(!apply_op(&mut ir.modules[0], MutOp::RemoveState, &mut rng));
        assert_eq!(
            crate::printer::print_ir(&ir),
            before,
            "infeasible op must not modify the module"
        );
    }

    /// Byte pin: seed 1 with the `small` preset renders EXACTLY this source.
    /// Re-pin deliberately only when the generator changes on purpose.
    const PIN_SEED_1_SMALL: &str = r#"module gen {
  enum State { A, B, C, D }
  initial A
  event E0(S0(f0: bool))
  event E0(U1)
  event E0(S2(f0: bool, f1: bit<4>))
  event E1(S0(f0: bit<4>))
  event E2(U0)
  event E2(U1)
  output O0(U0)
  var v0: bit<4> init 6
  var v1: bit<2> init 0
  transition A on E1(S0(b0_0)) {
    next B;
    set v0 = v0;
    set v1 = v1;
  }
  transition B on E1(S0(b1_0)) where true {
    next C;
    emit O0(U0);
    set v0 = v0;
  }
  transition C on E1(S0(b2_0)) {
    next D;
    emit O0(U0);
    set v1 = v1;
    set v0 = 6;
  }
  transition C on E2(U1) {
    next B;
    set v1 = 0;
    set v0 = v0;
  }
  transition A on E2(U1) {
    next D;
    emit O0(U0);
    emit O0(U0);
    set v1 = 1;
    set v0 = v0;
    priority 19;
  }
  transition B on E2(U0) where state == C && state != D && state == A {
    next A;
    set v0 = v0;
  }
  transition C on E2(U1) {
    next B;
    priority 44;
  }
  transition C on E2(U1) where state != B {
    next B;
    emit O0(U0);
    emit O0(U0);
    set v1 = 1;
    priority 35;
  }
  transition B on E2(U0) where state == B {
    next A;
    set v0 = 1;
    set v1 = v1;
    priority 83;
  }
  transition B on E2(U0) {
    next A;
    emit O0(U0);
    emit O0(U0);
    set v0 = 2;
    set v1 = v1;
    priority 5;
  }
  transition D on E0(U1) where state != C {
    next C;
    emit O0(U0);
    set v0 = v0;
    priority 13;
  }
  transition D on E0(S2(b11_0, _)) where b11_0 {
    next C;
    emit O0(U0);
    set v0 = v0;
    set v1 = v1;
    priority 4;
  }
  on_error {
    next A;
    emit O0(U0);
  }
  prop p0 : v1 == 1 && v0 < 6 ;
  prop p1 : eventually (input_is(E1(S0(_)))) ;
}
"#;

    #[test]
    fn generated_source_byte_pin_seed_one_small() {
        let gen = SpecGen::new(1, Bounds::small()).generate();
        assert_eq!(crate::printer::print_ir(&gen), PIN_SEED_1_SMALL);
    }
}
