# Phase 11, Task 3 - Implementation status

> Written once when all exit criteria of `PHASE_11_TASK_3.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

| Name | Description |
|---|---|
| `src/lang/mod.rs` | module root of the language core (error, ir, parser, printer, specgen, validator) + //! Docs: pointer to dev:code-map-lang |

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| `src/{parser,error,ir,printer,validator,specgen}.rs` | removed (moved) | git mv to src/lang/ - error.rs and ir.rs byte-identical; parser/printer/validator/specgen carry only import-line rewrites (their own sibling references + fully-qualified paths in inline tests + one doc-comment link) |
| `src/lib.rs` | changed | six `pub mod` declarations replaced by `pub mod lang;`; the four crate-root re-exports updated to their new paths (`pub use lang::error::{ParseError, Span};`, `pub use lang::ir::*;`, `pub use lang::parser::{parse_file, parse_source};`, `pub use lang::printer::print_ir;`) - the public API surface (rfs::Ir, rfs::parse_source, ...) is unchanged |
| ~20 source files with import sites | changed | scripted rewrites of all six modules in both `crate::` and `rfs::` forms across src/, tests/, fuzz/ (ir's 576 incoming edges included); bare-import forms (`use crate::printer;` in frontend.rs + two inline test modules, `use crate::validator;` in specgen) fixed individually |
| `tests/{frontend_test,training_test,examples_test}.rs` | changed | grouped root imports of the printer MODULE by name (`use rfs::{..., printer, ...}`) updated to `rfs::lang::printer` - module names live at their new paths; item re-exports stay at the root |
| `fuzz/fuzz_targets/common.rs` | changed | specgen lockstep: `use rfs::specgen::{Bounds, SpecGen};` -> `rfs::lang::specgen::...` + doc-comment mention (separate cargo project - edited in this commit) |
| `fuzz/fuzz_targets/ir_roundtrip.rs` | changed | specgen lockstep: fully-qualified `rfs::specgen::mutate(...)` -> `rfs::lang::specgen::mutate(...)` |
| `tests/ref_mc_diff_test.rs`, `tests/toolchain_fuzz_test.rs` | changed | specgen lockstep: fully-qualified SpecGen/Bounds call paths + doc-comment mentions |

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `tests/dev_docs_manifest.txt` | changed | six paths -> src/lang/...; new entry src/lang/mod.rs (sorted position) |
| `docs/dev/CODE_MAP.md` | changed | NEW fragment dev:code-map-lang + "Language core (src/lang/) (7 files)" section (all six files with descriptions carried over, specgen's stale "deterministic Rng" wording corrected to the shared utils::rng); Frontend area 10 -> 5 files (Root 6 -> 4, Parser 2 -> 1, the now-empty Validator+IR subsection removed); specgen entry removed from the CLI section; utils section count corrected 2 -> 3 |
| `docs/dev/*.md`, `docs/llm/*.md`, `docs/user/*.md` (all files with refs) | changed | every `src/{parser,error,ir,printer,validator,specgen}.rs[::item]` live-doc reference rewritten to the src/lang/... paths (per correction 1a - incremental, in this commit); roadmap history untouched by design |
| `docs/llm/TASK_MAP.md` | changed | file-list entry for src/lang/mod.rs (gate coverage requirement) + path rewrites |
| `tests/fixtures/dev_docs/good_doc.md` | changed | the POSITIVE-CONTROL fixture's doc->code refs (`src/parser.rs`, `src/ir.rs::Ir`) updated to the new paths - the fixtures must track the real tree or the gate's positive control fails |

## Other file changes

| Name | Operation | Description |
|---|---|---|

## Changed functionality

None - pure move + import-path rewrite; 535/535 green. The crate-root re-exports keep rfs::Ir / rfs::parse_source / rfs::print_ir / rfs::{ParseError, Span} exactly as before.

## Removed functionality

None.

## New functionality

- `src/lang/` directory: the language core pipeline (parse -> IR -> validate ->
  print) plus its spec generator in one place; all intra-group edges stay inside
  the directory.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|

## Tests run status

| Name | Status |
|---|---|
| `cargo build` | OK (after fixing lib.rs re-exports + bare import forms) |
| `cargo test` (full) | OK - 535 passed / 0 failed, exactly the documented baseline |
| `cargo test --test dev_docs_test` (9) | OK - green after the incremental re-sync + the positive-control fixture update |

## Found problems/issues/mismatches

- lib.rs's crate-root RE-EXPORTS (`pub use ir::*;` etc.) were not anticipated by
  the task file's "scripted use-path rewrite" - they are bare module names, not
  `crate::x::` paths, and broke the build first. Fixed by pointing them at
  lang::; the public API is unchanged.
- Three test binaries imported the printer MODULE by name from the root
  (`use rfs::{..., printer, ...}`) - a form neither the script nor the task file
  listed. Updated to rfs::lang::printer (modules live at their new paths; item
  re-exports stay at the root).
- The dev_docs positive-control FIXTURE (tests/fixtures/dev_docs/good_doc.md)
  references real source paths and must track the tree - it failed the gate
  until its two refs were updated. Future move tasks must check the fixtures.

## Other important findings/information

- The specgen lockstep's actual scope was four files (fuzz common + ir_roundtrip,
  tests ref_mc_diff + toolchain_fuzz) - prod_driver.rs no longer references
  specgen at all after task 1 renamed its only use (Rng) to utils::rng.
- Sibling imports inside lang/ were rewritten to absolute crate::lang:: forms by
  the same script (no relative-path style introduced).
