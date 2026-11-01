# Phase 5, Task 17 - Implementation status

Completed 2026-10-22. All exit criteria met; full verification green (see bottom).

## Added source files

| File | Description |
|------|-------------|
| `docs/roadmap/PHASE_5_TASK_17_STATUS.md` | this file |

## Changed/removed source files

| File | Change |
|------|--------|
| `src/ext_tool.rs` | NEW presets `CLANG` (`RFS_CLANG`/`RFS_CLANG_ARGS`) and `CARGO_FUZZ` (`RFS_CARGO_FUZZ`/`RFS_CARGO_FUZZ_ARGS`, bare name `cargo` + fixed subcommand `fuzz`); `ExtTool.subcommand` field (default "") rendered/invoked right after the resolved path, before extra args; `ResolvedTool.subcommand` + `display_name()` ("cargo-fuzz"); `run_in(cwd, args)` + `run_in_without(cwd, remove, args)` (the cargo-fuzz subprocess runs from the temp scaffold and with a non-nightly inherited `RUSTUP_TOOLCHAIN` stripped - C17-3); `Script::with_note` (header `# note:` lines for required runtimes); module-doc env-var list incl. `RFS_FUZZ_FLAGS` (documented as NOT a tool preset) |
| `src/test_cmd.rs` | `TestSuite` enum (`unit`/`fuzz`) + `from_str`; `TestOptions` gains `suites` (default `[Unit]`), `fuzz_max_total_time` (default 30), `clang_path/args`, `cargo_fuzz_path/args`, `fuzz_flags`; the unit pipeline extracted verbatim into `run_unit_suite` (byte-for-byte unchanged behavior); NEW fuzz pipelines: C = write skeleton + production source under its hardcoded `#include` name, build with clang `-O1 -fsanitize=fuzzer,address` (skeleton only - C17-1), bounded run with `ASAN_OPTIONS=detect_leaks=0` merged (C17-2); Rust = temp cargo-fuzz scaffold (root lib = generated production source, adapted skeleton `use crate::{` -> `use rfs_fuzz::{`, harness manifest with `[package.metadata] cargo-fuzz = true` + `libfuzzer-sys = "=0.4.13"` pinned to the local registry cache, `rust-toolchain.toml` pinning nightly - C17-3/C17-4), `cargo fuzz build` + `cargo fuzz run fuzz_target -- -max_total_time=N <flags>`; missing-tool diagnostics (clang spawn NotFound; cargo-fuzz `no such command` / nightly gaps); every SELECTED suite runs before the combined failure is reported; `--print-script` gains the fuzz steps (heredocs for both C sources / full Rust scaffold, build + bounded run + cleanup) with a header note naming the required tools; the extra-files debug note is now conditional (consumed vs `--suite fuzz` hint) |
| `src/main.rs` | `rfs test` flags: `--suite unit\|fuzz` (repeatable/comma-separated; first occurrence replaces the default `[Unit]`, later ones append; invalid name -> usage error), `--fuzz-max-total-time SEC` (positive integer), `--fuzz-flags "..."`, plus the full ext_tool-convention tool overrides `--clang/--clang-args/--cargo-fuzz/--cargo-fuzz-args` (beyond Action 4's three flags - decision D17-1); usage text updated |
| `crates/examples/fuzz_rust/fuzz/Cargo.toml` | PRE-EXISTING FIX (C17-4): added `[package.metadata] cargo-fuzz = true` - without it the README's documented `cargo fuzz run arq` fails (task 12 only verified `cargo check`, never cargo-fuzz itself) |

## Added/changed/extended test files

| File | Change |
|------|--------|
| `tests/frontend_test.rs` | +6 e2e tests: suite-selection pins (default path proven tool-free via a broken RFS_CLANG; `--suite fuzz` = fuzz only; `unit,fuzz` = both; invalid suite + non-positive budget usage errors), missing-tool diagnostics for both backends (pinned), stub-pinned fuzzer-crash exit (fake clang producing an exit-31 binary), full-pipeline stub argv proofs (C: env-then-CLI args before the pinned recipe; Rust: fixed `fuzz` subcommand placement, build + run invocations, `-max_total_time` then RFS_FUZZ_FLAGS env then --fuzz-flags), byte-pinned `--print-script` wrapper for the C fuzz script with both heredoc bodies cross-checked against actual codegen output + no-execution proof, real-tool gated e2e (both backends build + bounded-run on a passing spec -> exit 0; environment gaps skip with a note) |
| `src/ext_tool.rs` (tests mod) | +2 unit pins: CLANG/CARGO_FUZZ resolution precedence + fixed subcommand; subcommand placement in render/run + script header format for subcommand tools. Two pre-existing struct-literal tests gained the new `subcommand` field (pinned outputs unchanged) |

## Added/changed/extended documentation files

| File | Change |
|------|--------|
| `TOOLCHAIN.md` | NEW section 7 (after the diagram-tools section 6 - no renumbering): install commands verified in this environment (apt clang 18.1.3 incl. libFuzzer runtime; `cargo install cargo-fuzz --locked` v0.13.2; `rustup toolchain install nightly --profile minimal`), copy-paste verification recipe, env-var table (`RFS_CLANG*`, `RFS_CARGO_FUZZ*`, `RFS_FUZZ_FLAGS`), gotchas (leak detection off + why, skeleton-only compile, nightly pin + RUSTUP_TOOLCHAIN stripping, cargo-fuzz marker + offline libfuzzer-sys pin) |
| `BUILD.md` | `rfs test` command row rewritten (suites, fuzz pipelines, new flags/env vars, exit codes); counts re-pinned (root 361->369 with the task 17 suite in the breakdown, workspace 385->393, smt 363->371); baseline paragraph re-pinned to task 17 (clippy 12->13 warnings: the new `TestSuite::from_str` mirrors the pre-existing `CodegenCommandKind::from_str` pattern); verification checklist row for the fuzz pipelines |
| `KB.md` | status-table Tests row updated to current counts + task 17 suite; 4.5 `rfs test` paragraph rewritten (suite selection, both fuzz pipelines incl. scaffold details, env-var parameterization, missing-tool semantics, exit codes) |

## Other file changes

- Environment installs (documented in TOOLCHAIN.md section 7): clang 18.1.3 (`apt-get install -y clang`), cargo-fuzz 0.13.2 (`cargo install cargo-fuzz --locked` into the project-local CARGO_HOME), nightly toolchain (`rustup toolchain install nightly --profile minimal`). No repository files beyond the above.

## Changed functionality

- `rfs test`: gains `--suite unit|fuzz` (default unit - task 7 behavior byte-for-byte unchanged, all 82 pre-existing pins pass unmodified), the two libFuzzer pipelines, and the new flags/env vars. Missing clang/cargo-fuzz only errors when the fuzz suite is selected.

## Removed functionality

(none)

## New functionality

- `rfs test --suite fuzz` builds + bounded-runs the emitted libFuzzer skeletons for BOTH backends in one invocation; `--print-script` covers the fuzz steps; full env-var parameterization aligned with task 3's ext_tool conventions.

## Task corrections / decisions

- **C17-1**: the task's C build line lists BOTH sources (`fuzz_target.c rfs_generated_fsm.c`), but the skeleton `#include`s the production source - compiling both is a multiple-definition link error (verified empirically). The recipe compiles ONLY `fuzz_target.c`; the production file is still written next to it under its exact hardcoded name (same as the task 12 fuzz_c Makefile).
- **C17-2**: ASan's exit-time leak report fails EVERY C fuzz run: the generated C FSM has no free API (process-lifetime buffers, one allocation set per iteration - 740k allocations in a 2 s run on spec 43). The C pipeline merges `ASAN_OPTIONS=detect_leaks=0` (preserving user options after ours; first occurrence of a key wins). A command-line `-detect_leaks=0` does NOT work - ASan only reads env vars. In-flight memory errors remain detected.
- **C17-3**: cargo-fuzz builds with `-Zsanitizer=address` (nightly-only) - on the stable default toolchain every `cargo fuzz build` fails with "1 nightly option were parsed". The scaffold writes a `rust-toolchain.toml` pinning `channel = "nightly"` INSIDE the temp project (user's default untouched); rfs additionally strips a non-nightly inherited `RUSTUP_TOOLCHAIN` from the cargo-fuzz subprocess - under `cargo test`/`cargo run` rustup pins stable in the child env, which would override the file (found empirically: the real-tool e2e skipped until this fix).
- **C17-4**: the harness manifest needs `[package.metadata] cargo-fuzz = true` or `cargo fuzz` rejects it ("does not look like a cargo-fuzz manifest"). Fixed in the scaffold AND in task 12's `crates/examples/fuzz_rust/fuzz/Cargo.toml` (pre-existing latent bug: its README's `cargo fuzz run arq` would have failed).
- **C17-5**: `RFS_CLANG_ARGS`/`--clang-args` sit right after the clang path, BEFORE `-O1 -fsanitize=fuzzer,address` - the ext_tool contract shared by every preset (task 3 single-rendering invariant); the task sketch's mid-command position is not expressible in `ResolvedTool`. Stub-argv-pinned.
- **D17-1** (design decision): beyond Action 4's three flags, the full ext_tool-convention tool overrides were added (`--clang/--clang-args/--cargo-fuzz/--cargo-fuzz-args`) so flag > env var > PATH applies to the new presets exactly like the existing ones (the user requirement was env-var alignment with the other `rfs test` commands).

## Verification

- `cargo test` 369/369 root (was 361; +6 frontend e2e + 2 ext_tool unit), 393/393 `--workspace` (was 385), 371/371 `--features smt` (was 363).
- Task 7 regression: all 82 pre-existing frontend pins pass UNMODIFIED (unit path byte-for-byte unchanged, incl. the byte-pinned default `--print-script`).
- Real tools (installed per TOOLCHAIN.md section 7): C fuzz on spec 43 = ~500k runs in 3 s, exit 0; Rust fuzz via the temp cargo-fuzz scaffold (nightly) = build + bounded run, exit 0. Both also verified inside `cargo test` (the real-tool e2e runs, not skips).
- Manual `sh` execution of a printed C-fuzz script: exit 0, ~386k runs, all generated artifacts cleaned up by the script's own cleanup step (recorded here per the exit criteria).
- Error paths pinned + verified: missing clang / missing cargo-fuzz with `--suite fuzz` -> exit 1 + diagnostics; fuzzer non-zero exit (stub) -> exit 1; invalid suite name / non-positive budget -> usage errors; default suite unaffected by broken RFS_CLANG.
- clippy (`cargo clippy --all-targets`, root) = 13 warnings (baseline re-pinned: the new `TestSuite::from_str` mirrors the pre-existing `CodegenCommandKind::from_str` warning); `cargo fmt --check` clean.
