//! Per-command plugins of the `rfs` CLI: one module per top-level command
//! (`parse`, `validate`, `codegen`, `info`, `test`, `diagram`, `mc_ref`,
//! `gen_spec`), each exposing a `run_cli(args) -> ExitCode` entry point. The
//! `diagram` cluster additionally holds the four format sub-plugins; the
//! `info_format` cluster sits one level up at `crate::cli::info_format`.
//!
//! [`ALL_COMMANDS`] is the command registry (mirrors the diagram-format and
//! codegen-backend registries): `main.rs` dispatches by name through [`find`].
//! Adding a top-level command = one module + one registry line.
//! Docs: dev:code-map-cli-surface, dev:cli-commands-dispatch

use std::process::ExitCode;

pub mod codegen;
pub mod diagram;
pub mod gen_spec;
pub mod info;
pub mod mc_ref;
pub mod parse;
pub mod test;
pub mod validate;

/// A registered top-level `rfs` command (plugin): static data + entry point.
pub struct CommandPlugin {
    /// CLI name: `parse`, `validate`, ...
    pub name: &'static str,
    /// One-line summary (for a future `rfs --list`).
    pub summary: &'static str,
    /// The plugin entry point (argument parsing + dispatch to the engine).
    pub run: fn(&[String]) -> ExitCode,
}

/// The command registry - one entry per top-level command, in stable order.
pub static ALL_COMMANDS: &[CommandPlugin] = &[
    CommandPlugin {
        name: "parse",
        summary: "convert RFS/JSON files through the IR",
        run: parse::run_cli,
    },
    CommandPlugin {
        name: "validate",
        summary: "parse + semantically validate RFS/JSON files",
        run: validate::run_cli,
    },
    CommandPlugin {
        name: "info",
        summary: "static spec introspection (text or --json)",
        run: info::run_cli,
    },
    CommandPlugin {
        name: "test",
        summary: "generate + compile + run the selected test suites",
        run: test::run_cli,
    },
    CommandPlugin {
        name: "diagram",
        summary: "state-diagram + table formats as plugins",
        run: diagram::run_cli,
    },
    CommandPlugin {
        name: "codegen",
        summary: "target-language backend registry (production/test/mc)",
        run: codegen::run_cli,
    },
    CommandPlugin {
        name: "mc-ref",
        summary: "in-process reference model checker",
        run: mc_ref::run_cli,
    },
    CommandPlugin {
        name: "gen-spec",
        summary: "write a random or base-mutated spec",
        run: gen_spec::run_cli,
    },
];

/// Look up a command plugin by its CLI name.
pub fn find(name: &str) -> Option<&'static CommandPlugin> {
    ALL_COMMANDS.iter().find(|c| c.name == name)
}
