# Phase 10, Task 12 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start implementation of the task immediately without re-deriving scope.

## Goal
Enumerate every function and type of the independent reference-infrastructure
cluster - `src/ref_mc.rs`, `src/specgen.rs`, `src/spec_hash.rs`
(~193 fns / ~32 types) - into `SRC_STRUCTURE.md`. The largest single chunk.

## Context
- Conventions: SRC_STRUCTURE.md preamble + PHASE_10.md. Component/Functionality
  cells stay BLANK.
- Cluster notes:
  - `src/ref_mc.rs` (4,971 lines, 124 fns / 26 types): the brute-force reference
    model checker with DELIBERATELY independent data structures (Vec frontier,
    plain HashMap config keys, no bit-packing) + its own property evaluator;
    implements `docs/SEMANTICS.md` sections 2-9 directly. Public APIs: run/
    status, `step_trace`, `verify_counterexample` (replays a generated engine's
    JSON CE against the independent transition relation / monitors / liveness
    product); unified report rendering per D-P6-10.
  - `src/specgen.rs` (2,094 lines, 64 fns / 6 types): seeded random spec
    generator - fuzzer/seed bytes -> bounds, validity by construction (raw IR
    types deliberately NOT `Arbitrary`), 17-op incremental `mutate` API with
    per-step validator gate (invalid candidates rolled back + redrawn). Shared
    by the root `fuzz/` targets, `tests/toolchain_fuzz_test.rs`, and
    `rfs gen-spec`.
  - `src/spec_hash.rs` (158 lines, 5 fns / 0 types): hand-rolled minimal SHA-256
    (NIST test vectors, no dependencies) + the spec-hash helper - SHA-256 over
    the canonical IR JSON of the whole input file, first 16 hex chars stamped
    into line 1 of every generated artifact.
- Re-verify counts first (~193/~32); mismatch -> plan correction. This chunk is
  big: work file by file (ref_mc, then specgen, then spec_hash) and save rows
  after each file to keep the diff reviewable.
- Documented-in from each file's `//! Docs:` line (all three are manifest files).

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 1. | Skeleton + conventions | tables + row format |
| 2., 3., 4. | Chunks A-C | ref_mc/specgen rows depend on parser/IR/validator identifiers already in the tables |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 15., 16. | Classification fills | classify these rows |

### Affected documentation files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `SRC_STRUCTURE.md` | change | ~193 function rows + ~32 type rows appended |

## Planned actions

### Re-verify chunk counts
Fn/type greps over the three files vs ~193/~32; mismatch -> plan correction.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none) | - | verification only |

### Enumerate functions - ref_mc.rs
Grep signatures, read bodies, one row per fn (~124) with bidirectional project function/type
Depends and one-line descriptions.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `SRC_STRUCTURE.md` | change (`## All functions`) | ~124 rows for the reference MC |

### Enumerate functions - specgen.rs + spec_hash.rs
Same method for the remaining ~69 fns.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `SRC_STRUCTURE.md` | change (`## All functions`) | ~69 rows for spec generation + hashing |

### Enumerate types
One row per type in `## All types` (~32): ref_mc state/config/trace types,
specgen bounds/mutation types.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `SRC_STRUCTURE.md` | change (`## All types`) | ~32 type rows |

### Gate check
docs_md_test green.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none) | - | verification only |

## Task deliverables
- Complete function + type rows for `src/ref_mc.rs`, `src/specgen.rs`,
  `src/spec_hash.rs`.

## Task exit criteria
- [ ] Row counts equal re-verified grep counts per file.
- [ ] Every row: Depends, one-line description, Documented-in verbatim.
- [ ] Component/Functionality blank; rows one-per-line.
- [ ] docs_md_test green.

## Non-goals
- No classification, no other clusters, no source changes.
