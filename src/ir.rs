//! IR types and their hand-written JSON (de)serialization.
//!
//! Phase 5 task 2 removed the external JSON stack from the toolchain: every
//! type below is serialized/deserialized by explicit `to_json`/`from_json`
//! code built on
//! [`crate::json_rw`]. The wire format is a PINNED CONTRACT, byte-identical
//! to what the pre-migration (external-crate) format produced:
//!
//! - Structs: keys in DECLARATION order. `Option` fields serialize as
//!   `null` when absent, EXCEPT `Module.version` and `Module.on_error`, which
//!   are omitted entirely when `None` (both optional on read).
//! - Internally-tagged enums (`Type`, `ConstructorVariant`, `PatternVariant`,
//!   `PatternExpr`, `Formula`, `Expr`): the `"kind"` key comes FIRST with the
//!   snake_case variant name, followed by the variant's fields in declaration
//!   order.
//! - Unit variants of `UnaryOp`/`BinOp`: bare JSON strings (`"not"`, ...).
//! - Integers: unsigned IR fields (`u32`/`u64`/`usize`) render as non-negative
//!   JSON numbers; `to_json` fails (rather than clamping) when a `u64` value
//!   exceeds `i64::MAX`, since the value model is 64-bit signed. `from_json`
//!   rejects negative integers for those fields.
//!
//! Golden tests in `tests/frontend_test.rs` pin the rendered text; the
//! round-trip property (parse rfs -> to_json -> from_json == original IR) is
//! tested over all valid examples there as well.

use crate::json_rw::Json;

pub const IR_VERSION: u32 = 0;

#[derive(Debug, Clone, PartialEq)]
pub struct Ir {
    pub ir_version: u32,
    pub modules: Vec<Module>,
}

impl Ir {
    pub fn new() -> Self {
        Self {
            ir_version: IR_VERSION,
            modules: Vec::new(),
        }
    }
}

impl Default for Ir {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Module {
    pub name: String,
    /// Optional spec version header (`module Name version "1.0" {`).
    pub version: Option<String>,
    /// Optional `on_error` fallback executed when no transition matches.
    pub on_error: Option<OnError>,
    pub enums: Vec<EnumDecl>,
    pub initial_state: Option<String>,
    pub events: Vec<ConstructorGroup>,
    pub outputs: Vec<ConstructorGroup>,
    pub vars: Vec<VarDecl>,
    pub transitions: Vec<Transition>,
    pub properties: Vec<Property>,
    pub assumptions: Vec<Property>,
}

/// Fallback behavior for unexpected inputs (no matching transition).
/// Executed in any state; has no pattern bindings.
#[derive(Debug, Clone, PartialEq)]
pub struct OnError {
    pub next_state: String,
    pub emits: Vec<OutputExpr>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnumDecl {
    pub name: String,
    pub variants: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Bool,
    Bit { width: u32 },
    Enum { name: String },
}

#[derive(Debug, Clone, PartialEq)]
pub struct FieldDecl {
    pub name: String,
    pub ty: Type,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConstructorGroup {
    pub name: String,
    pub variants: Vec<ConstructorVariant>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConstructorVariant {
    Unit {
        name: String,
    },
    Struct {
        name: String,
        fields: Vec<FieldDecl>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct VarDecl {
    pub name: String,
    pub ty: Type,
    pub init: Expr,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Transition {
    pub id: usize,
    pub state: String,
    pub pattern: EventPattern,
    pub guard: Option<Expr>,
    pub next_state: String,
    pub emits: Vec<OutputExpr>,
    pub assignments: Vec<Assignment>,
    pub priority: Option<u32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EventPattern {
    pub group: String,
    pub variant: Option<PatternVariant>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OutputPattern {
    pub group: String,
    pub variant: Option<PatternVariant>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PatternVariant {
    Unit {
        name: String,
    },
    Struct {
        name: String,
        fields: Vec<PatternField>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct PatternField {
    pub name: Option<String>,
    pub pattern: PatternExpr,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PatternExpr {
    Wildcard,
    Bind { name: String },
    Int { value: u64 },
    Bool { value: bool },
    EnumVariant { path: Vec<String> },
}

#[derive(Debug, Clone, PartialEq)]
pub struct OutputExpr {
    pub group: String,
    pub variant: Option<String>,
    pub args: Vec<Expr>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Assignment {
    pub var: String,
    pub value: Expr,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Property {
    pub name: String,
    pub formula: Formula,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Formula {
    InputIs {
        pattern: EventPattern,
    },
    OutputHas {
        pattern: OutputPattern,
    },
    Expr {
        expr: Expr,
    },
    Not {
        inner: Box<Formula>,
    },
    And {
        left: Box<Formula>,
        right: Box<Formula>,
    },
    Or {
        left: Box<Formula>,
        right: Box<Formula>,
    },
    Implies {
        left: Box<Formula>,
        right: Box<Formula>,
    },
    Next {
        inner: Box<Formula>,
    },
    Always {
        inner: Box<Formula>,
    },
    /// `always[N] (inner)`: the body must hold at every sample 0..N-1.
    /// Additive Phase 3 variant (no IR_VERSION bump; D-P3-1).
    BoundedAlways {
        bound: u64,
        inner: Box<Formula>,
    },
    BoundedEventually {
        bound: u64,
        inner: Box<Formula>,
    },
    /// `eventually (inner)`: liveness obligation - the body must hold at some
    /// future sample (D-P4-8; engine semantics land in task 6). Additive
    /// Phase 4 variant (no IR_VERSION bump).
    Eventually {
        inner: Box<Formula>,
    },
    /// `left until right`: WEAK until (D-P4-8) - left must hold at every
    /// sample before the first sample where right holds; if right never
    /// holds, the obligation is vacuously satisfied. Strong until is
    /// expressible as `p until q && eventually q`. Additive Phase 4 variant
    /// (no IR_VERSION bump).
    Until {
        left: Box<Formula>,
        right: Box<Formula>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Int {
        value: u64,
    },
    Bool {
        value: bool,
    },
    Path {
        path: Vec<String>,
    },
    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
    },
    Binary {
        op: BinOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Not,
    BitNot,
    Neg,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
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

// ---------------------------------------------------------------------------
// JSON (de)serialization - hand-written, no external crates (Phase 5 task 2)
// ---------------------------------------------------------------------------

/// Serialization/deserialization of one IR type to/from [`Json`].
trait IrJson {
    fn to_json(&self) -> Result<Json, String>;
    fn from_json(v: &Json, path: &str) -> Result<Self, String>
    where
        Self: Sized;
}

/// A short human-readable name of a JSON value's type (for error messages).
fn json_type_name(v: &Json) -> &'static str {
    match v {
        Json::Null => "null",
        Json::Bool(_) => "bool",
        Json::Int(_) => "integer",
        Json::Str(_) => "string",
        Json::Arr(_) => "array",
        Json::Obj(_) => "object",
    }
}

fn type_err(v: &Json, path: &str, expected: &str) -> String {
    format!("expected {expected} at {path}, got {}", json_type_name(v))
}

/// Look up a required object field with a good error.
fn field<'a>(v: &'a Json, path: &str, key: &str) -> Result<&'a Json, String> {
    match v {
        Json::Obj(pairs) => pairs
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, val)| val)
            .ok_or_else(|| format!("missing field '{key}' at {path}")),
        other => Err(type_err(other, path, "object")),
    }
}

fn str_field(v: &Json, path: &str, key: &str) -> Result<String, String> {
    let f = field(v, path, key)?;
    match f {
        Json::Str(s) => Ok(s.clone()),
        other => Err(type_err(other, &format!("{path}.{key}"), "string")),
    }
}

fn opt_str_field(v: &Json, path: &str, key: &str) -> Result<Option<String>, String> {
    let f = field(v, path, key)?;
    match f {
        Json::Null => Ok(None),
        Json::Str(s) => Ok(Some(s.clone())),
        other => Err(type_err(other, &format!("{path}.{key}"), "string or null")),
    }
}

fn arr_field<'a>(v: &'a Json, path: &str, key: &str) -> Result<&'a [Json], String> {
    let f = field(v, path, key)?;
    match f {
        Json::Arr(items) => Ok(items),
        other => Err(type_err(other, &format!("{path}.{key}"), "array")),
    }
}

/// A non-negative integer field (u32/u64/usize IR values).
fn uint_field(v: &Json, path: &str, key: &str) -> Result<u64, String> {
    let f = field(v, path, key)?;
    match f {
        Json::Int(n) if *n >= 0 => Ok(*n as u64),
        Json::Int(_) => Err(format!(
            "expected non-negative integer at {path}.{key}, got negative"
        )),
        other => Err(type_err(other, &format!("{path}.{key}"), "integer")),
    }
}

fn bool_field(v: &Json, path: &str, key: &str) -> Result<bool, String> {
    let f = field(v, path, key)?;
    match f {
        Json::Bool(b) => Ok(*b),
        other => Err(type_err(other, &format!("{path}.{key}"), "bool")),
    }
}

fn opt_uint32_field(v: &Json, path: &str, key: &str) -> Result<Option<u32>, String> {
    let f = field(v, path, key)?;
    match f {
        Json::Null => Ok(None),
        Json::Int(n) if *n >= 0 && *n <= u32::MAX as i64 => Ok(Some(*n as u32)),
        _ => Err(format!(
            "expected integer in 0..=4294967295 or null at {path}.{key}"
        )),
    }
}

fn str_list(items: &[Json], path: &str) -> Result<Vec<String>, String> {
    items
        .iter()
        .enumerate()
        .map(|(i, x)| match x {
            Json::Str(s) => Ok(s.clone()),
            other => Err(type_err(other, &format!("{path}[{i}]"), "string")),
        })
        .collect()
}

/// The `"kind"` discriminator of an internally-tagged enum object.
fn kind<'a>(v: &'a Json, path: &str) -> Result<&'a str, String> {
    let k = field(v, path, "kind")?;
    match k {
        Json::Str(s) => Ok(s.as_str()),
        other => Err(type_err(other, &format!("{path}.kind"), "string")),
    }
}

fn unknown_kind(what: &str, got: &str, path: &str, allowed: &[&str]) -> String {
    format!(
        "unknown {what} kind '{got}' at {path} (expected one of: {})",
        allowed.join(", ")
    )
}

/// Render a u64 IR value as a JSON integer; the value model is i64, so values
/// above i64::MAX are rejected instead of clamped (see module docs).
fn uint_json(value: u64, path: &str) -> Result<Json, String> {
    if value > i64::MAX as u64 {
        return Err(format!(
            "value {value} at {path} exceeds the i64 JSON range; RFS IR integers \
             are limited to 9223372036854775807 in JSON form"
        ));
    }
    Ok(Json::Int(value as i64))
}

impl IrJson for Ir {
    fn to_json(&self) -> Result<Json, String> {
        Ok(Json::obj(vec![
            ("ir_version".into(), Json::int(self.ir_version as i64)),
            (
                "modules".into(),
                Json::arr(
                    self.modules
                        .iter()
                        .map(|m| m.to_json())
                        .collect::<Result<_, _>>()?,
                ),
            ),
        ]))
    }

    fn from_json(v: &Json, path: &str) -> Result<Self, String> {
        Ok(Ir {
            ir_version: uint_field(v, path, "ir_version")? as u32,
            modules: arr_field(v, path, "modules")?
                .iter()
                .enumerate()
                .map(|(i, m)| Module::from_json(m, &format!("{path}.modules[{i}]")))
                .collect::<Result<_, _>>()?,
        })
    }
}

impl IrJson for Module {
    fn to_json(&self) -> Result<Json, String> {
        let mut pairs = vec![("name".into(), Json::Str(self.name.clone()))];
        // Omitted (not null) when None - the two optional fields that the
        // pre-migration external-crate format skipped on serialization.
        if let Some(version) = &self.version {
            pairs.push(("version".into(), Json::Str(version.clone())));
        }
        if let Some(on_error) = &self.on_error {
            pairs.push(("on_error".into(), on_error.to_json()?));
        }
        pairs.extend([
            (
                "enums".into(),
                Json::arr(
                    self.enums
                        .iter()
                        .map(|e| e.to_json())
                        .collect::<Result<_, _>>()?,
                ),
            ),
            (
                "initial_state".into(),
                match &self.initial_state {
                    Some(s) => Json::Str(s.clone()),
                    None => Json::Null,
                },
            ),
            (
                "events".into(),
                Json::arr(
                    self.events
                        .iter()
                        .map(|g| g.to_json())
                        .collect::<Result<_, _>>()?,
                ),
            ),
            (
                "outputs".into(),
                Json::arr(
                    self.outputs
                        .iter()
                        .map(|g| g.to_json())
                        .collect::<Result<_, _>>()?,
                ),
            ),
            (
                "vars".into(),
                Json::arr(
                    self.vars
                        .iter()
                        .map(|d| d.to_json())
                        .collect::<Result<_, _>>()?,
                ),
            ),
            (
                "transitions".into(),
                Json::arr(
                    self.transitions
                        .iter()
                        .map(|t| t.to_json())
                        .collect::<Result<_, _>>()?,
                ),
            ),
            (
                "properties".into(),
                Json::arr(
                    self.properties
                        .iter()
                        .map(|p| p.to_json())
                        .collect::<Result<_, _>>()?,
                ),
            ),
            (
                "assumptions".into(),
                Json::arr(
                    self.assumptions
                        .iter()
                        .map(|p| p.to_json())
                        .collect::<Result<_, _>>()?,
                ),
            ),
        ]);
        Ok(Json::Obj(pairs))
    }

    fn from_json(v: &Json, path: &str) -> Result<Self, String> {
        // version / on_error are optional keys (absent when None).
        let version = match v.get("version") {
            None => None,
            Some(Json::Str(s)) => Some(s.clone()),
            Some(other) => return Err(type_err(other, &format!("{path}.version"), "string")),
        };
        let on_error = match v.get("on_error") {
            None => None,
            Some(x) => Some(OnError::from_json(x, &format!("{path}.on_error"))?),
        };
        Ok(Module {
            name: str_field(v, path, "name")?,
            version,
            on_error,
            enums: arr_field(v, path, "enums")?
                .iter()
                .enumerate()
                .map(|(i, e)| EnumDecl::from_json(e, &format!("{path}.enums[{i}]")))
                .collect::<Result<_, _>>()?,
            initial_state: opt_str_field(v, path, "initial_state")?,
            events: arr_field(v, path, "events")?
                .iter()
                .enumerate()
                .map(|(i, g)| ConstructorGroup::from_json(g, &format!("{path}.events[{i}]")))
                .collect::<Result<_, _>>()?,
            outputs: arr_field(v, path, "outputs")?
                .iter()
                .enumerate()
                .map(|(i, g)| ConstructorGroup::from_json(g, &format!("{path}.outputs[{i}]")))
                .collect::<Result<_, _>>()?,
            vars: arr_field(v, path, "vars")?
                .iter()
                .enumerate()
                .map(|(i, d)| VarDecl::from_json(d, &format!("{path}.vars[{i}]")))
                .collect::<Result<_, _>>()?,
            transitions: arr_field(v, path, "transitions")?
                .iter()
                .enumerate()
                .map(|(i, t)| Transition::from_json(t, &format!("{path}.transitions[{i}]")))
                .collect::<Result<_, _>>()?,
            properties: arr_field(v, path, "properties")?
                .iter()
                .enumerate()
                .map(|(i, p)| Property::from_json(p, &format!("{path}.properties[{i}]")))
                .collect::<Result<_, _>>()?,
            assumptions: arr_field(v, path, "assumptions")?
                .iter()
                .enumerate()
                .map(|(i, p)| Property::from_json(p, &format!("{path}.assumptions[{i}]")))
                .collect::<Result<_, _>>()?,
        })
    }
}

impl IrJson for OnError {
    fn to_json(&self) -> Result<Json, String> {
        Ok(Json::obj(vec![
            ("next_state".into(), Json::Str(self.next_state.clone())),
            (
                "emits".into(),
                Json::arr(
                    self.emits
                        .iter()
                        .map(|e| e.to_json())
                        .collect::<Result<_, _>>()?,
                ),
            ),
        ]))
    }

    fn from_json(v: &Json, path: &str) -> Result<Self, String> {
        Ok(OnError {
            next_state: str_field(v, path, "next_state")?,
            emits: arr_field(v, path, "emits")?
                .iter()
                .enumerate()
                .map(|(i, e)| OutputExpr::from_json(e, &format!("{path}.emits[{i}]")))
                .collect::<Result<_, _>>()?,
        })
    }
}

impl IrJson for EnumDecl {
    fn to_json(&self) -> Result<Json, String> {
        Ok(Json::obj(vec![
            ("name".into(), Json::Str(self.name.clone())),
            (
                "variants".into(),
                Json::arr(self.variants.iter().map(|s| Json::Str(s.clone())).collect()),
            ),
        ]))
    }

    fn from_json(v: &Json, path: &str) -> Result<Self, String> {
        Ok(EnumDecl {
            name: str_field(v, path, "name")?,
            variants: str_list(arr_field(v, path, "variants")?, &format!("{path}.variants"))?,
        })
    }
}

impl IrJson for Type {
    fn to_json(&self) -> Result<Json, String> {
        Ok(match self {
            Type::Bool => Json::obj(vec![("kind".into(), Json::Str("bool".into()))]),
            Type::Bit { width } => Json::obj(vec![
                ("kind".into(), Json::Str("bit".into())),
                ("width".into(), Json::int(*width as i64)),
            ]),
            Type::Enum { name } => Json::obj(vec![
                ("kind".into(), Json::Str("enum".into())),
                ("name".into(), Json::Str(name.clone())),
            ]),
        })
    }

    fn from_json(v: &Json, path: &str) -> Result<Self, String> {
        match kind(v, path)? {
            "bool" => Ok(Type::Bool),
            "bit" => Ok(Type::Bit {
                width: uint_field(v, path, "width")? as u32,
            }),
            "enum" => Ok(Type::Enum {
                name: str_field(v, path, "name")?,
            }),
            got => Err(unknown_kind("Type", got, path, &["bool", "bit", "enum"])),
        }
    }
}

impl IrJson for FieldDecl {
    fn to_json(&self) -> Result<Json, String> {
        Ok(Json::obj(vec![
            ("name".into(), Json::Str(self.name.clone())),
            ("ty".into(), self.ty.to_json()?),
        ]))
    }

    fn from_json(v: &Json, path: &str) -> Result<Self, String> {
        Ok(FieldDecl {
            name: str_field(v, path, "name")?,
            ty: Type::from_json(field(v, path, "ty")?, &format!("{path}.ty"))?,
        })
    }
}

impl IrJson for ConstructorGroup {
    fn to_json(&self) -> Result<Json, String> {
        Ok(Json::obj(vec![
            ("name".into(), Json::Str(self.name.clone())),
            (
                "variants".into(),
                Json::arr(
                    self.variants
                        .iter()
                        .map(|c| c.to_json())
                        .collect::<Result<_, _>>()?,
                ),
            ),
        ]))
    }

    fn from_json(v: &Json, path: &str) -> Result<Self, String> {
        Ok(ConstructorGroup {
            name: str_field(v, path, "name")?,
            variants: arr_field(v, path, "variants")?
                .iter()
                .enumerate()
                .map(|(i, c)| ConstructorVariant::from_json(c, &format!("{path}.variants[{i}]")))
                .collect::<Result<_, _>>()?,
        })
    }
}

impl IrJson for ConstructorVariant {
    fn to_json(&self) -> Result<Json, String> {
        Ok(match self {
            ConstructorVariant::Unit { name } => Json::obj(vec![
                ("kind".into(), Json::Str("unit".into())),
                ("name".into(), Json::Str(name.clone())),
            ]),
            ConstructorVariant::Struct { name, fields } => Json::obj(vec![
                ("kind".into(), Json::Str("struct".into())),
                ("name".into(), Json::Str(name.clone())),
                (
                    "fields".into(),
                    Json::arr(
                        fields
                            .iter()
                            .map(|f| f.to_json())
                            .collect::<Result<_, _>>()?,
                    ),
                ),
            ]),
        })
    }

    fn from_json(v: &Json, path: &str) -> Result<Self, String> {
        match kind(v, path)? {
            "unit" => Ok(ConstructorVariant::Unit {
                name: str_field(v, path, "name")?,
            }),
            "struct" => Ok(ConstructorVariant::Struct {
                name: str_field(v, path, "name")?,
                fields: arr_field(v, path, "fields")?
                    .iter()
                    .enumerate()
                    .map(|(i, f)| FieldDecl::from_json(f, &format!("{path}.fields[{i}]")))
                    .collect::<Result<_, _>>()?,
            }),
            got => Err(unknown_kind(
                "ConstructorVariant",
                got,
                path,
                &["unit", "struct"],
            )),
        }
    }
}

impl IrJson for VarDecl {
    fn to_json(&self) -> Result<Json, String> {
        Ok(Json::obj(vec![
            ("name".into(), Json::Str(self.name.clone())),
            ("ty".into(), self.ty.to_json()?),
            ("init".into(), self.init.to_json()?),
        ]))
    }

    fn from_json(v: &Json, path: &str) -> Result<Self, String> {
        Ok(VarDecl {
            name: str_field(v, path, "name")?,
            ty: Type::from_json(field(v, path, "ty")?, &format!("{path}.ty"))?,
            init: Expr::from_json(field(v, path, "init")?, &format!("{path}.init"))?,
        })
    }
}

impl IrJson for Transition {
    fn to_json(&self) -> Result<Json, String> {
        Ok(Json::obj(vec![
            ("id".into(), uint_json(self.id as u64, "id")?),
            ("state".into(), Json::Str(self.state.clone())),
            ("pattern".into(), self.pattern.to_json()?),
            (
                "guard".into(),
                match &self.guard {
                    Some(g) => g.to_json()?,
                    None => Json::Null,
                },
            ),
            ("next_state".into(), Json::Str(self.next_state.clone())),
            (
                "emits".into(),
                Json::arr(
                    self.emits
                        .iter()
                        .map(|e| e.to_json())
                        .collect::<Result<_, _>>()?,
                ),
            ),
            (
                "assignments".into(),
                Json::arr(
                    self.assignments
                        .iter()
                        .map(|a| a.to_json())
                        .collect::<Result<_, _>>()?,
                ),
            ),
            (
                "priority".into(),
                match self.priority {
                    Some(p) => Json::int(p as i64),
                    None => Json::Null,
                },
            ),
        ]))
    }

    fn from_json(v: &Json, path: &str) -> Result<Self, String> {
        let guard = match field(v, path, "guard")? {
            Json::Null => None,
            g => Some(Expr::from_json(g, &format!("{path}.guard"))?),
        };
        Ok(Transition {
            id: uint_field(v, path, "id")? as usize,
            state: str_field(v, path, "state")?,
            pattern: EventPattern::from_json(
                field(v, path, "pattern")?,
                &format!("{path}.pattern"),
            )?,
            guard,
            next_state: str_field(v, path, "next_state")?,
            emits: arr_field(v, path, "emits")?
                .iter()
                .enumerate()
                .map(|(i, e)| OutputExpr::from_json(e, &format!("{path}.emits[{i}]")))
                .collect::<Result<_, _>>()?,
            assignments: arr_field(v, path, "assignments")?
                .iter()
                .enumerate()
                .map(|(i, a)| Assignment::from_json(a, &format!("{path}.assignments[{i}]")))
                .collect::<Result<_, _>>()?,
            priority: opt_uint32_field(v, path, "priority")?,
        })
    }
}

impl IrJson for EventPattern {
    fn to_json(&self) -> Result<Json, String> {
        Ok(Json::obj(vec![
            ("group".into(), Json::Str(self.group.clone())),
            (
                "variant".into(),
                match &self.variant {
                    Some(v) => v.to_json()?,
                    None => Json::Null,
                },
            ),
        ]))
    }

    fn from_json(v: &Json, path: &str) -> Result<Self, String> {
        let variant = match field(v, path, "variant")? {
            Json::Null => None,
            x => Some(PatternVariant::from_json(x, &format!("{path}.variant"))?),
        };
        Ok(EventPattern {
            group: str_field(v, path, "group")?,
            variant,
        })
    }
}

impl IrJson for OutputPattern {
    fn to_json(&self) -> Result<Json, String> {
        Ok(Json::obj(vec![
            ("group".into(), Json::Str(self.group.clone())),
            (
                "variant".into(),
                match &self.variant {
                    Some(v) => v.to_json()?,
                    None => Json::Null,
                },
            ),
        ]))
    }

    fn from_json(v: &Json, path: &str) -> Result<Self, String> {
        let variant = match field(v, path, "variant")? {
            Json::Null => None,
            x => Some(PatternVariant::from_json(x, &format!("{path}.variant"))?),
        };
        Ok(OutputPattern {
            group: str_field(v, path, "group")?,
            variant,
        })
    }
}

impl IrJson for PatternVariant {
    fn to_json(&self) -> Result<Json, String> {
        Ok(match self {
            PatternVariant::Unit { name } => Json::obj(vec![
                ("kind".into(), Json::Str("unit".into())),
                ("name".into(), Json::Str(name.clone())),
            ]),
            PatternVariant::Struct { name, fields } => Json::obj(vec![
                ("kind".into(), Json::Str("struct".into())),
                ("name".into(), Json::Str(name.clone())),
                (
                    "fields".into(),
                    Json::arr(
                        fields
                            .iter()
                            .map(|f| f.to_json())
                            .collect::<Result<_, _>>()?,
                    ),
                ),
            ]),
        })
    }

    fn from_json(v: &Json, path: &str) -> Result<Self, String> {
        match kind(v, path)? {
            "unit" => Ok(PatternVariant::Unit {
                name: str_field(v, path, "name")?,
            }),
            "struct" => Ok(PatternVariant::Struct {
                name: str_field(v, path, "name")?,
                fields: arr_field(v, path, "fields")?
                    .iter()
                    .enumerate()
                    .map(|(i, f)| PatternField::from_json(f, &format!("{path}.fields[{i}]")))
                    .collect::<Result<_, _>>()?,
            }),
            got => Err(unknown_kind(
                "PatternVariant",
                got,
                path,
                &["unit", "struct"],
            )),
        }
    }
}

impl IrJson for PatternField {
    fn to_json(&self) -> Result<Json, String> {
        Ok(Json::obj(vec![
            (
                "name".into(),
                match &self.name {
                    Some(n) => Json::Str(n.clone()),
                    None => Json::Null,
                },
            ),
            ("pattern".into(), self.pattern.to_json()?),
        ]))
    }

    fn from_json(v: &Json, path: &str) -> Result<Self, String> {
        Ok(PatternField {
            name: opt_str_field(v, path, "name")?,
            pattern: PatternExpr::from_json(
                field(v, path, "pattern")?,
                &format!("{path}.pattern"),
            )?,
        })
    }
}

impl IrJson for PatternExpr {
    fn to_json(&self) -> Result<Json, String> {
        Ok(match self {
            PatternExpr::Wildcard => Json::obj(vec![("kind".into(), Json::Str("wildcard".into()))]),
            PatternExpr::Bind { name } => Json::obj(vec![
                ("kind".into(), Json::Str("bind".into())),
                ("name".into(), Json::Str(name.clone())),
            ]),
            PatternExpr::Int { value } => Json::obj(vec![
                ("kind".into(), Json::Str("int".into())),
                ("value".into(), uint_json(*value, "value")?),
            ]),
            PatternExpr::Bool { value } => Json::obj(vec![
                ("kind".into(), Json::Str("bool".into())),
                ("value".into(), Json::bool(*value)),
            ]),
            PatternExpr::EnumVariant { path: p } => Json::obj(vec![
                ("kind".into(), Json::Str("enum_variant".into())),
                (
                    "path".into(),
                    Json::arr(p.iter().map(|s| Json::Str(s.clone())).collect()),
                ),
            ]),
        })
    }

    fn from_json(v: &Json, path: &str) -> Result<Self, String> {
        match kind(v, path)? {
            "wildcard" => Ok(PatternExpr::Wildcard),
            "bind" => Ok(PatternExpr::Bind {
                name: str_field(v, path, "name")?,
            }),
            "int" => Ok(PatternExpr::Int {
                value: uint_field(v, path, "value")?,
            }),
            "bool" => Ok(PatternExpr::Bool {
                value: bool_field(v, path, "value")?,
            }),
            "enum_variant" => Ok(PatternExpr::EnumVariant {
                path: str_list(arr_field(v, path, "path")?, &format!("{path}.path"))?,
            }),
            got => Err(unknown_kind(
                "PatternExpr",
                got,
                path,
                &["wildcard", "bind", "int", "bool", "enum_variant"],
            )),
        }
    }
}

impl IrJson for OutputExpr {
    fn to_json(&self) -> Result<Json, String> {
        Ok(Json::obj(vec![
            ("group".into(), Json::Str(self.group.clone())),
            (
                "variant".into(),
                match &self.variant {
                    Some(v) => Json::Str(v.clone()),
                    None => Json::Null,
                },
            ),
            (
                "args".into(),
                Json::arr(
                    self.args
                        .iter()
                        .map(|e| e.to_json())
                        .collect::<Result<_, _>>()?,
                ),
            ),
        ]))
    }

    fn from_json(v: &Json, path: &str) -> Result<Self, String> {
        Ok(OutputExpr {
            group: str_field(v, path, "group")?,
            variant: opt_str_field(v, path, "variant")?,
            args: arr_field(v, path, "args")?
                .iter()
                .enumerate()
                .map(|(i, e)| Expr::from_json(e, &format!("{path}.args[{i}]")))
                .collect::<Result<_, _>>()?,
        })
    }
}

impl IrJson for Assignment {
    fn to_json(&self) -> Result<Json, String> {
        Ok(Json::obj(vec![
            ("var".into(), Json::Str(self.var.clone())),
            ("value".into(), self.value.to_json()?),
        ]))
    }

    fn from_json(v: &Json, path: &str) -> Result<Self, String> {
        Ok(Assignment {
            var: str_field(v, path, "var")?,
            value: Expr::from_json(field(v, path, "value")?, &format!("{path}.value"))?,
        })
    }
}

impl IrJson for Property {
    fn to_json(&self) -> Result<Json, String> {
        Ok(Json::obj(vec![
            ("name".into(), Json::Str(self.name.clone())),
            ("formula".into(), self.formula.to_json()?),
        ]))
    }

    fn from_json(v: &Json, path: &str) -> Result<Self, String> {
        Ok(Property {
            name: str_field(v, path, "name")?,
            formula: Formula::from_json(field(v, path, "formula")?, &format!("{path}.formula"))?,
        })
    }
}

impl IrJson for Formula {
    fn to_json(&self) -> Result<Json, String> {
        Ok(match self {
            Formula::InputIs { pattern } => Json::obj(vec![
                ("kind".into(), Json::Str("input_is".into())),
                ("pattern".into(), pattern.to_json()?),
            ]),
            Formula::OutputHas { pattern } => Json::obj(vec![
                ("kind".into(), Json::Str("output_has".into())),
                ("pattern".into(), pattern.to_json()?),
            ]),
            Formula::Expr { expr } => Json::obj(vec![
                ("kind".into(), Json::Str("expr".into())),
                ("expr".into(), expr.to_json()?),
            ]),
            Formula::Not { inner } => Json::obj(vec![
                ("kind".into(), Json::Str("not".into())),
                ("inner".into(), inner.to_json()?),
            ]),
            Formula::And { left, right } => Json::obj(vec![
                ("kind".into(), Json::Str("and".into())),
                ("left".into(), left.to_json()?),
                ("right".into(), right.to_json()?),
            ]),
            Formula::Or { left, right } => Json::obj(vec![
                ("kind".into(), Json::Str("or".into())),
                ("left".into(), left.to_json()?),
                ("right".into(), right.to_json()?),
            ]),
            Formula::Implies { left, right } => Json::obj(vec![
                ("kind".into(), Json::Str("implies".into())),
                ("left".into(), left.to_json()?),
                ("right".into(), right.to_json()?),
            ]),
            Formula::Next { inner } => Json::obj(vec![
                ("kind".into(), Json::Str("next".into())),
                ("inner".into(), inner.to_json()?),
            ]),
            Formula::Always { inner } => Json::obj(vec![
                ("kind".into(), Json::Str("always".into())),
                ("inner".into(), inner.to_json()?),
            ]),
            Formula::BoundedAlways { bound, inner } => Json::obj(vec![
                ("kind".into(), Json::Str("bounded_always".into())),
                ("bound".into(), uint_json(*bound, "bound")?),
                ("inner".into(), inner.to_json()?),
            ]),
            Formula::BoundedEventually { bound, inner } => Json::obj(vec![
                ("kind".into(), Json::Str("bounded_eventually".into())),
                ("bound".into(), uint_json(*bound, "bound")?),
                ("inner".into(), inner.to_json()?),
            ]),
            Formula::Eventually { inner } => Json::obj(vec![
                ("kind".into(), Json::Str("eventually".into())),
                ("inner".into(), inner.to_json()?),
            ]),
            Formula::Until { left, right } => Json::obj(vec![
                ("kind".into(), Json::Str("until".into())),
                ("left".into(), left.to_json()?),
                ("right".into(), right.to_json()?),
            ]),
        })
    }

    fn from_json(v: &Json, path: &str) -> Result<Self, String> {
        let inner = |p: &str| -> Result<Formula, String> {
            Formula::from_json(field(v, p, "inner")?, &format!("{p}.inner"))
        };
        let pair = |p: &str| -> Result<(Formula, Formula), String> {
            Ok((
                Formula::from_json(field(v, p, "left")?, &format!("{p}.left"))?,
                Formula::from_json(field(v, p, "right")?, &format!("{p}.right"))?,
            ))
        };
        match kind(v, path)? {
            "input_is" => Ok(Formula::InputIs {
                pattern: EventPattern::from_json(
                    field(v, path, "pattern")?,
                    &format!("{path}.pattern"),
                )?,
            }),
            "output_has" => Ok(Formula::OutputHas {
                pattern: OutputPattern::from_json(
                    field(v, path, "pattern")?,
                    &format!("{path}.pattern"),
                )?,
            }),
            "expr" => Ok(Formula::Expr {
                expr: Expr::from_json(field(v, path, "expr")?, &format!("{path}.expr"))?,
            }),
            "not" => Ok(Formula::Not {
                inner: Box::new(inner(path)?),
            }),
            "and" => {
                let (left, right) = pair(path)?;
                Ok(Formula::And {
                    left: Box::new(left),
                    right: Box::new(right),
                })
            }
            "or" => {
                let (left, right) = pair(path)?;
                Ok(Formula::Or {
                    left: Box::new(left),
                    right: Box::new(right),
                })
            }
            "implies" => {
                let (left, right) = pair(path)?;
                Ok(Formula::Implies {
                    left: Box::new(left),
                    right: Box::new(right),
                })
            }
            "next" => Ok(Formula::Next {
                inner: Box::new(inner(path)?),
            }),
            "always" => Ok(Formula::Always {
                inner: Box::new(inner(path)?),
            }),
            "bounded_always" => Ok(Formula::BoundedAlways {
                bound: uint_field(v, path, "bound")?,
                inner: Box::new(inner(path)?),
            }),
            "bounded_eventually" => Ok(Formula::BoundedEventually {
                bound: uint_field(v, path, "bound")?,
                inner: Box::new(inner(path)?),
            }),
            "eventually" => Ok(Formula::Eventually {
                inner: Box::new(inner(path)?),
            }),
            "until" => {
                let (left, right) = pair(path)?;
                Ok(Formula::Until {
                    left: Box::new(left),
                    right: Box::new(right),
                })
            }
            got => Err(unknown_kind(
                "Formula",
                got,
                path,
                &[
                    "input_is",
                    "output_has",
                    "expr",
                    "not",
                    "and",
                    "or",
                    "implies",
                    "next",
                    "always",
                    "bounded_always",
                    "bounded_eventually",
                    "eventually",
                    "until",
                ],
            )),
        }
    }
}

impl IrJson for Expr {
    fn to_json(&self) -> Result<Json, String> {
        Ok(match self {
            Expr::Int { value } => Json::obj(vec![
                ("kind".into(), Json::Str("int".into())),
                ("value".into(), uint_json(*value, "value")?),
            ]),
            Expr::Bool { value } => Json::obj(vec![
                ("kind".into(), Json::Str("bool".into())),
                ("value".into(), Json::bool(*value)),
            ]),
            Expr::Path { path: p } => Json::obj(vec![
                ("kind".into(), Json::Str("path".into())),
                (
                    "path".into(),
                    Json::arr(p.iter().map(|s| Json::Str(s.clone())).collect()),
                ),
            ]),
            Expr::Unary { op, expr } => Json::obj(vec![
                ("kind".into(), Json::Str("unary".into())),
                ("op".into(), op.to_json()?),
                ("expr".into(), expr.to_json()?),
            ]),
            Expr::Binary { op, left, right } => Json::obj(vec![
                ("kind".into(), Json::Str("binary".into())),
                ("op".into(), op.to_json()?),
                ("left".into(), left.to_json()?),
                ("right".into(), right.to_json()?),
            ]),
        })
    }

    fn from_json(v: &Json, path: &str) -> Result<Self, String> {
        match kind(v, path)? {
            "int" => Ok(Expr::Int {
                value: uint_field(v, path, "value")?,
            }),
            "bool" => Ok(Expr::Bool {
                value: bool_field(v, path, "value")?,
            }),
            "path" => Ok(Expr::Path {
                path: str_list(arr_field(v, path, "path")?, &format!("{path}.path"))?,
            }),
            "unary" => Ok(Expr::Unary {
                op: UnaryOp::from_json(field(v, path, "op")?, &format!("{path}.op"))?,
                expr: Box::new(Expr::from_json(
                    field(v, path, "expr")?,
                    &format!("{path}.expr"),
                )?),
            }),
            "binary" => Ok(Expr::Binary {
                op: BinOp::from_json(field(v, path, "op")?, &format!("{path}.op"))?,
                left: Box::new(Expr::from_json(
                    field(v, path, "left")?,
                    &format!("{path}.left"),
                )?),
                right: Box::new(Expr::from_json(
                    field(v, path, "right")?,
                    &format!("{path}.right"),
                )?),
            }),
            got => Err(unknown_kind(
                "Expr",
                got,
                path,
                &["int", "bool", "path", "unary", "binary"],
            )),
        }
    }
}

impl IrJson for UnaryOp {
    fn to_json(&self) -> Result<Json, String> {
        Ok(Json::Str(match self {
            UnaryOp::Not => "not".into(),
            UnaryOp::BitNot => "bit_not".into(),
            UnaryOp::Neg => "neg".into(),
        }))
    }

    fn from_json(v: &Json, path: &str) -> Result<Self, String> {
        match v.as_str() {
            Some("not") => Ok(UnaryOp::Not),
            Some("bit_not") => Ok(UnaryOp::BitNot),
            Some("neg") => Ok(UnaryOp::Neg),
            Some(got) => Err(format!(
                "unknown UnaryOp '{got}' at {path} (expected one of: not, bit_not, neg)"
            )),
            None => Err(type_err(v, path, "string")),
        }
    }
}

impl IrJson for BinOp {
    fn to_json(&self) -> Result<Json, String> {
        Ok(Json::Str(match self {
            BinOp::Plus => "plus".into(),
            BinOp::Minus => "minus".into(),
            BinOp::Star => "star".into(),
            BinOp::ShiftLeft => "shift_left".into(),
            BinOp::ShiftRight => "shift_right".into(),
            BinOp::Eq => "eq".into(),
            BinOp::NotEq => "not_eq".into(),
            BinOp::Lt => "lt".into(),
            BinOp::Le => "le".into(),
            BinOp::Gt => "gt".into(),
            BinOp::Ge => "ge".into(),
            BinOp::BitOr => "bit_or".into(),
            BinOp::BitXor => "bit_xor".into(),
            BinOp::BitAnd => "bit_and".into(),
            BinOp::LogicOr => "logic_or".into(),
            BinOp::LogicAnd => "logic_and".into(),
        }))
    }

    fn from_json(v: &Json, path: &str) -> Result<Self, String> {
        match v.as_str() {
            Some("plus") => Ok(BinOp::Plus),
            Some("minus") => Ok(BinOp::Minus),
            Some("star") => Ok(BinOp::Star),
            Some("shift_left") => Ok(BinOp::ShiftLeft),
            Some("shift_right") => Ok(BinOp::ShiftRight),
            Some("eq") => Ok(BinOp::Eq),
            Some("not_eq") => Ok(BinOp::NotEq),
            Some("lt") => Ok(BinOp::Lt),
            Some("le") => Ok(BinOp::Le),
            Some("gt") => Ok(BinOp::Gt),
            Some("ge") => Ok(BinOp::Ge),
            Some("bit_or") => Ok(BinOp::BitOr),
            Some("bit_xor") => Ok(BinOp::BitXor),
            Some("bit_and") => Ok(BinOp::BitAnd),
            Some("logic_or") => Ok(BinOp::LogicOr),
            Some("logic_and") => Ok(BinOp::LogicAnd),
            Some(got) => Err(format!(
                "unknown BinOp '{got}' at {path} (expected one of: plus, minus, star, \
                 shift_left, shift_right, eq, not_eq, lt, le, gt, ge, bit_or, bit_xor, \
                 bit_and, logic_or, logic_and)"
            )),
            None => Err(type_err(v, path, "string")),
        }
    }
}

// ---------------------------------------------------------------------------
// Public convenience API (used by frontend / printer / tests)
// ---------------------------------------------------------------------------

impl Ir {
    /// Serialize the whole IR document to a [`Json`] value.
    pub fn to_json(&self) -> Result<Json, String> {
        IrJson::to_json(self)
    }

    /// Deserialize an IR document from a parsed [`Json`] value.
    pub fn from_json(v: &Json) -> Result<Ir, String> {
        IrJson::from_json(v, "$")
    }
}
