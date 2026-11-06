# Phase 11, Task 6 - Implementation status

> Written once when all exit criteria of `PHASE_11_TASK_6.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

| Name | Description |
|---|---|
| `src/cli/mod.rs` | module root of the CLI surface (`pub mod commands; ext_tool; frontend; log;`) + //! Docs: pointer to the new dev:code-map-cli-surface fragment |
| `src/cli/commands/mod.rs` | module root of the per-command modules (`pub mod gen_spec; info; mc_ref; test;`; diagram + info_format join in task 7) + //! Docs: pointer to dev:code-map-cli-surface |

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| `src/cli_log.rs` | removed (moved) | git mv to `src/cli/log.rs`, contents byte-identical (0 changed lines) |
| `src/frontend.rs` | removed (moved) | git mv to `src/cli/frontend.rs`; zero use-line edits needed (its imports were already final after task 3) - the only content delta is cargo-fmt line-wrapping of three long `crate::json::json_rw::` lines (task-2 path-length drift) |
| `src/ext_tool.rs` | removed (moved) | git mv to `src/cli/ext_tool.rs`, contents byte-identical (0 changed lines) |
| `src/test_cmd.rs` | removed (moved) | git mv to `src/cli/commands/test.rs`; 4 use-line edits: cli_log -> cli::log, ext_tool -> cli::ext_tool, two fully-qualified crate::frontend refs -> crate::cli::frontend |
| `src/gen_spec_cmd.rs` | removed (moved) | git mv to `src/cli/commands/gen_spec.rs`; 2 use-line edits: `use crate::frontend::{self, Format}` -> cli::frontend (the self binding keeps the unqualified call site working), cli_log -> cli::log in the test module |
| `src/mc_ref_cmd.rs` | removed (moved) | git mv to `src/cli/commands/mc_ref.rs`; same two edit forms as gen_spec (its `crate::ref_mc` import stays - task 8) |
| `src/info.rs` | removed (moved) | git mv to `src/cli/commands/info.rs`; 2 fully-qualified crate::frontend refs -> crate::cli::frontend + fmt import re-ordering |
| `src/lib.rs` | changed | seven `pub mod` lines (cli_log, ext_tool, frontend, gen_spec_cmd, info, mc_ref_cmd, test_cmd) replaced by `pub mod cli;` (alphabetical position); the crate-root item re-export repointed `pub use frontend::{...}` -> `pub use cli::frontend::{...}` - the public API surface (rfs::Format, rfs::run_validate, ...) is unchanged |
| `src/main.rs` | changed | grouped root import aliased (`cli::frontend as frontend` - the five unqualified call sites stay untouched); dispatch repointed: mc-ref/gen-spec -> rfs::cli::commands::{mc_ref,gen_spec}::run_cli, info -> rfs::cli::commands::info::, test_cmd refs (incl. doc comment) -> rfs::cli::commands::test::; three test-module `use rfs::cli_log::{LogLevel, Logger}` -> rfs::cli::log |
| `src/diagram.rs`, `src/diagrams/{mod,dot,plantuml,mermaid,tables}.rs` | changed | import lines only (these files move in task 7): ext_tool imports/fully-qualified refs + doc comments -> cli::ext_tool; crate::info::render_* refs -> crate::cli::commands::info::; frontend refs in diagram.rs -> cli::frontend |
| `src/info_format.rs` | changed | two doc-comment prose mentions of the frontend module/file repointed (no import change - it never imported frontend) |
| `tests/frontend_test.rs` | changed | grouped root import aliased (`cli::frontend as frontend`); rfs::ext_tool refs -> cli::ext_tool; 15x rfs::mc_ref_cmd refs + the gen_spec_cmd ref + 3 rfs::info refs repointed to rfs::cli::commands::* |
| `tests/info_format_test.rs` | changed | 8x rfs::frontend refs (run_validate_structured / render_validate_json / Format) -> rfs::cli::frontend; the rfs::info ref -> rfs::cli::commands::info |
| ~20 further .rs files | changed | cargo fmt normalization only (path-length drift from tasks 1-5: line wraps + import sort order in rust/mc.rs, c/mc.rs, test_plan.rs, mc_plan.rs, lang/printer.rs, specgen.rs, json_schema.rs, codegen_test.rs, backend_parity_test.rs, ref_mc_diff_test.rs, training_test.rs, both bins, ...) - no semantic change |
| `src/codegen/backends/rust/mod.rs` | changed | unused `rust_binding_ident` import removed (task-5 leftover; only fsm.rs uses it) - restores the documented clippy baseline |

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|
| `tests/dev_docs_manifest.txt` | changed | seven old paths replaced by the nine new sorted entries (85 -> 87 entries; set-compared by the gate) |
| `docs/dev/CODE_MAP.md` | changed | NEW fragment dev:code-map-cli-surface + "CLI surface (`src/cli/`) (9 files)" section (all nine files, descriptions carried over); old "Frontend area" renamed "Root area - crate root + remaining top-level leaves (3 files)" (Root 4 -> 2; lib.rs bullet's module count corrected 30 -> 14 root modules); old "CLI + diagrams area (18)" trimmed to "Diagrams + bins area (12 files)" with the pre-existing "CLI commands (7)"-with-6-bullets count bug fixed to (1) |
| `docs/llm/TASK_MAP.md` | changed | Map-section path rewrites (7 refs); Coverage: the seven scattered bullets replaced by one grouped 9-entry block after src/main.rs (incl. entries for both new mod.rs files - gate coverage requirement); stale "all 71 manifest files" header corrected to 87 |
| `docs/dev/{cli_commands,ARCHITECTURE,DECISIONS,ir_json_rw,mc_engine,validator,DEPENDENCIES,ALGORITHMS}.md`, `docs/llm/{cli_commands,CHANGE_IMPACT}.md` | changed | every backticked reference to the seven old paths rewritten to the new ones (200 refs total - cli_commands.md alone carries 145) |
| `KB.md`, `DESIGN.md`, `BUILD.md`, `CODEGEN.md`, `START.md` | changed | ROOT-LEVEL DOCS - missed by mistake in tasks 1-5, included per user direction: all task-6 path refs rewritten PLUS the retroactive sweep of tasks 1-5 stale refs (KB 56+3, DESIGN 30+5, BUILD 3+1, CODEGEN 22+6 scripted; START.md's section-3 source tree block restructured to the directory layout + its doc-map/task-table refs fixed). Directory-level mentions (`src/backends/<lang>*.rs` globs, RUST.md/C.md pointers, CODEGEN.md's add-a-backend checklist) updated to the nested form. TASKS.md history + SRC_STRUCTURE/TEST_STRUCTURE inventories untouched by design (tasks 11-12) |

## Other file changes

| Name | Operation | Description |
|---|---|
| (none) | - | no examples/training/build-file changes; `.cargo/config.toml` aliases target bin names, untouched |

## Changed functionality

None - pure moves/renames + import-path rewrites; the crate-root item re-exports keep rfs::Format / rfs::run_validate / rfs::run_conversion / ... exactly as before. Codegen output byte-identical (determinism-pinning suites green).

## Removed functionality

None.

## New functionality

- `src/cli/` directory: the CLI surface in one place - shared infrastructure (log, frontend, ext_tool) at the root, per-command modules under `commands/` with the `_cmd` suffixes dropped per proposal rule 2. Task 7 nests the diagram + info_format clusters under it.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|
| (none) | - | import-path edits only in tests/frontend_test.rs + tests/info_format_test.rs; no test logic touched, no suites added/removed |

## Tests run status

| Name | Status |
|---|---|
| `cargo build` | OK (warning-free after the unused-import trim) |
| `cargo test` (full) | OK - 535 passed / 0 failed, exactly the documented baseline (all 31 test binaries green; determinism-pinning suites prove codegen output byte-identity) |
| `cargo test --test dev_docs_test` (9) | OK - green after the manifest + CODE_MAP + TASK_MAP re-sync |
| `cargo fmt --check` | OK - clean after the one-time normalization of tasks 1-5 path-length drift |

## Found problems/issues/mismatches

- ROOT DOCS MISSED IN TASKS 1-5 (user-confirmed mistake): KB.md/DESIGN.md/BUILD.md/CODEGEN.md/START.md still carried every pre-task-6 path. Fixed here with the retroactive sweep; tasks 7-9 must keep updating root docs in-commit as well.
- Task 5's ident extraction left an unused `rust_binding_ident` import in rust/mod.rs - a clippy warning line against the documented 46/35 baseline, invisible to the per-task build+test gate. Trimmed here.
- Pre-existing doc count bugs: CODE_MAP "CLI commands (7)" with 6 bullets; TASK_MAP "all 71 manifest files" with 85 entries. Both fixed.
- `diagram_render_parity_test::parity_external_mmd_pinned_subset` failed on the committed tree: mmdc present but its puppeteer browser cache empty (chrome-headless-shell 156.0.8078.47 missing). Environment fix: ran mmdc's bundled `puppeteer/install.mjs` (downloads into the sandbox puppeteer cache) - suite then 13/13. Not caused by this task.

## Other important findings/information

- The aliased-import pattern (`use rfs::{cli::frontend as frontend, ...}` in main.rs + tests/frontend_test.rs; `use crate::cli::frontend::{self, Format}` in gen_spec/mc_ref) kept every unqualified call site byte-identical - only the import lines moved.
- No fuzz/ or crates/ lockstep was needed: zero references to any of the seven modules there (grep-verified before the move).
- The manifest is compared as a SET by dev_docs_test (BTreeSet) - its group-separator blank lines and non-sorted groups are fine; the nine new entries were inserted at their alphabetical position regardless.
