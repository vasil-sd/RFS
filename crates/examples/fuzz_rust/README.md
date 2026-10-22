# fuzz_rust - cargo-fuzz harness for the generated ARQ sender

Phase 5 task 12 example: wires the **emitted** libfuzzer_sys skeleton (task 5,
the `fuzz_target.rs` extra file of the `test` codegen command) to the
generated spec-43 sliding-window ARQ sender.

Layout (cargo-fuzz convention):

- the root crate (`src/lib.rs`) is a plain lib holding the generated
  PRODUCTION FSM (build.rs generates it via rfs-build and pulls it in with
  `include!`);
- `fuzz/` is a DETACHED cargo-fuzz workspace (its own `[workspace]` table -
  not a member of this repository's workspace);
- `fuzz/fuzz_targets/arq.rs` is the emitted skeleton, adapted by build.rs
  (`use crate::{...}` -> `use fuzz_rust::{...}`) and gitignored.

## Build / run

```sh
# from crates/examples/fuzz_rust (needs the rfs toolchain built first: cargo build at repo root, or RFS_BIN)
cargo install cargo-fuzz        # one-time; not needed for `cargo check`
cargo fuzz run arq -- -max_total_time=10
```

Notes:

- `cargo check` in `fuzz/` type-checks the adapted skeleton against the
  generated module WITHOUT a fuzzer runtime (the libFuzzer runtime is only
  needed at link time) - that is what the repository test suite verifies.
- Building the actual fuzzer binary needs the libFuzzer runtime (a
  libFuzzer-enabled clang); see `TOOLCHAIN.md`.
- The fuzz target decodes bytes per the shared decode plan (first byte =
  input variant selector; bool = non-zero byte; bit<W> = ceil(W/8) masked LE
  bytes) and steps the FSM until the input is exhausted.

## Expected behavior

Bounded runs (`-max_total_time`) exit 0 on this spec: every decoded input
sequence is either accepted by the FSM or rejected as an unexpected event -
the generated `step` never panics, so no crash is reachable through this
interface.
