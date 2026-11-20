# Phase 17, Task 8 - Implementation status

> Written once when all exit criteria of `PHASE_17_TASK_8.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

| Name | Description |
|---|---|
| `crates/examples/mc_walkthrough/pass.rfs` | the passing spec (copy of examples/valid/03_timeout_retry.rfs) |
| `crates/examples/mc_walkthrough/fail.rfs` | the failing variant: same machine + the unsatisfiable `ack_bounded` obligation |
| `crates/examples/mc_walkthrough/run.sh` | generate + compile MC binaries for BOTH backends on both specs, run exact mode, capture text + JSON reports to captures/ (build/ is scratch, removed on exit) |
| `crates/examples/json_schema_ci/pass.rfs` | valid spec - source of the spec-json + info-json documents |
| `crates/examples/json_schema_ci/fail.rfs` | failing variant - source of the mc-report + ce-trace documents |
| `crates/examples/json_schema_ci/invalid_always_zero_bound.rfs` | invalid spec - source of the validate-json document |
| `crates/examples/json_schema_ci/validate.sh` | emit all five schemas, generate one real document per format, validate each with python3 -m jsonschema (availability-gated) |
| `crates/examples/specgen_mutation_loop/loop.sh` | generate base (seed 17), prove byte-determinism, validate, mc-ref (PASS), mutate (K=8 seed 1), validate, mc-ref (FAIL) - writing the transcript to work/transcript.txt |
| `crates/examples/tool_resolution/pass.rfs` | the spec argument for the rfs test / codegen invocations |
| `crates/examples/tool_resolution/run.sh` | set explicit RFS_* paths, run rfs test --print-script (C + Rust), capture the # tools: lines; prove flag-beats-env + bare-PATH fallback |
| `crates/examples/tool_resolution/Makefile` | CC ?= gcc with a documented make CC=clang alternative, building the generated C FSM warning-free under either compiler |

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| `tests/example_projects_test.rs` | changed | + `run_script` + `project_local_rustc_exists` helpers + four plain-dir test legs (one per example) |

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `crates/examples/mc_walkthrough/README.md` | new | template README: end-to-end report walkthrough with REAL captures - every PASS summary field explained (incl. why unexpected-terminations is not a failure), the FAIL property-violation header + CE trace in text AND json, cross-backend byte-identity |
| `crates/examples/json_schema_ci/README.md` | new | template README: what self-describing formats are for, the five-formats table (document + producing command), real VALID output, CI-wiring walkthrough |
| `crates/examples/specgen_mutation_loop/README.md` | new | template README: the pinned REAL seed-17 transcript, what each step proves (determinism / always-valid / instant checking / mutation-as-corner-case), when to use the loop |
| `crates/examples/tool_resolution/README.md` | new | template README: the RFS_* precedence contract, --print-script as the proof artifact, REAL # tools: lines proving explicit-path-wins / flag-beats-env / bare-PATH, the gcc/clang Makefile choice |
| `docs/llm/TEST_STRUCTURE.md` | changed | +6 rows for the new test fns (same-commit rule); Example-projects-suite row + example_projects_test file row extended (16 -> 22 fns); 1402/1430 |
| `docs/dev/workspace_crates.md` | changed | Examples opening: three -> seven plain-dir examples (the four task-8 script walkthroughs named); note that plain-dir examples carry no tracked .rs files so have no inventory rows |

## Changed functionality
None - example projects + tests + docs only.

## Removed functionality
None.

## New functionality
- Four script-driven plain-dir example projects covering the remaining usage scenarios: end-to-end MC report reading (both backends, PASS + FAIL, text + JSON), JSON-schema CI validation of all five self-describing formats, the gen-spec mutation prototyping loop (pinned seed-deterministic transcript), and explicit external-tool path passing (# tools: precedence proof + gcc/clang Makefile choice).

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|
| `example_projects_test::run_script` | new | run a shell script in a dir via sh, returning (exit code, combined stdout+stderr) |
| `example_projects_test::project_local_rustc_exists` | new | true when the project-local rustup toolchain provides a rustc (cold-checkout gating) |
| `example_projects_test::mc_walkthrough_generates_both_backends_and_pins_reports` | new | run.sh: pinned PASS summary (states=8 transitions=12 max_depth=5) + FAIL (label=ack_bounded step=2, trigger 1, violation 2) + JSON ce document; gated on gcc + rustc |
| `example_projects_test::json_schema_ci_validates_all_five_formats` | new | validate.sh: all five formats report VALID; skips with a note when python3 jsonschema is absent |
| `example_projects_test::specgen_mutation_loop_pins_the_real_transcript` | new | loop.sh: pinned determinism line + base PASS (states=4 transitions=12) + mutation FAIL at label=p2 step=0 (initial configuration); no external tools |
| `example_projects_test::tool_resolution_proves_precedence_and_builds_under_both_compilers` | new | run.sh pins # tools: gcc=/usr/bin/gcc (explicit wins), gcc=/bin/sh-fake (flag beats env), gcc=gcc (bare PATH), a rustc line; Makefile builds the generated C FSM under gcc and, availability-gated, clang |

## Tests run status

| Name | Status |
|---|---|
| `cargo test --test example_projects_test` (13 = 9 baseline + 4 new) | OK |
| `cargo test --test dev_docs_test` (9) | OK |
| `cargo test --test docs_md_test` (5) | OK (all four new READMEs' mermaid + links render/resolve) |
| `rfs-dev inventory audit` | EXACT: SRC 1013 fns / 213 types, TEST 1054 fns / 35 types - zero per-file diffs |

## Found problems/issues/mismatches
- The inventory audit walks crates/ for .rs files, so mc_walkthrough's generated Rust MC/FSM artifacts (written to build/) were flagged as un-inventoried. Fixed by having run.sh remove its scratch build/ dir on exit (trap) - the durable output is captures/. A stray fsm.rs from an early probe was also removed.
- Two READMEs prematurely linked to docs/user/USAGE.md, which does not exist until task 10 - docs_md_test's dead-link check caught it. Relinked to existing docs (cli_commands.md / TOOLCHAIN.md).
- `rfs mc-ref --counterexample-format json` and `rfs validate --json` both EXIT 1 on their "failing" inputs yet emit a valid single JSON document - the scripts capture stdout regardless of exit code (a failing run still yields a schema-conforming document).

## Other important findings/information
- The Rust MC's include!("<stem>-fsm.rs") resolves relative to the mc file's own directory, so an --out with a subdirectory prefix doubles the path. The scripts generate into per-tag subdirs under a BARE stem to keep the include resolvable.
- The resolver records an explicit RFS_* path in the # tools: line even if that file does not exist (print-script mode does not verify existence) - so the explicit-path pins are deterministic regardless of host layout.
- All pinned values were captured from REAL runs before writing the READMEs/test legs: the seed-17 base is PASS (states=4 transitions=12 max_depth=1), and the K=8/seed-1 mutation FAILS at label=p2 step=0 (initial configuration).
