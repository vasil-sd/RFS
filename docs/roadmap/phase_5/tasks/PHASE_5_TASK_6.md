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

## Task corrections

| Correction | Description + rationale |
|------------|-------------------------|
| C6-1: zero-warning bar extended to ALL generated C artifacts | The full 45-example sweep (beyond the parity suite's property-bearing set) found pre-existing `gcc -Wall` warnings on transition-less specs (`29_empty_module`, `30_multi_module`: unused `rfs_commit`; the old harness additionally left `ALL_INPUTS`/`clone_fsm`/`stop_handler` unused there). To meet the exit criterion "every generated C artifact compiles with gcc -Wall at zero warnings": (a) `rfs_commit` in `c_fsm.rs` now carries `RFS_UNUSED` (verified: the ONLY production-output delta vs the pre-change build on all 175 artifacts is that one token); (b) the harness emits `cont_handler` only when a test uses it, and the fuzz skeleton emits `fuzz_stop_handler` only for input-bearing modules. The new per-test structure already removed the three old-harness warnings on those specs. |
| C6-2: negative test now drives with `RFS_CONTINUE` (was `stop_handler`) | The pre-task C harness used a STOP assumption handler for the negative search's trial steps, while the Rust baseline (task 5) uses `ContinueAssumptionHandler` - an assumption-violating candidate is accepted in Rust but rejected in C. Mirrored Rust exactly (parity principle: fix the renderer that deviates from the plan data). No example changed outcome (suite green). |
| C6-3: per-test `PASS <name>`/`FAIL <name>` lines introduced for ALL tests | The task file's Context describes this wire format as pre-existing, but the old harness only printed `FAIL: <msg>` diagnostics + a summary (no per-test names at all). The new table-driven harness prints one line per test for every class - including the three module-level tests, which now carry the Rust stable names (`initial_state_is_correct`, `positive_sequence_runs`, `negative_unexpected_input_search`) so the parity comparison covers them too. C test-executable stdout therefore changes (additive lines + summary kept); no pinned test depended on the old format. |

## Other information
- If a per-test line differs between backends, fix the RENDERER that deviates
  from the plan data - never weaken the comparison.
