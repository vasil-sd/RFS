# Phase 18, Task 7 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start implementation of the task immediately without
> re-deriving scope.

## Goal
Promote three Phase 18 FSMs to WORKSPACE-MEMBER crates that demonstrate production
code generation AND integration on REAL machines (not just the Phase 17 teaching
specs): `tcp_full_states` over rfs-adapters, `bms_protection` with property monitors,
and `uart_i2c_line` in a genuinely `#![no_std]` crate. Rationale: this is the only way to
show the production-integration surface (adapters / monitors / no_std) applied to a
recognizable real protocol/controller.

## Context
- Depends on tasks 1, 3 and 5 for the three specs (`tcp_full_states`, `bms_protection`,
  `uart_i2c_line`). Reuse their `pass.rfs` content as the crate's generated spec (the
  crate's `build.rs` generates it via rfs-build from a `.rfs` checked into the crate dir).
- Follow the Phase 17 cargo-member example pattern exactly (`no_std_fsm`,
  `monitors_production`, `callback_minimal_adapters`): each crate has `Cargo.toml`
  (rfs-build build-dep, plus rfs-adapters where needed), a `build.rs` that generates the
  spec with the relevant flag and strips inner attributes into OUT_DIR, and a `src/lib.rs`
  that includes the artifact and drives it, with inline tests.
- The three integrations:
  - `tcp_full_states`: generate with `.with_protocol_fsm(true)`; drive a TCP-like session
    over ALL FOUR rfs-adapters (channel round-trip/ordering + error survival, async stream
    ordering/Pending-until-close/error carry-through, callback completion/error/step-limit,
    minimal forwarding/composition). This is the real-protocol analog of Phase 17's
    `callback_minimal_adapters`.
  - `bms_protection`: generate with `.with_property_monitors(true)`; a correct session keeps
    the monitors quiet, an injected over-voltage violation is surfaced by the debug-gated
    monitor at the step API (`#[cfg(debug_assertions)]`), and the release twin
    (`#[cfg(not(debug_assertions))]`) is zero-cost. Real-protocol analog of Phase 17's
    `monitors_production`.
  - `uart_i2c_line`: generate with `.no_std(true)`; compile in a genuinely `#![no_std]` +
    `extern crate alloc` crate with an identical public API to the std build. Real-protocol
    analog of Phase 17's `no_std_fsm`.
- Test-leg conventions: nested-cargo build+test legs in `tests/example_projects_test.rs`
  (the `cargo_member` helper from Phase 17 task 7), availability-gated, scratch under
  `target/`. Inventory rule: a TEST_STRUCTURE.md row per new test fn in the same commit.

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 1. | Protocol FSMs A1-A10 | provides the `tcp_full_states` spec |
| 3. | Control FSMs B1-B8 | provides the `bms_protection` spec |
| 5. | Other FSMs C1-C8 | provides the `uart_i2c_line` spec |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 8. | EXAMPLES.md "Other examples" section | the three crates' READMEs are linked there |
| 10. | Close-out | verifies the new members build/test green |

### Affected source files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `crates/examples/tcp_full_states/{Cargo.toml,build.rs,src/lib.rs}` | new | adapters integration crate |
| `crates/examples/bms_protection/{Cargo.toml,build.rs,src/lib.rs}` | new | property-monitors integration crate |
| `crates/examples/uart_i2c_line/{Cargo.toml,build.rs,src/lib.rs}` | new | no_std integration crate |
| `Cargo.toml` | change | workspace members list +3 |

### Affected test files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `tests/example_projects_test.rs` | change | 3 nested-cargo build+test legs |
| `docs/llm/TEST_STRUCTURE.md` | change | rows for the new test fns (same commit) |

## Planned actions

### 7.1 - tcp_full_states adapters crate
Copy the task-1 `pass.rfs` into the crate dir. `build.rs` generates it with
`.with_protocol_fsm(true)` and strips inner attributes into OUT_DIR. `src/lib.rs` includes
the artifact in a `pub mod`, then drives a full TCP-like session (connect -> exchange data ->
clean teardown) over each of the four rfs-adapters, with inline tests asserting ordering,
error survival, and completion. Rationale: shows a real protocol's generated FSM is
drivable through every adapter shape.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `crates/examples/tcp_full_states/Cargo.toml` | new | rfs-build build-dep + rfs-adapters dep |
| `crates/examples/tcp_full_states/build.rs` | new | generate with `.with_protocol_fsm(true)`; strip inner attrs to OUT_DIR |
| `crates/examples/tcp_full_states/src/lib.rs` | new | drive the session over channel/async/callback/minimal adapters + tests |

### 7.2 - bms_protection monitors crate
Copy the task-3 `pass.rfs`. `build.rs` generates with `.with_property_monitors(true)`.
`src/lib.rs`: `run_correct_session` (happy path, monitors quiet), `run_violating_session`
(`#[cfg(debug_assertions)]`, inject an over-voltage and assert the monitor fires at the step
API), `run_unmonitored_session` (`#[cfg(not(debug_assertions))]`, the release twin). Rationale:
shows the debug-gated property monitor catching a REAL controller violation, zero-cost in
release.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `crates/examples/bms_protection/Cargo.toml` | new | rfs-build build-dep only |
| `crates/examples/bms_protection/build.rs` | new | generate with `.with_property_monitors(true)`; strip inner attrs to OUT_DIR |
| `crates/examples/bms_protection/src/lib.rs` | new | correct / violating (debug) / unmonitored (release) sessions + tests |

### 7.3 - uart_i2c_line no_std crate
Copy the task-5 `pass.rfs`. `build.rs` generates with `.no_std(true)`. `src/lib.rs` is a
genuinely `#![no_std]` + `extern crate alloc` lib that includes the artifact and runs a bit-
level frame session, with a std-context test. Rationale: shows a real bit-level protocol's
generated FSM compiles and runs in a no_std target with an identical public API.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `crates/examples/uart_i2c_line/Cargo.toml` | new | rfs-build build-dep only |
| `crates/examples/uart_i2c_line/build.rs` | new | generate with `.no_std(true)`; strip inner attrs to OUT_DIR |
| `crates/examples/uart_i2c_line/src/lib.rs` | new | `#![no_std]` lib driving a frame session + std-context test |

### 7.4 - Workspace members + test legs + inventory
Add the three crates to the root `Cargo.toml` workspace members list. Add one nested-cargo
build+test leg per crate to `tests/example_projects_test.rs` (using the Phase 17
`cargo_member` helper; bms_protection additionally runs its release suite). Append a
TEST_STRUCTURE.md row per new test fn in the same commit.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `Cargo.toml` | change | +3 workspace members |
| `tests/example_projects_test.rs` | change | 3 nested-cargo build+test legs |
| `docs/llm/TEST_STRUCTURE.md` | change | one row per new fn (same-commit inventory rule) |

## Task deliverables
- Three cargo-member integration crates (adapters / property-monitors / no_std) over real
  Phase 18 machines.
- Root workspace members +3; three nested-cargo test legs + inventory rows.

## Task exit criteria
- [ ] All three crates build and their inline tests pass under `cargo test --workspace`.
- [ ] tcp_full_states drives a session over all four adapters (ordering + error survival asserted).
- [ ] bms_protection's debug-gated monitor catches the injected over-voltage; the release twin compiles zero-cost.
- [ ] uart_i2c_line compiles in a genuinely `#![no_std]` crate with an identical public API.
- [ ] TEST_STRUCTURE rows added; `cargo test --test dev_docs_test` green.
- [ ] Full checklist: `cargo build && cargo test && cargo test --workspace && cargo clippy --all-targets && cargo fmt --check`.

## Non-goals
- No changes to the three FSMs' plain-dir examples (those stay from tasks 1/3/5).
- No new RFS features - the crates exercise the EXISTING adapter/monitor/no_std surface.
