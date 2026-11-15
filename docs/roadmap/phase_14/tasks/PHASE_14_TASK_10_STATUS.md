# Phase 14, Task 10 - Implementation status

> Written once when all exit criteria of `PHASE_14_TASK_10.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

| Name | Description |
|---|---|

(none.)

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|

(none - training-data-only task.)

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `training/frontend_cli/rfs_training.jsonl` | changed (11 entries) | 10 prefix-swaps only (lines 3, 6, 7, 8, 9, 10, 11, 16, 17, 18 - parse/info/validate/test/diagram teaching; the `--out FILE.svg` mentions in the diagram entries are that command's UNCHANGED exact-path flag and were left alone). Line 15 (mc-ref vs generated binary) got a codegen-semantics update: `--out mc.rs` -> `--out mc` with the resulting names (mc-fsm.rs + mc-mc.rs) noted. |
| `training/codegen_rust/rfs_training.jsonl` | changed (5 entries) | Lines 16, 17: prefix-swaps only (--no-std and --with-protocol-fsm teaching). Line 13 ("How do I use the rfs codegen CLI?"): full surface rewrite - examples to spec-stem/BASE form, the artifact-model rule (test/mc REFERENCE the fsm file - Rust include!, C header pair - instead of duplicating it), the output-naming rule (spec-stem default, --out BASE, per-artifact flags, --out - stdout mode), and the file-layout sentence moved to src/codegen/mod.rs + src/cli/commands/codegen.rs. Line 19 (counterexample JSON): `--out mc.rs` -> `--out safety_mc`, rustc line -> safety_mc-mc.rs. Line 5 (simple FSM anatomy): command to spec-stem form + two stale pre-Phase-13 paths fixed (src/backends/rust_fsm.rs -> src/codegen/backends/rust/fsm.rs). |
| `training/design/rfs_training.jsonl` | changed (1 entry - found by the audit, NOT in the task's occurrence map) | Line 13 (artifact provenance header): both demonstrated commands to BASE form (`--out /tmp/p` writing /tmp/p-fsm.rs and /tmp/p-fsm.h + /tmp/p-fsm.c, names commented) and the drift-check procedure block updated (`--out /tmp/new` -> diff /tmp/new-fsm.rs). The "real output" claim was re-verified by running both commands - line-1 headers match byte-for-byte. |
| `training/project_tooling/rfs_training.jsonl` | changed (2 entries) | Line 2 (implementation status): prefix-swap + the output-model sentence rewritten ([--out FILE] "self-contained single files" -> spec-stem/BASE naming + one-fsm-file-per-invocation reference model). Line 4 (rfs-build): compile() now documented with per-slot flags (--out-fsm-code [+ --out-fsm-header C], one cargo:include per generated file) and .test_command()'s "extra file" wording -> fuzz-target slot. |
| `training/*/snippets/` | regenerated (no changes) | The trio re-extracted all snippets; every snippet file is byte-identical because the edits touched shell-command teaching only - no .rfs block changed. 138 valid / 128 invalid, same as before. |
| `all_rfs_training.jsonl` | regenerated | 267 lines - UNCHANGED from the current baseline (the task file's "264" predates Phase 8 task 15's 264->267 re-pin, commit 627ee4f). The diff is exactly the 19 updated entry lines. |
| `docs/roadmap/CURRENT_STATE.md` | changed | Phase-14 row: tasks 1-10 complete, task 11 pending. |

## Other file changes

| Name | Operation | Description |
|---|---|---|

(none.)

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|

(none - the training tests validate format + counts and pass unmodified.)

## Tests run status

| Name | Status |
|---|---|
| `grep -rn "cargo run --" training/ --include="*.jsonl"` | OK - zero hits (was 37 occurrences across 14 entries) |
| old-model audit greps (exact-file `--out X.rs/.c` in codegen contexts, "self-contained single file", "extra file") | OK - zero hits after the updates |
| `cargo extract-training-snippets` | OK - 138 valid / 128 invalid snippets (unchanged) |
| `cargo build-training` | OK - aggregated 267 examples (baseline unchanged) |
| `cargo test-training` | OK - training_test.rs 4 passed / 0 failed |

## Found problems/issues/mismatches
- **The task file's occurrence map was incomplete**: it listed the 14 entries carrying `cargo run --`, but the codegen-semantics audit (exit criterion 2) found FIVE more entries teaching the old output model without the cargo prefix - codegen_rust:5 and :13, design:13, project_tooling:2 and :4, frontend_cli:15. All updated; the design/ category was not in the task's affected-files table at all.
- **Baseline count drift**: the task file says the aggregate is "264 lines at baseline"; it is 267 (Phase 8 task 15 re-pinned 264->267 on 2026-11-0x, commit 627ee4f). The exit criterion's "unchanged from baseline" holds against the TRUE current baseline (267 before and after).

## Other important findings/information
- **Prefix-swaps only (12 entries)**: frontend_cli lines 3, 6, 7, 8, 9, 10, 11, 16, 17, 18; codegen_rust lines 16, 17. Arguments unchanged; no teaching point touched.
- **Codegen-semantics updates (7 entries - the ones a future reviewer should re-check against the final CLI)**: codegen_rust:13 (full CLI surface), codegen_rust:5 (spec-stem command + path fixes), codegen_rust:19 (--out safety_mc), design:13 (BASE-form provenance commands + drift-check block, re-verified by execution), project_tooling:2 (output-model sentence), project_tooling:4 (rfs-build per-slot flags), frontend_cli:15 (--out mc).
- **Pre-existing path drift left untouched (out of scope)**: codegen_rust lines 12 and 15 still name pre-Phase-13 layout paths (src/codegen.rs, src/backends/rust.rs, src/backends/c.rs) in architecture teaching that carries NO --out semantics; project_tooling:2's status summary likewise describes the old module layout. These predate this phase and are not output-model teaching - flagged here so a future doc-refresh task can pick them up.
- Every jsonl line was re-validated for the 2-message shape + `< think>` block after editing (the training tests also enforce this).
