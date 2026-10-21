# Phase 5, Task 6 - Implementation status

> Written once when all exit criteria of `docs/roadmap/PHASE_5_TASK_6.md` are
> reached. Not rewritten afterwards; later fixes go into TASKS.md entries and
> follow-up phase plans.

## Added source files

| Name | Description |
|---|---|
| (none) | The task extends `src/backends/c_test.rs` (renderer), extracts shared signal machinery in `src/backends/c_fsm.rs`, and extends the two test suites. |

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| `src/backends/c_test.rs` | changed | Rewritten as a per-test harness mirroring the Rust renderer (task 5) with IDENTICAL coverage: each test is a plain-assert function run from a table in `main()`, one line per test (`PASS <name>` / `FAIL <name>`) - same stable names as Rust for ALL classes. Module-level tests keep their logic (initial state, positive greedy drive, negative search) but the negative search now drives with `RFS_CONTINUE` like the Rust baseline (correction C6-2). New renderers: per-transition (`t<id>_<state>_on_<event>` / `_not_taken` - installs `f.state`/`f.vars`, drives with a continue handler, asserts on the RECORDED trace step `last->to`/`last->outputs`; not-taken compares the landing against the transition's own signature computed in C, refusing to emit when signature outputs are not statically evaluable), invariant (`invariant_<label>` - atom rendered against `const rfs_sample_t*` via the extracted `render_signal_expr_c` with per-invariant `rfs_inv_<label>_in/out_<i>` pattern helpers; sample 0 + every driven sample, bounded = samples `< N`, fails with step index), monitor debug (`monitor_<label>_fires_on_violation` - asserts sticky `f.prop_label` holds the target label; initial-config case right after `fsm_new`, one-step case after installing state/vars + drive), random-trace (`random_trace_seed<seed>_steps<K>` - precomputed LCG index table over the harness `ALL_INPUTS`, stop handler, prints the SAME deterministic coverage line as Rust). NEW: `emit_fuzz_target_c` emits the SEPARATE `fuzz_target.c` skeleton - `LLVMFuzzerTestOneInput` decoding per `FuzzDecodePlan` (first byte = variant selector `% N`, bool = non-zero byte, bit<W> = ceil(W/8) little-endian masked bytes, enum = constant representative), looping over `fsm_step` with a local stop handler; header documents the clang + libFuzzer build recipe (`clang -O1 -fsanitize=fuzzer,address`); generation needs no clang. `generate_unit_tests` appends it as `CodegenOutput.extra_files` (the CLI already writes extras next to `--out` and refuses stdout mode - task 5 plumbing, now exercised by the C backend too). |
| `src/backends/c_fsm.rs` | changed | (a) EXTRACTED the per-sample signal machinery out of `Gen` into pub(crate) free functions shared with the test renderer: `render_signal_expr_c(module, sig, prefix, input_pats, output_pats)` (prefix-parameterized helper names; `Gen::gen_signal_c` delegates with prefix `mon_<m>` - monitor rendering byte-identical), `input_is_sample_helper_c` / `output_has_sample_helper_c` (full-prefix form; Gen methods delegate), `input_match_cond_c` / `output_match_cond_c` (moved verbatim, `self.variants` -> parameter), and `collect_liv_signal_patterns` made pub(crate). (b) `rfs_commit` now carries `RFS_UNUSED` (correction C6-1): transition-less modules previously left it unused -> `gcc -Wall` warning; verified the ONLY production-output delta vs the pre-change build on all 175 artifacts (45 examples + 130 training snippets) is that one token. |
| `src/backends/c.rs` | changed | `TEST_INFO` help text: five test classes + separate `fuzz_target.c`, `--out FILE` documented as REQUIRED (stdout mode refused), mirroring the Rust backend's task 5 update. |
| `tests/smt_bounded_test.rs` | changed | Pre-existing regression fix (found during this task's verification, fails identically on the pristine tree): the scratch cargo project under `target/smt_e2e_*` lacked an empty `[workspace]` table, so since the Phase 5 task 1 workspace conversion cargo walked up into the repository workspace and refused the build ("current package believes it's in a workspace when it's not"). Same one-line fix as the aggregate generated-code crate. Restores the documented `--features smt` baseline (339/339). |

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `CODEGEN.md` | changed | Section 5.2: rendering section now covers BOTH backends - C harness details (per-test table, PASS/FAIL wire format, recorded-trace-step assertions, invariant signal via `render_signal_expr_c`, sticky `prop_label` check, identical coverage line) + the separate `fuzz_target.c` skeleton with its clang recipe. |
| `src/backends/C.md` | changed | File-layout row for `c_test.rs` (per-test harness + extra file); dependency note for the extracted signal machinery; new "Test artifact" paragraph in section 2 (stable names, wire format, zero-warning bar incl. `RFS_UNUSED`/conditional handlers). |
| `BUILD.md` | changed | Section 3 counts re-pinned: root 333 -> **337** (codegen 39 -> 43 incl. the task 6 C renderer suite; parity row now mentions the per-test result-list + coverage-line comparison), workspace 341 -> **345**; parity-suite description extended; known-good baseline updated (2026-10-22, after Phase 5 task 6: 337/337, 339 with smt). |
| `KB.md` | changed | Section 1 Tests row: 337 with the current per-suite breakdown (parity incl. task 6 comparison; codegen 43); source-map row for the C backend files updated (per-test harness + separate fuzz skeleton). |
| `docs/roadmap/INTRO.md` | changed | Phase-state row 5 -> "tasks 1-6 + 8-9 of 16 done". |

## Other file changes

- `TASKS.md`: one new entry (2026-10-22).
- `docs/roadmap/PHASE_5_TASK_6.md`: "Task corrections" section appended (C6-1 zero-warning bar extended to all artifacts, C6-2 negative-test handler mirrored to Rust, C6-3 per-test PASS/FAIL lines introduced for ALL tests).

## Changed functionality

- `rfs codegen c test` output GROWS: the four new test classes are rendered as
  plain-assert functions (module-level tests unchanged in logic), and the
  command now ALSO emits `fuzz_target.c`.
- C test-executable stdout changes: one `PASS <name>` / `FAIL <name>` line per
  test for every class (the old harness printed only `FAIL: <msg>` diagnostics
  + a summary, no names); the final summary lines are kept.
- CLI contract change for that one command: `--out FILE` is now REQUIRED
  (stdout mode exits 1 with the diagnostic naming `fuzz_target.c`) - same
  strict behavior as the Rust `test` command since task 5.
- C production core: `rfs_commit` carries `RFS_UNUSED` (one-token delta on all
  artifacts; behavior unchanged).
- Negative unexpected-input search (C): trial steps now use a CONTINUE
  assumption handler, mirroring the Rust baseline exactly.

## Removed functionality

- None.

## New functionality

- C `test` artifacts carry per-transition unit tests, invariant re-checks,
  property-monitor debug tests, and a seeded random-trace coverage test -
  IDENTICAL coverage to Rust (same stable names, same expected outcomes).
- Separate dependency-free-by-construction `fuzz_target.c` libFuzzer skeleton
  per module (decode strategy from the shared planner; documented clang
  recipe; compiles warning-free with plain gcc against the generated
  production source - no clang needed to generate or to compile-check).
- Parity suite now compares BOTH backends' generated test executables on the
  full per-test result list (names + outcomes, canonical order) AND the
  byte-identical random-trace coverage line over all property-bearing
  examples, and asserts the fuzz skeleton exists in both backends.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|
| `c_test_emits_all_four_extended_classes_for_property_examples` (tests/codegen_test.rs) | new | 41 + 43: all four C classes present with the pinned names. |
| `c_test_names_follow_the_pinned_convention` | new | every name in the C harness `tests[]` table matches the pinned scheme (t<id>..._on_.../not-taken, invariant_..., monitor_..._fires_on_violation, random_trace_seed12345_steps4, module-level names) and equals its C function name. |
| `c_test_steps_option_changes_random_trace_class` | new | option sensitivity: steps=2 vs 6 change the random-trace test name and the precomputed index table content. |
| `c_test_emits_fuzz_skeleton_as_extra_file` | new | exactly one extra file named `fuzz_target.c`; `LLVMFuzzerTestOneInput` + clang recipe; 43's skeleton decodes the bit<4>/bit<32> payload widths (masked little-endian); main artifact carries no libFuzzer reference; BOTH the main test artifact and the fuzz skeleton (against the generated production source it includes) compile with `gcc -Wall` at zero warnings. |
| `cli_test_command_writes_fuzz_target_next_to_out` | changed | now loops over BOTH backends: `--out` writes the extra file next to the artifact (announced on stdout); stdout mode exits 1 with the diagnostic naming the extra file. |
| `generated_test_executables_parity_all_property_bearing_examples` (tests/backend_parity_test.rs) | changed | extended per task 6: fuzz-skeleton presence in both backends (one extra file, pinned name), full per-test result lists normalized to canonical order and compared byte-for-byte, and the random-trace coverage line compared byte-identically (Rust side re-run with `--exact ... --nocapture` because libtest captures stdout otherwise). |
| `generate_full` helper (tests/backend_parity_test.rs) | new | returns the full `CodegenOutput` (extras included); `generate` delegates to it. |
| `tests/smt_bounded_test.rs` scratch manifest | changed | empty `[workspace]` table (pre-existing regression fix; see Changed source files). |

## Tests run status

| Name | Status |
|---|---|
| `cargo build` | OK (clean) |
| `cargo test` (root package) | OK (**337/337**; was 333: codegen 39 -> 43) |
| `cargo test --workspace` | OK (**345/345**; was 341) |
| `cargo test --features smt` (z3 env per TOOLCHAIN.md 5.2) | OK (**339/339**; was red pre-task due to the pre-existing scratch-workspace bug, now fixed) |
| `cargo generate-code-tests && cargo test-generated-code` | OK (**1453/1453**, 179 modules - unchanged: Rust output byte-identical) |
| C sweep: all 45 valid examples x `codegen c test` -> `gcc -Wall` -> run | OK (417 per-test lines, 0 failures, **zero warnings on every artifact**) |
| Fuzz-skeleton sweep: all 45 examples -> `gcc -Wall -c fuzz_target.c` (against generated production source) | OK (45/45 clean, no clang required) |
| C production byte-stability vs pre-change build (175 artifacts) | OK (only delta: the single `RFS_UNUSED` token on the `rfs_commit` line in every artifact) |
| `cargo clippy --all-targets` | OK (identical to the 12-warning baseline, verified by diff) |
| `cargo fmt --check` | OK (clean) |

## Found problems/issues/mismatches

- Pre-existing (since Phase 5 task 1 workspace conversion): `--features smt`
  suite red - the scratch cargo project under `target/smt_e2e_*` lacked an
  empty `[workspace]` table, so cargo walked up into the repository workspace
  and refused the build. Fixed in this task (one line, same fix as the
  aggregate crate); recorded here because it surfaced during task 6
  verification and fails identically on the pristine tree.
- Pre-existing: `gcc -Wall` warnings on transition-less specs
  (`29_empty_module`: unused `rfs_commit` + three unused harness items;
  `30_multi_module` module `alpha`: unused `rfs_commit`) - never compiled by
  any suite before this task. Fixed per correction C6-1 (the new per-test
  structure removed the three harness warnings; `RFS_UNUSED` + conditional
  handlers remove the rest).
- libtest with `--nocapture` places a test's stdout either INLINE after
  `test <name> ... ` or on its own line BEFORE the prefix, depending on
  buffering - the parity parser therefore extracts the coverage line by
  substring (position-independent) and compares per-test lists in canonical
  (sorted) order rather than execution order.

## Other important findings/information

- The C harness reuses the core's `rfs_sample_t` for invariant checks, so the
  atom rendering is the SAME code path as the online monitors (extracted, not
  duplicated) - a signal that renders for a monitor renders for an invariant.
- For task 7 (`rfs test`): the C executable already prints per-test results +
  the coverage line and exits 0/1 - the command can drive it directly; the
  Rust side needs `--test-threads=1` (deterministic order) but NOT
  `--nocapture` for pass/fail parsing (only for the coverage line).
- For task 12 (C fuzz example): the skeleton's include is pinned to
  `rfs_generated_fsm.c` next to itself (ADJUST note in the header); the
  compile-check pattern used by the tests (write production output under that
  exact name, `gcc -Wall -c`) works without clang.
