# Phase 10, Task 27 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start implementation of the task immediately without re-deriving scope.

## Goal
Enumerate every function and type of the monitor/PSL test cluster -
`tests/monitor_ref_test.rs` (21 fns / 6 types) +
`tests/monitor_integration_test.rs` (18 fns / 1 type) +
`tests/psl_translation_test.rs` (34 fns / 3 types) - into `TEST_STRUCTURE.md`.

## Context
- Conventions: TEST_STRUCTURE.md preamble (task 19). Component/Functionality
  cells stay BLANK.
- Cluster notes:
  - `tests/monitor_ref_test.rs` (724 lines): independent hand-written Rust + C
    (`gcc -Wall`) reference monitors for specs 43/44/45 vs generated monitors on
    random traces + crafted boundary scenarios.
  - `tests/monitor_integration_test.rs` (494 lines): drives an injected bug
    through the MC in BOTH backends and exercises debug-gated production
    monitors.
  - `tests/psl_translation_test.rs` (994 lines): independent recursive window
    evaluator vs generated monitors over all property-bearing examples; the
    `RFS_PSL_REF_TOOL` external hook (availability-gated, skips when unset).
- Re-verify counts first (~73/~10); mismatch -> plan correction.
- Documented-in: blank for all three.

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 19. | TEST skeleton + conventions | tables + row format |
| 20. | Shared helpers chunk | helper identifiers exist for the Depends columns |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 31., 32. | Test categories + classification | use these rows |

### Affected documentation files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `TEST_STRUCTURE.md` | change | ~73 function rows + ~10 type rows appended |

## Planned actions

### Re-verify chunk counts
Fn/type greps over the three files vs ~73/~10; mismatch -> plan correction.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none) | - | verification only |

### Enumerate functions + types
Grep signatures, read bodies, one row per fn/type with bidirectional project function/type Depends
and one-line descriptions.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `TEST_STRUCTURE.md` | change (`## All functions`, `## All types`) | ~73 fn rows + ~10 type rows for the monitor/PSL cluster |

### Gate check
docs_md_test green.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none) | - | verification only |

## Task deliverables
- Complete function + type rows for the three monitor/PSL suites.

## Task exit criteria
- [ ] Row counts equal re-verified grep counts per file.
- [ ] Every row: Depends, one-line description, Documented-in blank per rule.
- [ ] Component/Functionality blank; rows one-per-line.
- [ ] docs_md_test green.

## Non-goals
- No other suites, no classification, no source changes.

## Task corrections (optional)

| Correction | Description + rationale |
|------------|-------------------------|
| Row counts 73/10 -> 72/9 | The Goal's per-file figures ("21 fns / 6 types" monitor_ref, "18 fns / 1 type" monitor_integration) were grep counts inflated by two template phantoms - the class of corrections 7a/8a/20a/21a/23a/24a/25a. monitor_ref_test.rs: one of the six type hits is a C `enum {{ KIND_ALWAYS, KIND_OBLIGATION }}` on a line of c_reference_source's format! string template (it starts a line, so the naive counter sees it) - only Sample, RefViolation, RefProp, CTable and SpecDef are real. monitor_integration_test.rs: one of the eighteen fn hits is `fn main() {` at column 0 inside the RUST_DRIVER raw-string driver template - only 17 fns are real (McViolation is clean). psl_translation_test.rs is clean at 34/3 (the six Ctx methods get `<file>::<Type>::<method>` identifiers). Authoritative total: 72 fns / 9 types. Recorded as plan correction 27a. |
