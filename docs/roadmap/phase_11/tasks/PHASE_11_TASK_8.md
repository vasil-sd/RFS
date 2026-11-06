# Phase 11, Task 8 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start implementation of the task immediately without re-deriving scope.

## Goal
Move the model-checking cluster (mc_plan, smt_plan, smt_emit, ref_mc) into
`src/mc/` - the last source move of the phase.

## Context
- Mapping: mc_plan.rs -> mc/plan.rs (renamed - the directory carries the "mc"
  context); smt_plan.rs + smt_emit.rs move unrenamed (inseparable pair - all 59
  of smt_emit's outgoing edges target smt_plan); ref_mc.rs moves unrenamed.
- ref_mc's independence contract: NO imports from mc_plan / monitor /
  codegen_fsm / backends - after the move it must have no intra-mc/ imports at
  all (it is a sibling file with zero directory-internal edges). Verify by grep.
- External import sites: validator 8, codegen_mc 5, ref_mc 5 (planning-time
  counts - re-verify by grep); internal churn scripted.

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 1.-7. | all earlier moves | every import line in these four files is already final |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 9. | Test-suite renames | first non-src task; source tree is complete after this one |

### Affected source files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `src/mc_plan.rs` | remove (moved) | git mv to src/mc/plan.rs |
| `src/smt_plan.rs`, `src/smt_emit.rs`, `src/ref_mc.rs` | remove (moved) | git mv to src/mc/ (unrenamed) |
| `src/mc/mod.rs` | new | module root declaring plan, smt_plan, smt_emit, ref_mc |
| all import sites of the four modules | change | scripted use-path rewrite |

## Planned actions

### Move the four files + create the module root
git mv per mapping; write src/mc/mod.rs; script every use-path rewrite (`crate::mc_plan`
-> `crate::mc::plan`, the other three keep their names under mc::). Rationale:
model-checking planning plus the independent reference engine sit in one
directory; the smt pair stays together.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/mc_plan.rs` | remove | git mv to src/mc/plan.rs |
| `src/smt_plan.rs`, `src/smt_emit.rs`, `src/ref_mc.rs` | remove | git mv to src/mc/ |
| `src/mc/mod.rs` | new | declares plan, smt_plan, smt_emit, ref_mc |
| import sites of the four modules | change | scripted rewrite, one pass |

### Gate
cargo build && cargo test green; grep proves ref_mc has no intra-mc/ imports.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none) | - | verification only |

## Task deliverables
- `src/mc/` with all four files; the source tree now matches the proposal
  exactly (tasks 1-8 complete the move set).

## Task exit criteria
- [ ] cargo build && cargo test green (535 baseline).
- [ ] ref_mc.rs imports nothing from within src/mc/ (independence contract grep-verified).
- [ ] No file references the old paths for the four modules (grep-verified).
- [ ] The full move set is done: every path in the plan's mapping table exists at its new location.

## Non-goals
- No MC behavior change; test files not touched yet (task 9).

## Task corrections

| Correction | Description + rationale |
|------------|-------------------------|
| ref_mc independence audit updated in two places (task-4 lesson, 2nd occurrence) | The audit reads its own file via `concat!(env!("CARGO_MANIFEST_DIR"), "/src/ref_mc.rs")` - the path broke with the move and now points at `/src/mc/ref_mc.rs`. Its forbidden list carried the string `"mc_plan"`, which the rename to `mc::plan` would have silently vacated; updated to `"mc::plan"` so the contract keeps biting. Both changes are inside ref_mc.rs's test module - no functionality impact. |
| Positive-control fixtures updated (task-3 lesson, 2nd occurrence) | `tests/fixtures/dev_docs/good_doc.md` + `good_llm_doc.md` reference real source paths (`src/mc_plan.rs::boolean_over_atoms`) and must track the tree - both repointed to `src/mc/plan.rs`. The negative-control fixtures are deliberately broken and stay untouched. |
| DEPENDENCIES.md mermaid labels shortened to subgraph-relative form | The rebuilt graph with full-path node labels (e.g. `codegen/backends/rust/mod.rs`) tripped mermaid-rs-renderer's flowchart geometry validation (`endpoint_reentries: 2` panic in docs_md_test). Shortening every restructured subgraph's labels to the original convention (subgraph title carries the directory context: `rust/mod.rs` under "codegen/backends (9)", `parser.rs` under "lang (6)", ...) fixed the layout - docs gate 5/5. |
| Stale refs from earlier sweeps found and fixed repo-wide | A final grep for EVERY Phase-11 old path across all live *.md surfaced stragglers no previous task had touched: `docs/SEMANTICS.md` (5 refs incl. its "normative sources" paragraph), `ide/README.md`, `CODEGEN.md` + `TOOLCHAIN.md` (dir-form / single refs), `KB.md` (bare-name mentions like `mc_plan.rs` without the src/ prefix + a `src/backends/` dir mention), `src/codegen/backends/{RUST,C}.md` (tasks 1-4 refs - these docs moved with the code in task 5 without a full sweep), and three START.md stragglers from the task-6 sweep. All fixed; the repo-wide grep now returns zero hits outside roadmap history / TASKS.md / the two inventories. |
