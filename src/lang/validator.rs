//! Semantic validation and expression typing for RFS IR.
//! Docs: dev:validator-overview, dev:validator-fatal-pass, dev:validator-typing, dev:validator-warnings, dev:deps-fn-validator, dev:algorithms-parser-validation, llm:validator

use crate::lang::fold::{const_fit, fold_const};
use crate::lang::ir::*;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationError {
    pub message: String,
}

impl ValidationError {
    fn new<T: Into<String>>(message: T) -> Self {
        Self {
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExprType {
    Bool,
    Bit(u32),
    IntLit,
    Enum(String),
}

#[derive(Debug, Clone)]
pub struct Env {
    pub vars: HashMap<String, ExprType>,
    pub bindings: HashMap<String, ExprType>,
    /// RFS binding name -> the identifier actually emitted in the target
    /// language. Present only when codegen had to MANGLE a binding because
    /// it collided with an identifier used unqualified by the generated
    /// transition body (a shadowing bug class - see the backends' binding
    /// helpers). Empty means every binding is emitted under its plain name.
    pub binding_idents: HashMap<String, String>,
    pub state_variants: HashSet<String>,
    pub enums: HashMap<String, HashSet<String>>,
}

impl Env {
    pub fn for_module(module: &Module) -> Self {
        let mut vars = HashMap::new();
        for v in &module.vars {
            vars.insert(v.name.clone(), type_to_expr_type(&v.ty));
        }
        let mut state_variants = HashSet::new();
        let mut enums = HashMap::new();
        for e in &module.enums {
            let set: HashSet<String> = e.variants.iter().cloned().collect();
            if e.name == "State" {
                state_variants = set.clone();
            }
            enums.insert(e.name.clone(), set);
        }
        Self {
            vars,
            bindings: HashMap::new(),
            binding_idents: HashMap::new(),
            state_variants,
            enums,
        }
    }

    pub fn with_binding(mut self, name: String, ty: ExprType) -> Self {
        self.bindings.insert(name, ty);
        self
    }
}

pub fn type_to_expr_type(ty: &Type) -> ExprType {
    match ty {
        Type::Bool => ExprType::Bool,
        Type::Bit { width } => ExprType::Bit(*width),
        Type::Enum { name } => ExprType::Enum(name.clone()),
    }
}

pub fn validate_ir(ir: &Ir) -> Result<(), Vec<ValidationError>> {
    let mut errors = Vec::new();
    for module in &ir.modules {
        errors.extend(validate_module(module));
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// v1.0 syntax version header format (Phase 6, D-P6-1): `X.Y` or `X.Y.Z`
/// with non-empty all-digit components - the regex `^\d+\.\d+(\.\d+)?$`
/// checked without a dependency.
pub fn is_valid_version_format(v: &str) -> bool {
    let parts: Vec<&str> = v.split('.').collect();
    matches!(parts.len(), 2..=3)
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
}

pub fn validate_module(module: &Module) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    let env0 = Env::for_module(module);

    let state_variants: HashSet<String> = module
        .enums
        .iter()
        .find(|e| e.name == "State")
        .map(|e| e.variants.iter().cloned().collect())
        .unwrap_or_default();

    if state_variants.is_empty() {
        errors.push(ValidationError::new(format!(
            "module '{}' has no State enum variants",
            module.name
        )));
    }

    if let Some(init) = &module.initial_state {
        if !state_variants.contains(init) {
            errors.push(ValidationError::new(format!(
                "module '{}' initial state '{}' is not a State variant",
                module.name, init
            )));
        }
    } else if !module.transitions.is_empty() {
        errors.push(ValidationError::new(format!(
            "module '{}' has transitions but no initial state",
            module.name
        )));
    }

    // Phase 6 v1.0 (D-P6-1): formalized syntax version header - when present
    // it must match X.Y or X.Y.Z. The parser accepts any quoted string (old
    // files still PARSE); this is a semantic error, consistent with the other
    // module-level checks. Specs without a header are legacy v0.x and remain
    // valid.
    if let Some(v) = &module.version {
        if !is_valid_version_format(v) {
            errors.push(ValidationError::new(format!(
                "module '{}' invalid version header '{}' (expected X.Y or X.Y.Z)",
                module.name, v
            )));
        }
    }

    for v in &module.vars {
        let mut warns = Vec::new();
        match type_expr(&v.init, &env0, &mut warns) {
            Err(e) => errors.push(ValidationError::new(format!(
                "module '{}' var '{}' init: {}",
                module.name, v.name, e
            ))),
            Ok(ty) => {
                // Rule 4 / requirement 5 (task 3L): a constant-expression init
                // must fit the var's declared width - both MC modes now reject
                // identically at validation (the task-3H divergence is gone).
                if matches!(ty, ExprType::IntLit) {
                    if let Type::Bit { width } = &v.ty {
                        match fold_const(&v.init) {
                            Ok(c) => {
                                if let Err(e) = const_fit(c, *width, &mut warns) {
                                    errors.push(ValidationError::new(format!(
                                        "module '{}' var '{}' init: {}",
                                        module.name, v.name, e
                                    )));
                                }
                            }
                            Err(e) => errors.push(ValidationError::new(format!(
                                "module '{}' var '{}' init: {}",
                                module.name, v.name, e
                            ))),
                        }
                    }
                }
            }
        }
    }

    for (idx, t) in module.transitions.iter().enumerate() {
        let label = format!("module '{}' transition #{}", module.name, idx);
        if !state_variants.contains(&t.state) {
            errors.push(ValidationError::new(format!(
                "{label}: state '{}' is not a State variant",
                t.state
            )));
        }
        if !state_variants.contains(&t.next_state) {
            errors.push(ValidationError::new(format!(
                "{label}: next state '{}' is not a State variant",
                t.next_state
            )));
        }

        let mut env = env0.clone();
        if let Err(e) = validate_event_pattern(&t.pattern, module, &mut env) {
            errors.push(ValidationError::new(format!("{label}: event pattern: {e}")));
        }

        if let Some(guard) = &t.guard {
            if let Err(e) = type_expr(guard, &env, &mut Vec::new()) {
                errors.push(ValidationError::new(format!("{label}: guard: {e}")));
            } else if let Err(e) =
                type_expr(guard, &env, &mut Vec::new()).and_then(|ty| ty.must_bool("guard"))
            {
                errors.push(ValidationError::new(format!(
                    "{label}: guard must be bool: {e}"
                )));
            }
        }

        for a in &t.assignments {
            match env.vars.get(&a.var) {
                Some(_) => {}
                None => {
                    errors.push(ValidationError::new(format!(
                        "{label}: set unknown variable '{}'",
                        a.var
                    )));
                    continue;
                }
            }
            if let Err(e) = type_expr(&a.value, &env, &mut Vec::new()) {
                errors.push(ValidationError::new(format!(
                    "{label}: set '{}': {e}",
                    a.var
                )));
            }
        }

        for o in &t.emits {
            if let Err(e) = validate_output_expr(o, module, &env) {
                errors.push(ValidationError::new(format!("{label}: emit: {e}")));
            }
        }
    }

    // Phase 2: priority and ambiguity diagnostics (per state).
    let mut by_state: std::collections::BTreeMap<&str, Vec<&Transition>> =
        std::collections::BTreeMap::new();
    for t in &module.transitions {
        by_state.entry(&t.state).or_default().push(t);
    }
    for (state, ts) in &by_state {
        for i in 0..ts.len() {
            for j in (i + 1)..ts.len() {
                // Duplicate priorities are only a conflict when the two
                // transitions can match the same input (identical pattern);
                // numbers may be reused across different patterns.
                if ts[i].pattern == ts[j].pattern {
                    match (ts[i].priority, ts[j].priority) {
                        (Some(a), Some(b)) if a == b => {
                            errors.push(ValidationError::new(format!(
                                "module '{}' state '{}': duplicate priority {} for the same event pattern (transitions #{} and #{})",
                                module.name, state, a, ts[i].id, ts[j].id
                            )));
                        }
                        _ => {}
                    }
                }
                if ts[i].pattern == ts[j].pattern
                    && ts[i].priority.is_none()
                    && ts[j].priority.is_none()
                    && ts[i].guard == ts[j].guard
                {
                    // Identical pattern and identical guard (both absent or
                    // structurally equal): the second transition is shadowed.
                    errors.push(ValidationError::new(format!(
                        "module '{}' state '{}': duplicate transitions #{} and #{} - identical event pattern and guard; the second is unreachable",
                        module.name, state, ts[i].id, ts[j].id
                    )));
                }
            }
        }
    }

    // Phase 2: on_error block validation (no pattern bindings available).
    if let Some(oe) = &module.on_error {
        if !state_variants.contains(&oe.next_state) {
            errors.push(ValidationError::new(format!(
                "module '{}' on_error: next state '{}' is not a State variant",
                module.name, oe.next_state
            )));
        }
        for o in &oe.emits {
            if let Err(e) = validate_output_expr(o, module, &env0) {
                errors.push(ValidationError::new(format!(
                    "module '{}' on_error emit: {e}",
                    module.name
                )));
            }
        }
    }

    // Phase 3: duplicate prop/assume names would collide in generated monitor
    // struct names, so reject them early (D-P3-4).
    {
        let mut seen: HashSet<String> = HashSet::new();
        for p in module.properties.iter().chain(module.assumptions.iter()) {
            if !seen.insert(p.name.clone()) {
                errors.push(ValidationError::new(format!(
                    "module '{}' has duplicate property/assumption name '{}'",
                    module.name, p.name
                )));
            }
        }
    }

    for p in module.properties.iter().chain(module.assumptions.iter()) {
        if let Err(e) = type_formula(&p.formula, &env0, &mut Vec::new()) {
            errors.push(ValidationError::new(format!(
                "module '{}' property '{}': {}",
                module.name, p.name, e
            )));
        }
        if let Some(e) = bounded_always_error(&p.formula) {
            errors.push(ValidationError::new(format!(
                "module '{}' property '{}': {}",
                module.name, p.name, e
            )));
        }
    }

    errors
}

/// Phase 3 bound check (D-P3-2): `always[N]` requires N >= 1. Note that
/// `bounded_eventually[0]` stays legal and means "the body must hold at the
/// trigger sample itself".
fn bounded_always_error(f: &Formula) -> Option<String> {
    match f {
        Formula::BoundedAlways { bound, .. } if *bound < 1 => {
            Some("always[N] requires N >= 1".into())
        }
        Formula::Not { inner }
        | Formula::Next { inner }
        | Formula::Always { inner }
        | Formula::BoundedAlways { inner, .. }
        | Formula::BoundedEventually { inner, .. }
        | Formula::Eventually { inner } => bounded_always_error(inner),
        Formula::And { left, right }
        | Formula::Or { left, right }
        | Formula::Implies { left, right }
        | Formula::Until { left, right } => {
            bounded_always_error(left).or_else(|| bounded_always_error(right))
        }
        _ => None,
    }
}

fn pattern_variant_name(p: &PatternVariant) -> &str {
    match p {
        PatternVariant::Unit { name } => name,
        PatternVariant::Struct { name, .. } => name,
    }
}

fn validate_event_pattern(
    pattern: &EventPattern,
    module: &Module,
    env: &mut Env,
) -> Result<(), String> {
    let group = module
        .events
        .iter()
        .find(|g| g.name == pattern.group)
        .ok_or_else(|| format!("unknown event group '{}'", pattern.group))?;

    let Some(variant) = &pattern.variant else {
        return Ok(());
    };

    let vname = pattern_variant_name(variant).to_string();
    let decl = group
        .variants
        .iter()
        .find(|v| match v {
            ConstructorVariant::Unit { name } => name == &vname,
            ConstructorVariant::Struct { name, .. } => name == &vname,
        })
        .ok_or_else(|| {
            format!(
                "unknown event variant '{}' in group '{}'",
                vname, group.name
            )
        })?;

    match (decl, variant) {
        (ConstructorVariant::Unit { .. }, PatternVariant::Unit { .. }) => Ok(()),
        (
            ConstructorVariant::Struct { fields, .. },
            PatternVariant::Struct {
                fields: pfields, ..
            },
        ) => {
            if fields.len() != pfields.len() {
                return Err(format!(
                    "event variant '{}' has {} fields, pattern has {}",
                    vname,
                    fields.len(),
                    pfields.len()
                ));
            }
            let mut seen = HashSet::new();
            for (f, pf) in fields.iter().zip(pfields.iter()) {
                if let Some(name) = &pf.name {
                    if !seen.insert(name.clone()) {
                        return Err(format!("duplicate pattern binding '{}'", name));
                    }
                    env.bindings.insert(name.clone(), type_to_expr_type(&f.ty));
                }
                pattern_expr_compatible(&pf.pattern, &f.ty)
                    .map_err(|e| format!("field '{}': {}", f.name, e))?;
            }
            Ok(())
        }
        (ConstructorVariant::Unit { .. }, PatternVariant::Struct { .. }) => Err(format!(
            "event variant '{}' is unit but pattern has fields",
            vname
        )),
        (ConstructorVariant::Struct { .. }, PatternVariant::Unit { .. }) => Err(format!(
            "event variant '{}' has fields but pattern is unit",
            vname
        )),
    }
}

fn pattern_expr_compatible(pe: &PatternExpr, ty: &Type) -> Result<(), String> {
    match (pe, ty) {
        (PatternExpr::Wildcard, _) => Ok(()),
        (PatternExpr::Bind { .. }, _) => Ok(()),
        (PatternExpr::Int { value }, Type::Bit { width }) => {
            let max = if *width >= 64 {
                u64::MAX
            } else {
                (1u64 << width) - 1
            };
            if *value > max {
                Err(format!("literal {} does not fit bit<{}>", value, width))
            } else {
                Ok(())
            }
        }
        (PatternExpr::Int { .. }, Type::Bool) => Err("integer literal cannot match bool".into()),
        (PatternExpr::Int { .. }, Type::Enum { .. }) => {
            Err("integer literal cannot match enum".into())
        }
        (PatternExpr::Bool { value: _ }, Type::Bool) => Ok(()),
        (PatternExpr::Bool { .. }, _) => Err("boolean literal cannot match non-bool".into()),
        (PatternExpr::EnumVariant { path }, Type::Enum { name }) => {
            if path.first().map(|s| s.as_str()) == Some(name.as_str()) && path.len() >= 2 {
                Ok(())
            } else {
                Err(format!(
                    "enum variant path {:?} does not match enum {}",
                    path, name
                ))
            }
        }
        (PatternExpr::EnumVariant { .. }, _) => {
            Err("enum variant pattern cannot match non-enum".into())
        }
    }
}

fn validate_output_expr(o: &OutputExpr, module: &Module, env: &Env) -> Result<(), String> {
    let group = module
        .outputs
        .iter()
        .find(|g| g.name == o.group)
        .ok_or_else(|| format!("unknown output group '{}'", o.group))?;

    let variant_name = match &o.variant {
        Some(v) => v.clone(),
        None => {
            if group.variants.len() == 1 {
                match &group.variants[0] {
                    ConstructorVariant::Unit { name } => name.clone(),
                    ConstructorVariant::Struct { name, .. } => name.clone(),
                }
            } else {
                return Err(format!(
                    "bare output '{}' is ambiguous; name a variant",
                    o.group
                ));
            }
        }
    };

    let decl = group
        .variants
        .iter()
        .find(|v| match v {
            ConstructorVariant::Unit { name } => name == &variant_name,
            ConstructorVariant::Struct { name, .. } => name == &variant_name,
        })
        .ok_or_else(|| {
            format!(
                "unknown output variant '{}' in group '{}'",
                variant_name, o.group
            )
        })?;

    let field_count = match decl {
        ConstructorVariant::Unit { .. } => 0,
        ConstructorVariant::Struct { fields, .. } => fields.len(),
    };
    if o.args.len() != field_count {
        return Err(format!(
            "output '{}' has {} args, variant expects {}",
            variant_name,
            o.args.len(),
            field_count
        ));
    }
    if let ConstructorVariant::Struct { fields, .. } = decl {
        for (arg, f) in o.args.iter().zip(fields.iter()) {
            let ty = type_expr(arg, env, &mut Vec::new())
                .map_err(|e| format!("arg for '{}': {}", f.name, e))?;
            let expected = type_to_expr_type(&f.ty);
            ty.compatible_with(&expected)
                .map_err(|e| format!("arg for '{}': {}", f.name, e))?;
        }
    }
    Ok(())
}

/// Type an expression under the explicit no-implicit-conversion rules (task 3L).
/// `warns` collects rule-4 negative-constant conversion warnings (non-fatal);
/// fatal fit errors are returned as `Err`. A constant expression (`IntLit`) is
/// a FULL-PRECISION integer literal - it carries no width of its own and is
/// checked against a bit<N> context only where it meets one (rule 4).
pub fn type_expr(e: &Expr, env: &Env, warns: &mut Vec<String>) -> Result<ExprType, String> {
    match e {
        Expr::Int { .. } => Ok(ExprType::IntLit),
        Expr::Bool { .. } => Ok(ExprType::Bool),
        Expr::Path { path } => resolve_path(path, env),
        // Explicit conversion cast (task 3L): types as Bit(width); the operand
        // must be a bitvector - a constant expression is legal (the cast IS the
        // explicit conversion), bool/enum are rejected.
        Expr::Cast { width, expr } => {
            let ty = type_expr(expr, env, warns)?;
            match ty {
                ExprType::Bit(_) | ExprType::IntLit => Ok(ExprType::Bit(*width)),
                _ => Err("cast operand must be a bitvector".into()),
            }
        }
        Expr::Unary { op, expr } => {
            let ty = type_expr(expr, env, warns)?;
            match op {
                UnaryOp::Not => {
                    ty.must_bool("operand of !")?;
                    Ok(ExprType::Bool)
                }
                // A bare constant under ~ / - stays a full-precision constant
                // (no bit<8> default - task 3L rule 1).
                UnaryOp::BitNot | UnaryOp::Neg => match ty {
                    ExprType::Bit(w) => Ok(ExprType::Bit(w)),
                    ExprType::IntLit => Ok(ExprType::IntLit),
                    _ => Err(format!(
                        "{} requires bitvector",
                        if matches!(op, UnaryOp::Neg) {
                            "negation"
                        } else {
                            "bit_not"
                        }
                    )),
                },
            }
        }
        Expr::Binary { op, left, right } => {
            let l = type_expr(left, env, warns)?;
            let r = type_expr(right, env, warns)?;
            binary_type(*op, left, right, &l, &r, warns)
        }
    }
}

fn resolve_path(path: &[String], env: &Env) -> Result<ExprType, String> {
    if path.is_empty() {
        return Err("empty path".into());
    }
    if path.len() == 1 {
        let name = &path[0];
        if name == "state" {
            return Ok(ExprType::Enum("State".into()));
        }
        if let Some(ty) = env.bindings.get(name) {
            return Ok(ty.clone());
        }
        if let Some(ty) = env.vars.get(name) {
            return Ok(ty.clone());
        }
        if env.state_variants.contains(name) {
            return Ok(ExprType::Enum("State".into()));
        }
        return Err(format!("unknown identifier '{}'", name));
    }

    let enum_name = &path[0];
    let variant = path
        .get(1)
        .ok_or_else(|| "enum path too short".to_string())?;
    let variants = env
        .enums
        .get(enum_name)
        .ok_or_else(|| format!("unknown enum type '{}'", enum_name))?;
    if !variants.contains(variant) {
        return Err(format!(
            "unknown variant '{}' in enum '{}'",
            variant, enum_name
        ));
    }
    if path.len() > 2 {
        return Err(format!("unsupported path length {}", path.len()));
    }
    Ok(ExprType::Enum(enum_name.clone()))
}

fn binary_type(
    op: BinOp,
    left: &Expr,
    right: &Expr,
    l: &ExprType,
    r: &ExprType,
    warns: &mut Vec<String>,
) -> Result<ExprType, String> {
    match op {
        BinOp::LogicAnd | BinOp::LogicOr => {
            l.must_bool("left operand")?;
            r.must_bool("right operand")?;
            Ok(ExprType::Bool)
        }
        BinOp::Eq | BinOp::NotEq | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
            compare_types(left, right, l, r, warns)?;
            Ok(ExprType::Bool)
        }
        BinOp::Plus | BinOp::Minus | BinOp::Star | BinOp::BitAnd | BinOp::BitOr | BinOp::BitXor => {
            // Bool &/| act as logical connectives (preserved from before 3L).
            if matches!(op, BinOp::BitAnd | BinOp::BitOr)
                && matches!(l, ExprType::Bool)
                && matches!(r, ExprType::Bool)
            {
                return Ok(ExprType::Bool);
            }
            arith_bit_type(left, right, l, r, warns)
        }
        BinOp::ShiftLeft | BinOp::ShiftRight => shift_type(left, right, l, r, warns),
    }
}

/// Arithmetic / bitwise (non-bool) operand typing: equal widths required;
/// a constant meeting a bit<N> operand goes through rule 4; two constants form
/// a full-precision constant (no width cap).
fn arith_bit_type(
    left: &Expr,
    right: &Expr,
    l: &ExprType,
    r: &ExprType,
    warns: &mut Vec<String>,
) -> Result<ExprType, String> {
    match (l, r) {
        (ExprType::Bit(a), ExprType::Bit(b)) => {
            if a != b {
                return Err(format!("bitwidth mismatch: bit<{}> vs bit<{}>", a, b));
            }
            Ok(ExprType::Bit(*a))
        }
        (ExprType::IntLit, ExprType::Bit(b)) => {
            const_fit(fold_const(left)?, *b, warns)?;
            Ok(ExprType::Bit(*b))
        }
        (ExprType::Bit(b), ExprType::IntLit) => {
            const_fit(fold_const(right)?, *b, warns)?;
            Ok(ExprType::Bit(*b))
        }
        (ExprType::IntLit, ExprType::IntLit) => Ok(ExprType::IntLit),
        _ => Err("bitwise/arithmetic operator requires bitvector operands".into()),
    }
}

/// Shift typing: the result takes the shifted VALUE's width; a constant value
/// shifted by a VARIABLE amount takes the shift amount's width (the refinement);
/// two constants form a full-precision constant. The right operand must be a
/// bitvector or integer literal.
fn shift_type(
    left: &Expr,
    _right: &Expr,
    l: &ExprType,
    r: &ExprType,
    warns: &mut Vec<String>,
) -> Result<ExprType, String> {
    if matches!(r, ExprType::Bool | ExprType::Enum(_)) {
        return Err("shift right operand must be bitvector or integer literal".into());
    }
    match (l, r) {
        (ExprType::Bit(a), _) => Ok(ExprType::Bit(*a)),
        (ExprType::IntLit, ExprType::Bit(b)) => {
            const_fit(fold_const(left)?, *b, warns)?;
            Ok(ExprType::Bit(*b))
        }
        (ExprType::IntLit, ExprType::IntLit) => Ok(ExprType::IntLit),
        _ => Err("shift left operand must be bitvector".into()),
    }
}

/// Comparison typing: equal types, or a constant meeting a bit<N> operand via
/// rule 4 (decision (d): "any operation" includes comparisons - truncating
/// `x == 300` to `x == 44` would silently change meaning). Two constants
/// compare at full precision.
fn compare_types(
    left: &Expr,
    right: &Expr,
    l: &ExprType,
    r: &ExprType,
    warns: &mut Vec<String>,
) -> Result<(), String> {
    match (l, r) {
        (a, b) if a == b => Ok(()),
        (ExprType::IntLit, ExprType::Bit(b)) => const_fit(fold_const(left)?, *b, warns),
        (ExprType::Bit(b), ExprType::IntLit) => const_fit(fold_const(right)?, *b, warns),
        (ExprType::IntLit, ExprType::Bool) | (ExprType::Bool, ExprType::IntLit) => {
            Err("integer literal cannot be used as bool".into())
        }
        (a, b) => Err(format!("type mismatch: {:?} vs {:?}", a, b)),
    }
}

impl ExprType {
    fn must_bool(&self, what: &str) -> Result<(), String> {
        match self {
            ExprType::Bool => Ok(()),
            other => Err(format!("{} must be bool, found {:?}", what, other)),
        }
    }

    /// Lenient type conformance for OUTPUT-PATTERN argument checking (the V8a/
    /// V8b pattern-matching surface, which task 3L deliberately leaves as-is):
    /// a literal adapts to the field's bitvector type; equal types match.
    fn compatible_with(&self, other: &ExprType) -> Result<(), String> {
        match (self, other) {
            (ExprType::IntLit, ExprType::Bit(_)) | (ExprType::Bit(_), ExprType::IntLit) => Ok(()),
            (a, b) if a == b => Ok(()),
            (ExprType::IntLit, ExprType::Bool) | (ExprType::Bool, ExprType::IntLit) => {
                Err("integer literal cannot be used as bool".into())
            }
            (a, b) => Err(format!("type mismatch: {:?} vs {:?}", a, b)),
        }
    }
}

pub fn is_safety_formula(f: &Formula) -> bool {
    match f {
        // Liveness obligations are not safety formulas (D-P4-8).
        Formula::BoundedEventually { .. } | Formula::Eventually { .. } | Formula::Until { .. } => {
            false
        }
        Formula::Not { inner }
        | Formula::Next { inner }
        | Formula::Always { inner }
        | Formula::BoundedAlways { inner, .. } => is_safety_formula(inner),
        Formula::And { left, right }
        | Formula::Or { left, right }
        | Formula::Implies { left, right } => is_safety_formula(left) && is_safety_formula(right),
        Formula::InputIs { .. } | Formula::OutputHas { .. } | Formula::Expr { .. } => true,
    }
}

pub fn type_formula(f: &Formula, env: &Env, warns: &mut Vec<String>) -> Result<(), String> {
    match f {
        Formula::Expr { expr } => {
            let ty = type_expr(expr, env, warns)?;
            ty.must_bool("formula expression")?;
            Ok(())
        }
        Formula::InputIs { pattern } => check_pattern_group(pattern, env, true),
        Formula::OutputHas { pattern } => check_output_pattern_group(pattern, env),
        // `eventually (p)` produces a boolean formula; its body must
        // type-check as boolean like any other temporal body.
        Formula::Not { inner }
        | Formula::Next { inner }
        | Formula::Always { inner }
        | Formula::BoundedAlways { inner, .. }
        | Formula::BoundedEventually { inner, .. }
        | Formula::Eventually { inner } => type_formula(inner, env, warns),
        // `p until q` produces a boolean formula; both operands must
        // type-check as boolean.
        Formula::And { left, right }
        | Formula::Or { left, right }
        | Formula::Implies { left, right }
        | Formula::Until { left, right } => {
            type_formula(left, env, warns)?;
            type_formula(right, env, warns)
        }
    }
}

fn check_pattern_group(pattern: &EventPattern, env: &Env, is_input: bool) -> Result<(), String> {
    // Formula patterns introduce no usable bindings (task 3F): a named field is a
    // labeled wildcard - both MC backends render PatternExpr::Bind as a
    // no-constraint match (ref_mc.rs::input_matches / smt_plan.rs::resolve_input_is),
    // and the bound name is not referenceable anywhere in a formula. Mirrors
    // check_output_pattern_group, which never restricted names - this removes the
    // input/output asymmetry flagged by V1_R_R_AUDIT.md (row V17).
    let _ = (pattern, env, is_input);
    Ok(())
}

fn check_output_pattern_group(pattern: &OutputPattern, env: &Env) -> Result<(), String> {
    let _ = (pattern, env);
    Ok(())
}

// ---------------------------------------------------------------------------
// Phase 2 non-fatal diagnostics (warnings): unreachable states, dead events,
// unused variables, impossible guards. These never block code generation; the
// CLI and codegen surface them as warnings.
// ---------------------------------------------------------------------------

/// Collect all identifier segments referenced by an expression.
fn collect_names(e: &Expr, out: &mut HashSet<String>) {
    match e {
        Expr::Path { path } => {
            if path.len() == 1 {
                out.insert(path[0].clone());
            }
        }
        Expr::Unary { expr, .. } => collect_names(expr, out),
        Expr::Binary { left, right, .. } => {
            collect_names(left, out);
            collect_names(right, out);
        }
        _ => {}
    }
}

/// True when the guard is the literal `false`.
fn is_literal_false(e: &Expr) -> bool {
    matches!(e, Expr::Bool { value: false })
}

/// Find a variable bound to two different integer literals by equality atoms
/// joined at the top level with `&&` (syntactic impossibility detection).
fn contradictory_equalities(e: &Expr) -> Option<(String, u64, u64)> {
    let mut conjuncts = Vec::new();
    let mut stack = vec![e];
    while let Some(cur) = stack.pop() {
        match cur {
            Expr::Binary {
                op: BinOp::LogicAnd,
                left,
                right,
            } => {
                stack.push(&**left);
                stack.push(&**right);
            }
            other => conjuncts.push(other),
        }
    }
    let mut seen: HashMap<String, u64> = HashMap::new();
    for c in &conjuncts {
        let (var, value) = match c {
            Expr::Binary {
                op: BinOp::Eq,
                left,
                right,
            } => match (&**left, &**right) {
                (Expr::Path { path }, Expr::Int { value }) if path.len() == 1 => {
                    (path[0].clone(), *value)
                }
                (Expr::Int { value }, Expr::Path { path }) if path.len() == 1 => {
                    (path[0].clone(), *value)
                }
                _ => continue,
            },
            _ => continue,
        };
        if let Some(prev) = seen.get(&var) {
            if *prev != value {
                return Some((var, *prev, value));
            }
        } else {
            seen.insert(var, value);
        }
    }
    None
}

/// Non-fatal Phase 2 diagnostics for one module.
pub fn module_warnings(module: &Module) -> Vec<String> {
    let mut w: Vec<String> = Vec::new();

    // Rule 4 (task 3L): negative constants implicitly converted to unsigned
    // bit-vectors are a WARNING. Re-run typing over every expression with a real
    // collector (validate_module discards these - they are non-fatal).
    let env0 = Env::for_module(module);
    for v in &module.vars {
        // A constant-expression init is also fit-checked against the var's
        // declared width (rule 4) - validate_module does the same check but
        // discards its warnings; this is where the negative-constant warning
        // surfaces.
        if let Ok(ExprType::IntLit) = type_expr(&v.init, &env0, &mut w) {
            if let Type::Bit { width } = &v.ty {
                if let Ok(c) = fold_const(&v.init) {
                    let _ = const_fit(c, *width, &mut w);
                }
            }
        }
    }
    for t in &module.transitions {
        let mut env = env0.clone();
        let _ = validate_event_pattern(&t.pattern, module, &mut env);
        if let Some(guard) = &t.guard {
            let _ = type_expr(guard, &env, &mut w);
        }
        for a in &t.assignments {
            let _ = type_expr(&a.value, &env, &mut w);
        }
    }
    for p in module.properties.iter().chain(module.assumptions.iter()) {
        let _ = type_formula(&p.formula, &env0, &mut w);
    }

    // Unreachable states (guards ignored: reachability is over-approximated).
    if let Some(init) = &module.initial_state {
        let mut reachable: HashSet<&str> = HashSet::new();
        let mut stack = vec![init.as_str()];
        while let Some(s) = stack.pop() {
            if reachable.insert(s) {
                for t in module.transitions.iter().filter(|t| &t.state == s) {
                    if !reachable.contains(t.next_state.as_str()) {
                        stack.push(&t.next_state);
                    }
                }
            }
        }
        if let Some(se) = module.enums.iter().find(|e| e.name == "State") {
            for v in &se.variants {
                if !reachable.contains(v.as_str()) {
                    w.push(format!(
                        "module '{}': state '{}' is unreachable from the initial state",
                        module.name, v
                    ));
                }
            }
        }
    }

    // Dead events: input constructors never referenced by any transition pattern.
    let mut used: HashSet<(String, String)> = HashSet::new();
    for t in &module.transitions {
        match &t.pattern.variant {
            None => {
                if let Some(g) = module.events.iter().find(|g| g.name == t.pattern.group) {
                    for v in &g.variants {
                        let vn = match v {
                            ConstructorVariant::Unit { name } => name.clone(),
                            ConstructorVariant::Struct { name, .. } => name.clone(),
                        };
                        used.insert((g.name.clone(), vn));
                    }
                }
            }
            Some(pv) => {
                used.insert((
                    t.pattern.group.clone(),
                    pattern_variant_name(pv).to_string(),
                ));
            }
        }
    }
    for g in &module.events {
        for v in &g.variants {
            let vn = match v {
                ConstructorVariant::Unit { name } => name.clone(),
                ConstructorVariant::Struct { name, .. } => name.clone(),
            };
            if !used.contains(&(g.name.clone(), vn.clone())) {
                w.push(format!(
                    "module '{}': event '{}' is never used in any transition pattern (dead event)",
                    module.name, vn
                ));
            }
        }
    }

    // Unused variables: never read (guards/emit args) and never assigned.
    let mut touched: HashSet<String> = HashSet::new();
    for t in &module.transitions {
        if let Some(g) = &t.guard {
            collect_names(g, &mut touched);
        }
        for o in &t.emits {
            for a in &o.args {
                collect_names(a, &mut touched);
            }
        }
        for a in &t.assignments {
            touched.insert(a.var.clone());
        }
    }
    if let Some(oe) = &module.on_error {
        for o in &oe.emits {
            for a in &o.args {
                collect_names(a, &mut touched);
            }
        }
    }
    for v in &module.vars {
        if !touched.contains(&v.name) {
            w.push(format!(
                "module '{}': variable '{}' is never read or written (unused)",
                module.name, v.name
            ));
        }
    }

    // Order-dependent transition pairs: same state + identical pattern, no
    // priorities on either, differing (or one missing) guard. First match wins
    // in source order; flag so the author can pin intent with priorities.
    let mut by_state: std::collections::BTreeMap<&str, Vec<&Transition>> =
        std::collections::BTreeMap::new();
    for t in &module.transitions {
        by_state.entry(&t.state).or_default().push(t);
    }
    for (state, ts) in &by_state {
        for i in 0..ts.len() {
            for j in (i + 1)..ts.len() {
                if ts[i].pattern == ts[j].pattern
                    && ts[i].priority.is_none()
                    && ts[j].priority.is_none()
                    && ts[i].guard != ts[j].guard
                {
                    w.push(format!(
                        "module '{}' state '{}': transitions #{} and #{} share an event pattern with different guards and no priorities; first match wins in source order (add distinct `priority N;` to pin the intent)",
                        module.name, state, ts[i].id, ts[j].id
                    ));
                }
            }
        }
    }

    // Impossible guards: literal false, or contradictory top-level equalities.
    for t in &module.transitions {
        if let Some(g) = &t.guard {
            if is_literal_false(g) {
                w.push(format!(
                    "module '{}': transition #{} has a guard that is always false (dead transition)",
                    module.name, t.id
                ));
            } else if let Some((var, a, b)) = contradictory_equalities(g) {
                w.push(format!(
                    "module '{}': transition #{} guard is impossible: '{}' is compared equal to both {} and {}",
                    module.name, t.id, var, a, b
                ));
            }
        }
    }

    w
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lang::parser::parse_source;

    fn module_from(source: &str) -> Module {
        let ir = parse_source(source).expect("test source should parse");
        assert_eq!(ir.modules.len(), 1, "expected exactly one module");
        ir.modules.into_iter().next().unwrap()
    }

    #[test]
    fn version_header_is_parsed_into_module() {
        let m = module_from("module m version \"1.2.3\" { enum State { A } initial A }");
        assert_eq!(m.version.as_deref(), Some("1.2.3"));
    }

    #[test]
    fn version_header_format_check() {
        // Valid formats are accepted (X.Y and X.Y.Z).
        for v in ["1.0", "1.2.3", "10.20.30"] {
            let m = module_from(&format!(
                "module m version \"{v}\" {{ enum State {{ A }} initial A }}"
            ));
            assert!(
                validate_module(&m).is_empty(),
                "{v} must validate: {:?}",
                validate_module(&m)
            );
        }
        // Absent header (legacy v0.x) is accepted.
        let m = module_from("module m { enum State { A } initial A }");
        assert!(validate_module(&m).is_empty());
        // Malformed formats are semantic errors with the pinned message.
        for v in ["1.x", "1", "1.2.3.4", "1..2"] {
            let m = module_from(&format!(
                "module m version \"{v}\" {{ enum State {{ A }} initial A }}"
            ));
            let errs = validate_module(&m);
            assert!(
                errs.iter().any(|e| e.message
                    == format!("module 'm' invalid version header '{v}' (expected X.Y or X.Y.Z)")),
                "{v} must be rejected with the pinned message: {errs:?}"
            );
        }
    }

    #[test]
    fn on_error_block_is_parsed_into_module() {
        let m = module_from(
            "module m { enum State { A, B } initial A event E output O              transition A on E { next B; } on_error { next A; emit O; } }",
        );
        let oe = m.on_error.as_ref().expect("on_error parsed");
        assert_eq!(oe.next_state, "A");
        assert_eq!(oe.emits.len(), 1);
    }

    #[test]
    fn on_error_without_next_fails_parse() {
        let r = parse_source(
            "module m { enum State { A } initial A event E output O              on_error { emit O; } }",
        );
        assert!(r.is_err(), "on_error without next must not parse");
    }

    #[test]
    fn duplicate_priority_same_pattern_is_error() {
        let m = module_from(
            "module m { enum State { A, B } initial A event E              transition A on E { next B; priority 5; }              transition A on E { next A; priority 5; } }",
        );
        let errors = validate_module(&m);
        assert!(errors
            .iter()
            .any(|e| e.message.contains("duplicate priority")));
    }

    #[test]
    fn duplicate_priority_different_patterns_is_allowed() {
        let m = module_from(
            "module m { enum State { A, B } initial A event E event F              transition A on E { next B; priority 5; }              transition A on F { next A; priority 5; } }",
        );
        assert!(validate_module(&m).is_empty());
    }

    #[test]
    fn shadowed_duplicate_transition_is_error() {
        let m = module_from(
            "module m { enum State { A, B } initial A event E              transition A on E { next B; } transition A on E { next A; } }",
        );
        assert!(validate_module(&m)
            .iter()
            .any(|e| e.message.contains("duplicate transitions")));
    }

    #[test]
    fn order_dependent_guard_pair_is_warning_only() {
        let m = module_from(
            "module m { enum State { A, B } initial A event E var x: bit<8> init 0              transition A on E where x < 5 { next B; }              transition A on E where x >= 5 { next A; } }",
        );
        assert!(validate_module(&m).is_empty(), "no hard errors expected");
        let warnings = module_warnings(&m);
        assert!(warnings
            .iter()
            .any(|w| w.contains("share an event pattern")));
    }

    #[test]
    fn unreachable_state_is_warning() {
        let m = module_from(
            "module m { enum State { A, B, Isolated } initial A event E              transition A on E { next B; } }",
        );
        assert!(validate_module(&m).is_empty());
        assert!(module_warnings(&m)
            .iter()
            .any(|w| w.contains("'Isolated' is unreachable")));
    }

    #[test]
    fn dead_event_is_warning() {
        let m = module_from(
            "module m { enum State { A, B } initial A event E event Ghost              transition A on E { next B; } }",
        );
        assert!(validate_module(&m).is_empty());
        assert!(module_warnings(&m)
            .iter()
            .any(|w| w.contains("'Ghost' is never used")));
    }

    #[test]
    fn unused_var_is_warning() {
        let m = module_from(
            "module m { enum State { A, B } initial A event E var lonely: bit<8> init 0              transition A on E { next B; } }",
        );
        assert!(validate_module(&m).is_empty());
        assert!(module_warnings(&m)
            .iter()
            .any(|w| w.contains("'lonely' is never read or written")));
    }

    #[test]
    fn literal_false_guard_is_warning() {
        let m = module_from(
            "module m { enum State { A, B } initial A event E              transition A on E where false { next B; } }",
        );
        assert!(validate_module(&m).is_empty());
        assert!(module_warnings(&m)
            .iter()
            .any(|w| w.contains("always false")));
    }

    #[test]
    fn contradictory_equalities_guard_is_warning() {
        let m = module_from(
            "module m { enum State { A, B } initial A event E var x: bit<8> init 0              transition A on E where x == 1 && x == 2 { next B; } }",
        );
        assert!(validate_module(&m).is_empty());
        assert!(module_warnings(&m).iter().any(|w| w.contains("impossible")));
    }

    #[test]
    fn on_error_unknown_state_is_error() {
        let m = module_from(
            "module m { enum State { A, B } initial A event E              transition A on E { next B; } on_error { next Nowhere; } }",
        );
        // `Nowhere` is not a State variant: the parser accepts identifiers and
        // validation must reject them.
        assert!(validate_module(&m)
            .iter()
            .any(|e| e.message.contains("on_error")));
    }

    #[test]
    fn bounded_always_zero_bound_is_error() {
        let m = module_from(
            "module m { enum State { A } initial A prop p : always[0] (state == A) ; }",
        );
        assert!(validate_module(&m)
            .iter()
            .any(|e| e.message.contains("always[N] requires N >= 1")));
    }

    #[test]
    fn bounded_always_zero_bound_nested_is_error() {
        // Parenthesized left operand kept for readability; since the task 23
        // grammar fix bare `state == A && always[0] (...)` parses identically.
        let m = module_from(
            "module m { enum State { A } initial A prop p : always ((state == A) && always[0] (state == A)) ; }",
        );
        assert!(validate_module(&m)
            .iter()
            .any(|e| e.message.contains("always[N] requires N >= 1")));
    }

    #[test]
    fn bounded_eventually_zero_bound_is_allowed() {
        let m = module_from(
            "module m { enum State { A } initial A event E prop p : always (input_is(E) -> bounded_eventually[0] (state == A)) ; }",
        );
        assert!(validate_module(&m).is_empty());
    }

    #[test]
    fn duplicate_property_name_rejected_at_parse() {
        // Pre-existing parser guard (build_module): duplicate prop names never
        // reach validation.
        let r = parse_source(
            "module m { enum State { A } initial A prop p1 : always (state == A) ; prop p1 : always (state == A) ; }",
        );
        assert!(r.is_err(), "duplicate prop name must not parse");
    }

    #[test]
    fn duplicate_property_and_assume_name_rejected_at_parse() {
        let r = parse_source(
            "module m { enum State { A } initial A event E prop p1 : always (state == A) ; assume p1 : always (input_is(E) -> state == A) ; }",
        );
        assert!(r.is_err(), "cross-list duplicate name must not parse");
    }

    #[test]
    fn validator_rejects_duplicate_names_in_ir() {
        // The validator guard also covers IR loaded from JSON, which bypasses
        // the parser pass.
        let mut m =
            module_from("module m { enum State { A } initial A prop p1 : always (state == A) ; }");
        m.properties.push(m.properties[0].clone());
        assert!(validate_module(&m)
            .iter()
            .any(|e| e.message.contains("duplicate property/assumption name")));
    }

    // -- Task 5: liveness operator typing (D-P4-8) ----------------------------

    fn liveness_module(formula: &str) -> Module {
        module_from(&format!(
            "module m {{ enum State {{ A }} initial A event E \
             var flag: bool init false var x: bit<8> init 0 prop p : {} ; }}",
            formula
        ))
    }

    #[test]
    fn liveness_operands_must_be_boolean_accepted() {
        for formula in [
            "eventually (flag)",
            "eventually (flag || x == 1)",
            "flag until x == 0",
            "!flag until x == 1",
            "(flag && x == 2) until flag",
        ] {
            let m = liveness_module(formula);
            assert!(
                validate_module(&m).is_empty(),
                "'{}' must validate: {:?}",
                formula,
                validate_module(&m)
            );
        }
    }

    #[test]
    fn liveness_operands_must_be_boolean_rejected() {
        // Non-boolean operand to `eventually`.
        let m = liveness_module("eventually (x)");
        assert!(
            validate_module(&m)
                .iter()
                .any(|e| e.message.contains("must be bool")),
            "'eventually (x)' must be rejected with a bool diagnostic"
        );
        // Non-boolean left / right operand of `until`.
        for formula in ["x until flag", "flag until x"] {
            let m = liveness_module(formula);
            assert!(
                validate_module(&m)
                    .iter()
                    .any(|e| e.message.contains("must be bool")),
                "'{}' must be rejected with a bool diagnostic",
                formula
            );
        }
    }

    #[test]
    fn bounded_always_check_reaches_into_liveness_operands() {
        // The always[N] >= 1 bound check recurses through liveness operators.
        for formula in [
            "eventually (always[0] (flag))",
            "flag until always[0] (flag)",
        ] {
            let m = liveness_module(formula);
            assert!(
                validate_module(&m)
                    .iter()
                    .any(|e| e.message.contains("always[N] requires N >= 1")),
                "'{}' must carry the always[N] bound error",
                formula
            );
        }
    }

    #[test]
    fn liveness_operators_are_not_safety_formulas() {
        let m = liveness_module("eventually (flag)");
        assert!(!is_safety_formula(&m.properties[0].formula));
        let m = liveness_module("flag until x == 0");
        assert!(!is_safety_formula(&m.properties[0].formula));
    }

    // ------------------------------------------------------------------
    // Task 3I: pins for the V17 lift (task 3F) - named fields in formula
    // input_is patterns are labeled wildcards that constrain nothing.
    // ------------------------------------------------------------------

    #[test]
    fn formula_input_is_named_fields_validate() {
        // The lift: a named field in a formula input_is pattern validates -
        // the bound name constrains nothing (both MC backends render Bind as
        // a no-constraint match), closing the input/output asymmetry.
        let m = module_from(
            "module m { enum State { A, B } initial A event Recv(Ack(x: bit<8>)) \
             prop p : always (input_is(Recv(Ack(x))) -> state == A) ; }",
        );
        assert!(
            validate_module(&m).is_empty(),
            "named-field formula input_is must validate: {:?}",
            validate_module(&m)
        );
    }

    #[test]
    fn transition_pattern_field_typing_still_fires() {
        // The V17 lift is NARROW: TRANSITION patterns keep full field typing
        // (V8a) - an out-of-range literal there is still an error.
        let m = module_from(
            "module m { enum State { A, B } initial A event Recv(Ack(x: bit<8>)) \
             transition A on Recv(Ack(300)) { next B; } \
             transition B on Recv(Ack(_)) { next A; } }",
        );
        let errs = validate_module(&m);
        assert!(
            errs.iter()
                .any(|e| e.message.contains("literal 300 does not fit bit<8>")),
            "transition-pattern V8a must still fire: {errs:?}"
        );
    }

    #[test]
    fn formula_pattern_structural_checks_absent_by_design() {
        // Formula patterns NEVER had structural checks (task 3F finding):
        // check_pattern_group / check_output_pattern_group are no-ops, so an
        // unknown group, an arity mismatch, or an out-of-range literal inside
        // a formula input_is validates and is soundly VACUOUS in both MC modes
        // (the atom can never match). Pin the absence so nobody "restores" a
        // check that never existed.
        for src in [
            "module m { enum State { A } initial A event Recv(Ack(x: bit<8>)) prop p : always (input_is(Nope(Q(_))) -> state == A) ; }",
            "module m { enum State { A } initial A event Recv(Ack(x: bit<8>)) prop p : always (input_is(Recv(Ack(x, y))) -> state == A) ; }",
            "module m { enum State { A } initial A event Recv(Ack(x: bit<8>)) prop p : always (input_is(Recv(Ack(300))) -> state == A) ; }",
        ] {
            let m = module_from(src);
            assert!(
                validate_module(&m).is_empty(),
                "formula patterns are structurally unchecked by design: {src}"
            );
        }
    }

    // ------------------------------------------------------------------
    // Task 3L: rule 4 constant conversion, the cast operator, shift refinement.
    // ------------------------------------------------------------------

    #[test]
    fn rule4_positive_out_of_range_is_an_error_naming_the_cast() {
        let m = module_from(
            "module m { enum State { A } initial A event E output O var x: bit<8> init 0 \
             transition A on E { next A; set x = x + 300; emit O; } }",
        );
        let errs = validate_module(&m);
        assert!(
            errs.iter().any(|e| e.message
                == "module 'm' transition #0: set 'x': constant 300 does not fit in bit<8>; use an explicit cast, e.g. bit<8>(...)"),
            "pinned rule-4 error expected: {errs:?}"
        );
    }

    #[test]
    fn rule4_masked_constant_folds_positive_and_validates() {
        // (300 & 255) folds to the positive constant 44, which fits bit<8> -
        // no diagnostic (a mask is just a bitwise op under rule 1).
        let m = module_from(
            "module m { enum State { A } initial A event E output O var x: bit<8> init 0 \
             transition A on E { next A; set x = x + (300 & 255); emit O; } }",
        );
        assert!(validate_module(&m).is_empty());
    }

    #[test]
    fn rule4_negative_fitting_is_a_warning_not_an_error() {
        let m = module_from(
            "module m { enum State { A } initial A event E output O var a: bit<4> init -5 \
             transition A on E { next A; set a = a + 1; emit O; } }",
        );
        assert!(
            validate_module(&m).is_empty(),
            "fitting negative must validate"
        );
        let warns = module_warnings(&m);
        assert!(
            warns.iter().any(|w| w
                == "implicit conversion of negative constant to unsigned bit-vector: -5 becomes 11 (two's complement in bit<4>)"),
            "pinned rule-4 warning expected: {warns:?}"
        );
    }

    #[test]
    fn rule4_negative_not_fitting_is_an_error() {
        let m = module_from(
            "module m { enum State { A } initial A event E output O var a: bit<3> init -5 \
             transition A on E { next A; set a = a + 1; emit O; } }",
        );
        let errs = validate_module(&m);
        assert!(
            errs.iter().any(|e| e.message
                == "module 'm' var 'a' init: constant -5 does not fit in bit<3> (its two's-complement form needs 4 bits); use an explicit cast, e.g. bit<3>(...)"),
            "pinned rule-4 negative error expected: {errs:?}"
        );
    }

    #[test]
    fn rule4_explicit_cast_suppresses_all_diagnostics() {
        // Cast of a constant expression is LEGAL and the only explicitness
        // mechanism: no error, no warning.
        for src in [
            "module m { enum State { A } initial A event E output O var a: bit<4> init bit<4>(-5) transition A on E { next A; set a = a + 1; emit O; } }",
            "module m { enum State { A } initial A event E output O var b: bit<3> init bit<3>(1 - 6) transition A on E { next A; set b = b + 1; emit O; } }",
            "module m { enum State { A } initial A event E output O var x: bit<8> init 0 transition A on E { next A; set x = x + bit<8>(300); emit O; } }",
        ] {
            let m = module_from(src);
            assert!(validate_module(&m).is_empty(), "cast must validate: {src}");
            assert!(
                !module_warnings(&m)
                    .iter()
                    .any(|w| w.contains("implicit conversion")),
                "cast must suppress the warning: {src}"
            );
        }
    }

    #[test]
    fn rule4_masked_negative_folds_positive_no_warning() {
        // (-5) & 15 folds to the positive constant 11 - converts under rule 8
        // with no diagnostic (no syntactic mask detection needed).
        let m = module_from(
            "module m { enum State { A } initial A event E output O var a: bit<4> init (-5) & 15 \
             transition A on E { next A; set a = a + 1; emit O; } }",
        );
        assert!(validate_module(&m).is_empty());
        assert!(!module_warnings(&m)
            .iter()
            .any(|w| w.contains("implicit conversion")));
    }

    #[test]
    fn rule4_comparison_with_out_of_range_constant_is_an_error() {
        // "Any operation" includes comparisons (user decision d): truncating
        // `x == 300` to `x == 44` would silently change meaning.
        let m = module_from(
            "module m { enum State { A, B } initial A event E output O var x: bit<8> init 0 \
             transition A on E where x == 300 { next B; emit O; } }",
        );
        let errs = validate_module(&m);
        assert!(
            errs.iter().any(|e| e
                .message
                .contains("constant 300 does not fit in bit<8>; use an explicit cast")),
            "comparison must apply rule 4: {errs:?}"
        );
    }

    #[test]
    fn cast_operand_must_be_a_bitvector() {
        for (src, frag) in [
            (
                "module m { enum State { A } initial A event E output O var x: bit<8> init 0 transition A on E { next A; set x = bit<4>(true); emit O; } }",
                "cast operand must be a bitvector",
            ),
            (
                "module m { enum Mode { Run, Stop } enum State { A } initial A event E output O var m: Mode init Mode::Run transition A on E { next A; set m = Mode::Stop; emit O; } prop p : always (bit<4>(Mode::Run) == 0) ; }",
                "cast operand must be a bitvector",
            ),
        ] {
            let m = module_from(src);
            let errs = validate_module(&m);
            assert!(
                errs.iter().any(|e| e.message.contains(frag)),
                "expected '{frag}': {errs:?}"
            );
        }
    }

    #[test]
    fn two_bare_literals_form_a_full_precision_constant() {
        // 200 * 200 == 40000 type-checks and holds - impossible under the old
        // "two bare literals form bit<8>" rule (which would give 16).
        let m = module_from(
            "module m { enum State { A } initial A event E output O \
             prop p : 200 * 200 == 40000 ; transition A on E { next A; emit O; } }",
        );
        assert!(validate_module(&m).is_empty());
    }

    #[test]
    fn constant_expression_overflow_is_an_error() {
        // (2^64 - 1)^2 exceeds i128::MAX - the fold must error, never wrap.
        let m = module_from(
            "module m { enum State { A } initial A event E output O var x: bit<8> init 0 \
             transition A on E { next A; set x = x + (18446744073709551615 * 18446744073709551615); emit O; } }",
        );
        let errs = validate_module(&m);
        assert!(
            errs.iter()
                .any(|e| e.message.contains("constant expression overflows")),
            "overflow must be an error, never a silent wrap: {errs:?}"
        );
    }

    #[test]
    fn shift_refinement_constant_value_variable_amount_takes_amount_width() {
        // `1 << k` with k: bit<3> types as bit<3> (was bit<8> before 3L), so it
        // validates against a bit<3> var - the old rule would have rejected it.
        let m = module_from(
            "module m { enum State { A } initial A event E output O var x: bit<3> init 0 var k: bit<3> init 0 \
             transition A on E { next A; set x = 1 << k; emit O; } }",
        );
        assert!(
            validate_module(&m).is_empty(),
            "shift refinement must type as the amount's width"
        );
        // And a constant value shifted by a constant stays a full-precision
        // constant (no width at all).
        let m = module_from(
            "module m { enum State { A } initial A event E output O \
             prop p : 1 << 4 == 16 ; transition A on E { next A; emit O; } }",
        );
        assert!(validate_module(&m).is_empty());
    }

    #[test]
    fn bitwise_two_complement_pins_validate() {
        // Rule 1: bitwise ops on negative constants use the unsigned
        // two's-complement representation (full precision).
        for prop in ["2 + -3 == -1", "2 + ((-1) & 1) == 3", "2 + (-4) & 3 == 2"] {
            let m = module_from(&format!(
                "module m {{ enum State {{ A }} initial A event E output O prop p : {prop} ; transition A on E {{ next A; emit O; }} }}"
            ));
            assert!(validate_module(&m).is_empty(), "'{prop}' must validate");
        }
    }
}
