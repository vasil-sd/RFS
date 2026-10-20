# RFS Build, Test, and Run Instructions

**All build/run/test how-to for the RFS project lives here.** Toolchain locations,
versions, activation env vars, and reinstall procedure: see `TOOLCHAIN.md`.

## 1. Where the main build files are

| File | Role |
|---|---|
| `Cargo.toml` | Package `rfs` v0.1.0, edition 2021; lib + CLI (`src/main.rs`) + bins in `src/bin/`. Key deps: `nom`, `serde`/`serde_json`; dev-dep `chumsky` (comparison tests only). |
| `.cargo/config.toml` | Cargo aliases used throughout the docs (see section 4). |
| `Cargo.lock` | Version 4 lock file - requires a recent cargo; use the project toolchain (`TOOLCHAIN.md`). |
| `src/` | Library + CLI source (map in `KB.md` section 4.2). |
| `tests/` | Integration test suites (layout in section 3 below). |
| `target/rfs_codegen_tests/` | Generated aggregate unit-test crate (written by `cargo generate-code-tests`; not checked in). |

## 2. Activate the toolchain, then build

```sh
export RUSTUP_HOME=/workspace/host/rustup
export CARGO_HOME=/workspace/host/.cargo
export PATH="$CARGO_HOME/bin:$PATH"
cd /workspace/host/RFS

cargo build        # library + CLI + bins
cargo test         # full main-crate suite (see section 3)
```

Re-export the three env vars in **every** shell command that needs cargo (env does not
persist between tool calls). If `cargo --version` is not 1.99.0, see `TOOLCHAIN.md`.

SMT mode (Phase 4 task 11, optional, feature-gated) additionally requires the
project-local z3 install: activation env vars per `TOOLCHAIN.md` section 5.2
(`PATH=/workspace/host/z3/bin:$PATH` + `Z3_LIBRARY_PATH_OVERRIDE`); generated C
SMT output compiles per section 5.3.

## 3. Test layout and expected current results

| Suite | Command | Current result |
|---|---|---|
| All main-crate tests | `cargo test` | **230 pass** (125 lib, 7 cross-backend parity, 5 benchmark MC, 26 codegen, 3 examples, 41 frontend CLI logic, 4 parser-library comparison, 9 liveness MC, 4 monitor integration, 4 training; +2 more with `--features smt`) |
| Parser unit tests only | `cargo test --lib` | part of the 230 |
| Example suite (file-driven: drop `.rfs` into `examples/valid\|invalid`, no test-code change) | `cargo test --test examples_test` | 3 pass; all 45 valid parse+validate+round-trip, all 12 invalid are rejected (parse or validation) |
| Parser-library comparison (nom vs chumsky smoke) | `cargo test --test library_comparison` | 4 pass |
| Training-set structure/snippets/aggregation/freshness | `cargo test-training` (= `cargo test training`) | 5 pass (see command table note) |
| Generated-code aggregate unit tests (separate crate under `target/rfs_codegen_tests`) | `cargo test-generated-code` | **537 pass** (179 modules; includes valid training snippets, so the count moves when snippet sets change) |
| Benchmark MC suite (Phase 4 task 9: specs 43/44/45 x both backends in exact mode, inline FAIL variants, JSON report; task 10B adds the `--threads` parity gate + `--fast` smoke) | `cargo test --test benchmark_mc_test` | 5 pass; all three benchmarks pass-complete in BOTH backends with metrics identical to the task 6/8 record (165/330/22, 129985/519940/276, 11098/99882/22); FAIL variants pinned (safety `holder_no_request` step 1; liveness `up2_served`, cycle length 3); `--threads 1/2/4` byte-identical to each other and across backends on all five workloads; `target/mc_benchmarks.json` written + per-backend table printed (wall-clock informational only) |

Other suites: `tests/frontend_test.rs` covers all four conversion combinations, strict
extension handling, output placement, collision guard, err-file generation, and
end-to-end binary exit codes; `tests/codegen_test.rs` covers the backend registry,
per-command generation, option sensitivity (incl. `--with-property-monitors`), and CLI
argument parsing; `tests/monitor_integration_test.rs` (Phase 3) drives an injected bug
through the MC in BOTH backends and exercises debug-gated production monitors;
`tests/backend_parity_test.rs` (Phase 3) is the cross-backend differential suite -
same-seed MC runs over all property-bearing examples must be field-identical, and both
backends' generated test executables must agree on every example;
`tests/benchmark_mc_test.rs` (Phase 4 task 9) drives the three benchmark specs
(43/44/45) through both backends in exact mode - pass-complete statuses, pinned
deterministic metrics, inline FAIL variants with counterexample parity, and a JSON
report (`target/mc_benchmarks.json`, development artifact, wall-clock informational).

## 4. Common commands (cargo aliases in `.cargo/config.toml`)

| Task | Command |
|---|---|
| Build library and CLI | `cargo build` |
| Run all tests | `cargo test` |
| Lint | `cargo clippy --all-targets` |
| Format / check formatting | `cargo fmt` / `cargo fmt --check` |
| Validate one file (extension picks format) | `cargo run -- validate spec.rfs` or `spec.json` |
| Validate a directory (writes `<name>.rfs-validation.err` next to failures) | `cargo run -- validate --input rfs specs/` |
| Validate all valid examples (sanity check) | `cargo run -- validate --input rfs examples/valid` -> exit 0 |
| Convert RFS to IR JSON | `cargo run -- parse --input rfs --output json <file-or-dir> [--target-dir DIR]` |
| Reformat / convert any format pair (rfs/json in both directions) | `cargo run -- parse --input <rfs\|json> --output <rfs\|json> <file-or-dir>` |
| List codegen backends / per-backend help | `cargo run -- codegen --list` / `cargo run -- codegen rust --help` |
| Generate production/test/MC Rust code | `cargo run -- codegen rust <production\|test\|mc> <spec.rfs\|spec.json> [--out FILE] [--assumes ...] [--steps N] [--with-property-monitors]` |
| Generate C code (Phase 2; same commands) | `cargo run -- codegen c <production\|test\|mc> <spec.rfs\|spec.json> [--out FILE] [--assumes ...] [--with-property-monitors]` (compile with `gcc -Wall out.c`) |
| Property monitors in production builds (Phase 3) | `--with-property-monitors` is accepted on the `production` command of ANY backend only; `test` and `mc` always include monitors. Rust gates monitor emission/stepping on `#[cfg(debug_assertions)]` (zero cost in release); C gates on `-DRFS_PROPERTY_MONITORS` |
| Generate aggregate generated-code test crate (179 modules) | `cargo generate-code-tests [--test-steps N] [--assumes ...]` |
| Generate + compile + run all generated unit tests (522) | `cargo test-generated-code` |
| Extract training snippets from `training/*/rfs_training.jsonl` | `cargo extract-training-snippets` -> 130 valid / 127 invalid snippets |
| Aggregate training JSONL | `cargo build-training` -> writes `all_rfs_training.jsonl` (231 examples) |
| Run training tests | `cargo test-training` (= `cargo test training`; matches 5 tests: 4 dedicated + the codegen aggregate test whose name contains "training") |
| Train Qwen3.8-27B on RFS data (SFT then GRPO, needs GPU) | `python training.py --stage both` (`--dry-run` = CPU-only data/reward validation) |

CLI behavior rules (file/dir handling, exit codes): `KB.md` section 4.5. Codegen
plugin API and backend docs: `CODEGEN.md`, `src/backends/RUST.md`.

## 5. Verification checklist after changes

After any non-trivial change, run at minimum:

```sh
cargo build && cargo test && cargo clippy --all-targets && cargo fmt --check
```

Additional checks depending on what changed:

- Touched parser/validator/examples -> `cargo test --test examples_test`.
- Touched codegen or backends -> `cargo test-generated-code` (regenerate the aggregate
  crate first if you changed generation output).
- Touched training sets -> `cargo extract-training-snippets && cargo build-training && cargo test-training`.
- Touched language syntax -> also update `DESIGN.md`/`KB.md` and add valid+invalid examples first.
- Touched SMT mode / the feature-gated z3 dependency -> rebuild with the SMT feature
  and the z3 env vars from `TOOLCHAIN.md` section 5.2; spot-compile generated C SMT
  output per section 5.3.

Known-good baseline (2026-10-20, after Phase 4): 230/230 main tests (232 with
`--features smt`, z3 env per TOOLCHAIN.md section 5.2), 537/537 generated tests
(179 modules), clippy without errors (12 pre-existing warnings: 11 in lib -
mostly `src/backends/c_fsm.rs` monitor rendering + `from_str` naming - and 1
`&PathBuf` arg in `tests/codegen_test.rs`), fmt clean. C generated code compiles
with `gcc -Wall`; the Phase 2 integration tests compile the Rust output with rustc
and drive it over an mpsc channel; the parity suites (Phase 3, migrated to seedless
exact mode in Phase 4 task 4) compile BOTH backends' MC/test output (rustc + gcc)
and compare results field by field.
