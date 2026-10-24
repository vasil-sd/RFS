# Phase 6, Task 3 - Implementation status

> Written once when all exit criteria of `PHASE_6_TASK_3.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

| Name | Description |
|---|---|
| `src/specgen.rs` | seeded random spec generator (shared LCG lineage with `codegen_test::Lcg`) + incremental mutation API (17 ops, per-step validator gate with roll-back/redraw, bounded 100 attempts/step, skip on exhaustion; `k=0` identity) |
| `src/ref_mc.rs` | brute-force reference MC: independent representative-input rule, own monitor compilers + trace evaluator, level-synchronous BFS (Vec frontier + HashMap visited), liveness via end-component analysis, deadlock detection; unified text/JSON rendering reusing ONLY `codegen_mc` data types; `minimal_violation_labels` for differential testing |
| `src/mc_ref_cmd.rs` | `rfs mc-ref` command logic (thin lib pipeline + CLI shell) |
| `src/gen_spec_cmd.rs` | `rfs gen-spec` command logic (fully random + base-mutation modes, always validated before write, default seed 12345 pinned) |

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| `src/main.rs` | change | dispatch arms + usage text for `mc-ref` and `gen-spec`; unknown-subcommand list updated |
| `src/lib.rs` | change | `pub mod specgen / ref_mc / mc_ref_cmd / gen_spec_cmd` |
| `src/backends/rust_mc.rs` | change | sweep finding Class A: counterexample value rendering now emits ref_mc's prefixed forms (`State::A`, `Input::Go { ... }`, `Output::...`, `Enum::Variant` in JSON too) via new shared renderers; sweep finding Class B: monitor obligation `remaining` bits packed only while pending so config identity matches ref_mc's CfgKey byte-for-byte |
| `src/backends/c_mc.rs` | change | same two fixes for the C backend (prefixed name printers; pending-gated remaining-bit packing) |
| `src/backends/rust_fsm.rs` | change | new `emit_renderers` + shared `enum_renderer_names` map used by both the FSM artifact and the MC JSON renderer, so all generated engines share one prefixed value form |

## Added/changed test files

| Name | Operation | Description |
|---|---|---|
| `tests/ref_mc_sweep_test.rs` | new | byte-parity sweep ref_mc vs generated Rust MC over every tractable valid example (fail-fast, N-way parallelism default CPUs/2, per-sample timeout default 60s, failed-first reordering, intractable pair skipped by default) |
| `tests/ref_mc_diag_test.rs` | new | non-fail-fast one-shot enumeration of every mismatch (triage companion to the sweep) |
| `tests/frontend_test.rs` | change | +7 e2e tests: mc-ref exit codes 0/1/2 pins, flag-parsing errors, text+JSON agreement with the compiled generated binary; gen-spec seed determinism, base+mutations reproducibility, 20-seed x both-modes validity sweep, error cases |
| `tests/backend_parity_test.rs` | change | re-pins after the Class B config-key fix (14/43/44/45 state counts now equal ref_mc's) + JSON golden doc updated to prefixed value forms |
| `tests/benchmark_mc_test.rs` | change | same re-pins in the benchmark pins (incl. shorter canonical lasso cycle for elev_noup) |
| `tests/codegen_test.rs` | change | no_std header pin: new `use alloc::format;` line from the renderer emission |
| `tests/liveness_mc_test.rs` | change | witness step-input pin now the prefixed `Input::Pong` form |

## Changed functionality

- Generated exact-mode MC binaries (Rust + C) now print counterexample values in ref_mc's prefixed forms (`State::A`, `Input::Go`, `Output::Done { ... }`, enum fields as `Enum::Variant` in both text and JSON). This is a report-format change required by D-P6-10 (unified byte-comparable reports); all affected pins were re-pinned in this task.
- Generated engines' monitor config keys now pack obligation `remaining` bits only while pending, so their configuration identity matches ref_mc's CfgKey exactly; state counts on obligation-bearing specs drop to ref_mc's values (re-pinned).

## New functionality

- `rfs mc-ref <spec> [--max-states N] [--timeout-sec N] [--assumes all|none|labels] [--counterexample-format json|text] [--log-level L]`: in-process reference model checking, exit codes 0/1/2 mirroring the generated binaries; usage text documents the non-applicable flags (--seed/--steps/--violate: fuzz-mode concepts; --threads/--fast: single-threaded by design; --nondet: all enabled transitions are explored by construction).
- `rfs gen-spec --out FILE.rfs [--seed N] [--size small|medium] [--max-states/--max-events/--max-vars/--max-transitions/--max-props N] | --base SPEC --mutations K [--seed N]`: deterministic random or mutated specs, always parse+validate-checked before writing.
- Library APIs for differential/fuzz tasks: `specgen::{SpecGen, Bounds, Rng, mutate, mutate_traced, MutOp}` and `ref_mc::{run, render_text, to_json_report, minimal_violation_labels, RefConfig}`.

## Removed functionality

- None.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|
| `src/specgen.rs` unit tests (10) | new | byte-pin of seed-1 small render; same-seed determinism; different-seeds-differ; 50 seeds x both presets parse+validate+bounds; k=0 identity; per-step validity k=1..20 over 3 bases x 4 seeds; mutation determinism; AddState coverage over a batch; deterministic RemoveState op semantics (feasible + infeasible) |
| `src/ref_mc.rs` unit tests (16, +1) | new/extended | the five hand-computed micro-spec outcomes with byte-exact text+JSON pins, independence import audit, and the new mid-level timeout regression (a single BFS level exceeding the budget still fires `--timeout-sec`) |
| `src/mc_ref_cmd.rs` unit tests (7) | new | exit-code mapping for all three statuses, JSON wire format (compact line + newline), flag parsing errors, validation-error diagnostics |
| `src/gen_spec_cmd.rs` unit tests (5) | new | default seed constant, determinism + validity of both modes, caps override, k=0 re-render, missing-base error |

## Tests run status

| Name | Status |
|---|---|
| `cargo build` (debug + release) | OK |
| `cargo test` (full suite, 429 tests / 22 targets) | OK - 0 failed |
| `cargo clippy --all-targets` | OK - no errors; warning set identical to the pre-task HEAD baseline (verified via a clean worktree: same files/counts), all in files untouched by this task |
| `cargo fmt --check` | OK |
| `ref_mc_sweep_test` (45 tractable examples, text+JSON) | OK - 45/45 byte-identical to the generated Rust MC binaries |
| `ref_mc_diag_test` (non-fail-fast enumeration) | OK - 45/45 |

## Other information

Independence import list of `src/ref_mc.rs` (recorded per the task file):
`std::{collections,time}`, `crate::ir::*`, and
`crate::codegen_mc::{exact_summary_line, parse_assume_selection, ExactStatus,
FailReason, JsonCounterexample, JsonIo, JsonStep, McJsonReport,
EXACT_DEFAULT_MAX_STATES, EXACT_DEFAULT_TIMEOUT_SEC}` (data types + shared
report helpers only); unit tests additionally use `crate::json_rw::Json`.
NOT imported: `mc_plan`, `monitor`, `codegen_fsm`, `printer`, backend modules -
enforced by the `independence_import_audit` test.

Sweep triage findings and resolutions (the reason this task stalled):

- Class A (15 samples, enum rendering): generated engines printed bare variant
  names where ref_mc prints prefixed forms. Fixed in all three emitters
  (rust_mc text+JSON, c_mc, rust_fsm renderers); pins re-pinned.
- Class B (4 samples, state-count divergence): the generated engines packed a
  stale `remaining` value into monitor config keys even when the obligation
  was not pending, so they counted configurations ref_mc's CfgKey merges.
  Fixed by packing `remaining` only while pending (both backends); pins
  re-pinned to ref_mc's counts.
- Class C (2 samples, "ref_mc deadlock"): NOT a deadlock - both engines run at
  100% CPU on these specs (the zero-CPU observation was an artifact of
  sampling the `timeout` wrapper process). 19_guard_complex needs ~4.3e9
  transition steps in ref_mc (hours) and OOMs the generated MC;
  39_endpoint_protocol is 1.34e9 configurations for both. They stay skipped by
  default in sweep/diag (documented in `INTRACTABLE_EXACT_MC`).
- Real bug found while closing Class C: ref_mc's wall-clock timeout was checked
  only BETWEEN BFS levels, so on a spec whose single level exceeds the budget
  (`--timeout-sec` was silently ignored for hours). Fixed with a per-config
  deadline check in `ref_mc::run`; regression test
  `timeout_fires_mid_level_on_giant_level` pins it (3s budget fires at ~3.0s).
