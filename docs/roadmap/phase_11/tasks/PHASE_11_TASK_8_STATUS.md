# Phase 11, Task 8 - Implementation status

> Written once when all exit criteria of `PHASE_11_TASK_8.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

| Name | Description |
|---|---|
| `src/mc/mod.rs` | module root of the model-checking cluster (`pub mod plan; ref_mc; smt_emit; smt_plan;`) + //! Docs: pointer to the new dev:code-map-mc fragment; doc comment records the smt-pair inseparability and ref_mc's independence contract |

## Changed/removed source files

| Name | Operation | Description |
|---|---|
| `src/mc_plan.rs` | removed (moved) | git mv to `src/mc/plan.rs`, contents byte-identical (0 changed lines - the rename is the only change) |
| `src/ref_mc.rs` | removed (moved) | git mv to `src/mc/ref_mc.rs`; 6 changed lines = the independence audit's self-read path (`/src/ref_mc.rs` -> `/src/mc/ref_mc.rs`) + its forbidden list (`"mc_plan"` -> `"mc::plan"`, which the rename would have silently vacated) + one doc-comment path |
| `src/smt_plan.rs` | removed (moved) | git mv to `src/mc/smt_plan.rs`; 5 import rewrites (crate::mc_plan:: -> crate::mc::plan:: - its sibling inside the new directory) |
| `src/smt_emit.rs` | removed (moved) | git mv to `src/mc/smt_emit.rs`; 6 import rewrites (crate::smt_plan:: -> crate::mc::smt_plan::) - the inseparable pair stays together |
| `src/lib.rs` | changed | four `pub mod` lines replaced by `pub mod mc;` (alphabetical position after lang) - the crate root now declares 10 modules: cli, codegen, json, lang, mc, training, utils + ... |
| `src/codegen/backends/{rust,c}/mc.rs` | changed | scripted rewrites: crate::mc_plan:: -> crate::mc::plan:: (16 sites each), crate::smt_plan:: / crate::smt_emit:: -> crate::mc::... (1 each) |
| `src/cli/commands/mc_ref.rs` | changed | `use crate::ref_mc::{self, RefConfig}` -> `crate::mc::ref_mc::{self, RefConfig}` (the self binding keeps unqualified call sites working) + the module doc's bare `crate::ref_mc` mention |
| `tests/{info_format_test 15, ref_mc_diff_test 10, ref_mc_diag_test 4, ref_mc_sweep_test 4, toolchain_fuzz_test 3}.rs` | changed | rfs::ref_mc:: -> rfs::mc::ref_mc:: scripted rewrites; no test logic touched |

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|
| `tests/dev_docs_manifest.txt` | changed | four old paths replaced by the five new sorted entries (86 -> 87 entries; the mc/ block sits between main.rs and utils/hash.rs) |
| `docs/dev/CODE_MAP.md` | changed | NEW fragment dev:code-map-mc + "MC engine (`src/mc/`) (5 files)" section (mod.rs + the four files, ref_mc bullet noting the audit-tested independence); "Codegen core + backends" 19 -> 15 files with its "MC engine (4)" subsection removed - which also fixed a pre-existing count bug (the subsection carried FIVE bullets: the four MC files + monitor.rs, whose bullet moved into "Codegen core (6)") |
| `docs/llm/TASK_MAP.md` | changed | Map-section path rewrites (7 refs); Coverage: four bullets replaced by a grouped five-entry block (mod.rs first, ref_mc bullet noting no intra-mc/ imports); header 86 -> 87 |
| `docs/dev/DEPENDENCIES.md` | changed | file-level mermaid graph: MC subgraph retitled "mc (4)" with its four node labels updated; ALL restructured subgraphs' labels shortened to the original subgraph-relative convention (full-path labels tripped mermaid-rs-renderer's geometry validation - endpoint_reentries panic in docs_md_test) |
| `docs/dev/*.md` + `docs/llm/*.md` (16 files) | changed | 425 path refs rewritten to the new mc/ locations (mc_engine.md alone carries 255) |
| `KB.md`, `DESIGN.md`, `BUILD.md`, `CODEGEN.md`, `START.md`, `TOOLCHAIN.md`, `docs/SEMANTICS.md`, `ide/README.md` | changed | root + previously-un swept docs: the four mc paths rewritten wherever they appeared, plus stragglers from earlier tasks that a final repo-wide grep surfaced - SEMANTICS.md's "normative sources" paragraph (mc_plan/monitor/codegen_mc/backends/validator/parser), KB.md bare-name mentions (`mc_plan.rs` without src/ prefix) + a `src/backends/` dir mention, ide/README.md + TOOLCHAIN.md single refs, CODEGEN.md dir-form ref, three START.md stragglers from the task-6 sweep |
| `src/codegen/backends/{RUST,C}.md` | changed | the two backend architecture docs (moved with the code in task 5) carried tasks 1-4 stale refs - all fixed (`src/backends/rust*.rs` globs, `src/codegen.rs`, `src/spec_hash.rs`, `src/monitor.rs` x2, `src/codegen_mc.rs`, `src/mc_plan.rs`) |
| `tests/fixtures/dev_docs/{good_doc,good_llm_doc}.md` | changed | the POSITIVE-CONTROL fixtures' doc->code ref (`src/mc_plan.rs::boolean_over_atoms`) repointed to `src/mc/plan.rs` - the fixtures must track the real tree or the gate's positive control fails (negative controls untouched) |

## Other file changes

| Name | Operation | Description |
|---|---|
| (none) | - | no examples/training/build-file changes |

## Changed functionality

None - pure moves/renames + import-path rewrites; the ref_mc audit's two in-test updates keep its contract meaningful rather than changing behavior. MC output byte-identical (all ref-MC parity/differential/sweep suites green).

## Removed functionality

None.

## New functionality

- `src/mc/` directory: model-checking planning + the SMT pair + the independent reference engine in one place; the source tree now matches the Phase 10 proposal exactly (the src/ root holds only lib.rs, main.rs, training.rs - the complete move set is done).

## Tests add/change/remove

| Name | Operation | Description |
|---|---|
| `src/mc/ref_mc.rs` independence_import_audit | changed (in-place) | self-read path + forbidden list updated to the post-move reality so the audit keeps biting |
| `tests/fixtures/dev_docs/{good_doc,good_llm_doc}.md` | changed | positive-control refs track the tree |

## Tests run status

| Name | Status |
|---|---|
| `cargo build` | OK (warning-free) |
| `cargo test` (full) | OK - 535 passed / 0 failed, exactly the documented baseline (ref-MC sweep/differential/diag + toolchain-fuzz ground-truth suites prove MC behavior identity) |
| `cargo test --test dev_docs_test` (9) | OK - green after the manifest + CODE_MAP + TASK_MAP re-sync + fixture updates |
| `cargo test --test docs_md_test` (5) | OK - green after the mermaid label shortening (full-path labels had tripped the renderer's geometry validation) |
| `cargo fmt --check` | OK - clean after wrapping two over-length test lines the path rewrite created |

## Found problems/issues/mismatches

- The ref_mc independence audit hard-codes BOTH its own file path and forbidden module-name strings - a move silently breaks/vacates them unless updated in-commit (task-4 lesson, second occurrence; both fixed).
- docs_md_test's mermaid renderer is geometry-sensitive: full-path node labels in the rebuilt DEPENDENCIES.md graph caused an endpoint_reentries panic; subgraph-relative labels (the original convention) render cleanly.
- A final repo-wide grep for every Phase-11 old path found stale refs in files no previous task had swept: docs/SEMANTICS.md, ide/README.md, TOOLCHAIN.md, the two backend .md docs, plus bare-name/dir-form mentions in KB/CODEGEN/START. All fixed - the live-doc tree is now fully consistent with the refactored source.

## Other important findings/information

- mc/plan.rs moved byte-identical: its only intra-cluster edge (from smt_plan) was rewritten on the SENDER side, and no other file imported it by a form the script missed.
- The src/ root is now exactly {lib.rs, main.rs, training.rs} - the proposal's kept-root set. Tasks 9-14 are docs/test-renames/bookkeeping only; no further source moves remain.
