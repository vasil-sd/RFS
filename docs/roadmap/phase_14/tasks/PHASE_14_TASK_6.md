# Phase 14, Task 6 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal
Make every help surface reference ONLY the `rfs` binary - no cargo/rustc
invocation examples anywhere - and bring the codegen help texts up to date with
the new flag family, the spec-stem default naming policy and the `--out -`
stdout marker. Add a regression test that pins "no `cargo run` in any help
output".

## Context

- The ONLY in-source occurrence of the `cargo run --` pattern is the global
  `usage()` in `src/cli/args.rs`: four example lines at the end of the help
  text (`cargo run -- parse ...`, `cargo run -- validate ...` x2). Every
  per-command `usage()` (parse/validate/info/test/diagram/mc-ref/gen-spec) and
  the codegen help already reference only `rfs` - but the codegen help strings
  describe the OLD `--out FILE` single-file semantics and must be rewritten to
  the new surface (tasks 1-5 have landed by now).
- Help surfaces to audit/rewrite:
  - `src/cli/args.rs` `usage()`: the four example lines + the codegen synopsis
    line(s) (`rfs codegen <backend> <production|test|mc> <spec.rfs|spec.json>
    [--out FILE] ...`).
  - `src/codegen/mod.rs` `common_help()` (the `--out FILE` option line + the
    usage synopsis), `backend_help()` (renders the per-command descriptors),
    and all six `BackendCommandInfo` usage/options strings in
    `src/codegen/backends/{rust,c}/mod.rs` (they document `--out FILE`
    REQUIRED/default-stdout semantics for test/production/mc).
  - Per-command `usage()` functions: verify none teaches a cargo/rustc
    invocation or stale output semantics (the audit found them clean of
    `cargo run`; re-verify the codegen-related wording in `test.rs`'s usage,
    which mentions "the emitted skeleton").
- The rfs binary is usable on machines with NO Rust toolchain (C-only projects)
  - that is the rationale for goal 1: help text must not imply cargo/rustc are
  prerequisites.
- Legitimate cargo references that must SURVIVE the audit: the `rfs test` fuzz
  pipeline's cargo-fuzz machinery (`cargo fuzz build/run`, `RFS_CARGO_FUZZ`,
  the nightly-toolchain notes) - those document a real external dependency of
  one suite, not an invocation pattern for rfs itself.

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 1. | Codegen output model + CLI flag family | The help texts describe the flag family this task landed. |
| 2.-5. | Backend artifact tasks (as completed) | The help must describe the FINAL artifact model (pairs, references), so write the codegen help after they land. |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 7. | Living-documentation sync | Docs mirror the help text as the reference for CLI usage. |
| 10. | Training-set update | Entries teach the same `rfs <command>` pattern this task pins. |

### Affected source files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `src/cli/args.rs` | change | Four `cargo run --` example lines -> `rfs ...`; codegen synopsis line(s) rewritten to the new flag family + default policy. |
| `src/codegen/mod.rs` | change | `common_help()` usage synopsis + option list rewritten (flag family, spec-stem default, `--out -`, take-or-emit). |
| `src/codegen/backends/rust/mod.rs` | change | Three `BackendCommandInfo` usage/options strings updated to the new output options. |
| `src/codegen/backends/c/mod.rs` | change | Three `BackendCommandInfo` usage/options strings updated (header flags, pairs). |
| per-command `usage()` sites in `src/cli/commands/*.rs` | change (only if the audit finds stale wording) | No cargo/rustc invocation examples; no stale `--out FILE` semantics. |

### Affected test files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `tests/codegen_test.rs` | change | New regression test: no help output (global usage + all eight command helps + codegen --list/--help/<backend> --help) contains `cargo run`; existing help-shape tests updated to the new text. |

## Planned actions

### Action 6.1 - Fix the global usage examples + codegen synopsis

In `src/cli/args.rs` `usage()`: replace the four `cargo run -- ...` example
lines with `rfs ...` equivalents (same commands, same arguments); rewrite the
codegen synopsis line(s) to show the new surface (`[output options]` summary -
the full family is documented by `rfs codegen --help`, so the synopsis stays
short: `rfs codegen <backend> <production|test|mc> <spec.rfs|spec.json>
[--out BASE | --out-fsm-code FILE ...] [--fsm-file NAME ...] [options]`).

Rationale: the global help is what `rfs --help` and every error path print - it
is the first thing a user on a cargo-less machine reads.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/cli/args.rs` | change (examples block) | `cargo run --` -> `rfs`. |
| `src/cli/args.rs` | change (codegen synopsis line) | New flag family summary. |

### Action 6.2 - Rewrite the codegen help to the new surface

In `src/codegen/mod.rs`: `common_help()` - new usage synopsis, the output-
options section (per-artifact flags, fsm-reference flag forms incl. C form 1
stem expansion WITHOUT `-fsm` suffix and form 2 pair, the spec-stem default
policy, `--out -`), keep the generation-options section unchanged. In both
backend mod.rs files: update each `BackendCommandInfo` usage line + options
text to the command's actual flag subset (production: fsm flags; mc: mc flags
+ fsm-reference; test: test/fuzz-target flags + fsm-reference; C adds the
header flags). Keep the per-command runtime-flag documentation of the generated
binaries (that text is about the BINARY's CLI, not rfs - it stays).

Rationale: `rfs codegen --help` / `rfs codegen <backend> --help` are the
authoritative reference for the new CLI - they must match the 23-row contract
table in PHASE_14.md exactly.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/codegen/mod.rs` | change (`common_help`) | New surface documentation. |
| `src/codegen/backends/rust/mod.rs` | change (three descriptors) | Per-command flag subsets. |
| `src/codegen/backends/c/mod.rs` | change (three descriptors) | Per-command flag subsets + header flags. |

### Action 6.3 - Audit per-command helps + add the regression test

Walk every per-command `usage()` (parse/validate/info/test/diagram/mc-ref/
gen-spec): confirm zero cargo/rustc invocation examples (the cargo-fuzz
machinery notes in test.rs are legitimate and stay) and no stale output
semantics; fix anything found. Add the regression test in
tests/codegen_test.rs: invoke every help surface through the parsers
(`parse_codegen_args` Help/BackendHelp requests + each command's run_cli with
`--help`) and assert none of the captured output contains `cargo run`.

Rationale: goal 1 is a standing invariant, not a one-time cleanup - the test
keeps it that way.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/cli/commands/*.rs` usage() sites | change (only if audit finds issues) | Stale wording fixes. |
| `tests/codegen_test.rs` | change (new regression test) | No-`cargo run`-in-help assertion over all surfaces. |

## Task deliverables
- Global + per-command + codegen help texts referencing only the `rfs`
binary, documenting the final CLI surface.
- The no-`cargo run`-in-help regression test.

## Task exit criteria
- [ ] `rfs --help`, every `rfs <command> --help`, `rfs codegen --list`,
      `rfs codegen --help` and `rfs codegen {rust,c} --help` contain zero
      `cargo run` occurrences (verified by the new test).
- [ ] The codegen help documents: the flag family, C header flags, fsm-
      reference forms (incl. stem expansion without `-fsm` suffix), the spec-
      stem default policy, take-or-emit, and `--out -`.
- [ ] Legitimate cargo-fuzz references in the `rfs test` help survive unchanged.
- [ ] `cargo test` (root) green; fmt clean; clippy at baseline.

## Non-goals
- No documentation-file changes (task 7) - source help text only.
- No training-set changes (task 10).
- No CLI behaviour changes - this task rewrites TEXT, not parsing.

## Task corrections

| Correction | Description + rationale |
|------------|-------------------------|
| Action 6.3 regression-test mechanism: binary spawn instead of parser-level invocation, with a two-tier exit-code assertion | The action planned invoking the help surfaces "through the parsers (`parse_codegen_args` Help/BackendHelp requests + each command's run_cli with `--help`)". Implemented by spawning the REAL binary for 13 surfaces instead - a strict superset: it also covers main.rs's dispatch paths (bare `rfs` and `rfs --help`, which are NOT command-help invocations - a non-command first arg is an unknown subcommand printing usage to stderr with exit 1, no args prints usage with exit 1). Those two pre-existing exit-1 behaviours are outside this task's text-only scope, so the test asserts exit 0 only for the true help invocations and checks BOTH stdout and stderr for `cargo run` on every surface. Rationale: the exit criterion names `rfs --help` as a surface; only the binary spawn reaches the text it actually prints (stderr), and parser-level invocation would have missed main.rs entirely. |

## Other information

- When rewriting the BackendCommandInfo options strings, keep the existing
  line-wrapping style (the help renderer indents continuation lines) so the
  output stays readable at 80 columns.
