# RFS Test Structure - function and type inventory

Phase 10 analysis document (test source). Complete, dependency-annotated
inventory of every function and every data type in the project's TEST source,
classified into proposed component and functionality categories, with a
proposed new file-naming scheme and directory tree layout. Consumed by the
later refactoring phase that will actually reorganize the test tree - this
document is analysis and proposal only; no test source is moved or renamed
here. The dependency edges additionally support fine-grained impact analysis of
future test changes and precise scoping of refactoring/editing tasks. Non-test
source has its own inventory: `SRC_STRUCTURE.md`.

## Conventions (binding for all rows)

- Identifier formats: functions `<file>::<function>`, types `<file>::<Type>`,
  file paths relative to the repo root (e.g. `tests/frontend_test.rs::it_parses`,
  `fuzz/fuzz_targets/common.rs::Spec`). The All functions table's identifier
  column is named `Function`; the All types table's is named `Type`. A function
  nested inside another function body is `<file>::<parent_fn>::<child_fn>`
  (lowercase parent - distinct from the `<file>::<Type>::<method>` method form,
  whose middle segment is a capitalized type name); a type declared inside a
  function body follows the same form, `<file>::<parent_fn>::<Type>`.
- Inline `#[cfg(test)]` modules: functions and types declared inside an inline
  `#[cfg(test)] mod tests { ... }` in a `src/` file are enumerated HERE (not in
  SRC_STRUCTURE.md) with identifiers of the form `src/<file>.rs::tests::<fn>` -
  the host file is the identifier's file part and `tests` is the module segment.
  There are 25 such host files under `src/`.
- Depends column (BOTH tables, BOTH directions): comma-separated list of
  project identifiers - `<file>::<function>` AND `<file>::<Type>` - that the row
  depends on. A function row lists the functions it calls and the types it
  constructs/contains/matches; a type row lists the types it contains and the
  implementation functions its trait impls/methods delegate to. Cross-file and
  same-file alike; empty if none. Macro-generated call sites count once per
  distinct callee. External items (nom, std, dependency crates) are NOT
  dependencies - only project identifiers. Test rows frequently depend on
  NON-test source identifiers (those live in SRC_STRUCTURE.md's inventory); the
  edge still records the full `<file>::<item>` path.
- Component and Functionality cells may be multi-valued; values are
  comma-separated and must come from the `## Components` / `## Functionality`
  category tables (filled by task 31). They stay BLANK in every row until task
  32.
- Documented in column: copied verbatim from the file's single `//! Docs:`
  pointer line when it has one; blank otherwise. Most test files have no such
  line. A handful DO carry real `//! Docs:` pointers to fragments defined in
  `docs/` - the five fuzz targets (`fuzz/fuzz_targets/{common,parser,codegen,
  validator,ir_roundtrip}.rs`) and the three workspace-crate tests
  (`crates/rfs-adapters/tests/{adapters,nostd_minimal}.rs`,
  `crates/rfs-build/tests/integration.rs`) - and those fragments are copied like
  any other file's (these files are also in `tests/dev_docs_manifest.txt`). The
  three fixtures' `//! Docs:` lines are a separate case (see below). Where
  `docs/llm/TASK_MAP.md` has a suite entry for a test file, that is noted in the
  row's description instead of Documented-in.
- Fixtures: the three `tests/fixtures/dev_docs/*.rs` files
  (`good_code.rs`, `dangling_pointer.rs`, `missing_pointer.rs`) are
  negative-control FIXTURES for `dev_docs_test` - enumerated like any other file,
  but flagged as fixtures in their descriptions. Their "functions"/"types" are
  deliberate test subjects, not project code, and their `//! Docs:` lines point
  at fixture-only fragments (`dev:fixture-*` / `llm:fixture-*`) that are NOT
  defined in the doc set - they are the gate's own positive/negative controls,
  not real documentation pointers.
- Generated build output is NEVER enumerated: `crates/examples/fuzz_rust/fuzz/
  target/**` (and any other build-artifact tree) holds compiler/emitter output,
  not hand-written test source.
- Scope boundary (binding, from PHASE_10.md): this inventory covers `tests/**`,
  the inline `#[cfg(test)]` modules in `src/`, `crates/*/tests/**`, and
  `fuzz/fuzz_targets/**`. The fuzz targets' SOLE home is this document - they do
  not appear in SRC_STRUCTURE.md.
- Private (`fn`, no `pub`) items are included - the inventory is complete by
  construction, not public-API-only (most test functions are private).
- Every row stays on ONE physical line (grep-able, diff-friendly; future
  tooling and the refactoring phase rely on stable row identity).

## All functions

| Function | Depends | Component | Functionality | Short description | Documented in |
|---|---|---|---|---|---|
| tests/common/prod_driver.rs::rustc |  |  |  | rustc override - the RUSTC env var or "rustc" (same convention as backend_parity_test). |  |
| tests/common/prod_driver.rs::flatten_groups | src/ir.rs::ConstructorGroup, src/ir.rs::ConstructorVariant, tests/common/prod_driver.rs::FlatVariant |  |  | flatten constructor groups into variants using the backend naming convention (single-variant same-name group keeps the bare name; else Group_Variant; collisions get a numeric suffix). |  |
| tests/common/prod_driver.rs::flattened_inputs | src/ir.rs::Module, tests/common/prod_driver.rs::flatten_groups, tests/common/prod_driver.rs::FlatVariant |  |  | the module's input variants (flattened events). |  |
| tests/common/prod_driver.rs::flattened_outputs | src/ir.rs::Module, tests/common/prod_driver.rs::flatten_groups, tests/common/prod_driver.rs::FlatVariant |  |  | the module's output variants (flattened outputs). |  |
| tests/common/prod_driver.rs::state_variants | src/ir.rs::Module |  |  | the State enum's variant names (empty when the module has no State enum). |  |
| tests/common/prod_driver.rs::bit_rust_type |  |  |  | Rust integer type for a bit width (same mapping as the Rust backend): u8/u16/u32/u64/u128. |  |
| tests/common/prod_driver.rs::bit_c_type |  |  |  | C integer type for a bit width (same mapping as the C backend): uint8_t/uint16_t/uint32_t/uint64_t. |  |
| tests/common/prod_driver.rs::c_field_ty | src/ir.rs::Type, tests/common/prod_driver.rs::bit_c_type |  |  | C type of a payload/variable field (bool / bit<N> integer / {name}_t enum). |  |
| tests/common/prod_driver.rs::input_script | src/ir.rs::Module, src/ir.rs::Type, src/specgen.rs::Rng, tests/common/prod_driver.rs::flattened_inputs |  |  | deterministic input script (the LCG lineage of the fuzz wire format) - one line per step, variant picked by the RNG, field values drawn from each field's domain; the same bytes feed both drivers. |  |
| tests/common/prod_driver.rs::enum_helper_name |  |  |  | per-enum variant-index helper name (rfs_pp_enum_idx_{name}), shared by the Rust and C glue. |  |
| tests/common/prod_driver.rs::rust_driver_source | src/ir.rs::Module, src/ir.rs::Type, tests/common/prod_driver.rs::flattened_inputs, tests/common/prod_driver.rs::flattened_outputs, tests/common/prod_driver.rs::bit_rust_type, tests/common/prod_driver.rs::enum_helper_name |  |  | the Rust driver source - the production artifact verbatim plus generated glue (enum-index helpers, input parser, raw var/output renderers) and main, built by concatenation so it compiles as-is. |  |
| tests/common/prod_driver.rs::c_kind_name |  |  |  | the C kind-enum constant for a flattened variant ({prefix}_{UPPERCASE}). |  |
| tests/common/prod_driver.rs::c_payload_member |  |  |  | the C payload-union member name for a flattened variant ({prefix}_{lowercase}), same convention as the C backend. |  |
| tests/common/prod_driver.rs::c_union_member | tests/common/prod_driver.rs::c_payload_member |  |  | the input payload-union member name (c_payload_member with the "input" prefix). |  |
| tests/common/prod_driver.rs::c_driver_source | src/ir.rs::Module, tests/common/prod_driver.rs::flattened_inputs, tests/common/prod_driver.rs::flattened_outputs, tests/common/prod_driver.rs::c_kind_name, tests/common/prod_driver.rs::c_payload_member, tests/common/prod_driver.rs::c_union_member, tests/common/prod_driver.rs::c_field_ty |  |  | the C driver source - #include of the production artifact plus generated glue (kind enum, payload union, input parser, raw renderers) and main. |  |
| tests/common/prod_driver.rs::production_artifacts | src/ir.rs::Module, src/ir.rs::Ir, src/codegen.rs::BackendOptions, src/codegen.rs::CodegenCommandKind, src/codegen.rs::get_backend, tests/common/prod_driver.rs::ProductionArtifacts |  |  | generate the production artifacts (property monitors ON) for module into <scratch_root>/<tag>/ and return their paths + the Rust source; the input drivers build on top of this. |  |
| tests/common/prod_driver.rs::build_drivers | tests/common/prod_driver.rs::production_artifacts, tests/common/prod_driver.rs::rust_driver_source, tests/common/prod_driver.rs::c_driver_source, tests/common/prod_driver.rs::rustc |  |  | build both drivers for one spec - generate the production artifacts, write the driver sources next to them, compile with rustc and gcc (asserting the generated C is warning-free). |  |
| tests/common/prod_driver.rs::rust_sample_driver_source | src/ir.rs::Module, src/ir.rs::Type, tests/common/prod_driver.rs::state_variants, tests/common/prod_driver.rs::bit_rust_type |  |  | the Rust sample-level driver source - drives the generated property monitors over raw sample lines (State|v0,v1,...) instead of FSM inputs; emits M{k} OK/FAIL + FIRST. |  |
| tests/common/prod_driver.rs::c_sample_driver_source | src/ir.rs::Module, src/ir.rs::Type, tests/common/prod_driver.rs::state_variants, tests/common/prod_driver.rs::bit_c_type |  |  | the C sample-level driver source - the C mirror of rust_sample_driver_source (generated monitors over raw sample lines). |  |
| tests/common/prod_driver.rs::build_sample_drivers | tests/common/prod_driver.rs::production_artifacts, tests/common/prod_driver.rs::rust_sample_driver_source, tests/common/prod_driver.rs::c_sample_driver_source, tests/common/prod_driver.rs::rustc |  |  | build both sample-level drivers for one spec (generated monitors over raw sample lines); returns (rust bin, c bin). |  |
| tests/common/prod_driver.rs::run_driver |  |  |  | run one driver with the script on stdin; returns (exit code, stdout). |  |
| tests/common/fail_variants.rs::root |  |  |  | the repo root (CARGO_MANIFEST_DIR). |  |
| tests/common/fail_variants.rs::spec_source | tests/common/fail_variants.rs::root |  |  | read a valid example spec from examples/valid/<name> (panics if unreadable). |  |
| tests/common/fail_variants.rs::mutate_spec | tests/common/fail_variants.rs::spec_source |  |  | read a valid example and replace exactly one anchored block with new (asserts the anchor occurs exactly once - the drift guard). |  |
| tests/common/fail_variants.rs::http2_rst_fail_src | tests/common/fail_variants.rs::mutate_spec |  |  | 47_http2_stream with the Open RstStream arm retargeted off Closed, so rst_goes_closed is violated at step 2 (the pinned FAIL variant). |  |
| tests/common/fail_variants.rs::can_passive_fail_src | tests/common/fail_variants.rs::mutate_spec |  |  | 49_can_arbitration with the Data AckErr band-entry counter update weakened (+1 instead of +4), so passive_band is violated at step 6 (the pinned FAIL variant). |  |
| tests/common/mc_json.rs::zero_peak_rss |  |  |  | zero the peak_rss_kb field in one compact MC JSON document before byte-comparison (machine-dependent, never pinned). |  |

## All types

| Type | Depends | Component | Functionality | Short description | Documented in |
|---|---|---|---|---|---|
| tests/common/prod_driver.rs::FlatVariant | src/ir.rs::Type |  |  | one flattened input/output variant of a module (declaration order): collision-free name, its constructor group, the bare variant name, and (field name, field type) pairs. |  |
| tests/common/prod_driver.rs::ProductionArtifacts |  |  |  | generated production artifacts (property monitors ON) for one module - the Rust source kept in memory plus both artifact paths (the C driver #includes the .c file). |  |

## Components

(filled by task 31 - the proposed test-side component categories)

| Name | Depends on | Description and rationale |
|---|---|---|

## Functionality

(filled by task 31 - the proposed test-side functionality axes)

| Name | Depends on | Description and rationale |
|---|---|---|

## File mapping

(filled by task 33 - one row per non-generated test source file: components,
functionalities, description + rationale incl. its proposed new path)

| Source file name | Components | Functionalities | Description + rationale |
|---|---|---|---|

## Proposed layout

(filled by task 33 - the proposed test directory tree as a mermaid diagram)
