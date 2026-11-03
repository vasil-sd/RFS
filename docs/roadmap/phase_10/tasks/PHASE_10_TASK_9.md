# Phase 10, Task 9 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start implementation of the task immediately without re-deriving scope.

## Goal
Enumerate every function and type of the CLI-entry + tooling-command cluster -
`src/main.rs`, `src/frontend.rs`, `src/ext_tool.rs`, `src/cli_log.rs`,
`src/test_cmd.rs` (~90 fns / ~17 types) - into `SRC_STRUCTURE.md`.

## Context
- Conventions: SRC_STRUCTURE.md preamble + PHASE_10.md. Component/Functionality
  cells stay BLANK.
- Cluster notes:
  - `src/main.rs` (941 lines, 11 fns / 0 types): CLI entry point - EIGHT
    subcommands (`parse`, `validate`, `info`, `test`, `diagram`, `codegen`,
    `mc-ref`, `gen-spec`), argument parsing, exit codes; unknown subcommand
    prints usage and exits 1.
  - `src/frontend.rs` (747 lines, 24 fns / 6 types): parse/validate CLI logic -
    `run_conversion`/`plan_conversion`, output path planning (target-dir
    structure, extension replacement, collision guard via normalized absolute
    paths), validation dispatch incl. `.rfs-validation.err` writing and the
    Phase 5 task 13 `--json` structured diagnostics.
  - `src/ext_tool.rs` (622 lines, 26 fns / 4 types): shared external-tool
    resolver + invocation helper - presets RUSTC/GCC/DOT/PLANTUML/MERMAID/
    CLANG/CARGO_FUZZ with `RFS_*` env vars (+ `*_ARGS`), CLI-flag override,
    PATH default; fixed-subcommand extension (cargo-fuzz); deterministic POSIX
    sh script emission for `--print-script`.
  - `src/cli_log.rs` (100 lines, 10 fns / 2 types): `--log-level` handling.
  - `src/test_cmd.rs` (890 lines, 19 fns / 5 types): the `rfs test` command -
    suite selection (`unit`/`fuzz`), unit pipeline (codegen into temp dir +
    compile with resolved rustc/gcc + run), C/Rust libFuzzer fuzz pipelines,
    missing-tool diagnostics only for selected suites, `--print-script`
    coverage.
- Re-verify counts first (~90/~17); mismatch -> plan correction.
- Documented-in from each file's `//! Docs:` line (all five are manifest files).

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 1. | Skeleton + conventions | tables + row format |
| 2.-8. | Chunks A-H | CLI rows depend on frontend/codegen identifiers already in the tables |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 10., 11. | Chunks I-J | info/diagram command rows share the main.rs dispatch context |
| 15., 16. | Classification fills | classify these rows |

### Affected documentation files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `SRC_STRUCTURE.md` | change | ~90 function rows + ~17 type rows appended |

## Planned actions

### Re-verify chunk counts
Fn/type greps over the five files vs ~90/~17; mismatch -> plan correction.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none) | - | verification only |

### Enumerate functions
Grep signatures, read bodies, one row per fn in `## All functions` with bidirectional project function/type Depends and one-line descriptions.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `SRC_STRUCTURE.md` | change (`## All functions`) | ~90 rows for the CLI-entry cluster |

### Enumerate types
One row per type in `## All types` (~17): conversion/validation result types,
tool-resolver presets, test-command pipeline types.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `SRC_STRUCTURE.md` | change (`## All types`) | ~17 type rows |

### Gate check
docs_md_test green.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none) | - | verification only |

## Task deliverables
- Complete function + type rows for `src/main.rs`, `src/frontend.rs`,
  `src/ext_tool.rs`, `src/cli_log.rs`, `src/test_cmd.rs`.

## Task exit criteria
- [ ] Row counts equal re-verified grep counts per file.
- [ ] Every row: Depends, one-line description, Documented-in verbatim.
- [ ] Component/Functionality blank; rows one-per-line.
- [ ] docs_md_test green.

## Non-goals
- No classification, no other clusters, no source changes.
