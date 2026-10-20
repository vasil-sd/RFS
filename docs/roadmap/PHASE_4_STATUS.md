# Phase 4 - Implementation status

> Written once when all exit criteria of `PHASE_4.md` are reached. Not rewritten
> afterwards; later fixes go into `TASKS.md` entries and follow-up phase plans.

Phase 4 replaced the seeded-fuzz model checker with an EXACT reachable-state
engine (default in both backends), added unbounded liveness (`eventually`,
WEAK `until`) with lasso counterexamples, input abstraction for unused payload
fields, a pinned three-status report format, level-parallel exact BFS
(`--threads`, byte-identical for any N) plus runtime-only `--fast`, benchmark
specs 43/44/45 with a parity suite, and an optional SMT bounded mode
(feature-gated `smt`). Legacy fuzz is retained behind `--mode fuzz`.
Decisions as shipped: `DESIGN.md` section 22 (D-P4-1..D-P4-14).

## Added source files

| Name | Description |
|---|---|
| `src/mc_plan.rs` | Shared MC plan (D-P4-1): input-abstraction analysis, state-encoding layout, canonical traversal spec, liveness obligation decomposition - computed once in the Rust tool and embedded as constants/tables by BOTH backends. |
| `src/smt_plan.rs` | Backend-neutral SMT spec + IR->spec conversion with the shared v1-scope rejections (task 11). |
| `src/smt_emit.rs` | Shared emitter: renders one spec into BOTH backends' z3 constraint sections - parity by construction (task 11). |

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| `src/parser.rs` | changed | Liveness language: infix WEAK `until` level between `&&` and `!`, `eventually (f)` primary; dedicated diagnostic for parens-less `eventually` (task 13, user-requested). |
| `src/ir.rs` | changed | Additive `Formula::Eventually` / `Formula::Until` variants - no IR_VERSION bump. |
| `src/printer.rs` | changed | Liveness round-trip (RFS output for the new operators). |
| `src/validator.rs` | changed | Liveness scope check (D-P4-9): top-level obligations over boolean subformulas; deeper nesting -> warning + skip. |
| `src/codegen.rs` | changed | MC option surface: `--mode exact\|fuzz\|smt-bounded`, `--max-states`, `--nondet`, `--timeout-sec`, `--threads`, `--smt-bound` (embedded as runtime-overridable defaults). |
| `src/codegen_mc.rs` | changed | Exact-mode concepts: `ExactStatus` taxonomy + wire/human/exit_code, `FailReason`, pinned summary-line format, `EXACT_DEFAULT_*` / `SMT_DEFAULT_BOUND`; fuzz types retained for `--mode fuzz`. |
| `src/backends/rust_mc.rs` | changed | Exact BFS engine (default) + liveness engine + two-phase `--threads` + runtime `--fast`; fuzz runtime retained; standalone SMT branch. |
| `src/backends/c_mc.rs` | changed | Mirror of the Rust exact engine (sorted-vector containers, pthreads + C11 atomics for `--threads`, gcc -Wall clean); liveness mirror with per-property label cache; SMT branch linking libz3 5.3.1. |
| `src/backends/c_fsm.rs` | changed | Pre-existing gap closed (task 8): `+,-,*` now masked to RFS bit width after cast (was wrapping at the C type width - token ring's holder 3->0 wrap exposed it); trace/vars printers rewritten to the Rust Debug wire format (byte-identical FAIL traces). |
| `src/backends/rust_fsm.rs`, `rust.rs` | changed | Core rendering support for the MC rework (monitor/trace plumbing, helpers). |
| `src/monitor.rs` | changed | Monitor-subset boundary: any formula containing eventually/until is out-of-subset (identical skip warning, D-P4-9). |
| `src/lib.rs`, `src/main.rs`, `src/bin/generate_code_tests.rs`, `src/bin/test_generated_code.rs` | changed | Module declarations; CLI wiring for the new MC flags. |

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `DESIGN.md` | changed | Sections 5.10/6/7/8.3/9.7 updated; section 11 REWRITTEN around the exact engine (architecture, statuses/summary line, bounded/truncated, liveness formalization, SMT mode, flags/parallelism); section 12 subset table; NEW section 22 "Phase 4 model-checking decisions (as shipped)" D-P4-1..D-P4-14 incl. plan/task corrections. |
| `KB.md` | changed | State table (Phase 4 row, final counts), property operators + weak-until/precedence notes, MC codegen flags, parser-diagnostic corner case, superseded note on the Phase 1 MC section. |
| `BUILD.md` | changed | Suite table to final Phase 4 state; SMT feature notes; baseline line actualized in task 14 (see found problems). |
| `CODEGEN.md` | changed | File map (codegen_mc exact concepts, new mc_plan/smt rows), Generate fields + MC flag defaults/parse errors, section 5.3 rewritten, seedless-exact parity description. |
| `src/backends/RUST.md`, `src/backends/C.md` | changed | MC sections rewritten for the exact engine (SMT standalone branch, embedded plan tables, lasso counterexamples, pthreads/atomics, gcc -Wall bar, libz3 link). |
| `START.md` | changed | Section 1 snapshot (Phase 4 complete) + final baseline numbers; section 3 source tree (+mc_plan/monitor/smt rows). |
| `TOOLCHAIN.md` | changed | New section 5: z3 5.3.1 project-local install (`/workspace/host/z3`), activation env vars, reinstall recipe (plan revision before task 1). |

## Other file changes

| Name | Operation | Description |
|---|---|---|
| `examples/valid/43_sliding_window.rfs` | new | Benchmark: sliding-window ARQ - liveness + input-abstraction showcase; pass-complete at 165 states (task 6). |
| `examples/valid/44_token_ring.rfs` | new | Benchmark: 4-node token ring, bit<2> wrap, abstracted bit<16> Request payload; 129,985 states; exposed + fixed the C arithmetic masking gap (task 8). |
| `examples/valid/45_elevator.rfs` | new | Benchmark: 4-floor elevator, non-vacuous startup liveness, abstracted user_id; 11,098 states (task 8). |
| `training/model_checking_properties/rfs_training.jsonl` + snippets | changed | Task 12: exact-MC statuses/summary line, input abstraction, benchmark-sizing entries (+3 new valid snippets). |
| `training/properties_temporal_logic/rfs_training.jsonl` + snippets | changed | Task 12: eventually/until + liveness counterexamples (+2 valid, +2 invalid snippets); task 13 re-quoted the parens-less-`eventually` entry with the dedicated diagnostic. |
| `all_rfs_training.jsonl` | changed | Regenerated aggregate: 231 examples (130 valid / 127 invalid snippets). |
| `Cargo.toml`, `Cargo.lock` | changed | Optional `smt` feature + z3 dependency (z3 crate 0.21 / z3-sys 0.13.1) - default builds unaffected. |
| `docs/roadmap/*` | new/changed | PHASE_4_PLAN.md (+ revision: explicit z3 install step), task files {1..13} + 10B, per-task STATUS files, this file. |

## Changed functionality

- Model-checking default mode: seeded random fuzz -> EXACT reachable-state BFS in both backends (fuzz retained behind `--mode fuzz`).
  - Configurations now include monitor states; visited-at-enqueue; parent pointers for counterexample reconstruction.
  - Canonical successor order (events/fields in declaration order, values ascending) - deterministic and backend-independent.
- Status/reporting: old per-run taxonomy (`completed`/`stuck`/...) -> three-status `pass-complete` / `fail` (reason: property-violation | liveness-violation | deadlock) / `unknown-state-limit`, exit codes 0/1/2, pinned machine summary line + statistics block.
- C backend arithmetic: `+,-,*` on `bit<N>` now mask to the RFS bit width after the cast (previously wrapped at the C type width - a pre-existing gap exposed by benchmark 44).
- Parse diagnostics: parens-less `eventually` reports a dedicated message at the keyword position instead of generic "trailing input" (task 13, user-requested).

## Removed functionality

- Seeded fuzz as the DEFAULT `mc` mode (retained behind `--mode fuzz`; its wire format and LCG are unchanged for compatibility).

## New functionality

- Exact reachable-state engine in BOTH backends from one shared codegen-time plan (`src/mc_plan.rs`, embedded as constants/tables):
  - Input abstraction: unused payload fields collapse to a representative value (soundness argument in DESIGN.md section 11).
  - Deadlock = FAIL when a reachable configuration has no successor; unexpected inputs terminate their path and are counted without changing the status.
  - Active assumptions prune exploration (`--assumes none` explores everything).
- Liveness operators `eventually (p)` and infix WEAK `until` in the property language (parser/IR/printer/validator):
  - Scope: top-level obligations over boolean subformulas; deeper nesting -> codegen warning + skip (D-P4-9).
  - Engine: per-leaf progress automata + product limit-vector analysis on the completed graph; lasso counterexamples (prefix + one cycle iteration); truncated-mode soundness (never a false pass-complete); `--nondet` strong transition fairness.
  - Any formula containing them is outside the online monitor subset (identical skip warning, MC-only).
- Level-parallel exact BFS: `--threads N` two-phase per-level expansion (disjoint frontier ranges + sequential canonical commit) - byte-identical to `--threads 1` for any N in both backends; runtime-only non-parity `--fast` early abort.
- Optional SMT bounded mode (`--mode smt-bounded`, feature-gated `smt`): k-step unrolling over bitvectors from the shared plan/emitter, k-liveness, exit codes 0/1/2, PASS always labeled incomplete; shared v1-scope rejections (assumptions, on_error blocks, enum-typed vars, complex property formulas).
- Benchmark specs 43/44/45 + `tests/benchmark_mc_test.rs` parity suite (metrics, FAIL variants, threads identity, JSON report).

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|
| `tests/liveness_mc_test.rs` | new | 9 tests: fixpoint-analysis checks vs brute force, lasso structure + pinned fields, truncated-mode soundness, `--nondet` fairness discriminator (task 6/7). |
| `tests/benchmark_mc_test.rs` | new | 5 tests: pass-complete metric parity in both backends on specs 43/44/45, inline FAIL variants with counterexample parity, `--threads 1/2/4` byte-identity, `--fast` smoke, JSON report (tasks 9/10B). |
| `tests/smt_bounded_test.rs` | new | 2 tests (feature-gated): byte-identical stdout + exit codes across backends on wide-bitvector specs (task 11). |
| `tests/backend_parity_test.rs` | changed | Migrated from same-seed fuzz to seedless exact mode; re-pinned values incl. 8 NEW deadlock findings + 1 new property violation the fuzz walk never reported; now 7 tests (task 4). |
| `tests/monitor_integration_test.rs` | changed | Exact-mode migration with re-pinned counterexample fields (task 4). |
| parser unit tests (`src/parser.rs`) | added | Until-precedence pins (`!a until b`, `a && b until c`), liveness round-trips, malformed shapes, dedicated parens-less-`eventually` diagnostic (tasks 5/13). |
| lib unit tests | added | `mc_plan` abstraction/encoding/traversal specs, `codegen_mc` status wire/human/exit + summary-line format pins, `smt_plan` v1-scope rejections, monitor-subset boundary for liveness operators. |

## Tests run status

| Name | Status |
|---|---|
| `cargo build` | OK |
| `cargo test` | OK - **230/230** (125 lib, 7 parity, 5 benchmark, 26 codegen, 3 examples, 41 frontend, 4 parser-library comparison, 9 liveness, 4 monitor integration, 4 training) |
| `cargo test --features smt` (z3 env per TOOLCHAIN.md 5.2) | OK - **232/232** (+2 SMT parity tests) |
| `cargo clippy --all-targets` | OK - 0 errors, 12 pre-existing warnings (baseline: 11 lib + 1 codegen_test) |
| `cargo fmt --check` | OK - clean |
| `cargo generate-code-tests && cargo test-generated-code` | OK - 179 modules, **537/537** |
| `cargo run -- validate --input rfs examples/valid` | OK - exit 0 (45 valid / 12 invalid) |
| Training chain (`extract-training-snippets && build-training && test-training`) | OK - 130 valid / 127 invalid snippets, aggregate 231 lines, 5/5 tests |
| `gcc -Wall` spot check of new C MC output (example 43: exact engine + liveness) | OK - zero warnings; `PASS (complete)` with pinned metrics 165/330/22; `--threads 4` run byte-identical summary line, exit 0 |

## Found problems/issues/mismatches

- **Stale BUILD.md section 5 baseline line** (missed by the task 13 sweep because it is dated as history): still said "Known-good baseline (2026-10-17, after Phase 3): 170/170 main tests, 513/513 generated tests" + "same-seed results" wording. Actualized in task 14 to the final Phase 4 numbers.
- **Generated-code test count was recorded stale**: tasks 10B/12/13 reported 522 against a STALE aggregate crate - the aggregate crate also compiles the valid training snippets, and task 12 added +5 valid snippets without regenerating it. Task 14's regeneration gives the true final **537** tests in 179 modules; BUILD.md/START.md/KB.md actualized here.
- **`cargo test-training` matches 5 tests, not 4**: the alias is `test training`, which also selects the codegen aggregate test whose name contains "training". BUILD.md recorded 4 - actualized with a note.
- No functional bugs: every suite was green on first run in this task (after the doc fixes above).

## Other important findings/information

- Final baseline for Phase 5 planning: **230/230** main tests (232 with `--features smt`), **537/537** generated-code tests (179 modules), 45 valid / 12 invalid examples, snippets 130 valid / 127 invalid, aggregate 231 lines.
- z3 5.3.1 lives at `/workspace/host/z3` (TOOLCHAIN.md section 5); the `smt` cargo feature is optional - default builds and CI-free workflows are unaffected.
- Benchmark sizing record: spec 43 = 165 states, 44 = 129,985 states (~1 s single-threaded), 45 = 11,098 states; metrics pinned in `tests/benchmark_mc_test.rs`; `target/mc_benchmarks.json` is a development artifact (wall-clock informational only).
- Plan corrections of record (all in DESIGN.md section 22): D-P4-10 weak-until equation fix (verified against brute force), 10a/10B `--threads` deferral + revival with the canonical-selection design, task 5 until-precedence correction, task 10 `--timeout-sec` help-location deviation.
- The C arithmetic masking fix (task 8) was a PRE-EXISTING backend gap, not a Phase 4 regression: RFS `bit<N>` wrap semantics were only enforced by the Rust backend's u128+mask strategy until then; the token ring benchmark's holder 3->0 wrap exposed it and it is now pinned by parity.
