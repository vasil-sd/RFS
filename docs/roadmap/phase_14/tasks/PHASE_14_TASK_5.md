# Phase 14, Task 5 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal
Make the Rust `mc` and `test` artifacts reference the separately generated fsm
file via `include!("BASE-fsm.rs")` instead of inlining a duplicate of the FSM
code, keep the fuzz-target skeleton's crate-wiring model (comments updated),
migrate the `rfs test` Rust unit pipeline to the two-file layout - and preserve
the fused `generate_core` contract so the 1683-test generated-code suite stays
byte-for-byte untouched.

```
rfs codegen rust mc spec.rfs --out arq
  -> arq-fsm.rs                 (co-emitted or taken)
  -> arq-mc.rs                  (include!("arq-fsm.rs") + engine + main)
rustc --edition 2021 arq-mc.rs -o arq_mc && ./arq_mc
```

## Context

- Rust `production` (`src/codegen/backends/rust/fsm.rs`): one self-contained
  .rs of flat crate-root items (enums, structs, impls, optionally
  `#[cfg(debug_assertions)]` monitor code). Its content is UNCHANGED by this
  phase - it becomes the referenced file.
- Rust `test` (`src/codegen/backends/rust/test.rs`): `generate_unit_tests`
  calls `rust::fsm::generate_core(module, opts, with_tests=true, Always)` which
  FUSES fsm + tests into one string; the separate `fuzz_target.rs` skeleton is
  wired by the user into their own cargo-fuzz project (`use crate::{...}`, lib
  root = the production source - lines ~719-723, ~793).
- Rust `mc` (`src/codegen/backends/rust/mc.rs`): self-contained executable .rs
  with the FSM inlined (same pattern as the C mc before task 3).
- The aggregate-crate path (`src/bin/generate_code_tests.rs`) calls
  `generate_core(..., with_tests=true, ...)` DIRECTLY and wraps each module as
  `pub mod mNNN_... { <fused code> }` - it never touches the CLI or artifact
  files. The fused contract of `generate_core` MUST be preserved for this path.
- The `rfs test` Rust unit pipeline (`src/cli/commands/test.rs`) runs
  `rustc --edition 2021 --test` on the single fused artifact in a temp dir; the
  Rust fuzz pipeline scaffolds a temp cargo-fuzz project whose lib root is the
  generated production source (content unchanged by this phase).
- Task 1 has landed: slots, take-or-emit, resolved-name plumbing (`Backend-
Options` carries the fsm reference name - for Rust this is the .rs file name
  used inside `include!`).

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 1. | Codegen output model + CLI flag family | Slots + take-or-emit + resolved fsm file name in options. |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 8. | Codegen architecture docs + dev/llm re-sync | Documents the include! mechanism. |
| 11. | Close-out | Full verification incl. the unchanged 1683-test suite. |

### Affected source files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `src/codegen/backends/rust/fsm.rs` | change | Refactor `Gen` to expose fsm-core and test-section renderings separately; `generate_core` keeps returning the FUSED string (aggregate-crate contract); new entry point for the CLI test artifact = include! + test section only. |
| `src/codegen/backends/rust/mc.rs` | change | mc artifact = `include!("<fsm file>")` + engine + main; no inlined FSM; fills mc slot, references fsm slot. |
| `src/codegen/backends/rust/test.rs` | change | CLI test artifact = include! + tests (via the new entry point); fuzz skeleton keeps crate-wiring, comments updated to the new naming. |
| `src/cli/commands/test.rs` | change | Rust unit pipeline: write fsm.rs + test.rs into the temp dir, `rustc --edition 2021 --test` on the test file (include! resolves relative to it). |

### Affected test files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `tests/codegen_test.rs` | change | Rust mc/test legs: include! present, fsm body absent from the dependent artifacts; compile + run the two-file layout. |

## Planned actions

### Action 5.1 - Split the fused rendering without breaking the aggregate contract

In `src/codegen/backends/rust/fsm.rs`: refactor `Gen` so the fsm-core items and
the test-section items are rendered into SEPARATE strings (today they are
appended to one `code` field in emission order). `generate_core` keeps its
signature and returns the FUSED string exactly as before (fsm core + test
section concatenated - byte-identical to today's output for the aggregate-
crate path). Add a new entry point (e.g. `generate_test_reference`) that
returns the test section prefixed with `include!("<fsm file name>");` (name
from `BackendOptions`) - this is what the CLI `test` command uses.

Rationale: D14-5 - `include!` at the crate root inlines the fsm items exactly
as if they were written there, so the test section needs ZERO body changes;
keeping `generate_core` fused preserves the 1683-test suite by construction.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/codegen/backends/rust/fsm.rs` | change (`Gen` emission) | Separate fsm-core / test-section strings; fused output unchanged. |
| `src/codegen/backends/rust/fsm.rs` | change (new entry point) | Test-section + include! rendering for the CLI path. |

### Action 5.2 - mc artifact references the fsm file

In `src/codegen/backends/rust/mc.rs`: remove the inlined FSM from the
executable body; the mc .rs starts with its line-1 artifact header comment,
`include!("<fsm file>");`, then the MC engine + `main()`. Fill the mc slot and
mark the fsm slot referenced (take-or-emit applies - a present `--fsm-file`
is used as-is, absent one is co-emitted).

Rationale: mirrors task 3's C split with the Rust-appropriate mechanism; the
FSM implementation exists exactly once per invocation.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/codegen/backends/rust/mc.rs` | change (executable-body emission) | Inlined FSM removed; include! added. |
| `src/codegen/backends/rust/mc.rs` | change (`generate_model_checker` return) | mc slot filled; fsm slot referenced. |

### Action 5.3 - test artifact + fuzz skeleton wiring

In `src/codegen/backends/rust/test.rs`: `generate_unit_tests` (the CLI path)
uses the new entry point from Action 5.1 - the test .rs is include! + test
section only. The `fuzz_target.rs` skeleton KEEPS its crate-wiring model (lib
root = production source, `use crate::{...}`); update its wiring comments to
the new naming (`BASE-fsm.rs` as the lib root, generation via the new flags).

Rationale: the fuzz target already references shared code through the crate
mechanism - only its documentation of the layout is stale.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/codegen/backends/rust/test.rs` | change (`generate_unit_tests`) | CLI test artifact = include! + tests. |
| `src/codegen/backends/rust/test.rs` | change (fuzz skeleton comments) | New naming in the wiring instructions. |

### Action 5.4 - Migrate the `rfs test` Rust unit pipeline

In `src/cli/commands/test.rs`: write the fsm .rs + test .rs into the temp dir
under their resolved names, then `rustc --edition 2021 --test` on the TEST file
(`include!` resolves relative to the including file - both files sit in the
temp dir). The Rust fuzz pipeline is unchanged in shape (lib root = production
source content) - verify only.

Rationale: the CI entry point must compile the two-file layout exactly as users
will.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/cli/commands/test.rs` | change (Rust unit pipeline) | Two-file temp dir + rustc on the test file. |

### Action 5.5 - Pin the reference model in tests

Update the Rust mc/test legs of `tests/codegen_test.rs`: assert the include! line
is present with the resolved file name, assert the fsm body is ABSENT from the
dependent artifacts (no duplicate `impl`/enum definitions - a grep bar), and
compile + run the two-file layout (mc binary runs and reports; test binary
passes). Confirm the aggregate-crate suite output is byte-identical to pre-task
(the 1683 tests unchanged).

Rationale: the no-duplication invariant and the preserved aggregate contract
are both load-bearing - each needs a standing check.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `tests/codegen_test.rs` | change (Rust mc/test legs) | Reference assertions + two-file compile/run. |

## Task deliverables
- Rust `mc`/`test` artifacts reference the fsm file via include!; no FSM
duplication.
- Fuzz skeleton comments updated; crate-wiring model preserved.
- `rfs test --backend rust` unit pipeline compiles the two-file layout.
- 1683-test generated-code suite byte-for-byte unchanged.

## Task exit criteria
- [ ] `rfs codegen rust mc <spec> --out arq` writes `arq-fsm.rs` + `arq-mc.rs`
      (include! present, fsm body absent from the mc file); the binary compiles
      with plain `rustc --edition 2021` and runs with unchanged reports.
- [ ] `rfs codegen rust test <spec> --out arq` writes three files; the test file
      is include! + tests only.
- [ ] `rfs test --backend rust` unit suite green under the two-file layout.
- [ ] `cargo test-generated-code` passes with the SAME 1683 tests (fused
      `generate_core` contract preserved - byte-identical aggregate output).
- [ ] Take-or-emit verified on the Rust side (present `--fsm-file` taken;
      absent co-emitted; stale hash warns per D14-4).
- [ ] `cargo test` (root) green; fmt clean; clippy at baseline.

## Non-goals
- No change to the Rust production artifact content.
- No C backend changes (tasks 2-4).
- No change to the fuzz target's crate-wiring model - comments only.

## Other information

- Re-check BEFORE relying on include!: the Rust production artifact must be
  flat crate-root items with no `mod` declarations and no inner `include!` -
  verify against a property-bearing spec (e.g. examples/valid/
  16_next_property.rfs) so inlining at the mc/test crate root cannot collide.
- If the test section references fsm items through paths that assume module
  nesting (it should not - today's fused output is flat), fix the rendering in
  Action 5.1, not the include mechanism.

## Task corrections

| Correction | Description + rationale |
|------------|-------------------------|
| Clippy lints in task-5 test code (found at close-out, task 11) | `tests/codegen_test.rs` lines 1595 and 1947: `std::fs::write(&rs, &rust_code)` / `std::fs::write(&path, &nostd_code)` trip clippy::needless_borrow - `rust_code`/`nostd_code` are already `&str` (returned by the slot-model `primary_code()`), so the extra `&` borrows a reference. Fixed by dropping the inner borrow; the written bytes are identical. Rationale: PROTOCOL.md section 3 - close-out records green, it does not manufacture it; the clippy baseline comparison at task 11 flagged both as new warnings introduced by this task's test code. |
