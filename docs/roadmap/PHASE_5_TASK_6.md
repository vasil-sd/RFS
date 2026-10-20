# Phase 5, Task 6 - Implementation details

> Written for a future LLM session: after reading `START.md`,
> `docs/roadmap/INTRO.md` and this file, the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal
Mirror task 5 in the C backend: render the SAME extended test plan with
IDENTICAL coverage (same cases, same expected outcomes, same stable test
names) as plain-assert C, generate the libFuzzer skeleton with a documented
clang recipe, keep every artifact `gcc -Wall` clean, and extend the parity
suite to compare both backends' generated test executables on the new classes.

## Context

- Parity principle (INTRO.md): backend-generated deliverables ship for ALL
  backends with identical behavior. The C renderer consumes the same plan data as task 5 - coverage identity is by construction, verified by the parity
  suite comparing per-test results (names + pass/fail + printed coverage
  counts) of both executables over every property-bearing example.
- `src/backends/c_test.rs` today: plain-assert harness with its own `main`,
  module-level tests only. The C test executable prints one line per test
  (`PASS <name>` / `FAIL <name>`) - keep that wire format; the new classes use
  the SAME stable names as Rust (task 5 convention).
- libFuzzer skeleton (`fuzz_target.c`): `LLVMFuzzerTestOneInput` decoding bytes
  per the planner's decode strategy, stepping the FSM in a loop. BUILDING it
  requires clang - document the recipe in the emitted header comment + task 12
  example README; generation itself does not need clang.
- `gcc -Wall` zero-warning bar: the test suites' `compile_c` helper already
  asserts it (Phase 4 task 7) - keep using it for every new C artifact.

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 4. | Extended shared test planner | plan data source |
| 5. | Rust test renderer + libfuzzer_sys skeleton | the coverage baseline the parity suite compares against (names, expected outcomes) |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 7. | `rfs test` command | runs the C executable for `--backend c` |
| 12. | C example projects + fuzzing harness examples | wires the emitted libFuzzer skeleton |

### Affected source files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `src/backends/c_test.rs` | change | render the four new test classes in C + emit the libFuzzer skeleton file |

### Affected test files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `tests/backend_parity_test.rs` | change | extend per-test result comparison to the new classes over all property-bearing examples; fuzz skeleton presence in both backends |
| `tests/codegen_test.rs` | change | C-side presence/naming assertions mirroring task 4's |

## Planned actions

### Action 1: render the four new test classes in C
Plain-assert implementations of per-transition (assert next-state + outputs /
no-progress case), invariant (per-step re-check, fail with step index), monitor
debug (injected violating trace must set `RFS_PROPERTY_VIOLATION` for the
target label), random-trace (same LCG constants/seed/steps - coverage counts
printed identically). Same stable names as Rust.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/backends/c_test.rs` | change | the four C renderers |

### Action 2: libFuzzer skeleton + recipe
Emit `fuzz_target.c` (separate file) with the decode loop; header comment with
the clang build/run recipe. No clang required to generate.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/backends/c_test.rs` | change | skeleton emission |

### Action 3: parity extension
Extend `backend_parity_test.rs`: for every property-bearing example, run BOTH
generated test executables and compare the full per-test result list (names +
outcomes + coverage lines) byte-for-byte; assert fuzz skeletons exist in both.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `tests/backend_parity_test.rs` | change | the comparison extension |
| `tests/codegen_test.rs` | change | C presence/naming assertions |

## Task deliverables
- C `test` output with identical coverage to Rust (same names/outcomes).
- libFuzzer skeleton + documented clang recipe.
- Parity suite covering the new classes; gcc -Wall clean on all artifacts.

## Task exit criteria
- [ ] Per-test result lists of both backends' test executables are byte-identical over ALL property-bearing examples (parity test green).
- [ ] Every generated C artifact compiles with `gcc -Wall` at zero warnings (compile_c assertion holds).
- [ ] libFuzzer skeleton emitted in C with the documented recipe; generation works without clang installed.
- [ ] Full main suite green, clippy/fmt clean.

## Non-goals
- Building/running the C fuzz target here (clang wiring is task 12).
- `rfs test` CLI (task 6).
- Changing the Rust renderer (task 4's output is the baseline).

## Other information
- If a per-test line differs between backends, fix the RENDERER that deviates
  from the plan data - never weaken the comparison.
