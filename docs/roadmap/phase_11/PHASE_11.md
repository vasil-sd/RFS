# Phase 11: Source refactoring per the Phase 10 proposals (no functionality change)

## Goal
Execute the refactoring proposed in `SRC_STRUCTURE.md` and `TEST_STRUCTURE.md`:
rename, move and edit source files into the proposed directory trees while
keeping functionality byte-identical; then actualize both inventory documents
(their "Proposed" sections become the actual state, every identifier and path
updated), re-sync every docs<->source reference, move the two inventories into
`docs/llm/`, and codify in the protocol the rules for impact calculation (with
links to the inventories) and for keeping the inventories current on source
changes.

## Scope
- Execute the `SRC_STRUCTURE.md` proposal exactly: 36 file moves + 5 files
  absorbed as a directory's `mod.rs` (`codegen`, `info_format`, `diagram`,
  `rust`, `c`) + 1 merge (`src/diagrams/mod.rs` label helpers into
  `cli/commands/diagram/mod.rs`) + 6 new `mod.rs` files (utils, json, cli,
  cli/commands, lang, mc) + 1 new file (`src/utils/rng.rs`).
- The LCG consolidation (the proposal's only non-mechanical step): delete
  `specgen.rs`'s `Rng` and `codegen_test.rs`'s `Lcg`, create the single
  `utils::rng` generator (union of their methods + the two LCG constants as
  shared `pub const`s), and constant-lock the two GENERATED-runtime copies
  (`rust_mc.rs` FUZZ_RUNTIME string, `c_mc.rs` C seed function) so all four
  stay in one RNG family. Generated output stays byte-identical.
- Resolve the flagged cross-backend coupling (proposal risk notes): move
  `snake` / `rust_ident` / `rust_binding_ident` out of `backends/rust.rs` into
  a new neutral module `src/codegen/ident.rs`; the C emitters import from there.
- Execute the `TEST_STRUCTURE.md` proposal: three test-suite renames
  (`library_comparison.rs` -> `parser_comparison_test.rs`,
  `examples_test.rs` -> `example_corpus_test.rs`,
  `integration_example_test.rs` -> `endpoint_protocol_test.rs`); every other
  test file kept in place.
- Update ALL import sites lockstep with each move: `src/`, `src/bin/`,
  `tests/`, and the detached fuzz workspace (`fuzz/fuzz_targets/common.rs` +
  `ir_roundtrip.rs` import `rfs::specgen` by path - they must be edited in the
  same commit as the specgen move).
- Re-sync `//! Docs:` correspondence: `tests/dev_docs_manifest.txt` (42 of the
  71 paths - every moved file is manifest-listed; dev_docs_test's output is the
  authoritative update list) and EVERY backticked source-path reference in all
  `docs/**/*.md` files (109 distinct references at planning time).
- Actualize both inventory documents: rewrite every row identifier AND Depends
  entry to the new paths, convert "Proposed naming scheme" / "Proposed layout"
  into the actual sections, update the file-mapping tables to actual locations,
  reflect the LCG consolidation (delete the two old type rows + their method
  rows, add the `utils/rng.rs` rows, fix Depends), and re-verify with a fresh
  string-masked grep audit.
- Move `SRC_STRUCTURE.md` + `TEST_STRUCTURE.md` into `docs/llm/`; add protocol
  rules: (a) impact calculation - any source change must consult the two
  inventories' Depends columns (linked from PROTOCOL.md), and (b) docs<->source
  correspondence - every source-code change updates the affected inventory rows
  in the same commit.

## Inventory conventions after the refactor (binding for actualization)
- A file that becomes a directory's `mod.rs` is identified by its real path:
  `src/codegen/mod.rs::<item>`, `src/codegen/backends/rust/mod.rs::<item>`,
  etc. The merged diagram label helpers are identified as
  `src/cli/commands/diagram/mod.rs::<item>`.
- Module paths, not file names, drive imports: the zero-churn absorption
  (`codegen.rs` -> `codegen/mod.rs`) keeps module path `codegen`, so every
  existing `use rfs::codegen::*` site (incl. the detached fuzz workspace)
  survives untouched; every other move changes its module path and all import
  sites are updated in the same commit.

## Deliverables
- The refactored source tree exactly as proposed (+ `utils/rng.rs`,
  `codegen/ident.rs`), green at every task commit.
- Actualized `docs/llm/SRC_STRUCTURE.md` + `docs/llm/TEST_STRUCTURE.md`.
- Updated `tests/dev_docs_manifest.txt` and all docs source references.
- `PROTOCOL.md` (and `PRINCIPLES.md` where the doc-set rules live) carrying the
  impact-calculation and correspondence rules with links to the inventories.
- `CURRENT_STATE.md` row 11, `PHASE_11_STATUS.md`, TASKS.md entries.

## Exit criteria
- [ ] The source tree matches the proposals file-for-file: every moved/renamed
      file at its proposed path, nothing else moved; `git diff --stat` over the
      phase shows only source moves/edits, the two inventory docs, manifest,
      docs reference re-sync, protocol files, and roadmap bookkeeping.
- [ ] No functionality change: full verification green at close-out -
      `cargo test` (535 baseline incl. the determinism-pinning suites that make
      codegen output byte-identity a tested invariant + docs_md_test 5/5 +
      dev_docs_test 9/9), `cargo clippy --all-targets`, `cargo fmt --check`.
- [ ] Both inventories actualized: every row identifier and Depends entry uses
      the new paths; "Proposed" sections converted to actual; file-mapping
      tables show actual locations; the fresh string-masked grep audit is exact
      (zero per-file diffs) against the refactored tree.
- [ ] No stale source-path reference remains in any `docs/**/*.md` file
      (verified by a fresh grep for every old path); dev_docs_test 9/9 on the
      updated manifest.
- [ ] The inventories live at `docs/llm/SRC_STRUCTURE.md` +
      `docs/llm/TEST_STRUCTURE.md`; PROTOCOL.md carries the impact-calculation
      rule (linking both documents) and the correspondence rule (source change
      => inventory rows updated in the same commit).
- [ ] `CURRENT_STATE.md` row 11 -> COMPLETE with close-out date;
      `PHASE_11_STATUS.md` written; TASKS.md entries appended.

## Non-goals
- NO functionality, API, CLI, codegen-output, or test-behavior change of any
  kind - the refactor is moves/renames/imports + the LCG consolidation and the
  ident-module extraction, both proven output-neutral.
- No restructuring of `docs/dev/` or `docs/llm/` content beyond reference
  re-sync and the two inventory moves; no new docs besides protocol edits.
- No changes to `crates/*` internals (Cargo workspace constraint), to the
  Makefile-driven example projects, or to the fuzz workspace layout (its two
  specgen import lines are edited lockstep, nothing else).
- No bin renames (cargo alias targets - `.cargo/config.toml` untouched), no new
  dependencies, no Cargo.toml/workspace edits.
- No planning of Phase 12 in this phase (its `PHASE_12.md` description is
  written alongside this plan; its implementation plan comes later).
