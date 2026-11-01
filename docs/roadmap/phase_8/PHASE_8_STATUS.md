# Phase 8 - Implementation status

> Written once when all exit criteria of `PHASE_8.md` are reached. Not rewritten
> afterwards; later fixes go into `TASKS.md` entries and follow-up phase plans.

## Added source files

None - documentation-only phase by design (PHASE_8.md non-goals): the only new
Rust file is the sync-gate suite, listed under Tests.

## Changed/removed source files

All 71 hand-written `.rs` files changed, each by exactly one added `//! Docs:`
module-doc pointer line (the code→doc traceability mechanism); no executable
code touched (verified: the full diff over `src/`, `crates/`, `fuzz/` contains
only doc-comment lines and blank lines).

| Name | Operation | Description |
|---|---|---|
| `src/` (49 files: 31 root + 5 `diagrams/` + 9 `backends/` + 4 `bin/`) | changed | one `//! Docs:` line per file naming its dev:/llm: fragments; at close-out, 5 of them (`codegen.rs`, `frontend.rs`, `json_rw.rs`, `printer.rs`, `smt_plan.rs`) gained a blank `//!` separator before the pointer (clippy `doc_lazy_continuation` fix - see Found problems) |
| `crates/rfs-build/` (2: `src/lib.rs`, `tests/integration.rs`) | changed | one `//! Docs:` line each (`dev:workspace-crates-overview`, `llm:workspace-crates`) |
| `crates/rfs-adapters/` (7: 5 `src/`, 2 `tests/`) | changed | one `//! Docs:` line each (same fragments) |
| `crates/examples/` (8: async_handler, sync_handler, fuzz_rust incl. their `build.rs` files) | changed | one `//! Docs:` line each (same fragments) |
| `fuzz/fuzz_targets/` (5: parser, validator, ir_roundtrip, codegen, common) | changed | one `//! Docs:` line each (`dev:toolchain-fuzz-overview`, `llm:toolchain-fuzz`) |

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `docs/dev/` (18 files, 8843 lines, 23 mermaid blocks, 84 `dev:` fragments) | new | Class A developer set: README index; ARCHITECTURE (pipeline overview + component/data-flow diagram + functionality hierarchy + relationship map); CODE_MAP (all 71 `.rs` files in four areas, both backends at equal depth); DEPENDENCIES (file-level graph split into per-area subgraphs + function-level graphs for parser/validator/codegen core/Rust backend/C backend/MC engine/monitor - 16 mermaid blocks); DECISIONS (rationale index for EVERY decision entry in DESIGN.md: section 9 subsections 9.1-9.11, section 19 bullets, D1-D10, D-P3-1..6, D-P4-1..14, D-P5-1..14, D-P6-1..11, D-P7-1..13 - each with a 2-4 line why + DESIGN.md link + implementing code); ALGORITHMS (parser construction, validation passes, IR normalization + codegen pipeline, MC engine exact/liveness/SMT, monitor compilation - each at L1 abstract / L2 structures+invariants / L3 implementation detail, four L1 mermaid flowcharts, every level linked to code and to REFERENCES); REFERENCES (parsing theory, automata theory, compiler construction, model checking, SMT/SAT literature index with one-line annotations + which RFS component uses each); 11 per-subsystem deep dives (parser, validator, ir_json_rw, codegen_core, backend_rust, backend_c, mc_engine, monitor_compiler, cli_commands, workspace_crates, toolchain_fuzz) |
| `docs/llm/` (14 files, 921 lines, 14 `llm:` fragments) | new | Class B LLM-assistant set: README (index + context-budget rules); TASK_MAP (functionality→files/functions for EVERY CLI command, subsystem and test suite - the "read this fragment for this task" index extending START.md section 6); CHANGE_IMPACT (17-row change-type→affected source files + tests + doc fragments table mirroring/extending BUILD.md section 5 at file granularity); 11 per-module compact cards, each within its machine-checked line budget (task-13.3 census: parser 53/120, validator 43/120, ir_json_rw 56/150, codegen_core 47/150, backend_rust 56/150, backend_c 58/150, mc_engine 56/150, monitor_compiler 54/120, cli_commands 77/200, workspace_crates 77/200, toolchain_fuzz 78/200) |
| `START.md` | changed | section 2 doc-map rows for docs/dev/ + docs/llm/ (never-fully rule, TASK_MAP locator); section 6 three new task→read rows (locate a feature / before editing source / deep-dive a subsystem) |
| `KB.md` | changed | section 1 Roadmap row refreshed + Developer-docs (Class A) + LLM-assistant docs (Class B) rows; section 8 Phase 8 bullet; Tests row re-measured at close-out (453/477/455 → 535/559/537, dev_docs_test named - the staleness task 14 deferred to this review) |
| `BUILD.md` | changed | section 3 root 526→535 + workspace 550→559 + new doc-sync-gate suite row; section 5 docs-sync checklist bullet (docs/dev/ or docs/llm/ or `//! Docs:` line changes → dev_docs_test; its output IS the update list) + baseline re-dated 2026-11-01 (535/559/537, generated 1683 unchanged, aggregate 267) |
| `docs/user/README.md` | changed | "Where things live" cross-links to the docs/dev/ + docs/llm/ README indexes |
| `docs/roadmap/INTRO.md` | changed | phase-state table row 8 → COMPLETE (close-out 2026-11-01) |
| `docs/roadmap/phase_8/PHASE_8.md`, `PHASE_8_PLAN.md`, `PHASE_8_TASK_{1..15}.md` | new | two-stage protocol artifacts: frozen phase description; plan with 51 subtask rows + 3 corrections; per-plan-unit task files created at execution (subtask sections with checked exit criteria + corrections tables - Phase 8's house pattern in place of per-task `_STATUS.md` files) |

## Other file changes

| Name | Operation | Description |
|---|---|---|
| `training/project_tooling/rfs_training.jsonl` | changed | 6 → 9 entries: the two documentation sets, the LLM-session workflow over Class B (TASK_MAP → fragments → CHANGE_IMPACT → tests), the sync gate's five checks + failure-output-is-the-update-list behavior; fence-free by design |
| `all_rfs_training.jsonl` | changed | aggregate re-pinned 264 → 267 lines (snippets unchanged: 138 valid + 128 invalid) |

## Changed functionality

None - documentation-only phase: zero changes to parser, validator, codegen or
MC algorithms, CLI behavior, IR schema, or generated output.

## Removed functionality

None.

## New functionality

- Bidirectional documentation traceability, machine-enforced:
  - doc→code: every technical statement in docs/dev/ + docs/llm/ carries a repo-rooted inline-code reference (`path/file.rs[::item]`, stable identifiers, never line numbers) that the gate re-resolves on every `cargo test`.
  - code→doc: every one of the 71 hand-written `.rs` files carries exactly one `//! Docs:` pointer line naming its fragments; scope driven by `tests/dev_docs_manifest.txt` with a completeness test (manifest == filesystem scan).
- Sync-gate suite `tests/dev_docs_test.rs` in plain `cargo test`: five checks (doc→code reference resolution, code→doc pointer presence, TASK_MAP coverage, no orphaned fragments via fragment ownership, Class B line budgets) + 6 pinned negative-control fixtures; its failure output IS the list of docs to update.
- Two documentation classes with a link-only relationship: Class A (`docs/dev/`, rich, diagram-heavy, rationale- and education-focused) and Class B (`docs/llm/`, compact, task-oriented, context-budgeted - stable `llm:<module>` fragment IDs, machine-checked per-file budgets, no content duplication with Class A).
- Documented LLM-session workflow: TASK_MAP fragment → read exactly the named fragments (grep anchor + sed range) → CHANGE_IMPACT before editing → listed tests + dev_docs_test after doc-affecting edits.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|
| `tests/dev_docs_test.rs` | new | the sync gate: 9 tests (main gate over the real tree, positive control, manifest==filesystem-scan completeness, 6 negative controls) + reusable check functions; 985 lines |
| `tests/fixtures/dev_docs/` (8 files) | new | pinned negative-control fixtures: broken ref (missing file + missing item), missing pointer, dangling pointer, orphan fragment, budget overflow, malformed manifest entries + good doc/code/llm-doc controls |
| `tests/dev_docs_manifest.txt` | new | code→doc pointer-scope driver: 71 entries (all hand-written `.rs`; the build-generated git-ignored `crates/examples/fuzz_rust/fuzz/fuzz_targets/arq.rs` is excluded - plan said 72, actual hand-written count is 71) |

## Tests run status

| Name | Status |
|---|---|
| `cargo build` | OK |
| `cargo test` (535) | OK |
| `cargo test --workspace` (559) | OK |
| `cargo test --features smt` (537) | OK |
| `cargo test-generated-code` (1683, 192 modules) | OK |
| `cargo test --test examples_test` (3; 50 valid + 13 invalid) | OK |
| training pipeline: extract + build + `cargo test-training` (5; aggregate 267 lines, snippets 138+128 unchanged) | OK |
| `cargo test --test dev_docs_test` (9) | OK |
| `cargo test --test docs_md_test` (5; renders all 23 new mermaid blocks) | OK |
| `cargo clippy --all-targets` (no errors; 46 warning lines / 43 unique - all pre-existing patterns in shared test modules and old source, zero in Phase 8's new files; the 5 phase-introduced `doc_lazy_continuation` warnings fixed at close-out) | OK |
| `cargo fmt --check` | OK |

## Found problems/issues/mismatches

- The mandatory `//! Docs:` line triggered clippy `doc_lazy_continuation` ("doc list item without indentation") in the 5 files whose module doc ended in a bullet list (`src/codegen.rs`, `src/frontend.rs`, `src/json_rw.rs`, `src/printer.rs`, `src/smt_plan.rs`) - fixed at close-out with a blank `//!` separator line before the pointer (doc-comment-only; gate + full suite re-verified). Earlier task reports' "no new clippy warnings" was inaccurate for these five files.
- Plan said 72 hand-written `.rs` files; the actual count is 71 - `crates/examples/fuzz_rust/fuzz/fuzz_targets/arq.rs` is build-generated (git-ignored, absent on a fresh checkout). Recorded as task 1.10 correction; manifest + completeness test use 71.
- KB.md section 1 "Tests" row carried Phase 6 counts (453/477/455) - pre-existing staleness that task 14 explicitly deferred to this close-out review; re-measured and updated here (535/559/537).

## Other important findings/information

- Phase 8 used the per-plan-unit task-file pattern: one `PHASE_8_TASK_N.md` per plan group, created at execution, with subtask sections carrying checked exit-criteria boxes + corrections tables - in place of per-task `_STATUS.md` files (protocol step m). All 15 plan units followed it; this file is the phase-level status.
- Fragment census: 98 stable anchors (84 `dev:` + 14 `llm:`) defined across the set; docs/dev/ 8843 lines with 23 mermaid blocks (all rendered through docs_md_test); docs/llm/ 921 lines, every card within budget.
- Backend parity applied to documentation: Rust and C backends covered with equal depth in CODE_MAP (task 3.2), DEPENDENCIES function-level graphs (tasks 4.3/4.4) and deep dives (task 9.2 vs 10.1+10.2 incl. explicit parity notes).
- The untracked `nix/` directory in the repo root belongs to an unrelated project (hipfire/ROCm dev shell; predates Phase 8 work) - left untouched, not committed.
