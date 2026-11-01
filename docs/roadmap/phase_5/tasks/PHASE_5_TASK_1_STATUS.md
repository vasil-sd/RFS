# Phase 5, Task 1 - Implementation status

> Written once when all exit criteria of `PHASE_5_TASK_1.md` are reached. Not
> rewritten afterwards; later fixes go into TASKS.md entries and follow-up phase plans.

## Added source files

| Name | Description |
|---|---|
| `crates/rfs-build/Cargo.toml` | Package manifest: `rfs-build` v0.1.0, edition 2021, lib name `rfs_build`, ZERO dependencies (the crate only orchestrates the external `rfs` binary and prints cargo directives). |
| `crates/rfs-build/src/lib.rs` | The build.rs helper API. `RfsBuild::new(spec)` + builder options `.backend("rust"\|"c")` (default rust), `.production()` (default/only command, explicit for readability), `.with_property_monitors(bool)` (maps to the existing production-codegen flag); `.compile()` validates (backend, spec extension .rfs/.json, spec existence - all BEFORE touching the environment), resolves the toolchain (`RFS_BIN` env var wins over PATH; empty/whitespace RFS_BIN = unset; missing RFS_BIN file = error), prints pinned cargo directives (`rerun-if-changed` on the spec and on the RFS_BIN path when set, `rerun-if-env-changed=RFS_BIN`, `include` on the artifact), runs `rfs codegen <backend> production <spec> --out OUT_DIR/<stem>.<ext>` (+ `--with-property-monitors`), propagates parse/validation/codegen failures as build errors WITH the rfs stderr (and stdout) in the message, and returns the artifact path. Public `resolve_rfs_bin()` pins the precedence for tests. Module doc carries the consumer usage pattern (build.rs + include!), toolchain-location rules, emitted directives, and scope. `.no_std()` is deliberately NOT shipped yet - task correction T1-1 (the CLI flag lands in task 9). |
| `crates/rfs-build/tests/integration.rs` | Real end-to-end: runs the helper against `examples/valid/01_trivial_two_state.rfs` with the REAL workspace binary (`target/debug/rfs`, located via CARGO_MANIFEST_DIR) and OUT_DIR in a temp dir - rust backend: artifact exists + `rustc --edition 2021 --crate-type lib` compiles it; c backend: `.c` artifact generated with headers. |

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| `Cargo.toml` | changed | Became the WORKSPACE ROOT: `[workspace] members = ["crates/*"]` added above the untouched `[package]` (root package stays an implicit member; `default-run = "rfs"` preserved). Later crates (tasks 11/12) drop into `crates/` automatically. |
| `src/backends/rust_test.rs` | changed | Task correction T1-3: the generated aggregate crate's manifest now carries an empty `[workspace]` table. Without it, cargo walking up from `target/rfs_codegen_tests/Cargo.toml` discovers the new root workspace and refuses to build ("current package believes it's in a workspace when it's not") - this broke `cargo test-generated-code` immediately after the conversion; the fix makes the generated crate its own workspace root. |
| `.cargo/config.toml` | unchanged | No alias needed a `-p rfs` qualifier (verified, see below). |

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `BUILD.md` | changed | Section 1: workspace-root row + `crates/rfs-build` row + generated-crate `[workspace]` note (also fixed the stale serde mention in the Cargo.toml row - removed by task 2). Section 2: `cargo test --workspace` line + workspace-selection note (plain commands from the root select only the root package; `--workspace`/`-p` cover members). Section 3: suite table actualized to current truth (257 root / 264 workspace) + new rfs-build rows. Section 4: alias re-verification record (all 8 aliases + default-run re-run from the repo root after the conversion, all working unchanged) + stale "(522)" count fixed to 537 in the test-generated-code row. |

## Other file changes

| Name | Operation | Description |
|---|---|---|
| `Cargo.lock` | changed | Regenerated for the workspace (rfs-build added; no new external dependencies anywhere). |

## Changed functionality

- None user-visible in the toolchain itself: same binaries, same CLI, same aliases. The only behavioral change is in generated build artifacts: `target/rfs_codegen_tests/Cargo.toml` now contains an empty `[workspace]` table (required for it to build outside a workspace context).
- New capability: consumer Rust projects can wire RFS into `build.rs` via `rfs-build` (phase exit criterion "a team can add RFS to an existing Rust project with build.rs").

## Removed functionality

- None.

## New functionality

- Cargo workspace (root package + `crates/*`) - the plumbing tasks 11/12 need for adapter/example crates.
- `rfs-build` crate: spec -> generated code at build time with RFS_BIN/PATH toolchain resolution, pinned cargo rerun directives, and rfs diagnostics propagated into build failures.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|
| `rfs_build::tests::resolve_rfs_bin_precedence` | new | RFS_BIN existing file wins; empty/whitespace = unset; unset -> bare `rfs`; missing file -> error. (Env-mutating tests serialized via a static mutex.) |
| `rfs_build::tests::option_validation_errors_before_environment` | new | unknown backend / wrong extension / missing spec all error BEFORE OUT_DIR/RFS_BIN are consulted. |
| `rfs_build::tests::compile_success_and_error_propagation_with_stub` | new | hermetic stub rfs (shell script): success returns the artifact path + writes it; exact argv pinned (`codegen rust production <spec> --out <artifact> --with-property-monitors`); c backend picks `.c`; forced failure propagates exit code 3 + stderr text into the error. |
| `rfs_build::tests::compile_without_out_dir_is_an_error` | new | missing OUT_DIR -> clear "must run inside a cargo build script" error. |
| `rfs_build::tests::cargo_directives_pinned` | new | exact directive lines for RFS_BIN case (4 lines incl. binary watch) and PATH case (3 lines, no watch). |
| `integration::real_end_to_end_rust_backend_compiles` | new | real workspace binary + real spec; artifact compiles with rustc as a lib crate. |
| `integration::real_end_to_end_c_backend_generates_c_file` | new | same via the c backend. |

## Tests run status

| Name | Status |
|---|---|
| `cargo build` (workspace root) | OK |
| `cargo test` (root package) | OK (**257/257** - unchanged by this task; breakdown in BUILD.md section 3) |
| `cargo test --workspace` | OK (**264/264** = 257 + 7 rfs-build) |
| `cargo test -p rfs-build` | OK (5 unit + 2 integration, incl. the rustc compile check) |
| every alias in `.cargo/config.toml` | OK - re-run from the repo root after conversion: run (default-run), parse, validate, codegen --list, generate-code-tests (179 modules), test-generated-code (**537/537** - after the T1-3 manifest fix), extract-training-snippets (130/127), build-training (231 aggregate), test-training (5 pass). No qualifier changes needed. |
| `cargo clippy --workspace --all-targets` | OK (zero new warnings; rfs-build clean; pre-existing baseline unchanged: 11 lib + 1 codegen_test) |
| `cargo fmt --check` | OK (clean) |

## Found problems/issues/mismatches

- **T1-3 (the anticipated member-resolution surprise, materialized)**: after adding `[workspace]`, `cargo test-generated-code` failed - cargo loading `target/rfs_codegen_tests/Cargo.toml` walks up, finds the repo workspace, and errors because that manifest is not a member. Fixed at the source by emitting an empty `[workspace]` table in the generated manifest (the fix cargo's own error message suggests). Recorded as task correction T1-3.
- **Workspace command selection**: plain `cargo test`/`cargo build` from the repo root select ONLY the root package (standard behavior for non-virtual workspaces) - so rfs-build tests are NOT covered by a bare `cargo test`. Documented in BUILD.md sections 2/3 (`--workspace` = full check). This is why the exit criterion "cargo test green with unchanged results" holds literally (same 257) while full coverage lives at `--workspace` (264).
- Stale numbers found and fixed in place while updating BUILD.md: the Cargo.toml row still listed serde/serde_json (removed by task 2) and the test-generated-code alias row said "(522)" (true count 537 since Phase 4 close-out).

## Other important findings/information

- The conversion was otherwise zero-drama: no file moves, no alias changes, `default-run` intact, all binaries at the same paths.
- For tasks 11/12: new crates drop into `crates/` and join the workspace via the existing glob; the example-crate layout (and whether examples are workspace members or standalone) is decided there.
- For task 9: add `.no_std(bool)` to `RfsBuild` mapping to the new `--no-std` production flag (correction T1-1), plus a stub test pinning the argv.
