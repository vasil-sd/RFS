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
