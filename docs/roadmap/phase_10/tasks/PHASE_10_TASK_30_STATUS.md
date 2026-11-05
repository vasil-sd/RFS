# Phase 10, Task 30 - Implementation status

> Written once when all exit criteria of `PHASE_10_TASK_30.md` are reached. Not
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
| `TEST_STRUCTURE.md` | changed | 146 function rows + 5 type rows appended - the test inventory is now COMPLETE: inline backends/MC/CLI cluster (backends/rust.rs 3, backends/c.rs 4, specgen.rs 11, codegen_mc.rs 13, json_schema.rs 6, info_format.rs 4, smt_emit.rs 10, ref_mc.rs 24, mc_plan.rs 16, mc_ref_cmd.rs 7, gen_spec_cmd.rs 5 = 103), the unassigned crates/rfs-build/src/lib.rs inline module (12 - plan correction 30c), crate tests (rfs-build integration.rs 6, rfs-adapters adapters.rs 12 fns / 4 types, nostd_minimal.rs 5), and all five fuzz targets (common.rs 3 fns + RandSpec type + its Arbitrary method, codegen.rs check_structure, one `fuzz_target!` macro-entry row each for parser/validator/ir_roundtrip/codegen). Fuzz-target and crate-test rows carry their //! Docs: fragments in Documented-in per the binding preamble; inline rows blank. Also escaped four pre-existing raw-pipe cells (tasks 20-27) that split their rows into extra columns. |
| `docs/roadmap/phase_10/PHASE_10_PLAN.md` | changed | Plan corrections 30a (smt_emit 10 not 6 - correction 2a's figure already superseded by 6a), 30b (crate tests 23/4 not 26/14 - nostd_minimal template phantoms + associated-type bindings), 30c (rfs-build lib.rs inline module unassigned by both tasks + four fuzz macro-entry rows; final inventory 940 fn rows, delta vs ~901 fully explained). |
| `docs/roadmap/phase_10/tasks/PHASE_10_TASK_30.md` | changed | Task corrections section: the three count/scope corrections above + the Documented-in conflict resolution (binding preamble governs over the Context's "blank for crate tests"). |

## Other file changes

| Name | Operation | Description |
|---|---|---|

## Changed functionality

None - docs-only inventory phase.

## Removed functionality

None.

## New functionality

- The test inventory is COMPLETE (tasks 20-30): every function and type in
  tests/**, the inline #[cfg(test)] modules of all 25 host files,
  crates/*/tests/**, and fuzz/fuzz_targets/** is enumerated in
  TEST_STRUCTURE.md - 940 function rows + 34 type rows.
- This chunk covers: the Rust/C backend identifier-mangling/type-mapping/
  registration pins; the specgen determinism/bounds/mutation invariants incl.
  the seed-1 byte pin; the MC CLI wire formats (violation specs, assume
  selection, summary lines, counterexample formats) and the five byte-pinned
  JSON report shapes incl. the escaping contract; the JSON-schema emitter /
  walker / markdown renderer over a tiny + free-object + tagged-union-root
  corpus; the info-format registry invariants; the SMT emitter's width rules
  and Rust/C op-sequence parity; the reference MC's five outcome classes with
  byte-pinned text+JSON reports, mid-level timeout granularity, minimal-label
  regressions, CE replay/verification (trace/lasso/deadlock incl.
  assumption-pruned deadlock), and the independence import audit; the McPlan
  abstraction/encoding/traversal invariants (D-P4-2/D-P4-3) + liveness
  decomposition shapes/diagnostics (D-P4-9); the mc-ref CLI exit codes and
  pipeline diagnostics; the gen-spec CLI seed/caps/mutation pins; rfs-build's
  stub-driven RFS_BIN precedence, validation order, argv pins and cargo
  directives plus its real end-to-end compile check; the rfs-adapters minimal /
  callback / channel / async-stream behavior over a mock FSM (incl. the two
  no_std compile checks against a real generated FSM); and the toolchain fuzz
  layer's shared RandSpec wrapper + four target invariants.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|

## Tests run status

| Name | Status |
|---|---|
| `cargo test --test docs_md_test` (5) | OK - green after the rows were written (plain-text table cells; literal pipes in descriptions escaped as \|) |

## Found problems/issues/mismatches

- PLAN CORRECTION 30a: the Context's "smt_emit 6" inherited correction 2a's
  figure, which correction 6a ALREADY refuted during task 6 (all 10
  test-module fns are real - no template phantoms; the generated `fn` text
  lives inside format!/push_str strings and never starts a line). Task 30's
  masked read-through confirms 10 (5 #[test] + 5 helpers, zero raw strings in
  the module). Authoritative inline total: 103 (was ~99); every other
  per-file figure matched exactly.
- PLAN CORRECTION 30b: the Context's crate-test figures (26 fns / 14 types)
  were raw-grep inflated. nostd_minimal.rs is 5 fns / 0 types - its planned
  8/7 counted three phantom fns (fn new/step/check) and seven phantom type
  hits (struct NsMock, enum NsIn/NsOut/NsErr + the three `type Input/Output/
  Error` associated-type bindings) inside the r#"..."# no_std mock template.
  adapters.rs holds 4 real types - its `type Input/Output/Error` grep hits
  are associated-type bindings in `impl ProtocolFsm for MockFsm` (the binding-
  as-type phantom class of corrections 9a/13a; SRC_STRUCTURE.md enumerates no
  such bindings). Authoritative crate-test total: 23 fns / 4 types.
- PLAN CORRECTION 30c (completeness sweep): the preamble's "25 such host
  files" counts crates/rfs-build/src/lib.rs, whose REAL 12-fn test module was
  assigned to NEITHER task 29 NOR task 30 - enumerated here (12 rows,
  `crates/rfs-build/src/lib.rs::tests::<fn>` identifiers). Additionally, each
  macro-only fuzz target gets one row for its `fuzz_target!` libFuzzer entry
  (identifier `<file>::fuzz_target`) so all five files appear as their sole
  home - 4 rows beyond the 4 explicit fns correction 2a counted.
- The Context's "Documented-in: blank for inline tests and crate tests"
  conflicts with the BINDING preamble, which names the three workspace-crate
  test files as carrying real //! Docs: pointers whose fragments are copied
  (all eight files - three crate tests + five fuzz targets - are manifest
  members). The preamble governs: those rows carry their fragments; inline
  rows stay blank per the task-29 rule (the host's source rows carry the
  fragments).
- The extraction needed the same global string-state masking as task 29, and
  it caught a real bug in the first attempt: a per-line masker loses track of
  multi-line strings, so mc_plan.rs's REF_SPEC constant (unbalanced braces
  across lines) terminated the module scan early (1 fn found vs 16 real).
- Fixed four pre-existing raw-pipe cells from tasks 20-27 (prod_driver
  rust_sample_driver_source, examples_test example_count_is_reported,
  monitor_ref_test parse_samples, psl_translation_test parse_name_fields) -
  the same defect class task 24 fixed once; docs_md_test never catches it
  because comrak parses the extra columns as valid table cells.

## Other important findings/information

- adapters.rs's feature-gated test submodules get module-segment identifiers
  (`crates/rfs-adapters/tests/adapters.rs::channel_tests::<fn>`,
  `...::async_tests::<fn>`) - the same convention correction 21a established
  for library_comparison.rs's nom_smoke/chumsky_smoke modules. Its two
  ProtocolFsm impl methods get `<file>::MockFsm::{new,step}` method-form
  identifiers; the four mock types' rows note their methods in the
  description per the existing type-row convention.
- Final inventory totals: 940 function rows (tests/ tree 614 + inline 295 +
  crate tests 23 + fuzz 8) / 34 type rows. The delta vs correction 2a's ~901
  is fully explained: 30a +4, 30b -3, 30c +12 inline and +4 macro-entry rows,
  the remainder being correction 2a's own "~" approximations (its ~616 /
  ~255 component figures predate the string-masked read-throughs of tasks
  24-30). The task file's exit-criterion "~957" is the pre-correction-2a
  figure and is superseded accordingly.
- Every Depends target was cross-checked against the union of both documents'
  identifier columns - all resolve (no dangling edges).
