# Phase 17, Task 9 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal
Bring the five EXISTING example projects to the shared README template (so all
15 projects read uniformly), and make `c_sync_handler`'s compiler choice an
documented, tested option (`make CC=clang`).

## Context
- The five existing projects: `sync_handler`, `async_handler` (cargo members,
spec 43 ARQ sender over channel/async adapters), `c_sync_handler` (Makefile-
driven C mirror), `fuzz_rust` (cargo-fuzz layout), `fuzz_c` (clang+libFuzzer
harness). All five already have READMEs with real content - this task
RESTRUCTURES them to the template, preserving every fact.
- Template (BINDING, PHASE_17.md "Proposed example README.md template"): What
  it demonstrates / The machine (mermaid stateDiagram ALWAYS) / Files layout /
  Build-run-test / Expected results. The current READMEs lack the mermaid
  diagrams and the uniform section structure.
- The spec-43 automaton (sliding-window ARQ sender: Idle -> Active on wake,
  window bound 4, out-of-band Acks dropped, Done + quiescence) is described in
  docs/user/EXAMPLES.md section 5 - reuse its diagram shape (stateDiagram-v2).
- `c_sync_handler`'s Makefile already has `CC ?= gcc` (honors the standard CC
  var per Phase 12 portability). This task makes the choice EXPLICIT and
documented: the header comment names `make CC=clang` as a supported alternative
(the generated C is compiler-agnostic - warning-free under both, per
`tests/c_compiler_agnostic_test.rs`), and a new availability-gated test leg
builds the scenario under clang and pins the same outcome.
- The README must also show the `RFS_GCC`/`RFS_CLANG` equivalents when driving
  rfs itself (codegen/test invocations resolve tools the same way -
  TOOLCHAIN.md section 8.2).
- Inventory rule: the new clang test fn gets a TEST_STRUCTURE row in the same
  commit (budget extended to 1430 by task 3).

## Scope

### Affected documentation files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `crates/examples/sync_handler/README.md` | change | restructured to the template + mermaid diagram |
| `crates/examples/async_handler/README.md` | change | restructured to the template + mermaid diagram |
| `crates/examples/fuzz_rust/README.md` | change | restructured to the template + mermaid diagram |
| `crates/examples/fuzz_c/README.md` | change | restructured to the template + mermaid diagram |
| `crates/examples/c_sync_handler/README.md` | change | template + gcc/clang choice + RFS_GCC/RFS_CLANG equivalents |

### Affected test files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `tests/example_projects_test.rs` | change | new clang leg for c_sync_handler (availability-gated) |

### Other affected files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `crates/examples/c_sync_handler/Makefile` | change | header documents `make CC=clang` as a supported choice |
| `docs/llm/TEST_STRUCTURE.md` | change | row for the new clang test fn (same commit) |

## Planned actions

### Restructure the five READMEs to the template
For each of the five: keep every existing fact (scenario description, pinned
outcomes, build commands), reorganize into the template's five sections, add a
mermaid `stateDiagram-v2` of the spec-43 automaton (the fuzz examples diagram
the same FSM + note the fuzz-target relationship), and add the Files layout
table. Content is PRESERVED - this is a restructuring, not a rewrite; if a fact
is missing that the template requires (e.g. rationale), add it from the existing
README's own text or the phase-5 history in TASKS.md (`grep -n "task 1[12]"
TASKS.md` for the original descriptions).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `crates/examples/sync_handler/README.md` | change | uniform structure + diagram across all 15 projects |
| `crates/examples/async_handler/README.md` | change | same |
| `crates/examples/fuzz_rust/README.md` | change | same |
| `crates/examples/fuzz_c/README.md` | change | same |
| `crates/examples/c_sync_handler/README.md` | change | same + the compiler-choice documentation |

### Document + test the c_sync_handler gcc/clang choice
Makefile header: name `make CC=clang` explicitly as a supported alternative
(the generated C compiles warning-free under both - the compiler-agnosticism
guarantee). README Build-run-test section: show BOTH builds (`make` and `make
CC=clang`) + the `RFS_GCC`/`RFS_CLANG` env equivalents for driving rfs codegen/
test. New test leg: build c_sync_handler under clang (availability-gated via
`tests/common/toolchain.rs`), assert warning-free + the SAME pinned scenario
outcome as the gcc leg.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `crates/examples/c_sync_handler/Makefile` | change (header comment) | the choice must be discoverable without reading TOOLCHAIN.md |
| `tests/example_projects_test.rs` | change | the clang leg pins the compiler-agnosticism guarantee for this project |
| `docs/llm/TEST_STRUCTURE.md` | change | same-commit row for the new fn |

## Task deliverables
- Five template-conformant READMEs with mermaid diagrams.
- c_sync_handler's documented gcc/clang choice + pinned clang test leg.

## Task exit criteria
- [ ] All five READMEs have the template's five sections in order, each with a
      mermaid stateDiagram and a Files layout table.
- [ ] No existing fact was lost (diff the information content, not the words).
- [ ] `make CC=clang` builds c_sync_handler warning-free (verified by the new
      gated leg when clang is present; documented either way).
- [ ] The README shows the RFS_GCC/RFS_CLANG equivalents.
- [ ] `cargo test --test example_projects_test` + `--test docs_md_test` +
      `--test dev_docs_test` green.
- [ ] Full checklist: `cargo build && cargo test && cargo clippy --all-targets
      && cargo fmt --check`.

## Non-goals
- No changes to the example sources (lib.rs/main.rs/handler.c/build.rs stay as
  is).
- No new examples (tasks 6-8 cover those).
