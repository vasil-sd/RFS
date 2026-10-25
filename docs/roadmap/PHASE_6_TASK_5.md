# Phase 6, Task 5 - Implementation details

> Written for a future LLM session: after reading `START.md`,
> `docs/roadmap/INTRO.md` and this file, the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal
Fuzz the TOOLCHAIN ITSELF (D-P6-4) in two layers: fast in-process libFuzzer
targets in a root-level `fuzz/` cargo-fuzz workspace (parser, validator, IR
serialization, code generator), and a slow compile-and-run seeded deterministic
suite (`tests/toolchain_fuzz_test.rs`) covering the model checker on small
generated specs - meeting PHASE_6.md's goals: no crashes on malformed-but-
parseable input, no invalid target-language code from valid specs, no false
`PASS (complete)`, stable diagnostics.

## Context

- **Environment**: `cargo-fuzz` is installed; `clang` + `gcc` present;
  `libfuzzer-sys 0.4.13` + `arbitrary 1.5.0` are cached in CARGO_HOME (the root
  `fuzz/` workspace works OFFLINE). Nightly availability for cargo-fuzz RUNS on
  the Rust side: check TOOLCHAIN.md section 7 first - if no nightly, the Rust
  targets are build/check-only (the `crates/examples/fuzz_rust` precedent: build
  + `cargo check`, no fuzzer runtime) and the smoke gate is C-side only; record
  as a plan correction if that happens.
- **Layout precedent**: `crates/examples/fuzz_rust/fuzz/Cargo.toml` - detached
  `[workspace]` table, `[package.metadata] cargo-fuzz = true`, `[[bin]]` entries
  with `harness = false`. Mirror this at the repo root: `fuzz/Cargo.toml`
  (package `rfs-fuzz`, deps: `libfuzzer-sys = "0.4"`, `arbitrary = "1"`,
  `rfs = { path = ".." }`), targets in `fuzz/fuzz_targets/`. Add `fuzz/target`
  to `.gitignore` (check the current .gitignore first - it is short).
- **Arbitrary strategy**: do NOT derive `Arbitrary` over raw IR types (would
  produce invalid IR). Instead: (a) for parser/validator fuzzing, feed raw bytes
  to `parse_source` / `Ir::from_json`; (b) for codegen fuzzing, build random
  VALID specs through task 3's `specgen` builder driven by the fuzzer's data
  (implement a small `Arbitrary`-backed wrapper that maps fuzzer bytes ->
  specgen seed + bounds). This keeps every generated spec valid by construction.
- **Structural invariants for the codegen target** (on every generated artifact,
  both backends x production/test/mc): generation returns Ok; text has balanced
  braces/parens/brackets; line 1 matches the task-2 header format (`// Generated
  by RFS v` prefix + `spec sha256:` field); no placeholder tokens (grep for
  `TODO`, `FIXME`, `PLACEHOLDER`, `unimplemented!` in Rust output, `TODO` in C);
  Rust output contains the core symbols (`pub struct Fsm`), C output contains its
  core symbols (verify names against the backends).
- **Slow suite design** (`tests/toolchain_fuzz_test.rs`): seed sets = 3 seeds x
  10 specs (specgen small bounds, pinned seeds). Per spec: parse+validate ->
  generate all six artifacts (2 backends x production/test/mc) -> compile Rust
  with rustc + C with `gcc -Wall` (temp dir; compilation success IS the "no
  invalid target-language code" check) -> run the MC binaries: exact mode with a
  small `--max-states` AND fuzz mode (`--mode fuzz --runs 5 --bound 5 --seed S`) ->
  assert exit codes in {0, 1, 2} (no crashes); NO FALSE PASS: where task 3's
  `ref_mc` says Fail, the generated engine must NOT say PassComplete (and status
  agreement per task 4.1's semantics); STABLE DIAGNOSTICS: run each binary twice,
  stderr byte-identical. Runtime budget: keep the whole suite under ~5 minutes
  (measure; reduce N if needed and record it).
- **Docs**: BUILD.md gets a "Toolchain fuzzing" section (targets, run commands,
  nightly/clang requirements) + a bounded smoke gate in the section-5 checklist
  (`-max_total_time 10` per target, availability-gated exactly like the existing
e2e fuzz tests); TOOLCHAIN.md gets a short subsection under its fuzzing section
  (section 7) pointing at the root `fuzz/` workspace.

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 2. | Stability and versioning (v1.0) | the codegen target asserts the task-2 header format as a structural invariant |
| 3. | Test infrastructure + user-facing reference MC & spec generator | `specgen` builds the random valid specs; `ref_mc` is the ground truth for "no false PASS (complete)" |

### Affected source files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `fuzz/Cargo.toml` | new | detached cargo-fuzz harness package (`rfs-fuzz`) |
| `fuzz/fuzz_targets/parser.rs` | new | arbitrary bytes -> parse_source must not panic; on success validate + IR round-trip |
| `fuzz/fuzz_targets/validator.rs` | new | random valid IR (specgen-backed) -> validate must not panic; random JSON bytes -> `Ir::from_json` returns Err, never panics |
| `fuzz/fuzz_targets/ir_roundtrip.rs` | new | random IR -> to_json/from_json/to_json byte-stable |
| `fuzz/fuzz_targets/codegen.rs` | new | random valid spec -> 3 commands x 2 backends generate Ok + structural invariants |
| `fuzz/README.md` | new | build/run instructions incl. nightly requirement on the Rust side |

### Affected test files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `tests/toolchain_fuzz_test.rs` | new | slow compile-and-run fuzz suite (the CI-runnable layer) |

### Other affected files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `BUILD.md` | change | "Toolchain fuzzing" section + smoke gate in the verification checklist |
| `TOOLCHAIN.md` | change | subsection under the fuzzing section: root `fuzz/` workspace usage |
| `.gitignore` | change | `fuzz/target` (verify current content first) |

## Planned actions

### Action 1: cargo-fuzz workspace + four targets (subtask 5.1)
Create `fuzz/` per Context (detached workspace, cargo-fuzz marker, deps). Implement
the four targets with the specgen-backed Arbitrary wrapper shared in
`fuzz/fuzz_targets/common.rs` (or a small module each target includes - keep it
duplicated-free but simple). Verify: `cargo +nightly fuzz build` works if nightly
is available, else `cargo check` in the detached workspace (record which). Run a
bounded smoke per target (`-max_total_time 10`) on whatever toolchain is
available; any crash found => fix the underlying code via a plan-correction task,
never by weakening the target.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `fuzz/Cargo.toml` | new | harness package |
| `fuzz/fuzz_targets/{parser,validator,ir_roundtrip,codegen}.rs` | new | the four targets |
| `fuzz/README.md` | new | usage docs |
| `.gitignore` | change | fuzz/target |

### Action 2: slow compile-and-run fuzz suite (subtask 5.2)
New `tests/toolchain_fuzz_test.rs` per Context's design: pinned seed sets,
six artifacts per spec, rustc + gcc -Wall compilation, MC runs in exact + fuzz
mode, exit-code set assertion, ref_mc ground-truth cross-check (no false PASS),
double-run stderr stability. Temp-dir hygiene like the existing suites (unique
per-process subdir, cleaned on drop).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `tests/toolchain_fuzz_test.rs` | new | the suite |

### Action 3: documentation + smoke gate (subtask 5.3)
BUILD.md: "Toolchain fuzzing" section (what is fuzzed, how to run each target,
requirements) + one line in the section-5 verification checklist (bounded smoke,
availability-gated). TOOLCHAIN.md: short subsection under section 7 pointing at
the root `fuzz/` workspace and its nightly/clang requirements.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `BUILD.md` | change | fuzzing section + checklist line |
| `TOOLCHAIN.md` | change | subsection under the fuzzing section |

## Task deliverables
- Root `fuzz/` cargo-fuzz workspace with four targets (parser, validator,
  ir_roundtrip, codegen) + README.
- `tests/toolchain_fuzz_test.rs`: CI-runnable compile-and-run fuzz layer covering
  the model checker on small generated specs.
- BUILD.md/TOOLCHAIN.md documentation + bounded smoke gate.

## Task exit criteria
- [x] All four targets build (cargo-fuzz build if nightly available, else `cargo check` in the detached workspace - record which path was used). -> nightly WAS available: full `cargo +nightly fuzz build` (recorded in PHASE_6_TASK_5_STATUS.md).
- [x] Bounded smoke runs (`-max_total_time 10`) pass on the available toolchain(s); any crash found is fixed via a plan-correction task before this exit criterion is claimed. -> all four targets exit 0 (codegen extended to 60 s); crashes found were fixed as plan corrections 5a/5b (toolchain) + in-task harness fixes.
- [x] Slow suite green within ~5 min: all specs compile in BOTH backends (rustc + gcc -Wall), MC exit codes in {0,1,2}, no false PASS vs ref_mc, stderr stable across repeated runs. -> 30 specs in ~28 s.
- [x] BUILD.md + TOOLCHAIN.md document the workspace; smoke gate added to the verification checklist. -> BUILD.md section 6 + checklist line; TOOLCHAIN.md section 7.5.
- [x] Full `cargo test` green (the new suite is part of it); clippy/fmt clean. -> 447/447 root, 471/471 workspace, 1519/1519 generated; clippy no new warnings; fmt clean.

## Non-goals
- No coverage-guided CI enforcement (smoke runs only - long libFuzzer sessions are manual/nightly); no fuzzing of the SMT path (feature-gated, outside v1 guarantee); no changes to generated-code fuzz targets (those are Phase 5's `rfs test --suite fuzz` surface, untouched); no ASAN/LSAN beyond what libFuzzer enables by default.

## Task corrections

| Correction | Description + rationale |
|------------|-------------------------|
| Plan corrections 5a + 5b (toolchain bugs found by this task's own layers) | The `codegen` fuzz target found that `test` codegen FAILED on a fully valid spec (specgen seed 24099 / small): the shared planner keyed binding values by declared field names while `eval_env` looks them up by pattern-binding name, so binding-referencing emits evaluated to None and the not-taken renderer's strict backstop errored both backends. Fixed in `src/codegen_test.rs` (`ConcreteInput.binding_vals` keyed by binding name; both construction sites) + 2 lib regression pins; seed 24099 added to the slow-suite pinned seeds. The slow suite then found a C-only SEGV: the fuzz-mode property-violation report indexed `steps[prop_trigger_step - 1]` = `steps[-1]` when the obligation started at sample 0 (trigger step 0); fixed in `src/backends/c_mc.rs` to use the safe `trace_vars()` helper (mirroring the exact path; Rust's `vars_at` was already correct) + a deterministic micro-spec pin (`c_mc_trigger_step_zero_report_no_crash`). Both recorded in `PHASE_6_PLAN.md` "Plan corrections" per the bug-found protocol; neither weakened any target. |
| Fuzz-harness bugs fixed within the task (not toolchain code) | (a) The first `codegen` smoke hit a FALSE POSITIVE of the task-5 structural scanner: char literals containing a double quote (`'\"'`, as in the MC JSON escape renderers) desynchronized the comment/string-skipping balance check; the scanner now skips one-char and escaped char literals correctly (lifetimes like `'a` do not match the literal shapes). The crashing input (empty bytes -> fallback spec seed 0) is pinned in `fuzz/regression_seeds/codegen_empty_input`. (b) `ir_roundtrip.rs` itself panicked on short inputs (`copy_from_slice` with a <8-byte source); fixed to pad, crashing input pinned in `fuzz/regression_seeds/ir_roundtrip_short_rest`. (c) Initial targets wrapped `fuzz_target!` in their own `fn main()`, which SHADOWS the libFuzzer runtime's main - binaries exited silently with code 0 without fuzzing; switched to the RFS-skeleton convention `#![no_main]` + top-level macro. All three conventions documented in TOOLCHAIN.md section 7.5. |
| Slow-suite design details settled during execution | Shared exact-mode state cap (`MAX_STATES = 5000`) for BOTH ref_mc and the generated engines: with an identical cap both sides either complete or report UnknownStateLimit together (counting aligned per tasks 3/4), so status agreement holds regardless of actual space size while keeping runs fast. Scratch hygiene follows the sibling-suite pattern (`target/rfs_toolchain_fuzz/<pid>/`, cleaned by `cargo clean`) - no existing suite uses Drop-based cleanup, so "cleaned on drop" was realized as the established per-process-subdir convention. C production artifacts compile with `gcc -Wall -c` (no main in the artifact); Rust production with `--crate-type lib`; Rust test with `--test` (mirrors `rfs test`). |

## Other information
- The codegen target is the SLOWEST per iteration (six generations) - keep its
  structural checks cheap (string scans, no compilation inside the fuzzer;
  compilation lives in the slow suite).
- If a target finds a real bug: fix via plan correction + re-run; add the crashing
  input to the relevant test suite as a regression pin.
