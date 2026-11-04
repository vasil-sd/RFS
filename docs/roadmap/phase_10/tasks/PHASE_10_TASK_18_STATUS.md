# Phase 10, Task 18 - Implementation status

> Written once when all exit criteria of `PHASE_10_TASK_18.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

| Name | Description |
|---|---|

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `SRC_STRUCTURE.md` | changed | filled the two remaining empty sections, completing the document: (a) `## File mapping` - one row per non-test source file (63 rows = 49 src/ incl. bin/ + 14 crates/), each with Components + Functionalities aggregated from the classified inventory (tasks 15-16) and a description + rationale stating what the file is and where it lands under task 17's scheme; (b) `## Proposed layout` - a mermaid flowchart TD of the proposed tree (one subgraph per top-level group, directories as nodes, files as leaves, short labels). All six sections now carry content; no placeholders remain |
| `docs/roadmap/phase_10/tasks/PHASE_10_TASK_18.md` | changed | CORRECTION 18a in place: the Context's "~42 in src/ + ~17 in crates/" (~59) estimate superseded by the authoritative 63 (49 src/ incl. bin/ + 14 crates/) |

## Other file changes

| Name | Operation | Description |
|---|---|---|

## Changed functionality

None - docs-only phase (proposal only, nothing moved or renamed).

## Removed functionality

None.

## New functionality

- The completed SRC_STRUCTURE.md is now the executable bridge for the refactoring phase:
  - `## File mapping` gives every non-test source file its assigned components + functionalities (drawn from the task-14/15/16 classification, not re-derived) and its proposed destination (new path, or kept with why).
  - `## Proposed layout` renders the proposed directory tree as a mermaid diagram that survives docs_md_test's full pipeline.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|

## Tests run status

| Name | Status |
|---|---|
| `cargo test --test docs_md_test` (5) | OK - green once the mermaid was rewritten to a vertical-spine layout (see Found problems); the 63-row table parses cleanly under comrak GFM |

## Found problems/issues/mismatches

- The task file's "~42 in src/ + ~17 in crates/" (~59 rows) estimate was stale in both directions: the authoritative non-test source count is 63 (49 src/ incl. bin/ + 14 crates/). A `find` walk that excludes only true test files under `tests//fuzz_targets/` gives exactly 63; the `*_test.rs` emitters/planners (`backends/rust_test`, `backends/c_test`, `codegen_test`) are SOURCE, not tests, and stay in. Corrected in place as CORRECTION 18a.
- A multi-valued-component parsing bug was caught while aggregating the Components/Functionalities cells: some inventory rows carry a comma-joined Component cell (e.g. `Rust backend, C backend`) that an unsplit parse would have recorded as one bogus value, producing duplicates like "Rust backend, Rust backend, C backend". Fixed by splitting both the Component and Functionality columns on commas; affected files (rust.rs, info.rs, printer.rs) now carry clean multi-valued cells.
- The mermaid diagram did not render on the first two attempts under mermaid-rs-renderer 0.3's strict pipeline: (1) edge-to-subgraph links pointing into subgraphs that contain nested subgraphs panic in stage_validation (`final_layout_errors`); (2) a flat fan-out of all eight groups as siblings of `src` panics in subgraph_spacing ("top-level flowchart subgraphs overlap"). The working form - matching the repo's own ARCHITECTURE.md pattern - is a vertical spine of cross-boundary node-to-node edges so the groups stack at different ranks; a caption clarifies those arrows are layout spines, not dependencies.

## Other important findings/information

- Destination consistency was machine-checked: no two source files map to the same proposed path except the intended `diagram.rs` + `diagrams/mod.rs` merge into `cli/commands/diagram/mod.rs`; every "kept at" row keeps its own path; all 63 rows' first column exactly equals the `find` non-test set (no missing, no extra).
- The two zero-row source files (`src/backends/mod.rs` re-export hub, `crates/examples/fuzz_rust/src/lib.rs` include! wrapper) take their Components/Functionalities from the Components table rather than the inventory aggregation.
