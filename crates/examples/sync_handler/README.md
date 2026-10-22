# sync_handler - Phase 5 task 11 example

Drives the generated **sliding-window ARQ sender** (spec `examples/valid/43_sliding_window.rfs`)
from another thread over **std mpsc channels** via the `rfs-adapters`
channel adapter (`ChannelFsm`).

## What it demonstrates

The full Rust-ecosystem integration story:

1. `build.rs` generates the FSM at build time with `rfs-build` (task 1),
   passing the opt-in `--with-protocol-fsm` flag so the generated `Fsm`
   implements `rfs_adapters::ProtocolFsm` (the one documented opt-in that
   links a generated artifact to an external crate);
2. the generated artifact is pulled in as a `#[path]` module (`src/arq_generated.rs`,
   gitignored, regenerated on every build-script run);
3. the driver spawns the FSM on its own thread and plays the environment:
   a wake tick (Idle -> Active), then ticks until all 8 frames are sent and
   acknowledged - injecting an out-of-band `Ack` every third tick that the
   protocol must drop - then a completion tick (-> Done) and a quiescence
   tick;
4. the receiver side observes only what crosses the channel: SendFrame /
   RecvAck outputs, verified in-order with the window bound (max 4) held.

## Build & run

Prerequisite: the `rfs` toolchain must be built (`cargo build` at the repo
root produces `target/debug/rfs`) or `RFS_BIN` must point to it - see
`BUILD.md`. Then:

```sh
cargo run -p sync_handler   # demo: prints the observed session, exit 0 on success
cargo test -p sync_handler  # the same session as a test (CI record)
```
