# Phase 5, Task 7 - Implementation status

> Written once when all exit criteria of `docs/roadmap/PHASE_5_TASK_7.md` are
> reached. Not rewritten afterwards; later fixes go into TASKS.md entries and
> follow-up phase plans.

## Added source files

| Name | Description |
|---|---|
| `src/test_cmd.rs` | The `rfs test` command logic: option struct, the parse+validate -> codegen (registry dispatch) -> compile (task 3's resolver: flag > env > PATH) -> run pipeline in a temp dir with a Drop-cleanup guard, and the self-contained `--print-script` renderer (heredoc source + compile + run + cleanup steps via task 3's `Script` emitter). |

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| `src/main.rs` | changed | `test` subcommand dispatch + usage text (line + paragraph) + `cmd_test` arg parsing (`--backend`, `--steps`, `--rustc/--gcc PATH`, `--*-args`, `--print-script`, `--log-level`). |
| `src/lib.rs` | changed | `pub mod test_cmd;`. |

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `BUILD.md` | changed | Counts re-pinned (root 337 -> 342, workspace 345 -> 350, frontend 77 -> 82 with the task 7 suite), new command-table row for `rfs test`, known-good baseline updated (2026-10-22, after Phase 5 task 7; 344 with smt). |
| `KB.md` | changed | Section 4.5: stale "Three subcommands" intro corrected to the six-command set + new `test` subsection (pipeline, resolver precedence, --print-script semantics, exit codes); section 1 Tests row re-pinned (342 / 82 frontend). |
| `docs/roadmap/INTRO.md` | changed | Phase-state row 5 -> "tasks 1-7 + 8-9 of 16 done". |

## Other file changes

- `TASKS.md`: one new entry (2026-10-22).

## Changed functionality

- None to existing commands. The new subcommand is purely additive; `rfs codegen <backend> test` output is untouched (the command reuses it via the registry dispatch).

## Removed functionality

- None.

## New functionality

- `rfs test <spec.rfs|spec.json>`: generate + compile + run the extended test
  suite in ONE invocation (the CI entry point for generated tests):
  - backends: `--backend rust|c` (default rust); rust compiles with
    `rustc --edition 2021 --test`, c with `gcc -Wall`;
  - external-tool parameterization per task 3's resolver: `RFS_RUSTC`/`RFS_GCC`
    (+ `*_ARGS`) env vars preferred, overridden by `--rustc/--gcc PATH` +
    `--*-args` flags (env args first, CLI args appended), default bare name on
    PATH;
  - temp-dir workspace cleaned on every exit path (Drop guard); per-test
    results stream on stdout (libtest lines / PASS-FAIL lines), tool
    resolution + compile diagnostics on stderr via the logging helper;
  - `--print-script`: self-contained deterministic POSIX sh script (heredoc
    source, compile, run, cleanup) printed to stdout, exit 0, NO execution;
  - exit codes: 0 all tests pass; 1 test failure / compile error / usage or
    parse or validation error (consistent with the other commands).

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|
| `e2e_test_passing_spec_both_backends_exit_zero` | new | real binary, both backends on a passing spec: exit 0 + per-test results visible (libtest lines / PASS lines). |
| `e2e_test_failing_variant_exits_one` | new | injected invariant failure (`always (x)` with x init false): exit 1 in BOTH backends with the failing test name visible. |
| `e2e_test_print_script_pinned_and_no_execution` | new | wrapper byte-pinned (header/tools/steps); heredoc body asserted equal to the actual `rfs codegen rust test --out` output; no-execution proven via a nonexistent RFS_RUSTC path (would fail if executed). |
| `e2e_test_tool_precedence_stub_tools` | new | stub tools echoing argv: flag > env, env > PATH (stub named `rustc` on a replaced child PATH), extra args reach the compiler in env-then-CLI order; gcc pair proven too. |
| `e2e_test_usage_and_parse_errors` | new | no spec / unknown option / bad --steps / unknown backend / unparseable / validation error / wrong extension - all exit 1 with pinned diagnostics. |

## Tests run status

| Name | Status |
|---|---|
| `cargo test` (root package) | OK (**342/342**; frontend 77 -> 82; was 337) |
| `cargo test --workspace` | OK (**350/350**; was 345) |
| `cargo test --features smt` | OK (**344/344**) |
| `cargo test-generated-code` | OK (1453/1453 unchanged - no renderer changes) |
| `cargo clippy --all-targets` | OK (identical to the 12-warning baseline, diff-verified) |
| `cargo fmt --check` | OK (clean) |

## Found problems/issues/mismatches

- None new. (The pre-existing SMT scratch-project workspace bug was fixed in
  task 6; it stays green here.)

## Other important findings/information

- The Rust test artifact MUST be compiled with `--test` (correction C7-1): it
  is production code + a `#[cfg(test)]` module with no `main`.
- `rfs test --print-script <spec> | sh` was verified end-to-end by hand: the
  script is fully self-contained (writes source, compiles, runs, cleans up)
  and leaves no artifacts.
- For task 14 (training data): the new user-facing surface to cover is the
  `rfs test` command line + options, the RFS_RUSTC/RFS_GCC(+*_ARGS) env vars
  and overriding flags, `--print-script`, and the exit-code contract.

## Corrections during implementation

- C7-1 (task file deviation): rustc invocation includes `--test` (see Task
  corrections in PHASE_5_TASK_7.md).
- C7-2 (pinning strategy): script wrapper byte-pinned + heredoc body
  cross-checked against codegen output instead of a 218-line literal.
