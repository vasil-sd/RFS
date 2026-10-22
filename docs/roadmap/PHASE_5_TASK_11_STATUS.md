# Phase 5, Task 11 - Implementation status

> Written once when all exit criteria of `docs/roadmap/PHASE_5_TASK_11.md` are
> reached. Not rewritten afterwards; later fixes go into TASKS.md entries and
> follow-up phase plans.

## Added source files

| Name | Description |
|---|---|
| `crates/rfs-adapters/Cargo.toml` | Adapter crate manifest: `#![no_std]` core, features `std` (default) + `async` (implies std, adds the OPTIONAL `futures` 0.3 dep - the only external dependency of the workspace beyond the root package; it never leaks into the `rfs` toolchain or generated code). Dev-dep on the `rfs` crate for the no_std compile check's generation step only. |
| `crates/rfs-adapters/src/lib.rs` | The `ProtocolFsm` trait: associated types `Input`/`Output`/`Error` + `new()` + `step(&mut self, input) -> Result<Vec<Output>, Error>` - the exact shape of every generated Rust FSM. Module root; feature-gated module declarations. |
| `crates/rfs-adapters/src/channel.rs` | Sync channel adapter (`std` feature): `ChannelFsm::spawn()` runs the FSM on its own thread; inputs/outcomes cross std mpsc channels, FIFO order preserved; a failed step delivers `ChannelOutcome::Error` and does NOT stop the thread. Manual `Debug`/`PartialEq` bounded on the associated types (generated `Fsm` derives neither). |
| `crates/rfs-adapters/src/async_adapter.rs` | Futures stream adapter (`async` feature): `AsyncFsm` = input queue + `Stream` of `AsyncOutcome`; each `poll_next` steps at most one queued input synchronously (generated `step` does no I/O), so ANY executor works - no tokio. Empty queue -> `Pending` until `close()`, then the stream ends. |
| `crates/rfs-adapters/src/callback.rs` | Callback driver (core-only): `drive(fsm, decide, max_steps)` - the closure sees the PREVIOUS step's outputs and returns the next input or `None`; result is `Completed` / `Failed(err, steps_done)` / `StepLimit`. |
| `crates/rfs-adapters/src/minimal.rs` | no_std-minimal wrapper (core-only): `Minimal<F>` pure step-forwarding with no state of its own; itself a `ProtocolFsm`, so adapters compose. Works with the task-9 `#![no_std]` production artifacts. |
| `crates/examples/sync_handler/` (Cargo.toml, build.rs, src/lib.rs, src/main.rs, README.md) | Example crate: spec-43 sliding-window ARQ driven from another thread over std mpsc channels via `ChannelFsm`. build.rs generates the FSM at build time via rfs-build with `--with-protocol-fsm` and strips the inner attributes into an OUT_DIR body file; lib.rs pulls it in via `include!` under a mirroring outer `#[allow]`. The driver (wake tick, ticks until 8 frames sent + acked with an out-of-band `Ack` injected every third tick, completion + quiescence) is a library function returning an `ArqReport`, recorded as a TEST and exposed as a demo binary (exit 0 on success). |
| `crates/examples/async_handler/` (same layout) | Mirror example over the futures stream adapter (`AsyncFsm` + `futures::executor::block_on`; no tokio). Same scenario, identical observable results (19 ticks, in-order frames/acks `[0..8]`, 5 dropped out-of-band acks, max window exactly 4). |

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| `Cargo.toml` | changed | Workspace members: explicit list `["crates/rfs-build", "crates/rfs-adapters", "crates/examples/*"]` (the bare `crates/*` glob breaks once the manifest-less `crates/examples` dir exists - C11-2). |
| `Cargo.lock` | changed | +`futures` 0.3 and its transitive deps (workspace-only; not a dependency of the root package). |
| `src/codegen.rs` | changed | `BackendOptions.with_protocol_fsm` (default false) + `CodegenRequest::Generate` field + `--with-protocol-fsm` parsing: accepted for production/test, rejected for `mc` with the new pinned `PROTOCOL_FSM_MC_ERROR`. |
| `src/main.rs` | changed | Destructure + pass the new option through to `BackendOptions`. |
| `src/backends/rust_fsm.rs` | changed | `emit_protocol_fsm_impl()`: the opt-in block, emitted after the inherent `impl Fsm` (before the `#[cfg(test)]` module in test artifacts) when the flag is on. The impl forwards explicitly (`Fsm::new()` / `Fsm::step(self, input)`) so it cannot recurse into itself. |
| `src/backends/rust.rs` | changed | `production`/`test` usage + options text document the flag (opt-in; links the artifact to rfs-adapters). |
| `src/backends/rust_mc.rs` | changed | Library-level guard: `with_protocol_fsm` -> `PROTOCOL_FSM_MC_ERROR` (the mc keeps its own runtime). |
| `src/backends/c_mc.rs` | changed | Same library-level guard added for parity (C11-4) - the CLI parser already rejected the flag for both backends' mc; this closes the direct-library-call path. |
| `crates/rfs-build/src/lib.rs` | changed | `.with_protocol_fsm(bool)` builder option (maps to `--with-protocol-fsm`; argv-pinned test). PRE-EXISTING DOC FIX (C11-1): the module-doc consumer example showed `pub mod x { include!(...) }`, which does NOT compile against real generated artifacts - inner attributes are rejected inside a macro expansion; corrected to the pattern the examples actually use. |

## Added/changed/extended test files

| Name | Operation | Description |
|---|---|
| `crates/rfs-adapters/tests/adapters.rs` | new | 10 tests over an in-crate mock FSM (no generated code): minimal forwarding/unwrap + composition through the callback driver; callback completion-on-None / error-with-steps-done / step-limit; channel round-trip order preservation + error-outcome survival; async stream outcome ordering / Pending-until-close / error carry-through. |
| `crates/rfs-adapters/tests/nostd_minimal.rs` | new | 2 no_std compile checks (rustc-gated, like the toolchain's other compile checks): (1) a `#![no_std]` harness using minimal + callback over an in-file mock; (2) the SAME against a REAL task-9 `--no-std` generated FSM produced here via the rfs library with `--with-protocol-fsm` - the exit-criterion compile check. |
| `tests/codegen_test.rs` | changed | +4 tests: `cli_protocol_fsm_flag_parsing` (production/test accept on both backends, default off, mc rejected with the pinned message), `cli_protocol_fsm_e2e_exit_codes_and_output` (production exit 0 + block on stdout; mc exit 1 + pinned stderr), `protocol_fsm_emission_pins_and_default_unchanged` (byte pin: flag-on output = default output + the exact block once, after the inherent impl, before `#[cfg(test)]`; default has no trace of the trait; C backend byte-identical ignore), `mc_entry_points_reject_protocol_fsm` (both mc entry points at library level). |
| `crates/examples/*/src/lib.rs` | new | Each example carries its full ARQ session as a test (`arq_session_completes_over_channels` / `..._over_the_stream`) - the CI record for "builds and runs to completion". |

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|
| `CODEGEN.md` | changed | 3.1: `BackendOptions.with_protocol_fsm` + the `PROTOCOL_FSM_MC_ERROR` paragraph + a dedicated `--with-protocol-fsm` block (semantics, emission point, forwarding rationale, C ignore, byte pin); 4.1: request struct/defaults/parse errors; 4.3: example line; 9: invariant 1 gains the ONE documented exception. |
| `BUILD.md` | changed | File-map rows for rfs-adapters + examples (incl. the include!/inner-attribute pattern); counts re-pinned (root 354 -> 358, workspace 362 -> 381, smt 356 -> 360; codegen 45 -> 49, rfs-build 8 -> 9) + new rfs-adapters row; verification checklist gains the adapter/example check; known-good baseline updated (after Phase 5 task 11). |
| `KB.md` | changed | 4.1: dependencies corrected to current reality (serde/serde_json were removed in task 2 - pre-existing staleness fixed while documenting the new `futures` dep and its confinement); 4.2: workspace-crate rows + main.rs command list updated; 4.5: protocol-FSM opt-in paragraph. |
| `docs/roadmap/INTRO.md` | changed | Phase-state row 5 -> "tasks 1-11 of 16 done". |

## Other file changes

- `TASKS.md`: one new entry.

## Changed functionality

- None to default output: without the flag, every artifact is byte-identical (the emission block is gated; the generated-code sweep 1453/1453 passes unchanged; all pre-existing pins pass unmodified). The C backend accepts and ignores the flag with byte-identical output (pinned).

## Removed functionality

- None.

## New functionality

- `rfs-adapters` crate: `ProtocolFsm` trait + four generic adapters (sync channel, futures stream, callback, no_std-minimal), all generic over `F: ProtocolFsm`, none depending on generated code; `#![no_std]` core with optional `std`/`async` features.
- `rfs codegen rust production|test ... --with-protocol-fsm`: opt-in emission of `impl ProtocolFsm for Fsm` - the one documented opt-in linking a generated artifact to an external crate; `mc` rejects it (pinned message, CLI + library level, both backends); C ignores it.
- `rfs-build` `.with_protocol_fsm(bool)` for build.rs consumers.
- Two example crates demonstrating the full integration story on spec 43 (build.rs -> generated FSM with the trait impl -> adapter -> protocol driver), each recorded as a test and runnable as a demo binary.

## Task corrections

| Correction | Description + rationale |
|------------|-------------------------|
| C11-1: rfs-build's documented `include!` consumer pattern does not compile against real generated code (pre-existing, surfaced by the first real consumer) | The task-1 module doc showed `pub mod x { include!(concat!(env!("OUT_DIR"), ...)) }`, but rustc rejects inner attributes inside a macro expansion (`error: an inner attribute is not permitted in this context`) - and every generated artifact starts with `#![allow(...)]` (plus `#![no_std]` under --no-std). The rfs-build unit tests never caught it because they use a stub binary writing arbitrary content. `#[path]` was the first candidate but accepts only string literals (no OUT_DIR reference). Shipped pattern: build.rs strips the inner-attribute lines and writes the byte-identical body to OUT_DIR; lib.rs uses `include!` under an outer `#[allow]` that mirrors the artifact header exactly. Bonus: the body file stays outside the source tree, so `cargo fmt` never sees generated code (rustfmt would reformat ~400 lines of it). rfs-build docs corrected. |
| C11-2: workspace members switched from glob to explicit list | `members = ["crates/*"]` matches the manifest-less `crates/examples` directory once it exists -> `error: failed to load manifest for workspace member`. Explicit `["crates/rfs-build", "crates/rfs-adapters", "crates/examples/*"]` is unambiguous and self-documenting. |
| C11-3: no_std compile checks need `-L dependency=` + per-test temp dirs | The rlib's metadata references its own dependencies by name (futures under the `async` feature), so a bare `rustc --extern rfs_adapters=<rlib>` fails with E0463 when the consumer actually uses items - fixed with `-L dependency=<deps dir>`. And the two checks originally shared one temp dir and ran on parallel test threads, racing on cleanup ("couldn't create a temp dir") - now unique per test. |
| C11-4: C mc entry point gained the library-level guard | The task decision was "mc rejected, pinned"; the CLI parser rejects for both backends and `rust_mc` had the direct-library-call guard, but `c_mc` did not - the new pin test caught it. Added for parity (one string covers every path, same convention as `NO_STD_PRODUCTION_ONLY_ERROR`). |

## Verification

- Adapter unit tests green: 12/12 with `--all-features` (10 adapter over the mock FSM + 2 no_std compile checks), 9/9 default features; `--no-default-features` build clean (core-only path).
- `--with-protocol-fsm` output byte-pinned (`PF_IMPL_BLOCK`; flag-on = default + block exactly once); default output unchanged (no trait trace; generated-code sweep 1453/1453 unchanged); mc rejected with the pinned message at CLI level (exit 1) and library level (both backends).
- Both example crates build in the workspace and their protocol drivers run to completion: `cargo run -p sync_handler` / `-p async_handler` exit 0 with identical observable sessions (19 ticks, frames/acks `[0..8]` in order, 5 dropped out-of-band acks, max window exactly 4); recorded as tests (`cargo test -p sync_handler` / `-p async_handler`, 1 each).
- no_std-minimal adapter compiles against a real task-9 `--no-std` generated FSM with the opt-in impl (the exit-criterion compile check; the task file's "task-8" reference is the same feature - no_std production codegen is task 9).
- cargo test 358/358 root (was 354; +4 codegen pins), 381/381 workspace (was 362; +4 codegen, +1 rfs-build argv pin, +12 rfs-adapters under feature unification, +1+1 examples), 360/360 `--features smt` (was 356); generated-code 1453/1453 unchanged; clippy identical to the 12-warning baseline (diff-verified; zero warnings in the new crates); fmt clean.
