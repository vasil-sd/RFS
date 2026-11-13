# Phase 14, Task 3 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal
Make the C `mc` artifact a header/source pair that references the FSM from the
separately generated fsm files instead of inlining it: `BASE-mc.c` includes
the fsm header, and builds compile `fsm.c` + `mc.c` to objects and link. No MC
artifact ever contains a second copy of the FSM implementation.

```
rfs codegen c mc spec.rfs --out arq
  -> arq-fsm.h / arq-fsm.c   (co-emitted or taken, per take-or-emit)
  -> arq-mc.h                (MC engine public API + report types)
  -> arq-mc.c                (#include "arq-mc.h" + #include "arq-fsm.h" + engine + main)
gcc -Wall -c arq-fsm.c arq-mc.c && gcc -o arq_mc arq_fsm.o arq_mc.o
```

## Context

- The C MC generator (`src/codegen/backends/c/mc.rs`, ~2858 lines) currently
  inlines the WHOLE production FSM: line 66 calls
  `crate::codegen::backends::c::fsm::generate_core(...)` and splices that
  output into the executable, and it reuses `render_mc_step_all_c` /
  `render_liv_sample_evaluator_c` from c/fsm.rs for the MC-specific step
  enumeration. The result is one self-contained .c with `main()`.
- Task 2 has landed: the fsm header/source pair exists, `BackendOptions`
  carries the resolved fsm reference names, and the CLI write loop already
  implements take-or-emit for referenced fsm slots (including the partial-pair
  usage error and the D14-4 hash-mismatch warning).
- The MC engine's report structs (the text/JSON report model shared with
  `src/codegen/mc_concepts.rs`) are currently emitted inside the single .c;
  they become the header's public surface (D14-6): the run entry point plus
  the report types, so the pair is genuinely useful, not ceremonial.
- Test legs that compile generated C MC artifacts and must migrate to
  object+link: `tests/backend_parity_test.rs` (gcc -Wall single-file compile,
  lines ~146-163, with the zero-warning bar), `tests/benchmark_mc_test.rs`
  (~180-197, same pattern), `tests/c_compiler_agnostic_test.rs` (real `-c`
  compiles under gcc AND clang at -O2, byte-identical-output parity between
  the two compilers), plus the liveness/monitor suites as applicable (check
  each for a generated-C compile leg).
- The MC binary's runtime CLI flags and report formats are PARITY-PINNED and
  must stay byte-identical - this task changes only how the source is split
  and compiled, never what the binary prints.

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 1. | Codegen output model + CLI flag family | Slots, take-or-emit write loop, resolved-name plumbing. |
| 2. | C production header/source split | The fsm header that mc.c includes. |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 8. | Codegen architecture docs + dev/llm re-sync | Documents the mc pair model. |
| 11. | Close-out | Full verification incl. the migrated MC legs. |

### Affected source files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `src/codegen/backends/c/mc.rs` | change | Stop inlining `generate_core` output; emit mc-header (engine API + report types) and mc-source (engine + main, including own header + fsm header). |

### Affected test files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `tests/backend_parity_test.rs` | change | Compile fsm.c + mc.c to objects, link, run; keep the gcc -Wall zero-warning bar. |
| `tests/benchmark_mc_test.rs` | change | Same object+link migration. |
| `tests/c_compiler_agnostic_test.rs` | change | `-c` compile BOTH files under gcc and clang at -O2; parity comparison on the linked binaries. |
| liveness/monitor test suites with a generated-C MC compile leg | change | Same migration, as applicable (verify each suite during implementation). |

## Planned actions

### Action 3.1 - Split the MC artifact into header + source, reference the fsm header

In `src/codegen/backends/c/mc.rs`: remove the spliced `generate_core` output
from the executable body; the mc SOURCE now starts with its line-1 artifact
header, `#include "<own mc header>"`, `#include "<fsm header>"` (both names
from `BackendOptions`, stamped by the CLI layer), then the MC engine
(state encoding, traversal, liveness obligations, reporting) and `main()`. The
mc HEADER carries its own line-1 artifact header, an include guard, the std
includes its declarations need, the report type definitions (hoisted from the
engine - re-check that none is currently defined inside a function body or
behind statics; if so, hoist it as part of this split), and the public run
entry point declaration. The `render_mc_step_all_c` / `render_liv_sample_
evaluator_c` helpers from c/fsm.rs stay where they are (they render into the
mc source, not the fsm files). Fill BOTH mc slots in `generate_model_checker`
and mark the fsm slots as referenced so the write loop applies take-or-emit.

Rationale: D14-6 - the header gets real content (engine API + report types)
rather than a ceremonial empty file; the FSM implementation exists exactly
once, in the fsm source file.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/codegen/backends/c/mc.rs` | change (executable-body emission) | Inlined FSM removed; fsm-header include added. |
| `src/codegen/backends/c/mc.rs` | change (report-type definitions) | Hoisted into the mc header emission. |
| `src/codegen/backends/c/mc.rs` | change (`generate_model_checker` return) | Fills mc_header + mc_code slots; declares fsm slots referenced. |

### Action 3.2 - Migrate the MC-compiling test legs to object+link

Update every leg that compiles a generated C MC artifact: write the fsm pair
+ mc pair to the temp dir, `gcc -Wall -c` each .c (zero-warning bar kept),
link the objects into the binary, run it. In `c_compiler_agnostic_test.rs`
apply the same under clang at -O2 and keep the gcc-vs-clang byte-identical
output comparison on the LINKED binaries. Re-run the full parity/benchmark/
liveness suites.

Rationale: the compile shape changes from one self-contained file to two
sources + link; the legs must exercise exactly the shape users and the
examples will use, or the warning bar and the compiler-parity guarantee go
untested.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `tests/backend_parity_test.rs` | change (C MC compile leg) | Object+link; zero-warning bar preserved. |
| `tests/benchmark_mc_test.rs` | change (C MC compile leg) | Same. |
| `tests/c_compiler_agnostic_test.rs` | change (both compiler legs) | `-c` on both files; parity on linked binaries. |
| liveness/monitor suites as applicable | change | Same, where a generated-C MC compile leg exists. |

## Task deliverables
- C `mc` emits a header/source pair; the source references the fsm header;
  no FSM implementation inside any mc artifact.
- All MC-compiling test legs build via object+link and pass with byte-
  identical MC reports.

## Task exit criteria
- [ ] `rfs codegen c mc <spec> --out arq` writes all four files; `arq-mc.c`
      contains `#include "arq-fsm.h"` and NO definition of `fsm_step` (grep
      bar); the fsm implementation exists only in `arq-fsm.c`.
- [ ] Object+link build is warning-free under `gcc -Wall`; clang -O2 leg
      passes with byte-identical output vs gcc -O2.
- [ ] MC report bytes unchanged (existing parity assertions pass unmodified).
- [ ] Take-or-emit verified: pre-existing fsm pair is referenced without
      rewrite; partial pair is a usage error; stale hash warns (D14-4).
- [ ] `cargo test` (root) green; fmt clean; clippy at baseline.

## Non-goals
- No change to MC semantics, runtime flags, or report formats (parity-pinned).
- No Rust backend changes (task 5).
- No `rfs test` pipeline changes (the mc command is not part of `rfs test`).

## Other information

- Re-check at implementation start: whether any report struct is defined
  inside a function body or behind statics in c/mc.rs - if so, hoisting it to
  file scope for the header is part of this split (note it in the task status
  file).
- The smt-bounded mode generates a z3-linking variant of the mc artifact -
  verify its compile legs (if any test compiles smt-bounded C output) also
  migrate to object+link; the z3 link flags from TOOLCHAIN.md section 5.3 stay
  on the LINK step.
