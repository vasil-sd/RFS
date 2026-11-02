# Phase 9, Task 7 - Implementation status

> Written once when all exit criteria of `PHASE_9_TASK_7.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

(none - docs-only task)

| Name | Description |
|---|---|

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `docs/dev/README.md` | changed | Index-table File column (7 sibling names) + body mentions converted to same-dir links; the Class A index is now fully clickable |
| `docs/dev/DEPENDENCIES.md` | changed | 2 mentions -> links (`../roadmap/PRINCIPLES.md`, `../roadmap/phase_3/PHASE_3_PLAN.md`) |
| `docs/dev/REFERENCES.md` | changed | 4 sibling-doc mentions -> same-dir links (ALGORITHMS x3, DESIGN) |
| `docs/dev/ALGORITHMS.md` | changed | 5 plain mentions -> links; the existing REFERENCES.md#anchor links stayed byte-identical (masked during conversion) |
| `docs/dev/backend_c.md` | changed | 16 mentions -> links incl. `[C.md](../../src/backends/C.md)` x2 and repo-rooted docs/dev/... forms converted to same-dir names |
| `docs/dev/backend_rust.md` | changed | 12 mentions -> links incl. `[RUST.md](../../src/backends/RUST.md)` x2 |
| `docs/dev/cli_commands.md` | changed | 8 mentions -> links; the 3 repo-rooted docs/dev/codegen_core.md forms became same-dir codegen_core.md |
| `docs/dev/codegen_core.md` | changed | 11 mentions -> links (CODEGEN x5, siblings, root docs) |
| `docs/dev/ir_json_rw.md` | changed | 8 mentions -> links incl. `[MIGRATION.md](../user/MIGRATION.md)` x2 |
| `docs/dev/mc_engine.md` | changed | 7 plain mentions -> links; the existing REFERENCES.md#anchor link untouched |
| `docs/dev/monitor_compiler.md` | changed | 6 plain mentions -> links (backend_rust/backend_c sibling refs); existing anchor links untouched |
| `docs/dev/parser.md` | changed | 14 mentions -> links (DESIGN x6, SEMANTICS, START, siblings) |
| `docs/dev/toolchain_fuzz.md` | changed | 4 mentions -> links; the task-5-corrected "[KB.md](../../KB.md) section 1" pointers included |
| `docs/dev/validator.md` | changed | 8 mentions -> links (DESIGN x2, SEMANTICS, siblings, root docs) |

Unchanged (zero mentions - git-verified): `docs/dev/ARCHITECTURE.md`,
`docs/dev/CODE_MAP.md`, `docs/dev/workspace_crates.md`.

## Other file changes

| Name | Operation | Description |
|---|---|---|
| `docs/roadmap/phase_9/tasks/PHASE_9_TASK_7.md` | changed | Context path rules updated BEFORE execution (protocol step e) with task 6's corrected root-doc depth (`../../` for root docs, `../` for docs/-level targets) and the backtick-unwrap rule |

## Changed functionality

- None (no toolchain behavior changed).

## Removed functionality

- None - plain-text document references replaced by equivalent clickable links.

## New functionality

- One-click navigation across the whole Class A set: every cross-document
  reference in the 14 converted files resolves to its target, uniform with
  task 6's DECISIONS.md conversion (same link style, same depth rules).

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|

## Tests run status

| Name | Status |
|---|---|
| `cargo test --test dev_docs_test` (9) | OK (doc-to-code spans + fragment ownership intact across all 18 files) |
| `cargo test --test docs_md_test` (5) | OK on the FIRST run - the corrected root-doc depth from task 6's correction was applied preemptively, so no resolution failures this time |

## Found problems/issues/mismatches
- None recorded. The two failure classes from task 6 (wrong root-doc depth,
  backticked mentions rendering as literal link text) were both anticipated
  and handled up front: the Context was corrected before execution (protocol
  step e), existing links were masked during conversion so their targets could
  not be re-matched, and a second pass unwrapped the 15 backticked mentions
  (README's index table x7, DEPENDENCIES x2, REFERENCES x3, ALGORITHMS x1,
  parser x1, validator x1).

## Other important findings/information
- The actual bare-mention inventory was 112 across the 14 files (the plan
  estimated ~117 - the difference is mentions that were already inside
  clickable links and thus excluded from conversion). Per-file converted
  counts match the inventory exactly: ALGORITHMS 5, DEPENDENCIES 2, README 7,
  REFERENCES 4, backend_c 16, backend_rust 12, cli_commands 8, codegen_core
  11, ir_json_rw 8, mc_engine 7, monitor_compiler 6, parser 14,
  toolchain_fuzz 4, validator 8.
- `src/backends/RUST.md` / `C.md` link as `../../src/backends/...` (two levels
  up - they live at the repo root), NOT `../src/backends/...` as the original
  task-file note suggested; the gate confirms resolution.
