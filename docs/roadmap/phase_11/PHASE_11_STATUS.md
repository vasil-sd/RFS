# Phase 11 - Implementation status

> Written once when all exit criteria of `PHASE_11.md` are reached. Not rewritten
> afterwards; later fixes go into `TASKS.md` entries and follow-up phase plans.

## Added source files

| Name | Description |
|---|---|
| `src/utils/mod.rs`, `src/json/mod.rs`, `src/lang/mod.rs`, `src/cli/mod.rs`, `src/cli/commands/mod.rs`, `src/mc/mod.rs` | the six new module roots of the refactored tree (each carries its `//! Docs:` pointer to a CODE_MAP fragment) |
| `src/utils/rng.rs` | the single hand-rolled LCG generator (union method set of the deleted specgen `Rng` + codegen test-plan `Lcg`, plus `pub const LCG_MULT` / `LCG_ADD`) - the proposal's only non-mechanical step |
| `src/codegen/ident.rs` | neutral identifier-mangling module (`snake` / `rust_ident` / `rust_binding_ident` + `RUST_KEYWORDS`) extracted from the Rust backend - resolves the flagged cross-backend coupling (zero c/* imports from rust/) |

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| 43 files under `src/` | changed (renamed/moved) | the complete mapping of PHASE_11_PLAN.md Context executed: utils/hash, json/{json_rw,json_schema}, lang/{parser,error,ir,printer,validator,specgen}, codegen/{mod,fsm,test_plan,mc_concepts,monitor}, codegen/backends/{mod,rust/{mod,fsm,test,mc},c/{mod,fsm,test,mc}} + RUST.md/C.md moved with the code, cli/{log,frontend,ext_tool}, cli/commands/{test,gen_spec,mc_ref,info}, cli/commands/diagram/{mod,dot,plantuml,mermaid,tables}, cli/info_format/{mod,ebnf_grammar}, mc/{plan,smt_plan,smt_emit,ref_mc} |
| `src/diagrams/mod.rs` | removed | the phase's one genuine merge: five label-helper items moved verbatim into `src/cli/commands/diagram/mod.rs` |
| `src/lib.rs`, `src/main.rs`, `src/training.rs` | changed | pub-mod declarations + dispatch/import paths repointed (public API unchanged - crate-root re-exports preserved) |
| `src/bin/generate_code_tests.rs`, `src/bin/test_generated_code.rs` | changed | import paths repointed |
| `fuzz/fuzz_targets/{common,ir_roundtrip,parser,validator}.rs` | changed | the detached fuzz workspace edited lockstep (specgen move in the same commit as task 3; json_rw imports in task 2) - nothing else in fuzz/ touched |
| 14 files under `tests/` (+ `tests/common/prod_driver.rs`) | changed | import-path rewrites only; no test logic changed |
| `tests/dev_docs_manifest.txt` | changed | 42 moved paths + the new module roots - the manifest tracks the final tree (87 entries) |
| `tests/dev_docs_test.rs` | changed | `check_doc_code_refs` skips exactly the two inventory tables (plan correction 13a - they are machine-audited data tables, not navigational docs); no fn/type/file added, removed or renamed |
| `tests/fixtures/dev_docs/{good_doc,good_llm_doc}.md` | changed | positive-control fixtures re-pointed to track the refactored tree |

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `docs/llm/SRC_STRUCTURE.md` | changed (moved + actualized) | the source inventory: every row identifier + Depends rewritten to post-refactor paths, LCG rows swapped, proposal sections converted to actual state; moved from repo root with a gate-forced `llm-budget: 1710` header; fresh string-masked grep audit EXACT (936 fns / 196 types, zero per-file diffs) |
| `docs/llm/TEST_STRUCTURE.md` | changed (moved + actualized) | the test-tree inventory: three suite renames + 1794 stale source-path refs rewritten, proposal section converted; moved with a `llm-budget: 1287` header; audit EXACT (942 fns / 34 types) |
| `docs/roadmap/PROTOCOL.md` | changed | new section 4 (impact calculation - mandatory Depends-column consultation of the two inventories before scoping any source change) + section 5 (docs<->source correspondence - same-commit inventory row updates, string-masked audit as the check) |
| `docs/roadmap/PRINCIPLES.md` | changed | section-2 bullet naming the two protocol obligations |
| `docs/llm/README.md` | changed | two index-table rows for the moved inventories (user-directed) |
| 6 root docs (`BUILD`, `CODEGEN`, `DESIGN`, `KB`, `START`, `TOOLCHAIN`) + `docs/SEMANTICS.md` + 17 `docs/dev/*.md` + 13 `docs/llm/*.md` + `docs/user/MIGRATION.md` + `ide/README.md` | changed | docs<->source reference re-sync: every backticked source-path reference rewritten to the new paths (per-task incremental re-sync per correction 1a + the task-10 final sweep); zero old-path hits repo-wide |
| `docs/roadmap/phase_11/*` (29 files) | new | phase description, plan (+ corrections 1a/8a/13a), 14 task files (+ 3 correction tables), 14 status files (tasks 11-12's backfilled) |
| `docs/roadmap/CURRENT_STATE.md` | changed | row 11 -> COMPLETE (this close-out) |

## Other file changes

| Name | Operation | Description |
|---|---|---|
| `TASKS.md` | changed | one entry per executed task (13) + this close-out - the Phase 10 gap (tasks committed without entries) did not repeat |
| `docs/roadmap/phase_12/PHASE_12.md` | new | written alongside the Phase 11 plan by design (PHASE_11.md non-goals); its implementation plan comes later |

## Changed functionality
- None. The refactor is moves/renames/import-edits + the LCG consolidation and
  the ident extraction, both proven output-neutral: the determinism-pinning
  suites inside `cargo test` make codegen byte-identity a tested invariant,
  and all 535 tests pass unchanged on the final tree.

## Removed functionality
- None. (The specgen `Rng` and codegen test-plan `Lcg` structs were deleted as
  types, not behavior - every call site now uses `utils::rng`, and the two
  generated-runtime RNG copies are constant-locked to the same shared consts.)

## New functionality
- No user-visible behavior. Structural/process additions only:
  - the refactored directory tree exactly as proposed (src/ root is now exactly
    `{lib.rs, main.rs, training.rs}`);
  - one RNG family by construction: toolchain-side `utils::rng` + both
    generated runtimes interpolating the same `pub const`s;
  - PROTOCOL.md sections 4-5: impact calculation via the inventories' Depends
    columns + the same-commit correspondence obligation, with the inventories
    as their living docs/llm/ home.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|
| `tests/parser_comparison_test.rs`, `tests/example_corpus_test.rs`, `tests/endpoint_protocol_test.rs` | changed (renamed) | the TEST_STRUCTURE.md proposal: three suite renames, byte-identical content, cargo auto-discovery (no manifest edits) |
| 15 test files | changed | import-path rewrites only - no test logic, names, or expectations touched |
| `tests/dev_docs_test.rs` | changed | inventory exclusion in `check_doc_code_refs` (correction 13a); all 9 tests + negative controls intact |

## Tests run status

| Name | Status |
|---|---|
| `cargo test` (535) | OK - exact documented baseline on the final tree |
| `cargo test-generated-code` (1683, 192 modules) | OK - exact baseline; generated output byte-identical end-to-end |
| `cargo clippy --all-targets` | OK - 46 warning lines / 35 unique, the documented pre-existing baseline (zero in any file this phase added; correction 8a restored the baseline after task 8's mc/mod.rs doc comment) |
| `cargo fmt --check` | OK - clean |
| `docs_md_test` (5) | OK - every repo .md parses, mermaid renders, relative links resolve (incl. the inventories at their new home + PROTOCOL.md's new links) |
| `dev_docs_test` (9) | OK - manifest matches the final tree, budgets hold, no orphans |

## Found problems/issues/mismatches
- Plan correction 1a: the per-task green gate forced incremental docs<->source
  re-sync (task 1's move broke dev_docs_test immediately); task 10 re-scoped as
  the final verification sweep.
- Plan correction 8a: task 8's new `src/mc/mod.rs` doc comment tripped
  clippy::doc_list_item_no_indentation (4 lines) - caught only at close-out
  because task 8 verified `cargo build`, not clippy; rephrased, baseline
  restored.
- Plan correction 13a: moving the inventories into docs/llm/ broke dev_docs_test
  in two unforeseen ways (Class B budget headers + 27 non-navigational
  backticked spans under reference resolution); fixed with one header line per
  inventory + a documented two-path exclusion.
- Tasks 11-12 committed without their STATUS files (PROTOCOL.md section 3 step m
  gap) - backfilled in a dedicated commit before task 13 executed.
- Pre-existing environment failure: mmdc lost its chrome-headless-shell
  (puppeteer cache) mid-phase; restored the browser cache (twice - the same fix
  as task 6's). Unrelated to any phase change.

## Other important findings/information
- The src/ root is now exactly `{lib.rs, main.rs, training.rs}` - the complete
  move set is done and nothing else lives at the root.
- Both inventories are audit-exact against the refactored tree (zero per-file
  diffs) and are now the protocol-mandated impact-analysis substrate
  (PROTOCOL.md sections 4-5); keep them current per the correspondence rule.
- The detached fuzz workspace was edited lockstep in-commit (tasks 2/3) - it is
  a separate cargo project, so nothing else would have caught a break between
  commits.
- Phase 12 (`docs/roadmap/phase_12/PHASE_12.md`) is DESCRIBED only; its
  implementation plan is to be defined later per CURRENT_STATE.md.
