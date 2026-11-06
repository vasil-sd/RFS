//! CLI surface: shared command infrastructure (the level-filtered stderr
//! logger, the frontend conversion/validation logic behind `rfs parse` /
//! `rfs validate`, the external-tool resolver) + the per-command modules in
//! `commands/`.
//! Docs: dev:code-map-cli-surface

pub mod commands;
pub mod ext_tool;
pub mod frontend;
pub mod log;
