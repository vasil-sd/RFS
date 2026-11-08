# Phase 12, Task 1 - Implementation status

> Written once when all exit criteria of `PHASE_12_TASK_1.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

| Name | Description |
|---|---|
| `dev_tools/Cargo.toml` | crate manifest: package/bin `rfs-dev` v0.1.0, edition 2021, lib `rfs_dev`; deps exactly `syn = { version = "2", features = ["full"] }` (resolved 2.0.119) + `pulldown-cmark = "0.13"` (resolved 0.13.6) - the two declared exceptions, nothing else |
| `dev_tools/src/lib.rs` | crate-level doc comment only; no modules pre-declared (dead code until task 5) per the task file's Action 2 decision |
| `dev_tools/src/main.rs` | dispatch stub: the pinned 9-subcommand surface (`sync-check`, `sync-fix`, `context`, `edit-rename`, `edit-move`, `edit-replace-item`, `doc-section`, `inventory <add\|update\|remove\|audit>`, `calls <extract\|classify>`); unknown subcommand / missing or invalid subverb -> usage line + exit 2; every pinned subcommand -> pinned `rfs-dev <sub>: not implemented yet (Phase 12 task 5)` stderr + exit 2; no `unimplemented!()`/`todo!()`; 4 unit tests on the dispatch fn |
| `dev_tools/tests/dispatch_test.rs` | end-to-end CLI-contract pins: runs the real binary for all 13 pinned subcommand/subverb combinations (byte-exact stderr + exit 2) plus no-args, unknown-subcommand and invalid-subverb usage cases - 4 integration tests |

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| `Cargo.toml` | changed | `[workspace] members` gains `"dev_tools"` with a comment (explicit-list style preserved) - the planning decision: workspace member so `--workspace` test/clippy/fmt cover it automatically |

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `dev_tools/README.md` | new | H1 + role paragraph + the two section headers with placeholders: `## Analysis of tool calls patterns` (filled by task 3.5), `## Proposed tools` (filled by task 4) - the committed structure tasks 3.5/4 fill, not restructure |

## Other file changes

| Name | Operation | Description |
|---|---|---|
| `.gitignore` | changed | one entry: `dev_tools/analysis/calls.jsonl` (regenerable extractor output - plan Context decision; the script is committed, its output is not) |
| `Cargo.lock` | changed | +6 package entries: `rfs-dev`, `pulldown-cmark` 0.13.6 and its transitive tree (`bitflags` 2.13.2, `getopts` 0.2.24, `pulldown-cmark-escape` 0.11.0, `unicase` 2.10.0). `syn`/`proc-macro2` needed NO new entries - already locked by existing dependencies (syn 2.0.119 + 3.0.9 both pre-present) |
| `TASKS.md` | changed | task 1 entry appended |

## Changed functionality
- None in the `rfs` package: root-package test count is exactly 535/535 before and after (measured). Plain root `cargo build`/`cargo test` still select only the `rfs` package (workspace note unchanged).

## Removed functionality
- None.

## New functionality
- `rfs-dev` binary (all stubs): the complete Phase 12 CLI surface exists and is machine-pinned - every subcommand dispatches to a typed "not implemented yet" error (exit 2), usage errors print the pinned usage line. Task 4's proposal must match these names; task 5 replaces stub bodies without touching the surface.
- `dev_tools` as a workspace member: `cargo test --workspace` now covers it (567 = 559 + 8).

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|
| `dev_tools/src/main.rs` unit tests (4) | new | dispatch-level pins: no-args, unknown subcommand, grouped-subverb validation, all-13-stubs exit 2 |
| `dev_tools/tests/dispatch_test.rs` (4) | new | binary-level pins: byte-exact stub messages + exit codes for all 13 combinations; usage-line presence for the 6 error cases - the executable form of the fixed CLI contract |

## Tests run status

| Name | Status |
|---|---|
| `cargo build -p rfs-dev` | OK (first build fetched pulldown-cmark tree from crates.io; syn/proc-macro2 served from existing lock) |
| `cargo test -p rfs-dev` (8) | OK - 4 unit + 4 integration, all pass |
| `cargo test` root package (535) | OK - baseline UNCHANGED (the proof the rfs package is untouched) |
| `cargo test --workspace` (567) | OK - 559 previous + 8 dev_tools |
| `cargo clippy --all-targets` | OK - zero warnings mentioning rfs-dev; only the pre-existing rfs baseline warnings (21 frontend_test + 12 lib duplicates, unchanged) |
| `cargo fmt --check` | OK after one auto-fix (string-concat style in main.rs usage()) |
| `docs_md_test` (5) | OK after fixing two literal-triple-backtick prose occurrences (see Found problems) - all 379 repo docs pass the mdr-stack gate |

## Found problems/issues/mismatches
- The mdr doc-quality gate (`tests/docs_md_test.rs`) caught a real defect in the NEW task documentation: `PHASE_12_TASK_2.md` line 26 contained three consecutive backticks inside prose (the phrase "in a fenced code-block-marker block"), which opens an unclosed fenced code block under comrak's GFM parse. Fixed by rephrasing to "fenced code block"; the same pattern existed in `PHASE_12_PLAN.md`'s format table (line 18) and was fixed proactively. Lesson recorded for tasks 3-6: any future doc mentioning the commands.md fenced-payload form must say "fenced code block", never write three backticks in prose.
- Environment (not code): this sandbox instance had an EMPTY puppeteer cache, so `diagram_render_parity_test::parity_external_mmd_pinned_subset` failed on first full-suite run with "Could not find chrome-headless-shell (ver. 156.0.8078.47)". Restored with `npx puppeteer browsers install chrome-headless-shell` (the exact remedy recorded in previous sessions; TOOLCHAIN.md section 6 territory - mmdc is an optional external-renderer dependency). After the restore: 13/13 diagram parity tests pass, full root suite 535/535.

## Other important findings/information
- `syn` 2.0.119 and `proc-macro2` 1.0.107 were ALREADY in Cargo.lock (pulled by pre-existing dependencies) - task 5's syn-based tools add zero new lock entries; only pulldown-cmark's tree was new. The dependency-constraint exit criterion is therefore met with even less lock churn than planned.
- The dispatch stub's message format is `rfs-dev <sub> [subverb]: not implemented yet (Phase 12 task 5)` - grouped subcommands name the full pair (`inventory add`). Task 5 keeps this exact prefix convention when replacing bodies (the integration test asserts it byte-for-byte).
- Resolved dependency versions for the record: syn 2.0.119, pulldown-cmark 0.13.6, proc-macro2 1.0.107, bitflags 2.13.2, getopts 0.2.24, pulldown-cmark-escape 0.11.0, unicase 2.10.0.
