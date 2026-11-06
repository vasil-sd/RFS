# Phase 11, Task 1 - Implementation status

> Written once when all exit criteria of `PHASE_11_TASK_1.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

| Name | Description |
|---|---|
| `src/utils/mod.rs` | module root of the cross-cutting substrate (`pub mod hash; pub mod rng;`) + //! Docs: pointer to the new dev:code-map-utils fragment |
| `src/utils/rng.rs` | the project's single hand-rolled LCG generator: `Rng` with the union of specgen's Rng and codegen_test's Lcg methods (new/next_u64/next_f64/range/pick/chance/pick_index) + the shared `pub const LCG_MULT` / `LCG_ADD`; //! Docs: pointer to dev:code-map-utils + dev:codegen-test-plan |

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| `src/spec_hash.rs` | removed (moved) | git mv to `src/utils/hash.rs`, contents byte-identical |
| `src/lib.rs` | changed | `pub mod spec_hash;` replaced by `pub mod utils;` (alphabetical position) |
| `src/codegen.rs` | changed | one use-path: `crate::spec_hash::` -> `crate::utils::hash::` |
| `src/backends/rust_test.rs` | changed | one use-path: `crate::spec_hash::` -> `crate::utils::hash::` |
| `src/specgen.rs` | changed | Rng struct + impl deleted (the RNG section header with them); `use crate::utils::rng::Rng;` added - every call site and type annotation is unchanged because the consolidated type keeps the name Rng |
| `src/codegen_test.rs` | changed | Lcg struct + impl deleted; `use crate::utils::rng::Rng;` added; the four `Lcg::new(RANDOM_TRACE_SEED)` call sites renamed to `Rng::new(...)` (pick_index is in the consolidated method set) |
| `src/gen_spec_cmd.rs` | changed | Rng removed from the specgen import; `use crate::utils::rng::Rng;` added (it imported Rng through specgen's re-export path) |
| `src/backends/rust_mc.rs` | changed | FUZZ_RUNTIME template: the two literal constants replaced by `__LCG_MULT__` / `__LCG_ADD__` placeholders; the single call site fills them from `crate::utils::rng::{LCG_MULT,LCG_ADD}` - emitted bytes unchanged |
| `src/backends/c_mc.rs` | changed | C template inside fuzz_runtime's format!: literals replaced by `{lcg_mult}ULL` / `{lcg_add}ULL` + the two consts added as format arguments; comment updated |
| `tests/common/prod_driver.rs` | changed | fully-qualified `rfs::specgen::Rng::new` -> `rfs::utils::rng::Rng::new` |
| `fuzz/fuzz_targets/ir_roundtrip.rs` | changed | same fully-qualified rename in the DETACHED fuzz workspace - the lockstep pulled forward into this commit because task 1 (not task 3's specgen move) is what deletes specgen::Rng |

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `tests/dev_docs_manifest.txt` | changed | `src/spec_hash.rs` -> `src/utils/hash.rs`; new entries `src/utils/mod.rs` + `src/utils/rng.rs` (the filesystem scan counts every src/ .rs file) |
| `docs/dev/CODE_MAP.md` | changed | new fragment dev:code-map-utils + "Cross-cutting substrate (src/utils/)" section describing hash.rs + rng.rs; 6 spec_hash path refs rewritten to the new path |
| `docs/dev/{ALGORITHMS,ARCHITECTURE,CODE_MAP,DECISIONS,DEPENDENCIES,ir_json_rw}.md` | changed | all `src/spec_hash.rs[::item]` doc->code refs rewritten to `src/utils/hash.rs[::item]` (24 refs) |
| `docs/dev/{ALGORITHMS,backend_c,backend_rust,codegen_core}.md`, `docs/llm/codegen_core.md` | changed | 7 `src/codegen_test.rs::Lcg` refs rewritten to `src/utils/rng.rs::Rng` (the item's new home) |
| `docs/dev/cli_commands.md` | changed | 2 `src/specgen.rs::Rng` refs rewritten to `src/utils/rng.rs::Rng` |
| `docs/llm/{TASK_MAP,ir_json_rw}.md` | changed | spec_hash path refs rewritten; TASK_MAP gains the Shared-LCG-generator concern row + file-list entries for both new utils files (gate coverage requirement) |

## Other file changes

| Name | Operation | Description |
|---|---|---|

## Changed functionality

None - the LCG consolidation is output-neutral by construction: same constants,
same top-53-bit mapping, same method semantics; the determinism-pinning suites
(byte-identity invariants) pass.

## Removed functionality

- The duplicate toolchain-side RNG definitions: `specgen::Rng` and
  `codegen_test::Lcg` (both deleted; their call sites use `utils::rng::Rng`).

## New functionality

- `src/utils/` substrate directory: spec hashing leaf + the single shared LCG.
- Constant-locking of both generated runtimes: the Rust FUZZ_RUNTIME string and
  the C seed function now interpolate the shared pub consts instead of carrying
  their own literals - all four RNG copies stay in one family by construction.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|

## Tests run status

| Name | Status |
|---|---|
| `cargo build` | OK |
| `cargo test` (full) | OK - 535 passed / 0 failed, exactly the documented baseline (the determinism-pinning suites prove codegen output is byte-identical after the consolidation) |
| `cargo test --test dev_docs_test` (9) | OK - green after the manifest + doc-ref re-sync (it failed first with 36 problems - its output was the update list) |

## Found problems/issues/mismatches

- PLAN CORRECTION 1a: the per-task green gate forces incremental docs<->source
  re-sync - task 1's move broke dev_docs_test immediately, so the manifest +
  doc-reference updates that task 10 had batched are actually done inside each
  move task; task 10 is re-scoped as the final verification sweep.
- The two NEW utils files were invisible to the plan: the Phase 8 sync gate
  requires EVERY manifest-listed file to carry a //! Docs: pointer line AND a
  TASK_MAP.md coverage reference - so creating files (not just moving them)
  costs a new CODE_MAP fragment + pointer lines + TASK_MAP rows, which the task
  file did not anticipate.
- `gen_spec_cmd.rs` imported Rng through specgen's re-export path
  (`use crate::specgen::{..., Rng, ...}`) - an import site the planning grep
  (which looked for `spec_hash` and struct definitions) did not list; the
  compile error surfaced it.

## Other important findings/information

- Naming the consolidated type `Rng` (specgen's name) made specgen.rs a zero-
  call-site-change edit - only the struct deletion + one import line. codegen_
  test.rs paid four call-site renames (Lcg::new -> Rng::new).
- The fuzz-workspace lockstep for specgen::Rng landed in THIS commit, not task
  3's: deleting the struct is what breaks `rfs::specgen::Rng`, and task 1 is
  where the deletion happens. Task 3's remaining lockstep scope is only the
  module-path imports (Bounds/SpecGen/mutate).
