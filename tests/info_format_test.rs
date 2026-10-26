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

use std::path::PathBuf;
use std::process::{Command, Output};

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

    // Byte pin of the current registry (tasks 5-6 ship spec-rfs + spec-json).
    assert_eq!(
        expected,
        vec!["spec-rfs".to_string(), "spec-json".to_string()]
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
