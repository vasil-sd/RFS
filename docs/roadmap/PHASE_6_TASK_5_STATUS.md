# Phase 6, Task 5 - Implementation status

> Written once when all exit criteria of `PHASE_6_TASK_5.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

| Name | Description |
|---|---|
| `fuzz/Cargo.toml` | Detached cargo-fuzz harness package (`rfs-fuzz`, `[workspace]` + `cargo-fuzz = true` marker, deps `libfuzzer-sys = "0.4"` / `arbitrary = "1"` / `rfs = { path = ".." }`) with four `harness = false` bins - the fast layer of D-P6-4 |
| `fuzz/fuzz_targets/common.rs` | Shared specgen-backed Arbitrary wrapper (fuzzer bytes -> seed + bounds -> random VALID spec; raw IR types deliberately NOT `Arbitrary`) + head/rest split helpers |
| `fuzz/fuzz_targets/parser.rs` | Target: arbitrary bytes -> `parse_source` no panic; on success validate + JSON round-trip byte-stable |
| `fuzz/fuzz_targets/validator.rs` | Target: specgen-backed valid IRs -> `validate_ir` no panic; raw JSON bytes -> `Ir::from_json` Err-or-Ok, never panics |
| `fuzz/fuzz_targets/ir_roundtrip.rs` | Target: random valid IRs (optionally mutated) -> to_json/from_json/to_json byte-stable |
| `fuzz/fuzz_targets/codegen.rs` | Target: random valid spec -> 3 commands x 2 backends generate Ok + structural invariants (task-2 header line, balanced delimiters with comment/string/char-literal skipping, no placeholder tokens, core symbols `pub struct Fsm` / `fsm_t`+`fsm_new`) |
| `fuzz/README.md` | Build/run instructions incl. nightly requirement + target table |
| `tests/toolchain_fuzz_test.rs` | Slow CI-runnable layer: 3 pinned seeds x 10 small specgen specs -> render/re-parse/validate -> six artifacts generated + compiled (rustc / gcc -Wall) -> MC runs exact (shared `--max-states 5000`) + fuzz vs `ref_mc` ground truth (status agreement, NO false PASS, exit codes {0,1,2}, stable stderr, sample-0 assumption-abort asymmetry pinned) + deterministic trigger-step-0 report pin |

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| `src/codegen_test.rs` | changed | Plan correction 5a: `ConcreteInput` now carries binding values keyed by BINDING name (built in `concrete_input_for_pattern`); both construction sites (`plan_transitions`, monitor-violation drive) use it - fixes total-codegen failure on valid specs with binding-referencing emits. +2 regression pins (`binding_referenced_emit_evaluates_in_not_taken_signature`, `fuzz_regression_seed_24099_test_codegen_succeeds_both_backends`) |
| `src/backends/c_mc.rs` | changed | Plan correction 5b: fuzz-mode `report_property_violation` now renders the trigger-step vars via the safe `trace_vars()` helper (was raw `steps[prop_trigger_step - 1]` = `steps[-1]` SEGV at trigger step 0) |

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `BUILD.md` | changed | New section 6 "Toolchain fuzzing" (two-layer table, targets, run commands, offline note, smoke gate) + one availability-gated smoke line in the section-5 verification checklist |
| `TOOLCHAIN.md` | changed | New section 7.5: root `fuzz/` workspace usage + observed conventions (`#![no_main]` + top-level macro, `--` arg separator, specgen wrapper, CI layer pointer) |

## Other file changes

| Name | Operation | Description |
|---|---|---|
| `.gitignore` | changed | added `fuzz/target`, `fuzz/corpus/`, `fuzz/artifacts/` (libFuzzer runtime state is regenerable; regression inputs are tracked under `fuzz/regression_seeds/`) |
| `fuzz/regression_seeds/codegen_empty_input` | new | regression seed: empty input (scanner false-positive case, fallback spec seed 0) |
| `fuzz/regression_seeds/ir_roundtrip_short_rest` | new | regression seed: short-input `copy_from_slice` harness bug |
| `docs/roadmap/PHASE_6_PLAN.md` | changed | "Plan corrections" table: rows 5a (planner binding keying) + 5b (C MC trigger-step-0 SEGV) appended per the bug-found protocol; "Planned tasks" table untouched |

## Changed functionality
- Test planner (`src/codegen_test.rs`): emit/guard expressions referencing PATTERN BINDINGS now evaluate to concrete values. Taken tests that previously degraded to partial-match assertions can now carry full literal assertions; not-taken signature outputs are fully evaluable, so `test` codegen is TOTAL over valid specs (previously: hard error on both backends for such specs).
- C MC fuzz-mode report (`src/backends/c_mc.rs`): "vars at trigger step" renders correctly when the failing obligation started at sample 0 (previously: out-of-bounds read / SEGV).

## Removed functionality
- None.

## New functionality
- Root-level `fuzz/` cargo-fuzz workspace with four in-process libFuzzer targets over the toolchain itself (parser, validator, IR serialization, code generators) - D-P6-4 fast layer.
- `tests/toolchain_fuzz_test.rs`: seeded deterministic compile-and-run fuzz suite (part of plain `cargo test`): 30 small specs x six artifacts compiled in both backends, MC exact+fuzz runs cross-checked against `ref_mc` (no false `PASS (complete)`, stable diagnostics), plus the deterministic trigger-step-0 report pin - D-P6-4 slow/CI layer.
- BUILD.md section 6 + TOOLCHAIN.md section 7.5 documentation and the availability-gated bounded smoke gate in the verification checklist.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|
| `tests/toolchain_fuzz_test.rs` | new | the slow suite (`toolchain_fuzz_slow_suite`, ~28 s) + `c_mc_trigger_step_zero_report_no_crash` (plan correction 5b pin, both backends) |
| `src/codegen_test.rs` lib tests | changed | +2 pins for plan correction 5a (planner-level binding evaluation + seed-24099 both-backends codegen success) |

## Tests run status

| Name | Status |
|---|---|
| `cargo test` (root, 447) | OK |
| `cargo test --workspace` (471) | OK |
| `cargo generate-code-tests` + `cargo test-generated-code` (1519, 186 modules - regenerated after the planner fix) | OK |
| `cargo clippy --all-targets` | OK (no new warnings; pre-existing lib warnings only: c_fsm.rs x8, validator.rs, codegen.rs, test_cmd.rs) |
| `cargo fmt --check` | OK |
| fuzz smokes: `parser`/`validator`/`ir_roundtrip` (`-max_total_time=10`) + `codegen` (`-max_total_time=60`) via `cargo +nightly fuzz run` | OK (all exit 0, zero crashes; the two harness bugs found during smokes were fixed and their inputs pinned as regression seeds under `fuzz/regression_seeds/`) |

## Found problems/issues/mismatches
- **Plan correction 5a** (real toolchain bug, found by the new `codegen` fuzz target): valid spec (specgen seed 24099 / small) with a guarded transition whose pattern binds an input field and whose emit references that binding made `test` codegen FAIL on BOTH backends - the planner keyed binding values by declared field names while `eval_env` looks them up by binding name, so the not-taken renderer's strict "toothless test" backstop errored. Fixed + pinned (see above).
- **Plan correction 5b** (real toolchain bug, found by the new slow suite): generated C MC binary SEGFAULTED (ASan: SEGV READ in `print_vars_debug`) in fuzz mode whenever a property violation's obligation started at sample 0 (`prop_trigger_step == 0` -> `steps[-1]`). C-only: the exact-mode path and the Rust backend already handled pos 0. Fixed + pinned with a deterministic micro-spec.
- Harness-side bugs (task-5's own new code, fixed within the task, no plan correction needed): structural-scanner false positive on char literals containing `"` (the MC JSON escape renderers); `ir_roundtrip.rs` `copy_from_slice` panic on <8-byte inputs; initial targets' `fn main()` wrappers shadowing the libFuzzer runtime main (silent exit-0 binaries).

## Other important findings/information
- Nightly WAS available (`rustc 1.101.0-nightly`), so the full `cargo +nightly fuzz build/run` path was used - NOT the check-only fallback of the plan's assumption 1; no correction needed on that point.
- `arbitrary`'s integer `Arbitrary` impls are LENIENT on short input (they fill from the available bytes instead of erroring): a 2-byte fuzzer input maps to seed 24099, an empty input to seed 0 - which is how the crash inputs decoded.
- cargo-fuzz 0.13 forwards libFuzzer args only AFTER `--` (`cargo fuzz run <t> -- -max_total_time=10`).
- Slow-suite runtime measured ~28 s (budget ~5 min) - no seed-set reduction needed; 3 seeds x 10 specs kept as planned, with seed 24099 included as the 5a regression seed.
- Root main-test count moved 445 -> 447 (+2 new tests); workspace 469 -> 471. Final baseline re-pins (BUILD.md section 3/5 numbers) belong to task 9 per the plan's file ownership; this task touched only its own BUILD.md/TOOLCHAIN.md sections.
- The `fuzz/` workspace is detached: it is NOT covered by root `cargo test --workspace` / `clippy` / `fmt` (verified clean separately during development).
