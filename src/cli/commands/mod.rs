//! Per-command modules of the `rfs` CLI: one module per subcommand that has
//! its own logic file (`test`, `gen_spec`, `mc_ref`, `info`) + the `diagram`
//! cluster (registry + the four format plugins, Phase 11 task 7). The
//! `info_format` cluster sits one level up at `crate::cli::info_format`.
//! Docs: dev:code-map-cli-surface

pub mod diagram;
pub mod gen_spec;
pub mod info;
pub mod mc_ref;
pub mod test;
