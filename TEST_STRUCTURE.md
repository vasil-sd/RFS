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
