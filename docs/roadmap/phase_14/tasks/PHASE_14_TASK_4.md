# Phase 14, Task 4 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal
Make the C `test` artifact and the emitted `fuzz_target.c` skeleton reference
the FSM through the header (never `#include "...fsm.c"`), and migrate the
`rfs test` C pipelines (unit + fuzz) to compile the separate sources to objects
and link - including regenerating the `--print-script` heredocs and their pins.

```
rfs codegen c test spec.rfs --out arq
  -> arq-fsm.h / arq-fsm.c        (co-emitted or taken)
  -> arq-test.c                   (#include "arq-fsm.h" + tests + main)
  -> arq-fuzz-target.c            (#include "arq-fsm.h" + LLVMFuzzerTestOneInput)
rfs test --backend c spec.rfs     (unit: gcc -Wall -c both, link; fuzz: clang over both sources)
```

## Context

- The C test generator (`src/codegen/backends/c/test.rs`): `generate_unit_tests`
  (line ~787) emits a self-contained test .c (FSM inlined + module-level /
  per-transition / invariant / monitor-violation / random-trace tests +
  `main()`); `emit_fuzz_target_c` (line ~545) emits the libFuzzer skeleton
  which does `#include "rfs_generated_fsm.c"` (line 627) and carries wiring
  comments that teach the OLD generation command (`rfs codegen c production
  <spec> --out rfs_generated_fsm.c`, line ~582) and the old build line.
- The `rfs test` C pipelines (`src/cli/commands/test.rs`): the unit pipeline
  compiles the single self-contained test .c with `gcc -Wall`; the fuzz
  pipeline (lines ~375-384, ~849, ~864) writes the production output under the
  HARDCODED name `rfs_generated_fsm.c` next to the skeleton and clang-compiles
  ONLY the skeleton (the "compile ONLY the skeleton" gotcha in TOOLCHAIN.md
  7.4 - it exists because the skeleton source-includes the .c; with header
  references that gotcha disappears and BOTH sources are compiled together).
  The C fuzz run keeps `ASAN_OPTIONS=detect_leaks=0` merged over user values.
- The `--print-script` mode prints a self-contained POSIX sh script instead of
  executing; its heredocs pin the generated-file names, and
  `tests/utils_test.rs` (~lines 3490-3518) asserts those heredoc lines byte-
  for-byte (`cat > rfs_generated_fsm.c <<'__RFS_GENERATED_FSM_C__'`,
  `rm -f fuzz_target.c rfs_generated_fsm.c fuzz_bin`, ...).
- Tasks 1-2 have landed: slots, take-or-emit, resolved-name plumbing in
  `BackendOptions`, and the fsm header/source pair all exist.

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 1. | Codegen output model + CLI flag family | Slots + take-or-emit + resolved names. |
| 2. | C production header/source split | The fsm header the test/fuzz artifacts include. |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 9. | Example fixes | fuzz_c's Makefile mirrors this task's clang build shape. |
| 11. | Close-out | Full verification incl. `rfs test` both suites. |

### Affected source files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `src/codegen/backends/c/test.rs` | change | Test artifact includes the fsm header (no inlined FSM); fuzz skeleton includes the fsm header; wiring comments updated to the new generation commands. |
| `src/cli/commands/test.rs` | change | C unit pipeline: write fsm pair + test.c, `gcc -Wall -c` each, link. C fuzz pipeline: clang compiles fsm.c + fuzz_target.c together; hardcoded `rfs_generated_fsm.c` name replaced by the resolved names; `--print-script` heredocs regenerated. |

### Affected test files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `tests/utils_test.rs` | change | `--print-script` heredoc pins regenerated from the NEW script (never hand-edited to match). |
| `tests/codegen_test.rs` | change | C test legs: pair + reference assertions; no `#include "...fsm.c"` anywhere in generated C. |

## Planned actions

### Action 4.1 - Test artifact + fuzz skeleton reference the fsm header

In `src/codegen/backends/c/test.rs`: the test .c drops the inlined FSM and
starts with its line-1 artifact header + `#include "<fsm header>"` (name from
`BackendOptions`) + the test suites + `main()`. The fuzz skeleton replaces
`#include "rfs_generated_fsm.c"` with `#include "<fsm header>"` and its wiring
comments are rewritten to the new CLI (generation via `rfs codegen c test <
spec> --out BASE` or explicit flags; build with clang over BOTH sources). Fill
the fsm slots as referenced so take-or-emit applies.

Rationale: this is the user's concrete complaint - whole-source inclusion -
removed at the source of the pattern (the generators), so every consumer
(including hand-rolled builds) gets header references.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/codegen/backends/c/test.rs` | change (test .c emission) | Inlined FSM removed; fsm-header include added. |
| `src/codegen/backends/c/test.rs` | change (`emit_fuzz_target_c` include + comments) | Header include replaces the .c include; comments teach the new CLI. |

### Action 4.2 - Migrate the `rfs test` C pipelines

In `src/cli/commands/test.rs`: unit pipeline - generate the fsm pair + test.c
into the temp dir under their resolved names, `gcc -Wall -c` each .c (zero-
warning bar), link into the test binary, run. Fuzz pipeline - write the fsm
pair + skeleton, clang `-O1 -fsanitize=fuzzer,address` over BOTH sources (or
their objects) into the fuzz binary; drop the "compile ONLY the skeleton"
constraint and re-verify no multiple-definition errors (none expected: the
skeleton no longer source-includes the .c); keep the `ASAN_OPTIONS=
detect_leaks=0` merge. Regenerate the `--print-script` heredocs from the new
invocations.

Rationale: the CI entry point must exercise exactly the multi-file shape users
build by hand; the print-script output is a documented, pinned artifact.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/cli/commands/test.rs` | change (C unit pipeline) | Object+link build. |
| `src/cli/commands/test.rs` | change (C fuzz pipeline, lines ~375-384, ~849, ~864) | Both-source clang build; resolved names replace the hardcoded one. |

### Action 4.3 - Update the print-script pins + reference assertions

Regenerate the `--print-script` heredoc pins in `tests/utils_test.rs` from the
NEW script output; add/adjust `tests/codegen_test.rs` C test legs to assert the
header include in both artifacts and the absence of any `#include "...fsm.c"`
in all generated C (a grep bar over every C artifact of every command).

Rationale: pins must be captured from real output, and the no-source-include
invariant is the point of the phase - it needs a standing regression test.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `tests/utils_test.rs` | change (heredoc pins ~3490-3518) | Regenerated from the new script. |
| `tests/codegen_test.rs` | change (C test legs + grep bar) | Reference assertions. |

## Task deliverables
- C `test` + fuzz skeleton reference the fsm header; zero `#include "...fsm.
c"` in any generated C.
- `rfs test --backend c` unit + fuzz pipelines build via separate sources and
  pass; `--print-script` output updated with pins regenerated.

## Task exit criteria
- [ ] `rfs codegen c test <spec> --out arq` writes the four files; both .c
      artifacts include `"arq-fsm.h"`; no generated C file source-includes any
      other generated .c (grep bar over all commands/backends).
- [ ] `rfs test --backend c` unit suite green (object+link, zero gcc -Wall
      warnings); fuzz suite builds under clang and a bounded smoke run exits 0.
- [ ] `--print-script` pins in utils_test match the actual script byte-for-byte.
- [ ] `cargo test` (root) green; fmt clean; clippy at baseline.

## Non-goals
- No Rust pipeline changes (task 5).
- No example Makefile changes (task 9).
- No change to test-suite CONTENTS (which tests are generated) - only how the
  sources reference the FSM and how they are compiled.

## Other information

- Re-check: the fuzz skeleton's decode strategy (`build_fuzz_decode`) is
  independent of the include shape - confirm no other part of the skeleton
  assumes the .c-inclusion model (e.g. comments about translation units).
- If any test compiles the C test artifact directly (outside `rfs test`),
  migrate it to object+link in this task as well (grep for gcc invocations on
  generated test output).
