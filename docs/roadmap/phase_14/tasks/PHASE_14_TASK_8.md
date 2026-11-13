# Phase 14, Task 8 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal
Document the new codegen artifact model where codegen is architecturally
documented (CODEGEN.md + the two backend docs), then re-sync every doc<->source
pointer and structure-inventory row that tasks 1-6 made stale, using `rfs-dev`
(sync-check/sync-fix/inventory) per dev_tools/SKILL.md.

## Context

- `src/codegen/CODEGEN.md` documents the backend plugin API: the `Backend`
  struct, the command entry points, and the output contract - it currently
  describes `CodegenOutput { code, extra_files }` and the single `--out FILE`
  semantics. It must describe: the named-slot output model, the flag family,
  BASE resolution (spec-stem default), take-or-emit + partial-pair error +
  D14-4 hash warning, and `--out -`.
- `src/codegen/backends/C.md` documents the C backend's architecture - it must
  gain the header/source split model (what lives in the header vs the source,
  the include guard, dual line-1 headers, the pragma placement) and the
  header-reference rule for mc/test/fuzz artifacts.
- `src/codegen/backends/RUST.md` documents the Rust backend - it must gain the
  include! reference mechanism (D14-5), the fused-vs-referenced rendering split
  in rust/fsm.rs, and the preserved `generate_core` aggregate contract.
- Tasks 1-6 changed function signatures/behaviour inside EXISTING files (no new
  source files were added by this phase) - so `tests/dev_docs_manifest.txt`
  changes only if a `//! Docs:` pointer line was added/removed; the structure
  inventories (`docs/llm/SRC_STRUCTURE.md`, `TEST_STRUCTURE.md`) need row
  updates for any NEW functions (e.g. the naming-resolution helpers, the new
  rust/fsm.rs entry point) and Depends-column updates where call edges moved.
- The dev_docs sync gate (`tests/dev_docs_test.rs`, 9 tests) enforces: code->doc
  pointer validity, doc->code reference resolution, fragment ownership, TASK_MAP
  coverage, Class B line budgets. `rfs-dev sync-check` lists every stale
  pointer; `sync-fix` repairs them; `inventory add/update/remove` maintains the
  rows; `inventory audit` is the string-masked completeness check (must end
  EXACT).
- `docs/llm/TASK_MAP.md` (functionality -> files) and `CHANGE_IMPACT.md`
  (change type -> affected files) carry codegen entries that reference the old
  output model - update the descriptions, not just the paths.

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 1.-6. | All code + help tasks | The docs describe the FINAL state; writing them earlier would document a moving target. |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 11. | Close-out | The dev_docs gate is part of full verification. |

### Affected documentation files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `src/codegen/CODEGEN.md` | change | Slot model, flag family, naming resolution, take-or-emit, stdout mode. |
| `src/codegen/backends/C.md` | change | Header/source split model + header-reference rule. |
| `src/codegen/backends/RUST.md` | change | include! mechanism + fused/referenced rendering split. |
| `docs/dev/*` (cli_commands, codegen-core, backend-c, backend-rust, and any file with stale references) | change (via `rfs-dev sync-fix`) | Pointer/reference re-syncing. |
| `docs/llm/TASK_MAP.md` | change | Codegen entries describe the new model. |
| `docs/llm/CHANGE_IMPACT.md` | change | Codegen change-type rows updated. |
| `docs/llm/SRC_STRUCTURE.md` | change (via `rfs-dev inventory`) | Rows for new fns; Depends updates where edges moved. |
| `docs/llm/TEST_STRUCTURE.md` | change (via `rfs-dev inventory`) | Same for test-side changes. |
| `tests/dev_docs_manifest.txt` | change (only if `//! Docs:` lines changed) | Manifest re-sync. |

## Planned actions

### Action 8.1 - Rewrite the codegen architecture docs

Update CODEGEN.md's output-contract section to the slot model + the full CLI
surface (mirroring the help text from task 6 and the contract table in
PHASE_14.md); add the C header/source split section to backends/C.md (header
content rules, include guard derivation, pragma placement, the no-source-
include invariant); add the Rust reference-mechanism section to backends/RUST.md
(include! at crate root, the flat-items precondition, the fused `generate_core`
contract preserved for the aggregate crate).

Rationale: these three files are the maintainer reference for "how codegen
works" - after this phase they would actively mislead (describing extra_files
and single-file output) if not rewritten.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/codegen/CODEGEN.md` | change (output contract + CLI sections) | New model documentation. |
| `src/codegen/backends/C.md` | change (artifact layout section) | Split model documentation. |
| `src/codegen/backends/RUST.md` | change (artifact layout section) | Reference mechanism documentation. |

### Action 8.2 - Re-sync pointers + inventories with rfs-dev

Run `rfs-dev sync-check` - repair every stale/missing/orphan pointer it lists
with `rfs-dev sync-fix` (review the dry-run plan first). Update the structure
inventories: `inventory add` for new functions (naming-resolution helpers in
codegen/mod.rs or cli, the new rust/fsm.rs entry point, any new test fns),
`inventory update` for changed Depends edges, `inventory remove` for deleted
items (`ExtraFile` and anything else tasks 1-6 removed). Finish with
`rfs-dev inventory audit` - it must report EXACT (zero per-file diffs).
Update TASK_MAP/CHANGE_IMPACT codegen entries' descriptions to the new model.

Rationale: PROTOCOL.md section 5 - the inventories describe the tree as it is,
in the same commit; the audit is the mechanical proof.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `docs/dev/*` (as listed by sync-check) | change | Pointer re-syncing. |
| `docs/llm/SRC_STRUCTURE.md` | change | Row add/update/remove per the audit. |
| `docs/llm/TEST_STRUCTURE.md` | change | Same. |
| `docs/llm/TASK_MAP.md` / `CHANGE_IMPACT.md` | change | Description updates. |
| `tests/dev_docs_manifest.txt` | change (if needed) | Manifest re-sync. |

### Action 8.3 - Run the sync gate

`cargo test --test dev_docs_test` - all 9 tests green (pointer validity,
reference resolution, fragment ownership, TASK_MAP coverage, Class B budgets).
If a Class B budget is exceeded by legitimate new rows, raise the budget header
the way Phase 13 did (round ceiling with headroom) and record it in the status
file.

Rationale: the gate is the authoritative check that the re-sync is complete.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (verification only - budget header edits if triggered) | change (conditional) | Budget raise, recorded. |

## Task deliverables
- CODEGEN.md + backends/{C,RUST}.md describing the final artifact model.
- Fully re-synced doc pointers + inventories (audit EXACT).
- dev_docs gate green (9/9).

## Task exit criteria
- [ ] CODEGEN.md documents the slot model, flag family, naming resolution,
      take-or-emit and `--out -`; no mention of `extra_files` remains.
- [ ] backends/C.md documents the split; backends/RUST.md documents include! +
      the preserved fused contract.
- [ ] `rfs-dev sync-check` reports zero stale pointers; `rfs-dev inventory
      audit` reports EXACT.
- [ ] `cargo test --test dev_docs_test` 9/9 green.

## Non-goals
- No user-facing doc changes (task 7).
- No source changes (docs-only task, except conditional budget headers).
- No rewriting of historical roadmap records.

## Other information

- Use `rfs-dev doc-section` to read the current CODEGEN.md/backend docs before
  editing - they are long; do not load them whole into context.
- If sync-check surfaces stale pointers in files OUTSIDE this phase's change
  set (pre-existing drift), fix them anyway and note it in the status file -
  the gate must end green regardless.
