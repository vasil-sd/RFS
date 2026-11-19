# Phase 17, Task 7 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal
Create the three new WORKSPACE-MEMBER example crates that demonstrate usage
scenarios no current example covers: `--no-std` production codegen, production
property monitors (`--with-property-monitors`), and the two never-demonstrated
rfs-adapters (callback driver + no_std minimal wrapper).

## Context
- Conventions (from `crates/examples/sync_handler/`): `Cargo.toml` with
  `[dependencies] rfs-adapters = { path = "../../rfs-adapters" }` and
  `[build-dependencies] rfs-build = { path = "../../rfs-build" }`; `build.rs`
  generates the artifact via rfs-build, strips its inner attributes (which
  `include!` rejects) and writes the byte-identical body to OUT_DIR; `src/
  lib.rs` pulls it in via `#[path]` include of the OUT_DIR body file with a
  module-level `#[allow]` mirroring the artifact header; an inline
  `#[cfg(test)]` test drives the session; optionally a `src/main.rs` demo
  binary (exit 0 on success). The root `Cargo.toml` `[workspace] members`
  list must gain each new crate.
- rfs-build options relevant here: `.no_std(true)` (maps to production-only
  `--no-std`), `.with_property_monitors(true)`, `.with_protocol_fsm(true)`
  (needed when the generated Fsm must implement `rfs_adapters::ProtocolFsm`).
- no_std pattern reference: `crates/rfs-adapters/tests/nostd_minimal.rs`
  compiles a REAL `--no-std` generated FSM inside a `#![no_std]` crate (rustc-
  gated). The generated no_std artifact is `#![no_std]` + `extern crate alloc`
  with an IDENTICAL public API to the std build.
- Monitor reference material: `examples/valid/41_property_monitors.rfs` + the
  buggy-variant pattern from `tests/monitor_integration_test.rs` (drop the
  Ack-handling transition so `fin_acked_bounded` must expire). Rust gates
  monitor emission/stepping on `#[cfg(debug_assertions)]` (zero cost in
  release); C gates on `-DRFS_PROPERTY_MONITORS`.
- Adapter API: read `crates/rfs-adapters/src/lib.rs` + the callback/minimal
  modules for the actual type names (`CallbackFsm`, the minimal wrapper) and
  their test patterns in `crates/rfs-adapters/tests/adapters.rs` before writing
  the drivers.
- Inventory rule: every new fn/type in `crates/*/src/` gets a SRC_STRUCTURE.md
  row, every new test fn a TEST_STRUCTURE.md row - SAME commit (budgets already
  extended by task 3: SRC 1830 / TEST 1430). Use `rfs-dev inventory add`
  (dev_tools/SKILL.md) or match the existing row format exactly; run `rfs-dev
  inventory audit` at the end.
- Test legs go in `tests/example_projects_test.rs` (build + run each member via
  cargo, like the existing sync_handler leg).

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 3. | Fix stale LLM set + KB.md + extend inventory budgets | SRC/TEST_STRUCTURE headroom must exist before new rows are appended |

### Affected source files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `crates/examples/no_std_fsm/{Cargo.toml,build.rs,src/lib.rs}` | new | the no_std production example crate |
| `crates/examples/monitors_production/{Cargo.toml,build.rs,src/lib.rs}` | new | the production-monitors example crate |
| `crates/examples/callback_minimal_adapters/{Cargo.toml,build.rs,src/lib.rs}` | new | the callback + minimal adapter example crate |
| `Cargo.toml` | change | workspace members list gains the three crates |

### Affected test files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `tests/example_projects_test.rs` | change | one build+run leg per new member |

### Other affected files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `crates/examples/no_std_fsm/README.md` | new | template README |
| `crates/examples/monitors_production/README.md` | new | template README |
| `crates/examples/callback_minimal_adapters/README.md` | new | template README |
| `docs/llm/SRC_STRUCTURE.md` | change | rows for every new fn/type (same commit) |
| `docs/llm/TEST_STRUCTURE.md` | change | rows for the inline tests + test legs (same commit) |

## Planned actions

### 7.1 - no_std_fsm
build.rs generates a production artifact with `.no_std(true)` (pick a small
spec - e.g. `examples/valid/03_timeout_retry.rfs` or similar; the README must
describe the automaton). The crate lib is `#![no_std]` + `extern crate alloc`,
includes the generated body, and re-exposes the FSM API. A std `#[cfg(test)]`
test drives a full session through the no_std artifact (proving the identical
API works from a std context). README per the template: what `--no-std` buys
(no_std targets, identical public API), the automaton + mermaid diagram, files
layout, build/run/test, expected results.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `crates/examples/no_std_fsm/Cargo.toml` | new | member manifest (rfs-build build-dep; NO rfs-adapters dep needed) |
| `crates/examples/no_std_fsm/build.rs` | new | generate with `.no_std(true)` + strip inner attributes |
| `crates/examples/no_std_fsm/src/lib.rs` | new | `#![no_std]` lib including the artifact + std test driver |
| `crates/examples/no_std_fsm/README.md` | new | template README |

### 7.2 - monitors_production
build.rs generates production with `.with_property_monitors(true)` from
`examples/valid/41_property_monitors.rfs`. The lib drives TWO sessions: the
correct spec (no violation, monitors quiet) and the buggy variant (Ack
transition dropped - construct it in-test by driving a violating input sequence
through the generated FSM, per `tests/monitor_integration_test.rs`'s pattern)
where the debug-gated monitor surfaces the violation at the step API. README:
what production monitors are (debug-only runtime property checking, zero cost
in release), the automaton + property structure as mermaid diagrams, the C leg
note (`-DRFS_PROPERTY_MONITORS`), expected results (the exact violation label/
step the monitor reports).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `crates/examples/monitors_production/Cargo.toml` | new | member manifest |
| `crates/examples/monitors_production/build.rs` | new | generate with `.with_property_monitors(true)` |
| `crates/examples/monitors_production/src/lib.rs` | new | passing + violating session drivers + tests |
| `crates/examples/monitors_production/README.md` | new | template README |

### 7.3 - callback_minimal_adapters
build.rs generates the FSM with `.with_protocol_fsm(true)`. The lib drives a
real generated spec through `CallbackFsm` (completion + error callbacks) and
through the no_std `MinimalFsm` wrapper (forwarding/composition) - read
`crates/rfs-adapters/tests/adapters.rs` for the exact API shapes first. README:
what each adapter is for (callback-style integration without channels/streams;
the minimal wrapper for no_std composition), automaton + diagram, expected
results.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `crates/examples/callback_minimal_adapters/Cargo.toml` | new | member manifest (rfs-adapters dep) |
| `crates/examples/callback_minimal_adapters/build.rs` | new | generate with `.with_protocol_fsm(true)` |
| `crates/examples/callback_minimal_adapters/src/lib.rs` | new | callback + minimal drivers + tests |
| `crates/examples/callback_minimal_adapters/README.md` | new | template README |

### Workspace registration + test legs + inventory rows
Add the three crates to the root `Cargo.toml` `[workspace] members`. Add one
build+run leg per crate to `tests/example_projects_test.rs` (cargo build -p +
cargo test -p, like the existing sync_handler leg). Append SRC_STRUCTURE rows
for every new fn/type and TEST_STRUCTURE rows for every new test fn in the same
commit; finish with `rfs-dev inventory audit`.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `Cargo.toml` | change (members list) | the crates must be workspace members to build/test in CI |
| `tests/example_projects_test.rs` | change | 3 new legs - the examples are part of the verified surface |
| `docs/llm/SRC_STRUCTURE.md` | change | same-commit inventory rows for all new source items |
| `docs/llm/TEST_STRUCTURE.md` | change | same-commit inventory rows for all new test items |

## Task deliverables
- Three new workspace-member example crates, each with a template README.
- Workspace registration + three test legs + complete inventory rows.

## Task exit criteria
- [ ] `cargo build --workspace` and `cargo test --workspace` green (the new
      members' tests pass).
- [ ] Every README follows the template (all five sections; mermaid diagram
      present).
- [ ] no_std_fsm's lib is genuinely `#![no_std]` (verify the artifact header +
      the crate attribute) and its API matches the std build.
- [ ] monitors_production's violating session reports the monitor violation in
      debug mode with the pinned label/step.
- [ ] callback_minimal_adapters exercises BOTH previously-undemonstrated
      adapters against a real generated spec.
- [ ] `rfs-dev inventory audit` clean; `cargo test --test dev_docs_test` green.
- [ ] Full checklist: `cargo build && cargo test && cargo clippy --all-targets
      && cargo fmt --check`.

## Non-goals
- No changes to rfs-adapters or rfs-build themselves (the examples consume the
  existing API).
- No C-side example crates (the C leg is documented as a note, per the plan).
