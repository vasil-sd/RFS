# RFS Workspace Crates (Class A)

The cargo workspace members outside the root `rfs` crate: the build-script
helper, the protocol-adapter layer, and the example crates.

<a id="dev:workspace-crates-overview"></a>
## Overview

The cargo workspace has FIVE members (explicit list in the root
Cargo.toml, not globs): `crates/rfs-build`, `crates/rfs-adapters`, and three
example crates under `crates/examples/` (`sync_handler`, `async_handler`,
`fuzz_rust`). Two things are deliberately NOT members: `crates/examples`
itself has no manifest, and the C example projects (`c_sync_handler`,
`fuzz_c`) are Makefile-driven - the C equivalent surface of the Rust-
ecosystem examples.

**Dependency directions.** The arrows point OUTWARD from the toolchain:
the example crates depend on `rfs-adapters` (runtime) and `rfs-build`
(build-dependency); `rfs-build` adds NO dependencies at all - it shells out
to the `rfs` binary; `rfs-adapters`'s only external dependency is
`futures 0.3`, optional behind the `async` feature, and it must never leak
into the rfs toolchain or generated code (it is confined to rfs-adapters +
async_handler). The one reverse edge is test-only: `rfs-adapters`
dev-depends on the root `rfs` crate to generate a no_std artifact for its
compile checks. Generated artifacts never depend on the toolchain; the only
documented opt-in that links a generated artifact to an external crate is
`--with-protocol-fsm` (links it to rfs-adapters).

**The shared example pattern.** Every Rust example generates the SAME spec -
`examples/valid/43_sliding_window.rfs`, the sliding-window ARQ sender (8
frames, window bound 4) - at build time via rfs-build, and pulls the artifact
in with `include!` of a body file in OUT_DIR: because `include!` cannot carry
inner attributes (`#![allow(...)]`), each build.rs strips the `#!` lines and
writes the byte-identical remainder to OUT_DIR, and the module-level
`#[allow]` in lib.rs mirrors the artifact's own header line exactly. The body
file lives outside the source tree, so `cargo fmt` never sees generated code.
Toolchain resolution in every example build.rs: RFS_BIN > workspace
target/debug/rfs (set via set_var when not pinned) > PATH - a cold checkout
must run `cargo build` first or set RFS_BIN. All example outcomes are pinned
by tests/example_projects_test.rs.

Code: `crates/rfs-build/src/lib.rs::RfsBuild`,
`crates/rfs-adapters/src/lib.rs::ProtocolFsm`


<a id="dev:workspace-crates-build"></a>
## rfs-build (`crates/rfs-build/`)

The build.rs helper that drives `.rfs` compilation from dependent crates'
build scripts by invoking the `rfs` toolchain.

**Tool resolution.** `crates/rfs-build/src/lib.rs::resolve_rfs_bin`: the
RFS_BIN environment variable (preferred - pins an exact binary, e.g.
target/debug/rfs or a released toolchain), then bare `rfs` on PATH. An
empty/whitespace RFS_BIN is treated as unset; a non-empty RFS_BIN that does
not point to an existing file is an ERROR. When the binary comes from PATH
there is no stable path cargo can watch, so rebuilds are only re-triggered by
spec changes and RFS_BIN changes (documented limitation - set RFS_BIN to pin
the toolchain; compile prints a note line in that case).

**Builder.** `crates/rfs-build/src/lib.rs::RfsBuild` (new / backend /
production / test_command / with_property_monitors / no_std /
with_protocol_fsm / spec): defaults are backend rust and the production
codegen command. `.backend()` accepts "rust" or "c" (anything else errors at
compile time). `.test_command()` selects the `test` codegen command, whose
extra file is the libfuzzer fuzz skeleton written NEXT TO the main artifact
(fuzz_target.rs / fuzz_target.c) - used by the Phase 5 task 12 fuzz examples.
`.no_std(true)` maps to the production-only --no-std flag (generated Rust
becomes #![no_std] + extern crate alloc with an identical public API; for the
C backend the flag is a documented no-op). `.with_protocol_fsm(true)` maps to
--with-protocol-fsm: emits `impl ProtocolFsm for Fsm`, linking the artifact
to rfs-adapters, so the consuming crate must depend on it (Rust backend only;
C ignores the flag).

**compile().** `crates/rfs-build/src/lib.rs::RfsBuild::compile` validates
BEFORE touching the environment (backend known, spec extension .rfs/.json,
spec file exists, OUT_DIR set - rfs-build must run inside a cargo build
script), then: artifact path = OUT_DIR/<spec-stem>.<rs|c>; prints the cargo
directives (`crates/rfs-build/src/lib.rs::RfsBuild::cargo_directives`, pure
and unit-testable): rerun-if-changed=<spec absolute>, rerun-if-env-
changed=RFS_BIN, rerun-if-changed=<RFS_BIN> when it names an existing file,
cargo:include=<artifact>; invokes `rfs codegen <backend> <command> <spec>
--out <artifact>` plus --with-property-monitors / --no-std / --with-
protocol-fsm when enabled. The crate never modifies the spec; parse/
validation/codegen failures become build failures carrying the rfs exit code
+ stderr (and stdout when non-empty); a successful exit with no artifact on
disk is also an error.

**Tests.** In-module unit tests are hermetic: a stub `rfs` shell script
records its argv, honors --out by writing a marker file, and fails (exit 3 +
stderr) when RFS_STUB_FAIL is set; env-mutating tests serialize on a shared
mutex (process-global state); the directive lines are byte-pinned.
`crates/rfs-build/tests/integration.rs` is the one REAL end-to-end check:
rfs-build against a real spec with the workspace rfs binary (built by cargo
as part of cargo test) - the rust backend's generated artifact must COMPILE,
and the c backend must generate a .c file.

Code: `crates/rfs-build/src/lib.rs::RfsBuildError`,
`crates/rfs-build/src/lib.rs::resolve_rfs_bin`,
`crates/rfs-build/src/lib.rs::RfsBuild`,
`crates/rfs-build/src/lib.rs::RfsBuild::compile`,
`crates/rfs-build/tests/integration.rs`


<a id="dev:workspace-crates-adapters"></a>
## rfs-adapters (`crates/rfs-adapters/`)

The Rust-ecosystem integration layer for generated FSMs (Phase 5 task 11):
the `ProtocolFsm` trait + generic adapters. The crate is #![no_std] with
alloc; all adapters are GENERIC over F: ProtocolFsm and never depend on
generated code.

**The trait.** `crates/rfs-adapters/src/lib.rs::ProtocolFsm`: associated
types Input / Output / Error (the generated Input / Output / StepError enums)
+ `new()` + `step(&mut self, input) -> Result<Vec<Output>, Error>`. The shape
mirrors EVERY generated Rust FSM; codegen emits the impl opt-in via
--with-protocol-fsm - a mechanical forward to the inherent Fsm methods.
Features: `std` (default) enables the channel adapter; `async` implies std +
futures 0.3 (the crate's only external dependency); callback and minimal are
always available (core-only - they work with the #![no_std] production
artifacts).

**The four adapters.**

- `crates/rfs-adapters/src/minimal.rs::Minimal<F>`: a transparent step-
  forwarding wrapper with no state of its own and no allocations beyond the
  trait's Vec return (new / step / into_inner). Minimal is ITSELF a
  ProtocolFsm, so adapters compose (drive a Minimal<F> through callback).
- `crates/rfs-adapters/src/callback.rs::drive`: a reactive driving loop -
  before each step the user closure receives the PREVIOUS step's outputs
  (empty for the first) and returns the next input or None to stop. Result:
  `crates/rfs-adapters/src/callback.rs::CallbackResult` - Completed(steps)
  (closure stopped), Failed(error, steps completed before it), StepLimit(
  steps).
- `crates/rfs-adapters/src/channel.rs::ChannelFsm<F>`: the FSM runs on its
  OWN thread; inputs/outcomes cross std mpsc channels with FIFO ordering
  preserved (spawn / send_input / recv_outcome). A failed step does NOT stop
  the FSM thread - subsequent inputs are still processed (the generated FSM's
  state is unchanged by a rejected input); outcomes are
  `crates/rfs-adapters/src/channel.rs::ChannelOutcome` (Outputs | Error).
  Dropping the handle does not stop the thread: it exits when the input
  channel closes (tx dropped) or the receiver drops.
- `crates/rfs-adapters/src/async_adapter.rs::AsyncFsm<F>`: submit inputs with
  submit(), consume outcomes as a futures Stream (`crates/rfs-adapters/src/
  async_adapter.rs::AsyncOutcome`). The FSM is stepped SYNCHRONOUSLY inside
  poll_next (generated step does no I/O), so ANY executor works - no tokio
  dependency. Each poll consumes at most one queued input; with an empty
  queue it is Pending until close(), then the stream ends. The Unpin bounds
  hold for every generated FSM and keep poll_next free of unsafe.

The outcome enums carry MANUAL Debug/PartialEq impls bounded on the
ASSOCIATED types (not F): the generated Fsm struct itself derives neither.

**Tests.** `crates/rfs-adapters/tests/adapters.rs` runs over a small in-
crate MOCK FSM (no generated code needed): minimal forwards + is itself a
ProtocolFsm; callback stops on None / reports errors with steps done /
respects the step limit; channel + async modules cover FIFO delivery, error
non-fatality, and stream drain/close. `crates/rfs-adapters/tests/nostd_
minimal.rs` is the no_std exit criterion: the core-only adapters must compile
inside #![no_std] crates - both with a hand-written mock FSM and against a
REAL task-9 no_std generated FSM (with the opt-in ProtocolFsm impl); gated on
rustc availability, generation runs unconditionally.

Code: `crates/rfs-adapters/src/lib.rs::ProtocolFsm`,
`crates/rfs-adapters/src/minimal.rs::Minimal`,
`crates/rfs-adapters/src/callback.rs::drive`,
`crates/rfs-adapters/src/callback.rs::CallbackResult`,
`crates/rfs-adapters/src/channel.rs::ChannelFsm`,
`crates/rfs-adapters/src/channel.rs::ChannelOutcome`,
`crates/rfs-adapters/src/async_adapter.rs::AsyncFsm`,
`crates/rfs-adapters/src/async_adapter.rs::AsyncOutcome`,
`crates/rfs-adapters/tests/adapters.rs`,
`crates/rfs-adapters/tests/nostd_minimal.rs`


<a id="dev:workspace-crates-examples"></a>
## Examples (`crates/examples/`)

Five example projects, all on the spec-43 sliding-window ARQ sender. The
three Rust crates are workspace members; the two C projects are Makefile-
driven and NOT cargo members (the C equivalent surface of the Rust examples).

**sync_handler.** Drives the generated FSM from another thread over std mpsc
channels via the channel adapter (`crates/examples/sync_handler/src/lib.rs::
run`): ChannelFsm::<Fsm>::spawn(), then a scripted peer - wake tick, ticks
until all 8 frames are sent and acknowledged, an out-of-band Ack injected
every third tick that the protocol must drop. The receiver observes only what
crosses the channel; `crates/examples/sync_handler/src/lib.rs::ArqReport`
{ frames_sent, acks_received, out_of_band_acks_dropped, ticks, max_window }
pins the outcome; the demo binary prints it and exits 0/1.

**async_handler.** The SAME protocol scenario over the futures stream adapter
(`crates/examples/async_handler/src/lib.rs::run`): AsyncFsm::<Fsm> stepped
inside poll_next, driven with the plain futures executor (no tokio); identical
ArqReport shape and pinned outcome. Its build.rs is byte-identical to
sync_handler's (both use --with-protocol-fsm).

**fuzz_rust.** The cargo-fuzz harness wiring the EMITTED libfuzzer_sys
skeleton to the generated FSM (`crates/examples/fuzz_rust/build.rs`): step 1
generates the production source into the lib body (the shared strip pattern);
step 2 runs the TEST codegen command and adapts the emitted skeleton - its
`use crate::{...}` import becomes `use fuzz_rust::{...}` so it resolves
against the generated module re-exported at the root (`crates/examples/
fuzz_rust/src/lib.rs` does pub use arq::*), with an assert that the adaptation
actually happened. The adapted target is written to fuzz/fuzz_targets/arq.rs
- a git-ignored BUILD ARTIFACT, not a mapped file. fuzz/ is a DETACHED cargo-
fuzz workspace (empty [workspace] table + [package.metadata] cargo-fuzz =
true per cargo-fuzz convention; libfuzzer-sys 0.4; depends on fuzz_rust by
path) and is never built by the outer cargo test --workspace.

**c_sync_handler.** The C mirror of sync_handler (Makefile-driven): `rfs
codegen c production` generates build/arq_fsm.c, which src/handler.c INCLUDES
DIRECTLY - single translation unit, exactly the wiring the fuzz skeleton uses;
the assumption handler continues on violation (the scenario never violates);
built with gcc -Wall under a zero-warning bar. Its data lines are byte-
identical to the Rust sync_handler's (the header line is language-specific).

**fuzz_c.** The C side of the fuzz matrix: generation needs ONLY the rfs
toolchain (production source under the exact name the skeleton's #include
expects + the test command's emitted skeleton, both in build/ so no -I is
needed); BUILDING needs a libFuzzer-enabled clang (clang -O1
-fsanitize=fuzzer,address over the skeleton only). The run target is a bounded
smoke run (-max_total_time=10) - a real campaign is user work, not a build
target.

**Pinning.** tests/example_projects_test.rs: c_sync_handler builds wall-clean
and matches the Rust scenario; fuzz_rust's skeleton compiles in its cargo-
fuzz layout; fuzz_c generates without clang (build gated on clang
availability).

Code: `crates/examples/sync_handler/src/lib.rs::run`,
`crates/examples/sync_handler/src/lib.rs::ArqReport`,
`crates/examples/async_handler/src/lib.rs::run`,
`crates/examples/fuzz_rust/build.rs`, `crates/examples/fuzz_rust/src/lib.rs`,
`tests/example_projects_test.rs`
