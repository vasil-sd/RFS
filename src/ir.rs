use serde::{Deserialize, Serialize};

pub const IR_VERSION: u32 = 0;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Module {
    pub name: String,
    /// Optional spec version header (`module Name version "1.0" {`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// Optional `on_error` fallback executed when no transition matches.
    #[serde(default, skip_serializing_if = "Option::is_none")]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OnError {
    pub next_state: String,
    pub emits: Vec<OutputExpr>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnumDecl {
    pub name: String,
    pub variants: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Type {
    Bool,
    Bit { width: u32 },
    Enum { name: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FieldDecl {
    pub name: String,
    pub ty: Type,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConstructorGroup {
    pub name: String,
    pub variants: Vec<ConstructorVariant>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ConstructorVariant {
    Unit {
        name: String,
    },
    Struct {
        name: String,
        fields: Vec<FieldDecl>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VarDecl {
    pub name: String,
    pub ty: Type,
    pub init: Expr,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventPattern {
    pub group: String,
    pub variant: Option<PatternVariant>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OutputPattern {
    pub group: String,
    pub variant: Option<PatternVariant>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PatternVariant {
    Unit {
        name: String,
    },
    Struct {
        name: String,
        fields: Vec<PatternField>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PatternField {
    pub name: Option<String>,
    pub pattern: PatternExpr,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PatternExpr {
    Wildcard,
    Bind { name: String },
    Int { value: u64 },
    Bool { value: bool },
    EnumVariant { path: Vec<String> },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OutputExpr {
    pub group: String,
    pub variant: Option<String>,
    pub args: Vec<Expr>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Assignment {
    pub var: String,
    pub value: Expr,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Property {
    pub name: String,
    pub formula: Formula,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnaryOp {
    Not,
    BitNot,
    Neg,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
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
