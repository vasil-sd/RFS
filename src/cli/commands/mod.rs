//! Per-command modules of the `rfs` CLI: one module per subcommand that has
//! its own logic file (`test`, `gen_spec`, `mc_ref`, `info`). The `diagram`
//! and `info_format` clusters join this directory in Phase 11 task 7.
//! Docs: dev:code-map-cli-surface

pub mod gen_spec;
pub mod info;
pub mod mc_ref;
pub mod test;
