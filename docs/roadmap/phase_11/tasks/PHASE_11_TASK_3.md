# Phase 11, Task 3 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start implementation of the task immediately without re-deriving scope.

## Goal
Move the language core (parser, error, ir, printer, validator, specgen) into
`src/lang/`, including the lockstep edit of the detached fuzz workspace's two
`rfs::specgen` import lines in the SAME commit.

## Context
- ir.rs has 576 incoming edges - the biggest rename in the tree; all internal
  and purely mechanical. Script the use-path rewrites (`crate::ir` ->
  `crate::lang::ir`, etc.); do not hand-edit file by file (token budget).
- specgen lockstep: `fuzz/fuzz_targets/common.rs` imports
  `rfs::specgen::{Bounds, SpecGen}` and `fuzz/fuzz_targets/ir_roundtrip.rs`
  imports `rfs::specgen::{Rng, mutate}` - NOTE: after task 1 the Rng import is
  gone (consolidated into utils::rng), re-verify what ir_roundtrip actually
  imports at execution time. Three test files also import specgen:
  tests/common/prod_driver.rs, tests/ref_mc_diff_test.rs,
  tests/toolchain_fuzz_test.rs. The fuzz workspace is a SEPARATE cargo project -
  nothing in the main build catches its breakage; it must be edited in this
  commit and verified by building it (`cargo check` inside fuzz/ with the fuzz
  feature, or at minimum a grep-verified path update).
- error.rs moves with parser (all 7 of its incoming edges come from parser.rs);
  its module path becomes lang::error.

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 1. | utils/ substrate | specgen's Rng already deleted - the move is a pure rename |
| 2. | json/ substrate | lang files' json import lines are already final |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 4.-8. | all later move tasks | their lang import lines are already final |

### Affected source files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `src/{parser,error,ir,printer,validator,specgen}.rs` | remove (moved) | git mv to src/lang/ (names unchanged) |
| `src/lang/mod.rs` | new | module root declaring the six modules |
| all import sites of the six modules | change | scripted use-path rewrite |
| `fuzz/fuzz_targets/common.rs`, `fuzz/fuzz_targets/ir_roundtrip.rs` | change | specgen lockstep (separate cargo project) |
| `tests/common/prod_driver.rs`, `tests/ref_mc_diff_test.rs`, `tests/toolchain_fuzz_test.rs` | change | specgen import path updates |

## Planned actions

### Move the six files + create the module root
git mv all six; write src/lang/mod.rs; script every use-path rewrite across
src/, src/bin/, tests/. Rationale: the parse->IR->validate->print pipeline plus
its generator sit in one directory; all intra-group edges stay inside it.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/parser.rs`, `src/error.rs`, `src/ir.rs`, `src/printer.rs`, `src/validator.rs`, `src/specgen.rs` | remove | git mv to src/lang/ (contents untouched) |
| `src/lang/mod.rs` | new | declares parser, error, ir, printer, validator, specgen |
| import sites of the six modules | change | scripted rewrite, one pass |

### Lockstep: fuzz workspace + test importers
Update the two fuzz-target specgen imports and the three test-file specgen
imports to `rfs::lang::specgen` / `crate::lang::specgen` forms; verify the fuzz
workspace still type-checks (separate project - build it explicitly). Rationale:
a commit that moves specgen without these five lines leaves a broken detached
workspace behind.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `fuzz/fuzz_targets/common.rs` | change | rfs::specgen -> rfs::lang::specgen |
| `fuzz/fuzz_targets/ir_roundtrip.rs` | change | rfs::specgen -> rfs::lang::specgen (re-verify exact imports post-task-1) |
| `tests/common/prod_driver.rs`, `tests/ref_mc_diff_test.rs`, `tests/toolchain_fuzz_test.rs` | change | specgen import path updates |

### Gate
cargo build && cargo test green in the main workspace; fuzz workspace check
green (or grep-verified if its toolchain is unavailable - record which).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none) | - | verification only |

## Task deliverables
- `src/lang/` with all six files; every import site (main workspace + fuzz
  workspace + tests) at its final path.

## Task exit criteria
- [ ] cargo build && cargo test green (535 baseline).
- [ ] No file references the old top-level paths for the six modules (grep-verified, incl. fuzz/).
- [ ] The five specgen external import sites are edited in THIS commit's diff.
- [ ] File contents byte-identical to before (moves + use-line edits only).

## Non-goals
- No parser/validator/IR behavior change; no other file moved.
