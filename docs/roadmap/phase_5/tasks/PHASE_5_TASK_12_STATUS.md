# Phase 5, Task 12 - Implementation status

Completed 2026-10-22. All exit criteria met; full verification green (see bottom).

## Added source files

| File | Description |
|------|-------------|
| `crates/examples/c_sync_handler/Makefile` | generate (`rfs codegen c production`) + `gcc -Wall` build + run/clean targets; RFS_BIN > PATH resolution mirroring rfs-build |
| `crates/examples/c_sync_handler/src/handler.c` | the scripted peer: wake tick, drive until 8 frames sent+acked (out-of-band Ack injected every third tick), completion + quiescence ticks, invariant checks, pinned data-line output byte-identical to the Rust sync_handler's |
| `crates/examples/c_sync_handler/README.md` | build/run + expected output (pinned) |
| `crates/examples/fuzz_c/Makefile` | generate (production FSM under the skeleton's exact `#include` name + the emitted `fuzz_target.c` from the `test` command) + clang `-O1 -fsanitize=fuzzer,address` build + bounded smoke run; CLANG override |
| `crates/examples/fuzz_c/README.md` | the CLANG requirement (generation works without it; building does not) + recipe |
| `crates/examples/fuzz_rust/Cargo.toml`, `src/lib.rs`, `build.rs` | root lib crate holding the generated spec-43 production FSM (`include!` of an OUT_DIR body, inner attributes stripped - the task 11 pattern); build.rs ALSO runs rfs-build's `.test_command()` and writes the emitted skeleton, adapted (`use crate::{` -> `use fuzz_rust::{`), to `fuzz/fuzz_targets/arq.rs` |
| `crates/examples/fuzz_rust/fuzz/Cargo.toml`, `fuzz/Cargo.lock` | DETACHED cargo-fuzz workspace (own `[workspace]` table, never built by the outer one); libfuzzer-sys 0.4 + path dep on the root crate; `[[bin]] arq` with `harness = false` |
| `crates/examples/fuzz_rust/README.md` | layout + `cargo fuzz run arq -- -max_total_time=10` + why `cargo check` needs no fuzzer runtime (link-time only) |

## Changed/removed source files

| File | Change |
|------|--------|
| `Cargo.toml` | workspace members: the `crates/examples/*` glob became an explicit list adding `fuzz_rust` (the C projects are Makefile-driven and NOT members - documented in the comment) |
| `crates/rfs-build/src/lib.rs` | NEW `.test_command()` builder option: selects the `test` codegen command so build scripts can obtain the emitted libfuzzer skeleton (extra file written next to the main artifact as `fuzz_target.rs`/`fuzz_target.c`); module docs updated; +1 argv pin test |

## Added/changed/extended test files

| File | Change |
|------|--------|
| `tests/example_projects_test.rs` (new) | 3 tests: (1) c_sync_handler rebuilt from scratch via its Makefile with RFS_BIN pinned - gcc -Wall clean (gcc-diagnostic check, C12-1) + scenario data lines byte-identical to the Rust sync_handler's AND pinned (19 ticks / [0..8] frames+acks / 5 dropped out-of-band acks / max window 4); (2) fuzz_rust root-crate build + `cargo check` in the detached fuzz workspace (registry/network failure -> skip with note, compile error -> fail); (3) fuzz_c generation without clang + skeleton/FSM content pins, build gated on clang availability |
| `crates/rfs-build/src/lib.rs` (tests mod) | `test_command_pins_argv`: `.test_command()` switches the argv command from production to test |

## Added/changed/extended documentation files

| File | Change |
|------|--------|
| `BUILD.md` | file-map rows (Cargo.toml members, rfs-build options, task 12 example projects), root count 358->361 + the example-projects suite in the breakdown, workspace 381->385 (rfs-build 7->8 unit), rfs-build-only row 9->10, new command-table row for `--test example_projects_test`, baseline re-pinned to task 12 |
| `KB.md` | 4.1 dependency paragraph (fuzz_rust root crate dependency-free; the detached fuzz workspace's only external dep is libfuzzer-sys), rfs-build row (+`.test_command()`), new example-projects table row |
| `docs/roadmap/INTRO.md` | Phase 5 progress: tasks 1-12 of 16 done |

## Other file changes

- `.gitignore` per C project (`/build/` + the built binary) and extended for fuzz_rust (`/fuzz/target/`) - generated artifacts and build outputs are NOT checked in (C12-2).

## Changed functionality

- `rfs-build`: build scripts can now select the `test` codegen command (`.test_command()`); default behavior unchanged.

## Removed functionality

(none)

## New functionality

- The example-crate matrix is complete per the parity principle: plain-C sync handler (C mirror of the Rust examples, same scenario/outcome), and fuzzing harness examples for BOTH ecosystems wiring the emitted skeletons (cargo-fuzz/libfuzzer_sys; clang libFuzzer).

## Task corrections

- **C12-1**: the gcc zero-warning bar must check for GCC-style diagnostics (`handler.c:`/`arq_fsm.c:` file references), not any `warning` substring - the rfs codegen step prints expected `warning: property ... is outside the monitor subset; skipped` diagnostics on stderr for spec 43, which a naive check would trip on.
- **C12-2**: the root `.gitignore`'s `/target` is anchored to the repo root and does NOT cover `crates/examples/fuzz_rust/fuzz/target/`; per-project `.gitignore` files were needed (also for the C projects' `build/` dirs and binaries) - without them 280+ build artifacts would have been committed.
- **C12-3** (environment note, not a defect): clang is ABSENT in this environment, so the fuzz_c build gate + README documentation path is what was exercised; the Makefile recipe itself follows the documented libFuzzer convention (`clang -O1 -fsanitize=fuzzer,address`).

## Verification

- `cargo test` 361/361 root (was 358; +3 example_projects, and the rfs-build argv pin lands in the workspace count), 385/385 `--workspace` (was 381), 363/363 `--features smt` (was 360).
- c_sync_handler: `make clean && make` from scratch - gcc step emits zero diagnostics; run exit 0 with the pinned output; data lines byte-identical to `cargo run -p sync_handler` (test-enforced).
- fuzz_rust: `cargo build -p fuzz_rust` + `cargo check` in `fuzz/` clean (libfuzzer-sys resolved from the local registry cache; offline-safe via the committed `fuzz/Cargo.lock`).
- fuzz_c: `make generate` without clang produces `build/fuzz_target.c` + `build/rfs_generated_fsm.c`; build step gated on clang.
- clippy (`cargo clippy --all-targets`, root) identical to the 12-warning baseline; `cargo fmt --check` clean.
