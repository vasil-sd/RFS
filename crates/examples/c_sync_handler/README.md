# c_sync_handler - plain-C ARQ handler example (Phase 5 task 12)

The C mirror of `crates/examples/sync_handler`: drives the **generated**
spec-43 sliding-window ARQ sender (`examples/valid/43_sliding_window.rfs`)
with a scripted peer - same scenario, same pinned outcome. Makefile-driven by
design (NOT a cargo workspace member; the C equivalent surface of the
Rust-ecosystem examples).

## Layout

- `src/handler.c` - the scripted peer. Includes the generated FSM source
  directly (`#include "arq_fsm.c"`, single translation unit - the same wiring
  the fuzz skeleton uses). No sanitizers: plain `gcc -Wall`.
- `Makefile` - generates the C FSM via the rfs toolchain, then builds with
  `gcc -Wall` (zero-warning bar).

## Build & run

```sh
cargo build                      # from the repo root, first (produces target/debug/rfs)
make RFS_BIN=$PWD/../../../target/debug/rfs   # or: export RFS_BIN=... / put rfs on PATH
./c_sync_handler                 # exit 0 on success
```

`make generate` runs only the codegen step (no gcc needed).

## Expected output (pinned by tests/example_projects_test.rs)

The data lines are byte-identical to the Rust `sync_handler`'s (the header
line is language-specific):

```
  ticks issued:             19
  frames sent (in order):   [0, 1, 2, 3, 4, 5, 6, 7]
  acks received (in order): [0, 1, 2, 3, 4, 5, 6, 7]
  out-of-band acks dropped: 5
  max window observed:      4 (bound 4)
```
