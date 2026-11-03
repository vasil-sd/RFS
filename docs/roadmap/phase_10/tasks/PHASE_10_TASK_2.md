# Phase 10, Task 2 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start implementation of the task immediately without re-deriving scope.

## Goal
Enumerate every function and every type of the parser-core cluster -
`src/parser.rs`, `src/error.rs`, `src/lib.rs` (~120 fns / ~4 types) - into the
`## All functions` and `## All types` tables of `SRC_STRUCTURE.md`. First data
chunk of the inventory.

## Context
- Conventions: the Conventions preamble of `SRC_STRUCTURE.md` (task 1) +
  PHASE_10.md "Inventory conventions". Component/Functionality cells stay BLANK.
- Cluster notes:
  - `src/parser.rs` (2,091 lines, 108 fns / 2 types): nom 8 recursive-descent
    parser; comment stripping (`strip_comments`); the full module/item/enum/
    event/output/var/transition/pattern/expression/formula rule set; low-level
    token helpers (`identifier`, `keyword`, `soft_keyword`, `int_lit`,
    `p_tag`, `opt_p_tag`, `p_space0`, `p_space1`, `opt_semi`); error
    constructors (`semantic_err`, `syntax_err`); the Phase 4 task-13 targeted
    scan `find_eventually_missing_paren`. Also carries `pub const KEYWORDS`
    (28 RFS v0 keywords - shared source of truth with the `ide/` TextMate
    grammar; it is a const, not a type - note it in the parser row context,
    do not invent a type row for it).
  - `src/error.rs` (104 lines, 8 fns / 2 types): `Span`, `ParseError`, line/
    column computation and rich Display impls.
  - `src/lib.rs` (113 lines, 4 fns / 0 types): public API re-exports,
    `collect_rfs_files` (recursive directory walk, sorted results - what makes
    the example suite file-driven).
- Re-verify counts first: `grep -cE '^[[:space:]]*(pub(\(crate\))? )?fn '` and
  the type grep over the three files; if they differ from ~120/~4, record a
  plan correction before filling rows.
- Documented-in: copy the fragment names from each file's `//! Docs:` line
  (all three are in the 71-file manifest).

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 1. | Skeleton + conventions | provides the tables and the row format this task fills |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 15. | Classification fill (functions) | classifies the rows this task creates |
| 18. | File mapping table | maps these three files onto components/functionalities |

### Affected documentation files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `SRC_STRUCTURE.md` | change | ~120 function rows + ~4 type rows appended to the two inventory tables |

## Planned actions

### Re-verify chunk counts
Run the fn/type counting greps over the three files; compare with Context.
On mismatch: stop, record a plan correction in PHASE_10_PLAN.md, re-derive.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none) | - | verification only |

### Enumerate functions
For each of the three files: grep all fn signatures with line numbers, read
each body, and append one row per function to `## All functions`: identifier
`src/<file>.rs::<fn>`, Depends = every project function called AND every project type constructed/contained/matched, in `file::function` / `file::Type` form (nom/stdlib items are NOT dependencies - only project identifiers), Short description = one line of what it does. Keep rows one-per-line.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `SRC_STRUCTURE.md` | change (`## All functions`) | the ~120 new rows - complete coverage of the parser-core cluster, the inventory's foundation chunk |

### Enumerate types
Same pass for the ~4 types (`Span`, `ParseError`, and any parser-internal
types): identifier `src/<file>.rs::<Type>`, Depends = contained/constructed
project items where meaningful else empty, one-line description.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `SRC_STRUCTURE.md` | change (`## All types`) | the ~4 new type rows |

### Gate check
`cargo test --test docs_md_test` green.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none) | - | verification only |

## Task deliverables
- `SRC_STRUCTURE.md`: complete function + type rows for `src/parser.rs`,
  `src/error.rs`, `src/lib.rs`.

## Task exit criteria
- [ ] Row count in both tables for the three files equals the re-verified grep
      counts (verify by counting rows whose identifier starts with the file path).
- [ ] Every row has Depends (or a genuinely empty cell), a one-line Short
      description, and Documented-in copied verbatim from the `//! Docs:` line.
- [ ] Component/Functionality cells are blank in every new row.
- [ ] All rows one physical line; identifiers in `file::function` format.
- [ ] docs_md_test green.

## Non-goals
- No classification (task 15), no other clusters (tasks 3-13), no source changes.
