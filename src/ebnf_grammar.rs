//! EBNF grammar model of the RFS v0 input language + renderers (Phase 7 task 5).
//!
//! `GRAMMAR_RULES` mirrors DESIGN.md section 6 exactly - same rule names, same
//! order, same structure. The sync tests in `tests/info_format_test.rs` pin
//! both directions: the DESIGN.md ebnf fence parses back to this exact table
//! (structural equality), and every `parser::KEYWORDS` entry appears as a
//! terminal here. This module is the single source of truth for what
//! `rfs info format spec-rfs` prints; if the parser language changes, update
//! DESIGN.md section 6 and this table together (the sync test fails otherwise).
//! Docs: dev:parser-overview, llm:parser

/// One node of an EBNF rule body.
///
/// All variants are const-constructible (slices + references instead of the
/// plan's `Vec`/`Box`, which are not const-stable), so `GRAMMAR_RULES` is a
/// plain `static`.
///
/// `Terminal` holds the literal text content WITHOUT surrounding quotes; the
/// text renderers quote it (escaping inner quotes) and the SVG renderer shows
/// it bare. Lexical placeholders (`character`, `a..z`) are terminals too -
/// they name a character class rather than a rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleNode {
    /// A literal token (e.g. the text of `"module"` or `";"`).
    Terminal(&'static str),
    /// A reference to another rule by name.
    NonTerminal(&'static str),
    /// Juxtaposition: all children, in order.
    Sequence(&'static [RuleNode]),
    /// Alternation: exactly one child.
    Choice(&'static [RuleNode]),
    /// Zero or one occurrences.
    Optional(&'static RuleNode),
    /// Zero or more occurrences.
    ZeroOrMore(&'static RuleNode),
    /// One or more occurrences (semantically `X {X}`).
    OneOrMore(&'static RuleNode),
}

// ---------------------------------------------------------------------------
// The grammar - mirrors DESIGN.md section 6, rule for rule.
// ---------------------------------------------------------------------------

const SOURCE: RuleNode = RuleNode::ZeroOrMore(&RuleNode::NonTerminal("module"));

const MODULE: RuleNode = RuleNode::Sequence(&[
    RuleNode::Terminal("module"),
    RuleNode::NonTerminal("identifier"),
    RuleNode::Optional(&RuleNode::Sequence(&[
        RuleNode::Terminal("version"),
        RuleNode::NonTerminal("string_literal"),
    ])),
    RuleNode::Terminal("{"),
    RuleNode::ZeroOrMore(&RuleNode::NonTerminal("module_item")),
    RuleNode::Terminal("}"),
]);

const MODULE_ITEM: RuleNode = RuleNode::Choice(&[
    RuleNode::NonTerminal("enum_decl"),
    RuleNode::NonTerminal("initial_decl"),
    RuleNode::NonTerminal("event_decl"),
    RuleNode::NonTerminal("output_decl"),
    RuleNode::NonTerminal("var_decl"),
    RuleNode::NonTerminal("transition"),
    RuleNode::NonTerminal("prop_decl"),
    RuleNode::NonTerminal("assume_decl"),
    RuleNode::NonTerminal("on_error_decl"),
]);

const ENUM_DECL: RuleNode = RuleNode::Sequence(&[
    RuleNode::Terminal("enum"),
    RuleNode::NonTerminal("identifier"),
    RuleNode::Terminal("{"),
    RuleNode::NonTerminal("identifier"),
    RuleNode::ZeroOrMore(&RuleNode::Sequence(&[
        RuleNode::Terminal(","),
        RuleNode::NonTerminal("identifier"),
    ])),
    RuleNode::Optional(&RuleNode::Terminal(",")),
    RuleNode::Terminal("}"),
]);

const INITIAL_DECL: RuleNode = RuleNode::Sequence(&[
    RuleNode::Terminal("initial"),
    RuleNode::NonTerminal("identifier"),
]);

const EVENT_DECL: RuleNode = RuleNode::Sequence(&[
    RuleNode::Terminal("event"),
    RuleNode::NonTerminal("event_head"),
]);

const OUTPUT_DECL: RuleNode = RuleNode::Sequence(&[
    RuleNode::Terminal("output"),
    RuleNode::NonTerminal("output_head"),
]);

const EVENT_HEAD: RuleNode = RuleNode::Choice(&[
    RuleNode::NonTerminal("identifier"),
    RuleNode::Sequence(&[
        RuleNode::NonTerminal("identifier"),
        RuleNode::Terminal("("),
        RuleNode::NonTerminal("event_variant_payload"),
        RuleNode::Terminal(")"),
    ]),
]);

const EVENT_VARIANT_PAYLOAD: RuleNode = RuleNode::Choice(&[
    RuleNode::NonTerminal("identifier"),
    RuleNode::Sequence(&[
        RuleNode::NonTerminal("identifier"),
        RuleNode::Terminal("("),
        RuleNode::NonTerminal("field_list"),
        RuleNode::Terminal(")"),
    ]),
]);

const OUTPUT_HEAD: RuleNode = RuleNode::Choice(&[
    RuleNode::NonTerminal("identifier"),
    RuleNode::Sequence(&[
        RuleNode::NonTerminal("identifier"),
        RuleNode::Terminal("("),
        RuleNode::NonTerminal("output_variant_payload"),
        RuleNode::Terminal(")"),
    ]),
]);

const OUTPUT_VARIANT_PAYLOAD: RuleNode = RuleNode::Choice(&[
    RuleNode::NonTerminal("identifier"),
    RuleNode::Sequence(&[
        RuleNode::NonTerminal("identifier"),
        RuleNode::Terminal("("),
        RuleNode::NonTerminal("field_list"),
        RuleNode::Terminal(")"),
    ]),
]);

const FIELD_LIST: RuleNode = RuleNode::Sequence(&[
    RuleNode::NonTerminal("field"),
    RuleNode::ZeroOrMore(&RuleNode::Sequence(&[
        RuleNode::Terminal(","),
        RuleNode::NonTerminal("field"),
    ])),
    RuleNode::Optional(&RuleNode::Terminal(",")),
]);

const FIELD: RuleNode = RuleNode::Sequence(&[
    RuleNode::NonTerminal("identifier"),
    RuleNode::Terminal(":"),
    RuleNode::NonTerminal("type"),
]);

const TYPE: RuleNode = RuleNode::Choice(&[
    RuleNode::Terminal("bool"),
    RuleNode::Sequence(&[
        RuleNode::Terminal("bit"),
        RuleNode::Terminal("<"),
        RuleNode::NonTerminal("unsigned_integer_literal"),
        RuleNode::Terminal(">"),
    ]),
    RuleNode::NonTerminal("identifier"),
]);

const VAR_DECL: RuleNode = RuleNode::Sequence(&[
    RuleNode::Terminal("var"),
    RuleNode::NonTerminal("identifier"),
    RuleNode::Terminal(":"),
    RuleNode::NonTerminal("type"),
    RuleNode::Terminal("init"),
    RuleNode::NonTerminal("expression"),
]);

const TRANSITION: RuleNode = RuleNode::Sequence(&[
    RuleNode::Terminal("transition"),
    RuleNode::NonTerminal("identifier"),
    RuleNode::Terminal("on"),
    RuleNode::NonTerminal("event_pattern"),
    RuleNode::Optional(&RuleNode::Sequence(&[
        RuleNode::Terminal("where"),
        RuleNode::NonTerminal("expression"),
    ])),
    RuleNode::Terminal("{"),
    RuleNode::ZeroOrMore(&RuleNode::NonTerminal("action")),
    RuleNode::Terminal("}"),
]);

const EVENT_PATTERN: RuleNode = RuleNode::Choice(&[
    RuleNode::NonTerminal("identifier"),
    RuleNode::Sequence(&[
        RuleNode::NonTerminal("identifier"),
        RuleNode::Terminal("("),
        RuleNode::NonTerminal("pattern_variant_payload"),
        RuleNode::Terminal(")"),
    ]),
]);

const PATTERN_VARIANT_PAYLOAD: RuleNode = RuleNode::Choice(&[
    RuleNode::NonTerminal("identifier"),
    RuleNode::Sequence(&[
        RuleNode::NonTerminal("identifier"),
        RuleNode::Terminal("("),
        RuleNode::NonTerminal("pattern_field_list"),
        RuleNode::Terminal(")"),
    ]),
]);

const PATTERN_FIELD_LIST: RuleNode = RuleNode::Sequence(&[
    RuleNode::NonTerminal("pattern_field"),
    RuleNode::ZeroOrMore(&RuleNode::Sequence(&[
        RuleNode::Terminal(","),
        RuleNode::NonTerminal("pattern_field"),
    ])),
    RuleNode::Optional(&RuleNode::Terminal(",")),
]);

const PATTERN_FIELD: RuleNode = RuleNode::Choice(&[
    RuleNode::Terminal("_"),
    RuleNode::NonTerminal("literal"),
    RuleNode::NonTerminal("identifier"),
    RuleNode::Sequence(&[
        RuleNode::NonTerminal("identifier"),
        RuleNode::Terminal(":"),
        RuleNode::NonTerminal("pattern_expr"),
    ]),
]);

const PATTERN_EXPR: RuleNode = RuleNode::Choice(&[
    RuleNode::Terminal("_"),
    RuleNode::NonTerminal("literal"),
    RuleNode::NonTerminal("identifier_path"),
]);

const ACTION: RuleNode = RuleNode::Choice(&[
    RuleNode::NonTerminal("next_action"),
    RuleNode::NonTerminal("emit_action"),
    RuleNode::NonTerminal("set_action"),
    RuleNode::NonTerminal("priority_action"),
]);

const NEXT_ACTION: RuleNode = RuleNode::Sequence(&[
    RuleNode::Terminal("next"),
    RuleNode::NonTerminal("identifier"),
    RuleNode::Terminal(";"),
]);

const EMIT_ACTION: RuleNode = RuleNode::Sequence(&[
    RuleNode::Terminal("emit"),
    RuleNode::NonTerminal("output_expr"),
    RuleNode::Terminal(";"),
]);

const SET_ACTION: RuleNode = RuleNode::Sequence(&[
    RuleNode::Terminal("set"),
    RuleNode::NonTerminal("identifier"),
    RuleNode::Terminal("="),
    RuleNode::NonTerminal("expression"),
    RuleNode::Terminal(";"),
]);

const PRIORITY_ACTION: RuleNode = RuleNode::Sequence(&[
    RuleNode::Terminal("priority"),
    RuleNode::NonTerminal("unsigned_integer_literal"),
    RuleNode::Terminal(";"),
]);

/// `on_error` body: exactly one `next` (required) plus any number of `emit`s,
/// in any order - which is exactly `emit* next emit*`.
const ON_ERROR_DECL: RuleNode = RuleNode::Sequence(&[
    RuleNode::Terminal("on_error"),
    RuleNode::Terminal("{"),
    RuleNode::ZeroOrMore(&RuleNode::NonTerminal("emit_action")),
    RuleNode::NonTerminal("next_action"),
    RuleNode::ZeroOrMore(&RuleNode::NonTerminal("emit_action")),
    RuleNode::Terminal("}"),
]);

const OUTPUT_EXPR: RuleNode = RuleNode::Choice(&[
    RuleNode::NonTerminal("identifier"),
    RuleNode::Sequence(&[
        RuleNode::NonTerminal("identifier"),
        RuleNode::Terminal("("),
        RuleNode::NonTerminal("output_call_payload"),
        RuleNode::Terminal(")"),
    ]),
]);

const OUTPUT_CALL_PAYLOAD: RuleNode = RuleNode::Choice(&[
    RuleNode::NonTerminal("identifier"),
    RuleNode::Sequence(&[
        RuleNode::NonTerminal("identifier"),
        RuleNode::Terminal("("),
        RuleNode::NonTerminal("expression"),
        RuleNode::ZeroOrMore(&RuleNode::Sequence(&[
            RuleNode::Terminal(","),
            RuleNode::NonTerminal("expression"),
        ])),
        RuleNode::Optional(&RuleNode::Terminal(",")),
        RuleNode::Terminal(")"),
    ]),
]);

const PROP_DECL: RuleNode = RuleNode::Sequence(&[
    RuleNode::Terminal("prop"),
    RuleNode::NonTerminal("identifier"),
    RuleNode::Terminal(":"),
    RuleNode::NonTerminal("formula"),
    RuleNode::Terminal(";"),
]);

const ASSUME_DECL: RuleNode = RuleNode::Sequence(&[
    RuleNode::Terminal("assume"),
    RuleNode::NonTerminal("identifier"),
    RuleNode::Terminal(":"),
    RuleNode::NonTerminal("formula"),
    RuleNode::Terminal(";"),
]);

const FORMULA: RuleNode = RuleNode::NonTerminal("or_formula");

const OR_FORMULA: RuleNode = RuleNode::Sequence(&[
    RuleNode::NonTerminal("impl_formula"),
    RuleNode::ZeroOrMore(&RuleNode::Sequence(&[
        RuleNode::Terminal("||"),
        RuleNode::NonTerminal("impl_formula"),
    ])),
]);

const IMPL_FORMULA: RuleNode = RuleNode::Sequence(&[
    RuleNode::NonTerminal("and_formula"),
    RuleNode::Optional(&RuleNode::Sequence(&[
        RuleNode::Terminal("->"),
        RuleNode::NonTerminal("impl_formula"),
    ])),
]);

const AND_FORMULA: RuleNode = RuleNode::Sequence(&[
    RuleNode::NonTerminal("unary_formula"),
    RuleNode::ZeroOrMore(&RuleNode::Sequence(&[
        RuleNode::Terminal("&&"),
        RuleNode::NonTerminal("unary_formula"),
    ])),
]);

const UNARY_FORMULA: RuleNode = RuleNode::NonTerminal("until_formula");

const UNTIL_FORMULA: RuleNode = RuleNode::Sequence(&[
    RuleNode::NonTerminal("neg_operand"),
    RuleNode::ZeroOrMore(&RuleNode::Sequence(&[
        RuleNode::Terminal("until"),
        RuleNode::NonTerminal("neg_operand"),
    ])),
]);

const NEG_OPERAND: RuleNode = RuleNode::Choice(&[
    RuleNode::Sequence(&[
        RuleNode::Terminal("!"),
        RuleNode::NonTerminal("neg_operand"),
    ]),
    RuleNode::NonTerminal("primary_formula"),
]);

const PRIMARY_FORMULA: RuleNode = RuleNode::Choice(&[
    RuleNode::Sequence(&[
        RuleNode::Terminal("next"),
        RuleNode::Terminal("("),
        RuleNode::NonTerminal("formula"),
        RuleNode::Terminal(")"),
    ]),
    RuleNode::Sequence(&[
        RuleNode::Terminal("always"),
        RuleNode::Terminal("["),
        RuleNode::NonTerminal("unsigned_integer_literal"),
        RuleNode::Terminal("]"),
        RuleNode::Terminal("("),
        RuleNode::NonTerminal("formula"),
        RuleNode::Terminal(")"),
    ]),
    RuleNode::Sequence(&[
        RuleNode::Terminal("always"),
        RuleNode::Terminal("("),
        RuleNode::NonTerminal("formula"),
        RuleNode::Terminal(")"),
    ]),
    RuleNode::Sequence(&[
        RuleNode::Terminal("bounded_eventually"),
        RuleNode::Terminal("["),
        RuleNode::NonTerminal("unsigned_integer_literal"),
        RuleNode::Terminal("]"),
        RuleNode::Terminal("("),
        RuleNode::NonTerminal("formula"),
        RuleNode::Terminal(")"),
    ]),
    RuleNode::Sequence(&[
        RuleNode::Terminal("eventually"),
        RuleNode::Terminal("("),
        RuleNode::NonTerminal("formula"),
        RuleNode::Terminal(")"),
    ]),
    RuleNode::Sequence(&[
        RuleNode::Terminal("input_is"),
        RuleNode::Terminal("("),
        RuleNode::NonTerminal("event_pattern"),
        RuleNode::Terminal(")"),
    ]),
    RuleNode::Sequence(&[
        RuleNode::Terminal("output_has"),
        RuleNode::Terminal("("),
        RuleNode::NonTerminal("output_pattern"),
        RuleNode::Terminal(")"),
    ]),
    RuleNode::Sequence(&[
        RuleNode::Terminal("("),
        RuleNode::NonTerminal("formula"),
        RuleNode::Terminal(")"),
    ]),
    RuleNode::NonTerminal("non_boolean_expr"),
]);

const OUTPUT_PATTERN: RuleNode = RuleNode::Choice(&[
    RuleNode::NonTerminal("identifier"),
    RuleNode::Sequence(&[
        RuleNode::NonTerminal("identifier"),
        RuleNode::Terminal("("),
        RuleNode::NonTerminal("pattern_variant_payload"),
        RuleNode::Terminal(")"),
    ]),
]);

const NON_BOOLEAN_EXPR: RuleNode = RuleNode::NonTerminal("equality_expr");

const EXPRESSION: RuleNode = RuleNode::NonTerminal("logic_or");

const LOGIC_OR: RuleNode = RuleNode::Sequence(&[
    RuleNode::NonTerminal("logic_and"),
    RuleNode::ZeroOrMore(&RuleNode::Sequence(&[
        RuleNode::Terminal("||"),
        RuleNode::NonTerminal("logic_and"),
    ])),
]);

const LOGIC_AND: RuleNode = RuleNode::Sequence(&[
    RuleNode::NonTerminal("equality_expr"),
    RuleNode::ZeroOrMore(&RuleNode::Sequence(&[
        RuleNode::Terminal("&&"),
        RuleNode::NonTerminal("equality_expr"),
    ])),
]);

const EQUALITY_EXPR: RuleNode = RuleNode::Sequence(&[
    RuleNode::NonTerminal("compare_expr"),
    RuleNode::ZeroOrMore(&RuleNode::Sequence(&[
        RuleNode::Choice(&[RuleNode::Terminal("=="), RuleNode::Terminal("!=")]),
        RuleNode::NonTerminal("compare_expr"),
    ])),
]);

const COMPARE_EXPR: RuleNode = RuleNode::Sequence(&[
    RuleNode::NonTerminal("bitwise_or_expr"),
    RuleNode::ZeroOrMore(&RuleNode::Sequence(&[
        RuleNode::Choice(&[
            RuleNode::Terminal("<"),
            RuleNode::Terminal("<="),
            RuleNode::Terminal(">"),
            RuleNode::Terminal(">="),
        ]),
        RuleNode::NonTerminal("bitwise_or_expr"),
    ])),
]);

const BITWISE_OR_EXPR: RuleNode = RuleNode::Sequence(&[
    RuleNode::NonTerminal("bitwise_xor_expr"),
    RuleNode::ZeroOrMore(&RuleNode::Sequence(&[
        RuleNode::Terminal("|"),
        RuleNode::NonTerminal("bitwise_xor_expr"),
    ])),
]);

const BITWISE_XOR_EXPR: RuleNode = RuleNode::Sequence(&[
    RuleNode::NonTerminal("bitwise_and_expr"),
    RuleNode::ZeroOrMore(&RuleNode::Sequence(&[
        RuleNode::Terminal("^"),
        RuleNode::NonTerminal("bitwise_and_expr"),
    ])),
]);

const BITWISE_AND_EXPR: RuleNode = RuleNode::Sequence(&[
    RuleNode::NonTerminal("shift_expr"),
    RuleNode::ZeroOrMore(&RuleNode::Sequence(&[
        RuleNode::Terminal("&"),
        RuleNode::NonTerminal("shift_expr"),
    ])),
]);

const SHIFT_EXPR: RuleNode = RuleNode::Sequence(&[
    RuleNode::NonTerminal("add_expr"),
    RuleNode::ZeroOrMore(&RuleNode::Sequence(&[
        RuleNode::Choice(&[RuleNode::Terminal("<<"), RuleNode::Terminal(">>")]),
        RuleNode::NonTerminal("add_expr"),
    ])),
]);

const ADD_EXPR: RuleNode = RuleNode::Sequence(&[
    RuleNode::NonTerminal("mul_expr"),
    RuleNode::ZeroOrMore(&RuleNode::Sequence(&[
        RuleNode::Choice(&[RuleNode::Terminal("+"), RuleNode::Terminal("-")]),
        RuleNode::NonTerminal("mul_expr"),
    ])),
]);

const MUL_EXPR: RuleNode = RuleNode::Sequence(&[
    RuleNode::NonTerminal("unary_expr"),
    RuleNode::ZeroOrMore(&RuleNode::Sequence(&[
        RuleNode::Terminal("*"),
        RuleNode::NonTerminal("unary_expr"),
    ])),
]);

const UNARY_EXPR: RuleNode = RuleNode::Choice(&[
    RuleNode::Sequence(&[
        RuleNode::Choice(&[
            RuleNode::Terminal("!"),
            RuleNode::Terminal("~"),
            RuleNode::Terminal("-"),
        ]),
        RuleNode::NonTerminal("unary_expr"),
    ]),
    RuleNode::NonTerminal("primary_expr"),
]);

const PRIMARY_EXPR: RuleNode = RuleNode::Choice(&[
    RuleNode::NonTerminal("literal"),
    RuleNode::NonTerminal("identifier_path"),
    RuleNode::Sequence(&[
        RuleNode::Terminal("("),
        RuleNode::NonTerminal("expression"),
        RuleNode::Terminal(")"),
    ]),
]);

const LITERAL: RuleNode = RuleNode::Choice(&[
    RuleNode::NonTerminal("unsigned_integer_literal"),
    RuleNode::Terminal("true"),
    RuleNode::Terminal("false"),
]);

const IDENTIFIER_PATH: RuleNode = RuleNode::Sequence(&[
    RuleNode::NonTerminal("identifier"),
    RuleNode::ZeroOrMore(&RuleNode::Sequence(&[
        RuleNode::Terminal("::"),
        RuleNode::NonTerminal("identifier"),
    ])),
]);

const IDENTIFIER: RuleNode = RuleNode::Sequence(&[
    RuleNode::NonTerminal("letter"),
    RuleNode::ZeroOrMore(&RuleNode::Choice(&[
        RuleNode::NonTerminal("letter"),
        RuleNode::NonTerminal("digit"),
        RuleNode::Terminal("_"),
    ])),
]);

const LETTER: RuleNode =
    RuleNode::Choice(&[RuleNode::Terminal("a..z"), RuleNode::Terminal("A..Z")]);

const DIGIT: RuleNode = RuleNode::Terminal("0..9");

const UNSIGNED_INTEGER_LITERAL: RuleNode = RuleNode::OneOrMore(&RuleNode::NonTerminal("digit"));

/// Double-quoted, no escapes; `character` is any character except `"` and
/// whitespace. Used only for the module version header.
const STRING_LITERAL: RuleNode = RuleNode::Sequence(&[
    RuleNode::Terminal("\""),
    RuleNode::ZeroOrMore(&RuleNode::Terminal("character")),
    RuleNode::Terminal("\""),
]);

/// The full grammar, in DESIGN.md section 6 order.
pub static GRAMMAR_RULES: &[(&str, RuleNode)] = &[
    ("source", SOURCE),
    ("module", MODULE),
    ("module_item", MODULE_ITEM),
    ("enum_decl", ENUM_DECL),
    ("initial_decl", INITIAL_DECL),
    ("event_decl", EVENT_DECL),
    ("output_decl", OUTPUT_DECL),
    ("event_head", EVENT_HEAD),
    ("event_variant_payload", EVENT_VARIANT_PAYLOAD),
    ("output_head", OUTPUT_HEAD),
    ("output_variant_payload", OUTPUT_VARIANT_PAYLOAD),
    ("field_list", FIELD_LIST),
    ("field", FIELD),
    ("type", TYPE),
    ("var_decl", VAR_DECL),
    ("transition", TRANSITION),
    ("event_pattern", EVENT_PATTERN),
    ("pattern_variant_payload", PATTERN_VARIANT_PAYLOAD),
    ("pattern_field_list", PATTERN_FIELD_LIST),
    ("pattern_field", PATTERN_FIELD),
    ("pattern_expr", PATTERN_EXPR),
    ("action", ACTION),
    ("next_action", NEXT_ACTION),
    ("emit_action", EMIT_ACTION),
    ("set_action", SET_ACTION),
    ("priority_action", PRIORITY_ACTION),
    ("on_error_decl", ON_ERROR_DECL),
    ("output_expr", OUTPUT_EXPR),
    ("output_call_payload", OUTPUT_CALL_PAYLOAD),
    ("prop_decl", PROP_DECL),
    ("assume_decl", ASSUME_DECL),
    ("formula", FORMULA),
    ("or_formula", OR_FORMULA),
    ("impl_formula", IMPL_FORMULA),
    ("and_formula", AND_FORMULA),
    ("unary_formula", UNARY_FORMULA),
    ("until_formula", UNTIL_FORMULA),
    ("neg_operand", NEG_OPERAND),
    ("primary_formula", PRIMARY_FORMULA),
    ("output_pattern", OUTPUT_PATTERN),
    ("non_boolean_expr", NON_BOOLEAN_EXPR),
    ("expression", EXPRESSION),
    ("logic_or", LOGIC_OR),
    ("logic_and", LOGIC_AND),
    ("equality_expr", EQUALITY_EXPR),
    ("compare_expr", COMPARE_EXPR),
    ("bitwise_or_expr", BITWISE_OR_EXPR),
    ("bitwise_xor_expr", BITWISE_XOR_EXPR),
    ("bitwise_and_expr", BITWISE_AND_EXPR),
    ("shift_expr", SHIFT_EXPR),
    ("add_expr", ADD_EXPR),
    ("mul_expr", MUL_EXPR),
    ("unary_expr", UNARY_EXPR),
    ("primary_expr", PRIMARY_EXPR),
    ("literal", LITERAL),
    ("identifier_path", IDENTIFIER_PATH),
    ("identifier", IDENTIFIER),
    ("letter", LETTER),
    ("digit", DIGIT),
    ("unsigned_integer_literal", UNSIGNED_INTEGER_LITERAL),
    ("string_literal", STRING_LITERAL),
];

/// One-line descriptions for the markdown rule table (kept in sync with the
/// grammar by review; the sync test pins names, not prose).
pub static RULE_NOTES: &[(&str, &str)] = &[
    ("source", "Top level: zero or more modules"),
    ("module", "Module header (optional version string) + items in braces"),
    ("module_item", "Any item inside a module body"),
    ("enum_decl", "Enum type with named variants (trailing comma allowed)"),
    ("initial_decl", "Initial state declaration"),
    ("event_decl", "Event group, optionally with one variant payload"),
    ("output_decl", "Output group, optionally with one variant payload"),
    ("event_head", "Bare group name or group(variant)"),
    ("event_variant_payload", "Variant name, optionally with a field list"),
    ("output_head", "Bare group name or group(variant)"),
    ("output_variant_payload", "Variant name, optionally with a field list"),
    ("field_list", "Comma-separated typed fields (trailing comma allowed)"),
    ("field", "One named field with a type"),
    ("type", "bool, bit<N>, or an enum identifier"),
    ("var_decl", "Module variable with type and init expression"),
    ("transition", "State + event pattern, optional guard, action block"),
    ("event_pattern", "Event group name, optionally with a variant pattern"),
    ("pattern_variant_payload", "Variant name, optionally with a pattern field list"),
    ("pattern_field_list", "Comma-separated pattern fields (trailing comma allowed)"),
    ("pattern_field", "Wildcard, literal, bound identifier, or named pattern"),
    ("pattern_expr", "Wildcard, literal, or identifier path (bind / enum variant)"),
    ("action", "One statement in a transition body"),
    ("next_action", "Target state of the transition"),
    ("emit_action", "Emit an output expression"),
    ("set_action", "Assign an expression to a variable"),
    ("priority_action", "Transition priority for tie-breaking (higher value wins)"),
    ("on_error_decl", "Module-level fallback: required next + optional emits"),
    ("output_expr", "Output group name, optionally with a variant call"),
    ("output_call_payload", "Variant name, optionally with argument expressions"),
    ("prop_decl", "Property (monitor-checked invariant)"),
    ("assume_decl", "Assumption (environment constraint)"),
    ("formula", "Top-level formula entry point"),
    ("or_formula", "Formula-level || (loosest precedence)"),
    ("impl_formula", "Formula-level -> (right-associative)"),
    ("and_formula", "Formula-level &&"),
    ("unary_formula", "Level separator: alias for until_formula"),
    ("until_formula", "Formula-level until (left-associative)"),
    ("neg_operand", "Formula-level ! (binds tighter than until)"),
    ("primary_formula", "next / always / bounded_eventually / eventually / input_is / output_has atoms, parenthesized formula, or non-boolean expression"),
    ("output_pattern", "Output group name, optionally with a variant pattern"),
    ("non_boolean_expr", "Formula fallthrough: comparisons and below (no logic levels)"),
    ("expression", "Top-level expression entry point (full ladder)"),
    ("logic_or", "Expression-level ||"),
    ("logic_and", "Expression-level &&"),
    ("equality_expr", "== / !="),
    ("compare_expr", "< <= > >="),
    ("bitwise_or_expr", "Bitwise |"),
    ("bitwise_xor_expr", "Bitwise ^"),
    ("bitwise_and_expr", "Bitwise &"),
    ("shift_expr", "<< / >>"),
    ("add_expr", "+ / -"),
    ("mul_expr", "*"),
    ("unary_expr", "Prefix ! ~ -"),
    ("primary_expr", "Literal, identifier path, or parenthesized expression"),
    ("literal", "Unsigned integer, true, or false"),
    ("identifier_path", "Identifier with optional :: segments (enum variant paths)"),
    ("identifier", "Letter followed by letters, digits, underscores"),
    ("letter", "ASCII letter a-z or A-Z"),
    ("digit", "ASCII digit 0-9"),
    ("unsigned_integer_literal", "One or more digits (u64 range)"),
    ("string_literal", "Double-quoted, no escapes (version header only)"),
];

// ---------------------------------------------------------------------------
// Renderers.
// ---------------------------------------------------------------------------

/// Quote a terminal for text EBNF output: double quotes with inner quotes
/// escaped (`"` -> `\"`).
fn quote_terminal(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        if c == '"' {
            out.push('\\');
        }
        out.push(c);
    }
    out.push('"');
    out
}

/// Emit one rule body as classic EBNF text (the DESIGN.md section 6 style):
/// juxtaposition for sequences, `|` for choices, `X?` / `[X]` for optional,
/// `X*` / `{X}` for zero-or-more, `X {X}` for one-or-more. Choices get
/// parentheses only in sequence position (wrapper delimiters scope them).
fn emit_node(node: &RuleNode, seq_position: bool) -> String {
    match node {
        RuleNode::Terminal(t) => quote_terminal(t),
        RuleNode::NonTerminal(n) => n.to_string(),
        RuleNode::Sequence(children) => children
            .iter()
            .map(|c| emit_node(c, true))
            .collect::<Vec<_>>()
            .join(" "),
        RuleNode::Choice(alternatives) => {
            let body = alternatives
                .iter()
                .map(|a| emit_node(a, true))
                .collect::<Vec<_>>()
                .join(" | ");
            if seq_position {
                format!("({body})")
            } else {
                body
            }
        }
        RuleNode::Optional(inner) => match inner {
            RuleNode::Terminal(_) | RuleNode::NonTerminal(_) => {
                format!("{}?", emit_node(inner, false))
            }
            _ => format!("[ {} ]", emit_node(inner, false)),
        },
        RuleNode::ZeroOrMore(inner) => match inner {
            RuleNode::NonTerminal(n) => format!("{n}*"),
            _ => format!("{{ {} }}", emit_node(inner, false)),
        },
        RuleNode::OneOrMore(inner) => {
            let x = emit_node(inner, true);
            let z = match inner {
                RuleNode::NonTerminal(n) => format!("{n}*"),
                _ => format!("{{ {} }}", emit_node(inner, false)),
            };
            format!("{x} {z}")
        }
    }
}

/// The full grammar as classic EBNF text: one rule per line, `name = body ;`.
pub fn to_ebnf_text() -> String {
    let mut out = String::new();
    for (name, node) in GRAMMAR_RULES {
        out.push_str(name);
        out.push_str(" = ");
        out.push_str(&emit_node(node, false));
        out.push_str(" ;\n");
    }
    out
}

/// Markdown rendering: intro + ebnf code block + rule table.
pub fn to_md() -> String {
    let mut out = String::new();
    out.push_str("# RFS v0 input language - EBNF grammar\n\n");
    out.push_str(
        "The grammar below is what `rfs parse` accepts (the parser is at least as lenient: it also tolerates missing commas between list items and missing semicolons after actions, which the canonical printer never emits). It is kept in sync with DESIGN.md section 6 by a test; this output is generated from the same model.\n\n",
    );
    out.push_str("```ebnf\n");
    out.push_str(&to_ebnf_text());
    out.push_str("```\n\n");
    out.push_str(
        "Formula precedence, loosest to tightest: `||`, `->`, `&&`, `until`, `!`. `&&`/`||` are formula-level operators; each operand is parsed independently via `primary_formula`, whose fallthrough is the non-boolean expression level (comparisons and below) - so mixed atom kinds like `state == S && output_has(O)` parse. `until` sits between `&&` and `!`: `!a until b` = `(!a) until b`, `a && b until c` = `a && (b until c)`.\n\n",
    );
    out.push_str("## Rules\n\n");
    out.push_str("| Rule | Description |\n");
    out.push_str("|------|-------------|\n");
    for (name, _) in GRAMMAR_RULES {
        let note = RULE_NOTES
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, d)| *d)
            .unwrap_or("");
        out.push_str(&format!("| `{name}` | {note} |\n"));
    }
    out
}

/// PlantUML EBNF rendering (`@startebnf`/`@endebnf`). Verified against the
/// current PlantUML tokenizer (no range suffixes: optional is `[X]`,
/// zero-or-more `{X}`, one-or-more `{X}-`; every rule line ends with `;`).
pub fn to_plantuml_ebnf() -> String {
    let mut out = String::from("@startebnf\n");
    for (name, node) in GRAMMAR_RULES {
        out.push_str(name);
        out.push_str(" = ");
        out.push_str(&emit_node_plantuml(node, true));
        out.push_str(" ;\n");
    }
    out.push_str("@endebnf\n");
    out
}

/// PlantUML-specific emission: same structure as `emit_node`, but terminals
/// are quoted strings (single-quoted when the text contains a double quote)
/// and one-or-more uses the `{X}-` sugar.
fn emit_node_plantuml(node: &RuleNode, seq_position: bool) -> String {
    match node {
        RuleNode::Terminal(t) => {
            if t.contains('"') {
                format!("'{t}'")
            } else {
                format!("\"{t}\"")
            }
        }
        RuleNode::NonTerminal(n) => n.to_string(),
        RuleNode::Sequence(children) => children
            .iter()
            .map(|c| emit_node_plantuml(c, true))
            .collect::<Vec<_>>()
            .join(" "),
        RuleNode::Choice(alternatives) => {
            let body = alternatives
                .iter()
                .map(|a| emit_node_plantuml(a, true))
                .collect::<Vec<_>>()
                .join(" | ");
            if seq_position {
                format!("({body})")
            } else {
                body
            }
        }
        RuleNode::Optional(inner) => format!("[{}]", emit_node_plantuml(inner, false)),
        RuleNode::ZeroOrMore(inner) => format!("{{{}}}", emit_node_plantuml(inner, false)),
        RuleNode::OneOrMore(inner) => format!("{{{}}}-", emit_node_plantuml(inner, false)),
    }
}

/// Railroad-diagram SVG: one labeled box per rule, stacked vertically
/// (unconnected - each rule is an independent diagram).
pub fn to_svg() -> Result<String, Vec<String>> {
    use railroad::{
        Choice, Comment, Diagram, Empty, LabeledBox, NonTerminal, Optional, Repeat, Sequence,
        Stylesheet, Terminal, VerticalGrid,
    };

    fn to_railroad(node: &RuleNode) -> Box<dyn railroad::Node> {
        match node {
            RuleNode::Terminal(t) => Box::new(Terminal::new((*t).to_string())),
            RuleNode::NonTerminal(n) => Box::new(NonTerminal::new((*n).to_string())),
            RuleNode::Sequence(children) => {
                Box::new(Sequence::new(children.iter().map(to_railroad).collect()))
            }
            RuleNode::Choice(alternatives) => {
                Box::new(Choice::new(alternatives.iter().map(to_railroad).collect()))
            }
            RuleNode::Optional(inner) => Box::new(Optional::new(to_railroad(inner))),
            RuleNode::ZeroOrMore(inner) => {
                Box::new(Optional::new(Repeat::new(to_railroad(inner), Empty)))
            }
            RuleNode::OneOrMore(inner) => Box::new(Repeat::new(to_railroad(inner), Empty)),
        }
    }

    let root = VerticalGrid::new(
        GRAMMAR_RULES
            .iter()
            .map(|(name, node)| {
                LabeledBox::new(to_railroad(node), Comment::new((*name).to_string()))
            })
            .collect(),
    );
    let diagram = Diagram::new_with_stylesheet(root, &Stylesheet::Light);
    // `Diagram` implements Display (the io::Write path needs a Vec<u8>);
    // the Display impl is the same SVG string.
    Ok(diagram.to_string())
}
