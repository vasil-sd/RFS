# async_handler - Phase 5 task 11 example

Drives the generated **sliding-window ARQ sender** (spec `examples/valid/43_sliding_window.rfs`)
as a **futures Stream** via the `rfs-adapters` async adapter (`AsyncFsm`).

## What it demonstrates

The same integration story as `sync_handler`, over the async surface:

1. `build.rs` generates the FSM at build time with `rfs-build` (task 1),
   passing the opt-in `--with-protocol-fsm` flag so the generated `Fsm`
   implements `rfs_adapters::ProtocolFsm`;
2. the driver submits inputs with `AsyncFsm::submit` and consumes outcomes
   from the stream (`futures::stream::StreamExt`); the FSM is stepped
   synchronously inside `poll_next`, so the plain `futures` executor works -
   no tokio dependency (tokio interop is user code, per the task non-goals);
3. the protocol scenario matches `sync_handler` exactly (wake tick, ticks
   until all 8 frames are sent and acknowledged with an out-of-band `Ack`
   injected every third tick, completion + quiescence) - both examples
   observe identical behavior: in-order frames/acks `[0..8]`, max window 4,
   5 dropped out-of-band acks, 19 ticks.

`futures` is confined to this crate + `rfs-adapters` (never the `rfs`
toolchain or generated code).

## Build & run

Prerequisite: the `rfs` toolchain must be built (`cargo build` at the repo
root produces `target/debug/rfs`) or `RFS_BIN` must point to it - see
`BUILD.md`. Then:

```sh
cargo run -p async_handler   # demo: prints the observed session, exit 0 on success
cargo test -p async_handler  # the same session as a test (CI record)
```
