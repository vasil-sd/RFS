# Phase 5, Task 17 - Implementation details

> Written for a future LLM session: after reading `START.md`,
> `docs/roadmap/INTRO.md` and this file, the assistant must be able to start
> implementation of the task immediately without re-deriving scope.
>
> Added by user request (2026-10-22) as task No 17 on top of the planned 16;
> recorded in `PHASE_5_PLAN.md` plan corrections (C4).

## Goal
Extend the `rfs test` command (task 7) so it can ALSO build and run the
libFuzzer fuzz targets that tasks 5/6 already generate as extra files - for
BOTH backends - with explicit suite selection (unit / fuzz), full env-var
parameterization aligned with task 3's ext_tool conventions, and
`--print-script` coverage. Install clang + the libFuzzer runtime (+ cargo-fuzz
for the Rust side) in this environment and document them in TOOLCHAIN.md /
BUILD.md.

## Context

- The fuzz skeletons already exist as `extra_files` of the `test` codegen
  command (tasks 5/6, byte-pinned by the 45-example sweep - DO NOT change
  their generation):
  - `fuzz_target.rs`: `#![no_main]`, `use libfuzzer_sys::fuzz_target;`,
    expects the generated FSM items IN-CRATE (`use crate::{Fsm, Input,
    StopAssumptionHandler, ...}`) - i.e. it belongs in a cargo-fuzz project
    whose lib root is the generated production source.
  - `fuzz_target.c`: `LLVMFuzzerTestOneInput`, hardcodes
    `#include "rfs_generated_fsm.c"` - the generated PRODUCTION C source must
    sit next to it under exactly that name.
  - Shared decode plan (task 4): first byte = variant selector (`byte % N`),
    bool = one non-zero byte, bit<W> = ceil(W/8) little-endian bytes masked to
    W bits, enum = constant representative; the loop steps the FSM until the
    input is exhausted.
- Task 7's `rfs test` currently SKIPS the extra files with a debug note
  ("fuzz targets need cargo-fuzz/clang - see the Phase 5 task 12 examples").
  This task consumes them. Its pipeline, temp-dir Drop guard, exit-code
  contract (0 all-pass / 1 failure or error) and `--print-script` convention
  (deterministic POSIX sh, CWD-relative, no volatile temp paths) are the base
  to extend - keep the unit path byte-for-byte unchanged.
- Task 3's ext_tool resolver: PATH precedence CLI flag > env var > bare name
  on PATH; extra params via `RFS_<TOOL>` / `RFS_<TOOL>_ARGS` pairs (env args
  first, then CLI args appended); `Script` emitter renders `--print-script`.
  Existing presets: RUSTC, GCC, DOT, PLANTUML, MERMAID. The user requirement
  for this task: "align this functionality with other `rfs test...` commands
  to take all needed parameters for clang + libfuzzer from env variables" -
  i.e. new presets follow the exact same convention.
- Plan context (PHASE_5.md): "libFuzzer targets need CLANG to build - the
  skeleton is generated for both backends but building/running the C fuzz
  target is documented as a clang requirement (Rust side uses
cargo-fuzz/libfuzzer_sys)."
- Environment facts (2026-10-22): `clang` NOT installed, `cargo-fuzz` NOT
  installed. z3 was installed project-locally under `/workspace/host/z3`
  (TOOLCHAIN.md section 5) - use that as the model for a local clang install
  if the system package route is unavailable.
- Strict error semantics project-wide: missing/failing tool -> exit 1 +
  diagnostic, NO graceful degradation. Exception by design here: missing
  clang/cargo-fuzz must only be an error when the fuzz suite is SELECTED -
  the default (unit-only) path keeps working in environments without them.

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 3. | rfs info command + shared CLI infrastructure | ext_tool resolver (flag > env > PATH, `*_ARGS` convention) + Script emitter for `--print-script`; the new presets and script steps reuse both. |
| 5. | Rust test renderer + libfuzzer_sys skeleton | produces `fuzz_target.rs` (extra file) consumed by the rust fuzz pipeline. |
| 6. | C test renderer + libFuzzer skeleton | produces `fuzz_target.c` (extra file, hardcoded `#include "rfs_generated_fsm.c"`) consumed by the C fuzz pipeline. |
| 7. | rfs test command | the command being extended: pipeline shape, temp-dir guard, exit codes, script mode. |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| (none) | - | Task 12 (C example projects) stays as planned; its README can reference `rfs test --suite fuzz` once this task lands, but there is no code dependency. |

### Affected source files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `src/ext_tool.rs` | change | new presets `CLANG` (`RFS_CLANG`/`RFS_CLANG_ARGS`) and `CARGO_FUZZ` (`RFS_CARGO_FUZZ`/`RFS_CARGO_FUZZ_ARGS`, bare name `cargo` + fixed subcommand `fuzz`); small `ExtTool` extension for an optional fixed subcommand inserted right after the resolved path (before extra args); module-doc env-var list; unit pins. |
| `src/test_cmd.rs` | change | suite selection (`--suite unit\|fuzz`, default `unit`), the two fuzz pipelines (C: clang build of skeleton + production source; Rust: temp cargo-fuzz project scaffold), bounded run (`-max_total_time`), extra libFuzzer runtime flags, exit-code semantics, `--print-script` extension. |
| `src/main.rs` | change | new CLI flags (`--suite`, `--fuzz-max-total-time`, `--fuzz-flags`) + usage text for `rfs test`. |

### Other affected files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `TOOLCHAIN.md` | change | new section (next to the z3 section 5): installing clang + libFuzzer runtime and cargo-fuzz in this environment, verification commands, env-var table entries. |
| `BUILD.md` | change | env-var/tool table, test counts re-pinned, verification checklist row for the fuzz suite. |
| `KB.md` | change | 4.5 `rfs test` paragraph (suite selection + fuzz pipelines + exit codes); ext_tool env-var list. |
| `docs/roadmap/PHASE_5_PLAN.md` | change | plan-corrections row C4 recording the user-requested addition of task 17; execution-order line gains `17 <- 3+5+6+7`. |

### Affected test files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `tests/frontend_test.rs` | change | e2e: suite-selection behavior, real-tool fuzz runs (availability-gated like the other tool-gated tests), missing-tool error pins, stub argv proofs for the new presets, `--print-script` byte pin including the fuzz steps. |
| `src/ext_tool.rs` (unit tests) | change | CLANG/CARGO_FUZZ resolution + subcommand-placement rendering pins. |

## Planned actions

### Action 1: install clang + libFuzzer (+ cargo-fuzz), document the toolchain
Install a clang with the libFuzzer runtime (check the environment's package
route first; otherwise project-local under `/workspace/host/` following the
z3 model) and verify by building + briefly running a trivial
`LLVMFuzzerTestOneInput` target. Install `cargo-fuzz` (`cargo install
cargo-fuzz`) for the Rust side. Add the TOOLCHAIN.md section (install
location, verification commands, the new env vars) and the BUILD.md entries.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `TOOLCHAIN.md` | change | install docs + verification recipe; mirrors the z3 section so a future session can reproduce the setup. |
| `BUILD.md` | change | tool/env-var table rows for `RFS_CLANG*`, `RFS_CARGO_FUZZ*`, `RFS_FUZZ_FLAGS`. |

### Action 2: ext_tool presets for clang and cargo-fuzz
Add `ExtTool::CLANG` and `ExtTool::CARGO_FUZZ`. cargo-fuzz is a SUBCOMMAND of
`cargo`, so `ExtTool` gains an optional fixed-subcommand field (default empty)
rendered right after the resolved path, before extra args - unit-pinned with
stub argv. `RFS_FUZZ_FLAGS` (+ CLI `--fuzz-flags "..."`) carries EXTRA
libFuzzer RUNTIME flags for both backends: it is not a tool preset (no path),
documented in the ext_tool module header; env value first, then CLI appended
(same convention as the `*_ARGS` pairs).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/ext_tool.rs` | change | presets + subcommand extension + module docs + unit pins; keeps ONE resolution/rendering implementation for every external tool (task 3 invariant). |

### Action 3: suite selection + the two fuzz pipelines in `rfs test`
- `--suite unit|fuzz`, repeatable or comma-separated, DEFAULT `unit` (the
  existing behavior is unchanged and needs no clang/cargo-fuzz):
  - "all tests but clang+libfuzzer" = default (`--suite unit`);
  - "clang+libfuzzer only" = `--suite fuzz`;
  - "only the rust part of the tests" = `--backend rust --suite unit,fuzz`
    (the existing `--backend` flag selects which backend's artifacts are
    generated; suites apply to that backend).
  Invalid suite name -> usage error, exit 1.
- C fuzz pipeline: codegen the `test` artifact (existing dispatch) AND the
  production source (second registry dispatch, same options - no renderer
  logic duplicated); temp dir gets `fuzz_target.c` (the extra file) +
  `rfs_generated_fsm.c` (production; the exact name the skeleton's #include
  hardcodes); build `<clang> -O1 -fsanitize=fuzzer,address <RFS_CLANG_ARGS...>
  -o fuzz_bin fuzz_target.c rfs_generated_fsm.c`; run
  `./fuzz_bin -max_total_time=<N> <RFS_FUZZ_FLAGS...>`.
- Rust fuzz pipeline: scaffold a temp cargo project - `src/lib.rs` = generated
  production source, `fuzz/fuzz_targets/fuzz_target.rs` = the skeleton,
  minimal `Cargo.toml` (lib + `libfuzzer_sys` dep, version pinned below);
  then `<cargo> <RFS_CARGO_FUZZ_ARGS...> fuzz build` and
  `<cargo> <RFS_CARGO_FUZZ_ARGS...> fuzz run fuzz_target --
  -max_total_time=<N> <RFS_FUZZ_FLAGS...>`.
- `--fuzz-max-total-time <sec>` (default 30) maps to libFuzzer's
  `-max_total_time` so CI runs stay bounded; non-positive value -> usage error.
- Exit codes: 0 = every selected suite passes (fuzzer exits 0 within its
  budget); 1 = unit-test failure, fuzzer found a bug (non-zero exit), a
  SELECTED suite's tool is missing, or a usage/parse/validation error. Missing
  clang/cargo-fuzz with the default suite: no effect.
- `--print-script`: gains the fuzz steps when selected - C: heredocs for both
  sources + build + bounded run; Rust: scaffold via mkdir/cat + the cargo-fuzz
  commands. Same deterministic CWD-relative convention as task 7; the script
  header notes it needs clang/cargo-fuzz (and registry access for the Rust
  side).
- The "skipping N extra file(s)" debug note becomes conditional: consumed by
  the fuzz pipeline when selected, otherwise mentions `--suite fuzz`.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/test_cmd.rs` | change | the command logic; keeps the unit path byte-for-byte unchanged (all task 7 pins must pass unmodified). |

### Action 4: CLI flags + usage text
Wire `--suite`, `--fuzz-max-total-time`, `--fuzz-flags` through `main.rs`
into `TestOptions`; update the `rfs test` usage block (flag list + one-line
semantics, same style as the existing entries).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/main.rs` | change | flag parsing + usage text; no logic (stays thin per task 7 convention). |

### Action 5: tests
- ext_tool unit pins: CLANG/CARGO_FUZZ resolution (flag > env > PATH),
  subcommand placement in `render`, args ordering (env then CLI).
- frontend e2e (availability-gated where real tools are needed, like the
  existing tool-gated tests): both backends build + run a bounded fuzz target
  on a passing spec -> exit 0; fuzzer non-zero exit -> command exit 1 (pin
  with a stub "fuzz binary" that exits 1 - do NOT rely on finding a real bug);
  missing clang / missing cargo-fuzz with `--suite fuzz` -> exit 1 + pinned
  diagnostic; default suite unaffected when tools are absent.
- Stub argv proofs for the full C and Rust pipelines (exact command lines,
  including `-O1 -fsanitize=fuzzer,address`, `-max_total_time`, flag ordering).
- `--print-script` byte pin INCLUDING the fuzz steps + one manual `sh`
  execution of the printed script (recorded in the status file).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `tests/frontend_test.rs` | change | e2e coverage above; follows the task 7 test conventions (stubs, pins, gating). |
| `src/ext_tool.rs` | change | unit pins for the new presets. |

### Action 6: docs sync
TOOLCHAIN.md (Action 1), BUILD.md (counts re-pinned + env-var table +
verification row), KB.md 4.5 (`rfs test` paragraph: suite selection, fuzz
pipelines, exit codes) + ext_tool env-var list, PHASE_5_PLAN.md correction C4,
PHASE_5_TASK_17_STATUS.md, TASKS.md entry.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `BUILD.md`, `KB.md`, `docs/roadmap/PHASE_5_PLAN.md` | change | keep the knowledge base and plan consistent with the shipped surface. |

## Task deliverables
- clang + libFuzzer runtime (+ cargo-fuzz) installed in this environment and
  documented (TOOLCHAIN.md section, BUILD.md env-var table).
- `rfs test --suite fuzz` builds + runs bounded libFuzzer targets for BOTH
  backends; suite selection flags (`unit`/`fuzz`, default `unit`);
  env-var parameterization (`RFS_CLANG*`, `RFS_CARGO_FUZZ*`,
  `RFS_FUZZ_FLAGS`) aligned with the ext_tool conventions; `--print-script`
  covers the fuzz steps.
- Tests green (stubs + gated real-tool e2e); docs updated.

## Task exit criteria
- [ ] clang + libFuzzer runtime installed and verified (trivial fuzz target builds, links, runs briefly); cargo-fuzz installed; TOOLCHAIN.md section + BUILD.md env-var table in place.
- [ ] `rfs test` default behavior UNCHANGED: every pre-existing task 7 pin passes unmodified; the unit path needs no clang/cargo-fuzz.
- [ ] Suite selection pinned: default = unit only; `--suite fuzz` = fuzz only; `--suite unit,fuzz` = both; per-backend via the existing `--backend`; invalid suite name -> usage error exit 1.
- [ ] Both backends' fuzz targets build + run bounded (exit 0) on a passing spec with the real tools (gated e2e); fuzzer non-zero exit -> command exit 1 (stub-pinned).
- [ ] Env-var parameterization proven by stub argv: `RFS_CLANG`/`RFS_CLANG_ARGS`, `RFS_CARGO_FUZZ`/`RFS_CARGO_FUZZ_ARGS` (incl. the fixed `fuzz` subcommand placement), `RFS_FUZZ_FLAGS` (+ `--fuzz-flags`) with env-then-CLI ordering.
- [ ] Missing tool for a SELECTED suite -> exit 1 + clear diagnostic (pinned); missing tools with the default suite -> no effect.
- [ ] `--print-script` includes the fuzz steps; wrapper byte-pinned; one manual `sh` execution verified and recorded in the status file.
- [ ] Full main suite + workspace green, clippy/fmt clean (counts re-pinned in BUILD.md).

## Non-goals
- Corpus persistence/merging, coverage reporting, or artifact-dir management
  (per-run temp only; libFuzzer's default behavior inside the temp dir).
- Automating the clang/cargo-fuzz installation (documented manual step; the
  command detects absence and errors clearly when the fuzz suite is selected).
- Changing fuzz-skeleton generation (tasks 5/6 output is byte-pinned by the
  45-example sweep; the `#include "rfs_generated_fsm.c"` name and the wiring
  comments in the headers stay as-is - the CLI now provides the wiring).
- Parallel fuzzing across backends or suites (sequential is fine for v1).

## Other information
- `libfuzzer_sys` version: pick a stable release that is present in the local
  registry cache (`/workspace/host/.cargo/registry`) so the temp-project build
  works offline; pin it in the scaffolded Cargo.toml. It must not leak into
  the `rfs` toolchain crate or generated code (the skeleton already depends on
  it by design - that is its only consumer).
- Fuzz runs are exploratory: the CI gate is "no crash within
  `-max_total_time`". Keep the default budget small (30 s) so
  `rfs test --suite fuzz` stays CI-friendly; users raise it per run.
- The plan's "Other information" already anticipated this task: "clang: check
  availability for the C libFuzzer example build docs; the skeleton generation
  itself does not require clang."
- If the environment has no network access for `cargo install cargo-fuzz`,
  document the exact offline route used (e.g. prebuilt binary) in TOOLCHAIN.md
  - the command's contract (detect + clear error when absent) is unchanged.
