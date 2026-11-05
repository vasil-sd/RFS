# Phase 11, Task 1 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start implementation of the task immediately without re-deriving scope.

## Goal
Create the `src/utils/` substrate directory (`hash.rs` moved from `spec_hash.rs`,
NEW `rng.rs`) and execute the LCG consolidation - the proposal's only
non-mechanical source edit - so that every later task is a pure path rename.

## Context
- Read the "Target directory tree" + "Risk and effort" blocks of
  `SRC_STRUCTURE.md` (section "Proposed naming scheme") before starting - they
  are the single source of truth for this phase; the plan's Context table in
  `PHASE_11_PLAN.md` is the extracted old->new mapping.
- LCG facts: all four existing copies use constants 6364136223846793005 /
  1442695040888963407 with top-53-bit f64 mapping. The new `utils/rng.rs` holds
  ONE generator with the union of `specgen.rs`'s `Rng` and `codegen_test.rs`'s
  `Lcg` methods (`new`/`next_u64`/`next_f64`/`range`/`pick`/`chance`/`pick_index`)
  plus the two constants as shared `pub const`s. The two GENERATED-runtime copies
  (`rust_mc.rs` FUZZ_RUNTIME string, `c_mc.rs` C seed function) are emitted text -
  they cannot `use` the crate; they interpolate the same constant values so all
  four stay in one RNG family by construction. Generated output stays
  byte-identical (the determinism-pinning suites make this a tested invariant).
- `spec_hash.rs` import sites: 2 external test files + internal edges (re-verify
  with grep at task start; the proposal's counts are planning-time measurements).
- Per-task gate: `cargo build && cargo test` green before committing. clippy/fmt
  run at close-out only (task 14).

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| - | - | first task of the phase; no dependencies |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 3. | lang/ core | specgen moves with its Rng already deleted - pure rename |
| 4. | codegen/ shared layer | codegen_test moves with its Lcg already deleted - pure rename |

### Affected source files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `src/spec_hash.rs` | remove (moved) | becomes `src/utils/hash.rs` via git mv |
| `src/utils/mod.rs` | new | module root declaring hash + rng |
| `src/utils/rng.rs` | new | the single LCG generator + pub const constants |
| `src/specgen.rs` | change | Rng struct deleted; call sites use utils::rng |
| `src/codegen_test.rs` | change | Lcg struct deleted; call sites use utils::rng |
| `src/backends/rust_mc.rs` | change | FUZZ_RUNTIME string interpolates the pub consts |
| `src/backends/c_mc.rs` | change | C seed function interpolates the pub consts |
| import sites of spec_hash (2 test files + internal) | change | use-path updates, scripted |

## Planned actions

### Move spec_hash into utils/
git mv src/spec_hash.rs src/utils/hash.rs; write src/utils/mod.rs; script the
use-path rewrites (`crate::spec_hash` -> `crate::utils::hash`, external
`rfs::spec_hash` -> `rfs::utils::hash`). Rationale: substrate first, so no later
task touches these lines.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/spec_hash.rs` | remove | git mv to src/utils/hash.rs (contents untouched) |
| `src/utils/mod.rs` | new | `pub mod hash; pub mod rng;` |
| import sites of spec_hash | change | scripted use-path rewrite |

### Create utils/rng.rs + consolidate the two toolchain LCGs
Write the single generator (method union, same algorithm/constants) + the two
`pub const`s. Delete `specgen.rs`'s Rng struct and `codegen_test.rs`'s Lcg struct;
rewrite their call sites to `crate::utils::rng`. Rationale: one definition for
the toolchain-side RNG; the generated runtimes are handled in the next action.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/utils/rng.rs` | new | generator + pub const LCG_MULT / LCG_ADD (names per existing style) |
| `src/specgen.rs` | change | Rng struct + its impl removed; call sites -> utils::rng |
| `src/codegen_test.rs` | change | Lcg struct + its impl removed; call sites -> utils::rng |

### Constant-lock the two generated runtimes
In `rust_mc.rs`'s FUZZ_RUNTIME string and `c_mc.rs`'s C seed function, replace
the literal constants with interpolation of the shared pub consts (format!/
string-building at emit time). Rationale: all four copies stay in one RNG family
by construction; emitted bytes are unchanged.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/backends/rust_mc.rs` | change | FUZZ_RUNTIME template reads the pub consts |
| `src/backends/c_mc.rs` | change | C seed template reads the pub consts |

### Gate
cargo build && cargo test; confirm the determinism-pinning suites pass (byte-
identical codegen output) and no Rng/Lcg struct definition remains outside
utils/rng.rs.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none) | - | verification only |

## Task deliverables
- `src/utils/` with hash.rs + rng.rs; specgen/codegen_test LCG-free; both emitters constant-locked.

## Task exit criteria
- [ ] cargo build && cargo test green (535 baseline).
- [ ] No `struct Rng` / `struct Lcg` definition remains in specgen.rs or codegen_test.rs; utils/rng.rs is the sole toolchain-side definition.
- [ ] The two generated-runtime templates reference the shared pub consts (no literal 6364136223846793005 / 1442695040888963407 outside utils/rng.rs and the two templates' interpolation).
- [ ] All spec_hash import sites compile at the new path.

## Non-goals
- No move of any other file; no change to generated output bytes; no functionality edit beyond the consolidation.
