//! `rfs info format` suite (Phase 7 task 5, D-P7-6/7/9): the format registry,
//! the CLI dispatch, and the spec-rfs EBNF document in all three outputs
//! (md / plantuml / svg). The sync guarantee (D-P7-9) is pinned here:
//! - every `parser::KEYWORDS` entry appears as a terminal in the grammar model;
//! - the DESIGN.md section 6 ebnf fence parses back to exactly the model's rule
//!   names (the fence is the canonical text `rfs info format spec-rfs` prints);
//! - no non-terminal referenced by the model is left undefined.
//!
//! The checks are library-driven where possible (they stay green as later Phase 7
//! tasks register more formats) plus byte-level pins for the current registry.
//!
//! Task 7 adds the ce-trace entry: schema/md document pins and the D-P7-9 sync
//! walk of REAL counterexample documents - pinned FAIL-variant benchmark runs
//! (shared from `tests/common/fail_variants.rs`) on BOTH backends plus the
//! in-process reference MC, covering all three `kind`s (trace / lasso /
//! deadlock-config).

mod common;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::{LazyLock, Mutex};

fn rfs_bin() -> &'static str {
    env!("CARGO_BIN_EXE_rfs")
}

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn run_info_format(args: &[&str]) -> Output {
    Command::new(rfs_bin())
        .args(["info", "format"])
        .args(args)
        .output()
        .expect("spawn rfs")
}

fn stdout_of(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn stderr_of(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

// ---------------------------------------------------------------------------
// --list
// ---------------------------------------------------------------------------

#[test]
fn list_pins_all_registered_formats() {
    let out = run_info_format(&["--list"]);
    assert!(out.status.success(), "stderr: {}", stderr_of(&out));
    let text = stdout_of(&out);

    // Markdown table header.
    assert!(
        text.starts_with("| Format | Outputs (default first) | Description |\n"),
        "{text}"
    );
    assert!(
        text.contains("|--------|-------------------------|-------------|\n"),
        "{text}"
    );

    // Every registered format appears as a row, and no other names do.
    let mut expected = Vec::new();
    for f in rfs::info_format::FORMATS {
        let row_prefix = format!("| `{}` | ", f.name);
        assert!(
            text.contains(&row_prefix),
            "missing row for {}: {text}",
            f.name
        );
        expected.push(f.name.to_string());
    }
    // Row count = header + separator + one per format (println! adds a final
    // newline; trim before counting).
    let rows = text.trim_end().lines().count();
    assert_eq!(
        rows,
        2 + rfs::info_format::FORMATS.len(),
        "unexpected row count: {text}"
    );

    // Byte pin of the current registry (tasks 5-13 ship spec-rfs + spec-json
    // + ce-trace + mc-report + validate-json + info-json + diagram-dot
    // + diagram-puml + diagram-mmd).
    assert_eq!(
        expected,
        vec![
            "spec-rfs".to_string(),
            "spec-json".to_string(),
            "ce-trace".to_string(),
            "mc-report".to_string(),
            "validate-json".to_string(),
            "info-json".to_string(),
            "diagram-dot".to_string(),
            "diagram-puml".to_string(),
            "diagram-mmd".to_string(),
            "artifact-header".to_string()
        ]
    );
    assert!(
        text.contains(
            "| `spec-rfs` | md, plantuml, svg | RFS v0 input language - EBNF grammar (what rfs parse accepts) |"
        ),
        "{text}"
    );
    assert!(
        text.contains(
            "| `spec-json` | md, schema | IR v1 JSON document (rfs parse --output json; the machine-readable spec) |"
        ),
        "{text}"
    );
    assert!(
        text.contains(
            "| `ce-trace` | md, schema | MC counterexample trace (the report's `counterexample` object + the text trace format) |"
        ),
        "{text}"
    );
    assert!(
        text.contains(
            "| `mc-report` | md, schema | Full exact-mode MC report (--counterexample-format json; status, metrics, statistics + counterexample) |"
        ),
        "{text}"
    );
    assert!(
        text.contains(
            "| `validate-json` | md, schema | rfs validate --json diagnostics (file mode: one object; directory mode: array of failing files) |"
        ),
        "{text}"
    );
    assert!(
        text.contains(
            "| `info-json` | md, schema | rfs info --json introspection document (per-module static view; transitions is a count, monitor_status per property) |"
        ),
        "{text}"
    );
    assert!(
        text.contains(
            "| `diagram-dot` | md | the Graphviz DOT subset rfs diagram --format dot generates (header, node/edge line grammar, escaping) |"
        ),
        "{text}"
    );
    assert!(
        text.contains(
            "| `diagram-puml` | md | the PlantUML subset rfs diagram --format puml generates (frame lines, event \\n guard labels, note on link, dashed on_error) |"
        ),
        "{text}"
    );
    assert!(
        text.contains(
            "| `diagram-mmd` | md | the Mermaid stateDiagram-v2 subset rfs diagram --format mmd generates (multi-line edge labels carry event/guard/emits; [on_error] prefix) |"
        ),
        "{text}"
    );
    assert!(
        text.contains(
            "| `artifact-header` | md | line 1 of every generated artifact - the v1.0 stamp (toolchain version + spec hash + module name) |"
        ),
        "{text}"
    );
}

// ---------------------------------------------------------------------------
// spec-rfs: md (default output)
// ---------------------------------------------------------------------------

#[test]
fn spec_rfs_md_pins_all_rules_and_keywords() {
    let out = run_info_format(&["spec-rfs"]);
    assert!(out.status.success(), "stderr: {}", stderr_of(&out));
    let text = stdout_of(&out);

    // EBNF code block present.
    let fence_start = text.find("```ebnf\n").expect("missing ebnf fence");
    let body_start = fence_start + "```ebnf\n".len();
    let fence_end = text[body_start..]
        .find("\n```")
        .map(|i| body_start + i)
        .expect("missing fence close");
    let ebnf = &text[body_start..fence_end];

    // Every model rule appears in the ebnf block as `name = ... ;` and in the
    // markdown rule table.
    for (name, _) in rfs::ebnf_grammar::GRAMMAR_RULES {
        assert!(
            ebnf.lines()
                .any(|l| l.trim_start().starts_with(&format!("{name} = "))),
            "rule {name} missing from ebnf block"
        );
        assert!(
            text.contains(&format!("| `{name}` | ")),
            "rule {name} missing from rule table"
        );
    }

    // KEYWORDS coverage (D-P7-9): every reserved word is a quoted terminal in
    // the grammar.
    for kw in rfs::parser::KEYWORDS {
        assert!(
            ebnf.contains(&format!("\"{kw}\"")),
            "keyword '{kw}' does not appear as a terminal in the grammar"
        );
    }

    // Byte-level guard: the current model has exactly 61 rules.
    assert_eq!(rfs::ebnf_grammar::GRAMMAR_RULES.len(), 61);
}

// ---------------------------------------------------------------------------
// spec-rfs: plantuml
// ---------------------------------------------------------------------------

#[test]
fn spec_rfs_plantuml_pins_structure() {
    let out = run_info_format(&["spec-rfs", "--output", "plantuml"]);
    assert!(out.status.success(), "stderr: {}", stderr_of(&out));
    let text = stdout_of(&out);

    // @startebnf / @endebnf wrapper (plantuml.com/en/ebnf).
    assert!(
        text.starts_with("@startebnf\n"),
        "{}...",
        &text[..text.len().min(40)]
    );
    assert!(
        text.trim_end().ends_with("@endebnf"),
        "...{}",
        text.trim_end().chars().rev().take(40).collect::<String>()
    );

    // Every rule line: `name = body ;`.
    for (name, _) in rfs::ebnf_grammar::GRAMMAR_RULES {
        assert!(
            text.lines()
                .any(|l| l.trim_start().starts_with(&format!("{name} = "))
                    && l.trim_end().ends_with(';')),
            "rule {name} missing or unterminated in plantuml output"
        );
    }

    // Terminals are quoted (plantuml EBNF requires quotes on literals).
    for kw in rfs::parser::KEYWORDS {
        assert!(
            text.contains(&format!("\"{kw}\"")),
            "keyword '{kw}' not quoted"
        );
    }
}

// ---------------------------------------------------------------------------
// spec-rfs: svg (railroad)
// ---------------------------------------------------------------------------

#[test]
fn spec_rfs_svg_pins_labels() {
    let out = run_info_format(&["spec-rfs", "--output", "svg"]);
    assert!(out.status.success(), "stderr: {}", stderr_of(&out));
    let text = stdout_of(&out);

    // Standalone SVG document.
    assert!(
        text.starts_with("<svg "),
        "{}...",
        &text[..text.len().min(40)]
    );
    assert!(text.contains("</svg>"), "missing </svg>");

    // Every rule name is drawn as a text element (LabeledBox comment label or
    // non-terminal reference - both end `\n{name}</text>`).
    for (name, _) in rfs::ebnf_grammar::GRAMMAR_RULES {
        assert!(
            text.contains(&format!("\n{name}</text>")),
            "rule {name} not drawn in the SVG"
        );
    }

    // One comment label per rule (LabeledBox).
    let label_count = text.matches("class=\"comment\"").count();
    assert!(
        label_count >= rfs::ebnf_grammar::GRAMMAR_RULES.len(),
        "only {label_count} comment labels for {} rules",
        rfs::ebnf_grammar::GRAMMAR_RULES.len()
    );
}

// ---------------------------------------------------------------------------
// DESIGN.md section 6 sync (D-P7-9)
// ---------------------------------------------------------------------------

/// Extract the rule names from the ```ebnf fence in DESIGN.md section 6.
fn design_md_fence_rule_names() -> Vec<String> {
    let design = std::fs::read_to_string(root().join("DESIGN.md")).expect("read DESIGN.md");
    let section = design
        .split("## 6. ")
        .nth(1)
        .expect("DESIGN.md section 6 missing");
    let fence_start = section
        .find("```ebnf\n")
        .expect("section 6 ebnf fence missing");
    let after = &section[fence_start + "```ebnf\n".len()..];
    let fence_end = after.find("\n```").expect("section 6 fence not closed");
    let mut names = Vec::new();
    for line in after[..fence_end].lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('/') {
            continue;
        }
        // Rule lines are `name = body ;` - the name is a bare identifier.
        if let Some(eq) = line.find('=') {
            let name = line[..eq].trim();
            let ok = !name.is_empty()
                && name
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c == '_' || c.is_ascii_digit());
            assert!(ok, "unparseable rule line in DESIGN.md section 6: {line}");
            names.push(name.to_string());
        } else {
            panic!("line without '=' inside the ebnf fence: {line}");
        }
    }
    names
}

#[test]
fn design_md_section6_rule_names_match_model() {
    let fence_names = design_md_fence_rule_names();
    let model_names: Vec<&str> = rfs::ebnf_grammar::GRAMMAR_RULES
        .iter()
        .map(|(n, _)| *n)
        .collect();

    // Set equality in both directions (drift either way fails).
    for n in &fence_names {
        assert!(
            model_names.contains(&n.as_str()),
            "DESIGN.md fence rule '{n}' is not in the model"
        );
    }
    for n in &model_names {
        assert!(
            fence_names.contains(&n.to_string()),
            "model rule '{n}' is missing from the DESIGN.md fence"
        );
    }
    // No duplicates on either side.
    let mut sorted = fence_names.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(
        sorted.len(),
        fence_names.len(),
        "duplicate rule in DESIGN.md fence"
    );
}

// ---------------------------------------------------------------------------
// Library-level model checks
// ---------------------------------------------------------------------------

fn collect_nonterminals(node: &rfs::ebnf_grammar::RuleNode, out: &mut Vec<&str>) {
    match node {
        rfs::ebnf_grammar::RuleNode::NonTerminal(n) => out.push(n),
        rfs::ebnf_grammar::RuleNode::Sequence(children) => {
            for c in children.iter() {
                collect_nonterminals(c, out);
            }
        }
        rfs::ebnf_grammar::RuleNode::Choice(alternatives) => {
            for a in alternatives.iter() {
                collect_nonterminals(a, out);
            }
        }
        rfs::ebnf_grammar::RuleNode::Optional(inner)
        | rfs::ebnf_grammar::RuleNode::ZeroOrMore(inner)
        | rfs::ebnf_grammar::RuleNode::OneOrMore(inner) => collect_nonterminals(inner, out),
        rfs::ebnf_grammar::RuleNode::Terminal(_) => {}
    }
}

#[test]
fn no_dangling_nonterminals() {
    let defined: Vec<&str> = rfs::ebnf_grammar::GRAMMAR_RULES
        .iter()
        .map(|(n, _)| *n)
        .collect();
    for (name, node) in rfs::ebnf_grammar::GRAMMAR_RULES {
        let mut refs = Vec::new();
        collect_nonterminals(node, &mut refs);
        for r in refs {
            assert!(
                defined.contains(&r),
                "rule '{name}' references undefined rule '{r}'"
            );
        }
    }
}

#[test]
fn keywords_appear_as_terminals_in_model() {
    fn collect_terminals(node: &rfs::ebnf_grammar::RuleNode, out: &mut Vec<&str>) {
        match node {
            rfs::ebnf_grammar::RuleNode::Terminal(t) => out.push(t),
            rfs::ebnf_grammar::RuleNode::Sequence(children) => {
                for c in children.iter() {
                    collect_terminals(c, out);
                }
            }
            rfs::ebnf_grammar::RuleNode::Choice(alternatives) => {
                for a in alternatives.iter() {
                    collect_terminals(a, out);
                }
            }
            rfs::ebnf_grammar::RuleNode::Optional(inner)
            | rfs::ebnf_grammar::RuleNode::ZeroOrMore(inner)
            | rfs::ebnf_grammar::RuleNode::OneOrMore(inner) => collect_terminals(inner, out),
            rfs::ebnf_grammar::RuleNode::NonTerminal(_) => {}
        }
    }
    let mut terminals = Vec::new();
    for (_, node) in rfs::ebnf_grammar::GRAMMAR_RULES {
        collect_terminals(node, &mut terminals);
    }
    for kw in rfs::parser::KEYWORDS {
        assert!(
            terminals.contains(kw),
            "keyword '{kw}' is not a terminal in the model"
        );
    }
}

// ---------------------------------------------------------------------------
// spec-json: schema output (Phase 7 task 6, D-P7-8)
// ---------------------------------------------------------------------------

#[test]
fn spec_json_schema_document_pin() {
    let out = run_info_format(&["spec-json", "--output", "schema"]);
    assert!(out.status.success(), "stderr: {}", stderr_of(&out));
    let text = stdout_of(&out);

    // Pretty JSON through json_rw (project rule) - must round-trip.
    let doc = rfs::json_rw::parse(&text).expect("schema output must be valid JSON");
    assert_eq!(
        doc.get("$schema").and_then(|v| v.as_str()),
        Some("http://json-schema.org/draft-07/schema#"),
        "draft-07 (widest CI-validator support, D-P7-8)"
    );
    assert_eq!(
        doc.get("title").and_then(|v| v.as_str()),
        Some("IR v1 JSON document (spec-json)")
    );

    // Root: plain object with the two top-level keys, strict.
    assert_eq!(doc.get("type").and_then(|v| v.as_str()), Some("object"));
    let required = doc
        .get("required")
        .and_then(|v| v.as_arr())
        .cloned()
        .unwrap();
    let req_names: Vec<String> = required
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    assert_eq!(req_names, vec!["ir_version", "modules"]);
    assert_eq!(
        doc.get("additionalProperties").and_then(|v| v.as_bool()),
        Some(false)
    );

    // All 21 named definitions present.
    let defs = doc.get("definitions").cloned().unwrap();
    let def_names: Vec<String> = rfs::info_format::spec_json_schema()
        .definitions
        .iter()
        .map(|(n, _)| n.clone())
        .collect();
    assert_eq!(def_names.len(), 21);
    for n in &def_names {
        assert!(defs.get(n).is_some(), "missing definition {n}");
    }

    // Recursion: expr and formula $ref themselves (through their variants).
    let expr = defs.get("expr").cloned().unwrap();
    let expr_text = expr.to_string_compact();
    assert!(
        expr_text.contains("#/definitions/expr"),
        "expr must be recursive: {expr_text}"
    );
    let formula = defs.get("formula").cloned().unwrap();
    assert!(
        formula
            .to_string_compact()
            .contains("#/definitions/formula"),
        "formula must be recursive"
    );

    // Enum constraints: bin_op has all 16 operator names.
    let bin_op = defs.get("bin_op").cloned().unwrap();
    let vals = bin_op
        .get("enum")
        .and_then(|v| v.as_arr())
        .cloned()
        .unwrap();
    assert_eq!(vals.len(), 16);
    assert!(vals.iter().any(|v| v.as_str() == Some("logic_and")));

    // Nullable ref (guard) uses anyOf in draft-07 ($ref cannot take siblings).
    let transition = defs.get("transition").cloned().unwrap();
    let guard_schema = transition
        .get("properties")
        .cloned()
        .unwrap()
        .get("guard")
        .cloned()
        .unwrap();
    assert!(
        guard_schema.get("anyOf").is_some(),
        "guard must be anyOf: {guard_schema:?}"
    );

    // Optional keys (version/on_error) are NOT in module's required list.
    let module = defs.get("module").cloned().unwrap();
    let mod_req = module
        .get("required")
        .and_then(|v| v.as_arr())
        .cloned()
        .unwrap();
    let mod_req_names: Vec<&str> = mod_req.iter().filter_map(|v| v.as_str()).collect();
    assert!(
        !mod_req_names.contains(&"version"),
        "version must be optional"
    );
    assert!(
        !mod_req_names.contains(&"on_error"),
        "on_error must be optional"
    );
}

#[test]
fn spec_json_md_pins_all_sections() {
    let out = run_info_format(&["spec-json"]);
    assert!(out.status.success(), "stderr: {}", stderr_of(&out));
    let text = stdout_of(&out);

    assert!(
        text.starts_with("# IR v1 JSON document (spec-json)\n"),
        "{}...",
        &text[..text.len().min(60)]
    );
    assert!(text.contains("## document"), "missing document section");
    assert!(
        text.contains("| `ir_version` | integer | yes |"),
        "missing ir_version row"
    );

    // One section per named definition.
    for (name, _) in rfs::info_format::spec_json_schema().definitions {
        assert!(
            text.contains(&format!("\n## {name}\n")),
            "missing section for {name}"
        );
    }
}

// ---------------------------------------------------------------------------
// spec-json: D-P7-9 sync walk over every valid example's parse JSON
// ---------------------------------------------------------------------------

#[test]
fn spec_json_walk_all_valid_examples() {
    let doc = rfs::info_format::spec_json_schema();
    let files = rfs::collect_rfs_files(root().join("examples/valid"))
        .expect("valid examples directory should exist");
    assert!(!files.is_empty(), "no valid examples found");

    for path in &files {
        let ir = rfs::parse_file(path)
            .unwrap_or_else(|e| panic!("{}: valid example failed to parse: {e}", path.display()));
        let json = ir
            .to_json()
            .unwrap_or_else(|e| panic!("{}: to_json failed: {e}", path.display()));
        // (a) every required key present, (b) no undocumented key at any
        // object level, (c) types + enum constraints agree.
        if let Err(errs) = rfs::json_schema::walk_check(&doc, &json) {
            panic!(
                "{}: spec-json model out of sync with ir.rs to_json:\n{}",
                path.display(),
                errs.join("\n")
            );
        }
    }
}

// ---------------------------------------------------------------------------
// ce-trace: schema output (Phase 7 task 7, D-P7-8)
// ---------------------------------------------------------------------------

#[test]
fn ce_trace_schema_document_pin() {
    let out = run_info_format(&["ce-trace", "--output", "schema"]);
    assert!(out.status.success(), "stderr: {}", stderr_of(&out));
    let text = stdout_of(&out);

    // Pretty JSON through json_rw (project rule) - must round-trip.
    let doc = rfs::json_rw::parse(&text).expect("schema output must be valid JSON");
    assert_eq!(
        doc.get("$schema").and_then(|v| v.as_str()),
        Some("http://json-schema.org/draft-07/schema#"),
        "draft-07 (widest CI-validator support, D-P7-8)"
    );
    assert_eq!(
        doc.get("title").and_then(|v| v.as_str()),
        Some("Counterexample trace document (ce-trace)")
    );

    // Root: tagged union -> allOf[oneOf] with the three kinds, in wire order.
    let allof = doc.get("allOf").and_then(|v| v.as_arr()).cloned().unwrap();
    assert_eq!(allof.len(), 1);
    let oneof = allof[0]
        .get("oneOf")
        .and_then(|v| v.as_arr())
        .cloned()
        .unwrap();
    assert_eq!(oneof.len(), 3);
    let kinds: Vec<&str> = oneof
        .iter()
        .map(|alt| {
            alt.get("properties")
                .as_ref()
                .unwrap()
                .get("kind")
                .as_ref()
                .unwrap()
                .get("enum")
                .as_ref()
                .unwrap()
                .as_arr()
                .as_ref()
                .unwrap()[0]
                .as_str()
                .unwrap()
        })
        .collect();
    assert_eq!(
        kinds,
        vec![
            rfs::codegen_mc::JSON_CE_KIND_TRACE,
            rfs::codegen_mc::JSON_CE_KIND_LASSO,
            rfs::codegen_mc::JSON_CE_KIND_DEADLOCK_CONFIG
        ]
    );

    // ce_steps is required in EVERY variant and is the last key (wire order).
    for alt in &oneof {
        let req = alt
            .get("required")
            .and_then(|v| v.as_arr())
            .cloned()
            .unwrap();
        let names: Vec<&str> = req.iter().filter_map(|v| v.as_str()).collect();
        assert!(
            names.contains(&rfs::codegen_mc::JSON_KEY_CE_STEPS),
            "ce_steps missing from a variant's required list: {names:?}"
        );
        assert_eq!(
            *names.last().unwrap(),
            rfs::codegen_mc::JSON_KEY_CE_STEPS,
            "ce_steps must be the last key: {names:?}"
        );
    }

    // All four named definitions present.
    let defs = doc.get("definitions").cloned().unwrap();
    for n in ["ce_step", "ce_io", "ce_fields", "ce_value"] {
        assert!(defs.get(n).is_some(), "missing definition {n}");
    }

    // ce_fields: free-keyed object - additionalProperties is the $ref to
    // ce_value, and there is NO fixed properties list (keys are spec-defined
    // variable/field names).
    let fields = defs.get("ce_fields").cloned().unwrap();
    assert_eq!(fields.get("type").and_then(|v| v.as_str()), Some("object"));
    assert!(fields.get("properties").is_none(), "{fields:?}");
    let ap = fields.get("additionalProperties").cloned().unwrap();
    assert_eq!(
        ap.get("$ref").and_then(|v| v.as_str()),
        Some("#/definitions/ce_value")
    );

    // ce_value: anyOf boolean | integer | string - the bool / bit<N> / enum
    // value encoding of the name->value maps.
    let value = defs.get("ce_value").cloned().unwrap();
    let alts = value
        .get("anyOf")
        .and_then(|v| v.as_arr())
        .cloned()
        .unwrap();
    let types: Vec<&str> = alts
        .iter()
        .map(|a| a.get("type").and_then(|v| v.as_str()).unwrap())
        .collect();
    assert_eq!(types, vec!["boolean", "integer", "string"]);

    // ce_step: the five step keys in wire order.
    let step = defs.get("ce_step").cloned().unwrap();
    let props = step.get("properties").cloned().unwrap();
    let prop_names: Vec<String> = props
        .as_obj()
        .cloned()
        .unwrap()
        .iter()
        .map(|(k, _)| k.clone())
        .collect();
    assert_eq!(
        prop_names,
        vec!["input", "outputs", "from_state", "to_state", "vars"]
    );
}

// ---------------------------------------------------------------------------
// ce-trace: md output (Phase 7 task 7)
// ---------------------------------------------------------------------------

#[test]
fn ce_trace_md_pins_sections() {
    let out = run_info_format(&["ce-trace"]);
    assert!(out.status.success(), "stderr: {}", stderr_of(&out));
    let text = stdout_of(&out);

    assert!(
        text.starts_with("# Counterexample trace document (ce-trace)\n"),
        "{}...",
        &text[..text.len().min(60)]
    );

    // The document section carries the kind table with all three kinds.
    assert!(text.contains("## document"), "missing document section");
    for k in [
        rfs::codegen_mc::JSON_CE_KIND_TRACE,
        rfs::codegen_mc::JSON_CE_KIND_LASSO,
        rfs::codegen_mc::JSON_CE_KIND_DEADLOCK_CONFIG,
    ] {
        assert!(
            text.contains(&format!("| `{k}` | ")),
            "missing kind row {k}"
        );
    }

    // One section per named definition.
    for (name, _) in rfs::info_format::ce_trace_schema().definitions {
        assert!(
            text.contains(&format!("\n## {name}\n")),
            "missing section for {name}"
        );
    }

    // The text trace format section documents the step-line grammar as
    // printed by both backends (D-P6-10 unified report).
    assert!(
        text.contains("## text trace format (default report)"),
        "missing text trace format section"
    );
    assert!(text.contains("counterexample trace:"), "{text}");
    assert!(
        text.contains("initial state=State::X vars=Vars { v1: val, ... }"),
        "{text}"
    );
    assert!(
        text.contains(
            "step 0 input=Input::V { f: val } outputs=[Output::O { f: val }] from=State::A to=State::B vars=Vars { ... }"
        ),
        "{text}"
    );
    assert!(
        text.contains(
            "model-check summary: status=fail states=N transitions=M max_depth=D reason=<r> label=<L> step=S"
        ),
        "{text}"
    );
}

// ---------------------------------------------------------------------------
// ce-trace: D-P7-9 sync walk over REAL counterexample documents (Phase 7
// task 7) - pinned FAIL-variant benchmark runs on both backends + the
// in-process reference MC, covering all three `kind`s.
// ---------------------------------------------------------------------------

static CE_BIN_CACHE: LazyLock<Mutex<HashMap<String, PathBuf>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn rustc_path() -> String {
    std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_string())
}

/// Codegen + compile one exact-mode MC binary for `src` (cached per key so
/// the suite's tests share binaries, like benchmark_mc_test).
fn ce_mc_binary(key: &str, backend: &str, src: &str) -> PathBuf {
    let mut cache = CE_BIN_CACHE.lock().unwrap();
    if let Some(p) = cache.get(key) {
        return p.clone();
    }
    let dir = root().join("target").join("rfs_info_format");
    std::fs::create_dir_all(&dir).expect("create scratch dir");

    let ir = rfs::parse_source(src).unwrap_or_else(|e| panic!("{key} must parse ({backend}): {e}"));
    assert_eq!(ir.modules.len(), 1);
    let be = rfs::codegen::get_backend(backend)
        .unwrap_or_else(|| panic!("backend {backend} registered"));
    let code = rfs::codegen::CodegenCommandKind::ModelChecker
        .run(be, &ir, &rfs::codegen::BackendOptions::default())
        .unwrap_or_else(|e| panic!("codegen {backend} for {key}: {e:?}"))
        .code;

    // The file name doubles as the rustc crate name: sanitize the cache key.
    let stem: String = key
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let ext = if backend == "c" { "c" } else { "rs" };
    let src_path = dir.join(format!("{stem}_mc.{ext}"));
    std::fs::write(&src_path, code).expect("write MC source");
    let bin = dir.join(format!("{stem}_mc_{backend}"));
    let out = if backend == "c" {
        Command::new("/usr/bin/gcc")
            .arg("-Wall")
            .arg(&src_path)
            .arg("-o")
            .arg(&bin)
            .output()
    } else {
        Command::new(rustc_path())
            .args(["--edition", "2021", "-O"])
            .arg(&src_path)
            .arg("-o")
            .arg(&bin)
            .output()
    };
    let out = out.unwrap_or_else(|e| panic!("run compiler for {key}: {e}"));
    assert!(
        out.status.success(),
        "{backend} compile failed for {key}:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    cache.insert(key.to_string(), bin.clone());
    bin
}

/// One `--counterexample-format json` run: returns the full report document.
/// The exit code must match the reported status (0 = pass-complete,
/// 1 = fail, 2 = unknown-state-limit).
fn mc_report_of(bin: &Path, key: &str, extra_args: &[&str]) -> rfs::json_rw::Json {
    let mut args: Vec<&str> = extra_args.to_vec();
    args.push("--counterexample-format");
    args.push("json");
    let out = Command::new(bin)
        .args(&args)
        .output()
        .unwrap_or_else(|e| panic!("run MC binary for {key}: {e}"));
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let doc = rfs::json_rw::parse(&stdout)
        .unwrap_or_else(|e| panic!("{key}: JSON report must parse: {e}\n{stdout}"));
    let want_code = match doc.get("status").and_then(rfs::json_rw::Json::as_str) {
        Some("pass-complete") => 0,
        Some("fail") => 1,
        Some("unknown-state-limit") => 2,
        other => panic!("{key}: unexpected status {other:?} in: {stdout}"),
    };
    assert_eq!(
        out.status.code(),
        Some(want_code),
        "{key}: exit code must match the reported status; {stdout}"
    );
    doc
}

/// One `--counterexample-format json` run of a FAILING binary: returns the
/// report's `counterexample` object (present iff status is fail).
fn ce_object(bin: &Path, key: &str) -> rfs::json_rw::Json {
    let doc = mc_report_of(bin, key, &[]);
    assert_eq!(
        doc.get("status").and_then(rfs::json_rw::Json::as_str),
        Some("fail"),
        "{key}"
    );
    doc.get("counterexample")
        .cloned()
        .unwrap_or_else(|| panic!("{key}: fail carries a counterexample"))
}

/// The D-P7-9 walk against the ce-trace model (required keys present, no
/// undocumented key at any object level, types + enum constraints agree).
fn walk_ce(ce: &rfs::json_rw::Json, what: &str) {
    let doc = rfs::info_format::ce_trace_schema();
    if let Err(errs) = rfs::json_schema::walk_check(&doc, ce) {
        panic!(
            "{what}: ce-trace model out of sync with the generated report:\n{}",
            errs.join("\n")
        );
    }
}

#[test]
fn ce_trace_walk_trace_and_lasso_both_backends() {
    // The pinned FAIL variants shared with benchmark_mc_test: ring_no_grant
    // = property violation (trace), elev_noup = liveness violation (lasso).
    let cases: Vec<(&str, &str, &str)> = vec![
        (
            "ring_no_grant",
            common::fail_variants::RING_NO_GRANT_SRC,
            rfs::codegen_mc::JSON_CE_KIND_TRACE,
        ),
        (
            "elev_noup",
            common::fail_variants::ELEV_NO_UP_SRC,
            rfs::codegen_mc::JSON_CE_KIND_LASSO,
        ),
    ];
    for (key, src, want_kind) in cases {
        let mut docs: [Option<rfs::json_rw::Json>; 2] = [None, None];
        for (i, backend) in ["rust", "c"].iter().enumerate() {
            let bin = ce_mc_binary(&format!("ce|{key}|{backend}"), backend, src);
            let ce = ce_object(&bin, key);
            assert_eq!(
                ce.get("kind").and_then(rfs::json_rw::Json::as_str),
                Some(want_kind),
                "{key}/{backend}"
            );
            walk_ce(&ce, &format!("{key}/{backend}"));
            docs[i] = Some(ce);
        }
        // The counterexample object is byte-identical across backends (the
        // parity contract; peak_rss_kb lives in statistics, not here).
        assert_eq!(
            docs[0].as_ref().unwrap().to_string_compact(),
            docs[1].as_ref().unwrap().to_string_compact(),
            "{key}: counterexample must be identical across backends"
        );
    }

    // Pins (deterministic exact mode; the same values benchmark_mc_test pins
    // via ce_steps / cycle_length): ring_no_grant is a one-step trace,
    // elev_noup a lasso with cycle length 2.
    let rt = ce_object(
        &ce_mc_binary(
            "ce|ring_no_grant|rust",
            "rust",
            common::fail_variants::RING_NO_GRANT_SRC,
        ),
        "ring_no_grant",
    );
    assert_eq!(
        rt.get(rfs::codegen_mc::JSON_KEY_CE_STEPS)
            .and_then(rfs::json_rw::Json::as_int),
        Some(1)
    );
    let steps = rt
        .get(rfs::codegen_mc::JSON_KEY_STEPS)
        .and_then(rfs::json_rw::Json::as_arr)
        .cloned()
        .unwrap();
    assert_eq!(steps.len(), 1);

    let rl = ce_object(
        &ce_mc_binary(
            "ce|elev_noup|rust",
            "rust",
            common::fail_variants::ELEV_NO_UP_SRC,
        ),
        "elev_noup",
    );
    assert_eq!(
        rl.get(rfs::codegen_mc::JSON_KEY_CYCLE_LENGTH)
            .and_then(rfs::json_rw::Json::as_int),
        Some(2)
    );
    let cycle = rl
        .get(rfs::codegen_mc::JSON_KEY_CYCLE)
        .and_then(rfs::json_rw::Json::as_arr)
        .cloned()
        .unwrap();
    assert_eq!(cycle.len(), 2);
}

#[test]
fn ce_trace_walk_deadlock_config_both_backends() {
    // Example 04: a terminal state with no enabled transition -> deadlock at
    // depth 2 (the same case backend_parity_test pins for the five
    // status/reason combinations).
    let src = common::fail_variants::spec_source("04_tcp_handshake_like.rfs");
    let mut docs: [Option<rfs::json_rw::Json>; 2] = [None, None];
    for (i, backend) in ["rust", "c"].iter().enumerate() {
        let bin = ce_mc_binary(&format!("ce|ex04_deadlock|{backend}"), backend, &src);
        let ce = ce_object(&bin, "ex04_deadlock");
        assert_eq!(
            ce.get("kind").and_then(rfs::json_rw::Json::as_str),
            Some(rfs::codegen_mc::JSON_CE_KIND_DEADLOCK_CONFIG),
            "{backend}"
        );
        walk_ce(&ce, &format!("ex04_deadlock/{backend}"));
        docs[i] = Some(ce);
    }
    assert_eq!(
        docs[0].as_ref().unwrap().to_string_compact(),
        docs[1].as_ref().unwrap().to_string_compact(),
        "ex04_deadlock: counterexample must be identical across backends"
    );

    // Pin: depth 2 (deterministic; matches the text run's `deadlock:
    // configuration at depth 2`), ce_steps = depth for a deadlock config.
    let ce = docs[0].as_ref().unwrap();
    assert_eq!(
        ce.get(rfs::codegen_mc::JSON_KEY_DEPTH)
            .and_then(rfs::json_rw::Json::as_int),
        Some(2)
    );
    assert_eq!(
        ce.get(rfs::codegen_mc::JSON_KEY_CE_STEPS)
            .and_then(rfs::json_rw::Json::as_int),
        Some(2)
    );
}

#[test]
fn ce_trace_walk_ref_mc_in_process() {
    // The toolchain-side builder (ref_mc::to_json_report) is the third
    // producer of the document - it must agree with the model too.
    let cases: Vec<(&str, String, &str)> = vec![
        (
            "ring_no_grant",
            common::fail_variants::RING_NO_GRANT_SRC.to_string(),
            rfs::codegen_mc::JSON_CE_KIND_TRACE,
        ),
        (
            "elev_noup",
            common::fail_variants::ELEV_NO_UP_SRC.to_string(),
            rfs::codegen_mc::JSON_CE_KIND_LASSO,
        ),
        (
            "ex04_deadlock",
            common::fail_variants::spec_source("04_tcp_handshake_like.rfs"),
            rfs::codegen_mc::JSON_CE_KIND_DEADLOCK_CONFIG,
        ),
    ];
    for (key, src, want_kind) in cases {
        let ir = rfs::parse_source(&src).unwrap_or_else(|e| panic!("{key} must parse: {e}"));
        let module = &ir.modules[0];
        let res = rfs::ref_mc::run(module, &rfs::ref_mc::RefConfig::default())
            .unwrap_or_else(|e| panic!("{key}: ref_mc: {e}"));
        assert_eq!(res.status, rfs::codegen_mc::ExactStatus::Fail, "{key}");
        let report = rfs::ref_mc::to_json_report(&res, module).to_json();
        let ce = report
            .get("counterexample")
            .cloned()
            .unwrap_or_else(|| panic!("{key}: fail carries a counterexample"));
        assert_eq!(
            ce.get("kind").and_then(rfs::json_rw::Json::as_str),
            Some(want_kind),
            "{key}"
        );
        walk_ce(&ce, key);
    }
}

// ---------------------------------------------------------------------------
// mc-report: schema + md document pins (Phase 7 task 8)
// ---------------------------------------------------------------------------

fn prop_names(j: &rfs::json_rw::Json) -> Vec<String> {
    j.get("properties")
        .and_then(rfs::json_rw::Json::as_obj)
        .cloned()
        .unwrap()
        .into_iter()
        .map(|(k, _)| k)
        .collect()
}

fn enum_vals(j: &rfs::json_rw::Json) -> Vec<String> {
    j.get("enum")
        .and_then(rfs::json_rw::Json::as_arr)
        .cloned()
        .unwrap()
        .into_iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect()
}

/// The D-P7-9 walk against the mc-report model (required keys present, no
/// undocumented key at any object level, types + enum constraints agree).
fn walk_mc_report(rep: &rfs::json_rw::Json, what: &str) {
    let doc = rfs::info_format::mc_report_schema();
    if let Err(errs) = rfs::json_schema::walk_check(&doc, rep) {
        panic!(
            "{what}: mc-report model out of sync with the generated report:\n{}",
            errs.join("\n")
        );
    }
}

#[test]
fn mc_report_schema_document_pin() {
    let out = run_info_format(&["mc-report", "--output", "schema"]);
    assert!(out.status.success(), "stderr: {}", stderr_of(&out));
    let text = stdout_of(&out);

    let doc = rfs::json_rw::parse(&text).expect("schema output must be valid JSON");
    assert_eq!(
        doc.get("$schema").and_then(|v| v.as_str()),
        Some("http://json-schema.org/draft-07/schema#")
    );
    assert_eq!(
        doc.get("title").and_then(|v| v.as_str()),
        Some("MC report document (mc-report)")
    );

    // Root: plain object, strict; required = status/metrics/statistics in
    // wire order. The fail fields are optional (present IFF status is fail).
    assert_eq!(doc.get("type").and_then(|v| v.as_str()), Some("object"));
    let req = doc
        .get("required")
        .and_then(|v| v.as_arr())
        .cloned()
        .unwrap();
    let req_names: Vec<&str> = req.iter().filter_map(|v| v.as_str()).collect();
    assert_eq!(req_names, vec!["status", "metrics", "statistics"]);
    assert_eq!(
        doc.get("additionalProperties").and_then(|v| v.as_bool()),
        Some(false)
    );
    for k in ["reason", "label", "step", "counterexample"] {
        assert!(!req_names.contains(&k), "{k} must be optional");
    }

    // status enum = ExactStatus wire names; reason enum = FailReason wire
    // names (derived from the methods - no string literals in the model).
    let props = doc.get("properties").cloned().unwrap();
    let status_enum = props
        .as_obj()
        .cloned()
        .unwrap()
        .into_iter()
        .find(|(k, _)| k == "status")
        .unwrap()
        .1;
    assert_eq!(
        enum_vals(&status_enum),
        vec![
            rfs::codegen_mc::ExactStatus::PassComplete.wire(),
            rfs::codegen_mc::ExactStatus::Fail.wire(),
            rfs::codegen_mc::ExactStatus::UnknownStateLimit.wire()
        ]
    );
    let reason_enum = props
        .as_obj()
        .cloned()
        .unwrap()
        .into_iter()
        .find(|(k, _)| k == "reason")
        .unwrap()
        .1;
    assert_eq!(
        enum_vals(&reason_enum),
        vec![
            rfs::codegen_mc::FailReason::PropertyViolation.wire(),
            rfs::codegen_mc::FailReason::LivenessViolation.wire(),
            rfs::codegen_mc::FailReason::Deadlock.wire()
        ]
    );

    // All seven definitions present.
    let defs = doc.get("definitions").cloned().unwrap();
    for n in [
        "mc_metrics",
        "mc_statistics",
        "counterexample",
        "ce_step",
        "ce_io",
        "ce_fields",
        "ce_value",
    ] {
        assert!(defs.get(n).is_some(), "missing definition {n}");
    }

    // metrics / statistics keys in wire order.
    let metrics = defs.get("mc_metrics").cloned().unwrap();
    assert_eq!(
        prop_names(&metrics),
        vec!["states", "transitions", "max_depth"]
    );
    let stats = defs.get("mc_statistics").cloned().unwrap();
    assert_eq!(
        prop_names(&stats),
        vec![
            "unexpected_terminations",
            "monitor_size_bits",
            "memory_estimate_bytes",
            "peak_rss_kb"
        ]
    );

    // No-drift pin: the counterexample definition is byte-identical to the
    // ce-trace document's root (shared builder - task 7).
    let ce_def = defs.get("counterexample").cloned().unwrap();
    let ce_root_emitted = rfs::json_schema::emit_type(&rfs::info_format::ce_trace_schema().root);
    assert_eq!(
        ce_def.to_string_compact(),
        ce_root_emitted.to_string_compact(),
        "mc-report counterexample definition drifted from the ce-trace root"
    );
}

#[test]
fn mc_report_md_pins_sections() {
    let out = run_info_format(&["mc-report"]);
    assert!(out.status.success(), "stderr: {}", stderr_of(&out));
    let text = stdout_of(&out);

    assert!(
        text.starts_with("# MC report document (mc-report)\n"),
        "{}...",
        &text[..text.len().min(60)]
    );
    assert!(text.contains("## document"), "missing document section");
    assert!(
        text.contains(
            "| `status` | string (one of: pass-complete, fail, unknown-state-limit) | yes |"
        ),
        "missing status row"
    );

    // The fail fields are documented as optional (key omitted).
    for k in ["reason", "label", "step", "counterexample"] {
        assert!(text.contains(&format!("| `{k}` | ")), "missing row for {k}");
    }

    // One section per named definition (7: mc_metrics, mc_statistics,
    // counterexample + the four ce-* sub-schemas).
    let defs = rfs::info_format::mc_report_schema().definitions;
    assert_eq!(defs.len(), 7);
    for (name, _) in defs {
        assert!(
            text.contains(&format!("\n## {name}\n")),
            "missing section for {name}"
        );
    }

    // The counterexample section carries the kind table (all three kinds).
    for k in [
        rfs::codegen_mc::JSON_CE_KIND_TRACE,
        rfs::codegen_mc::JSON_CE_KIND_LASSO,
        rfs::codegen_mc::JSON_CE_KIND_DEADLOCK_CONFIG,
    ] {
        assert!(
            text.contains(&format!("| `{k}` | ")),
            "missing kind row {k}"
        );
    }
}

// ---------------------------------------------------------------------------
// mc-report: D-P7-9 sync walk over real benchmark runs (Phase 7 task 8)
// ---------------------------------------------------------------------------

// The 6-tuple case type is complex by design (key, source, runtime args,
// expected reason/label/step) - a struct would add more lines than it saves.
#[allow(clippy::type_complexity)]
#[test]
fn mc_report_walk_benchmarks_both_backends() {
    // Pass runs: all six benchmark specs -> status=pass-complete, no fail
    // fields, no counterexample.
    let bench_specs = [
        "43_sliding_window.rfs",
        "44_token_ring.rfs",
        "45_elevator.rfs",
        "47_http2_stream.rfs",
        "48_usb_control_transfer.rfs",
        "49_can_arbitration.rfs",
    ];
    for name in bench_specs {
        let src = common::fail_variants::spec_source(name);
        for backend in ["rust", "c"] {
            let bin = ce_mc_binary(&format!("mcrep|{name}|{backend}"), backend, &src);
            let rep = mc_report_of(&bin, name, &[]);
            assert_eq!(
                rep.get("status").and_then(rfs::json_rw::Json::as_str),
                Some("pass-complete"),
                "{name}/{backend}"
            );
            for k in ["reason", "label", "step", "counterexample"] {
                assert!(
                    rep.get(k).is_none(),
                    "{name}/{backend}: '{k}' on a pass-complete run"
                );
            }
            walk_mc_report(&rep, &format!("{name}/{backend}"));
        }
    }

    // Fail runs: the five pinned FAIL variants shared with benchmark_mc_test
    // (trace x4 + lasso) plus example 04 for the deadlock reason. The pins
    // are the same deterministic values benchmark_mc_test asserts.
    let cases: Vec<(&str, String, &[&str], &str, &str, i64)> = vec![
        (
            "ring_no_grant",
            common::fail_variants::RING_NO_GRANT_SRC.to_string(),
            &[],
            "property-violation",
            "holder_no_request",
            1,
        ),
        (
            "elev_noup",
            common::fail_variants::ELEV_NO_UP_SRC.to_string(),
            &[],
            "liveness-violation",
            "up2_served",
            0,
        ),
        (
            "http2_rst_fail",
            common::fail_variants::http2_rst_fail_src(),
            &[],
            "property-violation",
            "rst_goes_closed",
            2,
        ),
        (
            "48_usb_control_transfer",
            common::fail_variants::spec_source("48_usb_control_transfer.rfs"),
            &["--assumes", "none"],
            "property-violation",
            "completes_after_setup",
            16,
        ),
        (
            "can_passive_fail",
            common::fail_variants::can_passive_fail_src(),
            &[],
            "property-violation",
            "passive_band",
            6,
        ),
        (
            "ex04_deadlock",
            common::fail_variants::spec_source("04_tcp_handshake_like.rfs"),
            &[],
            "deadlock",
            "-",
            2,
        ),
    ];
    for (key, src, args, want_reason, want_label, want_step) in cases {
        for backend in ["rust", "c"] {
            let bin = ce_mc_binary(&format!("mcrep|{key}|{backend}"), backend, &src);
            let rep = mc_report_of(&bin, key, args);
            assert_eq!(
                rep.get("status").and_then(rfs::json_rw::Json::as_str),
                Some("fail"),
                "{key}/{backend}"
            );
            assert_eq!(
                rep.get("reason").and_then(rfs::json_rw::Json::as_str),
                Some(want_reason),
                "{key}/{backend}"
            );
            assert_eq!(
                rep.get("label").and_then(rfs::json_rw::Json::as_str),
                Some(want_label),
                "{key}/{backend}"
            );
            assert_eq!(
                rep.get("step").and_then(rfs::json_rw::Json::as_int),
                Some(want_step),
                "{key}/{backend}"
            );
            assert!(
                rep.get("counterexample").is_some(),
                "{key}/{backend}: fail carries a counterexample"
            );
            walk_mc_report(&rep, &format!("{key}/{backend}"));
        }
    }

    // unknown-state-limit: example 43 truncated to 1 state (the same case
    // backend_parity_test pins for the five status/reason combinations).
    let src = common::fail_variants::spec_source("43_sliding_window.rfs");
    for backend in ["rust", "c"] {
        let bin = ce_mc_binary(&format!("mcrep|ex43_unknown|{backend}"), backend, &src);
        let rep = mc_report_of(&bin, "ex43_unknown", &["--max-states", "1"]);
        assert_eq!(
            rep.get("status").and_then(rfs::json_rw::Json::as_str),
            Some("unknown-state-limit"),
            "{backend}"
        );
        for k in ["reason", "label", "step", "counterexample"] {
            assert!(
                rep.get(k).is_none(),
                "ex43_unknown/{backend}: '{k}' on an unknown run"
            );
        }
        walk_mc_report(&rep, &format!("ex43_unknown/{backend}"));
    }
}

#[test]
fn mc_report_walk_ref_mc_in_process() {
    // The toolchain-side builder (ref_mc::to_json_report) is the third
    // producer of the full report - it must agree with the model too. One
    // run per status: pass-complete, fail (trace), fail (deadlock), and
    // unknown-state-limit via the state cap.
    let pass_src = common::fail_variants::spec_source("43_sliding_window.rfs");
    let ir = rfs::parse_source(&pass_src).expect("43 must parse");
    let module = &ir.modules[0];

    let res = rfs::ref_mc::run(module, &rfs::ref_mc::RefConfig::default()).expect("ref_mc 43");
    assert_eq!(res.status, rfs::codegen_mc::ExactStatus::PassComplete);
    walk_mc_report(
        &rfs::ref_mc::to_json_report(&res, module).to_json(),
        "ref_mc/43_pass",
    );

    let ir = rfs::parse_source(common::fail_variants::RING_NO_GRANT_SRC)
        .expect("ring_no_grant must parse");
    let module = &ir.modules[0];
    let res =
        rfs::ref_mc::run(module, &rfs::ref_mc::RefConfig::default()).expect("ref_mc ring_no_grant");
    assert_eq!(res.status, rfs::codegen_mc::ExactStatus::Fail);
    walk_mc_report(
        &rfs::ref_mc::to_json_report(&res, module).to_json(),
        "ref_mc/ring_no_grant",
    );

    let dl_src = common::fail_variants::spec_source("04_tcp_handshake_like.rfs");
    let ir = rfs::parse_source(&dl_src).expect("04 must parse");
    let module = &ir.modules[0];
    let res = rfs::ref_mc::run(module, &rfs::ref_mc::RefConfig::default()).expect("ref_mc 04");
    assert_eq!(res.status, rfs::codegen_mc::ExactStatus::Fail);
    walk_mc_report(
        &rfs::ref_mc::to_json_report(&res, module).to_json(),
        "ref_mc/ex04_deadlock",
    );

    let ir = rfs::parse_source(&pass_src).expect("43 must parse");
    let module = &ir.modules[0];
    let cfg = rfs::ref_mc::RefConfig {
        max_states: 1,
        ..Default::default()
    };
    let res = rfs::ref_mc::run(module, &cfg).expect("ref_mc 43 capped");
    assert_eq!(res.status, rfs::codegen_mc::ExactStatus::UnknownStateLimit);
    walk_mc_report(
        &rfs::ref_mc::to_json_report(&res, module).to_json(),
        "ref_mc/43_unknown",
    );
}

// ---------------------------------------------------------------------------
// validate-json: schema output (Phase 7 task 9, D-P7-8)
// ---------------------------------------------------------------------------

#[test]
fn validate_json_schema_document_pin() {
    let out = run_info_format(&["validate-json", "--output", "schema"]);
    assert!(out.status.success(), "stderr: {}", stderr_of(&out));
    let text = stdout_of(&out);

    // Pretty JSON through json_rw (project rule) - must round-trip.
    let doc = rfs::json_rw::parse(&text).expect("schema output must be valid JSON");
    assert_eq!(
        doc.get("$schema").and_then(|v| v.as_str()),
        Some("http://json-schema.org/draft-07/schema#")
    );
    assert_eq!(
        doc.get("title").and_then(|v| v.as_str()),
        Some("Validate diagnostics document (validate-json)")
    );

    // Root: union of the two shapes - file mode object | directory mode
    // array (draft-07 anyOf; a non-object root is wrapped in allOf).
    let allof = doc.get("allOf").and_then(|v| v.as_arr()).cloned().unwrap();
    assert_eq!(allof.len(), 1);
    let anyof = allof[0]
        .get("anyOf")
        .and_then(|v| v.as_arr())
        .cloned()
        .unwrap();
    assert_eq!(anyof.len(), 2);
    assert_eq!(
        anyof[0].get("$ref").and_then(|v| v.as_str()),
        Some("#/definitions/validate_file")
    );
    let arr = &anyof[1];
    assert_eq!(arr.get("type").and_then(|v| v.as_str()), Some("array"));
    assert_eq!(
        arr.get("items")
            .cloned()
            .unwrap()
            .get("$ref")
            .and_then(|v| v.as_str()),
        Some("#/definitions/validate_file")
    );

    // Both definitions present, strict, keys in writer order.
    let defs = doc.get("definitions").cloned().unwrap();
    for n in ["validate_file", "validate_error"] {
        assert!(defs.get(n).is_some(), "missing definition {n}");
    }
    let file_def = defs.get("validate_file").cloned().unwrap();
    assert_eq!(prop_names(&file_def), vec!["file", "ok", "errors"]);
    assert_eq!(
        file_def
            .get("additionalProperties")
            .and_then(|v| v.as_bool()),
        Some(false)
    );
    let err_def = defs.get("validate_error").cloned().unwrap();
    assert_eq!(prop_names(&err_def), vec!["kind", "line", "col", "message"]);
    assert_eq!(
        err_def
            .get("additionalProperties")
            .and_then(|v| v.as_bool()),
        Some(false)
    );

    // kind enum = the two StructuredError kinds from frontend.rs.
    let kind_schema = err_def
        .get("properties")
        .cloned()
        .unwrap()
        .get("kind")
        .cloned()
        .unwrap();
    assert_eq!(
        enum_vals(&kind_schema),
        vec!["parse".to_string(), "semantic".to_string()]
    );

    // All four error keys are required (line/col are 0, never omitted).
    let err_req_arr = err_def
        .get("required")
        .and_then(|v| v.as_arr())
        .cloned()
        .unwrap();
    let err_req: Vec<&str> = err_req_arr.iter().filter_map(|v| v.as_str()).collect();
    assert_eq!(err_req, vec!["kind", "line", "col", "message"]);
}

#[test]
fn validate_json_md_pins_sections() {
    let out = run_info_format(&["validate-json"]);
    assert!(out.status.success(), "stderr: {}", stderr_of(&out));
    let text = stdout_of(&out);

    assert!(
        text.starts_with("# Validate diagnostics document (validate-json)\n"),
        "{}...",
        &text[..text.len().min(60)]
    );
    // Union root: both shapes named in the document section.
    assert!(
        text.contains("Type: [validate_file] | array of [validate_file]"),
        "{text}"
    );

    // One section per named definition.
    for (name, _) in rfs::info_format::validate_json_schema().definitions {
        assert!(
            text.contains(&format!("\n## {name}\n")),
            "missing section for {name}"
        );
    }

    // Every documented key appears as a table row.
    for k in ["file", "ok", "errors", "kind", "line", "col", "message"] {
        assert!(text.contains(&format!("| `{k}` | ")), "missing row for {k}");
    }

    // The modes section carries both example shapes.
    assert!(text.contains("## file mode vs directory mode"), "{text}");
    assert!(text.contains("\"ok\": true"), "{text}");
    assert!(text.contains("\"kind\": \"parse\""), "{text}");
}

// ---------------------------------------------------------------------------
// validate-json: D-P7-9 sync walk over real validation runs (Phase 7 task 9)
// ---------------------------------------------------------------------------

/// The D-P7-9 walk against the validate-json model (required keys present,
/// no undocumented key at any object level, types + enum constraints agree).
fn walk_validate_json(doc: &rfs::json_schema::SchemaDoc, json: &rfs::json_rw::Json, what: &str) {
    if let Err(errs) = rfs::json_schema::walk_check(doc, json) {
        panic!(
            "{what}: validate-json model out of sync with the rendered document:\n{}",
            errs.join("\n")
        );
    }
}

#[test]
fn validate_json_walk_invalid_examples_both_kinds() {
    let doc = rfs::info_format::validate_json_schema();
    let files = rfs::collect_rfs_files(root().join("examples/invalid"))
        .expect("invalid examples directory should exist");
    assert_eq!(files.len(), 13, "corpus changed: update the pin");

    let mut kinds: std::collections::HashSet<&str> = std::collections::HashSet::new();
    for path in &files {
        // File mode: one object for the input file.
        let failed = rfs::frontend::run_validate_structured(None, path)
            .unwrap_or_else(|e| panic!("{}: {e:?}", path.display()));
        assert!(
            !failed.is_empty(),
            "{} must fail validation",
            path.display()
        );
        let text = rfs::frontend::render_validate_json(path, false, &failed);
        let json = rfs::json_rw::parse(&text).unwrap_or_else(|e| {
            panic!(
                "{}: rendered document is not valid JSON: {e}",
                path.display()
            )
        });
        walk_validate_json(&doc, &json, &format!("file mode {}", path.display()));

        // ok=false with at least one error of a documented kind.
        assert_eq!(json.get("ok").and_then(|v| v.as_bool()), Some(false));
        for (p, errs) in &failed {
            assert_eq!(p, path);
            for e in errs {
                kinds.insert(e.kind);
            }
        }
    }
    // Both error kinds must be observed across the corpus.
    assert!(
        kinds.contains("parse"),
        "no parse errors observed: {kinds:?}"
    );
    assert!(
        kinds.contains("semantic"),
        "no semantic errors observed: {kinds:?}"
    );
}

#[test]
fn validate_json_walk_valid_files_ok_true() {
    let doc = rfs::info_format::validate_json_schema();
    let files = rfs::collect_rfs_files(root().join("examples/valid"))
        .expect("valid examples directory should exist");
    assert!(!files.is_empty(), "no valid examples found");

    for path in &files {
        let failed = rfs::frontend::run_validate_structured(None, path)
            .unwrap_or_else(|e| panic!("{}: {e:?}", path.display()));
        assert!(failed.is_empty(), "{} must validate clean", path.display());
        // File mode prints the object even on success: ok=true, errors=[].
        let text = rfs::frontend::render_validate_json(path, false, &failed);
        let json = rfs::json_rw::parse(&text).unwrap_or_else(|e| {
            panic!(
                "{}: rendered document is not valid JSON: {e}",
                path.display()
            )
        });
        walk_validate_json(&doc, &json, &format!("file mode {}", path.display()));
        assert_eq!(json.get("ok").and_then(|v| v.as_bool()), Some(true));
        assert!(
            json.get("errors")
                .and_then(|v| v.as_arr())
                .map(|a| a.is_empty())
                == Some(true),
            "{}: errors must be an empty array",
            path.display()
        );
    }
}

#[test]
fn validate_json_walk_directory_mode() {
    let doc = rfs::info_format::validate_json_schema();

    // All-failing directory: one object per failing file.
    let invalid_dir = root().join("examples/invalid");
    let failed =
        rfs::frontend::run_validate_structured(Some(rfs::frontend::Format::Rfs), &invalid_dir)
            .expect("directory validation should run");
    assert_eq!(failed.len(), 13, "every invalid example must fail");
    let text = rfs::frontend::render_validate_json(&invalid_dir, true, &failed);
    let json = rfs::json_rw::parse(&text).expect("directory document must be valid JSON");
    walk_validate_json(&doc, &json, "directory mode examples/invalid");
    let arr = json.as_arr().cloned().unwrap();
    assert_eq!(arr.len(), 13);
    for entry in &arr {
        assert_eq!(entry.get("ok").and_then(|v| v.as_bool()), Some(false));
        assert!(
            !entry
                .get("errors")
                .and_then(|v| v.as_arr())
                .cloned()
                .unwrap()
                .is_empty(),
            "failing file must carry errors"
        );
    }

    // All-valid directory: the array is EMPTY (valid files are omitted).
    let valid_dir = root().join("examples/valid");
    let failed =
        rfs::frontend::run_validate_structured(Some(rfs::frontend::Format::Rfs), &valid_dir)
            .expect("directory validation should run");
    assert!(failed.is_empty(), "all valid examples must pass");
    let text = rfs::frontend::render_validate_json(&valid_dir, true, &failed);
    let json = rfs::json_rw::parse(&text).unwrap();
    walk_validate_json(&doc, &json, "directory mode examples/valid");
    assert!(json.as_arr().cloned().unwrap().is_empty());
}

// ---------------------------------------------------------------------------
// info-json: schema output (Phase 7 task 10, D-P7-8)
// ---------------------------------------------------------------------------

#[test]
fn info_json_schema_document_pin() {
    let out = run_info_format(&["info-json", "--output", "schema"]);
    assert!(out.status.success(), "stderr: {}", stderr_of(&out));
    let text = stdout_of(&out);

    // Pretty JSON through json_rw (project rule) - must round-trip.
    let doc = rfs::json_rw::parse(&text).expect("schema output must be valid JSON");
    assert_eq!(
        doc.get("$schema").and_then(|v| v.as_str()),
        Some("http://json-schema.org/draft-07/schema#")
    );
    assert_eq!(
        doc.get("title").and_then(|v| v.as_str()),
        Some("Info introspection document (info-json)")
    );

    // Root: plain object with the single top-level key, strict.
    assert_eq!(doc.get("type").and_then(|v| v.as_str()), Some("object"));
    let required = doc
        .get("required")
        .and_then(|v| v.as_arr())
        .cloned()
        .unwrap();
    let req_names: Vec<String> = required
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    assert_eq!(req_names, vec!["modules"]);
    assert_eq!(
        doc.get("additionalProperties").and_then(|v| v.as_bool()),
        Some(false)
    );

    // All nine named definitions present.
    let defs = doc.get("definitions").cloned().unwrap();
    for n in [
        "info_module",
        "info_enum",
        "info_group",
        "info_variant",
        "info_field",
        "info_var",
        "info_on_error",
        "info_emit",
        "info_prop",
    ] {
        assert!(defs.get(n).is_some(), "missing definition {n}");
    }

    // Module keys in writer order (11 keys).
    let module = defs.get("info_module").cloned().unwrap();
    assert_eq!(
        prop_names(&module),
        vec![
            "name",
            "version",
            "initial_state",
            "enums",
            "events",
            "outputs",
            "vars",
            "transitions",
            "on_error",
            "properties",
            "assumptions"
        ]
    );

    // transitions is an INTEGER count (the documented non-array).
    let transitions_schema = module
        .get("properties")
        .cloned()
        .unwrap()
        .get("transitions")
        .cloned()
        .unwrap();
    assert_eq!(
        transitions_schema.get("type").and_then(|v| v.as_str()),
        Some("integer")
    );

    // Nullable simple types merge into a type array (string|null);
    // nullable REFS need anyOf ($ref cannot take siblings in draft-07).
    let props = module.get("properties").cloned().unwrap();
    for k in ["version", "initial_state"] {
        let s = props.get(k).cloned().unwrap();
        let type_arr = s.get("type").and_then(|v| v.as_arr()).cloned().unwrap();
        let types: Vec<&str> = type_arr.iter().filter_map(|v| v.as_str()).collect();
        assert_eq!(
            types,
            vec!["string", "null"],
            "{k} must be string|null: {s:?}"
        );
    }
    let on_error_schema = props.get("on_error").cloned().unwrap();
    assert!(
        on_error_schema
            .get("anyOf")
            .and_then(|v| v.as_arr())
            .cloned()
            .unwrap()
            .iter()
            .any(|a| a.get("$ref").is_some()),
        "on_error must be anyOf with $ref: {on_error_schema:?}"
    );

    // monitor_status is a PLAIN string (open-ended "skipped: <reason>" text -
    // not an enum).
    let prop_def = defs.get("info_prop").cloned().unwrap();
    assert_eq!(
        prop_names(&prop_def),
        vec!["name", "formula", "monitor_status"]
    );
    let status_schema = prop_def
        .get("properties")
        .cloned()
        .unwrap()
        .get("monitor_status")
        .cloned()
        .unwrap();
    assert_eq!(
        status_schema.get("type").and_then(|v| v.as_str()),
        Some("string")
    );
    assert!(
        status_schema.get("enum").is_none(),
        "no closed enum: {status_schema:?}"
    );

    // Unit-variant fields are null (nullable array of info_field).
    let variant_def = defs.get("info_variant").cloned().unwrap();
    let fields_schema = variant_def
        .get("properties")
        .cloned()
        .unwrap()
        .get("fields")
        .cloned()
        .unwrap();
    assert!(fields_schema.get("anyOf").is_some(), "{fields_schema:?}");
}

#[test]
fn info_json_md_pins_sections() {
    let out = run_info_format(&["info-json"]);
    assert!(out.status.success(), "stderr: {}", stderr_of(&out));
    let text = stdout_of(&out);

    assert!(
        text.starts_with("# Info introspection document (info-json)\n"),
        "{}...",
        &text[..text.len().min(60)]
    );
    assert!(text.contains("## document"), "missing document section");
    assert!(
        text.contains("| `modules` | array of [info_module] | yes |"),
        "missing modules row"
    );

    // One section per named definition (9).
    let defs = rfs::info_format::info_json_schema().definitions;
    assert_eq!(defs.len(), 9);
    for (name, _) in &defs {
        assert!(
            text.contains(&format!("\n## {name}\n")),
            "missing section for {name}"
        );
    }

    // The documented non-array: transitions row says integer + count.
    assert!(
        text.contains("| `transitions` | integer | yes |"),
        "missing transitions row"
    );
    assert!(
        text.contains("NOT an array of transition objects"),
        "{text}"
    );

    // Nullable rows render as "... or null".
    for k in ["version", "initial_state", "on_error"] {
        assert!(
            text.contains(&format!("| `{k}` | ")) && text.contains("or null"),
            "missing nullable row {k}"
        );
    }
}

// ---------------------------------------------------------------------------
// info-json: D-P7-9 sync walk over every valid example (Phase 7 task 10)
// ---------------------------------------------------------------------------

#[test]
fn info_json_walk_all_valid_examples() {
    let doc = rfs::info_format::info_json_schema();
    let files = rfs::collect_rfs_files(root().join("examples/valid"))
        .expect("valid examples directory should exist");
    assert!(!files.is_empty(), "no valid examples found");

    // Corpus coverage: the walk must exercise the optional shapes, not just
    // the all-null fast path.
    let mut saw_on_error = false;
    let mut saw_assumption = false;
    let mut saw_skipped_monitor = false;

    for path in &files {
        // The exact command path: parse + validate + render_json.
        let text = rfs::info::run_info(path, &rfs::info::InfoOptions { json: true })
            .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let json = rfs::json_rw::parse(&text).unwrap_or_else(|e| {
            panic!(
                "{}: rendered document is not valid JSON: {e}",
                path.display()
            )
        });

        // (a) every required key present, (b) no undocumented key at any
        // object level, (c) types + constraints agree.
        if let Err(errs) = rfs::json_schema::walk_check(&doc, &json) {
            panic!(
                "{}: info-json model out of sync with info.rs render_json:\n{}",
                path.display(),
                errs.join("\n")
            );
        }

        // (d) the transitions count is really the module's transition count.
        let ir = rfs::parse_file(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let modules = json
            .get("modules")
            .and_then(|v| v.as_arr())
            .cloned()
            .unwrap();
        assert_eq!(modules.len(), ir.modules.len());
        for (i, m) in modules.iter().enumerate() {
            let count = m
                .get("transitions")
                .and_then(|v| v.as_int())
                .expect("transitions must be an integer");
            assert_eq!(
                count,
                ir.modules[i].transitions.len() as i64,
                "{}: transitions count mismatch",
                path.display()
            );

            if m.get("on_error")
                .is_some_and(|v| !matches!(v, rfs::json_rw::Json::Null))
            {
                saw_on_error = true;
            }
            if let Some(a) = m.get("assumptions").and_then(|v| v.as_arr()) {
                if !a.is_empty() {
                    saw_assumption = true;
                }
            }
            for list in ["properties", "assumptions"] {
                if let Some(arr) = m.get(list).and_then(|v| v.as_arr()) {
                    for p in arr {
                        if let Some(s) = p.get("monitor_status").and_then(|v| v.as_str()) {
                            if s.starts_with("skipped:") {
                                saw_skipped_monitor = true;
                            }
                        }
                    }
                }
            }
        }
    }

    assert!(saw_on_error, "corpus lost its on_error coverage");
    assert!(saw_assumption, "corpus lost its assumption coverage");
    assert!(
        saw_skipped_monitor,
        "corpus lost its skipped-monitor coverage"
    );
}

// ---------------------------------------------------------------------------
// diagram-dot: md document + generated-output sync (Phase 7 task 11)
// ---------------------------------------------------------------------------

#[test]
fn diagram_dot_md_pins_sections() {
    let out = run_info_format(&["diagram-dot"]);
    assert!(out.status.success(), "stderr: {}", stderr_of(&out));
    let text = stdout_of(&out);

    assert!(
        text.starts_with("# DOT state diagram subset (diagram-dot)\n"),
        "{}...",
        &text[..text.len().min(60)]
    );

    // All documented sections.
    for s in [
        "## Graph header",
        "## Node lines",
        "## Edge lines",
        "## on_error edges",
        "## String escaping",
    ] {
        assert!(text.contains(s), "missing section {s}");
    }

    // The exact documented line forms.
    for l in [
        "digraph <module_name> {",
        "  rankdir=LR;",
        "  node [shape=box];",
        "  \"<state>\";",
        "  \"<state>\" [style=filled, fillcolor=\"#e0e0e0\"];",
        "[label=\"<EventLabel> [guard] -> <next>\"]",
        "[label=\"on_error -> <fallback>\", style=dashed]",
    ] {
        assert!(text.contains(l), "missing documented line form: {l}");
    }

    // md-only entry: schema output is rejected with the supported list.
    let out = run_info_format(&["diagram-dot", "--output", "schema"]);
    assert!(!out.status.success());
    let err = stderr_of(&out);
    assert!(err.contains("does not support output 'schema'"), "{err}");
    assert!(err.contains("supported: md"), "{err}");
}

/// The coverage check for the diagram-dot format (D-P7-9 style): every line
/// of the DOT text `generate()` emits for a module must be one of the
/// documented line shapes filled with that module's IR data - no undocumented
/// token. The expected lines are rebuilt from the IR with the documented
/// templates (event/guard label text via the pub label helpers, which implement
/// the documented `<EventLabel>` / `[guard]` renderings).
fn check_dot_documented_subset(text: &str, m: &rfs::ir::Module, what: &str) {
    let lines: Vec<&str> = text.lines().collect();

    // Header: exactly the three documented lines.
    assert_eq!(
        lines.first().copied(),
        Some(format!("digraph {} {{", m.name).as_str()),
        "{what}: undocumented header line"
    );
    assert_eq!(
        lines.get(1),
        Some(&"  rankdir=LR;"),
        "{what}: undocumented rankdir line"
    );
    assert_eq!(
        lines.get(2),
        Some(&"  node [shape=box];"),
        "{what}: undocumented node-default line"
    );

    // Footer.
    assert_eq!(lines.last(), Some(&"}"), "{what}: undocumented footer line");

    let states: Vec<&str> = m
        .enums
        .iter()
        .find(|e| e.name == "State")
        .map(|e| e.variants.iter().map(String::as_str).collect())
        .unwrap_or_default();

    let mut idx = 3;
    // Node lines: one per state, declaration order; filled IFF the initial.
    for s in states.iter().copied() {
        let line = *lines.get(idx).unwrap_or(&"<missing line>");
        if Some(s) == m.initial_state.as_deref() {
            assert_eq!(
                line,
                format!("  \"{s}\" [style=filled, fillcolor=\"#e0e0e0\"];"),
                "{what}: initial-state node line not in the documented shape"
            );
        } else {
            assert_eq!(
                line,
                format!("  \"{s}\";"),
                "{what}: node line not in the documented shape"
            );
        }
        idx += 1;
    }

    // Edge lines: one per transition, IR order; label = <EventLabel> [guard] -> <next>.
    for t in &m.transitions {
        let line = *lines.get(idx).unwrap_or(&"<missing line>");
        let ev = rfs::diagrams::event_label(&t.pattern);
        let label = match rfs::diagrams::guard_label(&t.guard) {
            None => format!("{ev} -> {}", t.next_state),
            Some(g) => format!("{ev} [{g}] -> {}", t.next_state),
        };
        assert_eq!(
            line,
            format!(
                "  \"{}\" -> \"{}\" [label=\"{}\"];",
                t.state, t.next_state, label
            ),
            "{what}: edge line not in the documented shape"
        );
        idx += 1;
    }

    // on_error edges: IFF declared, one per state to the fallback target.
    if let Some(on) = &m.on_error {
        for s in &states {
            let line = *lines.get(idx).unwrap_or(&"<missing line>");
            assert_eq!(
                line,
                format!(
                    "  \"{s}\" -> \"{}\" [label=\"on_error -> {}\", style=dashed];",
                    on.next_state, on.next_state
                ),
                "{what}: on_error edge line not in the documented shape"
            );
            idx += 1;
        }
    }

    // No extra lines beyond the documented structure (the footer is the
    // last line; idx points at it after the content lines).
    assert_eq!(idx, lines.len() - 1, "{what}: undocumented trailing lines");
}

#[test]
fn diagram_dot_generated_output_matches_documented_subset() {
    let files = rfs::collect_rfs_files(root().join("examples/valid"))
        .expect("valid examples directory should exist");
    assert!(!files.is_empty(), "no valid examples found");

    // Corpus coverage: the walk must exercise every documented construct,
    // not just the plain fast path.
    let mut saw_guard = false;
    let mut saw_on_error = false;
    let mut saw_struct_event = false;

    for path in &files {
        let ir = rfs::parse_file(path)
            .unwrap_or_else(|e| panic!("{}: valid example failed to parse: {e}", path.display()));
        for m in &ir.modules {
            let text = rfs::diagrams::dot::generate(m);
            check_dot_documented_subset(&text, m, &path.display().to_string());

            for t in &m.transitions {
                if t.guard.is_some() {
                    saw_guard = true;
                }
                if matches!(
                    &t.pattern.variant,
                    Some(rfs::ir::PatternVariant::Struct { .. })
                ) {
                    saw_struct_event = true;
                }
            }
            if m.on_error.is_some() {
                saw_on_error = true;
            }
        }
    }

    assert!(saw_guard, "corpus lost its guard-label coverage");
    assert!(saw_on_error, "corpus lost its on_error-edge coverage");
    assert!(
        saw_struct_event,
        "corpus lost its struct-event-label coverage"
    );
}

// ---------------------------------------------------------------------------
// diagram-puml: md document + generated-output sync (Phase 7 task 12)
// ---------------------------------------------------------------------------

#[test]
fn diagram_puml_md_pins_sections() {
    let out = run_info_format(&["diagram-puml"]);
    assert!(out.status.success(), "stderr: {}", stderr_of(&out));
    let text = stdout_of(&out);

    assert!(
        text.starts_with("# PlantUML state diagram subset (diagram-puml)\n"),
        "{}...",
        &text[..text.len().min(60)]
    );

    // All documented sections.
    for s in [
        "## Frame lines",
        "## Transition lines",
        "## note-on-link lines",
        "## on_error edges",
        "## State names and escaping",
    ] {
        assert!(text.contains(s), "missing section {s}");
    }

    // The exact documented line forms.
    for l in [
        "@startuml",
        "title <module_name>",
        "[*] --> <initial>",
        "@enduml",
        "<from> --> <to> : <EventLabel>",
        "<from> --> <to> : <EventLabel> \\n <guard>",
        "note on link : emits <OutputLabel>",
        "<state> -[dashed]-> <fallback> : on_error",
    ] {
        assert!(text.contains(l), "missing documented line form: {l}");
    }

    // md-only entry: schema output is rejected with the supported list.
    let out = run_info_format(&["diagram-puml", "--output", "schema"]);
    assert!(!out.status.success());
    let err = stderr_of(&out);
    assert!(err.contains("does not support output 'schema'"), "{err}");
    assert!(err.contains("supported: md"), "{err}");
}

/// The coverage check for the diagram-puml format (D-P7-9 style): every line
/// of the PlantUML text `generate()` emits for a module must be one of the
/// documented line shapes filled with that module's IR data - no undocumented
/// token. The expected lines are rebuilt from the IR with the documented
/// templates (event/guard/emit label text via the pub label helpers, which
/// implement the documented renderings).
fn check_puml_documented_subset(text: &str, m: &rfs::ir::Module, what: &str) {
    let lines: Vec<&str> = text.lines().collect();

    // Frame: @startuml + title first, @enduml last.
    assert_eq!(
        lines.first().copied(),
        Some("@startuml"),
        "{what}: undocumented first line"
    );
    assert_eq!(
        lines.get(1).copied(),
        Some(format!("title {}", m.name).as_str()),
        "{what}: undocumented title line"
    );
    assert_eq!(
        lines.last().copied(),
        Some("@enduml"),
        "{what}: undocumented last line"
    );

    let mut idx = 2;
    // [*] --> initial IFF the module has an initial state.
    if let Some(init) = &m.initial_state {
        assert_eq!(
            lines.get(idx).copied(),
            Some(format!("[*] --> {init}").as_str()),
            "{what}: undocumented initial-state line"
        );
        idx += 1;
    }

    // Transition lines + note-on-link lines, IR order.
    for t in &m.transitions {
        let ev = rfs::diagrams::event_label(&t.pattern);
        let label = match rfs::diagrams::guard_label(&t.guard) {
            None => ev.clone(),
            Some(g) => format!("{ev} \\n {g}"),
        };
        assert_eq!(
            lines.get(idx).copied(),
            Some(format!("{} --> {} : {}", t.state, t.next_state, label).as_str()),
            "{what}: transition line not in the documented shape"
        );
        idx += 1;
        for e in rfs::diagrams::emit_labels(&t.emits) {
            assert_eq!(
                lines.get(idx).copied(),
                Some(format!("note on link : emits {e}").as_str()),
                "{what}: note-on-link line not in the documented shape"
            );
            idx += 1;
        }
    }

    // on_error edges: IFF declared, one per state to the fallback target.
    if let Some(on) = &m.on_error {
        for s in rfs::diagrams::states(m) {
            assert_eq!(
                lines.get(idx).copied(),
                Some(format!("{s} -[dashed]-> {} : on_error", on.next_state).as_str()),
                "{what}: on_error edge line not in the documented shape"
            );
            idx += 1;
            for e in rfs::diagrams::emit_labels(&on.emits) {
                assert_eq!(
                    lines.get(idx).copied(),
                    Some(format!("note on link : emits {e}").as_str()),
                    "{what}: on_error note-on-link line not in the documented shape"
                );
                idx += 1;
            }
        }
    }

    // No extra lines beyond the documented structure (@enduml is last).
    assert_eq!(idx, lines.len() - 1, "{what}: undocumented trailing lines");
}

#[test]
fn diagram_puml_generated_output_matches_documented_subset() {
    let files = rfs::collect_rfs_files(root().join("examples/valid"))
        .expect("valid examples directory should exist");
    assert!(!files.is_empty(), "no valid examples found");

    // Corpus coverage: the walk must exercise every documented construct,
    // not just the plain fast path.
    let mut saw_guard = false;
    let mut saw_on_error = false;
    let mut saw_struct_event = false;
    let mut saw_emit_note = false;

    for path in &files {
        let ir = rfs::parse_file(path)
            .unwrap_or_else(|e| panic!("{}: valid example failed to parse: {e}", path.display()));
        for m in &ir.modules {
            let text = rfs::diagrams::plantuml::generate(m);
            check_puml_documented_subset(&text, m, &path.display().to_string());

            for t in &m.transitions {
                if t.guard.is_some() {
                    saw_guard = true;
                }
                if !t.emits.is_empty() {
                    saw_emit_note = true;
                }
                if matches!(
                    &t.pattern.variant,
                    Some(rfs::ir::PatternVariant::Struct { .. })
                ) {
                    saw_struct_event = true;
                }
            }
            if let Some(on) = &m.on_error {
                saw_on_error = true;
                if !on.emits.is_empty() {
                    saw_emit_note = true;
                }
            }
        }
    }

    assert!(saw_guard, "corpus lost its guard-label coverage");
    assert!(saw_on_error, "corpus lost its on_error-edge coverage");
    assert!(
        saw_struct_event,
        "corpus lost its struct-event-label coverage"
    );
    assert!(saw_emit_note, "corpus lost its note-on-link coverage");
}

// ---------------------------------------------------------------------------
// diagram-mmd: md document + generated-output sync (Phase 7 task 13)
// ---------------------------------------------------------------------------

#[test]
fn diagram_mmd_md_pins_sections() {
    let out = run_info_format(&["diagram-mmd"]);
    assert!(out.status.success(), "stderr: {}", stderr_of(&out));
    let text = stdout_of(&out);

    assert!(
        text.starts_with("# Mermaid state diagram subset (diagram-mmd)\n"),
        "{}...",
        &text[..text.len().min(60)]
    );

    // All documented sections.
    for s in [
        "## Frame lines",
        "## Transition lines",
        "## on_error edges",
        "## State names and escaping",
    ] {
        assert!(text.contains(s), "missing section {s}");
    }

    // The exact documented line forms.
    for l in [
        "stateDiagram-v2",
        "  [*] --> <initial>",
        "  <from> --> <to>: <EventLabel>",
        "  <from> --> <to>: <EventLabel> \\n <guard>",
        "  <from> --> <to>: <EventLabel> \\n emits <OutputLabel>",
        "  <state> --> <fallback>: [on_error]",
        "  <state> --> <fallback>: [on_error] \\n emits <OutputLabel>",
    ] {
        assert!(text.contains(l), "missing documented line form: {l}");
    }

    // md-only entry: schema output is rejected with the supported list.
    let out = run_info_format(&["diagram-mmd", "--output", "schema"]);
    assert!(!out.status.success());
    let err = stderr_of(&out);
    assert!(err.contains("does not support output 'schema'"), "{err}");
    assert!(err.contains("supported: md"), "{err}");
}

/// The coverage check for the diagram-mmd format (D-P7-9 style): every line
/// of the Mermaid text `generate()` emits for a module must be one of the
/// documented line shapes filled with that module's IR data - no undocumented
/// token. The expected lines are rebuilt from the IR with the documented
/// templates (event/guard/emit label text via the pub label helpers, which
/// implement the documented renderings; the multi-line label is assembled
/// exactly as `push_transition` does: ` \n <guard>` then one
/// ` \n emits <O>` per output).
fn check_mmd_documented_subset(text: &str, m: &rfs::ir::Module, what: &str) {
    let lines: Vec<&str> = text.lines().collect();

    // Header: always exactly `stateDiagram-v2` (the only unindented line).
    assert_eq!(
        lines.first().copied(),
        Some("stateDiagram-v2"),
        "{what}: undocumented header line"
    );

    let mut idx = 1;
    // [*] --> initial IFF the module has an initial state.
    if let Some(init) = &m.initial_state {
        assert_eq!(
            lines.get(idx).copied(),
            Some(format!("  [*] --> {init}").as_str()),
            "{what}: undocumented initial-state line"
        );
        idx += 1;
    }

    // Transition lines, IR order: label = <EventLabel> [ \n <guard>] [ \n emits <O>]*.
    for t in &m.transitions {
        let ev = rfs::diagrams::event_label(&t.pattern);
        let mut label = ev;
        if let Some(g) = rfs::diagrams::guard_label(&t.guard) {
            label.push_str(" \\n ");
            label.push_str(&g);
        }
        for e in rfs::diagrams::emit_labels(&t.emits) {
            label.push_str(&format!(" \\n emits {e}"));
        }
        assert_eq!(
            lines.get(idx).copied(),
            Some(format!("  {} --> {}: {}", t.state, t.next_state, label).as_str()),
            "{what}: transition line not in the documented shape"
        );
        idx += 1;
    }

    // on_error edges: IFF declared, one per state to the fallback target.
    if let Some(on) = &m.on_error {
        for s in rfs::diagrams::states(m) {
            let mut label = String::from("[on_error]");
            for e in rfs::diagrams::emit_labels(&on.emits) {
                label.push_str(&format!(" \\n emits {e}"));
            }
            assert_eq!(
                lines.get(idx).copied(),
                Some(format!("  {s} --> {}: {}", on.next_state, label).as_str()),
                "{what}: on_error edge line not in the documented shape"
            );
            idx += 1;
        }
    }

    // No extra lines beyond the documented structure (mmd has no footer).
    assert_eq!(idx, lines.len(), "{what}: undocumented trailing lines");
}

#[test]
fn diagram_mmd_generated_output_matches_documented_subset() {
    let files = rfs::collect_rfs_files(root().join("examples/valid"))
        .expect("valid examples directory should exist");
    assert!(!files.is_empty(), "no valid examples found");

    // Corpus coverage: the walk must exercise every documented construct,
    // not just the plain fast path.
    let mut saw_guard = false;
    let mut saw_on_error = false;
    let mut saw_struct_event = false;
    let mut saw_emit_label = false;

    for path in &files {
        let ir = rfs::parse_file(path)
            .unwrap_or_else(|e| panic!("{}: valid example failed to parse: {e}", path.display()));
        for m in &ir.modules {
            let text = rfs::diagrams::mermaid::generate(m);
            check_mmd_documented_subset(&text, m, &path.display().to_string());

            for t in &m.transitions {
                if t.guard.is_some() {
                    saw_guard = true;
                }
                if !t.emits.is_empty() {
                    saw_emit_label = true;
                }
                if matches!(
                    &t.pattern.variant,
                    Some(rfs::ir::PatternVariant::Struct { .. })
                ) {
                    saw_struct_event = true;
                }
            }
            if let Some(on) = &m.on_error {
                saw_on_error = true;
                if !on.emits.is_empty() {
                    saw_emit_label = true;
                }
            }
        }
    }

    assert!(saw_guard, "corpus lost its guard-label coverage");
    assert!(saw_on_error, "corpus lost its on_error-edge coverage");
    assert!(
        saw_struct_event,
        "corpus lost its struct-event-label coverage"
    );
    assert!(saw_emit_label, "corpus lost its emits-label coverage");
}

// ---------------------------------------------------------------------------
// artifact-header: md document + generated-output sync (Phase 7 task 14)
// ---------------------------------------------------------------------------

#[test]
fn artifact_header_md_pins_sections() {
    let out = run_info_format(&["artifact-header"]);
    assert!(out.status.success(), "stderr: {}", stderr_of(&out));
    let text = stdout_of(&out);

    assert!(
        text.starts_with("# Generated artifact header (artifact-header)\n"),
        "{}...",
        &text[..text.len().min(60)]
    );

    // All documented sections.
    for s in ["## Fields", "## Which artifacts carry it", "## Purpose"] {
        assert!(text.contains(s), "missing section {s}");
    }

    // The exact documented line form.
    assert!(
        text.contains(
            "// Generated by RFS v<toolchain_version> | spec sha256:<spec_hash> | module <module_name>"
        ),
        "missing documented line form"
    );

    // md-only entry: schema output is rejected with the supported list.
    let out = run_info_format(&["artifact-header", "--output", "schema"]);
    assert!(!out.status.success());
    let err = stderr_of(&out);
    assert!(err.contains("does not support output 'schema'"), "{err}");
    assert!(err.contains("supported: md"), "{err}");
}

/// The documented line-1 template filled with the IR data (D-P7-9 style):
/// toolchain version = the rfs crate version, spec hash = first 16 hex chars
/// of SHA-256 over the WHOLE file's canonical IR JSON text, module = the
/// FIRST module (the CLI rule `codegen::run` applies).
fn expected_header_line(ir: &rfs::ir::Ir) -> String {
    let json_text = ir.to_json().expect("canonical IR JSON").to_string();
    format!(
        "// Generated by RFS v{} | spec sha256:{} | module {}",
        env!("CARGO_PKG_VERSION"),
        rfs::spec_hash::spec_hash16(&json_text),
        ir.modules[0].name
    )
}

#[test]
fn artifact_header_generated_output_matches_documented_format() {
    let files = rfs::collect_rfs_files(root().join("examples/valid"))
        .expect("valid examples directory should exist");
    assert!(!files.is_empty(), "no valid examples found");

    // Corpus sanity: the stamp must actually vary with the spec (a constant
    // hash would make the sync test vacuous).
    let mut stamps = std::collections::BTreeSet::new();

    for path in &files {
        let src = std::fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        let ir = rfs::parse_source(&src)
            .unwrap_or_else(|e| panic!("{}: valid example failed to parse: {e}", path.display()));
        stamps.insert(expected_header_line(&ir));

        for backend_name in ["rust", "c"] {
            let backend = rfs::codegen::get_backend(backend_name)
                .unwrap_or_else(|| panic!("backend {backend_name} registered"));
            let options = rfs::codegen::BackendOptions::default();
            for command in [
                rfs::codegen::CodegenCommandKind::Production,
                rfs::codegen::CodegenCommandKind::UnitTests,
                rfs::codegen::CodegenCommandKind::ModelChecker,
            ] {
                let out = command.run(backend, &ir, &options).unwrap_or_else(|e| {
                    panic!(
                        "codegen {backend_name} {:?} {} must succeed: {e:?}",
                        command,
                        path.display()
                    )
                });
                let what = format!("{path:?} {backend_name} {command:?}");
                assert_eq!(
                    out.code.lines().next(),
                    Some(expected_header_line(&ir).as_str()),
                    "{what}: line 1 not in the documented header form"
                );
                // The test command's fuzz skeleton extra file carries the same
                // stamp; no other command emits extra files.
                if matches!(command, rfs::codegen::CodegenCommandKind::UnitTests) {
                    assert_eq!(out.extra_files.len(), 1, "{what}: one extra file");
                    let name = if backend_name == "rust" {
                        "fuzz_target.rs"
                    } else {
                        "fuzz_target.c"
                    };
                    assert_eq!(out.extra_files[0].name, name, "{what}");
                    assert_eq!(
                        out.extra_files[0].content.lines().next(),
                        Some(expected_header_line(&ir).as_str()),
                        "{what}: fuzz skeleton line 1 not in the documented header form"
                    );
                } else {
                    assert!(out.extra_files.is_empty(), "{what}: no extra files");
                }
            }
        }
    }

    assert!(
        stamps.len() > 1,
        "corpus lost its spec-hash variation (sync test would be vacuous)"
    );
}

// ---------------------------------------------------------------------------
// CLI error cases
// ---------------------------------------------------------------------------

#[test]
fn unknown_format_errors_with_list_hint() {
    let out = run_info_format(&["nope"]);
    assert!(!out.status.success());
    let err = stderr_of(&out);
    assert!(err.contains("unknown format 'nope'"), "{err}");
    assert!(err.contains("--list"), "{err}");
}

#[test]
fn unsupported_output_errors() {
    let out = run_info_format(&["spec-rfs", "--output", "schema"]);
    assert!(!out.status.success());
    let err = stderr_of(&out);
    assert!(err.contains("does not support output 'schema'"), "{err}");
    assert!(err.contains("md, plantuml, svg"), "{err}");
}

#[test]
fn no_name_and_no_list_errors() {
    let out = run_info_format(&[]);
    assert!(!out.status.success());
    let err = stderr_of(&out);
    assert!(err.contains("requires a format name"), "{err}");
}
