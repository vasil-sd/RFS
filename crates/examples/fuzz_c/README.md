# fuzz_c - clang + libFuzzer harness for the emitted spec-43 skeleton

Phase 5 task 12 example (C side of the fuzz matrix): wires the `fuzz_target.c`
skeleton EMITTED by `rfs codegen c test` (task 6) into a buildable libFuzzer
target. The skeleton decodes fuzzer bytes per the shared decode plan (first
byte = input-variant selector, then bool/bit<4>/bit<32> fields) and loops them
over `fsm_step` with the stop-on-assumption-violation handler.

## Layout

- `Makefile` - generation + clang build + bounded smoke run.
- `build/` (generated, not checked in): `rfs_generated_fsm.c` (production FSM
  source, under the exact name the skeleton's `#include` expects) and
  `fuzz_target.c` (the emitted skeleton).

## CLANG requirement

**Generation works without clang; building does not.** The build recipe is a
libFuzzer-enabled clang:

```sh
clang -O1 -fsanitize=fuzzer,address -o fuzz_target build/fuzz_target.c
```

If `clang` is absent, `make generate` still works and `make` fails at the
build step with clang's own diagnostic (no graceful degradation). Install per
`TOOLCHAIN.md` (or set `CLANG=/path/to/clang`).

## Build / run

From the repository root, after `cargo build` (or with `RFS_BIN` set):

```sh
cd crates/examples/fuzz_c
make RFS_BIN=$PWD/../../../target/debug/rfs   # or: export RFS_BIN=...
./fuzz_target -max_total_time=10              # bounded smoke run
```

The target exits 0 when the time budget elapses without a crash; a non-zero
exit means libFuzzer found a crashing input (it is kept under `crash-*`).
