//! Transition tables + variable/event summaries (plugins for
//! `rfs diagram --format table` and `--format md`).
//!
//! Both formats share one data layer ([`build`]) - the state x event matrix,
//! the on_error summary, and the typed variable/event/output lists - so the
//! plain-text and Markdown renderings can never disagree. Neither format has
//! an external renderer (no `--render` support).
//! Docs: dev:cli-commands-overview, llm:cli-commands

use crate::diagram::DiagramPlugin;
use crate::diagrams::{emit_labels, event_label, guard_label, states};
use crate::ir::{ConstructorGroup, ConstructorVariant, Module};

/// One transition inside a matrix cell.
#[derive(Debug, Clone)]
struct CellEntry {
    guard: Option<String>,
    next: String,
    emits: Vec<String>,
}

impl CellEntry {
    /// `[guard] -> next; emits A, B` (absent parts omitted).
    fn text(&self) -> String {
        let mut t = String::new();
        if let Some(g) = &self.guard {
            t.push('[');
            t.push_str(g);
            t.push_str("] ");
        }
        t.push_str(&format!("-> {}", self.next));
        if !self.emits.is_empty() {
            t.push_str("; emits ");
            t.push_str(&self.emits.join(", "));
        }
        t
    }
}

/// Event/output group with variant field types: `Req: R(node: bit<2>)`.
fn group_line(g: &ConstructorGroup) -> String {
    let variants: Vec<String> = g
        .variants
        .iter()
        .map(|v| match v {
            ConstructorVariant::Unit { name } => name.clone(),
            ConstructorVariant::Struct { name, fields } => {
                let inner: Vec<String> = fields
                    .iter()
                    .map(|f| format!("{}: {}", f.name, crate::info::render_type(&f.ty)))
                    .collect();
                format!("{name}({})", inner.join(", "))
            }
        })
        .collect();
    // A bare event/output (one unit variant named like the group) renders
    // without the redundant `Name: Name` doubling.
    if variants.len() == 1 && variants[0] == g.name {
        g.name.clone()
    } else {
        format!("{}: {}", g.name, variants.join(", "))
    }
}

struct TableData {
    module: String,
    initial: Option<String>,
    state_names: Vec<String>,
    /// Distinct event labels in first-occurrence order (matrix columns).
    columns: Vec<String>,
    /// cells[state][column] = transitions from that state with that event.
    cells: Vec<Vec<Vec<CellEntry>>>,
    on_error_next: Option<String>,
    on_error_emits: Vec<String>,
    vars: Vec<(String, String, String)>, // (name, type, init)
    events: Vec<String>,
    outputs: Vec<String>,
}

fn build(m: &Module) -> TableData {
    let state_names: Vec<String> = states(m).iter().map(|s| s.to_string()).collect();
    let mut columns: Vec<String> = Vec::new();
    for t in &m.transitions {
        let l = event_label(&t.pattern);
        if !columns.contains(&l) {
            columns.push(l);
        }
    }
    let mut cells: Vec<Vec<Vec<CellEntry>>> =
        vec![vec![Vec::new(); columns.len()]; state_names.len()];
    for t in &m.transitions {
        let Some(si) = state_names.iter().position(|s| s == &t.state) else {
            continue;
        };
        let ci = columns
            .iter()
            .position(|c| c == &event_label(&t.pattern))
            .expect("column added above");
        cells[si][ci].push(CellEntry {
            guard: guard_label(&t.guard),
            next: t.next_state.clone(),
            emits: emit_labels(&t.emits),
        });
    }
    let (on_error_next, on_error_emits) = match &m.on_error {
        Some(on) => (Some(on.next_state.clone()), emit_labels(&on.emits)),
        None => (None, Vec::new()),
    };
    TableData {
        module: m.name.clone(),
        initial: m.initial_state.clone(),
        state_names,
        columns,
        cells,
        on_error_next,
        on_error_emits,
        vars: m
            .vars
            .iter()
            .map(|v| {
                (
                    v.name.clone(),
                    crate::info::render_type(&v.ty),
                    crate::info::render_expr(&v.init),
                )
            })
            .collect(),
        events: m.events.iter().map(group_line).collect(),
        outputs: m.outputs.iter().map(group_line).collect(),
    }
}

// ---------------------------------------------------------------------------
// Plain-text rendering (`table` plugin, extension .txt)
// ---------------------------------------------------------------------------

fn render_text(d: &TableData) -> String {
    let mut out = String::new();
    out.push_str(&format!("Module: {}\n", d.module));
    out.push_str(&format!(
        "States: {}\n",
        if d.state_names.is_empty() {
            "(none)".into()
        } else {
            d.state_names.join(", ")
        }
    ));
    match &d.initial {
        Some(s) => out.push_str(&format!("Initial: {s}\n")),
        None => out.push_str("Initial: (none)\n"),
    }
    out.push('\n');

    out.push_str("Transition matrix (rows = states, columns = events):\n");
    out.push('\n');
    if d.state_names.is_empty() || d.columns.is_empty() {
        out.push_str("(no transitions)\n");
    } else {
        // Column widths: header + every cell (multi-entry cells join with " | ").
        let cell_text: Vec<Vec<String>> = d
            .cells
            .iter()
            .map(|row| {
                row.iter()
                    .map(|entries| {
                        entries
                            .iter()
                            .map(CellEntry::text)
                            .collect::<Vec<_>>()
                            .join(" | ")
                    })
                    .collect()
            })
            .collect();
        let widths: Vec<usize> = d
            .columns
            .iter()
            .enumerate()
            .map(|(ci, col)| {
                let mut w = col.len();
                for row in &cell_text {
                    w = w.max(row[ci].len());
                }
                w
            })
            .collect();
        // At least 5 so the "state" header fits.
        let state_w = d
            .state_names
            .iter()
            .map(|s| s.len())
            .max()
            .unwrap_or(0)
            .max(5);

        // Header.
        let mut header = String::new();
        header.push_str(&format!("{:<w$} | ", "state", w = state_w));
        let mut sep = format!("{}-+", "-".repeat(state_w));
        for (ci, col) in d.columns.iter().enumerate() {
            if ci > 0 {
                header.push_str(" | ");
                sep.push_str("+-");
            }
            header.push_str(&format!("{:<w$}", col, w = widths[ci]));
            sep.push_str(&"-".repeat(widths[ci] + 1));
        }
        out.push_str(header.trim_end());
        out.push('\n');
        out.push_str(sep.trim_end());
        out.push('\n');

        for (si, s) in d.state_names.iter().enumerate() {
            let mut line = String::new();
            line.push_str(&format!("{:<w$} | ", s, w = state_w));
            let mut first = true;
            for c in &cell_text[si] {
                if !first {
                    line.push_str(" | ");
                }
                first = false;
                line.push_str(if c.is_empty() { "-" } else { c });
            }
            out.push_str(line.trim_end());
            out.push('\n');
        }
    }

    if let Some(next) = &d.on_error_next {
        let mut t = format!("\non_error (from any state): -> {next}");
        if !d.on_error_emits.is_empty() {
            t.push_str("; emits ");
            t.push_str(&d.on_error_emits.join(", "));
        }
        t.push('\n');
        out.push_str(&t);
    }

    out.push('\n');
    out.push_str("Variables:\n");
    if d.vars.is_empty() {
        out.push_str("  (none)\n");
    } else {
        for (name, ty, init) in &d.vars {
            out.push_str(&format!("  {name}: {ty} = {init}\n"));
        }
    }
    out.push_str("Events:\n");
    if d.events.is_empty() {
        out.push_str("  (none)\n");
    } else {
        for e in &d.events {
            out.push_str(&format!("  {e}\n"));
        }
    }
    out.push_str("Outputs:\n");
    if d.outputs.is_empty() {
        out.push_str("  (none)\n");
    } else {
        for o in &d.outputs {
            out.push_str(&format!("  {o}\n"));
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Markdown rendering (`md` plugin, extension .md)
// ---------------------------------------------------------------------------

fn md_escape(s: &str) -> String {
    s.replace('|', "\\|")
}

fn render_md(d: &TableData) -> String {
    let mut out = String::new();
    out.push_str(&format!("# {} - transition table\n\n", d.module));
    let state_list = if d.state_names.is_empty() {
        "(none)".to_string()
    } else {
        d.state_names.join(", ")
    };
    let initial = match &d.initial {
        Some(s) => s.clone(),
        None => "(none)".to_string(),
    };
    out.push_str(&format!(
        "**States:** {state_list}\n\n**Initial state:** {initial}\n\n"
    ));
    out.push_str("## Transition matrix\n\n");
    if d.state_names.is_empty() || d.columns.is_empty() {
        out.push_str("_No transitions._\n");
    } else {
        // Header row.
        let cols: Vec<String> = d.columns.iter().map(|c| md_escape(c)).collect();
        out.push_str(&format!("| state | {} |\n", cols.join(" | ")));
        let sep_cells: Vec<String> = vec!["---".to_string(); d.columns.len() + 1];
        out.push_str(&format!("| {} |\n", sep_cells.join(" | ")));
        for (si, s) in d.state_names.iter().enumerate() {
            let cells: Vec<String> = d.cells[si]
                .iter()
                .map(|entries| {
                    if entries.is_empty() {
                        "-".to_string()
                    } else {
                        entries
                            .iter()
                            .map(CellEntry::text)
                            .collect::<Vec<_>>()
                            .join("<br>")
                    }
                })
                .collect();
            out.push_str(&format!("| {} | {} |\n", md_escape(s), cells.join(" | ")));
        }
    }
    if let Some(next) = &d.on_error_next {
        let mut t = format!("\n## on_error\n\nFrom any state: `-> {next}`");
        if !d.on_error_emits.is_empty() {
            t.push_str(&format!("; emits {}", d.on_error_emits.join(", ")));
        }
        t.push('\n');
        out.push_str(&t);
    }
    out.push_str("\n## Variables\n\n");
    if d.vars.is_empty() {
        out.push_str("_(none)_\n");
    } else {
        for (name, ty, init) in &d.vars {
            out.push_str(&format!("- `{name}: {ty}` = `{init}`\n"));
        }
    }
    out.push_str("\n## Events\n\n");
    if d.events.is_empty() {
        out.push_str("_(none)_\n");
    } else {
        for e in &d.events {
            out.push_str(&format!("- `{e}`\n"));
        }
    }
    out.push_str("\n## Outputs\n\n");
    if d.outputs.is_empty() {
        out.push_str("_(none)_\n");
    } else {
        for o in &d.outputs {
            out.push_str(&format!("- `{o}`\n"));
        }
    }
    out
}

pub fn generate_table(m: &Module) -> String {
    render_text(&build(m))
}

pub fn generate_md(m: &Module) -> String {
    render_md(&build(m))
}

/// The registered plugins (one line each in `crate::diagram::ALL_DIAGRAMS`).
pub const TABLE_PLUGIN: DiagramPlugin = DiagramPlugin {
    name: "table",
    description: "Transition table + variable/event summary (plain text)",
    ext: "txt",
    generate: generate_table,
    svg_tool: None,
    plan_render: None,
    render_svg: None,
    internal_render_svg: None,
    internal_renderer_name: None,
};

pub const MD_PLUGIN: DiagramPlugin = DiagramPlugin {
    name: "md",
    description: "Transition table + variable/event summary (Markdown)",
    ext: "md",
    generate: generate_md,
    svg_tool: None,
    plan_render: None,
    render_svg: None,
    internal_render_svg: None,
    internal_renderer_name: None,
};
