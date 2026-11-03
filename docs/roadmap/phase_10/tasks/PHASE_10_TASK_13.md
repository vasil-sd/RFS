# Phase 10, Task 13 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start implementation of the task immediately without re-deriving scope.

## Goal
Enumerate every function and type outside the root package -
`src/training.rs`, `crates/rfs-build/src/lib.rs`, `crates/rfs-adapters/src/*.rs`
(5 files), `crates/examples/{sync_handler,async_handler,fuzz_rust}/src/*.rs` +
their `build.rs` (~55 fns / ~19 types before the bins) - into `SRC_STRUCTURE.md`. Completes the
non-test inventory.

## Context
- Conventions: SRC_STRUCTURE.md preamble + PHASE_10.md. Component/Functionality
  cells stay BLANK.
- Cluster notes:
  - `src/training.rs` (255 lines, 10 fns / 1 type): training-set aggregation,
    JSONL validation, and RFS snippet extraction.
  - `crates/rfs-build/src/lib.rs` (621 lines, 13 non-test fns + 12 inline tests enumerated in task 30 with the crate tests / 2 types): the build.rs
    helper crate - invokes the `rfs` toolchain at build time (`RFS_BIN` -> PATH
    resolution, cargo rerun directives); builder options `.backend()`,
    `.production()`, `.test_command()`, `.with_property_monitors()`, `.no_std()`,
    `.with_protocol_fsm()`. Dependency-free.
  - `crates/rfs-adapters/src/` (5 files, 21 fns / 14 types): the `ProtocolFsm`
    trait (`Input`/`Output`/`Error` associated types + `new()` +
    `step(&mut self, input) -> Result<Vec<Output>, Error>`) + four generic
    adapters - `channel` (std mpsc), `async_adapter` (futures Stream),
    `callback` (`drive(fsm, decide, max_steps)`), `minimal` (no_std step-
    forwarding wrapper). `#![no_std]` core with `std`/`async` features.
  - `crates/examples/sync_handler/src/lib.rs` (158 lines, 5 non-test fns + 1 inline test in task 30 / 3 types) +
    `build.rs` (42 / 1 fn): spec-43 sliding-window ARQ over the channel adapter,
    session-as-test + demo binary.
  - `crates/examples/async_handler/src/lib.rs` (167 lines, 3 non-test fns + 1 inline test in task 30 / 1 type) +
    `build.rs` (42 / 1 fn): same session as a futures Stream.
  - `crates/examples/fuzz_rust/src/lib.rs` (23 / 0 fns) + `build.rs` (61 /
    1 fn): cargo-fuzz layout wiring the emitted libfuzzer_sys skeleton.
  - The four `src/bin/*.rs` files (build_training, extract_training_snippets,
    generate_code_tests, test_generated_code - 1 fn each) are part of the root
    package: enumerate them HERE as well (they are thin bin wrappers; +4 fns /
    +0 types -> chunk total ~59 fns / ~19 types). Rationale: they belong to the
    training/test-generation tooling, not to any earlier cluster.
- Re-verify counts first (~59/~19 incl. bins); mismatch -> plan correction.
- Documented-in from each file's `//! Docs:` line (all are manifest files).
- The fuzz targets are NOT in this chunk - they live in the test inventory
  (task 30) per the boundary rule.

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 1. | Skeleton + conventions | tables + row format |
| 2.-12. | Chunks A-K | crate/bin rows depend on root-package identifiers already in the tables (rfs-build invokes the toolchain; adapters wrap generated FSMs) |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 14. | Categories proposal | this chunk's completion means the full inventory exists - task 14 can start |
| 15., 16. | Classification fills | classify these rows |

### Affected documentation files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `SRC_STRUCTURE.md` | change | ~73 function rows + ~21 type rows appended - final non-test chunk |

## Planned actions

### Re-verify chunk counts
Fn/type greps over all listed files (incl. the four `src/bin/*.rs`) vs
~73/~21; mismatch -> plan correction.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none) | - | verification only |

### Enumerate functions
Grep signatures, read bodies, one row per fn in `## All functions` with bidirectional project function/type Depends and one-line descriptions. Work crate by crate.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `SRC_STRUCTURE.md` | change (`## All functions`) | ~73 rows for training pipeline + workspace crates + bins |

### Enumerate types
One row per type in `## All types` (~21): builder option types, ProtocolFsm +
adapter types, example-crate session types.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `SRC_STRUCTURE.md` | change (`## All types`) | ~21 type rows |

### Gate check + inventory completeness sweep
docs_md_test green; then count ALL rows in both tables and compare with the
plan's corrected non-test totals (~999 fns / ~233 types per correction 2a) - a mismatch means a file was
missed: find it and record a plan correction.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none) | - | verification only |

## Task deliverables
- Complete function + type rows for every non-test source file in the repo.
- A completeness note (row totals vs plan totals) recorded in the task status.

## Task exit criteria
- [ ] Row counts equal re-verified grep counts per file.
- [ ] Every row: Depends, one-line description, Documented-in verbatim.
- [ ] Component/Functionality blank; rows one-per-line.
- [ ] Total table row counts match the plan's non-test totals (or a plan
      correction explains the delta).
- [ ] docs_md_test green.

## Non-goals
- No classification, no fuzz targets (task 30), no source changes.
