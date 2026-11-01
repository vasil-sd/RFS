<!-- llm-budget: 200 -->
# RFS Workspace Crates (Class B)

Compact card for LLM assistants working on workspace crates (`rfs-build`,
`rfs-adapters`, examples).

<a id="llm:workspace-crates"></a>
## Card

**Responsibility.** The cargo workspace members outside the root `rfs`
crate: `crates/rfs-build` (the build.rs helper that shells out to the rfs
toolchain - zero dependencies), `crates/rfs-adapters` (#![no_std] ProtocolFsm
trait + four generic adapters; only external dep is optional futures 0.3),
and the examples under `crates/examples/` (three Rust workspace members + two
Makefile-driven C projects that are NOT cargo members). All five Rust example
projects generate the SAME spec - examples/valid/43_sliding_window.rfs,
the sliding-window ARQ sender (8 frames, window bound 4) - at build time.

**Crate-to-file map (17 tracked .rs files).**

| Crate | Files |
|---|---|
| rfs-build | src/lib.rs (RfsBuild builder + resolve_rfs_bin), tests/integration.rs (real end-to-end) |
| rfs-adapters | src/lib.rs (ProtocolFsm trait), src/minimal.rs, src/callback.rs, src/channel.rs, src/async_adapter.rs, tests/adapters.rs (mock-FSM tests), tests/nostd_minimal.rs (no_std compile checks) |
| sync_handler | build.rs, src/lib.rs (run + ArqReport), src/main.rs |
| async_handler | build.rs, src/lib.rs (run + ArqReport), src/main.rs |
| fuzz_rust | build.rs (production lib body + adapted fuzz skeleton), src/lib.rs (pub use arq::*) - fuzz/fuzz_targets/arq.rs is a git-ignored build artifact, not tracked |

**Key items.**

| Item | Role |
|---|---|
| `crates/rfs-build/src/lib.rs::resolve_rfs_bin` | RFS_BIN (empty = unset; nonexistent file = ERROR) > bare PATH; PATH resolution is a documented rebuild limitation - set RFS_BIN to pin |
| `crates/rfs-build/src/lib.rs::RfsBuild::compile` | validates before touching the environment; artifact = OUT_DIR/<stem>.<rs\|c>; emits rerun-if-changed spec + env RFS_BIN (+ bin when pinned) + cargo:include; failures carry rfs exit code + stderr; success without artifact on disk = error |
| `crates/rfs-build/src/lib.rs::RfsBuild::test_command` | selects the test codegen command - the libfuzzer skeleton is written NEXT TO the main artifact (fuzz_target.rs/.c); how the fuzz examples obtain the emitted skeleton |
| `crates/rfs-adapters/src/lib.rs::ProtocolFsm` | Input/Output/Error associated types + new() + step() -> Result<Vec<Output>, Error>; mirrors EVERY generated Rust FSM; codegen emits the impl opt-in via --with-protocol-fsm (the one documented opt-in linking a generated artifact to an external crate) |
| `crates/rfs-adapters/src/callback.rs::drive` | reactive loop: closure gets the PREVIOUS step's outputs (empty first), returns next input or None; CallbackResult = Completed / Failed(e, steps) / StepLimit |
| `crates/rfs-adapters/src/channel.rs::ChannelFsm` | FSM on its own thread over std mpsc (FIFO); a failed step does NOT stop the thread; dropping the handle does not stop it (exits when the input channel closes) |
| `crates/rfs-adapters/src/async_adapter.rs::AsyncFsm` | futures Stream; stepped SYNCHRONOUSLY in poll_next (generated step does no I/O) so any executor works - no tokio; one queued input per poll, Pending until close() then ends |
| example build.rs (all three) | shared pattern: RFS_BIN > workspace target/debug/rfs > PATH; strip `#!` lines from the artifact (include! cannot carry inner attributes), byte-identical body to OUT_DIR; module-level #[allow] mirrors the artifact header exactly; cargo fmt never sees generated code |
| `crates/examples/fuzz_rust/build.rs` | step 2: test command -> emitted skeleton, `use crate::{` rewritten to `use fuzz_rust::{` (asserted), written to fuzz/fuzz_targets/arq.rs; fuzz/ is a DETACHED cargo-fuzz workspace (never built by the outer cargo test --workspace) |

**Invariants that must not break.**

- Build-script contract: rfs-build never modifies the spec; parse/validation/
codegen failures are build failures with the rfs diagnostics in the message;
  OUT_DIR is required (must run inside a cargo build script).
- Dependency hygiene: futures stays confined to rfs-adapters + async_handler
  (never the toolchain or generated code); rfs-build has zero dependencies;
  generated artifacts never depend on the toolchain except via the
  --with-protocol-fsm opt-in.
- Adapter generics: adapters are generic over F: ProtocolFsm and never name
  generated types; outcome enums carry manual Debug/PartialEq bounded on the
  ASSOCIATED types (the generated Fsm derives neither).
- no_std: callback + minimal are core-only and must compile inside #![no_std]
  crates (tests/nostd_minimal.rs pins this, incl. against a real task-9
  no_std generated FSM).
- Example parity: sync_handler / async_handler / c_sync_handler run the SAME
  scripted scenario with the SAME pinned outcome (c data lines byte-identical
to the Rust one); fuzz_c generation works without clang, building needs a
libFuzzer-enabled clang.
- Cold checkouts: examples need `cargo build` first (produces
target/debug/rfs) or RFS_BIN set - their build scripts prefer the workspace-
built toolchain over PATH when RFS_BIN is not pinned.

**Change impact.** A new adapter = one module + feature gate if it needs
std/futures + mock-FSM tests in tests/adapters.rs. A new example = crate dir
+ workspace member line (root Cargo.toml) + build.rs following the shared
pattern + a pin in tests/example_projects_test.rs. Verify with cargo test
--workspace (559 baseline) - the integration + example tests run the real
toolchain. Full table: llm:change-impact (task 13.2).

**Deep dive.** dev:workspace-crates-overview + dev:workspace-crates-build /
dev:workspace-crates-adapters / dev:workspace-crates-examples in
docs/dev/workspace_crates.md; file-level entries: dev:code-map-workspace;
codegen commands used by the examples: docs/dev/codegen_core.md; toolchain
fuzz targets (root fuzz/): dev:toolchain-fuzz-overview (task 12.3).
