//! Semantic validation and expression typing for RFS IR.

use crate::ir::*;
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

    for v in &module.vars {
        if let Err(e) = type_expr(&v.init, &env0) {
            errors.push(ValidationError::new(format!(
                "module '{}' var '{}' init: {}",
                module.name, v.name, e
            )));
        } else {
            // initializer literal may adapt to var type; check compatibility loosely.
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
            if let Err(e) = type_expr(guard, &env) {
                errors.push(ValidationError::new(format!("{label}: guard: {e}")));
            } else if let Err(e) = type_expr(guard, &env).and_then(|ty| ty.must_bool("guard")) {
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
            if let Err(e) = type_expr(&a.value, &env) {
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

    for p in module.properties.iter().chain(module.assumptions.iter()) {
        if let Err(e) = type_formula(&p.formula, &env0) {
            errors.push(ValidationError::new(format!(
                "module '{}' property '{}': {}",
                module.name, p.name, e
            )));
        }
    }

    errors
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
            let ty = type_expr(arg, env).map_err(|e| format!("arg for '{}': {}", f.name, e))?;
            let expected = type_to_expr_type(&f.ty);
            ty.compatible_with(&expected)
                .map_err(|e| format!("arg for '{}': {}", f.name, e))?;
        }
    }
    Ok(())
}

pub fn type_expr(e: &Expr, env: &Env) -> Result<ExprType, String> {
    match e {
        Expr::Int { .. } => Ok(ExprType::IntLit),
        Expr::Bool { .. } => Ok(ExprType::Bool),
        Expr::Path { path } => resolve_path(path, env),
        Expr::Unary { op, expr } => {
            let ty = type_expr(expr, env)?;
            match op {
                UnaryOp::Not => {
                    ty.must_bool("operand of !")?;
                    Ok(ExprType::Bool)
                }
                UnaryOp::BitNot => match ty {
                    ExprType::Bit(w) => Ok(ExprType::Bit(w)),
                    ExprType::IntLit => Ok(ExprType::Bit(8)),
                    _ => Err("bit_not requires bitvector".into()),
                },
                UnaryOp::Neg => match ty {
                    ExprType::Bit(w) => Ok(ExprType::Bit(w)),
                    ExprType::IntLit => Ok(ExprType::Bit(8)),
                    _ => Err("negation requires bitvector".into()),
                },
            }
        }
        Expr::Binary { op, left, right } => {
            let l = type_expr(left, env)?;
            let r = type_expr(right, env)?;
            binary_type(*op, &l, &r)
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

fn binary_type(op: BinOp, l: &ExprType, r: &ExprType) -> Result<ExprType, String> {
    match op {
        BinOp::LogicAnd | BinOp::LogicOr => {
            l.must_bool("left operand")?;
            r.must_bool("right operand")?;
            Ok(ExprType::Bool)
        }
        BinOp::Eq | BinOp::NotEq | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
            l.compatible_with(r).map(|_| ExprType::Bool)
        }
        BinOp::Plus | BinOp::Minus | BinOp::Star => bit_op_type(l, r),
        BinOp::ShiftLeft | BinOp::ShiftRight => {
            let w = match l {
                ExprType::Bit(w) => *w,
                ExprType::IntLit => 8,
                _ => return Err("shift left operand must be bitvector".into()),
            };
            // right may be bit or literal; no strict width equality required.
            if matches!(r, ExprType::Bool | ExprType::Enum(_)) {
                return Err("shift right operand must be bitvector or integer literal".into());
            }
            Ok(ExprType::Bit(w))
        }
        BinOp::BitAnd | BinOp::BitOr => {
            if matches!(l, ExprType::Bool) && matches!(r, ExprType::Bool) {
                return Ok(ExprType::Bool);
            }
            bit_op_type(l, r)
        }
        BinOp::BitXor => bit_op_type(l, r),
    }
}

fn bit_op_type(l: &ExprType, r: &ExprType) -> Result<ExprType, String> {
    let w = match (l, r) {
        (ExprType::Bit(a), ExprType::Bit(b)) => {
            if a != b {
                return Err(format!("bitwidth mismatch: bit<{}> vs bit<{}>", a, b));
            }
            *a
        }
        (ExprType::IntLit, ExprType::Bit(b)) | (ExprType::Bit(b), ExprType::IntLit) => *b,
        (ExprType::IntLit, ExprType::IntLit) => 8,
        _ => return Err("bitwise/arithmetic operator requires bitvector operands".into()),
    };
    Ok(ExprType::Bit(w))
}

impl ExprType {
    fn must_bool(&self, what: &str) -> Result<(), String> {
        match self {
            ExprType::Bool => Ok(()),
            other => Err(format!("{} must be bool, found {:?}", what, other)),
        }
    }

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
        Formula::BoundedEventually { .. } => false,
        Formula::Not { inner } | Formula::Next { inner } | Formula::Always { inner } => {
            is_safety_formula(inner)
        }
        Formula::And { left, right }
        | Formula::Or { left, right }
        | Formula::Implies { left, right } => is_safety_formula(left) && is_safety_formula(right),
        Formula::InputIs { .. } | Formula::OutputHas { .. } | Formula::Expr { .. } => true,
    }
}

pub fn type_formula(f: &Formula, env: &Env) -> Result<(), String> {
    match f {
        Formula::Expr { expr } => {
            let ty = type_expr(expr, env)?;
            ty.must_bool("formula expression")?;
            Ok(())
        }
        Formula::InputIs { pattern } => check_pattern_group(pattern, env, true),
        Formula::OutputHas { pattern } => check_output_pattern_group(pattern, env),
        Formula::Not { inner }
        | Formula::Next { inner }
        | Formula::Always { inner }
        | Formula::BoundedEventually { inner, .. } => type_formula(inner, env),
        Formula::And { left, right }
        | Formula::Or { left, right }
        | Formula::Implies { left, right } => {
            type_formula(left, env)?;
            type_formula(right, env)
        }
    }
}

fn check_pattern_group(pattern: &EventPattern, env: &Env, is_input: bool) -> Result<(), String> {
    // Formula patterns do not introduce bindings in Phase 1. We only ensure group/variant names are plausible.
    let _ = is_input;
    if pattern.variant.is_some() {
        if let PatternVariant::Struct { fields, .. } = &pattern.variant.clone().unwrap() {
            for f in fields {
                if f.name.is_some() {
                    return Err("formula input patterns do not bind variables in Phase 1".into());
                }
            }
        }
    }
    let _ = env;
    Ok(())
}

fn check_output_pattern_group(pattern: &OutputPattern, env: &Env) -> Result<(), String> {
    let _ = (pattern, env);
    Ok(())
}
