//! `rfs info format` - machine-readable descriptions of rfs input/output
//! formats (Phase 7 task 5).
//!
//! Registry pattern mirroring `DIAGRAMS` / `BACKENDS`: a static `FORMATS`
//! table, one entry per documented format. Each entry knows its supported
//! `--output` kinds and carries a render closure producing the document text.
//! New format entries are added by later Phase 7 tasks (spec-json, ce-trace,
//! mc-report, validate-json, info-json, diagram-*, artifact-header).
//!
//! Rationale: users/CI get actual information about rfs internals from the
//! binary itself, kept in sync with the code by tests - no stale docs.

use crate::codegen_mc as mc;
use crate::{ebnf_grammar, json_schema};
use json_schema::{Field, FieldType, SchemaDoc, TaggedVariant};

/// One documented format.
pub struct FormatInfo {
    /// Registry name (the CLI argument), e.g. `spec-rfs`.
    pub name: &'static str,
    /// One-line summary for `--list`.
    pub summary: &'static str,
    /// Supported `--output` kinds, in preference order; the first is the
    /// default when `--output` is omitted.
    pub outputs: &'static [&'static str],
    /// Render the document for one output kind into a string. Errors carry
    /// diagnostics (e.g. SVG rendering failures).
    pub render: fn(&str) -> Result<String, Vec<String>>,
}

fn spec_rfs_render(output: &str) -> Result<String, Vec<String>> {
    match output {
        "md" => Ok(ebnf_grammar::to_md()),
        "plantuml" => Ok(ebnf_grammar::to_plantuml_ebnf()),
        "svg" => ebnf_grammar::to_svg(),
        other => Err(vec![format!(
            "spec-rfs: unsupported output '{other}' (expected md, plantuml or svg)"
        )]),
    }
}

// ---------------------------------------------------------------------------
// spec-json - the IR v1 JSON document (Phase 7 task 6)
// ---------------------------------------------------------------------------

/// The field model of the IR v1 JSON document, derived from `ir.rs`
/// `to_json` (the single source of truth). Every key the writer can emit is
/// documented here; the walk test over all valid examples' parse JSON proves
/// the two stay in sync (D-P7-9).
pub fn spec_json_schema() -> SchemaDoc {
    let type_def = FieldType::tagged_union(vec![
        TaggedVariant::new("bool", vec![]),
        TaggedVariant::new(
            "bit",
            vec![Field::req("width", FieldType::int(), Some("bit width"))],
        ),
        TaggedVariant::new(
            "enum",
            vec![Field::req("name", FieldType::str(), Some("enum type name"))],
        ),
    ]);
    let field_decl = FieldType::obj(vec![
        Field::req("name", FieldType::str(), Some("field name")),
        Field::req("ty", FieldType::r#ref("type"), None),
    ]);
    let constructor_variant = FieldType::tagged_union(vec![
        TaggedVariant::new(
            "unit",
            vec![Field::req("name", FieldType::str(), Some("variant name"))],
        ),
        TaggedVariant::new(
            "struct",
            vec![
                Field::req("name", FieldType::str(), Some("variant name")),
                Field::req(
                    "fields",
                    FieldType::arr(FieldType::r#ref("field_decl")),
                    None,
                ),
            ],
        ),
    ]);
    let pattern_expr = FieldType::tagged_union(vec![
        TaggedVariant::new("wildcard", vec![]),
        TaggedVariant::new(
            "bind",
            vec![Field::req(
                "name",
                FieldType::str(),
                Some("bound variable name"),
            )],
        ),
        TaggedVariant::new("int", vec![Field::req("value", FieldType::int(), None)]),
        TaggedVariant::new("bool", vec![Field::req("value", FieldType::bool(), None)]),
        TaggedVariant::new(
            "enum_variant",
            vec![Field::req(
                "path",
                FieldType::arr(FieldType::str()),
                Some("enum variant path (type :: variant)"),
            )],
        ),
    ]);
    let pattern_variant = FieldType::tagged_union(vec![
        TaggedVariant::new(
            "unit",
            vec![Field::req("name", FieldType::str(), Some("variant name"))],
        ),
        TaggedVariant::new(
            "struct",
            vec![
                Field::req("name", FieldType::str(), Some("variant name")),
                Field::req(
                    "fields",
                    FieldType::arr(FieldType::r#ref("pattern_field")),
                    None,
                ),
            ],
        ),
    ]);
    let output_expr = FieldType::obj(vec![
        Field::req("group", FieldType::str(), Some("output group name")),
        Field::req(
            "variant",
            FieldType::nullable(FieldType::str()),
            Some("variant name; null for a bare `emit Group;` (implicit variant)"),
        ),
        Field::req("args", FieldType::arr(FieldType::r#ref("expr")), None),
    ]);
    let expr = FieldType::tagged_union(vec![
        TaggedVariant::new(
            "int",
            vec![Field::req(
                "value",
                FieldType::int(),
                Some("u64 value (i64 range in JSON)"),
            )],
        ),
        TaggedVariant::new("bool", vec![Field::req("value", FieldType::bool(), None)]),
        TaggedVariant::new(
            "path",
            vec![Field::req(
                "path",
                FieldType::arr(FieldType::str()),
                Some("identifier path (multi-segment = enum variant)"),
            )],
        ),
        TaggedVariant::new(
            "unary",
            vec![
                Field::req("op", FieldType::r#ref("unary_op"), None),
                Field::req("expr", FieldType::r#ref("expr"), None),
            ],
        ),
        TaggedVariant::new(
            "binary",
            vec![
                Field::req("op", FieldType::r#ref("bin_op"), None),
                Field::req("left", FieldType::r#ref("expr"), None),
                Field::req("right", FieldType::r#ref("expr"), None),
            ],
        ),
    ]);
    let formula = FieldType::tagged_union(vec![
        TaggedVariant::new(
            "input_is",
            vec![Field::req(
                "pattern",
                FieldType::r#ref("event_pattern"),
                None,
            )],
        ),
        TaggedVariant::new(
            "output_has",
            vec![Field::req(
                "pattern",
                FieldType::r#ref("output_pattern"),
                None,
            )],
        ),
        TaggedVariant::new(
            "expr",
            vec![Field::req("expr", FieldType::r#ref("expr"), None)],
        ),
        TaggedVariant::new(
            "not",
            vec![Field::req("inner", FieldType::r#ref("formula"), None)],
        ),
        TaggedVariant::new(
            "and",
            vec![
                Field::req("left", FieldType::r#ref("formula"), None),
                Field::req("right", FieldType::r#ref("formula"), None),
            ],
        ),
        TaggedVariant::new(
            "or",
            vec![
                Field::req("left", FieldType::r#ref("formula"), None),
                Field::req("right", FieldType::r#ref("formula"), None),
            ],
        ),
        TaggedVariant::new(
            "implies",
            vec![
                Field::req("left", FieldType::r#ref("formula"), None),
                Field::req("right", FieldType::r#ref("formula"), None),
            ],
        ),
        TaggedVariant::new(
            "next",
            vec![Field::req("inner", FieldType::r#ref("formula"), None)],
        ),
        TaggedVariant::new(
            "always",
            vec![Field::req("inner", FieldType::r#ref("formula"), None)],
        ),
        TaggedVariant::new(
            "bounded_always",
            vec![
                Field::req("bound", FieldType::int(), Some("tick bound")),
                Field::req("inner", FieldType::r#ref("formula"), None),
            ],
        ),
        TaggedVariant::new(
            "bounded_eventually",
            vec![
                Field::req("bound", FieldType::int(), Some("tick bound")),
                Field::req("inner", FieldType::r#ref("formula"), None),
            ],
        ),
        TaggedVariant::new(
            "eventually",
            vec![Field::req("inner", FieldType::r#ref("formula"), None)],
        ),
        TaggedVariant::new(
            "until",
            vec![
                Field::req("left", FieldType::r#ref("formula"), None),
                Field::req("right", FieldType::r#ref("formula"), None),
            ],
        ),
    ]);

    SchemaDoc {
        title: "IR v1 JSON document (spec-json)".into(),
        description: "The machine-readable form of an RFS specification: what\n`rfs parse --output json` writes and what every rfs command accepts as\n`--input json`. Pretty-printed JSON (2-space indent, one trailing newline)\nproduced by json_rw; key order is fixed by the writer. Internally-tagged\nobjects carry a `\"kind\"` discriminator - each variant's allowed fields are\nlisted under its kind. `null` marks an absent value that is still present as\na key (e.g. `guard`, `priority`); the keys `version` and `on_error` are\nomitted entirely when absent.".into(),
        root: FieldType::obj(vec![
            Field::req(
                "ir_version",
                FieldType::int(),
                Some("IR format version; this toolchain writes 1 and reads 0..=1"),
            ),
            Field::req(
                "modules",
                FieldType::arr(FieldType::r#ref("module")),
                Some("modules in file order"),
            ),
        ]),
        definitions: vec![
            (
                "module".into(),
                FieldType::obj(vec![
                    Field::req("name", FieldType::str(), Some("module name")),
                    Field::opt(
                        "version",
                        FieldType::str(),
                        Some("version string from the `version \"...\"` header; key omitted when absent"),
                    ),
                    Field::opt(
                        "on_error",
                        FieldType::r#ref("on_error"),
                        Some("module-level fallback for unexpected inputs; key omitted when absent"),
                    ),
                    Field::req(
                        "enums",
                        FieldType::arr(FieldType::r#ref("enum_decl")),
                        None,
                    ),
                    Field::req(
                        "initial_state",
                        FieldType::nullable(FieldType::str()),
                        Some("initial state name; null for a module with no states"),
                    ),
                    Field::req(
                        "events",
                        FieldType::arr(FieldType::r#ref("constructor_group")),
                        None,
                    ),
                    Field::req(
                        "outputs",
                        FieldType::arr(FieldType::r#ref("constructor_group")),
                        None,
                    ),
                    Field::req(
                        "vars",
                        FieldType::arr(FieldType::r#ref("var_decl")),
                        None,
                    ),
                    Field::req(
                        "transitions",
                        FieldType::arr(FieldType::r#ref("transition")),
                        None,
                    ),
                    Field::req(
                        "properties",
                        FieldType::arr(FieldType::r#ref("property")),
                        None,
                    ),
                    Field::req(
                        "assumptions",
                        FieldType::arr(FieldType::r#ref("property")),
                        None,
                    ),
                ]),
            ),
            (
                "on_error".into(),
                FieldType::obj(vec![
                    Field::req(
                        "next_state",
                        FieldType::str(),
                        Some("fallback target state (required)"),
                    ),
                    Field::req(
                        "emits",
                        FieldType::arr(FieldType::r#ref("output_expr")),
                        Some("outputs emitted on fallback (may be empty)"),
                    ),
                ]),
            ),
            (
                "enum_decl".into(),
                FieldType::obj(vec![
                    Field::req("name", FieldType::str(), None),
                    Field::req(
                        "variants",
                        FieldType::arr(FieldType::str()),
                        Some("variant names in declaration order"),
                    ),
                ]),
            ),
            ("type".into(), type_def),
            ("field_decl".into(), field_decl),
            (
                "constructor_group".into(),
                FieldType::obj(vec![
                    Field::req("name", FieldType::str(), Some("group name")),
                    Field::req(
                        "variants",
                        FieldType::arr(FieldType::r#ref("constructor_variant")),
                        None,
                    ),
                ]),
            ),
            ("constructor_variant".into(), constructor_variant),
            (
                "var_decl".into(),
                FieldType::obj(vec![
                    Field::req("name", FieldType::str(), None),
                    Field::req("ty", FieldType::r#ref("type"), None),
                    Field::req("init", FieldType::r#ref("expr"), Some("initial value expression")),
                ]),
            ),
            (
                "transition".into(),
                FieldType::obj(vec![
                    Field::req(
                        "id",
                        FieldType::int(),
                        Some("stable transition id (declaration order)"),
                    ),
                    Field::req("state", FieldType::str(), None),
                    Field::req("pattern", FieldType::r#ref("event_pattern"), None),
                    Field::req(
                        "guard",
                        FieldType::nullable(FieldType::r#ref("expr")),
                        Some("guard expression; null when the transition is unconditional"),
                    ),
                    Field::req("next_state", FieldType::str(), None),
                    Field::req(
                        "emits",
                        FieldType::arr(FieldType::r#ref("output_expr")),
                        None,
                    ),
                    Field::req(
                        "assignments",
                        FieldType::arr(FieldType::r#ref("assignment")),
                        None,
                    ),
                    Field::req(
                        "priority",
                        FieldType::nullable(FieldType::int()),
                        Some("tie-breaking priority (higher wins); null when unset"),
                    ),
                ]),
            ),
            (
                "event_pattern".into(),
                FieldType::obj(vec![
                    Field::req("group", FieldType::str(), None),
                    Field::req(
                        "variant",
                        FieldType::nullable(FieldType::r#ref("pattern_variant")),
                        Some("null when the pattern matches any variant of the group"),
                    ),
                ]),
            ),
            (
                "output_pattern".into(),
                FieldType::obj(vec![
                    Field::req("group", FieldType::str(), None),
                    Field::req(
                        "variant",
                        FieldType::nullable(FieldType::r#ref("pattern_variant")),
                        Some("null when the pattern matches any variant of the group"),
                    ),
                ]),
            ),
            ("pattern_variant".into(), pattern_variant),
            (
                "pattern_field".into(),
                FieldType::obj(vec![
                    Field::req(
                        "name",
                        FieldType::nullable(FieldType::str()),
                        Some("bound variable name; null for anonymous (positional) patterns"),
                    ),
                    Field::req("pattern", FieldType::r#ref("pattern_expr"), None),
                ]),
            ),
            ("pattern_expr".into(), pattern_expr),
            ("output_expr".into(), output_expr),
            (
                "assignment".into(),
                FieldType::obj(vec![
                    Field::req("var", FieldType::str(), None),
                    Field::req("value", FieldType::r#ref("expr"), None),
                ]),
            ),
            (
                "property".into(),
                FieldType::obj(vec![
                    Field::req("name", FieldType::str(), None),
                    Field::req("formula", FieldType::r#ref("formula"), None),
                ]),
            ),
            ("formula".into(), formula),
            ("expr".into(), expr),
            (
                "unary_op".into(),
                FieldType::enum_str(&["not", "bit_not", "neg"]),
            ),
            (
                "bin_op".into(),
                FieldType::enum_str(&[
                    "plus",
                    "minus",
                    "star",
                    "shift_left",
                    "shift_right",
                    "eq",
                    "not_eq",
                    "lt",
                    "le",
                    "gt",
                    "ge",
                    "bit_or",
                    "bit_xor",
                    "bit_and",
                    "logic_or",
                    "logic_and",
                ]),
            ),
        ],
        extra_sections: vec![],
    }
}

fn spec_json_render(output: &str) -> Result<String, Vec<String>> {
    let doc = spec_json_schema();
    match output {
        "md" => Ok(json_schema::render_md(&doc)),
        "schema" => Ok(json_schema::emit_schema_pretty(&doc)),
        other => Err(vec![format!(
            "spec-json: unsupported output '{other}' (expected md or schema)"
        )]),
    }
}

// ---------------------------------------------------------------------------
// ce-trace - the counterexample trace document (Phase 7 task 7)
// ---------------------------------------------------------------------------

/// The value model of the counterexample object + its named sub-schemas,
/// built against the `JSON_KEY_*` constants in `codegen_mc.rs` (the single
/// source of truth for key names and the per-kind key order). Shared by the
/// ce-trace entry (where it is the whole document) and the mc-report entry
/// (where it is the `counterexample` definition), so the two documents can
/// never drift apart. The walk tests over pinned FAIL-variant runs on both
/// backends prove the model stays in sync with the writer (D-P7-9).
pub fn counterexample_model() -> (FieldType, Vec<(String, FieldType)>) {
    // One input/output value: variant name + field values in declaration
    // order. Field KEYS are spec-defined names (arbitrary), so `fields` is a
    // free-keyed object over the shared value type.
    let io = FieldType::obj(vec![
        Field::req(
            mc::JSON_KEY_VARIANT,
            FieldType::str(),
            Some("event/output variant name (unique_name form, e.g. `Go_G`)"),
        ),
        Field::req(
            mc::JSON_KEY_FIELDS,
            FieldType::r#ref("ce_fields"),
            Some("field values in declaration order; keys are the field names"),
        ),
    ]);
    // One step of a trace (mirrors one `step N` line of the text report).
    let step = FieldType::obj(vec![
        Field::req(mc::JSON_KEY_INPUT, FieldType::r#ref("ce_io"), None),
        Field::req(
            mc::JSON_KEY_OUTPUTS,
            FieldType::arr(FieldType::r#ref("ce_io")),
            Some("outputs emitted by the step (may be empty)"),
        ),
        Field::req(
            mc::JSON_KEY_FROM_STATE,
            FieldType::str(),
            Some("state before the step (`State::Name`)"),
        ),
        Field::req(
            mc::JSON_KEY_TO_STATE,
            FieldType::str(),
            Some("state after the step (`State::Name`)"),
        ),
        Field::req(
            mc::JSON_KEY_VARS,
            FieldType::r#ref("ce_fields"),
            Some("variable values AFTER the step; keys are the variable names"),
        ),
    ]);

    let root = FieldType::tagged_union(vec![
        TaggedVariant::new(
            mc::JSON_CE_KIND_TRACE,
            vec![
                Field::req(
                    mc::JSON_KEY_INITIAL_STATE,
                    FieldType::str(),
                    Some("initial state (`State::Name`)"),
                ),
                Field::req(
                    mc::JSON_KEY_INITIAL_VARS,
                    FieldType::r#ref("ce_fields"),
                    Some(
                        "variable values at the initial configuration; keys are the variable names",
                    ),
                ),
                Field::req(
                    mc::JSON_KEY_STEPS,
                    FieldType::arr(FieldType::r#ref("ce_step")),
                    Some("the violating steps in execution order (0-based)"),
                ),
                ce_steps_field(),
            ],
        ),
        TaggedVariant::new(
            mc::JSON_CE_KIND_LASSO,
            vec![
                Field::req(
                    mc::JSON_KEY_INITIAL_STATE,
                    FieldType::str(),
                    Some("initial state (`State::Name`)"),
                ),
                Field::req(
                    mc::JSON_KEY_INITIAL_VARS,
                    FieldType::r#ref("ce_fields"),
                    Some(
                        "variable values at the initial configuration; keys are the variable names",
                    ),
                ),
                Field::req(
                    mc::JSON_KEY_PREFIX,
                    FieldType::arr(FieldType::r#ref("ce_step")),
                    Some("steps before the cycle (may be empty)"),
                ),
                Field::req(
                    mc::JSON_KEY_CYCLE,
                    FieldType::arr(FieldType::r#ref("ce_step")),
                    Some("one iteration of the violating cycle"),
                ),
                Field::req(
                    mc::JSON_KEY_CYCLE_LENGTH,
                    FieldType::int(),
                    Some("number of steps in `cycle` (== its length)"),
                ),
                ce_steps_field(),
            ],
        ),
        TaggedVariant::new(
            mc::JSON_CE_KIND_DEADLOCK_CONFIG,
            vec![
                Field::req(
                    mc::JSON_KEY_DEPTH,
                    FieldType::int(),
                    Some("graph depth of the dead configuration"),
                ),
                Field::req(
                    mc::JSON_KEY_STATE,
                    FieldType::str(),
                    Some("state of the dead configuration (`State::Name`)"),
                ),
                Field::req(
                    mc::JSON_KEY_VARS,
                    FieldType::r#ref("ce_fields"),
                    Some("variable values of the dead configuration; keys are the variable names"),
                ),
                ce_steps_field(),
            ],
        ),
    ]);
    let definitions = vec![
        ("ce_step".into(), step),
        ("ce_io".into(), io),
        (
            "ce_fields".into(),
            FieldType::free_obj(FieldType::r#ref("ce_value")),
        ),
        (
            "ce_value".into(),
            FieldType::union(vec![FieldType::bool(), FieldType::int(), FieldType::str()]),
        ),
    ];
    (root, definitions)
}

/// The ce-trace format entry: the counterexample object as a standalone
/// document (Phase 7 task 7).
pub fn ce_trace_schema() -> SchemaDoc {
    let (root, definitions) = counterexample_model();
    SchemaDoc {
        title: "Counterexample trace document (ce-trace)".into(),
        description: format!(
            "The counterexample payload of an exact-mode MC report - the value\nof the `{}` key, present IFF `status` is \"fail\" (see the mc-report\nformat for the full document). Emitted by both backends' generated engines\n(`--counterexample-format json`) and by the in-process reference MC; key\norder is part of the cross-backend byte-parity contract. Internally-tagged\nby `{}`: a property violation carries the full trace, a liveness violation\ncarries prefix + one cycle iteration, a deadlock carries the terminal\nconfiguration. `{}` (last key of every counterexample) is the trace-length\nquality metric: `steps.len()` for a trace, prefix + cycle length for a\nlasso, the depth for a deadlock configuration. Value encoding in the\nname->value maps (`initial_vars`, `vars`, `fields`): bool -> JSON boolean,\nbit<N> -> unsigned decimal integer, enum -> \"Type::Variant\" string - the\nsame names the text report prints.",
            mc::JSON_KEY_COUNTEREXAMPLE,
            mc::JSON_KEY_KIND,
            mc::JSON_KEY_CE_STEPS
        ),
        root,
        definitions,
        extra_sections: vec![(
            "text trace format (default report)".into(),
            TEXT_TRACE_MD.to_string(),
        )],
    }
}

fn ce_steps_field() -> Field {
    Field::req(
        mc::JSON_KEY_CE_STEPS,
        FieldType::int(),
        Some("trace length in steps (quality metric; last key of the object)"),
    )
}

/// The default `--counterexample-format text` report, as printed by both
/// backends' generated engines and the in-process reference MC (byte-
/// identical format, D-P6-10). Documented from `ref_mc::render_text_impl`
/// / `codegen_mc::exact_summary_line`.
const TEXT_TRACE_MD: &str = r#"The default report is a pinned human-readable document. On a failing run it consists of:

1. The status line: `FAIL` (or `PASS (complete)` / `UNKNOWN (state limit reached)`).
2. The machine summary line, followed by a blank line:
   `model-check summary: status=fail states=N transitions=M max_depth=D reason=<r> label=<L> step=S`
   - `<r>` is `property-violation`, `liveness-violation` or `deadlock`;
   - for a deadlock `label=-` and `step` is the dead configuration's depth;
   - otherwise `step` is the violation step (0-based).
3. Indented statistics lines: `unexpected terminations: N`, optionally
   `first unexpected: config C input index I`, then `monitor size: N bits`
   and `memory estimate: N bytes`.
4. A reason block:
   - property violation: `property violation: <label>` plus indented
     `trigger step: T` and `vars at trigger step: ...` (when the monitor
     fired before the violation was confirmed) and `violation step: S` with
     `vars at violation step: ...`; when the violation holds at the initial
     configuration the trace is empty and both lines carry the
     `(initial configuration)` suffix instead of var lines.
   - liveness violation: `liveness violation: <label>` plus indented
     `formula: <text>`, `violation step: S`, `cycle length: N` (the number of
     steps in the cycle) and a one-line `note:` explaining the cycle.
   - deadlock: `deadlock: configuration at depth D has no enabled transition`.
5. The trace itself:

   ```text
   counterexample trace:
     initial state=State::X vars=Vars { v1: val, ... }
     step 0 input=Input::V { f: val } outputs=[Output::O { f: val }] from=State::A to=State::B vars=Vars { ... }
   ```

   One `step N` line per executed step (N = 0-based index; for a lasso the
   prefix and cycle steps are printed as ONE flat sequence - the JSON report
   is what splits them). Value rendering: bool -> `true`/`false`, bit<N> ->
   unsigned decimal, enum -> `Enum::Variant`. A module with no variables
   prints bare `Vars`; an input/output without fields prints `Input::V` /
   `Output::O`; a step with no outputs prints `outputs=[]`."#;

fn ce_trace_render(output: &str) -> Result<String, Vec<String>> {
    let doc = ce_trace_schema();
    match output {
        "md" => Ok(json_schema::render_md(&doc)),
        "schema" => Ok(json_schema::emit_schema_pretty(&doc)),
        other => Err(vec![format!(
            "ce-trace: unsupported output '{other}' (expected md or schema)"
        )]),
    }
}

// ---------------------------------------------------------------------------
// mc-report - the full MC report document (Phase 7 task 8)
// ---------------------------------------------------------------------------

/// The field model of the full exact-mode MC report, built against the
/// `JSON_KEY_*` constants in `codegen_mc.rs` (key names + wire order) and the
/// `ExactStatus` / `FailReason` wire names (enum values - derived from the
/// methods, never string literals). The `counterexample` definition reuses
/// [`counterexample_model`] so it cannot drift from the ce-trace document.
/// The walk test over real benchmark runs on both backends proves the model
/// stays in sync with the writer (D-P7-9).
pub fn mc_report_schema() -> SchemaDoc {
    let (ce_root, ce_defs) = counterexample_model();
    let mut definitions = vec![
        (
            "mc_metrics".into(),
            FieldType::obj(vec![
                Field::req(
                    mc::JSON_KEY_STATES,
                    FieldType::int(),
                    Some("configurations enqueued (including the initial one)"),
                ),
                Field::req(
                    mc::JSON_KEY_TRANSITIONS,
                    FieldType::int(),
                    Some("enabled effects processed (success + pruned + violation; unexpected inputs do not count)"),
                ),
                Field::req(
                    mc::JSON_KEY_MAX_DEPTH,
                    FieldType::int(),
                    Some("greatest depth reached"),
                ),
            ]),
        ),
        (
            "mc_statistics".into(),
            FieldType::obj(vec![
                Field::req(
                    mc::JSON_KEY_UNEXPECTED_TERMINATIONS,
                    FieldType::int(),
                    Some("inputs with no enabled transition at their configuration"),
                ),
                Field::req(
                    mc::JSON_KEY_MONITOR_SIZE_BITS,
                    FieldType::int(),
                    Some("encoded bit width of all property + assumption monitors"),
                ),
                Field::req(
                    mc::JSON_KEY_MEMORY_ESTIMATE_BYTES,
                    FieldType::int(),
                    Some("states * (config bytes + 32) - the documented estimate formula"),
                ),
                Field::req(
                    mc::JSON_KEY_PEAK_RSS_KB,
                    FieldType::int(),
                    Some("INFORMATIONAL (machine-dependent): actual peak RSS in kB self-reported by the binary; 0 on read failure. Cross-backend byte comparisons normalize it to 0"),
                ),
            ]),
        ),
        ("counterexample".into(), ce_root),
    ];
    definitions.extend(ce_defs);

    SchemaDoc {
        title: "MC report document (mc-report)".into(),
        description: format!(
            "The full exact-mode MC report - the single compact JSON document\nboth backends' generated engines print with `--counterexample-format json`\nand the in-process reference MC builds (`rfs mc-ref --counterexample-format\njson`). Key order is part of the cross-backend byte-parity contract, EXCEPT\n`{peak}` which is INFORMATIONAL (machine-dependent; cross-backend byte\ncomparisons normalize it to 0). The fail fields (`{reason}`, `{label}`,\n`{step}`) and `{ce}` are present IFF `status` is \"fail\" - the schema marks\nthem optional (draft-07 without if/then conditionals, for widest validator\nsupport); the walk test over real runs pins the actual presence. `{ce}` is\nthe ce-trace document (see the ce-trace format for its text rendering).",
            peak = mc::JSON_KEY_PEAK_RSS_KB,
            reason = mc::JSON_KEY_REASON,
            label = mc::JSON_KEY_LABEL,
            step = mc::JSON_KEY_STEP,
            ce = mc::JSON_KEY_COUNTEREXAMPLE,
        ),
        root: FieldType::obj(vec![
            Field::req(
                mc::JSON_KEY_STATUS,
                FieldType::enum_str(&[
                    mc::ExactStatus::PassComplete.wire(),
                    mc::ExactStatus::Fail.wire(),
                    mc::ExactStatus::UnknownStateLimit.wire(),
                ]),
                Some("run status (wire names of ExactStatus)"),
            ),
            Field::req(
                mc::JSON_KEY_METRICS,
                FieldType::r#ref("mc_metrics"),
                None,
            ),
            Field::opt(
                mc::JSON_KEY_REASON,
                FieldType::enum_str(&[
                    mc::FailReason::PropertyViolation.wire(),
                    mc::FailReason::LivenessViolation.wire(),
                    mc::FailReason::Deadlock.wire(),
                ]),
                Some("why the run failed (wire names of FailReason); present IFF status is \"fail\""),
            ),
            Field::opt(
                mc::JSON_KEY_LABEL,
                FieldType::str(),
                Some("violated property/assumption label; `-` for a deadlock; present IFF status is \"fail\""),
            ),
            Field::opt(
                mc::JSON_KEY_STEP,
                FieldType::int(),
                Some("violation step (0-based); the dead configuration's depth for a deadlock; present IFF status is \"fail\""),
            ),
            Field::req(
                mc::JSON_KEY_STATISTICS,
                FieldType::r#ref("mc_statistics"),
                None,
            ),
            Field::opt(
                mc::JSON_KEY_COUNTEREXAMPLE,
                FieldType::r#ref("counterexample"),
                Some("the counterexample payload (ce-trace document); present IFF status is \"fail\""),
            ),
        ]),
        definitions,
        extra_sections: vec![],
    }
}

fn mc_report_render(output: &str) -> Result<String, Vec<String>> {
    let doc = mc_report_schema();
    match output {
        "md" => Ok(json_schema::render_md(&doc)),
        "schema" => Ok(json_schema::emit_schema_pretty(&doc)),
        other => Err(vec![format!(
            "mc-report: unsupported output '{other}' (expected md or schema)"
        )]),
    }
}

// ---------------------------------------------------------------------------
// validate-json - structured diagnostics of `rfs validate --json`
// (Phase 7 task 9)
// ---------------------------------------------------------------------------

/// The field model of the `rfs validate --json` document, derived from
/// `frontend::validate_json_doc` / `render_validate_json` (the single source
/// of truth for key names and order). File mode emits ONE object for the
/// input file; directory mode emits an array of one object per FAILING file
/// (valid files are omitted), so the root is a union of the two shapes over
/// the same `validate_file` definition. The walk test over all 13 invalid
/// examples + every valid example proves the model stays in sync with the
/// writer (D-P7-9).
pub fn validate_json_schema() -> SchemaDoc {
    let file_doc = FieldType::obj(vec![
        Field::req(
            "file",
            FieldType::str(),
            Some("the input path as passed on the command line (file mode) or the collected file path (directory mode)"),
        ),
        Field::req(
            "ok",
            FieldType::bool(),
            Some("true IFF `errors` is empty"),
        ),
        Field::req(
            "errors",
            FieldType::arr(FieldType::r#ref("validate_error")),
            Some("structured diagnostics; empty when ok (file mode prints the object even on success)"),
        ),
    ]);
    let error_obj = FieldType::obj(vec![
        Field::req(
            "kind",
            // The literals mirror the call sites in frontend.rs
            // (`StructuredError { kind: "parse" | "semantic", .. }`).
            FieldType::enum_str(&["parse", "semantic"]),
            Some("\"parse\" = RFS source rejected by the parser (line/col are the 1-based error position); \"semantic\" = parsed IR rejected by the validator, or a .json input that is not valid IR JSON in directory mode (no span: line/col are 0)"),
        ),
        Field::req(
            "line",
            FieldType::int(),
            Some("1-based error line; 0 when unknown (semantic errors carry no span)"),
        ),
        Field::req(
            "col",
            FieldType::int(),
            Some("1-based error column; 0 when unknown"),
        ),
        Field::req(
            "message",
            FieldType::str(),
            Some("the diagnostic text (same wording as the stderr report)"),
        ),
    ]);

    SchemaDoc {
        title: "Validate diagnostics document (validate-json)".into(),
        description:
            "The structured diagnostics document of `rfs validate --json` -\nthe shape editor integrations consume (Phase 5 task 13). File mode prints\nONE object for the input file (`ok: true` with an empty `errors` array on\nsuccess); directory mode prints an array of one object per FAILING file -\nvalid files are omitted entirely, so an all-valid directory prints `[]`.\nExit codes mirror the text path: 0 when nothing failed, 1 otherwise. The\nroot is a union of the two shapes (draft-07 anyOf over the same\n`validate_file` definition). A `.json` input that is not valid IR JSON\nproduces a CLI-level stderr error in file mode (no document) but appears as\na `kind: \"semantic\"` entry (line/col 0) in directory mode.".into(),
        root: FieldType::union(vec![
            FieldType::r#ref("validate_file"),
            FieldType::arr(FieldType::r#ref("validate_file")),
        ]),
        definitions: vec![
            ("validate_file".into(), file_doc),
            ("validate_error".into(), error_obj),
        ],
        extra_sections: vec![(
            "file mode vs directory mode".into(),
            VALIDATE_MODES_MD.to_string(),
        )],
    }
}

/// The two document shapes with examples (md-only section; never emitted
/// into the schema).
const VALIDATE_MODES_MD: &str = r#"The same `validate_file` object appears in two shapes, selected by the input kind:

File mode (`rfs validate --json spec.rfs`) - one object, printed even on success:

```json
{
  "file": "spec.rfs",
  "ok": true,
  "errors": []
}
```

Directory mode (`rfs validate --json --input rfs specs/`) - an array of one
object per FAILING file; valid files are omitted (an all-valid directory
prints `[]`):

```json
[
  {
    "file": "specs/broken.rfs",
    "ok": false,
    "errors": [
      { "kind": "parse", "line": 7, "col": 9, "message": "RFS parse error" }
    ]
  }
]
```

Exit codes mirror the text path: 0 when nothing failed, 1 otherwise. A
`.json` input that is not valid IR JSON produces a CLI-level stderr error in
file mode (no document) but appears as a `kind: "semantic"` entry (line/col
0) in directory mode."#;

fn validate_json_render(output: &str) -> Result<String, Vec<String>> {
    let doc = validate_json_schema();
    match output {
        "md" => Ok(json_schema::render_md(&doc)),
        "schema" => Ok(json_schema::emit_schema_pretty(&doc)),
        other => Err(vec![format!(
            "validate-json: unsupported output '{other}' (expected md or schema)"
        )]),
    }
}

// ---------------------------------------------------------------------------
// info-json - the `rfs info <spec> --json` introspection document
// (Phase 7 task 10)
// ---------------------------------------------------------------------------

/// The field model of the `rfs info <spec> --json` document, derived from
/// `info.rs` `render_json` / `module_json` (the single source of truth for
/// key names and order). Unlike the spec-json IR document: types/expressions/
/// formulas are rendered as human-readable strings (the same text the default
/// `rfs info` prints), `transitions` is an integer COUNT (not an array of
/// transition objects), and each property/assumption carries its
/// `monitor_status` (the R1-R4 monitor-subset verdict shared with the
/// codegen backends). The walk test over all valid examples proves the model
/// stays in sync with the writer (D-P7-9).
pub fn info_json_schema() -> SchemaDoc {
    let field = FieldType::obj(vec![
        Field::req("name", FieldType::str(), Some("field name")),
        Field::req(
            "type",
            FieldType::str(),
            Some("rendered type text (e.g. bool, bit<8>, an enum type name)"),
        ),
    ]);
    let variant = FieldType::obj(vec![
        Field::req("name", FieldType::str(), Some("variant name")),
        Field::req(
            "fields",
            FieldType::nullable(FieldType::arr(FieldType::r#ref("info_field"))),
            Some("null for a unit variant; the field list for a struct variant"),
        ),
    ]);
    let group = FieldType::obj(vec![
        Field::req(
            "name",
            FieldType::str(),
            Some("group name (event or output)"),
        ),
        Field::req(
            "variants",
            FieldType::arr(FieldType::r#ref("info_variant")),
            None,
        ),
    ]);
    let on_error = FieldType::obj(vec![
        Field::req(
            "next_state",
            FieldType::str(),
            Some("the state entered when an input has no enabled transition"),
        ),
        Field::req(
            "emits",
            FieldType::arr(FieldType::r#ref("info_emit")),
            Some("outputs emitted on the error transition (may be empty)"),
        ),
    ]);
    let emit = FieldType::obj(vec![
        Field::req("group", FieldType::str(), Some("output group name")),
        Field::req(
            "variant",
            FieldType::nullable(FieldType::str()),
            Some("variant name; null when the group has a single unnamed variant"),
        ),
    ]);
    let prop = FieldType::obj(vec![
        Field::req("name", FieldType::str(), Some("property/assumption label")),
        Field::req(
            "formula",
            FieldType::str(),
            Some("rendered formula text (the same text the default `rfs info` prints)"),
        ),
        Field::req(
            "monitor_status",
            FieldType::str(),
            Some("\"monitored\" or \"skipped: <reason>\" - the SAME R1-R4 monitor-subset check the codegen backends use, so info never disagrees with generated code; the reason text is open-ended, hence a plain string (not an enum)"),
        ),
    ]);

    SchemaDoc {
        title: "Info introspection document (info-json)".into(),
        description:
            "The machine-readable introspection document of `rfs info <spec> --json`\n(Phase 5 task 3) - what IDE/tooling consumers parse. One object per module in\nthe top-level `modules` array; stable key order, all keys always present with\nJSON null for absent values (a fixed schema). Unlike the spec-json IR\ndocument: types/expressions/formulas are rendered as human-readable strings,\n`transitions` is an integer COUNT (not an array of transition objects - use\nspec-json for the full transition data), and each property/assumption carries\nits `monitor_status`.".into(),
        root: FieldType::obj(vec![Field::req(
            "modules",
            FieldType::arr(FieldType::r#ref("info_module")),
            Some("one object per module, in spec order"),
        )]),
        definitions: vec![
            (
                "info_module".into(),
                FieldType::obj(vec![
                    Field::req("name", FieldType::str(), Some("module name")),
                    Field::req(
                        "version",
                        FieldType::nullable(FieldType::str()),
                        Some("the version header (X.Y or X.Y.Z); null when the module has none"),
                    ),
                    Field::req(
                        "initial_state",
                        FieldType::nullable(FieldType::str()),
                        Some("the initial state name; null when absent"),
                    ),
                    Field::req(
                        "enums",
                        FieldType::arr(FieldType::r#ref("info_enum")),
                        None,
                    ),
                    Field::req(
                        "events",
                        FieldType::arr(FieldType::r#ref("info_group")),
                        None,
                    ),
                    Field::req(
                        "outputs",
                        FieldType::arr(FieldType::r#ref("info_group")),
                        None,
                    ),
                    Field::req(
                        "vars",
                        FieldType::arr(FieldType::r#ref("info_var")),
                        None,
                    ),
                    Field::req(
                        "transitions",
                        FieldType::int(),
                        Some("the NUMBER of transitions in the module - an integer count, NOT an array of transition objects (use spec-json for the full transition data)"),
                    ),
                    Field::req(
                        "on_error",
                        FieldType::nullable(FieldType::r#ref("info_on_error")),
                        Some("the on_error declaration; null when the module has none"),
                    ),
                    Field::req(
                        "properties",
                        FieldType::arr(FieldType::r#ref("info_prop")),
                        None,
                    ),
                    Field::req(
                        "assumptions",
                        FieldType::arr(FieldType::r#ref("info_prop")),
                        None,
                    ),
                ]),
            ),
            (
                "info_enum".into(),
                FieldType::obj(vec![
                    Field::req("name", FieldType::str(), Some("enum type name")),
                    Field::req(
                        "variants",
                        FieldType::arr(FieldType::str()),
                        Some("variant names, in declaration order"),
                    ),
                ]),
            ),
            ("info_group".into(), group),
            ("info_variant".into(), variant),
            ("info_field".into(), field),
            (
                "info_var".into(),
                FieldType::obj(vec![
                    Field::req("name", FieldType::str(), Some("variable name")),
                    Field::req(
                        "type",
                        FieldType::str(),
                        Some("rendered type text (e.g. bool, bit<8>, an enum type name)"),
                    ),
                    Field::req(
                        "init",
                        FieldType::str(),
                        Some("rendered initializer expression text"),
                    ),
                ]),
            ),
            ("info_on_error".into(), on_error),
            ("info_emit".into(), emit),
            ("info_prop".into(), prop),
        ],
        extra_sections: vec![],
    }
}

fn info_json_render(output: &str) -> Result<String, Vec<String>> {
    let doc = info_json_schema();
    match output {
        "md" => Ok(json_schema::render_md(&doc)),
        "schema" => Ok(json_schema::emit_schema_pretty(&doc)),
        other => Err(vec![format!(
            "info-json: unsupported output '{other}' (expected md or schema)"
        )]),
    }
}

// ---------------------------------------------------------------------------
// diagram-dot - the DOT subset `rfs diagram --format dot` generates
// (Phase 7 task 11)
// ---------------------------------------------------------------------------

/// The documented DOT subset, derived from `diagrams/dot.rs` `generate`
/// (the single source of truth). The sync test over all 50 valid examples
/// asserts that every generated line is one of the documented shapes filled
/// with the module's IR data - no undocumented token (D-P7-9).
const DOT_SUBSET_MD: &str = r##"# DOT state diagram subset (diagram-dot)

The Graphviz DOT text that `rfs diagram --format dot` generates for one module.
Generation is pure Rust and deterministic: states in State-enum declaration
order, transitions in IR order, on_error edges last. The output is EXACTLY the
following structure - nothing else is ever emitted (the sync test over all 50
valid examples enforces it):

```text
digraph <module_name> {
  rankdir=LR;
  node [shape=box];
  <one line per state, declaration order>
  <one line per transition, IR order>
  <one dashed line per state - only when the module declares on_error>
}
```

## Graph header (always exactly these three lines)

| Line | Meaning |
|---|---|
| `digraph <module_name> {` | graph name = the module name (the only unquoted string in the file) |
| `  rankdir=LR;` | left-to-right layout (fixed) |
| `  node [shape=box];` | default node shape for every state (fixed) |

## Node lines (one per State-enum variant, declaration order)

| Line | When |
|---|---|
| `  "<state>";` | a non-initial state |
| `  "<state>" [style=filled, fillcolor="#e0e0e0"];` | the initial state - the only node that carries attributes |

## Edge lines (one per transition, IR order)

```text
  "<from>" -> "<to>" [label="<EventLabel> [guard] -> <next>"];
```

- `<EventLabel>`: the event pattern rendered as `Group`, `Group(Variant)` or
  `Group(Variant(args))`; args are pattern expressions - `_` wildcard, a bound
  name, an integer/boolean literal, or a `Type.Variant` path.
- `[guard]`: present IFF the transition has a guard; the rendered expression
  (literals, dot-separated paths, unary `!`/`~`/`-`, binary operators with
  parentheses).
- The label always ends with ` -> <next>` where `<next>` is the edge target.

## on_error edges (only when the module declares on_error)

One dashed edge per state to the fallback target (it fires from ANY state):

```text
  "<state>" -> "<fallback>" [label="on_error -> <fallback>", style=dashed];
```

## String escaping

Every node name and label is wrapped in double quotes; inside, backslash and
double quote are escaped (backslash -> double backslash, double quote ->
backslash + double quote). The module name in the `digraph` line is the only
unquoted string.
"##;

fn diagram_dot_render(output: &str) -> Result<String, Vec<String>> {
    match output {
        "md" => Ok(DOT_SUBSET_MD.to_string()),
        other => Err(vec![format!(
            "diagram-dot: unsupported output '{other}' (expected md)"
        )]),
    }
}

// ---------------------------------------------------------------------------
// diagram-puml - the PlantUML subset `rfs diagram --format puml` generates
// (Phase 7 task 12)
// ---------------------------------------------------------------------------

/// The documented PlantUML subset, derived from `diagrams/plantuml.rs`
/// `generate` (the single source of truth). The sync test over all 50 valid
/// examples asserts that every generated line is one of the documented shapes
/// filled with the module's IR data - no undocumented token (D-P7-9).
const PUML_SUBSET_MD: &str = r##"# PlantUML state diagram subset (diagram-puml)

The PlantUML text that `rfs diagram --format puml` generates for one module.
Generation is pure Rust and deterministic: transitions in IR order, on_error
edges last. The output is EXACTLY the following structure - nothing else is
ever emitted (the sync test over all 50 valid examples enforces it):

```text
@startuml
title <module_name>
[*] --> <initial>            <- only when the module has an initial state
<transition lines + note-on-link lines, IR order>
<on_error lines + note-on-link lines - only when the module declares on_error>
@enduml
```

## Frame lines (always exactly these)

| Line | When |
|---|---|
| `@startuml` | first line, always |
| `title <module_name>` | second line, always (the module name verbatim) |
| `[*] --> <initial>` | only when the module has an initial state |
| `@enduml` | last line, always |

## Transition lines (one per transition, IR order)

```text
<from> --> <to> : <EventLabel>
<from> --> <to> : <EventLabel> \n <guard>
```

- `<EventLabel>`: the event pattern rendered as `Group`, `Group(Variant)` or
  `Group(Variant(args))`; args are pattern expressions - `_` wildcard, a bound
  name, an integer/boolean literal, or a `Type.Variant` path.
- With a guard the label is `<EventLabel> \n <guard>`: `\n` is the LITERAL
  two-character sequence (backslash + n) followed by a space - not a newline.
  The guard is the rendered expression (literals, dot-separated paths, unary
  `!`/`~`/`-`, binary operators with parentheses).

## note-on-link lines (emitted data)

Directly after each transition line, one line per emitted output:

```text
note on link : emits <OutputLabel>
```

- `<OutputLabel>`: the output rendered as `Group`, `Group(Variant)` or
  `Group(Variant(args))` (args are rendered expressions).
- The position word is omitted (`note on link`, not `note right on link`) -
  plantuml-core's parser rejects the position word; real PlantUML accepts both.

## on_error edges (only when the module declares on_error)

One dashed edge per state to the fallback target (it fires from ANY state):

```text
<state> -[dashed]-> <fallback> : on_error
```

- The bracketed `-[dashed]->` form is used - unbracketed `-dashed->` is not
  valid PlantUML.
- The label is exactly `on_error` (no guard part).
- Directly after, one `note on link : emits <OutputLabel>` line per output the
  on_error declaration emits.

## State names and escaping

States have no explicit declarations - they appear implicitly as edge
endpoints (and as the `[*] -->` target). Names are inserted VERBATIM: no
quoting or escaping is performed, which is safe because RFS identifier grammar
guarantees none of the characters that would break PlantUML line structure
(newlines, `:` inside a label) occur in emitted data.
"##;

fn diagram_puml_render(output: &str) -> Result<String, Vec<String>> {
    match output {
        "md" => Ok(PUML_SUBSET_MD.to_string()),
        other => Err(vec![format!(
            "diagram-puml: unsupported output '{other}' (expected md)"
        )]),
    }
}

// ---------------------------------------------------------------------------
// diagram-mmd - the Mermaid subset `rfs diagram --format mmd` generates
// (Phase 7 task 13)
// ---------------------------------------------------------------------------

/// The documented Mermaid subset, derived from `diagrams/mermaid.rs`
/// `generate` (the single source of truth). The sync test over all 50 valid
/// examples asserts that every generated line is one of the documented shapes
/// filled with the module's IR data - no undocumented token (D-P7-9).
const MMD_SUBSET_MD: &str = r##"# Mermaid state diagram subset (diagram-mmd)

The Mermaid text that `rfs diagram --format mmd` generates for one module.
Generation is pure Rust and deterministic: transitions in IR order, on_error
edges last. The output is EXACTLY the following structure - nothing else is
ever emitted (the sync test over all 50 valid examples enforces it):

```text
stateDiagram-v2
  [*] --> <initial>            <- only when the module has an initial state
  <transition lines, IR order>
  <on_error lines - only when the module declares on_error>
```

## Frame lines (always exactly these)

| Line | When |
|---|---|
| `stateDiagram-v2` | first line, always (the only unindented line) |
| `  [*] --> <initial>` | only when the module has an initial state |

There is no closing frame line - the file ends with the last edge line.

## Transition lines (one per transition, IR order)

```text
  <from> --> <to>: <EventLabel>
  <from> --> <to>: <EventLabel> \n <guard>
  <from> --> <to>: <EventLabel> \n emits <OutputLabel>
```

- `<EventLabel>`: the event pattern rendered as `Group`, `Group(Variant)` or
  `Group(Variant(args))`; args are pattern expressions - `_` wildcard, a bound
  name, an integer/boolean literal, or a `Type.Variant` path.
- With a guard the label gains a second line: `<EventLabel> \n <guard>` where
  `\n` is the LITERAL two-character sequence (backslash + n) followed by a
  space - not a newline. The guard is the rendered expression (literals,
  dot-separated paths, unary `!`/`~`/`-`, binary operators with parentheses).
- Each emitted output appends one more label line: ` \n emits <OutputLabel>`
  where `<OutputLabel>` is the output rendered as `Group`, `Group(Variant)` or
  `Group(Variant(args))` (args are rendered expressions).
- Mermaid state diagrams have NO notes-on-links and NO dotted/dashed edges
  (both verified unsupported), so ALL transition information - event, guard,
  emitted outputs - lives in this one multi-line edge label.

## on_error edges (only when the module declares on_error)

One edge per state to the fallback target (it fires from ANY state):

```text
  <state> --> <fallback>: [on_error]
  <state> --> <fallback>: [on_error] \n emits <OutputLabel>
```

- The first label line is exactly `[on_error]` (the event name in brackets);
  there is no guard part on on_error edges.
- One ` \n emits <OutputLabel>` label line per output the on_error declaration
  emits, same form as on transition lines.

## State names and escaping

States have no explicit declarations - they appear implicitly as edge
endpoints (and as the `[*] -->` target). Names are inserted VERBATIM: no
quoting or escaping is performed, which is safe because RFS identifier grammar
guarantees none of the characters that would break Mermaid line structure
(newlines, `-->`, `:`) occur in emitted data.
"##;

fn diagram_mmd_render(output: &str) -> Result<String, Vec<String>> {
    match output {
        "md" => Ok(MMD_SUBSET_MD.to_string()),
        other => Err(vec![format!(
            "diagram-mmd: unsupported output '{other}' (expected md)"
        )]),
    }
}

/// The format registry. Order = `--list` order.
pub static FORMATS: &[FormatInfo] = &[
    FormatInfo {
        name: "spec-rfs",
        summary: "RFS v0 input language - EBNF grammar (what rfs parse accepts)",
        outputs: &["md", "plantuml", "svg"],
        render: spec_rfs_render,
    },
    FormatInfo {
        name: "spec-json",
        summary: "IR v1 JSON document (rfs parse --output json; the machine-readable spec)",
        outputs: &["md", "schema"],
        render: spec_json_render,
    },
    FormatInfo {
        name: "ce-trace",
        summary:
            "MC counterexample trace (the report's `counterexample` object + the text trace format)",
        outputs: &["md", "schema"],
        render: ce_trace_render,
    },
    FormatInfo {
        name: "mc-report",
        summary: "Full exact-mode MC report (--counterexample-format json; status, metrics, statistics + counterexample)",
        outputs: &["md", "schema"],
        render: mc_report_render,
    },
    FormatInfo {
        name: "validate-json",
        summary: "rfs validate --json diagnostics (file mode: one object; directory mode: array of failing files)",
        outputs: &["md", "schema"],
        render: validate_json_render,
    },
    FormatInfo {
        name: "info-json",
        summary: "rfs info --json introspection document (per-module static view; transitions is a count, monitor_status per property)",
        outputs: &["md", "schema"],
        render: info_json_render,
    },
    FormatInfo {
        name: "diagram-dot",
        summary: "the Graphviz DOT subset rfs diagram --format dot generates (header, node/edge line grammar, escaping)",
        outputs: &["md"],
        render: diagram_dot_render,
    },
    FormatInfo {
        name: "diagram-puml",
        summary: "the PlantUML subset rfs diagram --format puml generates (frame lines, event \\n guard labels, note on link, dashed on_error)",
        outputs: &["md"],
        render: diagram_puml_render,
    },
    FormatInfo {
        name: "diagram-mmd",
        summary: "the Mermaid stateDiagram-v2 subset rfs diagram --format mmd generates (multi-line edge labels carry event/guard/emits; [on_error] prefix)",
        outputs: &["md"],
        render: diagram_mmd_render,
    },
];

/// Look up a format entry by name.
pub fn find(name: &str) -> Option<&'static FormatInfo> {
    FORMATS.iter().find(|f| f.name == name)
}

/// Markdown table for `rfs info format --list`.
pub fn list_markdown() -> String {
    let mut out = String::from("| Format | Outputs (default first) | Description |\n");
    out.push_str("|--------|-------------------------|-------------|\n");
    for f in FORMATS {
        out.push_str(&format!(
            "| `{}` | {} | {} |\n",
            f.name,
            f.outputs.join(", "),
            f.summary
        ));
    }
    out
}

/// Render one format document. `output` must be one of the entry's supported
/// kinds (already validated by the caller); returns the document text.
pub fn render(name: &str, output: &str) -> Result<String, String> {
    let entry = find(name).ok_or_else(|| {
        format!("unknown format '{name}'; run `rfs info format --list` for registered formats")
    })?;
    if !entry.outputs.contains(&output) {
        return Err(format!(
            "format '{}' does not support output '{output}' (supported: {})",
            name,
            entry.outputs.join(", ")
        ));
    }
    (entry.render)(output).map_err(|errs| errs.join("; "))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_names_are_unique() {
        let names: Vec<&str> = FORMATS.iter().map(|f| f.name).collect();
        let mut sorted = names.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(names.len(), sorted.len(), "duplicate format names");
    }

    #[test]
    fn every_entry_renders_its_default_output() {
        for f in FORMATS {
            let default = f.outputs[0];
            let doc = (f.render)(default).unwrap_or_else(|e| panic!("{}: {e:?}", f.name));
            assert!(!doc.trim().is_empty(), "{}: empty document", f.name);
        }
    }

    #[test]
    fn unknown_format_errors() {
        let err = render("nope", "md").unwrap_err();
        assert!(err.contains("unknown format 'nope'"), "{err}");
        assert!(err.contains("--list"), "{err}");
    }

    #[test]
    fn unsupported_output_errors() {
        let err = render("spec-rfs", "schema").unwrap_err();
        assert!(err.contains("does not support output 'schema'"), "{err}");
    }
}
