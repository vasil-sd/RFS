# rfs-fuzz - toolchain fuzzing workspace (Phase 6 task 5, D-P6-4)

Root-level cargo-fuzz harness for the RFS TOOLCHAIN itself. This is the FAST
layer of the two-layer strategy; the slow CI-runnable layer is
`tests/toolchain_fuzz_test.rs` (see BUILD.md "Toolchain fuzzing").

## Targets

| Target | What it fuzzes | Invariant |
|---|---|---|
| `parser` | raw bytes -> `parse_source` | no panic; on success: validate + JSON round-trip byte-stable |
| `validator` | specgen-backed valid IRs + raw JSON bytes | `validate_ir` no panic; `Ir::from_json` Err-or-Ok, never panics |
| `ir_roundtrip` | random valid IRs (optionally mutated) | to_json/from_json/to_json byte-stable |
| `codegen` | random valid specs -> 3 commands x 2 backends | generation Ok + structural invariants (task-2 header, balanced delimiters, no placeholder tokens, core symbols) |

Random specs are built through the task-3 `rfs::specgen` module (seed + size
bounds read from fuzzer bytes by the shared wrapper in `fuzz_targets/common.rs`)
- validity by construction. Raw IR types are deliberately NOT made `Arbitrary`
(they would produce invalid IR).

## Requirements

- Rust side: a NIGHTLY toolchain (`cargo fuzz build/run` uses
  `-Zsanitizer=address`) - see TOOLCHAIN.md section 7 for the install/verify
  recipe. The harness pins no channel; your default nightly is used.
- Building also needs clang (libFuzzer runtime) per TOOLCHAIN.md section 7;
  `cargo check` in this directory works without it.

## Commands (run from THIS directory)

```sh
cd fuzz
cargo +nightly fuzz build                      # build all four targets
cargo +nightly fuzz run parser -max_total_time=10        # bounded smoke per target
cargo +nightly fuzz run validator -max_total_time=10
cargo +nightly fuzz run ir_roundtrip -max_total_time=10
cargo +nightly fuzz run codegen -max_total_time=10       # slowest per iteration
```

Long coverage-guided sessions are manual/nightly work, not CI: the CI layer is
`tests/toolchain_fuzz_test.rs` (seeded deterministic compile-and-run suite).

## Layout

- `Cargo.toml` - DETACHED harness package (`[workspace]` table +
  `[package.metadata] cargo-fuzz = true`), never part of the repo workspace.
- `fuzz_targets/common.rs` - shared specgen-backed Arbitrary wrapper.
- `fuzz_targets/{parser,validator,ir_roundtrip,codegen}.rs` - the targets.
- `target/` - build artifacts (gitignored).
