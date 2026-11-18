# Phase 16, Task 3 - Implementation status

> Written once when all exit criteria of `PHASE_16_TASK_3.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

| Name | Description |
|---|---|
| `src/mc/ref_mc_fuzz.rs` | the seeded fuzz-mode engine (665 lines, no feature gate): `FuzzReport` (Completed = summary line / Violated = per-run report), `FuzzFsm` (one run's machine state: control state, vars, property + assumption monitor states, trace cloned for draw-free trial steps), `TraceStep`, `StepErr` (AssumptionViolation / PropertyViolation / UnexpectedEvent); `step_fsm` - one deterministic first-match step recording the trace, stepping SELECTED assumption monitors under Stop/Continue policy then property monitors (first fire wins), committing state/vars only on success (mirrors the generated `commit_step`); `render_violation_report` - the per-run report in the EXACT byte format of the generated `report_property_violation` + `print_trace`, built from ref_mc's reused renderers; `run` - the run loop with the generated runtimes' EXACT draw sequence (per-run `violating[i] = chance(prob)` draws, then per step: forced phase `chance(target_fraction)` + draw-free ALL_INPUTS scan for a label-matching violation input, unexpected phase `chance(unexpected_prob)` + `pick_index`, allowed phase draw-free scan + `pick_index`; trial steps consume NO draws); sample-0 assumption check uses ALL assumptions (`Fsm::new` parity). Five unit tests pin the draw-sequence contract on tiny specs (all-complete summary bytes, forced violation at 100%, unexpected injection every step, stuck on empty allowed list, report byte-pins). |

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| `src/codegen/mc_concepts.rs` | changed | D-P16-5 single-sourced wire format: `FUZZ_SUMMARY_FORMAT` constant (the exact summary line both generated runtimes print) + `fuzz_summary_line(runs, bound, completed, unexpected, assumption_stops, property_violations)` renderer (fills the six `{}` slots in order - `format!` needs a literal on stable; a unit test byte-pins the rendered form). Plus the shared fuzz defaults `FUZZ_DEFAULT_BOUND` (5) / `FUZZ_DEFAULT_RUNS` (20) / `FUZZ_DEFAULT_SEED` (12345) so `rfs mc-ref --mode fuzz` and the generated binaries agree without copy-paste. |
| `src/codegen/backends/rust/mc.rs` | changed | `FUZZ_RUNTIME`'s inline summary `println!` now interpolates the `__FUZZ_SUMMARY_FORMAT__` placeholder, filled from the shared constant at codegen time (the LCG-interpolation precedent). Emitted bytes UNCHANGED - proven by 1683/1683. |
| `src/codegen/backends/c/mc.rs` | changed | the C fuzz runtime's printf format is derived at codegen time as `FUZZ_SUMMARY_FORMAT.replace("{}", "%d")`. Emitted bytes UNCHANGED - proven by 1683/1683. |
| `src/mc/ref_mc.rs` | changed | mode-dispatch entry (D-P16-1): new `FuzzConfig` (bound/runs/seed/assume_arg/violate/unexpected_prob, Default = the generated built-ins) + `pub fn run_fuzz(module, cfg)` delegating to `ref_mc_fuzz::run`. New `pub(crate) fn fuzz_inputs` - ONE representative input per event variant in declaration order, exactly the generated runtimes' `ALL_INPUTS` (one representative value per field, NOT exact mode's full-domain Cartesian product); matching it is what makes the seeded draw sequence parity-hold. Widened to `pub(crate)` for the engine: `render_vars`, `render_input`, `render_output`, `render_outputs`, `initial_config`. |
| `src/mc/mod.rs` | changed | module declaration for `ref_mc_fuzz`; doc comment updated (the independence contract is now scoped per file). |
| `src/cli/commands/mc_ref.rs` | changed | D-P16-4 unified CLI: new `McMode` enum + case-sensitive `parse`, `--mode exact\|fuzz` (default exact), the six fuzz flags (`--bound N` 5, `--runs N` 20, `--seed N` 12345, `--assumes`, `--violate label:prob:percent%,...`, `--unexpected-prob P` 0.0). Exact-only flags (`--max-states`/`--timeout-sec`/`--counterexample-format`/`--nondet`) are a usage error under `--mode fuzz` (the generated fuzz binary has no such flags). Exit contract mirrors the generated binaries: fuzz 0 no property violation (summary line) / 1 violation found (per-run report, NO summary line); exact unchanged. Module doc + usage text rewritten with the full per-mode surface (`--threads`/`--fast` documented N/A). |

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `docs/llm/SRC_STRUCTURE.md` | changed | +10 rows: `fuzz_summary_line`, `McMode::parse`, `FuzzConfig` (+Default), `run_fuzz`, `fuzz_inputs`, and the six `ref_mc_fuzz.rs` items (`run`, `step_fsm`, `io_in`, `io_out`, `render_violation_report`, plus the type rows `FuzzReport`/`FuzzFsm`/`TraceStep`/`StepErr`); llm-budget 1750 -> 1760. |
| `docs/llm/TEST_STRUCTURE.md` | changed | +22 rows: the mc_concepts byte-pin, the five engine unit tests (+2 helpers), and the whole `tests/ref_mc_fuzz_test.rs` suite (helpers + 3 tests + `RefFuzz`); llm-budget 1335 -> 1357. |
| `docs/llm/TASK_MAP.md` | changed | the mc-ref row now documents `--mode exact\|fuzz` with the per-mode flag lists, the exact-only rejection, and the mirrored exit codes. |
| `docs/dev/mc_engine.md` | changed | new dev:mc-engine-fuzz fragment: the 3-phase strategy, the EXACT draw sequence (which draws happen where, trial steps draw-free), the sample-0 ALL-assumptions parity, the report/summary byte formats and their single-sourcing. |
| `BUILD.md` | changed | mc-ref CLI line: `--mode exact\|fuzz` + the fuzz flag surface with defaults, the exact-only rejection, and the per-mode exit codes. |

## Other file changes

| Name | Operation | Description |
|---|---|---|
| `tests/dev_docs_manifest.txt` | changed | +`src/mc/ref_mc_fuzz.rs` (the dev-docs coverage manifest). |
| `TASKS.md` | appended | the task-3 entry. |

## Changed functionality
- `rfs mc-ref` CLI surface: `--mode exact\|fuzz` selects the engine (exact is the
  default and byte-unchanged); the exact-only flags (`--max-states`,
  `--timeout-sec`, `--counterexample-format`, `--nondet`) are now a usage error
  when combined with `--mode fuzz`.
- The generated fuzz runtimes' summary line is now rendered from the shared
  `mc_concepts::FUZZ_SUMMARY_FORMAT` constant instead of an inline literal in
  each template - emitted artifact bytes are UNCHANGED (1683/1683 proves it).

## Removed functionality
- None.

## New functionality
- `rfs mc-ref --mode fuzz`: seeded random testing WITHOUT any compilation cycle -
  the 3-phase input strategy with the generated runtimes' EXACT RNG draw sequence
  (shared LCG), `--violate label:prob:percent%,...` forced-violation directives,
  per-run property-violation reports byte-identical to both generated binaries,
  the fuzz summary line after all runs, and the mirrored 0/1 exit contract
  (a violating run prints the report and NO summary line).
- Shared wire format (D-P16-5): `FUZZ_SUMMARY_FORMAT` + `fuzz_summary_line` in
  `mc_concepts.rs` - one encoding, three consumers (Rust template, C template,
  ref engine) - plus the shared fuzz defaults.
- Cross-backend byte-parity suite: summary lines + per-run reports vs BOTH
  generated fuzz binaries at fixed seeds over eligible examples + 20 pinned-seed
  random specs, with exit-code mirroring.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|
| `src/mc/ref_mc_fuzz.rs::tests::*` (5 tests + 2 helpers) | new | draw-sequence pins on tiny specs: all-runs-complete summary bytes, forced violation at prob=1.0/percent=100% (assumption_stops=runs), unexpected injection every step (`--unexpected-prob 1.0`), stuck on an empty allowed list, and the per-run report byte-pins. |
| `src/codegen/mc_concepts.rs::tests::fuzz_summary_line_format` | new | byte-pins the rendered `FUZZ_SUMMARY_FORMAT` (two value sets). |
| `tests/ref_mc_fuzz_test.rs` (3 tests) | new | `fuzz_default_flags_byte_parity` + `fuzz_flag_combinations_byte_parity` (custom bound/runs/seed + `--unexpected-prob 0.5` + a `--violate` directive) over eligible examples + 20 pinned-seed random specs, and `fuzz_violation_report_byte_parity_and_exit_one` (guaranteed violation: report byte-identical to BOTH backends, exit 1, no summary line). Eligible set = the exact-mode diff exclusions PLUS `29_empty_module` (see findings). |

## Tests run status

| Name | Status |
|---|---|
| `cargo test` (572) | OK - = task 2's 563 + 6 lib + 3 integration |
| `cargo test --workspace` (700 passed, 2 ignored) | OK - = 691 + 9; the 2 ignored are pre-existing |
| `cargo test --features smt` (574) | OK - = 565 + 9 |
| `cargo test-generated-code` (1683/1683) | OK - UNCHANGED: the template interpolation moved no bytes |
| `cargo fmt --check` | OK - clean |
| `cargo clippy --all-targets` (61 warning lines / 37 unique) | at baseline + 5 new lib warnings in `ref_mc_fuzz.rs` + 1 new test-target summary line - routed to task 5's close-out per user direction (see findings) |
| `rfs-dev inventory audit` | OK - EXACT (SRC 976 fns / 206 types, TEST 1009 fns / 35 types - zero per-file diffs) |
| `rfs-dev sync-check` | OK - clean (exit 0) |

## Found problems/issues/mismatches
- **Five new clippy warnings in `src/mc/ref_mc_fuzz.rs`** (all lib, none in any
  pre-existing file): 2x `doc_lazy_continuation` (module doc lines 17-18 - list
  continuation without indentation), 2x `too_many_arguments` (8/7) on
  `render_violation_report` and `step_fsm`, 1x `collapsible_if` (line 198).
  Clippy is therefore at 61 warning lines / 37 unique instead of the 55/34
  baseline. Per user direction these were NOT fixed in this task (the session
  had been looping on clippy); they are routed to task 5's close-out, which was
  adjusted to fix them first and record the fix as a plan correction, so the
  phase still exits with clippy at baseline.
- **Pre-existing Rust/C fuzz parity gap on `29_empty_module`** (a no-event
  module): at seed 7 / bound 3 / runs 4 / `--unexpected-prob 0.5` the two
  GENERATED backends disagree with each other - Rust reports completed=1, C
  reports unexpected=3 (the C runtime mis-counts an empty input list as
  unexpected events). ref-mc follows the documented draw-sequence contract and
  matches the RUST backend; no ref-mc behavior can match both. Excluded from the
  byte-parity suite (alongside the two budget-bound examples) so the suite stays
  green; flagged for a future codegen fix.

## Other important findings/information
- The plan's Context said "add `pick_index` to `utils::rng`" - it ALREADY EXISTS
  with exactly the required f64 form `((next_f64() * len as f64) as usize) %
  len`; no rng.rs change was needed (only use), as the task file anticipated.
- The generated templates' inline `chance` clamps p to [0,1] while
  `utils::rng::Rng::chance` does not - behaviorally identical because
  `next_f64()` is in [0,1) (p > 1 always true, p < 0 always false either way);
  no action needed.
- Sample-0 assumption-violation asymmetry: the generated fuzz binaries
  panic/abort on a sample-0 assumption violation while ref-mc reports a clean
  error naming the same label - `compare_one_fuzz` handles it (both must name
  the same label), mirroring the exact-mode diff suite's convention.
- The `property_violations` summary field is always 0 by construction (a
  violation exits immediately, printing no summary line) - kept and pinned for
  a stable format, as planned.
