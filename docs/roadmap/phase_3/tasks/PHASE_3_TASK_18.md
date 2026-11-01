# Phase 3, Task 18 - Implementation details

> Written for a future LLM session: after reading `START.md`,
> `docs/roadmap/INTRO.md` and this file, the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal
Planned task 18: regenerate the generated-code test aggregate and run it - the new
examples' monitor code must compile and pass in both backends; record updated counts
for the phase status file.

## Context

- Cargo aliases (BUILD.md): `cargo generate-code-tests` builds
  `target/rfs_codegen_tests` (aggregate crate over ALL valid examples + training
  snippets, NOT checked in); `cargo test-generated-code` runs it.
- Phase 2 baseline counts: 164 modules / 492 generated tests - expected to GROW with
  examples 41/42 (and any snippet changes from tasks 19-20 if they run first; here we
  record the counts as of THIS task's run).
- The aggregate exercises production/test/mc codegen for every module and compiles the
  generated Rust - so monitor rendering bugs surface here.
- Requires task 13 (all valid examples parse) and task 15 (codegen stable).

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 13. | Valid examples fixed | the aggregate parses every valid example |
| 15. | Codegen tests green | codegen behavior pinned before mass regeneration |

### Other affected files (optional)

| File | Operation | Brief description |
|------|-----------|-------------------|
| `target/rfs_codegen_tests` | change (generated, not checked in) | regenerated aggregate crate |

## Planned actions

### Action 1: Regenerate + run the aggregate

Run `cargo generate-code-tests && cargo test-generated-code` (toolchain env per
START.md section 4 / TOOLCHAIN.md). If generated monitor code fails to compile or a
generated test fails: that is a backend rendering bug - record a plan correction and
open/fix via the responsible backend path; do NOT weaken the aggregate or skip modules.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `target/rfs_codegen_tests` | change (regenerated) | full re-generation over current examples/snippets |

### Action 2: Record updated counts

Capture the new module count and generated-test count (from the cargo output lines)
and record them in this task's status file - task 22 copies them into
`PHASE_3_STATUS.md` "Tests run status".

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none - counts recorded in PHASE_3_TASK_18_STATUS.md) | - | input for task 22 |

## Task deliverables
- Green aggregate regeneration + run; exact new module/test counts recorded.

## Task exit criteria
- [ ] `cargo generate-code-tests` exits 0 (all valid examples + snippets, both backends' Rust-side aggregate).
- [ ] `cargo test-generated-code` exits 0 with the full new test count passing.
- [ ] New module/test counts recorded in the status file (supersedes Phase 2's 164/492).

## Non-goals
- No checked-in file changes unless a rendering bug forces one (then: plan correction +
  fix, documented in status).
- C-side aggregate compilation is covered by tasks 16/17 integration harnesses (gcc),
  not by this Rust aggregate.

## Other information

- If counts change again later (tasks 19-20 add snippets), task 20 re-runs the training
  side and task 22 records the FINAL numbers in PHASE_3_STATUS.md - use whichever run
  is last before close-out, but keep this task's numbers as the codegen-side baseline.
