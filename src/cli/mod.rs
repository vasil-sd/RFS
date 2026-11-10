//! CLI surface: shared command infrastructure (the level-filtered stderr
//! logger, the conversion/format utilities in `utils`, the external-tool
//! resolver) + one plugin per top-level `rfs <command>` in `commands/`.
//! Docs: dev:code-map-cli-surface

pub mod args;
pub mod commands;
pub mod ext_tool;
pub mod info_format;
pub mod log;
pub mod utils;
