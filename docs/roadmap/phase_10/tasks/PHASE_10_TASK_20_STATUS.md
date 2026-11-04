# Phase 10, Task 20 - Implementation status

> Written once when all exit criteria of `PHASE_10_TASK_20.md` are reached. Not
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
| `TEST_STRUCTURE.md` | changed | first data chunk of the test inventory: 27 function rows + 2 type rows appended to `## All functions` / `## All types` for the four shared test helpers (tests/common/{mod,prod_driver,fail_variants,mc_json}.rs). Every row carries bidirectional project Depends (helpers call into the rfs library API - mapped to src/ir.rs, src/codegen.rs, src/specgen.rs - AND same-file helper fns/types), a one-line description, and blank Component/Functionality/Documented-in cells. These helpers are the dependency base every later suite chunk (21-28) references |
| `docs/roadmap/phase_10/PHASE_10_PLAN.md` | changed | plan correction 20a: the cluster notes' "~8 types" overcounted by 6 - only 2 real Rust types exist (both in prod_driver.rs); see Found problems |

## Other file changes

| Name | Operation | Description |
|---|---|---|

## Changed functionality

None - docs-only inventory phase (no source moved or renamed).

## Removed functionality

None.

## New functionality

- The shared test-helper dependency base is now enumerated in TEST_STRUCTURE.md:
  - prod_driver.rs: the production-FSM driver infrastructure (flatten_groups/flattened_inputs/outputs, bit_rust_type/bit_c_type/c_field_ty, input_script, rust_driver_source/c_driver_source, production_artifacts/build_drivers, the sample-level driver pair, run_driver) + its two types (FlatVariant, ProductionArtifacts).
  - fail_variants.rs: the pinned FAIL-variant spec builders (spec_source/mutate_spec drift guard + http2_rst_fail_src/can_passive_fail_src).
  - mc_json.rs: zero_peak_rss (the peak_rss_kb normalization for byte-comparing MC JSON reports).

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|

## Tests run status

| Name | Status |
|---|---|
| `cargo test --test docs_md_test` (5) | OK - green; the 29 new rows parse cleanly under comrak GFM (Depends cells are plain text, not backticked paths, so no link checks are triggered) |

## Found problems/issues/mismatches

- Plan correction 20a: the cluster notes' "~8 types" was wrong. The read-through found only 2 real Rust types, both in prod_driver.rs (FlatVariant, ProductionArtifacts). fail_variants.rs's two `enum State` grep hits are RFS spec declarations INSIDE the pinned-FAIL-variant raw strings (`r#"..."#`), not toolchain definitions - the same template-phantom class corrections 7a/8a established for the emitter files. mc_json.rs holds NO types (its only type-ish grep hit is a local `const MARKER: &str`). Authoritative per-file counts: mod.rs 0/0, prod_driver.rs 21 fns / 2 types, fail_variants.rs 5 fns / 0 types, mc_json.rs 1 fn / 0 types = 27 fns / 2 types total (the ~27 fn estimate was already correct; only the type estimate was wrong).

## Other important findings/information

- tests/common/mod.rs contributes NO rows (0 fns / 0 types - just `pub mod` declarations), exactly like src/backends/mod.rs did in the source inventory; it will still get a File-mapping row in task 33.
- The top-level `const`s (prod_driver's GCC, fail_variants' RING_NO_GRANT_SRC / ELEV_NO_UP_SRC) are neither functions nor types, so they get no inventory rows; likewise production_artifacts' use of the rfs::IR_VERSION const is omitted from its Depends cell per the "functions and types only" convention.
- The helpers' cross-file Depends into the rfs library were mapped to real project identifiers: rfs::{Ir,Module,Type,ConstructorGroup,ConstructorVariant} -> src/ir.rs, rfs::codegen::{BackendOptions,CodegenCommandKind,get_backend} -> src/codegen.rs, rfs::specgen::Rng -> src/specgen.rs.
