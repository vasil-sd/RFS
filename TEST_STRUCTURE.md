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
| tests/frontend_test.rs::tmp_root |  |  |  | create a fresh per-test temp dir under target/ and return its path. |  |
| tests/frontend_test.rs::cleanup |  |  |  | remove a test temp-dir tree (ignore errors). |  |
| tests/frontend_test.rs::write_file |  |  |  | write text to a path, creating parent directories as needed. |  |
| tests/frontend_test.rs::ir_json | src/parser.rs::parse_source |  |  | serialize an Ir to compact JSON text through the IR wire format. |  |
| tests/frontend_test.rs::json_ir | src/json_rw.rs::parse |  |  | parse an IR JSON document text into an Ir (panics with the error on failure). |  |
| tests/frontend_test.rs::format_from_str_and_extension |  |  |  | resolve a Format from a CLI string plus the file extension (the strict-extension rule). |  |
| tests/frontend_test.rs::file_input_rejects_unsupported_extension | src/frontend.rs::run_conversion |  |  | a single-file input with an unsupported extension is rejected before any conversion. |  |
| tests/frontend_test.rs::file_input_rejects_mismatched_format_extension | src/frontend.rs::run_conversion |  |  | a file whose extension disagrees with the requested input format is rejected. |  |
| tests/frontend_test.rs::directory_scan_only_collects_matching_extension | src/frontend.rs::run_conversion |  |  | a directory scan collects only files with the matching extension, ignoring the rest. |  |
| tests/frontend_test.rs::output_names_drop_original_extension_without_double_extensions | src/frontend.rs::run_conversion |  |  | conversion output names drop the original extension and never produce a double extension. |  |
| tests/frontend_test.rs::file_input_writes_next_to_source_without_target_dir | src/frontend.rs::run_conversion |  |  | with no --target-dir, a file input's output is written next to the source. |  |
| tests/frontend_test.rs::file_input_with_target_dir_writes_into_target_dir | src/frontend.rs::run_conversion |  |  | with --target-dir, a file input's output is written into that directory. |  |
| tests/frontend_test.rs::directory_input_preserves_structure_in_target_dir | src/frontend.rs::run_conversion |  |  | a directory input with --target-dir preserves the relative structure under the target. |  |
| tests/frontend_test.rs::directory_input_without_target_dir_writes_next_to_each_source | src/frontend.rs::run_conversion |  |  | a directory input without --target-dir writes each output next to its source. |  |
| tests/frontend_test.rs::normalize_abs_resolves_relative_components | src/frontend.rs::normalize_abs |  |  | normalize_abs collapses . / .. components into an absolute path. |  |
| tests/frontend_test.rs::in_place_same_format_conversion_is_rejected | src/frontend.rs::run_conversion |  |  | converting a file in place to the same format is rejected as a misuse. |  |
| tests/frontend_test.rs::target_dir_equal_to_source_dir_is_rejected | src/frontend.rs::run_conversion |  |  | a --target-dir equal to the source directory is rejected. |  |
| tests/frontend_test.rs::directory_conversion_into_its_own_tree_is_rejected | src/frontend.rs::run_conversion |  |  | converting a directory into its own subtree is rejected. |  |
| tests/frontend_test.rs::check_roundtrip_source | src/parser.rs::parse_source |  |  | assert one source file round-trips through a conversion and re-parses to an equivalent IR. |  |
| tests/frontend_test.rs::converts_rfs_to_json | src/frontend.rs::run_conversion |  |  | an RFS source converts to JSON that parses back to the same IR. |  |
| tests/frontend_test.rs::converts_json_to_rfs | src/frontend.rs::run_conversion, src/parser.rs::parse_source |  |  | a JSON document converts to RFS source that parses back to the same IR. |  |
| tests/frontend_test.rs::converts_rfs_to_rfs_with_target_dir | src/frontend.rs::run_conversion, src/parser.rs::parse_source |  |  | RFS-to-RFS conversion with --target-dir rewrites the source into the target dir. |  |
| tests/frontend_test.rs::converts_json_to_json_with_target_dir | src/frontend.rs::run_conversion |  |  | JSON-to-JSON conversion with --target-dir rewrites the document into the target dir. |  |
| tests/frontend_test.rs::json_to_json_in_place_is_rejected_by_collision_guard | src/frontend.rs::run_conversion |  |  | an in-place JSON-to-JSON conversion is rejected by the output-collision guard. |  |
| tests/frontend_test.rs::conversion_reports_errors_and_continues_other_files | src/frontend.rs::run_conversion |  |  | a batch conversion reports per-file errors and still processes the remaining files. |  |
| tests/frontend_test.rs::conversion_rejects_nonexistent_path | src/frontend.rs::run_conversion |  |  | converting a nonexistent path is reported as an error, not a panic. |  |
| tests/frontend_test.rs::conversion_rejects_directory_without_matching_files | src/frontend.rs::run_conversion |  |  | converting a directory with no matching files is reported as an error. |  |
| tests/frontend_test.rs::conversion_rejects_invalid_json_input | src/frontend.rs::run_conversion |  |  | converting invalid JSON input is reported as a parse error. |  |
| tests/frontend_test.rs::validate_valid_rfs_file | src/frontend.rs::run_validate |  |  | validating a valid RFS file reports no errors. |  |
| tests/frontend_test.rs::validate_valid_json_file | src/frontend.rs::run_validate |  |  | validating a valid JSON file reports no errors. |  |
| tests/frontend_test.rs::validate_invalid_rfs_file_reports_errors | src/frontend.rs::run_validate |  |  | validating an invalid RFS file reports the semantic errors. |  |
| tests/frontend_test.rs::validate_unparseable_file_reports_error | src/frontend.rs::run_validate |  |  | validating an unparseable file reports a parse error. |  |
| tests/frontend_test.rs::validate_file_with_conflicting_input_format_is_misuse | src/frontend.rs::run_validate |  |  | validating a single file with an explicit conflicting input format is a misuse error. |  |
| tests/frontend_test.rs::validate_file_with_unknown_extension_is_misuse | src/frontend.rs::run_validate |  |  | validating a single file with an unknown extension is a misuse error. |  |
| tests/frontend_test.rs::validate_nonexistent_path_is_misuse | src/frontend.rs::run_validate |  |  | validating a nonexistent path is a misuse error. |  |
| tests/frontend_test.rs::validate_directory_requires_input_format | src/frontend.rs::run_validate |  |  | validating a directory without an explicit input format is a misuse error. |  |
| tests/frontend_test.rs::validate_directory_writes_err_files_next_to_failures | src/frontend.rs::run_validate |  |  | directory validation writes a .rfs-validation.err file next to each failing RFS source. |  |
| tests/frontend_test.rs::validate_json_directory_writes_err_files_with_json_name | src/frontend.rs::run_validate |  |  | directory validation names the err file after the JSON source for JSON inputs. |  |
| tests/frontend_test.rs::validate_directory_all_valid_has_no_err_files | src/frontend.rs::run_validate |  |  | a directory of all-valid specs produces no err files. |  |
| tests/frontend_test.rs::positive_specs_pass_validation | src/frontend.rs::run_validate |  |  | every positive example spec passes validation with no errors. |  |
| tests/frontend_test.rs::negative_specs_fail_validation_with_messages | src/frontend.rs::run_validate |  |  | every negative example spec fails validation with a non-empty diagnostic message. |  |
| tests/frontend_test.rs::rfs_bin |  |  |  | locate the compiled rfs test binary (panics if the build is missing). |  |
| tests/frontend_test.rs::e2e_parse_rfs_to_json_exits_zero_and_writes_output | src/parser.rs::parse_source |  |  | the rfs parse subcommand converts RFS to JSON, exits 0, and writes the output file. |  |
| tests/frontend_test.rs::e2e_parse_in_place_same_format_exits_one |  |  |  | rfs parse in place to the same format exits 1 with a usage error. |  |
| tests/frontend_test.rs::e2e_parse_missing_flags_exits_one |  |  |  | rfs parse with missing required flags exits 1. |  |
| tests/frontend_test.rs::e2e_validate_valid_file_exits_zero |  |  |  | rfs validate on a valid file exits 0. |  |
| tests/frontend_test.rs::e2e_validate_invalid_file_exits_one_with_stderr |  |  |  | rfs validate on an invalid file exits 1 and prints diagnostics to stderr. |  |
| tests/frontend_test.rs::e2e_validate_directory_writes_err_file_and_exits_one |  |  |  | rfs validate on a directory writes the err file and exits 1. |  |
| tests/frontend_test.rs::ir_json_golden_compact_pins_key_order | src/parser.rs::parse_source |  |  | the compact IR JSON golden pins the exact key order of the wire format. |  |
| tests/frontend_test.rs::ir_json_golden_pretty_pins_layout | src/parser.rs::parse_source |  |  | the pretty IR JSON golden pins the exact whitespace/layout of the wire format. |  |
| tests/frontend_test.rs::ir_version_read_policy_v0_loads_future_rejected | src/json_rw.rs::Json, src/json_rw.rs::parse, src/parser.rs::parse_source |  |  | a v0 reader loads its own version but rejects a future ir_version. |  |
| tests/frontend_test.rs::ir_json_roundtrip_property_all_valid_examples | src/lib.rs::collect_rfs_files, src/parser.rs::parse_source |  |  | every valid example round-trips through to_json/from_json to an identical IR. |  |
| tests/frontend_test.rs::example_root_dir |  |  |  | the examples/valid directory (the corpus for the info sweeps). |  |
| tests/frontend_test.rs::info_text_on_all_valid_examples | src/info.rs::InfoOptions, src/info.rs::run_info, src/lib.rs::collect_rfs_files |  |  | rfs info in text form runs over every valid example without error. |  |
| tests/frontend_test.rs::info_json_parses_and_matches_pinned_example | src/info.rs::InfoOptions, src/info.rs::run_info, src/json_rw.rs::Json, src/json_rw.rs::parse, src/parser.rs::parse_source |  |  | rfs info --json on a pinned example parses and matches the expected structure. |  |
| tests/frontend_test.rs::e2e_info_valid_exits_zero_and_prints_module |  |  |  | the rfs info subcommand on a valid spec exits 0 and prints the module summary. |  |
| tests/frontend_test.rs::e2e_info_invalid_spec_exits_one_with_diagnostics |  |  |  | rfs info on an invalid spec exits 1 with diagnostics. |  |
| tests/frontend_test.rs::e2e_info_unparseable_spec_exits_one |  |  |  | rfs info on an unparseable spec exits 1. |  |
| tests/frontend_test.rs::e2e_info_log_level_filters_stderr_progress |  |  |  | rfs info's --log-level filters the stderr progress output. |  |
| tests/frontend_test.rs::e2e_info_usage_lists_info_and_rejects_unknown_option |  |  |  | rfs info usage lists the subcommand and rejects unknown options. |  |
| tests/frontend_test.rs::diagram_opts | src/diagram.rs::DiagramOptions |  |  | build the default DiagramOptions for an in-process run_diagram call. |  |
| tests/frontend_test.rs::diagram_stdout | src/diagram.rs::run_diagram |  |  | run run_diagram in-process and return its stdout (panics on a non-Ok result). |  |
| tests/frontend_test.rs::diagram_pinned_dot_output |  |  |  | the DOT renderer's output on the pinned spec matches the byte-pinned expectation. |  |
| tests/frontend_test.rs::diagram_pinned_puml_output |  |  |  | the PlantUML renderer's output on the pinned spec matches the byte-pinned expectation. |  |
| tests/frontend_test.rs::diagram_pinned_mmd_output |  |  |  | the Mermaid renderer's output on the pinned spec matches the byte-pinned expectation. |  |
| tests/frontend_test.rs::diagram_pinned_table_output |  |  |  | the table renderer's output on the pinned spec matches the byte-pinned expectation. |  |
| tests/frontend_test.rs::diagram_pinned_md_output |  |  |  | the markdown renderer's output on the pinned spec matches the byte-pinned expectation. |  |
| tests/frontend_test.rs::diagram_all_formats_sweep_valid_examples | src/diagram.rs::run_diagram, src/lib.rs::collect_rfs_files |  |  | every diagram format renders every valid example without error. |  |
| tests/frontend_test.rs::tool_on_path |  |  |  | true when a bare tool name resolves on PATH (for gating real-tool tests). |  |
| tests/frontend_test.rs::write_stub |  |  |  | write a stub executable that records its argv/env for the tool-precedence proofs. |  |
| tests/frontend_test.rs::e2e_diagram_valid_exits_zero_and_prints |  |  |  | the rfs diagram subcommand on a valid spec exits 0 and prints the diagram. |  |
| tests/frontend_test.rs::e2e_diagram_invalid_spec_exits_one |  |  |  | rfs diagram on an invalid spec exits 1. |  |
| tests/frontend_test.rs::e2e_diagram_render_dot_real_tool |  |  |  | rfs diagram --render svg with a real dot tool produces an SVG (gated on dot being present). |  |
| tests/frontend_test.rs::e2e_diagram_render_puml_real_tool |  |  |  | rfs diagram --render svg with a real plantuml tool produces an SVG (gated). |  |
| tests/frontend_test.rs::e2e_diagram_render_mmd_real_tool |  |  |  | rfs diagram --render svg with a real mmdc tool produces an SVG (gated). |  |
| tests/frontend_test.rs::diagram_puml_negative_control_exit_200 | src/diagrams/plantuml.rs::render_svg, src/ext_tool.rs::ExtTool |  |  | a PlantUML syntax error surfaces as the pinned exit code 200. |  |
| tests/frontend_test.rs::diagram_mmd_negative_control_parse_error | src/diagrams/mermaid.rs::render_svg, src/ext_tool.rs::ExtTool |  |  | a Mermaid parse error surfaces with the pinned diagnostic. |  |
| tests/frontend_test.rs::clear_tool_env |  |  |  | clear every diagram tool env var so ambient settings can't leak in. |  |
| tests/frontend_test.rs::diag_lock |  |  |  | take the diagram env lock, recovering from poisoning so one panicking test can't wedge the rest. |  |
| tests/frontend_test.rs::set_env |  |  |  | set a diagram tool env var under the diag lock. |  |
| tests/frontend_test.rs::e2e_diagram_stub_plantuml_receives_env_and_cli_args |  |  |  | a stub plantuml receives the expected env vars and CLI args from rfs diagram. |  |
| tests/frontend_test.rs::e2e_diagram_stub_dot_receives_env_and_cli_args |  |  |  | a stub dot receives the expected env vars and CLI args from rfs diagram. |  |
| tests/frontend_test.rs::e2e_diagram_flag_path_beats_env_for_mermaid |  |  |  | the --mermaid flag path takes precedence over the MERMAID env var. |  |
| tests/frontend_test.rs::e2e_diagram_missing_dot_tool_exits_one |  |  |  | rfs diagram with no dot tool available exits 1 with a clear error. |  |
| tests/frontend_test.rs::e2e_diagram_failing_plantuml_propagates_stderr |  |  |  | a failing plantuml stub's stderr is propagated by rfs diagram. |  |
| tests/frontend_test.rs::e2e_diagram_failing_mmdc_exits_one |  |  |  | a failing mmdc stub makes rfs diagram exit 1. |  |
| tests/frontend_test.rs::print_script_of |  |  |  | extract the --print-script output (the exact external command) from a diagram run. |  |
| tests/frontend_test.rs::e2e_diagram_print_script_puml_render_pinned |  |  |  | --print-script for a PlantUML render pins the exact external command. |  |
| tests/frontend_test.rs::e2e_diagram_print_script_mmd_render_pinned |  |  |  | --print-script for a Mermaid render pins the exact external command. |  |
| tests/frontend_test.rs::e2e_diagram_print_script_no_tool_pinned |  |  |  | --print-script with no tool installed still pins the would-be command. |  |
| tests/frontend_test.rs::e2e_diagram_list_pinned |  |  |  | rfs diagram --list pins the list of available formats/renderers. |  |
| tests/frontend_test.rs::e2e_diagram_rejects_unknown_format |  |  |  | rfs diagram rejects an unknown format argument. |  |
| tests/frontend_test.rs::e2e_diagram_render_requires_out_svg |  |  |  | rfs diagram --render svg requires an --out path and errors without one. |  |
| tests/frontend_test.rs::e2e_diagram_table_md_have_no_renderer |  |  |  | the table and md formats have no SVG renderer (render is rejected for them). |  |
| tests/frontend_test.rs::e2e_diagram_render_mmd_internal_default |  |  |  | rfs diagram --render svg defaults to the internal Mermaid renderer when no tool is set. |  |
| tests/frontend_test.rs::diagram_internal_mmd_sweep_valid_examples | src/diagram.rs::DiagramOptions, src/diagram.rs::Renderer, src/diagram.rs::run_diagram, src/lib.rs::collect_rfs_files |  |  | the internal Mermaid renderer renders every valid example to SVG without error. |  |
| tests/frontend_test.rs::diagram_mmd_internal_negative_control_rejects_garbage | src/diagrams/mermaid.rs::render_svg_internal |  |  | the internal Mermaid renderer rejects garbage input with a pinned error. |  |
| tests/frontend_test.rs::e2e_diagram_print_script_mmd_internal_pinned |  |  |  | --print-script for the internal Mermaid renderer pins its command form. |  |
| tests/frontend_test.rs::e2e_diagram_renderer_flag_rejects_unknown_value |  |  |  | the --renderer flag rejects an unknown value. |  |
| tests/frontend_test.rs::e2e_diagram_render_puml_internal_default |  |  |  | rfs diagram --render svg defaults to the internal PlantUML renderer when no tool is set. |  |
| tests/frontend_test.rs::diagram_internal_puml_sweep_valid_examples | src/diagram.rs::DiagramOptions, src/diagram.rs::Renderer, src/diagram.rs::run_diagram, src/lib.rs::collect_rfs_files |  |  | the internal PlantUML renderer renders every valid example to SVG without error. |  |
| tests/frontend_test.rs::diagram_puml_internal_negative_control_rejects_garbage | src/diagrams/plantuml.rs::render_svg_internal |  |  | the internal PlantUML renderer rejects garbage input with a pinned error. |  |
| tests/frontend_test.rs::e2e_diagram_print_script_puml_internal_pinned |  |  |  | --print-script for the internal PlantUML renderer pins its command form. |  |
| tests/frontend_test.rs::e2e_diagram_render_dot_internal_default |  |  |  | rfs diagram --render svg defaults to the internal DOT renderer when no tool is set. |  |
| tests/frontend_test.rs::diagram_internal_dot_sweep_valid_examples | src/diagram.rs::DiagramOptions, src/diagram.rs::Renderer, src/diagram.rs::run_diagram, src/lib.rs::collect_rfs_files |  |  | the internal DOT renderer renders every valid example to SVG without error. |  |
| tests/frontend_test.rs::diagram_dot_internal_negative_control_rejects_garbage | src/diagrams/dot.rs::render_svg_internal |  |  | the internal DOT renderer rejects garbage input with a pinned error. |  |
| tests/frontend_test.rs::e2e_diagram_print_script_dot_internal_pinned |  |  |  | --print-script for the internal DOT renderer pins its command form. |  |
| tests/frontend_test.rs::test_lock |  |  |  | the rfs-test tool-env lock (serializes tests that mutate the environment). |  |
| tests/frontend_test.rs::clear_test_env |  |  |  | clear the rfs-test tool env vars so ambient settings can't leak in. |  |
| tests/frontend_test.rs::e2e_test_passing_spec_both_backends_exit_zero |  |  |  | rfs test on a passing spec exits 0 for both the rust and c backends. |  |
| tests/frontend_test.rs::e2e_test_failing_variant_exits_one |  |  |  | rfs test on a failing variant exits 1. |  |
| tests/frontend_test.rs::e2e_test_print_script_pinned_and_no_execution |  |  |  | rfs test --print-script pins the generated test command and does not execute it. |  |
| tests/frontend_test.rs::e2e_test_tool_precedence_stub_tools |  |  |  | stub cargo/rustc tools prove rfs test's tool-resolution precedence. |  |
| tests/frontend_test.rs::e2e_test_usage_and_parse_errors |  |  |  | rfs test usage and argument-parse errors are pinned. |  |
| tests/frontend_test.rs::clear_fuzz_env |  |  |  | clear the task 17 fuzz tool env vars so ambient settings can't leak in. |  |
| tests/frontend_test.rs::clear_all_test_env |  |  |  | clear both the rfs-test and fuzz tool env vars. |  |
| tests/frontend_test.rs::e2e_fuzz_suite_selection_pinned |  |  |  | rfs test --suite fuzz selection behaviour is pinned (which suites run per backend). |  |
| tests/frontend_test.rs::e2e_fuzz_missing_tools_pinned |  |  |  | rfs test --suite fuzz with missing tools produces the pinned error. |  |
| tests/frontend_test.rs::e2e_fuzz_real_tools_both_backends_exit_zero |  |  |  | rfs test --suite fuzz with real tools exits 0 for both backends (gated). |  |
| tests/frontend_test.rs::e2e_fuzz_crash_stub_exits_one |  |  |  | a crashing fuzz stub makes rfs test --suite fuzz exit 1. |  |
| tests/frontend_test.rs::e2e_fuzz_stub_argv_proofs |  |  |  | stub fuzz tools prove the exact argv rfs test --suite fuzz passes them. |  |
| tests/frontend_test.rs::e2e_fuzz_print_script_pinned_and_no_execution |  |  |  | rfs test --suite fuzz --print-script pins the command and does not execute it. |  |
| tests/frontend_test.rs::e2e_validate_json_parse_error_shape_and_position | src/json_rw.rs::Json, src/json_rw.rs::parse, src/parser.rs::parse_source |  |  | validate --json on a parse error pins the JSON error shape and source position. |  |
| tests/frontend_test.rs::e2e_validate_json_semantic_error_shape_pinned | src/json_rw.rs::Json, src/json_rw.rs::parse |  |  | validate --json on a semantic error pins the JSON error shape. |  |
| tests/frontend_test.rs::e2e_validate_json_ok_true_on_valid_file | src/json_rw.rs::Json, src/json_rw.rs::parse |  |  | validate --json on a valid file reports ok=true. |  |
| tests/frontend_test.rs::e2e_validate_json_directory_mode_array | src/json_rw.rs::Json, src/json_rw.rs::parse |  |  | validate --json in directory mode emits an array of per-file results. |  |
| tests/frontend_test.rs::ide_grammar_valid_json_and_covers_all_keywords | src/json_rw.rs::Json, src/json_rw.rs::parse |  |  | the IDE grammar export is valid JSON and covers every RFS keyword. |  |
| tests/frontend_test.rs::rustc_on_path |  |  |  | true when rustc resolves on PATH (for gating the generated-binary agreement test). |  |
| tests/frontend_test.rs::mc_ref_run | src/mc_ref_cmd.rs::McRefOptions, src/mc_ref_cmd.rs::run_mc_ref |  |  | run the mc-ref CLI in-process and return its exit code + stdout + stderr. |  |
| tests/frontend_test.rs::mc_ref_exit_codes_pinned_for_all_three_statuses | src/mc_ref_cmd.rs::McRefOptions |  |  | mc-ref exit codes are pinned for all three MC statuses (pass/fail/unknown). |  |
| tests/frontend_test.rs::mc_ref_flag_errors_are_usage_failures | src/mc_ref_cmd.rs::run_cli |  |  | mc-ref flag errors are usage failures (pinned exit code + message). |  |
| tests/frontend_test.rs::mc_ref_output_agrees_with_generated_binary_text_and_json | src/codegen.rs::BackendOptions, src/codegen.rs::get_backend, src/mc_ref_cmd.rs::CeFormat, src/mc_ref_cmd.rs::McRefOptions, src/parser.rs::parse_file, tests/common/mc_json.rs::zero_peak_rss |  |  | mc-ref text and JSON output agree with the generated MC binary's output. |  |
| tests/frontend_test.rs::gen_spec_run | src/gen_spec_cmd.rs::run_cli |  |  | run the gen-spec CLI in-process and return its exit code + stdout + stderr. |  |
| tests/frontend_test.rs::gen_spec_seed_determinism_same_args_byte_identical |  |  |  | gen-spec with the same seed/args is byte-identical across runs. |  |
| tests/frontend_test.rs::gen_spec_base_mutations_reproducible | src/parser.rs::parse_source |  |  | gen-spec base mutations are reproducible for a pinned seed. |  |
| tests/frontend_test.rs::gen_spec_output_validity_sweep_20_seeds_both_modes | src/parser.rs::parse_source, src/validator.rs::validate_ir |  |  | gen-spec output is valid for 20 seeds in both generation modes. |  |
| tests/frontend_test.rs::gen_spec_error_cases_pinned |  |  |  | gen-spec error cases (bad flags/seed) are pinned. |  |
| tests/library_comparison.rs::nom_smoke::ident_start |  |  |  | nom: match an identifier's first character (alphabetic or _). |  |
| tests/library_comparison.rs::nom_smoke::ident_cont |  |  |  | nom: match an identifier's continuation character (alphanumeric or _). |  |
| tests/library_comparison.rs::nom_smoke::identifier | tests/library_comparison.rs::nom_smoke::ident_start, tests/library_comparison.rs::nom_smoke::ident_cont |  |  | nom: parse a full identifier (first char then many0 continuation chars). |  |
| tests/library_comparison.rs::nom_smoke::module_header | tests/library_comparison.rs::nom_smoke::identifier |  |  | nom: parse a `module <name>` header (whitespace, keyword, identifier). |  |
| tests/library_comparison.rs::nom_smoke::nom_can_parse_module_header | tests/library_comparison.rs::nom_smoke::module_header |  |  | nom smoke test: a well-formed module header parses to its name. |  |
| tests/library_comparison.rs::nom_smoke::nom_reports_errors_for_bad_input | tests/library_comparison.rs::nom_smoke::module_header |  |  | nom smoke test: malformed input yields a parse error. |  |
| tests/library_comparison.rs::chumsky_smoke::module_header |  |  |  | chumsky: build the `module <name>` parser (keyword then identifier). |  |
| tests/library_comparison.rs::chumsky_smoke::chumsky_can_parse_module_header | tests/library_comparison.rs::chumsky_smoke::module_header |  |  | chumsky smoke test: a well-formed module header parses to its name. |  |
| tests/library_comparison.rs::chumsky_smoke::chumsky_reports_errors_for_bad_input | tests/library_comparison.rs::chumsky_smoke::module_header |  |  | chumsky smoke test: malformed input yields a parse error. |  |
| tests/info_format_test.rs::rfs_bin |  |  |  | locate the compiled rfs test binary (panics if the build is missing). |  | 
| tests/info_format_test.rs::root |  |  |  | the repo root (CARGO_MANIFEST_DIR). |  | 
| tests/info_format_test.rs::run_info_format |  |  |  | run `rfs info format` in-process and return its result. |  | 
| tests/info_format_test.rs::stdout_of |  |  |  | extract stdout from an info-format run. |  | 
| tests/info_format_test.rs::stderr_of |  |  |  | extract stderr from an info-format run. |  | 
| tests/docs_md_test.rs::check_markdown::walk | tests/docs_md_test.rs::check_link |  |  | recursively walk the comrak AST checking fence closure, mermaid rendering, and links. |  | 
| tests/info_format_test.rs::list_pins_all_registered_formats |  |  |  | `rfs info format --list` pins the full set of registered formats. |  | 
| tests/info_format_test.rs::spec_rfs_md_pins_all_rules_and_keywords |  |  |  | the spec-rfs markdown output pins every grammar rule and keyword. |  | 
| tests/info_format_test.rs::spec_rfs_plantuml_pins_structure |  |  |  | the spec-rfs PlantUML output pins the diagram structure. |  | 
| tests/info_format_test.rs::spec_rfs_svg_pins_labels |  |  |  | the spec-rfs SVG output pins the rendered labels. |  | 
| tests/info_format_test.rs::design_md_fence_rule_names |  |  |  | extract the rule names from the ```ebnf fence in DESIGN.md section 6. |  | 
| tests/info_format_test.rs::design_md_section6_rule_names_match_model |  |  |  | DESIGN.md section 6's rule names match the EBNF grammar model exactly. |  | 
| tests/info_format_test.rs::collect_nonterminals | src/ebnf_grammar.rs::RuleNode |  |  | collect every non-terminal rule name referenced in the grammar model. |  | 
| tests/info_format_test.rs::no_dangling_nonterminals |  |  |  | every non-terminal referenced by the grammar model is itself defined (no dangling rules). |  | 
| tests/info_format_test.rs::keywords_appear_as_terminals_in_model | src/ebnf_grammar.rs::RuleNode |  |  | every RFS keyword appears as a terminal in the EBNF grammar model. |  | 
| tests/info_format_test.rs::keywords_appear_as_terminals_in_model::collect_terminals | src/ebnf_grammar.rs::RuleNode |  |  | recursively collect every terminal string from a grammar RuleNode tree. |  | 
| tests/info_format_test.rs::spec_json_schema_document_pin | src/info_format.rs::spec_json_schema, src/json_rw.rs::parse |  |  | the spec-json draft-07 schema document is byte-pinned. |  | 
| tests/info_format_test.rs::spec_json_md_pins_all_sections | src/info_format.rs::spec_json_schema |  |  | the spec-json markdown output pins all its sections. |  | 
| tests/info_format_test.rs::spec_json_walk_all_valid_examples | src/info_format.rs::spec_json_schema, src/json_schema.rs::walk_check, src/lib.rs::collect_rfs_files, src/parser.rs::parse_file |  |  | every valid example's spec-json output walks cleanly against the embedded model. |  | 
| tests/info_format_test.rs::ce_trace_schema_document_pin | src/json_rw.rs::parse |  |  | the ce-trace draft-07 schema document is byte-pinned. |  | 
| tests/info_format_test.rs::ce_trace_md_pins_sections | src/info_format.rs::ce_trace_schema |  |  | the ce-trace markdown output pins its sections. |  | 
| tests/info_format_test.rs::rustc_path |  |  |  | locate rustc for compiling the exact-mode MC binaries. |  | 
| tests/info_format_test.rs::ce_mc_binary | src/codegen.rs::BackendOptions, src/codegen.rs::CodegenCommandKind, src/codegen.rs::get_backend, src/parser.rs::parse_source |  |  | codegen + compile one exact-mode MC binary for a spec (cached per key so the suite stays fast). |  | 
| tests/info_format_test.rs::mc_report_of | src/json_rw.rs::Json, src/json_rw.rs::parse |  |  | one --counterexample-format json run returning the full report document. |  | 
| tests/info_format_test.rs::ce_object | src/json_rw.rs::Json |  |  | one --counterexample-format json run of a FAILING binary returning the report's ce object. |  | 
| tests/info_format_test.rs::walk_ce | src/info_format.rs::ce_trace_schema, src/json_rw.rs::Json, src/json_schema.rs::walk_check |  |  | the D-P7-9 walk against the ce-trace model (required keys present, no undocumented keys). |  | 
| tests/info_format_test.rs::ce_trace_walk_trace_and_lasso_both_backends | src/json_rw.rs::Json |  |  | the ce-trace walk passes for trace and lasso counterexamples on both backends. |  | 
| tests/info_format_test.rs::ce_trace_walk_deadlock_config_both_backends | src/json_rw.rs::Json, tests/common/fail_variants.rs::spec_source |  |  | the ce-trace walk passes for deadlock-config counterexamples on both backends. |  | 
| tests/info_format_test.rs::ce_trace_walk_ref_mc_in_process | src/codegen_mc.rs::ExactStatus, src/json_rw.rs::Json, src/parser.rs::parse_source, src/ref_mc.rs::RefConfig, src/ref_mc.rs::run, src/ref_mc.rs::to_json_report, tests/common/fail_variants.rs::spec_source |  |  | the ce-trace walk passes against the in-process reference MC's output. |  | 
| tests/info_format_test.rs::prop_names | src/json_rw.rs::Json |  |  | the property names for a spec (helper for the mc-report walks). |  | 
| tests/info_format_test.rs::enum_vals | src/json_rw.rs::Json |  |  | the enum values for a spec (helper for the mc-report walks). |  | 
| tests/info_format_test.rs::walk_mc_report | src/info_format.rs::mc_report_schema, src/json_rw.rs::Json, src/json_schema.rs::walk_check |  |  | the D-P7-9 walk against the mc-report model (required keys present, no undocumented keys). |  | 
| tests/info_format_test.rs::mc_report_schema_document_pin | src/codegen_mc.rs::ExactStatus, src/codegen_mc.rs::FailReason, src/info_format.rs::ce_trace_schema, src/json_rw.rs::parse, src/json_schema.rs::emit_type |  |  | the mc-report draft-07 schema document is byte-pinned. |  | 
| tests/info_format_test.rs::mc_report_md_pins_sections | src/info_format.rs::mc_report_schema |  |  | the mc-report markdown output pins its sections. |  | 
| tests/info_format_test.rs::mc_report_walk_benchmarks_both_backends | src/json_rw.rs::Json, tests/common/fail_variants.rs::can_passive_fail_src, tests/common/fail_variants.rs::http2_rst_fail_src, tests/common/fail_variants.rs::spec_source |  |  | the mc-report walk passes for the benchmark specs on both backends. |  | 
| tests/info_format_test.rs::mc_report_walk_ref_mc_in_process | src/codegen_mc.rs::ExactStatus, src/parser.rs::parse_source, src/ref_mc.rs::RefConfig, src/ref_mc.rs::run, src/ref_mc.rs::to_json_report, tests/common/fail_variants.rs::spec_source |  |  | the mc-report walk passes against the in-process reference MC's output. |  | 
| tests/info_format_test.rs::validate_json_schema_document_pin | src/json_rw.rs::parse |  |  | the validate-json draft-07 schema document is byte-pinned. |  | 
| tests/info_format_test.rs::validate_json_md_pins_sections | src/info_format.rs::validate_json_schema |  |  | the validate-json markdown output pins its sections. |  | 
| tests/info_format_test.rs::walk_validate_json | src/json_rw.rs::Json, src/json_schema.rs::SchemaDoc, src/json_schema.rs::walk_check |  |  | the D-P7-9 walk against the validate-json model (required keys present, no undocumented keys). |  | 
| tests/info_format_test.rs::validate_json_walk_invalid_examples_both_kinds | src/frontend.rs::render_validate_json, src/frontend.rs::run_validate_structured, src/info_format.rs::validate_json_schema, src/json_rw.rs::parse, src/lib.rs::collect_rfs_files |  |  | the validate-json walk passes for invalid examples of both error kinds. |  | 
| tests/info_format_test.rs::validate_json_walk_valid_files_ok_true | src/frontend.rs::render_validate_json, src/frontend.rs::run_validate_structured, src/info_format.rs::validate_json_schema, src/json_rw.rs::parse, src/lib.rs::collect_rfs_files |  |  | the validate-json walk reports ok=true for valid files. |  | 
| tests/info_format_test.rs::validate_json_walk_directory_mode | src/frontend.rs::Format, src/frontend.rs::render_validate_json, src/frontend.rs::run_validate_structured, src/info_format.rs::validate_json_schema, src/json_rw.rs::parse |  |  | the validate-json walk handles directory mode (an array of per-file results). |  | 
| tests/info_format_test.rs::info_json_schema_document_pin | src/json_rw.rs::parse |  |  | the info-json draft-07 schema document is byte-pinned. |  | 
| tests/info_format_test.rs::info_json_md_pins_sections | src/info_format.rs::info_json_schema |  |  | the info-json markdown output pins its sections. |  | 
| tests/info_format_test.rs::info_json_walk_all_valid_examples | src/info.rs::InfoOptions, src/info.rs::run_info, src/info_format.rs::info_json_schema, src/json_rw.rs::Json, src/json_rw.rs::parse, src/json_schema.rs::walk_check, src/lib.rs::collect_rfs_files, src/parser.rs::parse_file |  |  | every valid example's info-json output walks cleanly against the embedded model. |  | 
| tests/info_format_test.rs::diagram_dot_md_pins_sections |  |  |  | the diagram-dot markdown output pins its sections. |  | 
| tests/info_format_test.rs::check_dot_documented_subset | src/diagrams/mod.rs::event_label, src/diagrams/mod.rs::guard_label, src/ir.rs::Module |  |  | the coverage check for the diagram-dot format (D-P7-9 style): every documented line appears in generated output. |  | 
| tests/info_format_test.rs::diagram_dot_generated_output_matches_documented_subset | src/diagrams/dot.rs::generate, src/ir.rs::PatternVariant, src/lib.rs::collect_rfs_files, src/parser.rs::parse_file |  |  | the generated DOT output matches the documented subset exactly. |  | 
| tests/info_format_test.rs::diagram_puml_md_pins_sections |  |  |  | the diagram-puml markdown output pins its sections. |  | 
| tests/info_format_test.rs::check_puml_documented_subset | src/diagrams/mod.rs::emit_labels, src/diagrams/mod.rs::event_label, src/diagrams/mod.rs::guard_label, src/diagrams/mod.rs::states, src/ir.rs::Module |  |  | the coverage check for the diagram-puml format (D-P7-9 style): every documented line appears in generated output. |  | 
| tests/info_format_test.rs::diagram_puml_generated_output_matches_documented_subset | src/diagrams/plantuml.rs::generate, src/ir.rs::PatternVariant, src/lib.rs::collect_rfs_files, src/parser.rs::parse_file |  |  | the generated PlantUML output matches the documented subset exactly. |  | 
| tests/info_format_test.rs::diagram_mmd_md_pins_sections |  |  |  | the diagram-mmd markdown output pins its sections. |  | 
| tests/info_format_test.rs::check_mmd_documented_subset | src/diagrams/mod.rs::emit_labels, src/diagrams/mod.rs::event_label, src/diagrams/mod.rs::guard_label, src/diagrams/mod.rs::states, src/ir.rs::Module |  |  | the coverage check for the diagram-mmd format (D-P7-9 style): every documented line appears in generated output. |  | 
| tests/info_format_test.rs::diagram_mmd_generated_output_matches_documented_subset | src/diagrams/mermaid.rs::generate, src/ir.rs::PatternVariant, src/lib.rs::collect_rfs_files, src/parser.rs::parse_file |  |  | the generated Mermaid output matches the documented subset exactly. |  | 
| tests/info_format_test.rs::artifact_header_md_pins_sections |  |  |  | the artifact-header markdown output pins its sections. |  | 
| tests/info_format_test.rs::expected_header_line | src/ir.rs::Ir, src/spec_hash.rs::spec_hash16 |  |  | the documented line-1 template filled with the IR data (D-P7-9 style). |  | 
| tests/info_format_test.rs::artifact_header_generated_output_matches_documented_format | src/codegen.rs::BackendOptions, src/codegen.rs::CodegenCommandKind, src/codegen.rs::get_backend, src/lib.rs::collect_rfs_files, src/parser.rs::parse_source |  |  | the generated artifact header matches the documented format, including hash variation. |  | 
| tests/info_format_test.rs::unknown_format_errors_with_list_hint |  |  |  | an unknown `rfs info format` argument errors with a --list hint. |  | 
| tests/info_format_test.rs::unsupported_output_errors |  |  |  | an unsupported output combination errors cleanly. |  | 
| tests/info_format_test.rs::no_name_and_no_list_errors |  |  |  | calling `rfs info format` with neither a name nor --list errors. |  | 
| tests/docs_md_test.rs::mdr_options |  |  |  | the GFM extension set mdr enables (tables, task lists, strikethrough, footnotes, autolinks). |  | 
| tests/docs_md_test.rs::url_decode |  |  |  | minimal percent-decoding for link targets (%20 -> space, ...). |  | 
| tests/docs_md_test.rs::hex_val |  |  |  | decode one hex digit to a u8 (None if not a hex digit). |  | 
| tests/docs_md_test.rs::check_link |  |  |  | check one link/image URL - skip external/fragment targets, resolve relative ones against the doc dir. |  | 
| tests/docs_md_test.rs::check_markdown |  |  |  | run all document-quality checks over one Markdown source, returning the problem list. |  | 
| tests/docs_md_test.rs::collect_docs |  |  |  | collect every .md file in the repo, skipping target/, __pycache__/, and hidden dirs. |  | 
| tests/docs_md_test.rs::collect_docs::walk_dir |  |  |  | recursively walk a directory collecting .md files. |  | 
| tests/docs_md_test.rs::all_repo_docs_pass_quality_checks |  |  |  | assert every repo .md passes the quality checks (the main gate). |  | 
| tests/docs_md_test.rs::valid_mermaid_diagrams_and_links_pass |  |  |  | negative-control fixture: valid mermaid + links pass the checker. |  | 
| tests/docs_md_test.rs::unterminated_fence_is_detected |  |  |  | negative-control fixture: an unterminated fence is detected. |  | 
| tests/docs_md_test.rs::non_rendering_mermaid_is_detected |  |  |  | negative-control fixture: a non-rendering mermaid block is detected. |  | 
| tests/docs_md_test.rs::dead_relative_link_is_detected |  |  |  | negative-control fixture: a dead relative link is detected. |  |
| tests/codegen_test.rs::files | src/ir.rs::Ir |  |  | test-data helper: the valid example .rfs file paths used across the codegen sweeps. |  |
| tests/codegen_test.rs::parse_module | src/ir.rs::Ir, src/parser.rs::parse_file |  |  | parse one .rfs file into an Ir (panics with the error on failure). |  |
| tests/codegen_test.rs::registry_lists_rust_backend | src/codegen.rs::get_backend, src/codegen.rs::list_backends |  |  | the backend registry lists the rust backend with all of its commands. |  |
| tests/codegen_test.rs::registry_help_texts_are_nonempty | src/codegen.rs::backend_help, src/codegen.rs::common_help, src/codegen.rs::get_backend |  |  | every backend and command help text is non-empty. |  |
| tests/codegen_test.rs::all_rust_commands_generate_for_valid_examples | src/codegen.rs::AssumptionSelection, src/codegen.rs::BackendOptions, src/codegen.rs::CodegenCommandKind, src/codegen.rs::get_backend |  |  | every rust codegen command generates successfully for all valid examples. |  |
| tests/codegen_test.rs::rust_production_flag_toggles_property_monitors | src/codegen.rs::AssumptionSelection, src/codegen.rs::BackendOptions, src/codegen.rs::CodegenCommandKind, src/codegen.rs::get_backend |  |  | --with-property-monitors toggles the #[cfg(debug_assertions)] monitor block in rust production output. |  |
| tests/codegen_test.rs::steps_option_changes_test_generation | src/codegen.rs::BackendOptions, src/codegen.rs::CodegenCommandKind, src/codegen.rs::get_backend |  |  | the --test-steps option changes the generated random-trace test. |  |
| tests/codegen_test.rs::argss |  |  |  | helper: split a CLI argument string into the Vec passed to parse_codegen_args. |  |
| tests/codegen_test.rs::cli_parses_list_and_help | src/codegen.rs::CodegenRequest, src/codegen.rs::parse_codegen_args |  |  | the CLI parses `list` and `help` requests. |  |
| tests/codegen_test.rs::cli_parses_backend_help | src/codegen.rs::CodegenRequest, src/codegen.rs::parse_codegen_args |  |  | the CLI parses backend-specific help requests. |  |
| tests/codegen_test.rs::cli_parses_generate_with_options | src/codegen.rs::AssumptionSelection, src/codegen.rs::CodegenCommandKind, src/codegen.rs::CodegenRequest, src/codegen.rs::parse_codegen_args |  |  | the CLI parses a generate request carrying all of its options. |  |
| tests/codegen_test.rs::cli_generate_defaults | src/codegen.rs::AssumptionSelection, src/codegen.rs::CodegenCommandKind, src/codegen.rs::CodegenRequest, src/codegen.rs::parse_codegen_args |  |  | a bare generate request fills in the documented defaults. |  |
| tests/codegen_test.rs::cli_parses_assume_labels | src/codegen.rs::AssumptionSelection, src/codegen.rs::CodegenRequest, src/codegen.rs::parse_codegen_args |  |  | the CLI parses --assume labels/all/none into an AssumptionSelection. |  |
| tests/codegen_test.rs::cli_reports_errors | src/codegen.rs::parse_codegen_args |  |  | the CLI reports usage errors for malformed arguments. |  |
| tests/codegen_test.rs::aggregate_codegen_succeeds_for_valid_examples_and_training_snippets | src/backends/rust_test.rs::generate_aggregate_test_crate, src/codegen.rs::AssumptionSelection, src/codegen.rs::BackendOptions |  |  | aggregate test-crate codegen succeeds for all valid examples plus the training snippets. |  |
| tests/codegen_test.rs::model_checker_codegen_contains_main | src/codegen.rs::AssumptionSelection, src/codegen.rs::BackendOptions, src/codegen.rs::CodegenCommandKind, src/codegen.rs::get_backend |  |  | the generated model checker contains a main entry point. |  |
| tests/codegen_test.rs::priority_10_arm_precedes_priority_1_in_generated_step | src/codegen.rs::AssumptionSelection, src/codegen.rs::BackendOptions, src/codegen.rs::CodegenCommandKind, src/codegen.rs::get_backend |  |  | in the generated step, the priority-10 guard arm is checked before the priority-1 arm. |  |
| tests/codegen_test.rs::on_error_fallback_replaces_unexpected_event_error | src/codegen.rs::AssumptionSelection, src/codegen.rs::BackendOptions, src/codegen.rs::CodegenCommandKind, src/codegen.rs::get_backend |  |  | an on_error fallback replaces the unexpected-event error path in the generated code. |  |
| tests/codegen_test.rs::version_header_becomes_spec_version_constant | src/codegen.rs::AssumptionSelection, src/codegen.rs::BackendOptions, src/codegen.rs::CodegenCommandKind, src/codegen.rs::get_backend |  |  | a spec version header becomes the SPEC_VERSION constant in generated code. |  |
| tests/codegen_test.rs::generated_negative_test_asserts_on_error_landing | src/codegen.rs::AssumptionSelection, src/codegen.rs::BackendOptions, src/codegen.rs::CodegenCommandKind, src/codegen.rs::get_backend |  |  | the generated negative test asserts the on_error landing state. |  |
| tests/codegen_test.rs::c_backend_registered_with_all_commands | src/codegen.rs::get_backend, src/codegen.rs::list_backends |  |  | the backend registry lists the c backend with all of its commands. |  |
| tests/codegen_test.rs::all_c_commands_generate_for_valid_examples | src/codegen.rs::AssumptionSelection, src/codegen.rs::BackendOptions, src/codegen.rs::CodegenCommandKind, src/codegen.rs::get_backend, src/spec_hash.rs::spec_hash16 |  |  | every c codegen command generates successfully for all valid examples. |  |
| tests/codegen_test.rs::c_production_flag_toggles_property_monitors | src/codegen.rs::AssumptionSelection, src/codegen.rs::BackendOptions, src/codegen.rs::CodegenCommandKind, src/codegen.rs::get_backend |  |  | --with-property-monitors toggles the monitor block in c production output. |  |
| tests/codegen_test.rs::c_production_matches_rust_semantics_on_endpoint_protocol | src/codegen.rs::AssumptionSelection, src/codegen.rs::BackendOptions, src/codegen.rs::CodegenCommandKind, src/codegen.rs::get_backend |  |  | the c production engine matches rust semantics on the endpoint-protocol example. |  |
| tests/codegen_test.rs::mc_and_test_always_include_property_monitors | src/codegen.rs::AssumptionSelection, src/codegen.rs::BackendOptions, src/codegen.rs::CodegenCommandKind, src/codegen.rs::get_backend, src/codegen.rs::parse_codegen_args |  |  | mc and test codegen always include property monitors (no flag required). |  |
| tests/codegen_test.rs::no_skip_warnings_for_in_subset_properties_both_backends | src/codegen.rs::AssumptionSelection, src/codegen.rs::BackendOptions, src/codegen.rs::CodegenCommandKind, src/codegen.rs::get_backend, src/ir.rs::Formula, src/spec_hash.rs::spec_hash16 |  |  | no skip-warnings are emitted for in-subset properties on both backends. |  |
| tests/codegen_test.rs::formula_has_liveness | src/ir.rs::Formula |  |  | true iff the formula contains an eventually/until operator anywhere (liveness detector). |  |
| tests/codegen_test.rs::liveness_properties_skip_with_identical_warning_both_backends | src/codegen.rs::AssumptionSelection, src/codegen.rs::BackendOptions, src/codegen.rs::CodegenCommandKind, src/codegen.rs::get_backend, src/parser.rs::parse_source |  |  | liveness properties are skipped with an identical warning on both backends. |  |
| tests/codegen_test.rs::mc_exact_flags_embedded_as_runtime_defaults | src/codegen.rs::Backend, src/codegen.rs::BackendOptions, src/codegen.rs::get_backend, src/ir.rs::Ir |  |  | D-P4-7: codegen-time MC flags are embedded as runtime defaults, each changing the generated output as expected. |  |
| tests/codegen_test.rs::mc_exact_flags_embedded_as_runtime_defaults::gen | src/codegen.rs::Backend, src/codegen.rs::BackendOptions, src/codegen.rs::CodegenCommandKind, src/ir.rs::Ir |  |  | helper (nested): run ModelChecker codegen for (ir, backend, opts) and return the generated code string. |  |
| tests/codegen_test.rs::mc_flag_parsing_roundtrip | src/codegen.rs::CodegenRequest, src/codegen.rs::McMode, src/codegen.rs::parse_codegen_args, src/codegen_mc.rs::CounterexampleFormat |  |  | MC flag parsing round-trips - parse then re-emit yields the same embedded defaults. |  |
| tests/codegen_test.rs::mc_json_writer_embedded_and_dependency_free | src/codegen.rs::BackendOptions, src/codegen.rs::CodegenCommandKind, src/codegen.rs::get_backend |  |  | the --counterexample-format json writer is embedded in the generated MC and stays dependency-free. |  |
| tests/codegen_test.rs::mc_enum_input_field_domain_qualified_both_backends | src/codegen.rs::BackendOptions, src/codegen.rs::CodegenCommandKind, src/codegen.rs::get_backend, src/parser.rs::parse_source |  |  | a concrete enum input field generates compiling, domain-qualified engines on both backends (the task-10 fix). |  |
| tests/codegen_test.rs::rfs_bin | src/ir.rs::Ir |  |  | locate the compiled rfs test binary (CARGO_BIN_EXE_rfs). |  |
| tests/codegen_test.rs::example_module | src/ir.rs::Ir |  |  | parse one valid example module into an Ir. |  |
| tests/codegen_test.rs::cli_no_std_flag_parsing | src/codegen.rs::CodegenRequest, src/codegen.rs::parse_codegen_args |  |  | production accepts --no-std (both backends parse it identically). |  |
| tests/codegen_test.rs::cli_no_std_e2e_exit_codes_and_output |  |  |  | --no-std end-to-end produces the right exit codes and no_std output. |  |
| tests/codegen_test.rs::test_mc_entry_points_reject_no_std | src/backends/c_mc.rs::generate_model_checker, src/backends/c_test.rs::generate_unit_tests, src/backends/rust_mc.rs::generate_model_checker, src/backends/rust_test.rs::generate_unit_tests, src/codegen.rs::BackendOptions |  |  | the test and mc entry points reject --no-std with an error. |  |
| tests/codegen_test.rs::no_std_header_pins_and_std_unchanged | src/backends/rust_fsm.rs::generate_production, src/codegen.rs::BackendOptions, src/spec_hash.rs::spec_hash16 |  |  | the no_std file header is byte-pinned and the std header is unchanged. |  |
| tests/codegen_test.rs::no_std_header_imports_vec_macro_when_outputs_exist | src/backends/rust_fsm.rs::generate_production, src/codegen.rs::BackendOptions |  |  | the no_std header imports the vec! macro only when the module has outputs. |  |
| tests/codegen_test.rs::c_production_no_std_is_documented_noop | src/backends/c_fsm.rs::generate_production, src/codegen.rs::BackendOptions |  |  | --no-std on c production is a documented no-op. |  |
| tests/codegen_test.rs::strip_generated_rust_header |  |  |  | strip the generated file header (comment line, inner attributes, and the use block) for body comparison. |  |
| tests/codegen_test.rs::tool_on_path |  |  |  | true when a bare tool name resolves on PATH (for gating real compile checks). |  |
| tests/codegen_test.rs::no_std_compile_sweep_and_body_parity_all_valid_examples | src/codegen.rs::BackendOptions, src/codegen.rs::get_backend, src/lib.rs::collect_rfs_files |  |  | the no_std output compiles for all valid examples and its body matches the std body. |  |
| tests/codegen_test.rs::cli_protocol_fsm_flag_parsing | src/codegen.rs::CodegenRequest, src/codegen.rs::parse_codegen_args |  |  | production accepts --with-protocol-fsm (rust only) in CLI parsing. |  |
| tests/codegen_test.rs::cli_protocol_fsm_e2e_exit_codes_and_output |  |  |  | --with-protocol-fsm end-to-end produces the right exit codes and emission. |  |
| tests/codegen_test.rs::protocol_fsm_emission_pins_and_default_unchanged | src/backends/c_fsm.rs::generate_production, src/backends/rust_fsm.rs::generate_production, src/backends/rust_test.rs::generate_unit_tests, src/codegen.rs::BackendOptions |  |  | the ProtocolFsm emission block is byte-pinned and absent by default. |  |
| tests/codegen_test.rs::mc_entry_points_reject_protocol_fsm | src/backends/c_mc.rs::generate_model_checker, src/backends/rust_mc.rs::generate_model_checker, src/codegen.rs::BackendOptions |  |  | the mc entry points reject --with-protocol-fsm with an error. |  |
| tests/codegen_test.rs::extract_generated_test_names |  |  |  | extract the generated test function names from a #[cfg(test)] module. |  |
| tests/codegen_test.rs::rust_test_emits_all_four_extended_classes_for_property_examples | src/codegen.rs::BackendOptions, src/codegen.rs::CodegenCommandKind, src/codegen.rs::get_backend |  |  | rust test codegen emits all four extended test classes for property examples. |  |
| tests/codegen_test.rs::rust_test_names_follow_the_pinned_convention | src/codegen.rs::BackendOptions, src/codegen.rs::CodegenCommandKind, src/codegen.rs::get_backend |  |  | the generated rust test names follow the pinned naming convention. |  |
| tests/codegen_test.rs::rust_test_steps_option_changes_random_trace_class | src/codegen.rs::BackendOptions, src/codegen.rs::CodegenCommandKind, src/codegen.rs::get_backend |  |  | --test-steps changes the generated random-trace test class. |  |
| tests/codegen_test.rs::rust_test_emits_fuzz_skeleton_as_separate_file | src/codegen.rs::BackendOptions, src/codegen.rs::CodegenCommandKind, src/codegen.rs::get_backend |  |  | rust test codegen emits the libfuzzer skeleton as a separate file. |  |
| tests/codegen_test.rs::cli_test_command_writes_fuzz_target_next_to_out |  |  |  | the rfs test CLI writes the fuzz target next to the --out artifact. |  |
| tests/codegen_test.rs::aggregate_crate_carries_new_classes_and_stays_dependency_free | src/backends/rust_test.rs::generate_aggregate_test_crate, src/codegen.rs::BackendOptions |  |  | the aggregate test crate carries the new test classes and stays dependency-free. |  |
| tests/codegen_test.rs::extract_c_test_names |  |  |  | extract the generated C test names from the harness tests[] table. |  |
| tests/codegen_test.rs::c_test_emits_all_four_extended_classes_for_property_examples | src/codegen.rs::BackendOptions, src/codegen.rs::CodegenCommandKind, src/codegen.rs::get_backend |  |  | c test codegen emits all four extended test classes for property examples. |  |
| tests/codegen_test.rs::c_test_names_follow_the_pinned_convention | src/codegen.rs::BackendOptions, src/codegen.rs::CodegenCommandKind, src/codegen.rs::get_backend |  |  | the generated c test names follow the pinned naming convention. |  |
| tests/codegen_test.rs::c_test_steps_option_changes_random_trace_class | src/codegen.rs::BackendOptions, src/codegen.rs::CodegenCommandKind, src/codegen.rs::get_backend |  |  | --test-steps changes the generated c random-trace test class. |  |
| tests/codegen_test.rs::c_test_emits_fuzz_skeleton_as_extra_file | src/codegen.rs::BackendOptions, src/codegen.rs::CodegenCommandKind, src/codegen.rs::get_backend |  |  | c test codegen emits the libfuzzer skeleton as an extra file. |  |
| tests/codegen_test.rs::c_fuzz_target_frees_per_input_and_bounded_run_stays_green | src/codegen.rs::BackendOptions, src/codegen.rs::CodegenCommandKind, src/codegen.rs::get_backend |  |  | the c fuzz target frees its per-input allocation and the bounded run stays green (the C-fuzz free regression). |  |
| tests/examples_test.rs::example_root |  |  |  | the examples/ directory root. |  |
| tests/examples_test.rs::valid_examples_parse_and_roundtrip | src/json_rw.rs::parse |  |  | all 50 valid examples parse, validate, and round-trip through conversion. |  |
| tests/examples_test.rs::invalid_examples_rejected | src/validator.rs::validate_ir |  |  | all 13 invalid examples are rejected by parse or validate. |  |
| tests/examples_test.rs::example_count_is_reported |  |  |  | the valid/invalid example counts are reported; dropping a new .rfs into examples/valid|invalid works without test-code change. |  |
| tests/backend_parity_test.rs::root |  |  |  | the repo root (CARGO_MANIFEST_DIR). |  |
| tests/backend_parity_test.rs::rustc |  |  |  | rustc override - the RUSTC env var or "rustc". |  |
| tests/backend_parity_test.rs::property_bearing_examples | src/lib.rs::collect_rfs_files, tests/backend_parity_test.rs::root |  |  | all valid examples carrying prop/assume declarations (dynamic set incl. tla_adapted; anchored on 41_property_monitors + 33_german_transfer_tla). |  |
| tests/backend_parity_test.rs::example_name |  |  |  | the file name of a spec path. |  |
| tests/backend_parity_test.rs::cached_binary | tests/backend_parity_test.rs::root |  |  | compile-and-cache under one lock so each key is built exactly once even when test threads race for it. |  |
| tests/backend_parity_test.rs::generate_full | src/parser.rs::parse_source, src/codegen.rs::get_backend, src/codegen.rs::BackendOptions, src/codegen.rs::AssumptionSelection, src/codegen.rs::CodegenCommandKind, src/codegen.rs::CodegenOutput |  |  | codegen one spec through the named backend + command (assumptions all, 4 steps, no monitors) returning the full output incl. extra files. |  |
| tests/backend_parity_test.rs::generate | tests/backend_parity_test.rs::generate_full, src/codegen.rs::CodegenCommandKind, src/codegen.rs::CodegenOutput |  |  | the generated code text of generate_full (the file-based twin used by the MC/test binary builders). |  |
| tests/backend_parity_test.rs::compile_rust | tests/backend_parity_test.rs::rustc |  |  | compile generated Rust with rustc --edition 2021 (--test when as_test) and assert success. |  |
| tests/backend_parity_test.rs::compile_c |  |  |  | compile generated C with gcc -Wall and assert zero warnings (the hard bar). |  |
| tests/backend_parity_test.rs::mc_binary | tests/backend_parity_test.rs::cached_binary, tests/backend_parity_test.rs::generate, src/codegen.rs::CodegenCommandKind, tests/backend_parity_test.rs::compile_c, tests/backend_parity_test.rs::compile_rust |  |  | the cached exact-mode MC binary for a spec + backend (generate, write, compile). |  |
| tests/backend_parity_test.rs::test_binary | tests/backend_parity_test.rs::cached_binary, tests/backend_parity_test.rs::generate, src/codegen.rs::CodegenCommandKind, tests/backend_parity_test.rs::compile_c, tests/backend_parity_test.rs::compile_rust |  |  | the cached generated-test-executable binary for a spec + backend. |  |
| tests/backend_parity_test.rs::run_mc | tests/backend_parity_test.rs::mc_binary, tests/backend_parity_test.rs::McOutcome |  |  | run one MC binary with extra args and capture (exit code, stdout). |  |
| tests/backend_parity_test.rs::parse_exact | tests/backend_parity_test.rs::McExact |  |  | parse one exact-mode run's stdout + exit code into status, summary-line fields, fail suffix and counterexample fields. |  |
| tests/backend_parity_test.rs::mc_exact_runs_parity_all_property_bearing_examples | tests/backend_parity_test.rs::property_bearing_examples, tests/backend_parity_test.rs::example_name, tests/backend_parity_test.rs::run_mc, tests/backend_parity_test.rs::parse_exact, tests/backend_parity_test.rs::McExact |  |  | exact-mode MC runs over ALL property-bearing examples are field-identical across backends (exit code + parsed McExact). |  |
| tests/backend_parity_test.rs::pinned | tests/backend_parity_test.rs::Pinned |  |  | the re-pinned exact-mode expectation (status/metrics/fail fields) for one example name; panics on an unknown name. |  |
| tests/backend_parity_test.rs::mc_exact_results_match_re_pinned_values | tests/backend_parity_test.rs::property_bearing_examples, tests/backend_parity_test.rs::example_name, tests/backend_parity_test.rs::run_mc, tests/backend_parity_test.rs::pinned, tests/backend_parity_test.rs::parse_exact, tests/backend_parity_test.rs::McExact, tests/backend_parity_test.rs::Pinned |  |  | every property-bearing example's exact-mode results match the re-pinned values (14 pass-complete / 9 fail split). |  |
| tests/backend_parity_test.rs::parse_rust_test_results |  |  |  | per-test results from a Rust libtest executable, normalized to test name + PASS/FAIL outcome. |  |
| tests/backend_parity_test.rs::parse_c_test_results |  |  |  | per-test results from the C plain-assert harness, normalized to test name + PASS/FAIL outcome. |  |
| tests/backend_parity_test.rs::extract_coverage_line |  |  |  | the deterministic random-trace coverage line wherever it appears in the output. |  |
| tests/backend_parity_test.rs::generated_test_executables_parity_all_property_bearing_examples | tests/backend_parity_test.rs::property_bearing_examples, tests/backend_parity_test.rs::example_name, tests/backend_parity_test.rs::generate_full, src/codegen.rs::CodegenCommandKind, src/codegen.rs::CodegenOutput, tests/backend_parity_test.rs::test_binary, tests/backend_parity_test.rs::parse_rust_test_results, tests/backend_parity_test.rs::parse_c_test_results, tests/backend_parity_test.rs::extract_coverage_line |  |  | both backends' generated test executables agree on every example: fuzz skeleton present in both, identical per-test result lists, byte-identical random-trace coverage line. |  |
| tests/backend_parity_test.rs::generate_inline | src/parser.rs::parse_source, src/codegen.rs::get_backend, src/codegen.rs::BackendOptions, src/codegen.rs::AssumptionSelection, src/codegen.rs::CodegenCommandKind |  |  | MC codegen for an inline spec string (the file-based twin of generate). |  |
| tests/backend_parity_test.rs::liv_mc_binary | tests/backend_parity_test.rs::cached_binary, tests/backend_parity_test.rs::generate_inline, tests/backend_parity_test.rs::compile_c, tests/backend_parity_test.rs::compile_rust |  |  | the cached MC binary for one inline liveness spec + backend. |  |
| tests/backend_parity_test.rs::run_bin |  |  |  | run one binary with args and capture (exit code, stdout). |  |
| tests/backend_parity_test.rs::mc_liveness_example_43_exact_parity | tests/backend_parity_test.rs::property_bearing_examples, tests/backend_parity_test.rs::run_mc, tests/backend_parity_test.rs::parse_exact, tests/backend_parity_test.rs::McExact |  |  | example 43 exact-mode status + full stdout byte-identical across backends (pass-complete, no counterexample). |  |
| tests/backend_parity_test.rs::mc_liveness_inline_variants_parity | tests/backend_parity_test.rs::liv_mc_binary, tests/backend_parity_test.rs::run_bin |  |  | all inline liveness variants (pass/fail/until/skeleton/input_is/nondet/truncated/safety-precedence) agree on exit code + byte-identical stdout across backends. |  |
| tests/backend_parity_test.rs::mc_liveness_counterexample_fields_pinned | tests/backend_parity_test.rs::liv_mc_binary, tests/backend_parity_test.rs::run_bin |  |  | pinned lasso counterexample fields on the eventually-fail variant (cycle length 2, violation step 0). |  |
| tests/backend_parity_test.rs::mc_liveness_truncated_mode_sound | tests/backend_parity_test.rs::liv_mc_binary, tests/backend_parity_test.rs::run_bin, tests/backend_parity_test.rs::parse_exact, tests/backend_parity_test.rs::McExact |  |  | bounded mode is sound: bad cycle inside the explored part => FAIL, cut off => UNKNOWN, never a false pass-complete. |  |
| tests/backend_parity_test.rs::json_options | src/codegen.rs::BackendOptions, src/codegen_mc.rs::CounterexampleFormat |  |  | MC codegen options with the JSON counterexample report format. |  |
| tests/backend_parity_test.rs::generate_spec_json | src/parser.rs::parse_source, src/codegen.rs::get_backend, tests/backend_parity_test.rs::json_options, src/codegen.rs::CodegenCommandKind |  |  | MC codegen for a spec file with the JSON report format. |  |
| tests/backend_parity_test.rs::mc_json_binary | tests/backend_parity_test.rs::cached_binary, tests/backend_parity_test.rs::generate_spec_json, tests/backend_parity_test.rs::compile_c, tests/backend_parity_test.rs::compile_rust |  |  | the cached JSON-report MC binary for a spec + backend. |  |
| tests/backend_parity_test.rs::generate_inline_json | src/parser.rs::parse_source, src/codegen.rs::get_backend, tests/backend_parity_test.rs::json_options, src/codegen.rs::CodegenCommandKind |  |  | MC codegen for an inline spec with the JSON report format. |  |
| tests/backend_parity_test.rs::mc_json_binary_for | tests/backend_parity_test.rs::cached_binary, tests/backend_parity_test.rs::generate_inline_json, tests/backend_parity_test.rs::compile_c, tests/backend_parity_test.rs::compile_rust |  |  | the cached JSON-report MC binary for one inline spec + backend. |  |
| tests/backend_parity_test.rs::parse_text_stats |  |  |  | the statistics block of a TEXT run (unexpected terminations, monitor size bits, memory estimate bytes). |  |
| tests/backend_parity_test.rs::text_step_count |  |  |  | number of step lines in a TEXT run's counterexample trace. |  |
| tests/backend_parity_test.rs::mc_json_report_parity_all_property_bearing_examples | tests/backend_parity_test.rs::property_bearing_examples, tests/backend_parity_test.rs::example_name, tests/backend_parity_test.rs::mc_json_binary, tests/backend_parity_test.rs::run_bin, tests/common/mc_json.rs::zero_peak_rss, src/json_rw.rs::parse, tests/backend_parity_test.rs::run_mc, tests/backend_parity_test.rs::parse_exact, tests/backend_parity_test.rs::McExact, tests/backend_parity_test.rs::parse_text_stats, tests/backend_parity_test.rs::text_step_count |  |  | for EVERY property-bearing example the JSON report is byte-identical across backends (modulo peak_rss_kb), strict-parseable in canonical form, and value-agreeing with the text run. |  |
| tests/backend_parity_test.rs::mc_json_five_status_reason_cases | tests/backend_parity_test.rs::root, tests/backend_parity_test.rs::mc_json_binary, tests/backend_parity_test.rs::run_bin, tests/common/mc_json.rs::zero_peak_rss, src/json_rw.rs::parse, tests/backend_parity_test.rs::mc_json_binary_for, tests/backend_parity_test.rs::mc_json_five_status_reason_cases::JsonCase |  |  | the five status/reason combinations produce byte-identical JSON across backends (pass-complete, property-violation, deadlock, unknown-state-limit + the inline liveness lasso). |  |
| tests/backend_parity_test.rs::mc_json_golden_document_pin | tests/backend_parity_test.rs::mc_json_binary_for, tests/backend_parity_test.rs::run_bin, tests/common/mc_json.rs::zero_peak_rss |  |  | golden full-document pin: the exact JSON bytes for an enum-fields+outputs spec are the cross-backend contract (key order, escaping, value rendering). |  |
| tests/backend_parity_test.rs::assumption_bearing_examples | src/lib.rs::collect_rfs_files, tests/backend_parity_test.rs::root |  |  | all valid examples carrying assume declarations (dynamic set, anchored on example 43). |  |
| tests/backend_parity_test.rs::assume_labels |  |  |  | the assumption labels of one spec in declaration order (parsed from the assume lines). |  |
| tests/backend_parity_test.rs::fv_args |  |  |  | the pinned forced-violation run args (--mode fuzz, 5 runs, bound 10, seed 777, --violate directive). |  |
| tests/backend_parity_test.rs::mc_forced_violation_parity_all_assumption_bearing_examples | tests/backend_parity_test.rs::assumption_bearing_examples, tests/backend_parity_test.rs::example_name, tests/backend_parity_test.rs::assume_labels, tests/backend_parity_test.rs::fv_args, tests/backend_parity_test.rs::mc_binary, tests/backend_parity_test.rs::run_bin |  |  | seeded forced-violation runs (--violate per label + combined all-labels) are byte-identical across backends on all assumption-bearing examples. |  |
| tests/wide_ops_parity_test.rs::repo_root |  |  |  | the repo root (CARGO_MANIFEST_DIR). |  |
| tests/wide_ops_parity_test.rs::tool_on_path |  |  |  | whether a tool name resolves on PATH (availability gate for the differential runs). |  |
| tests/wide_ops_parity_test.rs::rust_driver_source |  |  |  | the Rust driver source appended to the generated production code - steps the pinned boundary sequence and prints per-step state. |  |
| tests/wide_ops_parity_test.rs::c_driver_source |  |  |  | the C driver source appended to the generated production code - the C mirror of rust_driver_source (same lockstep contract). |  |
| tests/wide_ops_parity_test.rs::generate | tests/wide_ops_parity_test.rs::repo_root, src/parser.rs::parse_file, src/codegen.rs::get_backend, src/codegen.rs::CodegenCommandKind, src/codegen.rs::BackendOptions |  |  | codegen examples/valid/50_wide_ops.rfs through the named backend + command (production or mc). |  |
| tests/wide_ops_parity_test.rs::wide_ops_production_drivers_byte_identical_and_pinned | tests/wide_ops_parity_test.rs::tool_on_path, tests/wide_ops_parity_test.rs::generate, tests/wide_ops_parity_test.rs::rust_driver_source, tests/wide_ops_parity_test.rs::c_driver_source |  |  | per-step output of the wide-ops boundary sequence is byte-identical across backends AND matches the pinned expected values (shifts by k >= width give 0; wide products 63744 then 53248). |  |
| tests/wide_ops_parity_test.rs::wide_ops_exact_mc_status_parity | tests/wide_ops_parity_test.rs::tool_on_path, tests/wide_ops_parity_test.rs::generate |  |  | exact-mode MC on the wide-ops spec: both backends pass-complete with identical summary lines. |  |
| tests/prod_parity_test.rs::root |  |  |  | the repo root (CARGO_MANIFEST_DIR). |  |
| tests/prod_parity_test.rs::run_one | tests/prod_parity_test.rs::root, src/parser.rs::parse_source, tests/common/prod_driver.rs::build_drivers, tests/common/prod_driver.rs::input_script, tests/common/prod_driver.rs::run_driver, tests/prod_parity_test.rs::DriverRun |  |  | build both production drivers for one spec (monitors ON) and drive them with the identical seeded LCG script (seed 43210, 200 steps). |  |
| tests/prod_parity_test.rs::production_drivers_byte_identical_all_valid_examples | src/lib.rs::collect_rfs_files, tests/prod_parity_test.rs::root, tests/prod_parity_test.rs::run_one, tests/prod_parity_test.rs::DriverRun |  |  | EVERY valid example x both backends: byte-identical driver stdout + exit code 0, and the unexpected-event path is exercised somewhere in the suite. |  |
| tests/benchmark_mc_test.rs::root |  |  |  | the repo root (CARGO_MANIFEST_DIR). |  |
| tests/benchmark_mc_test.rs::rustc |  |  |  | rustc override - the RUSTC env var or "rustc". |  |
| tests/benchmark_mc_test.rs::pin_for | tests/benchmark_mc_test.rs::Pin |  |  | pinned exact-mode metrics per benchmark spec (deterministic; recorded in tasks 6/8); panics on an unknown name. |  |
| tests/benchmark_mc_test.rs::cached_binary | tests/benchmark_mc_test.rs::root |  |  | compile-and-cache under one lock so each key is built exactly once even when test threads race for it. |  |
| tests/benchmark_mc_test.rs::generate_from_source | src/parser.rs::parse_source, src/codegen.rs::get_backend, src/codegen.rs::BackendOptions, src/codegen.rs::AssumptionSelection, src/codegen.rs::CodegenCommandKind |  |  | MC codegen from an in-memory spec source through the named backend (assumptions all, 4 steps, no monitors). |  |
| tests/benchmark_mc_test.rs::compile_rust | tests/benchmark_mc_test.rs::rustc |  |  | compile generated Rust with rustc --edition 2021 -O (the suite is a performance artifact) and assert success. |  |
| tests/benchmark_mc_test.rs::compile_c |  |  |  | compile generated C with gcc -Wall and assert zero warnings (the hard bar). |  |
| tests/benchmark_mc_test.rs::bench_binary | tests/benchmark_mc_test.rs::cached_binary, tests/benchmark_mc_test.rs::generate_from_source, tests/benchmark_mc_test.rs::compile_c, tests/benchmark_mc_test.rs::compile_rust |  |  | the cached MC binary for one spec source + backend (generate, write, compile). |  |
| tests/benchmark_mc_test.rs::det | tests/benchmark_mc_test.rs::BenchRun |  |  | the deterministic projection of a run (wall clock zeroed) - what parity compares. |  |
| tests/benchmark_mc_test.rs::run_args_full | tests/benchmark_mc_test.rs::parse_run, tests/benchmark_mc_test.rs::BenchRun |  |  | one exact-mode run with explicit runtime args, keeping the raw stdout for byte-identity comparisons (task 10B threads gate). |  |
| tests/benchmark_mc_test.rs::run_parsed | tests/benchmark_mc_test.rs::run_args_full, tests/benchmark_mc_test.rs::BenchRun |  |  | one exact-mode run with no extra args, parsed. |  |
| tests/benchmark_mc_test.rs::parse_run | tests/benchmark_mc_test.rs::BenchRun |  |  | parse one exact-mode run's stdout into its deterministic fields (status, summary, statistics, fail suffix, counterexample). |  |
| tests/benchmark_mc_test.rs::run_json_mode | src/json_rw.rs::parse, src/json_rw.rs::Json |  |  | one --counterexample-format json run: stdout is the single compact JSON document (D-P5-10 shape - key order is the cross-backend byte-parity contract). |  |
| tests/benchmark_mc_test.rs::json_peak_rss_kb | src/json_rw.rs::Json |  |  | statistics.peak_rss_kb (always present; 0 on read failure - informational). |  |
| tests/benchmark_mc_test.rs::json_ce_steps | src/json_rw.rs::Json |  |  | counterexample.ce_steps (present iff the run failed). |  |
| tests/benchmark_mc_test.rs::bench_mc_pass_complete_metrics_parity | tests/common/fail_variants.rs::spec_source, tests/benchmark_mc_test.rs::bench_binary, tests/benchmark_mc_test.rs::run_parsed, tests/benchmark_mc_test.rs::pin_for, tests/benchmark_mc_test.rs::det, tests/benchmark_mc_test.rs::Pin, tests/benchmark_mc_test.rs::BenchRun |  |  | all six benchmark specs pass-complete in BOTH backends with pinned deterministic metrics (incl. monitor size) field-identical across backends. |  |
| tests/benchmark_mc_test.rs::bench_mc_fail_variants_both_backends | tests/common/fail_variants.rs::spec_source, tests/common/fail_variants.rs::http2_rst_fail_src, tests/common/fail_variants.rs::can_passive_fail_src, tests/benchmark_mc_test.rs::bench_binary, tests/benchmark_mc_test.rs::run_args_full, tests/benchmark_mc_test.rs::run_json_mode, tests/benchmark_mc_test.rs::json_ce_steps, tests/benchmark_mc_test.rs::det, tests/benchmark_mc_test.rs::FailPin, tests/benchmark_mc_test.rs::BenchRun |  |  | the five pinned FAIL variants (safety, liveness, assumption relaxation) fail identically in both backends with counterexample parity + ce_steps pins from the JSON report. |  |
| tests/benchmark_mc_test.rs::time_parse_codegen | tests/common/fail_variants.rs::spec_source, src/parser.rs::parse_source, src/codegen.rs::get_backend, src/codegen.rs::BackendOptions, src/codegen.rs::AssumptionSelection, src/codegen.rs::CodegenCommandKind, tests/benchmark_mc_test.rs::CodegenTimes |  |  | in-process parse + codegen wall clocks in ms (informational only - never asserted on). |  |
| tests/benchmark_mc_test.rs::codegen_times_json | tests/benchmark_mc_test.rs::CodegenTimes, src/json_rw.rs::Json |  |  | the two timing fields as a compact JSON object. |  |
| tests/benchmark_mc_test.rs::run_json | tests/benchmark_mc_test.rs::BenchRun, src/json_rw.rs::Json |  |  | one benchmark spec's per-backend report entry (status, deterministic metrics, peak_rss_kb, wall_ms) as JSON. |  |
| tests/benchmark_mc_test.rs::bench_mc_json_report_and_table | tests/common/fail_variants.rs::spec_source, tests/benchmark_mc_test.rs::bench_binary, tests/benchmark_mc_test.rs::run_parsed, tests/benchmark_mc_test.rs::run_json_mode, tests/benchmark_mc_test.rs::json_peak_rss_kb, tests/benchmark_mc_test.rs::time_parse_codegen, tests/benchmark_mc_test.rs::codegen_times_json, tests/benchmark_mc_test.rs::run_json, tests/benchmark_mc_test.rs::root, src/json_rw.rs::parse, src/json_rw.rs::Json, tests/benchmark_mc_test.rs::SpecRuns, tests/benchmark_mc_test.rs::BenchRun |  |  | writes target/mc_benchmarks.json (per spec x backend + codegen timing section), asserts on parsed fields only (never byte-for-byte), and prints per-backend summary + timing tables. |  |
| tests/benchmark_mc_test.rs::threads_gate_workloads | tests/common/fail_variants.rs::spec_source, tests/common/fail_variants.rs::http2_rst_fail_src, tests/common/fail_variants.rs::can_passive_fail_src |  |  | all eight workloads of the threads gate: the six benchmark specs plus all five inline FAIL variants (the 48 assumption-relaxation run carries its --assumes none args). |  |
| tests/benchmark_mc_test.rs::bench_mc_threads_parity | tests/benchmark_mc_test.rs::threads_gate_workloads, tests/benchmark_mc_test.rs::bench_binary, tests/benchmark_mc_test.rs::run_args_full |  |  | --threads 1/2/4 runs are byte-identical to each other and across backends on all eight workloads (determinism by construction - a broken pin here means a bug in the parallel path). |  |
| tests/benchmark_mc_test.rs::bench_mc_fast_smoke | tests/benchmark_mc_test.rs::bench_binary, tests/benchmark_mc_test.rs::run_args_full |  |  | --fast smoke: soundness only (exit 1, status=fail, genuinely violated label) - documented non-parity mode. |  |
| tests/liveness_mc_test.rs::root |  |  |  | the repo root (CARGO_MANIFEST_DIR). |  |
| tests/liveness_mc_test.rs::scratch | tests/liveness_mc_test.rs::root |  |  | scratch dir under target/ (self-cleaning with cargo clean), removed first so re-runs start from a clean slate; one dir per test. |  |
| tests/liveness_mc_test.rs::rustc |  |  |  | rustc override - the RUSTC env var or "rustc". |  |
| tests/liveness_mc_test.rs::codegen_mc | src/parser.rs::parse_source, src/codegen.rs::get_backend, src/codegen.rs::BackendOptions, src/codegen.rs::AssumptionSelection, src/codegen.rs::CodegenCommandKind |  |  | MC codegen for an inline spec; returns (code, warnings). |  |
| tests/liveness_mc_test.rs::build | tests/liveness_mc_test.rs::codegen_mc, tests/liveness_mc_test.rs::rustc |  |  | generate + write + compile one inline spec's MC binary with rustc --edition 2021. |  |
| tests/liveness_mc_test.rs::run |  |  |  | run the MC binary with args and capture (exit code, stdout). |  |
| tests/liveness_mc_test.rs::parse_liv | tests/liveness_mc_test.rs::LivOut |  |  | parse one exact-mode run's stdout into summary fields, liveness counterexample fields and the per-position (state, vars) trace. |  |
| tests/liveness_mc_test.rs::assert_lasso | tests/liveness_mc_test.rs::LivOut |  |  | the lasso structure check: the trace is prefix + one cycle iteration and its final configuration equals the configuration at the cycle start. |  |
| tests/liveness_mc_test.rs::liveness_pass_complete | tests/liveness_mc_test.rs::scratch, tests/liveness_mc_test.rs::build, tests/liveness_mc_test.rs::run, tests/liveness_mc_test.rs::parse_liv |  |  | the correct eventually variant reaches pass-complete with no reason. |  |
| tests/liveness_mc_test.rs::liveness_eventually_violation | tests/liveness_mc_test.rs::scratch, tests/liveness_mc_test.rs::build, tests/liveness_mc_test.rs::run, tests/liveness_mc_test.rs::parse_liv, tests/liveness_mc_test.rs::assert_lasso |  |  | eventually violation: pinned formula/step/cycle/note + a structurally valid lasso counterexample. |  |
| tests/liveness_mc_test.rs::liveness_until_violation | tests/liveness_mc_test.rs::scratch, tests/liveness_mc_test.rs::build, tests/liveness_mc_test.rs::run, tests/liveness_mc_test.rs::parse_liv, tests/liveness_mc_test.rs::assert_lasso |  |  | WEAK until violation: the cycle starts at the first not-p position where the right side does not hold. |  |
| tests/liveness_mc_test.rs::liveness_skeleton_safety_leaf_falsified | tests/liveness_mc_test.rs::scratch, tests/liveness_mc_test.rs::build, tests/liveness_mc_test.rs::run, tests/liveness_mc_test.rs::parse_liv, tests/liveness_mc_test.rs::assert_lasso |  |  | skeleton property: the safety leaf is falsified at step 1 and the bad cycle starts at step 2. |  |
| tests/liveness_mc_test.rs::liveness_input_is_edge_labels | tests/liveness_mc_test.rs::scratch, tests/liveness_mc_test.rs::build, tests/liveness_mc_test.rs::run, tests/liveness_mc_test.rs::parse_liv, tests/liveness_mc_test.rs::assert_lasso |  |  | input_is edge labels: the witness cycle must use the Pong self-loop (edge-based labeling, not node-based). |  |
| tests/liveness_mc_test.rs::nondet_strong_fairness_filter | tests/liveness_mc_test.rs::scratch, tests/liveness_mc_test.rs::build, tests/liveness_mc_test.rs::run, tests/liveness_mc_test.rs::parse_liv |  |  | --nondet strong-fairness filter: the Stay self-loop is unfair under nondet (Esc enabled and never taken), so the property holds. |  |
| tests/liveness_mc_test.rs::truncated_analysis_sound_fail_and_unknown | tests/liveness_mc_test.rs::scratch, tests/liveness_mc_test.rs::build, tests/liveness_mc_test.rs::run, tests/liveness_mc_test.rs::parse_liv, tests/liveness_mc_test.rs::assert_lasso |  |  | truncated-mode soundness: bad cycle inside the explored part => FAIL with lasso; cut off => unknown-state-limit, never a false pass-complete. |  |
| tests/liveness_mc_test.rs::deeper_nesting_codegen_diagnostic | tests/liveness_mc_test.rs::codegen_mc, tests/liveness_mc_test.rs::scratch, tests/liveness_mc_test.rs::rustc, tests/liveness_mc_test.rs::run, tests/liveness_mc_test.rs::parse_liv |  |  | deeper-nesting diagnostic: the property is skipped with a warning (not a parse error) and the binary compiles and runs to pass-complete. |  |
| tests/liveness_mc_test.rs::safety_failure_takes_precedence_over_liveness | tests/liveness_mc_test.rs::scratch, tests/liveness_mc_test.rs::build, tests/liveness_mc_test.rs::run, tests/liveness_mc_test.rs::parse_liv |  |  | safety failure takes precedence over liveness: the monitor-checkable safety violation is reported and the liveness path never runs. |  |
| tests/smt_bounded_test.rs::root |  |  |  | the repo root (CARGO_MANIFEST_DIR). |  |
| tests/smt_bounded_test.rs::generate | src/parser.rs::parse_source, src/codegen.rs::get_backend, src/codegen.rs::BackendOptions, src/codegen.rs::McMode, src/codegen.rs::CodegenCommandKind |  |  | SMT-bounded MC codegen for an inline spec through the named backend (bound 3 at codegen time). |  |
| tests/smt_bounded_test.rs::compile_c |  |  |  | compile generated C with gcc -Wall -O2 against the project-local z3 (include/bin/-lz3) and assert zero warnings. |  |
| tests/smt_bounded_test.rs::rust_e2e_project | tests/smt_bounded_test.rs::root |  |  | scratch cargo project for the generated Rust binaries (offline build from the local registry cache; empty [workspace] table so cargo does not walk up into the repository workspace). |  |
| tests/smt_bounded_test.rs::build_rust |  |  |  | write the generated main.rs and build the scratch project with cargo build --release --offline (Z3_LIBRARY_PATH_OVERRIDE + Z3_SYS_Z3_VERSION pinned to the project-local z3 5.3.1). |  |
| tests/smt_bounded_test.rs::run_bin |  |  |  | run one generated binary with LD_LIBRARY_PATH pointed at the project-local libz3, capturing (exit code, stdout). |  |
| tests/smt_bounded_test.rs::smt_bounded_backend_parity | tests/smt_bounded_test.rs::rust_e2e_project, tests/smt_bounded_test.rs::root, tests/smt_bounded_test.rs::generate, tests/smt_bounded_test.rs::compile_c, tests/smt_bounded_test.rs::build_rust, tests/smt_bounded_test.rs::run_bin |  |  | both backends x three specs (FAIL/liveness/PASS): byte-identical stdout + exit codes, pinned status/label/step lines, incomplete labeling on the PASS spec. |  |
| tests/smt_bounded_test.rs::smt_bounded_runtime_bound_flag | tests/smt_bounded_test.rs::rust_e2e_project, tests/smt_bounded_test.rs::root, tests/smt_bounded_test.rs::generate, tests/smt_bounded_test.rs::compile_c, tests/smt_bounded_test.rs::build_rust, tests/smt_bounded_test.rs::run_bin |  |  | the runtime --bound flag overrides the codegen default in both backends (same behavior, same output shape). |  |
| tests/ref_mc_sweep_test.rs::prep | src/parser.rs::parse_file, src/validator.rs::validate_ir, src/ref_mc.rs::RefConfig, src/ref_mc.rs::run, src/ref_mc.rs::render_text, src/ref_mc.rs::to_json_report, src/codegen.rs::get_backend, src/codegen.rs::BackendOptions, tests/ref_mc_sweep_test.rs::Prep |  |  | all in-process phases for one sample (parse, validate, ref_mc run, text+JSON rendering, Rust MC codegen); runs in a watchdog thread so a stuck ref_mc cannot stall the suite. |  |
| tests/ref_mc_sweep_test.rs::wait_watchdog | tests/ref_mc_sweep_test.rs::SampleErr, tests/ref_mc_sweep_test.rs::Prep |  |  | wait for the watchdog result, honoring stop + deadline in 50ms ticks. |  |
| tests/ref_mc_sweep_test.rs::run_bounded | tests/ref_mc_sweep_test.rs::SampleErr |  |  | run a subprocess with stdout/stderr captured, bounded by a deadline; the child is killed on timeout or when stop is observed. |  |
| tests/ref_mc_sweep_test.rs::run_sample | tests/ref_mc_sweep_test.rs::prep, tests/ref_mc_sweep_test.rs::wait_watchdog, tests/ref_mc_sweep_test.rs::run_bounded, tests/ref_mc_sweep_test.rs::diff_snippet, tests/common/mc_json.rs::zero_peak_rss, tests/ref_mc_sweep_test.rs::Job, tests/ref_mc_sweep_test.rs::SampleErr |  |  | the full per-sample pipeline under one deadline: watchdog prep, write source, rustc -O compile, run the generated binary in text+JSON and byte-compare against ref_mc modulo peak_rss_kb. |  |
| tests/ref_mc_sweep_test.rs::diff_snippet |  |  |  | a byte-offset diff snippet around the first mismatch between the generated output and the reference output. |  |
| tests/ref_mc_sweep_test.rs::ref_mc_vs_generated_rust_mc_sweep | src/lib.rs::collect_rfs_files, tests/ref_mc_sweep_test.rs::run_sample, tests/ref_mc_sweep_test.rs::Job, tests/ref_mc_sweep_test.rs::SampleErr |  |  | byte-parity sweep of ref_mc vs the generated Rust MC over all tractable valid examples: fail-fast N-way parallel workers, per-sample timeout, failed-first reordering via target/rfs_refmc_sweep/failed_order.txt. |  |
| tests/ref_mc_diag_test.rs::prep | src/parser.rs::parse_file, src/validator.rs::validate_ir, src/ref_mc.rs::RefConfig, src/ref_mc.rs::run, src/ref_mc.rs::render_text, src/ref_mc.rs::to_json_report, src/codegen.rs::get_backend, src/codegen.rs::BackendOptions, tests/ref_mc_diag_test.rs::Prep |  |  | all in-process phases for one sample (parse, validate, ref_mc run, text+JSON rendering, Rust MC codegen). |  |
| tests/ref_mc_diag_test.rs::run_bounded |  |  |  | bounded subprocess run (no stop flag; hard deadline only). |  |
| tests/ref_mc_diag_test.rs::diff_snippet |  |  |  | a byte-offset diff snippet around the first mismatch between the generated output and the reference output. |  |
| tests/ref_mc_diag_test.rs::check_sample | tests/ref_mc_diag_test.rs::prep, tests/ref_mc_diag_test.rs::run_bounded, tests/ref_mc_diag_test.rs::diff_snippet |  |  | one sample's full pipeline (watchdog prep with a 15s in-process deadline, rustc -O compile, text+JSON runs) returning an OK/HANG/MISMATCH/ERROR status string. |  |
| tests/ref_mc_diag_test.rs::enumerate_all_mismatches | src/lib.rs::collect_rfs_files, tests/ref_mc_diag_test.rs::check_sample |  |  | the non-fail-fast diagnostic companion: 8 workers enumerate every ref_mc-vs-generated-mc mismatch over all tractable valid examples in one shot. |  |
| tests/ref_mc_diff_test.rs::root |  |  |  | the repo root (CARGO_MANIFEST_DIR). |  |
| tests/ref_mc_diff_test.rs::rustc |  |  |  | rustc override - the RUSTC env var or "rustc". |  |
| tests/ref_mc_diff_test.rs::eligible_examples | src/lib.rs::collect_rfs_files, tests/ref_mc_diff_test.rs::root |  |  | all valid examples except the pinned exclusions (dynamic enumeration, so future additions are covered automatically; asserts the set has not shrunk below 40). |  |
| tests/ref_mc_diff_test.rs::example_name |  |  |  | the file name of a spec path. |  |
| tests/ref_mc_diff_test.rs::cached_binary | tests/ref_mc_diff_test.rs::root |  |  | compile-and-cache under one lock so each key is built exactly once even when test threads race for it. |  |
| tests/ref_mc_diff_test.rs::generate_mc_json | src/codegen.rs::get_backend, src/codegen.rs::BackendOptions, src/codegen_mc.rs::CounterexampleFormat, src/codegen.rs::CodegenCommandKind, src/ir.rs::Ir |  |  | MC codegen for a parsed IR with the JSON counterexample report format. |  |
| tests/ref_mc_diff_test.rs::compile_rust | tests/ref_mc_diff_test.rs::rustc |  |  | compile generated Rust with rustc --edition 2021 and assert success. |  |
| tests/ref_mc_diff_test.rs::compile_c |  |  |  | compile generated C with gcc -Wall and assert success. |  |
| tests/ref_mc_diff_test.rs::tag_of |  |  |  | cache/file tag: the example name without its .rfs extension (a dot in the file name would derive an invalid Rust crate name). |  |
| tests/ref_mc_diff_test.rs::mc_json_binary | tests/ref_mc_diff_test.rs::tag_of, tests/ref_mc_diff_test.rs::cached_binary, tests/ref_mc_diff_test.rs::generate_mc_json, tests/ref_mc_diff_test.rs::compile_c, tests/ref_mc_diff_test.rs::compile_rust, src/ir.rs::Ir |  |  | the cached JSON-report MC binary for one IR + backend. |  |
| tests/ref_mc_diff_test.rs::run_mc_json | src/json_rw.rs::parse, src/json_rw.rs::Json |  |  | run one generated MC binary (exact mode is the default) and return (exit code, parsed JSON report document). |  |
| tests/ref_mc_diff_test.rs::ref_cfg | src/ref_mc.rs::RefConfig |  |  | the ref_mc configuration for eligible runs (500k state budget, 60s timeout, all assumptions). |  |
| tests/ref_mc_diff_test.rs::ref_json_doc | src/ref_mc.rs::to_json_report, src/ir.rs::Module, src/ref_mc.rs::RefResult, src/json_rw.rs::Json |  |  | the ref JSON document rendered through the SAME unified report model as the generated binaries (D-P6-10) - directly comparable. |  |
| tests/ref_mc_diff_test.rs::doc_status | src/json_rw.rs::Json |  |  | the status field of a JSON report document. |  |
| tests/ref_mc_diff_test.rs::doc_fail | src/json_rw.rs::Json, src/codegen_mc.rs::FailReason |  |  | the fail suffix of a JSON report (reason wire name -> FailReason, label, step); None on pass/unknown. |  |
| tests/ref_mc_diff_test.rs::json_io | src/json_rw.rs::Json, src/codegen_mc.rs::JsonIo |  |  | one JSON input/output record -> the shared report-model JsonIo. |  |
| tests/ref_mc_diff_test.rs::json_step | tests/ref_mc_diff_test.rs::json_io, src/json_rw.rs::Json, src/codegen_mc.rs::JsonStep |  |  | one JSON step record -> the shared report-model JsonStep (input, outputs, from/to state, vars). |  |
| tests/ref_mc_diff_test.rs::json_ce | tests/ref_mc_diff_test.rs::json_step, src/json_rw.rs::Json, src/codegen_mc.rs::JsonCounterexample |  |  | the counterexample object of a JSON report -> the shared report-model enum (so ref_mc::verify_counterexample can consume it); asserts ce_steps equals the CE trace length. |  |
| tests/ref_mc_diff_test.rs::compare_one | tests/ref_mc_diff_test.rs::ref_cfg, tests/ref_mc_diff_test.rs::mc_json_binary, tests/ref_mc_diff_test.rs::run_mc_json, tests/ref_mc_diff_test.rs::doc_status, tests/ref_mc_diff_test.rs::doc_fail, tests/ref_mc_diff_test.rs::json_ce, src/ref_mc.rs::run, src/ref_mc.rs::minimal_violation_labels, src/ref_mc.rs::verify_counterexample, src/codegen_mc.rs::ExactStatus, src/codegen_mc.rs::FailReason, src/ir.rs::Ir |  |  | the per-spec comparison shared by examples and random specs: status agreement, exact states/max_depth, label ground truth via the minimal-violation set (label+step equality for unique-minimal specs), CE replay through ref_mc's independent evaluators, plus the documented sample-0 assumption asymmetry. |  |
| tests/ref_mc_diff_test.rs::ref_mc_vs_generated_all_eligible_examples | tests/ref_mc_diff_test.rs::eligible_examples, tests/ref_mc_diff_test.rs::example_name, src/parser.rs::parse_source, tests/ref_mc_diff_test.rs::compare_one |  |  | ref-MC vs both generated engines over every eligible valid example (first module of multi-module files). |  |
| tests/ref_mc_diff_test.rs::ref_mc_vs_generated_random_specs | src/specgen.rs::SpecGen, src/specgen.rs::Bounds, src/printer.rs::print_ir, src/parser.rs::parse_source, tests/ref_mc_diff_test.rs::compare_one |  |  | ref-MC vs both generated engines over 20 small random specs (pinned seeds), with a printer round-trip drift guard. |  |
| tests/ref_mc_diff_test.rs::tiny_state_limit_never_pass_complete | tests/ref_mc_diff_test.rs::root, src/parser.rs::parse_source, src/ref_mc.rs::run, src/ref_mc.rs::RefConfig, src/codegen_mc.rs::ExactStatus, tests/ref_mc_diff_test.rs::mc_json_binary, tests/ref_mc_diff_test.rs::run_mc_json, tests/ref_mc_diff_test.rs::doc_status |  |  | negative control: a tiny --max-states on 44_token_ring must yield UnknownStateLimit in ref_mc AND both generated backends - never PassComplete. |  |
| tests/ref_mc_diff_test.rs::rfs_bin |  |  |  | the path to the built rfs CLI binary (CARGO_BIN_EXE_rfs). |  |
| tests/ref_mc_diff_test.rs::mc_ref_cli_matches_in_process_and_generated_engine | tests/ref_mc_diff_test.rs::root, src/parser.rs::parse_source, tests/ref_mc_diff_test.rs::ref_cfg, src/ref_mc.rs::run, tests/ref_mc_diff_test.rs::ref_json_doc, tests/ref_mc_diff_test.rs::rfs_bin, tests/ref_mc_diff_test.rs::mc_json_binary, tests/ref_mc_diff_test.rs::run_mc_json, tests/common/mc_json.rs::zero_peak_rss |  |  | the rfs mc-ref CLI itself: exit codes mirror the generated binaries and the JSON stdout is byte-identical (modulo peak_rss_kb) to the in-process render and both engines. |  |
| tests/toolchain_fuzz_test.rs::root |  |  |  | the repo root (CARGO_MANIFEST_DIR). |  |
| tests/toolchain_fuzz_test.rs::rustc |  |  |  | rustc override - the RUSTC env var or "rustc". |  |
| tests/toolchain_fuzz_test.rs::scratch_dir | tests/toolchain_fuzz_test.rs::root |  |  | per-process scratch dir under target/ (unique per process, cleaned by cargo clean). |  |
| tests/toolchain_fuzz_test.rs::compile_rust | tests/toolchain_fuzz_test.rs::rustc |  |  | compile a generated Rust artifact (rlib for production, libtest harness for test) and assert success - compilation success IS the no-invalid-target-language-code check. |  |
| tests/toolchain_fuzz_test.rs::compile_c |  |  |  | compile generated C with gcc -Wall (-c compile-only for production) and assert success. |  |
| tests/toolchain_fuzz_test.rs::run_mc | tests/toolchain_fuzz_test.rs::Run |  |  | run one MC binary with args, capturing exit code + stdout + stderr (a signal crash is a panic). |  |
| tests/toolchain_fuzz_test.rs::doc_status | src/json_rw.rs::Json |  |  | the status field of a JSON report document. |  |
| tests/toolchain_fuzz_test.rs::run_one | src/printer.rs::print_ir, src/parser.rs::parse_source, src/validator.rs::validate_ir, tests/toolchain_fuzz_test.rs::scratch_dir, src/codegen.rs::get_backend, src/codegen.rs::CodegenCommandKind, src/codegen.rs::BackendOptions, src/codegen_mc.rs::CounterexampleFormat, tests/toolchain_fuzz_test.rs::compile_rust, tests/toolchain_fuzz_test.rs::compile_c, src/ref_mc.rs::run, src/ref_mc.rs::RefConfig, tests/toolchain_fuzz_test.rs::check_status_agreement, tests/toolchain_fuzz_test.rs::check_sample0_assumption_abort, src/ir.rs::Ir |  |  | the per-spec flow: render -> re-parse -> validate (exercising printer + parser), generate ALL six artifacts (2 backends x production/test/mc with JSON report), compile them all, then ref_mc ground truth vs the generated engines. |  |
| tests/toolchain_fuzz_test.rs::check_status_agreement | tests/toolchain_fuzz_test.rs::scratch_dir, tests/toolchain_fuzz_test.rs::run_mc, src/json_rw.rs::parse, tests/toolchain_fuzz_test.rs::doc_status, src/codegen_mc.rs::ExactStatus, src/ref_mc.rs::RefResult |  |  | exact + fuzz runs of BOTH generated engines vs the ref_mc verdict: exit-code taxonomy {0,1,2}, NO FALSE PASS, full status agreement, stderr stability across two runs. |  |
| tests/toolchain_fuzz_test.rs::check_sample0_assumption_abort | tests/toolchain_fuzz_test.rs::scratch_dir |  |  | the documented sample-0 assumption-violation asymmetry: ref_mc reports a clean error; the generated binaries panic/abort naming the SAME label - in both exact and fuzz mode. |  |
| tests/toolchain_fuzz_test.rs::c_mc_trigger_step_zero_report_no_crash | src/parser.rs::parse_source, tests/toolchain_fuzz_test.rs::scratch_dir, src/codegen.rs::get_backend, src/codegen.rs::CodegenCommandKind, src/codegen.rs::BackendOptions, tests/toolchain_fuzz_test.rs::rustc |  |  | regression pin (plan correction 5b): a fuzz-mode property violation with trigger step 0 must not crash the C report path - both backends exit 1 with the trigger-step-0 + vars lines. |  |
| tests/toolchain_fuzz_test.rs::toolchain_fuzz_slow_suite | src/specgen.rs::SpecGen, src/specgen.rs::Bounds, tests/toolchain_fuzz_test.rs::run_one |  |  | the seeded suite: 3 pinned seeds x 10 small specgen specs through the full per-spec flow (the 24099 seed is the plan-correction-5a regression seed). |  |
## All types

| Type | Depends | Component | Functionality | Short description | Documented in |
|---|---|---|---|---|---|
| tests/common/prod_driver.rs::FlatVariant | src/ir.rs::Type |  |  | one flattened input/output variant of a module (declaration order): collision-free name, its constructor group, the bare variant name, and (field name, field type) pairs. |  |
| tests/common/prod_driver.rs::ProductionArtifacts |  |  |  | generated production artifacts (property monitors ON) for one module - the Rust source kept in memory plus both artifact paths (the C driver #includes the .c file). |  |
| tests/backend_parity_test.rs::McOutcome |  |  |  | one MC binary run's captured result (exit code + stdout). |  |
| tests/backend_parity_test.rs::McExact |  |  |  | one exact-mode run fully parsed: status, summary-line fields, fail suffix and human-line counterexample fields. |  |
| tests/backend_parity_test.rs::Pinned |  |  |  | one pinned exact-mode expectation per property-bearing example (re-pinned from actual engine output). |  |
| tests/backend_parity_test.rs::mc_json_five_status_reason_cases::JsonCase |  |  |  | one status/reason combination to verify: tag, spec path, extra args, expected status + reason. |  |
| tests/prod_parity_test.rs::DriverRun |  |  |  | one example's driver results: spec name plus (exit code, stdout) per backend. |  |
| tests/benchmark_mc_test.rs::Pin |  |  |  | pinned exact-mode metrics per benchmark spec: states/transitions/max_depth + the encoded monitor bit width. |  |
| tests/benchmark_mc_test.rs::BenchRun |  |  |  | one exact-mode run fully parsed: status, summary fields, statistics, fail suffix, counterexample fields + the informational wall clock. |  |
| tests/benchmark_mc_test.rs::FailPin |  |  |  | pinned FAIL expectation: reason/label/step/metrics/violation_step/cycle_length + the JSON report's ce_steps. |  |
| tests/benchmark_mc_test.rs::CodegenTimes |  |  |  | in-process parse + codegen wall clocks in ms (informational only). |  |
| tests/benchmark_mc_test.rs::SpecRuns | tests/benchmark_mc_test.rs::BenchRun, src/json_rw.rs::Json |  |  | one benchmark spec's results per backend: the text-mode run plus the JSON-report document (a type alias keeping the entries tuple below clippy's complexity bar). |  |
| tests/liveness_mc_test.rs::LivOut |  |  |  | one exact-mode run fully parsed: summary fields, liveness counterexample fields (formula/cycle_length/note) and the per-position (state, vars) trace + step inputs. |  |
| tests/ref_mc_sweep_test.rs::Job |  |  |  | one sweep sample: the example name + path. |  |
| tests/ref_mc_sweep_test.rs::Prep |  |  |  | result of the in-process phases for one sample: ref text+JSON reports, ref exit code, generated MC source. |  |
| tests/ref_mc_sweep_test.rs::SampleErr |  |  |  | per-sample failure classification: genuine Fail (recorded, triggers fail-fast), TimedOut (deadline exceeded), Stopped (another sample failed first - abandoned without being recorded). |  |
| tests/ref_mc_diag_test.rs::Prep |  |  |  | result of the in-process phases for one sample: ref text+JSON reports, ref exit code, generated MC source. |  |
| tests/toolchain_fuzz_test.rs::Run |  |  |  | one MC binary run's captured result: exit code + stdout + stderr. |  |
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
