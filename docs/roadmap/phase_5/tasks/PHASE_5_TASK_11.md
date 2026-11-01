# Phase 5, Task 11 - Implementation details

> Written for a future LLM session: after reading `START.md`,
> `docs/roadmap/INTRO.md` and this file, the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal
Ship the Rust-ecosystem integration layer: the `ProtocolFsm` trait + generic
adapters (sync channel-based, futures-based async, callback, no_std-minimal) in
a new workspace crate, OPTIONAL emission of the trait impl by codegen
(`--with-protocol-fsm`), and two example crates (sync handler, async handler)
demonstrating a real protocol end to end.

## Context

- Design constraint (plan Context): generated code stays self-contained. The
  trait therefore lives in `crates/rfs-adapters`; codegen optionally emits
  `impl ProtocolFsm for Fsm { ... }` when `--with-protocol-fsm` is passed
  (OFF by default - the plain surface is unchanged). The emitted impl references
  the crate by name (`use rfs_adapters::ProtocolFsm;`) - documented as the one
  opt-in that links a generated artifact to an external crate.
- Trait shape (from PHASE_5.md): associated types Input/Output/Error + `new()`
  + `step(&mut self, input) -> Result<Vec<Output>, Error>`. The generated
  `Fsm` already matches this shape (concrete types instead of associated ones) -
  the emitted impl is mechanical.
- Adapters (all GENERIC over `F: ProtocolFsm`, no dependency on generated code):
  - sync channel-based: std mpsc sender/receiver around step;
  - futures async: `futures`-based stream/sink adapter (the NEW external dep,
    confined to this crate + the async example);
  - callback-based: user-supplied `FnMut(input) -> ...` driving loop;
  - no_std-minimal: pure step-forwarding wrapper with no alloc/std (works with
    task 9's no_std FSMs).
- Example crates (workspace members under `crates/examples/`): `sync_handler`
  and `async_handler`, both built on spec 43 (sliding-window ARQ) generated at
  build time via rfs-build (task 1) - they demonstrate the full integration
  story: build.rs -> generated FSM -> adapter -> a small protocol driver.
- Rust-ecosystem-only by design (parity principle): documented as such; the C
  equivalent surface is the plain-C step API + task 11's C example projects.

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 1. | Workspace conversion + rfs-build crate | the new crate + examples are workspace members; examples use rfs-build in build.rs |
| 9. | no_std production codegen (`--no-std`) | the no_std-minimal adapter targets these FSMs (compile-checked against a no_std example) |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 12. | C example projects + fuzzing harness examples | mirrors the Rust examples; shares the crate layout conventions |

### Affected source files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `crates/rfs-adapters/Cargo.toml` | new | adapter crate manifest (deps: futures [optional], no others required) |
| `crates/rfs-adapters/src/lib.rs` (+ module files) | new | ProtocolFsm trait + the four adapters |
| `src/codegen.rs` | change | `--with-protocol-fsm` flag (production/test; mc rejected or allowed - decide: production+test only, pinned) |
| `src/backends/rust_fsm.rs` | change | optional impl emission block |

### Other affected files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `crates/examples/sync_handler/` (Cargo.toml, build.rs, src/) | new | sync example on spec 43 via rfs-build + channel adapter |
| `crates/examples/async_handler/` (Cargo.toml, build.rs, src/) | new | async example on spec 43 via rfs-build + futures adapter |

### Affected test files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `crates/rfs-adapters/tests/` (new) | new | adapter unit tests over a small in-crate mock FSM (no generated code needed): channel round-trip, async step ordering, callback loop, no_std-minimal under `#![no_std]` cfg test |
| `tests/codegen_test.rs` | change | `--with-protocol-fsm` emission pins + rejection on mc |

## Planned actions

### Action 1: rfs-adapters crate (trait + adapters)
Implement the trait per PHASE_5.md and the four generic adapters; unit tests
over a mock FSM. Keep the crate small - one module per adapter.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `crates/rfs-adapters/Cargo.toml` | new | manifest |
| `crates/rfs-adapters/src/lib.rs` (+ modules) | new | trait + adapters |
| `crates/rfs-adapters/tests/` (new) | new | mock-FSM adapter tests |

### Action 2: codegen impl emission
`--with-protocol-fsm` flag; emitted impl block (mechanical mapping of the
generated Input/Output/StepError types); pinned output test; rejected for `mc`.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/codegen.rs` | change | flag |
| `src/backends/rust_fsm.rs` | change | emission block |
| `tests/codegen_test.rs` | change | pins |

### Action 3: example crates
sync_handler + async_handler on spec 43: build.rs via rfs-build, a short
protocol driver (send/receive loop exercising the ARQ), README with build/run
instructions. Both must build + run in the workspace (`cargo test -p` smoke).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `crates/examples/sync_handler/` (new dir) | new | the sync example |
| `crates/examples/async_handler/` (new dir) | new | the async example |
| `Cargo.toml` | change | workspace members list |

## Task deliverables
- `rfs-adapters` crate: ProtocolFsm + 4 generic adapters, tested.
- `--with-protocol-fsm` codegen option (pinned emission).
- sync_handler + async_handler example crates building and running on spec 43.

## Task exit criteria
- [ ] Adapter unit tests green (channel/async/callback/no_std-minimal over the mock FSM).
- [ ] `--with-protocol-fsm` output pinned; default output unchanged; mc rejected with a clear message.
- [ ] Both example crates build in the workspace and their protocol driver runs to completion (exit 0) - recorded as tests or documented manual steps if CI-infeasible (prefer tests).
- [ ] no_std-minimal adapter compiles against a task-8 no_std generated FSM (compile check).
- [ ] Full main suite green, clippy/fmt clean (workspace-wide).

## Non-goals
- C-side adapters (plain-C step API + task 11 examples are the C surface).
- Procedural macros ("later" per PHASE_5.md).
- tokio-specific adapters (futures covers the async need; tokio interop is user code).

## Other information
- `futures` version: pick a stable release; it must not leak into the `rfs`
  toolchain package or generated code.
