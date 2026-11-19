# Phase 17, Task 3 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal
Fix the stale rows in the LLM-assistant set (`docs/llm/`) and KB.md section
4.2, and EXTEND both structure-inventory line budgets (user instruction,
2026-11-19) so that tasks 6-9 can append their new rows without failing the
Class B budget gate.

## Context
- Budgets (verified 2026-11-19): SRC_STRUCTURE 1786/1786 and TEST_STRUCTURE
  1379/1379 - BOTH exactly at their `<!-- llm-budget: N -->` headers. This phase
  adds source files (task 7's three cargo-member crates) and test functions
  (tasks 6-9), so both budgets must grow BEFORE those tasks run. Extension:
  SRC_STRUCTURE 1786 -> 1830, TEST_STRUCTURE 1379 -> 1430.
- Stale spots (verified 2026-11-19):
  - `docs/llm/TASK_MAP.md` row 31: the mc-ref CLI row names only
    `src/mc/ref_mc.rs::run` as the engine - no mode entry points. Rows 64-68
    (the per-mode engine rows) are already current from Phase 16.
  - `docs/llm/CHANGE_IMPACT.md` row 35: consumer name `gen_spec_cmd` (the file
    is `src/cli/commands/gen_spec.rs`).
  - `docs/llm/TEST_STRUCTURE.md` rows 1196-1197: "the gen_spec_cmd inline
    tests" / "the mc_ref_cmd inline tests" - pre-Phase 11 names. These two
    edits stay LINE-NEUTRAL (in-line replacements only).
  - `docs/llm/cli_commands.md` row 33: "the dispatch match - mc-ref/gen-spec
    delegate to their run_cli shells" (pre-Phase 13 - now ALL commands do);
    row 39: `exit_code_for` described as exact-only (0 pass-complete / 1 fail /
    2 unknown-state-limit) - now per-mode.
  - `KB.md` section 4.2: NO rows for `src/mc/ref_mc_exact.rs`,
    `src/mc/ref_mc_fuzz.rs`, `src/mc/ref_mc_smt.rs`; the `ref_mc.rs` row still
    describes the monolithic engine; the `cli/commands/mc_ref.rs` row still
    lists exact-only flags + "exit codes 0/1/2 mirrored".
- The Phase 16 status file (`docs/roadmap/phase_16/PHASE_16_STATUS.md`) is the
  authoritative source for what each new ref_mc_* file contains - use it (plus
  the files themselves) to write the KB.md rows.
- Class B budgets are machine-checked by `tests/dev_docs_test.rs`: every
  `docs/llm/*.md` must declare exactly one budget header within its first 10
  lines and stay within it. TASK_MAP (207/400), CHANGE_IMPACT (36/250) and
  cli_commands (77/200) have headroom - no header change needed there.

## Scope

### Affected documentation files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `docs/llm/TASK_MAP.md` | change | mc-ref CLI row gains the mode entry points |
| `docs/llm/CHANGE_IMPACT.md` | change | stale `gen_spec_cmd` name fixed |
| `docs/llm/SRC_STRUCTURE.md` | change | budget header 1786 -> 1830 ONLY (no row changes in this task) |
| `docs/llm/TEST_STRUCTURE.md` | change | rows 1196-1197 line-neutral name fixes + budget header 1379 -> 1430 |
| `docs/llm/cli_commands.md` | change | dispatch description + per-mode exit_code_for |
| `KB.md` | change | section 4.2: three new ref_mc_* rows + updated ref_mc.rs / mc_ref.rs rows |

## Planned actions

### Extend both inventory budgets
Change the `<!-- llm-budget: N -->` header in SRC_STRUCTURE.md from 1786 to
1830 and in TEST_STRUCTURE.md from 1379 to 1430. No other change to either
file in this action.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `docs/llm/SRC_STRUCTURE.md` | change (header line) | headroom for task 7's new crate rows (~15-20) |
| `docs/llm/TEST_STRUCTURE.md` | change (header line) | headroom for tasks 6-9's new test rows (~15-20) |

### Fix the four stale LLM-set spots
TASK_MAP row 31: add the mode entry points to the mc-ref CLI row (`run()`
delegates to `ref_mc_exact::run` / `ref_mc_fuzz::run` / `ref_mc_smt::run`,
D-P16-1) - keep the row's existing format. CHANGE_IMPACT row 35: `gen_spec_cmd`
-> `src/cli/commands/gen_spec.rs`. TEST_STRUCTURE rows 1196-1197: replace the
stale names in-line (LINE-NEUTRAL). llm/cli_commands.md row 33: ALL commands
delegate to their run_cli shells via the registry; row 39: `exit_code_for` is
per-mode (exact 0/1/2, fuzz 0/1, smt-bounded 0/1/2 with pass-within-bound).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `docs/llm/TASK_MAP.md` | change (row 31) | the CLI row must name where each mode's engine lives |
| `docs/llm/CHANGE_IMPACT.md` | change (row 35) | stale file name |
| `docs/llm/TEST_STRUCTURE.md` | change (rows 1196-1197, line-neutral) | stale file names |
| `docs/llm/cli_commands.md` | change (rows 33, 39) | pre-Phase 13 dispatch + exact-only exit codes |

### Update KB.md section 4.2
Add three rows (matching the table's existing column format): `src/mc/
ref_mc_exact.rs` (the exact-mode engine: BFS + liveness SCC + minimal-violation
labels, moved out of ref_mc.rs in Phase 16 task 1), `src/mc/ref_mc_fuzz.rs`
(the seeded fuzz-mode engine: the generated runtimes' exact LCG draw sequence,
byte-parity summary line + per-run violation reports, Phase 16 task 3),
`src/mc/ref_mc_smt.rs` (the in-process SMT-bounded executor over z3, third
consumer of the shared SmtSpec model, `#[cfg(feature = "smt")]`, Phase 16
task 4). Update the existing `ref_mc.rs` row: now the common core + entry
points (`run`/`run_fuzz`/`run_smt` delegate per D-P16-1; values/typing,
evaluator, monitor compiler+stepper, rendering, counterexample verification
stay here). Update the `cli/commands/mc_ref.rs` row: mode dispatch, per-mode
flags + exit codes. Content source: PHASE_16_STATUS.md + the files themselves.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `KB.md` | change (section 4.2) | the source map must name the Phase 16 files or future sessions cannot find them |

## Task deliverables
- Both inventory budgets extended (1830 / 1430).
- All four LLM-set stale spots fixed.
- KB.md section 4.2 current with the Phase 16 source layout.

## Task exit criteria
- [ ] `grep 'llm-budget' docs/llm/SRC_STRUCTURE.md` shows 1830; TEST_STRUCTURE
      shows 1430.
- [ ] No stale name remains: `grep -rn 'gen_spec_cmd\|mc_ref_cmd' docs/llm/
      docs/dev/` returns nothing (the phase_10 plan file is history - exempt).
- [ ] KB.md section 4.2 has rows for all three ref_mc_* files; the ref_mc.rs
      and mc_ref.rs rows describe mode dispatch.
- [ ] `cargo test --test dev_docs_test` green (budgets + reference resolution).
- [ ] `cargo fmt --check` clean.

## Non-goals
- No row ADDITIONS to either inventory in this task - that belongs to tasks
  6-9 (same-commit rule, PROTOCOL.md section 5).
- No changes to the other KB.md sections.
