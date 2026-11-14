# Phase 14, Task 6 - Implementation status

> Written once when all exit criteria of `PHASE_14_TASK_6.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

| Name | Description |
|---|---|

(none - the task rewrites existing help strings and adds one test.)

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| `src/cli/args.rs` | changed | The four `cargo run -- ...` example lines in the global `usage()` became `rfs ...` (same commands, same arguments); the codegen synopsis line now shows the new surface (`[--out BASE \| --out-fsm-code FILE ...] [--fsm-file NAME ...] [options]`); the codegen description paragraph rewritten from the old "writes generated source to --out or stdout / requires --out" wording to the final policy (BASE := --out value or spec-file stem, artifacts in CWD, per-artifact overrides, take-or-emit for mc/test, `--out -` rust-production-only). |
| `src/codegen/mod.rs` | changed | `common_help()`: usage synopsis now `[output options] [options]`; the old `--out FILE (default: stdout)` option line replaced by a full output-options section (the six per-artifact flags with their command/backend subsets, `--out BASE` + spec-stem default policy, `--out -`) and an fsm-reference-flags section (Rust `--fsm-file NAME[.rs]`, C form-1 STEM expansion to `STEM.h`+`STEM.c` WITHOUT the `-fsm` suffix, C form-2 pair), plus a default-names paragraph covering all three commands x both backends and the D14-4 stale-hash warning. The generation-options section is byte-unchanged. |
| `src/codegen/backends/rust/mod.rs` | changed | All three `BackendCommandInfo` descriptors rewritten to the final surface: production (`--out BASE \| --out-fsm-code FILE`, writes `BASE-fsm.rs`, `--out -` prints to stdout), test (three files, `--fsm-file NAME[.rs]` take-or-emit, `--out -` refused), mc (two files, include! reference, `--out -` refused). The per-command generation options and the generated-binary runtime-flag documentation are unchanged. |
| `src/codegen/backends/c/mod.rs` | changed | All three descriptors rewritten: production (header/source pair `BASE-fsm.h`+`BASE-fsm.c`, explicit-pair flags, `--out -` refused), test (four files, both .c include the fsm header never a .c, form-1/form-2 fsm reference, partial pair = usage error), mc (four files, mc source includes its own header + the fsm header). Generation options and runtime-flag documentation unchanged. |

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `docs/roadmap/CURRENT_STATE.md` | changed | Phase-14 row: tasks 1-6 complete, tasks 7-11 pending. |

(User-facing / dev architecture docs are task 7/8's scope - the phase plan defers them.)

## Other file changes

| Name | Operation | Description |
|---|---|---|

(none.)

## Changed functionality
- Help text only - no CLI behaviour changed (parsing, exit codes and artifact
  writing are untouched):
  - `rfs --help` / bare `rfs` / every per-command `--help`: the four example
    lines now show `rfs parse ...` / `rfs validate ...` instead of
    `cargo run -- ...`; the codegen synopsis and description paragraph describe
    the flag family, the spec-stem default naming and `--out -` instead of the
    retired `--out FILE` (default stdout) semantics.
  - `rfs codegen --help`: documents the output-options section (per-artifact
    flags, BASE policy, stdout marker) and the fsm-reference flag forms
    (incl. C stem expansion without the `-fsm` suffix and the form-2 pair)
    instead of a single `--out FILE` line.
  - `rfs codegen {rust,c} --help`: each command's usage line + options text now
    state its actual flag subset, the files it writes (1/2/3/4 per command and
    backend), the take-or-emit semantics for the referenced fsm artifact and
    where `--out -` is refused.

## Removed functionality
- The four `cargo run -- ...` example lines from the global usage text (the
  only in-source occurrence of the pattern).
- The old `--out FILE` single-file help wording everywhere it appeared
  (`common_help`, all six descriptors, the global synopsis + description).

## New functionality
- A standing regression test that the rfs binary is self-describing without a
  Rust toolchain: every help surface (global usage via bare `rfs` and
  `rfs --help`, all eight command helps, `codegen --list/--help/<backend>
  --help`) is spawned and asserted cargo-run-free on BOTH stdout and stderr.
- Help text that matches the 23-row CLI contract table in PHASE_14.md: the
  flag family, C header flags, fsm-reference forms, spec-stem default policy,
  take-or-emit (incl. the partial-pair usage error) and `--out -` are all
  documented where users look for them.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|
| `tests/codegen_test.rs::no_help_output_mentions_cargo_run` | new | Spawns the real binary for 13 help surfaces (bare `rfs`, `rfs --help`, the eight command helps, `codegen --list/--help/rust --help/c --help`) and asserts none of stdout/stderr contains `cargo run`; exit-0 asserted only for true help invocations (bare/`--help` are unknown-subcommand/no-arg paths that exit 1 by pre-existing design). |

## Tests run status

| Name | Status |
|---|---|
| `cargo test` (root, all 32 suites) | OK - 548 passed / 0 failed (547 pre-task + the new regression test) |
| `cargo fmt --check` | OK |
| `cargo clippy --all-targets` | OK - at baseline: the 39 unique warning sites are IDENTICAL to git HEAD (same files, same warnings; only line numbers shifted by this task's insertions) |

## Found problems/issues/mismatches
- The first version of the regression test asserted exit 0 for `rfs --help` and failed: main.rs treats a non-command first arg as an unknown subcommand (usage on stderr, exit 1) and bare `rfs` prints usage with exit 1 - both pre-existing behaviours outside this task's "text only" scope. The test now checks both streams for every surface and asserts exit 0 only for the true help invocations.
- A `{h,c}` brace pair in the new `common_help` default-names text was parsed as a format placeholder by `writeln!` (compile error); rewritten as explicit `.h + .c` names instead of escaping braces.

## Other important findings/information
- The per-command audit found NO stale wording beyond what the task file predicted: parse/validate/info/test/diagram all print the global usage; gen-spec and mc-ref carry their own clean `rfs`-only usage(); the test description's "emitted skeleton" + cargo-fuzz machinery references are accurate and legitimate (they document a real external dependency of the fuzz suite, not an rfs invocation pattern) and survived unchanged.
- The help renderer indents descriptor option continuation lines by 14 spaces (`backend_help`), so the new options strings keep the existing two-space-first-column wrapping style to stay readable at 80 columns.
