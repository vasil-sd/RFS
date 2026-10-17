use crate::error::{ParseError, Span};
use crate::ir::*;
use nom::{
    branch::alt,
    bytes::complete::{tag, take_while1},
    character::complete::{multispace0, multispace1, satisfy},
    combinator::opt,
    error::ErrorKind,
    multi::{many0, many1},
    Err, IResult, Parser,
};
use std::path::Path;

#[derive(Debug)]
enum ModuleItem {
    Enum(EnumDecl),
    Initial(String),
    Event(ConstructorGroup),
    Output(ConstructorGroup),
    Var(VarDecl),
    Transition(Transition),
    Prop(Property),
    Assume(Property),
    OnError(OnError),
}

#[derive(Debug, Default, Clone)]
struct TransitionBody {
    next_state: Option<String>,
    emits: Vec<OutputExpr>,
    assignments: Vec<Assignment>,
    priority: Option<u32>,
}

pub fn parse_source(source: &str) -> Result<Ir, ParseError> {
    let clean = strip_comments(source);
    match rfs_source(clean.as_str()) {
        Ok((remaining, ir)) => {
            if remaining.trim().is_empty() {
                Ok(ir)
            } else {
                Err(err_at(
                    &clean,
                    remaining,
                    source,
                    "trailing input after RFS source",
                ))
            }
        }
        Err(e) => Err(nom_error_to_parse_error(e, &clean, source)),
    }
}

pub fn parse_file(path: &Path) -> Result<Ir, ParseError> {
    let source = std::fs::read_to_string(path).map_err(|e| {
        ParseError::new(
            format!("failed to read {}: {}", path.display(), e),
            Span::point(0),
        )
    })?;
    parse_source(&source)
}

fn strip_comments(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    let mut chars = source.chars().peekable();
    while let Some(&c) = chars.peek() {
        if c == '#' || (c == '/' && chars.clone().next() == Some('/')) {
            if c == '/' {
                out.push(' ');
                chars.next();
                out.push(' ');
                chars.next();
            } else {
                out.push(' ');
                chars.next();
            }
            while let Some(&ch) = chars.peek() {
                if ch == '\n' {
                    break;
                }
                out.push(' ');
                chars.next();
            }
        } else {
            out.push(c);
            chars.next();
        }
    }
    out
}

fn nom_error_to_parse_error(
    err: Err<nom::error::Error<&str>>,
    clean: &str,
    original: &str,
) -> ParseError {
    let offset = match &err {
        Err::Error(e) | Err::Failure(e) => clean.len().saturating_sub(e.input.len()),
        _ => 0,
    };
    ParseError::new("RFS parse error", Span::point(offset)).with_source(original.to_string())
}

fn err_at(clean: &str, residual: &str, original: &str, msg: &str) -> ParseError {
    let offset = clean.len().saturating_sub(residual.len());
    ParseError::new(msg, Span::point(offset)).with_source(original.to_string())
}

fn semantic_err(input: &str) -> Err<nom::error::Error<&str>> {
    Err::Failure(nom::error::Error {
        input,
        code: ErrorKind::Verify,
    })
}

fn syntax_err(input: &str) -> Err<nom::error::Error<&str>> {
    Err::Error(nom::error::Error {
        input,
        code: ErrorKind::Verify,
    })
}

fn rfs_source(i: &str) -> IResult<&str, Ir> {
    let (i, modules) = many0(module).parse_complete(i)?;
    Ok((
        i,
        Ir {
            ir_version: IR_VERSION,
            modules,
        },
    ))
}

fn module(i: &str) -> IResult<&str, Module> {
    let (i, _) = p_space0(i)?;
    let (i, _) = keyword(i, "module")?;
    let (i, _) = p_space1(i)?;
    let (i, name) = identifier(i)?;
    let (i, _) = p_space0(i)?;
    // Optional version header: `version "x.y.z"` before the opening brace.
    let (i, version) = match keyword(i, "version") {
        Ok((i_kw, _)) => {
            let (i2, _) = p_space1(i_kw)?;
            let (i3, v) = string_lit(i2)?;
            (i3, Some(v))
        }
        Err(_) => (i, None),
    };
    let (i, _) = p_space0(i)?;
    let (i, _) = p_tag("{", i)?;
    let (i, items) = many0(module_item).parse_complete(i)?;
    let (i, _) = p_space0(i)?;
    let (i, _) = p_tag("}", i)?;
    let (i, mut m) = build_module(name, items, i)?;
    m.version = version;
    Ok((i, m))
}

/// A double-quoted string literal without escapes (used for the version header).
fn string_lit(i: &str) -> IResult<&str, String> {
    let (i, _) = p_tag("\"", i)?;
    let (i, s) = take_while1(|c: char| c != '"' && !c.is_whitespace())(i)?;
    let (i, _) = p_tag("\"", i)?;
    Ok((i, s.to_string()))
}

fn module_item(i: &str) -> IResult<&str, ModuleItem> {
    alt((
        enum_decl,
        initial_decl,
        event_decl,
        output_decl,
        var_decl,
        transition_decl,
        prop_decl,
        assume_decl,
        on_error_decl,
    ))
    .parse_complete(i)
}

fn enum_decl(i: &str) -> IResult<&str, ModuleItem> {
    let (i, _) = p_space0(i)?;
    let (i, _) = keyword(i, "enum")?;
    let (i, _) = p_space1(i)?;
    let (i, name) = identifier(i)?;
    let (i, variants) = enum_variants(i)?;
    Ok((i, ModuleItem::Enum(EnumDecl { name, variants })))
}

fn enum_variants(i: &str) -> IResult<&str, Vec<String>> {
    let (i, _) = p_space0(i)?;
    let (i, _) = p_tag("{", i)?;
    let mut i = i;
    let mut out = Vec::new();
    loop {
        let (i_ws, _) = p_space0(i)?;
        if let Ok((after, _)) = p_tag("}", i_ws) {
            return Ok((after, out));
        }
        let (i_id, variant) = identifier(i_ws)?;
        if out.contains(&variant) {
            return Err(semantic_err(i_id));
        }
        out.push(variant);
        let (i_after, _) = p_space0(i_id)?;
        if let Ok((i_comma, _)) = p_tag(",", i_after) {
            i = i_comma;
            continue;
        }
        i = i_after;
    }
}

fn initial_decl(i: &str) -> IResult<&str, ModuleItem> {
    let (i, _) = p_space0(i)?;
    let (i, _) = keyword(i, "initial")?;
    let (i, _) = p_space1(i)?;
    let (i, name) = identifier(i)?;
    let (i, _) = opt_semi(i)?;
    Ok((i, ModuleItem::Initial(name)))
}

fn event_decl(i: &str) -> IResult<&str, ModuleItem> {
    let (i, _) = p_space0(i)?;
    let (i, _) = keyword(i, "event")?;
    let (i, _) = p_space1(i)?;
    let (i, name) = identifier(i)?;
    let (i, variant) = opt_constructor_payload(i, name.clone())?;
    let (i, _) = opt_semi(i)?;
    Ok((
        i,
        ModuleItem::Event(ConstructorGroup {
            name,
            variants: vec![variant],
        }),
    ))
}

fn output_decl(i: &str) -> IResult<&str, ModuleItem> {
    let (i, _) = p_space0(i)?;
    let (i, _) = keyword(i, "output")?;
    let (i, _) = p_space1(i)?;
    let (i, name) = identifier(i)?;
    let (i, variant) = opt_constructor_payload(i, name.clone())?;
    let (i, _) = opt_semi(i)?;
    Ok((
        i,
        ModuleItem::Output(ConstructorGroup {
            name,
            variants: vec![variant],
        }),
    ))
}

fn opt_constructor_payload(i: &str, group_name: String) -> IResult<&str, ConstructorVariant> {
    let (i, _) = p_space0(i)?;
    if let Ok((i_paren, _)) = p_tag("(", i) {
        let (i2, variant) = constructor_variant(i_paren)?;
        let (i3, _) = p_tag(")", i2)?;
        Ok((i3, variant))
    } else {
        Ok((i, ConstructorVariant::Unit { name: group_name }))
    }
}

fn constructor_variant(i: &str) -> IResult<&str, ConstructorVariant> {
    let (i, name) = identifier(i)?;
    let (i, has_inner) = opt_p_tag("(", i)?;
    match has_inner {
        Some(_) => {
            let (i2, fields) = field_list(i)?;
            Ok((i2, ConstructorVariant::Struct { name, fields }))
        }
        None => Ok((i, ConstructorVariant::Unit { name })),
    }
}

fn field_list(i: &str) -> IResult<&str, Vec<FieldDecl>> {
    let mut i = i;
    let mut out = Vec::new();
    loop {
        let (i_ws, _) = p_space0(i)?;
        if let Ok((after, _)) = p_tag(")", i_ws) {
            return Ok((after, out));
        }
        let (i_name, name) = identifier(i_ws)?;
        let (i_colon, _) = p_space0(i_name)?;
        let (i_colon, _) = p_tag(":", i_colon)?;
        let (i_ty, ty) = type_parser(i_colon)?;
        if out.iter().any(|f| f.name == name) {
            return Err(semantic_err(i_ty));
        }
        out.push(FieldDecl { name, ty });
        let (i_after, _) = p_space0(i_ty)?;
        if let Ok((i_comma, _)) = p_tag(",", i_after) {
            i = i_comma;
            continue;
        }
        i = i_after;
    }
}

fn type_parser(i: &str) -> IResult<&str, Type> {
    let (i, _) = p_space0(i)?;
    if let Ok((i2, _)) = keyword(i, "bool") {
        return Ok((i2, Type::Bool));
    }
    if let Ok(res) = bit_type(i) {
        return Ok(res);
    }
    let (i2, name) = identifier(i)?;
    Ok((i2, Type::Enum { name }))
}

fn bit_type(i: &str) -> IResult<&str, Type> {
    let (i, _) = keyword(i, "bit")?;
    let (i, _) = p_tag("<", i)?;
    let (i, width) = int_lit(i)?;
    let (i, _) = p_tag(">", i)?;
    let width = u32::try_from(width).map_err(|_| semantic_err(i))?;
    Ok((i, Type::Bit { width }))
}

fn var_decl(i: &str) -> IResult<&str, ModuleItem> {
    let (i, _) = p_space0(i)?;
    let (i, _) = keyword(i, "var")?;
    let (i, _) = p_space1(i)?;
    let (i, name) = identifier(i)?;
    let (i, _) = p_space0(i)?;
    let (i, _) = p_tag(":", i)?;
    let (i, ty) = type_parser(i)?;
    let (i, _) = p_space1(i)?;
    let (i, _) = keyword(i, "init")?;
    let (i, init) = expr(i)?;
    let (i, _) = opt_semi(i)?;
    Ok((i, ModuleItem::Var(VarDecl { name, ty, init })))
}

fn transition_decl(i: &str) -> IResult<&str, ModuleItem> {
    let (i, _) = p_space0(i)?;
    let (i, _) = keyword(i, "transition")?;
    let (i, _) = p_space1(i)?;
    let (i, state) = identifier(i)?;
    let (i, _) = p_space1(i)?;
    let (i, _) = keyword(i, "on")?;
    let (i, pattern) = event_pattern(i)?;

    let (i_ws, _) = p_space0(i)?;
    let (guard, i_after_guard) = if let Ok((i_where, _)) = keyword(i_ws, "where") {
        let (i_guard, guard) = expr(i_where)?;
        let (i2, _) = p_space0(i_guard)?;
        (Some(guard), i2)
    } else {
        (None, i_ws)
    };

    let (i, body) = action_block(i_after_guard)?;
    let transition = build_transition(state, pattern, guard, body, i)?;
    Ok((i, ModuleItem::Transition(transition)))
}

fn build_transition(
    state: String,
    pattern: EventPattern,
    guard: Option<Expr>,
    body: TransitionBody,
    i: &str,
) -> Result<Transition, Err<nom::error::Error<&str>>> {
    let next_state = body.next_state.ok_or_else(|| semantic_err(i))?;
    if body.assignments.len()
        != body
            .assignments
            .iter()
            .map(|a| a.var.clone())
            .collect::<std::collections::HashSet<_>>()
            .len()
    {
        return Err(semantic_err(i));
    }
    Ok(Transition {
        id: 0,
        state,
        pattern,
        guard,
        next_state,
        emits: body.emits,
        assignments: body.assignments,
        priority: body.priority,
    })
}

fn action_block(i: &str) -> IResult<&str, TransitionBody> {
    let (i, _) = p_space0(i)?;
    let (i, _) = p_tag("{", i)?;
    let mut i = i;
    let mut body = TransitionBody::default();
    loop {
        let (i_ws, _) = p_space0(i)?;
        if let Ok((after, _)) = p_tag("}", i_ws) {
            return Ok((after, body));
        }
        let (i_action, action) = action(i_ws, &body)?;
        body = action;
        let (i_after, _) = p_space0(i_action)?;
        if let Ok((i_semi, _)) = p_tag(";", i_after) {
            i = i_semi;
            continue;
        }
        i = i_after;
    }
}

fn action<'a>(i: &'a str, previous: &TransitionBody) -> IResult<&'a str, TransitionBody> {
    let (i, name) = identifier(i)?;
    match name.as_str() {
        "next" => {
            if previous.next_state.is_some() {
                return Err(semantic_err(i));
            }
            let (i, _) = p_space1(i)?;
            let (i, state) = identifier(i)?;
            let body = TransitionBody {
                next_state: Some(state),
                emits: previous.emits.clone(),
                assignments: previous.assignments.clone(),
                priority: previous.priority,
            };
            Ok((i, body))
        }
        "emit" => {
            let (i, _) = p_space1(i)?;
            let (i, output) = output_expr(i)?;
            let mut body = previous.clone();
            body.emits.push(output);
            Ok((i, body))
        }
        "set" => {
            let (i, _) = p_space1(i)?;
            let (i, var) = identifier(i)?;
            if previous.assignments.iter().any(|a| a.var == var) {
                return Err(semantic_err(i));
            }
            let (i, _) = p_space0(i)?;
            let (i, _) = p_tag("=", i)?;
            let (i, value) = expr(i)?;
            let mut body = previous.clone();
            body.assignments.push(Assignment { var, value });
            Ok((i, body))
        }
        "priority" => {
            if previous.priority.is_some() {
                return Err(semantic_err(i));
            }
            let (i, _) = p_space1(i)?;
            let (i, value) = int_lit(i)?;
            let value = u32::try_from(value).map_err(|_| semantic_err(i))?;
            let mut body = previous.clone();
            body.priority = Some(value);
            Ok((i, body))
        }
        _ => Err(syntax_err(i)),
    }
}

fn prop_decl(i: &str) -> IResult<&str, ModuleItem> {
    let (i, _) = p_space0(i)?;
    let (i, _) = keyword(i, "prop")?;
    let (i, _) = p_space1(i)?;
    let (i, name) = identifier(i)?;
    let (i, _) = p_space0(i)?;
    let (i, _) = p_tag(":", i)?;
    let (i, formula) = formula(i)?;
    let (i, _) = opt_semi(i)?;
    Ok((i, ModuleItem::Prop(Property { name, formula })))
}

fn assume_decl(i: &str) -> IResult<&str, ModuleItem> {
    let (i, _) = p_space0(i)?;
    let (i, _) = keyword(i, "assume")?;
    let (i, _) = p_space1(i)?;
    let (i, name) = identifier(i)?;
    let (i, _) = p_space0(i)?;
    let (i, _) = p_tag(":", i)?;
    let (i, formula) = formula(i)?;
    let (i, _) = opt_semi(i)?;
    Ok((i, ModuleItem::Assume(Property { name, formula })))
}

fn on_error_decl(i: &str) -> IResult<&str, ModuleItem> {
    let (i, _) = p_space0(i)?;
    let (i, _) = keyword(i, "on_error")?;
    let (i, _) = p_space1(i)?;
    let (i, _) = p_tag("{", i)?;
    let mut i = i;
    let mut next_state: Option<String> = None;
    let mut emits: Vec<OutputExpr> = Vec::new();
    loop {
        let (i_ws, _) = p_space0(i)?;
        if let Ok((after, _)) = p_tag("}", i_ws) {
            let Some(state) = next_state else {
                return Err(semantic_err(i_ws)); // on_error requires `next`
            };
            return Ok((
                after,
                ModuleItem::OnError(OnError {
                    next_state: state,
                    emits,
                }),
            ));
        }
        let (i_act, name) = identifier(i_ws)?;
        match name.as_str() {
            "next" => {
                if next_state.is_some() {
                    return Err(semantic_err(i_act));
                }
                let (i2, _) = p_space1(i_act)?;
                let (i3, state) = identifier(i2)?;
                next_state = Some(state);
                i = i3;
            }
            "emit" => {
                let (i2, _) = p_space1(i_act)?;
                let (i3, o) = output_expr(i2)?;
                emits.push(o);
                i = i3;
            }
            _ => return Err(semantic_err(i_act)),
        }
        let (i_after, _) = p_space0(i)?;
        if let Ok((i_semi, _)) = p_tag(";", i_after) {
            i = i_semi;
            continue;
        }
        i = i_after;
    }
}

fn build_module(name: String, items: Vec<ModuleItem>, i: &str) -> IResult<&str, Module> {
    let mut module = Module {
        name,
        ..Default::default()
    };

    for item in items {
        match item {
            ModuleItem::Enum(e) => {
                if module.enums.iter().any(|x| x.name == e.name) {
                    return Err(semantic_err(i));
                }
                module.enums.push(e);
            }
            ModuleItem::Initial(s) => {
                if module.initial_state.is_some() {
                    return Err(semantic_err(i));
                }
                module.initial_state = Some(s);
            }
            ModuleItem::Event(g) => merge_group(&mut module.events, g, i)?,
            ModuleItem::Output(g) => merge_group(&mut module.outputs, g, i)?,
            ModuleItem::Var(v) => {
                if module.vars.iter().any(|x| x.name == v.name) {
                    return Err(semantic_err(i));
                }
                module.vars.push(v);
            }
            ModuleItem::Transition(mut t) => {
                t.id = module.transitions.len();
                module.transitions.push(t);
            }
            ModuleItem::Prop(p) => {
                if module.properties.iter().any(|x| x.name == p.name)
                    || module.assumptions.iter().any(|x| x.name == p.name)
                {
                    return Err(semantic_err(i));
                }
                module.properties.push(p);
            }
            ModuleItem::OnError(o) => {
                if module.on_error.is_some() {
                    return Err(semantic_err(i));
                }
                module.on_error = Some(o);
            }
            ModuleItem::Assume(p) => {
                if module.properties.iter().any(|x| x.name == p.name)
                    || module.assumptions.iter().any(|x| x.name == p.name)
                {
                    return Err(semantic_err(i));
                }
                module.assumptions.push(p);
            }
        }
    }

    Ok((i, module))
}

fn merge_group<'i>(
    groups: &mut Vec<ConstructorGroup>,
    group: ConstructorGroup,
    i: &'i str,
) -> Result<(), Err<nom::error::Error<&'i str>>> {
    if let Some(existing) = groups.iter_mut().find(|g| g.name == group.name) {
        for v in group.variants {
            let duplicate = match &v {
                ConstructorVariant::Unit { name } => existing
                    .variants
                    .iter()
                    .any(|x| matches!(x, ConstructorVariant::Unit { name: n } if n == name)),
                ConstructorVariant::Struct { name, .. } => existing
                    .variants
                    .iter()
                    .any(|x| matches!(x, ConstructorVariant::Struct { name: n, .. } if n == name)),
            };
            if duplicate {
                return Err(semantic_err(i));
            }
            existing.variants.push(v);
        }
    } else {
        groups.push(group);
    }
    Ok(())
}

fn event_pattern(i: &str) -> IResult<&str, EventPattern> {
    let (i, (group, variant)) = group_pattern(i)?;
    Ok((i, EventPattern { group, variant }))
}

fn output_pattern(i: &str) -> IResult<&str, OutputPattern> {
    let (i, (group, variant)) = group_pattern(i)?;
    Ok((i, OutputPattern { group, variant }))
}

fn group_pattern(i: &str) -> IResult<&str, (String, Option<PatternVariant>)> {
    let (i, _) = p_space0(i)?;
    let (i, group) = identifier(i)?;
    let (i, has_outer) = opt_p_tag("(", i)?;
    match has_outer {
        Some(_) => {
            let (i, vname) = identifier(i)?;
            let (i, has_inner) = opt_p_tag("(", i)?;
            let (i, variant) = match has_inner {
                Some(_) => {
                    let (i_fields, fields) = pattern_field_list(i)?;
                    (
                        i_fields,
                        PatternVariant::Struct {
                            name: vname,
                            fields,
                        },
                    )
                }
                None => (i, PatternVariant::Unit { name: vname }),
            };
            let (i, _) = p_tag(")", i)?;
            Ok((i, (group, Some(variant))))
        }
        None => Ok((i, (group, None))),
    }
}

fn pattern_field_list(i: &str) -> IResult<&str, Vec<PatternField>> {
    let mut i = i;
    let mut out = Vec::new();
    loop {
        let (i_ws, _) = p_space0(i)?;
        if let Ok((after, _)) = p_tag(")", i_ws) {
            return Ok((after, out));
        }
        let (i_field, field) = pattern_field(i_ws)?;
        out.push(field);
        let (i_after, _) = p_space0(i_field)?;
        if let Ok((i_comma, _)) = p_tag(",", i_after) {
            i = i_comma;
            continue;
        }
        i = i_after;
    }
}

fn pattern_field(i: &str) -> IResult<&str, PatternField> {
    let (i, _) = p_space0(i)?;
    if let Ok((i2, _)) = p_tag("_", i) {
        return Ok((
            i2,
            PatternField {
                name: None,
                pattern: PatternExpr::Wildcard,
            },
        ));
    }
    if let Ok((i2, value)) = int_lit(i) {
        return Ok((
            i2,
            PatternField {
                name: None,
                pattern: PatternExpr::Int { value },
            },
        ));
    }
    if let Ok((i2, _)) = soft_keyword(i, "true") {
        return Ok((
            i2,
            PatternField {
                name: None,
                pattern: PatternExpr::Bool { value: true },
            },
        ));
    }
    if let Ok((i2, _)) = soft_keyword(i, "false") {
        return Ok((
            i2,
            PatternField {
                name: None,
                pattern: PatternExpr::Bool { value: false },
            },
        ));
    }

    let (i, name) = identifier(i)?;
    let (i_colon, _) = p_space0(i)?;
    if let Ok((i3, _)) = p_tag(":", i_colon) {
        let (i4, pattern) = pattern_expr(i3)?;
        Ok((
            i4,
            PatternField {
                name: Some(name),
                pattern,
            },
        ))
    } else {
        Ok((
            i_colon,
            PatternField {
                name: Some(name.clone()),
                pattern: PatternExpr::Bind { name },
            },
        ))
    }
}

fn pattern_expr(i: &str) -> IResult<&str, PatternExpr> {
    let (i, _) = p_space0(i)?;
    if let Ok((i2, _)) = p_tag("_", i) {
        return Ok((i2, PatternExpr::Wildcard));
    }
    if let Ok((i2, value)) = int_lit(i) {
        return Ok((i2, PatternExpr::Int { value }));
    }
    if let Ok((i2, _)) = soft_keyword(i, "true") {
        return Ok((i2, PatternExpr::Bool { value: true }));
    }
    if let Ok((i2, _)) = soft_keyword(i, "false") {
        return Ok((i2, PatternExpr::Bool { value: false }));
    }

    let (i2, path) = ident_path(i)?;
    if path.len() == 1 {
        Ok((
            i2,
            PatternExpr::Bind {
                name: path[0].clone(),
            },
        ))
    } else {
        Ok((i2, PatternExpr::EnumVariant { path }))
    }
}

fn output_expr(i: &str) -> IResult<&str, OutputExpr> {
    let (i, _) = p_space0(i)?;
    let (i, group) = identifier(i)?;
    let (i, has_outer) = opt_p_tag("(", i)?;
    match has_outer {
        Some(_) => {
            let (i, vname) = identifier(i)?;
            let (i, has_inner) = opt_p_tag("(", i)?;
            let (i, args) = match has_inner {
                Some(_) => expr_list(i)?,
                None => (i, Vec::new()),
            };
            let (i, _) = p_tag(")", i)?;
            Ok((
                i,
                OutputExpr {
                    group,
                    variant: Some(vname),
                    args,
                },
            ))
        }
        None => Ok((
            i,
            OutputExpr {
                group,
                variant: None,
                args: Vec::new(),
            },
        )),
    }
}

fn expr_list(i: &str) -> IResult<&str, Vec<Expr>> {
    let mut i = i;
    let mut out = Vec::new();
    loop {
        let (i_ws, _) = p_space0(i)?;
        if let Ok((after, _)) = p_tag(")", i_ws) {
            return Ok((after, out));
        }
        let (i_expr, e) = expr(i_ws)?;
        out.push(e);
        let (i_after, _) = p_space0(i_expr)?;
        if let Ok((i_comma, _)) = p_tag(",", i_after) {
            i = i_comma;
            continue;
        }
        i = i_after;
    }
}

fn expr(i: &str) -> IResult<&str, Expr> {
    logic_or(i)
}

fn logic_or(i: &str) -> IResult<&str, Expr> {
    let (mut i, mut left) = logic_and(i)?;
    loop {
        let (i_ws, _) = p_space0(i)?;
        if let Ok((i_op, _)) = p_tag("||", i_ws) {
            let (i2, right) = logic_and(i_op)?;
            left = Expr::Binary {
                op: BinOp::LogicOr,
                left: Box::new(left),
                right: Box::new(right),
            };
            i = i2;
        } else {
            return Ok((i_ws, left));
        }
    }
}

fn logic_and(i: &str) -> IResult<&str, Expr> {
    let (mut i, mut left) = equality(i)?;
    loop {
        let (i_ws, _) = p_space0(i)?;
        if let Ok((i_op, _)) = p_tag("&&", i_ws) {
            let (i2, right) = equality(i_op)?;
            left = Expr::Binary {
                op: BinOp::LogicAnd,
                left: Box::new(left),
                right: Box::new(right),
            };
            i = i2;
        } else {
            return Ok((i_ws, left));
        }
    }
}

fn equality(i: &str) -> IResult<&str, Expr> {
    let (mut i, mut left) = relational(i)?;
    loop {
        let (i_ws, _) = p_space0(i)?;
        if let Ok((i_op, _)) = p_tag("==", i_ws) {
            let (i2, right) = relational(i_op)?;
            left = Expr::Binary {
                op: BinOp::Eq,
                left: Box::new(left),
                right: Box::new(right),
            };
            i = i2;
        } else if let Ok((i_op, _)) = p_tag("!=", i_ws) {
            let (i2, right) = relational(i_op)?;
            left = Expr::Binary {
                op: BinOp::NotEq,
                left: Box::new(left),
                right: Box::new(right),
            };
            i = i2;
        } else {
            return Ok((i_ws, left));
        }
    }
}

fn relational(i: &str) -> IResult<&str, Expr> {
    let (mut i, mut left) = bitwise_or(i)?;
    loop {
        let (i_ws, _) = p_space0(i)?;
        if let Ok((i_op, _)) = p_tag("<=", i_ws) {
            let (i2, right) = bitwise_or(i_op)?;
            left = Expr::Binary {
                op: BinOp::Le,
                left: Box::new(left),
                right: Box::new(right),
            };
            i = i2;
        } else if let Ok((i_op, _)) = p_tag("<", i_ws) {
            if i_op.starts_with('<') {
                return Ok((i_ws, left));
            }
            let (i2, right) = bitwise_or(i_op)?;
            left = Expr::Binary {
                op: BinOp::Lt,
                left: Box::new(left),
                right: Box::new(right),
            };
            i = i2;
        } else if let Ok((i_op, _)) = p_tag(">=", i_ws) {
            let (i2, right) = bitwise_or(i_op)?;
            left = Expr::Binary {
                op: BinOp::Ge,
                left: Box::new(left),
                right: Box::new(right),
            };
            i = i2;
        } else if let Ok((i_op, _)) = p_tag(">", i_ws) {
            if i_op.starts_with('>') {
                return Ok((i_ws, left));
            }
            let (i2, right) = bitwise_or(i_op)?;
            left = Expr::Binary {
                op: BinOp::Gt,
                left: Box::new(left),
                right: Box::new(right),
            };
            i = i2;
        } else {
            return Ok((i_ws, left));
        }
    }
}

fn bitwise_or(i: &str) -> IResult<&str, Expr> {
    let (mut i, mut left) = bitwise_xor(i)?;
    loop {
        let (i_ws, _) = p_space0(i)?;
        if let Ok((i_op, _)) = p_tag("|", i_ws) {
            if i_op.starts_with('|') {
                return Ok((i_ws, left));
            }
            let (i2, right) = bitwise_xor(i_op)?;
            left = Expr::Binary {
                op: BinOp::BitOr,
                left: Box::new(left),
                right: Box::new(right),
            };
            i = i2;
        } else {
            return Ok((i_ws, left));
        }
    }
}

fn bitwise_xor(i: &str) -> IResult<&str, Expr> {
    let (mut i, mut left) = bitwise_and(i)?;
    loop {
        let (i_ws, _) = p_space0(i)?;
        if let Ok((i_op, _)) = p_tag("^", i_ws) {
            let (i2, right) = bitwise_and(i_op)?;
            left = Expr::Binary {
                op: BinOp::BitXor,
                left: Box::new(left),
                right: Box::new(right),
            };
            i = i2;
        } else {
            return Ok((i_ws, left));
        }
    }
}

fn bitwise_and(i: &str) -> IResult<&str, Expr> {
    let (mut i, mut left) = shift(i)?;
    loop {
        let (i_ws, _) = p_space0(i)?;
        if let Ok((i_op, _)) = p_tag("&", i_ws) {
            if i_op.starts_with('&') {
                return Ok((i_ws, left));
            }
            let (i2, right) = shift(i_op)?;
            left = Expr::Binary {
                op: BinOp::BitAnd,
                left: Box::new(left),
                right: Box::new(right),
            };
            i = i2;
        } else {
            return Ok((i_ws, left));
        }
    }
}

fn shift(i: &str) -> IResult<&str, Expr> {
    let (mut i, mut left) = additive(i)?;
    loop {
        let (i_ws, _) = p_space0(i)?;
        if let Ok((i_op, _)) = p_tag("<<", i_ws) {
            let (i2, right) = additive(i_op)?;
            left = Expr::Binary {
                op: BinOp::ShiftLeft,
                left: Box::new(left),
                right: Box::new(right),
            };
            i = i2;
        } else if let Ok((i_op, _)) = p_tag(">>", i_ws) {
            let (i2, right) = additive(i_op)?;
            left = Expr::Binary {
                op: BinOp::ShiftRight,
                left: Box::new(left),
                right: Box::new(right),
            };
            i = i2;
        } else {
            return Ok((i_ws, left));
        }
    }
}

fn additive(i: &str) -> IResult<&str, Expr> {
    let (mut i, mut left) = multiplicative(i)?;
    loop {
        let (i_ws, _) = p_space0(i)?;
        if let Ok((i_op, _)) = p_tag("+", i_ws) {
            let (i2, right) = multiplicative(i_op)?;
            left = Expr::Binary {
                op: BinOp::Plus,
                left: Box::new(left),
                right: Box::new(right),
            };
            i = i2;
        } else if let Ok((i_op, _)) = p_tag("-", i_ws) {
            if i_op.starts_with('>') {
                return Ok((i_ws, left));
            }
            let (i2, right) = multiplicative(i_op)?;
            left = Expr::Binary {
                op: BinOp::Minus,
                left: Box::new(left),
                right: Box::new(right),
            };
            i = i2;
        } else {
            return Ok((i_ws, left));
        }
    }
}

fn multiplicative(i: &str) -> IResult<&str, Expr> {
    let (mut i, mut left) = unary_expr(i)?;
    loop {
        let (i_ws, _) = p_space0(i)?;
        if let Ok((i_op, _)) = p_tag("*", i_ws) {
            let (i2, right) = unary_expr(i_op)?;
            left = Expr::Binary {
                op: BinOp::Star,
                left: Box::new(left),
                right: Box::new(right),
            };
            i = i2;
        } else {
            return Ok((i_ws, left));
        }
    }
}

fn unary_expr(i: &str) -> IResult<&str, Expr> {
    let (i, _) = p_space0(i)?;
    if let Ok((i_op, _)) = p_tag("!", i) {
        let (i2, e) = unary_expr(i_op)?;
        return Ok((
            i2,
            Expr::Unary {
                op: UnaryOp::Not,
                expr: Box::new(e),
            },
        ));
    }
    if let Ok((i_op, _)) = p_tag("~", i) {
        let (i2, e) = unary_expr(i_op)?;
        return Ok((
            i2,
            Expr::Unary {
                op: UnaryOp::BitNot,
                expr: Box::new(e),
            },
        ));
    }
    if let Ok((i_op, _)) = p_tag("-", i) {
        let (i2, e) = unary_expr(i_op)?;
        return Ok((
            i2,
            Expr::Unary {
                op: UnaryOp::Neg,
                expr: Box::new(e),
            },
        ));
    }
    primary_expr(i)
}

fn primary_expr(i: &str) -> IResult<&str, Expr> {
    let (i, _) = p_space0(i)?;
    if let Ok((i2, value)) = int_lit(i) {
        return Ok((i2, Expr::Int { value }));
    }
    if let Ok((i2, _)) = soft_keyword(i, "true") {
        return Ok((i2, Expr::Bool { value: true }));
    }
    if let Ok((i2, _)) = soft_keyword(i, "false") {
        return Ok((i2, Expr::Bool { value: false }));
    }
    if let Ok((i2, _)) = p_tag("(", i) {
        let (i3, e) = expr(i2)?;
        let (i4, _) = p_tag(")", i3)?;
        return Ok((i4, e));
    }
    let (i2, path) = ident_path(i)?;
    Ok((i2, Expr::Path { path }))
}

fn formula(i: &str) -> IResult<&str, Formula> {
    or_formula(i)
}

fn or_formula(i: &str) -> IResult<&str, Formula> {
    let (mut i, mut left) = impl_formula(i)?;
    loop {
        let (i_ws, _) = p_space0(i)?;
        if let Ok((i_op, _)) = p_tag("||", i_ws) {
            let (i2, right) = impl_formula(i_op)?;
            left = Formula::Or {
                left: Box::new(left),
                right: Box::new(right),
            };
            i = i2;
        } else {
            return Ok((i_ws, left));
        }
    }
}

fn impl_formula(i: &str) -> IResult<&str, Formula> {
    let (i, left) = and_formula(i)?;
    let (i_ws, _) = p_space0(i)?;
    if let Ok((i_op, _)) = p_tag("->", i_ws) {
        let (i2, right) = impl_formula(i_op)?;
        Ok((
            i2,
            Formula::Implies {
                left: Box::new(left),
                right: Box::new(right),
            },
        ))
    } else {
        Ok((i_ws, left))
    }
}

fn and_formula(i: &str) -> IResult<&str, Formula> {
    let (mut i, mut left) = unary_formula(i)?;
    loop {
        let (i_ws, _) = p_space0(i)?;
        if let Ok((i_op, _)) = p_tag("&&", i_ws) {
            let (i2, right) = unary_formula(i_op)?;
            left = Formula::And {
                left: Box::new(left),
                right: Box::new(right),
            };
            i = i2;
        } else {
            return Ok((i_ws, left));
        }
    }
}

fn unary_formula(i: &str) -> IResult<&str, Formula> {
    let (i, _) = p_space0(i)?;
    if let Ok((i_op, _)) = p_tag("!", i) {
        let (i2, inner) = unary_formula(i_op)?;
        return Ok((
            i2,
            Formula::Not {
                inner: Box::new(inner),
            },
        ));
    }
    primary_formula(i)
}

fn primary_formula(i: &str) -> IResult<&str, Formula> {
    let (i, _) = p_space0(i)?;
    if let Ok((i2, _)) = soft_keyword(i, "next") {
        let (i2, _) = p_space0(i2)?;
        let (i3, _) = p_tag("(", i2)?;
        let (i4, inner) = formula(i3)?;
        let (i5, _) = p_tag(")", i4)?;
        return Ok((
            i5,
            Formula::Next {
                inner: Box::new(inner),
            },
        ));
    }
    if let Ok((i2, _)) = soft_keyword(i, "always") {
        let (i2, _) = p_space0(i2)?;
        // Optional `[N]` bound: `always[N] (...)` (Phase 3). Absent N keeps
        // the unbounded `always (...)`.
        let mut bound: Option<u64> = None;
        let mut i_after_kw = i2;
        if let Ok((ib, _)) = p_tag("[", i2) {
            let (ib, b) = int_lit(ib)?;
            let (ib, _) = p_tag("]", ib)?;
            bound = Some(b);
            i_after_kw = ib;
        }
        let (i3, _) = p_space0(i_after_kw)?;
        let (i3, _) = p_tag("(", i3)?;
        let (i4, inner) = formula(i3)?;
        let (i5, _) = p_tag(")", i4)?;
        return Ok((
            i5,
            match bound {
                Some(b) => Formula::BoundedAlways {
                    bound: b,
                    inner: Box::new(inner),
                },
                None => Formula::Always {
                    inner: Box::new(inner),
                },
            },
        ));
    }
    if let Ok((i2, _)) = soft_keyword(i, "bounded_eventually") {
        let (i2, _) = p_space0(i2)?;
        let (i3, _) = p_tag("[", i2)?;
        let (i4, bound) = int_lit(i3)?;
        let (i5, _) = p_tag("]", i4)?;
        let (i6, _) = p_space0(i5)?;
        let (i7, _) = p_tag("(", i6)?;
        let (i8, inner) = formula(i7)?;
        let (i9, _) = p_tag(")", i8)?;
        return Ok((
            i9,
            Formula::BoundedEventually {
                bound,
                inner: Box::new(inner),
            },
        ));
    }
    if let Ok((i2, _)) = soft_keyword(i, "input_is") {
        let (i2, _) = p_space0(i2)?;
        let (i3, _) = p_tag("(", i2)?;
        let (i4, pattern) = event_pattern(i3)?;
        let (i5, _) = p_tag(")", i4)?;
        return Ok((i5, Formula::InputIs { pattern }));
    }
    if let Ok((i2, _)) = soft_keyword(i, "output_has") {
        let (i2, _) = p_space0(i2)?;
        let (i3, _) = p_tag("(", i2)?;
        let (i4, pattern) = output_pattern(i3)?;
        let (i5, _) = p_tag(")", i4)?;
        return Ok((i5, Formula::OutputHas { pattern }));
    }
    if let Ok((i2, _)) = p_tag("(", i) {
        let (i3, inner) = formula(i2)?;
        let (i4, _) = p_tag(")", i3)?;
        return Ok((i4, inner));
    }
    // Fallthrough: a bare non-keyword operand is a NON-BOOLEAN expression
    // (comparisons and below). The full `expr` grammar must NOT be used here:
    // its `logic_or`/`logic_and` levels would mis-consume identifier-looking
    // formula atoms as variable paths - in `state == S && output_has(O)` the
    // old fallthrough absorbed `output_has` as an `Expr::Path` and left
    // `(O)` as trailing input. Formula-level `&&`/`||` are handled by
    // `and_formula`/`or_formula`, each operand parsed independently via
    // `primary_formula`, so `output_has(...)` reaches its keyword branch.
    let (i2, e) = non_boolean_expr(i)?;
    Ok((i2, Formula::Expr { expr: e }))
}

/// Non-boolean expression level (comparisons and below): the fallthrough
/// entry point for formula operands. See the comment at the call site in
/// `primary_formula` for why the full `expr` grammar is forbidden here.
fn non_boolean_expr(i: &str) -> IResult<&str, Expr> {
    equality(i)
}

fn identifier(i: &str) -> IResult<&str, String> {
    let (i, first) = ident_start(i)?;
    let (i, rest) = many0(ident_cont).parse_complete(i)?;
    let mut s = String::with_capacity(rest.len() + 1);
    s.push(first);
    s.extend(rest);
    Ok((i, s))
}

fn ident_start(i: &str) -> IResult<&str, char> {
    satisfy(|c: char| c.is_ascii_alphabetic() || c == '_')(i)
}

fn ident_cont(i: &str) -> IResult<&str, char> {
    satisfy(|c: char| c.is_ascii_alphanumeric() || c == '_')(i)
}

fn digit(i: &str) -> IResult<&str, char> {
    satisfy(|c: char| c.is_ascii_digit())(i)
}

fn ident_path(i: &str) -> IResult<&str, Vec<String>> {
    let (i, first) = identifier(i)?;
    let mut path = vec![first];
    let mut i = i;
    loop {
        let (i_ws, _) = p_space0(i)?;
        if let Ok((i2, _)) = p_tag("::", i_ws) {
            let (i3, next) = identifier(i2)?;
            path.push(next);
            i = i3;
        } else {
            return Ok((i_ws, path));
        }
    }
}

fn keyword<'a>(i: &'a str, kw: &'static str) -> IResult<&'a str, ()> {
    let (i, word) = identifier(i)?;
    if word == kw {
        Ok((i, ()))
    } else {
        Err(syntax_err(i))
    }
}

fn soft_keyword<'a>(i: &'a str, kw: &'static str) -> IResult<&'a str, ()> {
    let (i, word) = identifier(i)?;
    if word == kw {
        Ok((i, ()))
    } else {
        Err(syntax_err(i))
    }
}

fn int_lit(i: &str) -> IResult<&str, u64> {
    let (i, digits) = many1(digit).parse_complete(i)?;
    let s: String = digits.into_iter().collect();
    match s.parse::<u64>() {
        Ok(v) => Ok((i, v)),
        Err(_) => Err(semantic_err(i)),
    }
}

fn opt_semi(i: &str) -> IResult<&str, ()> {
    let (i, _) = p_space0(i)?;
    let (i, _) = opt_p_tag(";", i)?;
    Ok((i, ()))
}

fn p_tag<'a>(t: &'a str, i: &'a str) -> IResult<&'a str, ()> {
    tag(t)(i).map(|(rest, _)| (rest, ()))
}

fn opt_p_tag<'a>(t: &'a str, i: &'a str) -> IResult<&'a str, Option<()>> {
    let p = tag(t);
    opt(p)
        .parse_complete(i)
        .map(|(rest, x)| (rest, x.map(|_| ())))
}

fn p_space0(i: &str) -> IResult<&str, ()> {
    multispace0(i).map(|(rest, _)| (rest, ()))
}

fn p_space1(i: &str) -> IResult<&str, ()> {
    multispace1(i).map(|(rest, _)| (rest, ()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::printer;

    #[test]
    fn parses_minimal_module() {
        let source =
            "module m { enum State { A } initial A event Go transition A on Go { next A; } }";
        let ir = parse_source(source).expect("parse minimal module");
        assert_eq!(ir.modules.len(), 1);
        assert_eq!(ir.modules[0].name, "m");
        assert_eq!(ir.modules[0].transitions.len(), 1);
        assert_eq!(ir.modules[0].transitions[0].next_state, "A");
        // Round-trips: rfs -> json -> rfs and json -> rfs -> json.
        printer::roundtrip_rfs(source).expect("rfs round-trip");
    }

    #[test]
    fn parses_grouped_event_with_payload() {
        let source = "module m { enum State { A } initial A event Recv(Ack(x: bit<4>)) transition A on Recv(Ack(x)) where x == 0 { next A; } }";
        let ir = parse_source(source).expect("parse grouped event");
        let module = &ir.modules[0];
        assert_eq!(module.events.len(), 1);
        assert_eq!(module.events[0].name, "Recv");
        assert_eq!(module.transitions[0].pattern.group, "Recv");
        printer::roundtrip_rfs(source).expect("rfs round-trip");
    }

    #[test]
    fn parses_temporal_formula() {
        let source = "module m { enum State { A } initial A event E prop p : always (input_is(E) -> next (state == A)) ; }";
        let ir = parse_source(source).expect("parse formula");
        assert_eq!(ir.modules[0].properties.len(), 1);
        printer::roundtrip_rfs(source).expect("rfs round-trip");
    }

    // -- Task 23: formula-level boolean operators with mixed atom kinds ----

    #[test]
    fn mixed_conjunction_state_comparison_left_parses() {
        // The exact pre-fix failing shape: a bare state comparison on the
        // LEFT of `&&` with a non-comparison right operand. Before the fix
        // the expr fallthrough absorbed `output_has` as a variable path and
        // left `(O)` as trailing input.
        let source =
            "module m { enum State { A } initial A event E output O prop p : state == A && output_has(O) ; }";
        let ir = parse_source(source).expect("mixed conjunction must parse");
        match &ir.modules[0].properties[0].formula {
            Formula::And { left, right } => {
                assert!(
                    matches!(&**left, Formula::Expr { expr }
                        if matches!(expr, Expr::Binary { op: BinOp::Eq, .. })),
                    "left operand must be the state comparison, got {:?}",
                    left
                );
                assert!(
                    matches!(&**right, Formula::OutputHas { .. }),
                    "right operand must be output_has, got {:?}",
                    right
                );
            }
            other => panic!("expected formula-level And, got {:?}", other),
        }
        printer::roundtrip_rfs(source).expect("rfs round-trip");
    }

    #[test]
    fn mixed_disjunction_state_comparison_left_parses() {
        let source =
            "module m { enum State { A } initial A event E output O prop p : state == A || output_has(O) ; }";
        let ir = parse_source(source).expect("mixed disjunction must parse");
        match &ir.modules[0].properties[0].formula {
            Formula::Or { left, right } => {
                assert!(
                    matches!(&**left, Formula::Expr { .. }),
                    "left must be the state comparison, got {:?}",
                    left
                );
                assert!(
                    matches!(&**right, Formula::OutputHas { .. }),
                    "right must be output_has, got {:?}",
                    right
                );
            }
            other => panic!("expected formula-level Or, got {:?}", other),
        }
        printer::roundtrip_rfs(source).expect("rfs round-trip");
    }

    #[test]
    fn pre_fix_working_shapes_still_parse() {
        // Shapes that already worked before the fix must parse unchanged:
        // reordered operands, left-parenthesized state comparison, and
        // comparison/comparison mixes.
        let shapes: [(&str, bool); 4] = [
            ("output_has(O) && state == A", true),
            ("(state == A) && output_has(O)", true),
            ("state == A && state == B", true),
            ("state != A || state == B", false),
        ];
        for (formula, is_and) in shapes {
            let source = format!(
                "module m {{ enum State {{ A, B }} initial A event E output O prop p : {} ; }}",
                formula
            );
            let ir =
                parse_source(&source).unwrap_or_else(|e| panic!("must parse '{}': {}", formula, e));
            let top_is_and = match &ir.modules[0].properties[0].formula {
                Formula::And { .. } => true,
                Formula::Or { .. } => false,
                other => panic!("'{}': expected And/Or top node, got {:?}", formula, other),
            };
            assert_eq!(
                top_is_and, is_and,
                "'{}': wrong top boolean operator",
                formula
            );
        }
    }

    #[test]
    fn pure_expr_conjunction_parses_as_formula_and() {
        // Task 23 consequence: a pure-expression conjunction in a property now
        // parses as Formula::And(Expr, Expr) instead of
        // Expr(Binary(LogicAnd)) - the old shape is only reachable from JSON.
        let source = "module m { enum State { A } initial A prop p : x && y ; }";
        let ir = parse_source(source).expect("pure-expression conjunction must parse");
        match &ir.modules[0].properties[0].formula {
            Formula::And { left, right } => {
                assert!(
                    matches!(&**left, Formula::Expr { expr } if matches!(expr, Expr::Path { .. })),
                    "left must be a bare path, got {:?}",
                    left
                );
                assert!(
                    matches!(&**right, Formula::Expr { expr } if matches!(expr, Expr::Path { .. })),
                    "right must be a bare path, got {:?}",
                    right
                );
            }
            other => panic!("expected Formula::And of two Exprs, got {:?}", other),
        }
    }

    #[test]
    fn formula_level_precedence_and_tighter_than_or() {
        // `a && b || c` -> Or(And(a, b), c) at the formula level.
        let source = "module m { enum State { A } initial A prop p : x && y || z ; }";
        let ir = parse_source(source).expect("parse");
        match &ir.modules[0].properties[0].formula {
            Formula::Or { left, right } => {
                assert!(
                    matches!(&**left, Formula::And { .. }),
                    "left must be the And group, got {:?}",
                    left
                );
                assert!(
                    matches!(&**right, Formula::Expr { .. }),
                    "right must be a bare expr, got {:?}",
                    right
                );
            }
            other => panic!("expected Or(And(..), ..), got {:?}", other),
        }
    }

    #[test]
    fn comparison_mix_keeps_both_comparisons() {
        // `x == y && z == w` keeps both comparisons intact as separate
        // formula operands.
        let source = "module m { enum State { A } initial A prop p : x == y && z == w ; }";
        let ir = parse_source(source).expect("parse");
        match &ir.modules[0].properties[0].formula {
            Formula::And { left, right } => {
                assert!(
                    matches!(&**left, Formula::Expr { expr } if matches!(expr, Expr::Binary { op: BinOp::Eq, .. })),
                    "left must be the first comparison, got {:?}",
                    left
                );
                assert!(
                    matches!(&**right, Formula::Expr { expr } if matches!(expr, Expr::Binary { op: BinOp::Eq, .. })),
                    "right must be the second comparison, got {:?}",
                    right
                );
            }
            other => panic!("expected And of two comparisons, got {:?}", other),
        }
    }

    #[test]
    fn parses_bounded_always() {
        let source = "module m { enum State { A } initial A event E prop p : always[3] (state == A) ; prop q : always (input_is(E) -> always[2] (state == A)) ; }";
        let ir = parse_source(source).expect("parse bounded always");
        let module = &ir.modules[0];
        assert_eq!(module.properties.len(), 2);
        match &module.properties[0].formula {
            Formula::BoundedAlways { bound, inner } => {
                assert_eq!(*bound, 3);
                assert!(matches!(inner.as_ref(), Formula::Expr { .. }));
            }
            other => panic!("expected BoundedAlways, got {:?}", other),
        }
        // Unbounded `always` keeps parsing as Always.
        match &module.properties[1].formula {
            Formula::Always { inner } => {
                assert!(matches!(inner.as_ref(), Formula::Implies { .. }))
            }
            other => panic!("expected Always, got {:?}", other),
        }
        printer::roundtrip_rfs(source).expect("rfs round-trip");
    }

    #[test]
    fn ir_json_roundtrip() {
        let source = "module m { enum State { A, B } initial A event E(Value(x: bit<8>)) output O transition A on E(Value(x)) where x == 0 { next B; emit O; } prop p : always (state == A) ; }";
        let ir = parse_source(source).expect("parse source");
        let json = serde_json::to_string(&ir).expect("serialize ir");
        let back: Ir = serde_json::from_str(&json).expect("deserialize ir");
        assert_eq!(ir, back);
        // Full round-trips through the RFS printer as well.
        printer::roundtrip_rfs(source).expect("rfs round-trip");
        printer::roundtrip_json(&json).expect("json round-trip");
    }

    #[test]
    fn duplicate_next_fails() {
        let source = "module m { enum State { A, B } initial A event E transition A on E { next B; next A; } }";
        assert!(parse_source(source).is_err());
    }

    #[test]
    fn missing_next_fails() {
        let source = "module m { enum State { A, B } initial A event E transition A on E { } }";
        assert!(parse_source(source).is_err());
    }

    #[test]
    fn parses_version_header() {
        let source = "module m version \"0.9\" { enum State { A } initial A }";
        let ir = parse_source(source).expect("parse versioned module");
        assert_eq!(ir.modules[0].version.as_deref(), Some("0.9"));
        printer::roundtrip_rfs(source).expect("rfs round-trip");
    }

    #[test]
    fn parses_on_error_block_with_emit() {
        let source = "module m { enum State { A, B } initial A event E output O                       transition A on E { next B; } on_error { next A; emit O; } }";
        let ir = parse_source(source).expect("parse on_error module");
        let oe = ir.modules[0].on_error.as_ref().expect("on_error present");
        assert_eq!(oe.next_state, "A");
        assert_eq!(oe.emits.len(), 1);
        printer::roundtrip_rfs(source).expect("rfs round-trip");
    }

    #[test]
    fn duplicate_on_error_fails_parse() {
        let source = "module m { enum State { A, B } initial A event E                       on_error { next A; } on_error { next B; } }";
        assert!(parse_source(source).is_err());
    }

    #[test]
    fn version_header_requires_quoted_string() {
        let source = "module m version 1.0 { enum State { A } initial A }";
        assert!(
            parse_source(source).is_err(),
            "unquoted version must not parse"
        );
    }
}
