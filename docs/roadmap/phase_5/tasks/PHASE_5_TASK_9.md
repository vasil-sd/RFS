# Phase 5, Task 9 - Implementation details

> Written for a future LLM session: after reading `START.md`,
> `docs/roadmap/INTRO.md` and this file, the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal
Implement `--no-std` for the PRODUCTION codegen command (Rust): generated FSM
code becomes `#![no_std]` + `extern crate alloc` with an IDENTICAL public API
shape, so embedded teams can use RFS-generated state machines without std
(D9 explicitly deferred no_std to Phase 5). test/mc stay std; C accepts the
flag as a documented no-op.

## Context

- Scope decision (plan Context): PRODUCTION command only. The `test` and `mc`
  artifacts keep using std (allocation-heavy test/MC machinery, panics with
  debug info, etc.). Passing `--no-std` to test/mc -> codegen error with a
  clear message ("--no-std is only supported for the production command").
- Generated Rust today uses std heavily: `Vec<Output>`, `String`/`format!` in
  error messages, `Box`. Transformation rules (keep the API shape IDENTICAL -
  same types from the user's perspective):
  - `#![no_std]` + `extern crate alloc;`; `alloc::vec::Vec`, `alloc::string::String`,
    `alloc::boxed::Box` replace the std paths.
  - `format!` -> `alloc::format!` (macro import) or pre-built static strings
    where trivial.
  - Panics: keep them (core provides panic); no std-debug dependency.
  - No other std items may remain (the compile sweep proves it).
- C backend: plain C is already dependency-free -> the flag is ACCEPTED and
  documented as a no-op (CLI symmetry; parity of behavior is trivially kept).
- Verification: a COMPILE SWEEP test - for every valid example, generate
  production Rust with `--no-std` and compile it with `rustc --edition 2021
  --crate-type lib` (must succeed; this catches any leftover std item). Also
  assert the public API surface is unchanged: generate with and without
  `--no-std` and diff the non-header parts / or pin a structural assertion
  (same pub items) - decide the cheap robust variant in the action.

## Scope

### Depends on tasks

(none)

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 11. | ProtocolFsm trait + adapter crate + Rust example crates | includes the no_std-compatible minimal adapter that wraps these FSMs |

### Affected source files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `src/codegen.rs` | change | `no_std: bool` option (production-only validation) + CLI flag parsing |
| `src/backends/rust_fsm.rs` | change | no_std rendering path (header, alloc imports, error strings) |
| `src/backends/c_fsm.rs` | change | accept-and-noop handling + generated header comment noting it |

### Affected test files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `tests/codegen_test.rs` | change | flag parsing (production ok, test/mc rejected), no_std output shape pins, the 45-example compile sweep, API-surface equality check |

## Planned actions

### Action 1: flag wiring
`--no-std` in codegen CLI parsing; stored in BackendOptions; validation that it
applies only to `production` (error message pinned). C backend: accepted,
documented no-op (emitted comment in the artifact header).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/codegen.rs` | change | option + parsing + validation |
| `src/backends/c_fsm.rs` | change | no-op note emission |

### Action 2: rust_fsm no_std rendering
Apply the transformation rules from Context to the production renderer:
`#![no_std]` header, `extern crate alloc`, alloc-qualified collections/format,
static error strings where trivial. Keep every public item name/type identical.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/backends/rust_fsm.rs` | change | the no_std rendering path |

### Action 3: compile sweep + API equality tests
For all 45 valid examples: generate production with `--no-std`, compile with
rustc (lib) - must pass. API-surface check: pub items of no_std vs std output
are identical (structural assertion on the generated source or a pinned diff).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `tests/codegen_test.rs` | change | sweep + equality tests |

## Task deliverables
- `--no-std` production codegen (Rust core+alloc, identical API shape).
- 45-example no_std compile sweep + API-equality test.
- C no-op handling documented in the generated header.

## Task exit criteria
- [ ] All 45 valid examples' production output compiles under `#![no_std]` (sweep test green).
- [ ] Public API surface identical with/without `--no-std` (asserted).
- [ ] `--no-std` on test/mc rejected with the pinned error message; C accepts it as a no-op (header comment present).
- [ ] Default (std) output byte-identical to pre-task output for all examples (no accidental drift - parity suite green).
- [ ] Full main suite green, clippy/fmt clean.

## Non-goals
- no_std for test/mc artifacts (explicitly out of scope).
- Heapless/static-array variants (alloc is enough for v1; revisit in Phase 6 if requested).
- C-side changes beyond the no-op note.

## Other information
- If any example's production code uses a std item not covered by the rules
  (e.g. `std::collections`), extend the transformation rules - do NOT weaken
  the sweep to make it pass.
