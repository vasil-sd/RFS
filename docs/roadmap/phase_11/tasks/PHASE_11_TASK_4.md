# Phase 11, Task 4 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start implementation of the task immediately without re-deriving scope.

## Goal
Move the shared pre-backend codegen layer into `src/codegen/`: the zero-churn
absorption of `codegen.rs` as `codegen/mod.rs` plus fsm, test_plan, mc_concepts
and monitor.

## Context
- ZERO-CHURN: `codegen.rs` -> `codegen/mod.rs` keeps module path `codegen`, so
  every existing `use rfs::codegen::*` / `crate::codegen::*` site - including
  the detached fuzz workspace's `fuzz/fuzz_targets/codegen.rs` - compiles
  untouched. Verify by confirming no fuzz/ file appears in this task's diff.
- Renames: codegen_fsm.rs -> codegen/fsm.rs; codegen_test.rs ->
  codegen/test_plan.rs (NOT test.rs - that name belongs to the rfs-test command
  module created in task 6); codegen_mc.rs -> codegen/mc_concepts.rs; monitor.rs
  -> codegen/monitor.rs (no rename - the Monitor compiler is a shared pre-
  backend layer).
- The real 2-edge cycle between Codegen core and Shared MC concepts stays inside
  one directory after the move.

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 1.-3. | utils, json, lang | all import lines of these four files are already final |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 5. | backends/{rust,c}/ | nests under codegen/; its import lines must be final first |
| 6.-8. | cli, mc | their codegen import lines are already final |

### Affected source files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `src/codegen.rs` | remove (moved) | git mv to src/codegen/mod.rs (module path unchanged) |
| `src/codegen_fsm.rs` | remove (moved) | git mv to src/codegen/fsm.rs |
| `src/codegen_test.rs` | remove (moved) | git mv to src/codegen/test_plan.rs |
| `src/codegen_mc.rs` | remove (moved) | git mv to src/codegen/mc_concepts.rs |
| `src/monitor.rs` | remove (moved) | git mv to src/codegen/monitor.rs |
| all import sites of the four renamed modules | change | scripted use-path rewrite |

## Planned actions

### Absorb codegen.rs + move the four siblings
git mv all five; script use-path rewrites for the four renamed modules only
(`crate::codegen_fsm` -> `crate::codegen::fsm`, `crate::codegen_test` ->
`crate::codegen::test_plan`, `crate::codegen_mc` -> `crate::codegen::mc_concepts`,
`crate::monitor` -> `crate::codegen::monitor`). The codegen module path itself
is untouched.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/codegen.rs` | remove | git mv to src/codegen/mod.rs (zero-churn) |
| `src/codegen_fsm.rs`, `src/codegen_test.rs`, `src/codegen_mc.rs`, `src/monitor.rs` | remove | git mv per mapping |
| import sites of the four renamed modules | change | scripted rewrite, one pass |

### Gate
cargo build && cargo test green; confirm no fuzz/ file is in the diff (zero-
churn proof).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none) | - | verification only |

## Task deliverables
- `src/codegen/` holding mod.rs + fsm + test_plan + mc_concepts + monitor; all
  import sites final.

## Task exit criteria
- [ ] cargo build && cargo test green (535 baseline).
- [ ] No fuzz/ file modified by this task (zero-churn verified).
- [ ] No file references the old paths for the four renamed modules (grep-verified).
- [ ] File contents byte-identical to before (moves + use-line edits only).

## Non-goals
- No codegen behavior change; backends/ not touched yet (task 5).
