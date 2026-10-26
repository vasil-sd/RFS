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

use crate::ebnf_grammar;

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

/// The format registry. Order = `--list` order.
pub static FORMATS: &[FormatInfo] = &[FormatInfo {
    name: "spec-rfs",
    summary: "RFS v0 input language - EBNF grammar (what rfs parse accepts)",
    outputs: &["md", "plantuml", "svg"],
    render: spec_rfs_render,
}];

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
