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
- [ ] All four targets build (cargo-fuzz build if nightly available, else `cargo check` in the detached workspace - record which path was used).
- [ ] Bounded smoke runs (`-max_total_time 10`) pass on the available toolchain(s); any crash found is fixed via a plan-correction task before this exit criterion is claimed.
- [ ] Slow suite green within ~5 min: all specs compile in BOTH backends (rustc + gcc -Wall), MC exit codes in {0,1,2}, no false PASS vs ref_mc, stderr stable across repeated runs.
- [ ] BUILD.md + TOOLCHAIN.md document the workspace; smoke gate added to the verification checklist.
- [ ] Full `cargo test` green (the new suite is part of it); clippy/fmt clean.

## Non-goals
- No coverage-guided CI enforcement (smoke runs only - long libFuzzer sessions are manual/nightly); no fuzzing of the SMT path (feature-gated, outside v1 guarantee); no changes to generated-code fuzz targets (those are Phase 5's `rfs test --suite fuzz` surface, untouched); no ASAN/LSAN beyond what libFuzzer enables by default.

## Other information
- The codegen target is the SLOWEST per iteration (six generations) - keep its
  structural checks cheap (string scans, no compilation inside the fuzzer;
  compilation lives in the slow suite).
- If a target finds a real bug: fix via plan correction + re-run; add the crashing
  input to the relevant test suite as a regression pin.
