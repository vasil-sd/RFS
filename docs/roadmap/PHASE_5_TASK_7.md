# Phase 5, Task 7 - Implementation details

> Written for a future LLM session: after reading `START.md`,
> `docs/roadmap/INTRO.md` and this file, the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal
Add the `rfs test` command: generate + compile + run the extended test suite
for a spec in ONE invocation, for either backend - with external-tool
parameterization (env-var-preferred) and `--print-script` mode per the user
requirement. This is the CI entry point ("CI can run model checking and
generated tests").

## Context

- Pipeline: parse+validate spec -> codegen `test` artifact (via the existing
  registry dispatch, in a temp dir) -> compile with the resolved tool
  (`rustc --edition 2021 <artifact>.rs -o <tmp>/spec_test` or
  `gcc -Wall <artifact>.c -o <tmp>/spec_test`) -> run the executable -> report.
- External-tool parameterization (task 3's resolver, user requirement):
  env vars `RFS_RUSTC`/`RFS_GCC` (+ `RFS_RUSTC_ARGS`/`RFS_GCC_ARGS`) preferred
  mechanism; CLI flags `--rustc PATH`, `--gcc PATH`, `--rustc-args "..."`,
  `--gcc-args "..."` override the env vars; default = bare name on PATH.
- `--print-script`: instead of executing, print the deterministic POSIX sh
  script (task 2's emitter): header comment (rfs version, spec path, resolved
  tools + args), compile step, run step, artifact cleanup note - to stdout,
  exit 0. Byte-pinned by a test.
- Options: `--backend rust|c` (default rust), `--steps N` (passed to codegen,
  default 4 as today), `--log-level`. Exit codes: 0 all tests pass; 1 test
  failure or compile error; 2 usage/parse/validation error (keep consistent
  with the other commands' 0/1 where possible - decide and pin in a test:
  parse/validation errors are 1, matching `validate`).
- Diagnostics: per-test results on stdout (the executable already prints them),
  tool resolution + compile diagnostics on stderr via the logging helper.

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 5. | Rust test renderer + libfuzzer_sys skeleton | the Rust artifact this command compiles/runs |
| 6. | C test renderer + libFuzzer skeleton | the C artifact for `--backend c` |

### Dependent tasks

(none directly; tasks 14/15 document and train it)

### Affected source files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `src/main.rs` | change | dispatch for the new `test` subcommand + usage text |
| `src/test_cmd.rs` | new | command logic: option parsing, pipeline, tool resolution, script mode |

### Affected test files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `tests/frontend_test.rs` | change | end-to-end: both backends on a passing spec (exit 0) and an injected-failing variant (exit 1); --print-script pinning; env-var/flag parameterization with stub tools; usage/parse errors |

## Planned actions

### Action 1: command parsing + dispatch
`rfs test <spec.rfs|spec.json> [--backend rust|c] [--steps N] [--rustc PATH]
[--gcc PATH] [--rustc-args "..."] [--gcc-args "..."] [--print-script]
[--log-level L]`. Extension selects input format (same rule as codegen).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/test_cmd.rs` | new | parsing + validation |
| `src/main.rs` | change | dispatch + usage text |

### Action 2: execution pipeline
Temp-dir workspace (std::env::temp_dir + unique subdir, cleaned on exit):
codegen -> resolved compile (capture stderr; non-zero exit -> report + exit 1)
-> run executable (stream stdout; non-zero -> exit 1). Tool resolution via
task 3's resolver with the flag/env/PATH precedence.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/test_cmd.rs` | change | pipeline implementation |

### Action 3: --print-script mode
Emit via task 3's emitter (compile step with resolved tool + args, run step);
no execution, exit 0. Pin the output byte-for-byte in a test (stub paths).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/test_cmd.rs` | change | script mode branch |

### Action 4: tests
End-to-end both backends on `examples/valid/01_*.rfs` (exit 0, per-test lines
present) and on a spec variant with a deliberately failing assertion (exit 1);
`--print-script` pinning; RFS_RUSTC env + `--rustc` flag proven via stub tools
that echo their argv.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `tests/frontend_test.rs` | change | the tests above |

## Task deliverables
- `rfs test` command (both backends, tool parameterization, --print-script).
- End-to-end frontend tests incl. stub-tool parameterization proof.

## Task exit criteria
- [ ] `rfs test <spec>` exits 0 on a passing spec in BOTH backends, streaming per-test results.
- [ ] A failing variant exits 1 with the failing test visible.
- [ ] Tool precedence (flag > env > PATH) proven end-to-end with stub tools; extra args reach the compiler (stub echoes argv).
- [ ] `--print-script` output byte-pinned; no execution happens in that mode.
- [ ] Usage/parse errors handled consistently; full suite green, clippy/fmt clean.

## Non-goals
- Running the MODEL CHECKER here (MC stays a separate generated binary / its
  own invocation; "CI can run model checking" is satisfied by `rfs codegen
  ... mc` + the generated binary - document in task 14).
- Fuzz target execution (cargo-fuzz/clang wiring is task 12).
- Parallel test execution across backends (sequential is fine for v1).

## Other information
- Temp dir cleanup must happen on failure paths too (use a guard/drop).
- Keep the command thin: no logic duplicated from codegen/renderers.
