# RFS CLI Commands + Diagrams (Class A)

The command layer of the toolchain: per-command logic modules, the diagram
plugin system, and the shared external-tool infrastructure.

<a id="dev:cli-commands-overview"></a>
## Overview

TBD (task 12.1): dispatch from `src/main.rs` to the per-command modules -
`rfs info` (`src/info.rs::run_info`) + `rfs info format`
(`src/info_format.rs`), `rfs gen-spec` (`src/gen_spec_cmd.rs`, backed by
`src/specgen.rs::SpecGen`), `rfs test` (`src/test_cmd.rs`), `rfs mc-ref`
(`src/mc_ref_cmd.rs`), and the plugin-based `rfs diagram`
(`src/diagram.rs::DiagramPlugin`, format plugins in `src/diagrams/`) - plus
the shared external-tool resolution/invocation layer (`src/ext_tool.rs::ExtTool`).
