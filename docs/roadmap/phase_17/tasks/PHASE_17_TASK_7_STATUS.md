# Phase 17, Task 7 - Implementation status

> Written once when all exit criteria of `PHASE_17_TASK_7.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

| Name | Description |
|---|---|
| `crates/examples/no_std_fsm/Cargo.toml` | member manifest: rfs-build build-dep only (no rfs-adapters - the driver uses the artifact's inherent API) |
| `crates/examples/no_std_fsm/build.rs` | generates spec 03 with `.no_std(true)`; strips inner attributes; byte-identical body to OUT_DIR |
| `crates/examples/no_std_fsm/src/lib.rs` | genuinely `#![no_std]` + `extern crate alloc` lib: includes the artifact in `pub mod timeout_retry`, `run_session` drives a full session, std-context test |
| `crates/examples/monitors_production/Cargo.toml` | member manifest: rfs-build build-dep only |
| `crates/examples/monitors_production/build.rs` | generates spec 41 with `.with_property_monitors(true)`; shared strip pattern |
| `crates/examples/monitors_production/src/lib.rs` | `run_correct_session` (happy path, monitors quiet), `run_violating_session` (`#[cfg(debug_assertions)]`, surfaces the violation), `run_unmonitored_session` (`#[cfg(not(debug_assertions))]`, release twin) + tests |
| `crates/examples/callback_minimal_adapters/Cargo.toml` | member manifest: rfs-adapters dep + rfs-build build-dep |
| `crates/examples/callback_minimal_adapters/build.rs` | generates spec 03 with `.with_protocol_fsm(true)`; shared strip pattern |
| `crates/examples/callback_minimal_adapters/src/lib.rs` | callback drivers (completion / failure / step-limit) + minimal drivers (forwarding / composition) over the real generated FSM + tests |

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| `Cargo.toml` | changed | workspace members list gains the three crates |
| `tests/example_projects_test.rs` | changed | + `cargo_member` helper + three nested-cargo build+test legs (monitors_production additionally runs its release suite) |

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `crates/examples/no_std_fsm/README.md` | new | template README: what --no-std buys, the automaton + mermaid diagram + transition table, files layout, build/run/test, expected results (pinned session outcome) |
| `crates/examples/monitors_production/README.md` | new | template README: production monitors explained, automaton + property-structure mermaid diagrams, debug violation pin (fin_acked_bounded at sample 5), release zero-cost, C leg note (-DRFS_PROPERTY_MONITORS) |
| `crates/examples/callback_minimal_adapters/README.md` | new | template README: both never-demonstrated adapters, all three CallbackResult outcomes, minimal forwarding + composition, automaton diagram |
| `docs/llm/SRC_STRUCTURE.md` | changed | +18 rows (same-commit rule); "Example crates" component row + directory tree extended; 1810/1830 |
| `docs/llm/TEST_STRUCTURE.md` | changed | +13 rows (9 inline tests + cargo_member + 3 legs); host-file count 26 -> 29, Inline-unit-tests row count updated, Example-projects-suite row + example_projects_test file row extended; 1396/1430 |
| `docs/llm/TASK_MAP.md` | changed | Examples row + six new file entries (doc->code coverage for the sync gate) |
| `docs/dev/workspace_crates.md` | changed | Examples section: opening rewritten (six Rust members), three new crate paragraphs, Pinning + Code lists extended |
| `docs/llm/workspace_crates.md` | changed | card: member count, crate-to-file map (17 -> 23 files) with three new rows, build.rs pattern note (unused_comparisons), example-parity invariants, change-impact recipe (+ dev_docs_manifest.txt step); stale "559 baseline" count dropped |
| `tests/dev_docs_manifest.txt` | changed | six new entries (the sync gate's completeness test pins manifest == filesystem scan) |

## Changed functionality
None - example crates + tests + docs only.

## Removed functionality
None.

## New functionality
- Three workspace-member example crates covering usage scenarios no current example covers: `--no-std` production codegen (a genuinely no_std crate around the generated artifact), production property monitors (debug violation at the step API + mechanically pinned release zero-cost), and the two never-demonstrated rfs-adapters (callback driver reaching all three outcomes; minimal wrapper forwarding + composition) - each driven against a REAL generated spec.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|
| `no_std_fsm/src/lib.rs::tests::full_session_through_nostd_artifact` | new | full session through the #![no_std] artifact: 3 Retries, 1 Drop, idle Ack rejected as UnexpectedEvent, resend + Ack success |
| `monitors_production/src/lib.rs::tests::correct_session_keeps_monitors_quiet` | new | happy path: three Ok steps, one Send(Ack), no violation |
| `monitors_production/src/lib.rs::tests::violating_session_surfaces_property_violation` | new | Fin + four Ticks (no Ack): steps 1-4 Ok, step 5 Err(PropertyViolation { label: "fin_acked_bounded", step: 5 }) - the MC counterexample values |
| `monitors_production/src/lib.rs::tests::release_build_compiles_monitors_out` | new | release build: the identical sequence steps all Ok (zero-cost claim pinned mechanically) |
| `callback_minimal_adapters/src/lib.rs::tests::callback_completion_stops_after_drop` | new | Completed(5) - the reactive driver stops when it observes Drop |
| `callback_minimal_adapters/src/lib.rs::tests::callback_failure_reports_the_fsm_error` | new | Failed(UnexpectedEvent, 0) - out-of-band Ack rejected |
| `callback_minimal_adapters/src/lib.rs::tests::callback_step_limit_halts_at_max_steps` | new | StepLimit(4) - never-stopping closure halted at max_steps |
| `callback_minimal_adapters/src/lib.rs::tests::minimal_forwards_outputs_and_errors` | new | the wrapper forwards Retry x3 + Drop and the idle-Ack rejection, then into_inner hands the FSM back |
| `callback_minimal_adapters/src/lib.rs::tests::minimal_composes_with_the_callback_driver` | new | Minimal<Fsm> driven through the callback driver: the same Completed(5) |
| `example_projects_test::cargo_member` | new | nested-cargo build/test helper (fails with stderr on nonzero exit) |
| `example_projects_test::no_std_fsm_builds_and_tests_the_nostd_artifact` | new | build + test leg, pinned session test must pass |
| `example_projects_test::monitors_production_tests_debug_violation_and_release_quiet` | new | build + DEBUG suite (pinned violation) + RELEASE suite (pinned zero-cost) legs |
| `example_projects_test::callback_minimal_adapters_exercises_both_adapters` | new | build + test leg, pinned completion + composition tests must pass |

## Tests run status

| Name | Status |
|---|---|
| `cargo build --workspace` | OK |
| `cargo test --workspace` (715 = 704 baseline + 11 new) | OK |
| `cargo test -p monitors_production --release` | OK (2 tests, zero-cost pin) |
| `cargo test --test dev_docs_test` (9) | OK |
| `cargo test --test docs_md_test` (5) | OK (new READMEs' mermaid + links render/resolve) |
| `rfs-dev inventory audit` | EXACT: SRC 1013 fns / 213 types, TEST 1048 fns / 35 types - zero per-file diffs |
| `cargo clippy --all-targets` | no warnings in the new code (the two rfs-adapters lib warnings are pre-existing) |
| `cargo fmt --check` | clean |

## Found problems/issues/mismatches
- `cargo test -q` replaces per-test lines with dots, so a leg that pins test names must run nested cargo WITHOUT -q (the legs' comment says so).
- The generated `always[10]` monitor code contains an always-true `idx as u64 >= 0` comparison: monitors_production's module-level #[allow] carries the mirrored artifact list PLUS unused_comparisons (documented in the lib.rs comment + the llm card).
- A reactive closure that detects "first call" via an empty previous-outputs slice breaks when the first step emits no output (Send) - the shared driver carries its own `sent` flag instead.
- The release `StepError` has NO PropertyViolation variant, so code matching it must be cfg-gated: run_violating_session is debug-only, run_unmonitored_session is the release twin.

## Other important findings/information
- All pinned values were verified against real engine output BEFORE writing the tests (probe drivers over the generated artifacts): spec 03's session shape (3 Retries / Drop / UnexpectedEvent) and spec 41's monitor behavior (correct session quiet; violating session Err at step 5 with label fin_acked_bounded).
- The no_std artifact's `extern crate alloc;` is a plain item, not an inner attribute - the shared strip pattern leaves it in the body, where it is legal (and redundant) inside the including module.
