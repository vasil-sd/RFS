# Phase 5, Task 1 - Implementation details

> Written for a future LLM session: after reading `START.md`,
> `docs/roadmap/INTRO.md` and this file, the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal
Convert the repository from a single cargo package to a WORKSPACE and ship the
`rfs-build` crate - the build.rs helper that makes "a team can add RFS to an
existing Rust project with build.rs" (phase exit criterion) true.

## Context

- The repo is currently ONE package (`rfs`, `default-run = "rfs"`). All cargo
  aliases live in `.cargo/config.toml` - after the conversion EVERY alias must
  still work (they are referenced throughout BUILD.md and the training data).
- `rfs-build` API shape (planned): `RfsBuild::new(spec_path)` with builder
  options (`.backend("rust"|"c")`, `.production()`, `.no_std()`,
  `.with_property_monitors(bool)`) and `.compile()` - it locates the `rfs`
  executable, runs `codegen <backend> production <spec> --out <OUT_DIR>/<name>.rs`,
  prints `cargo:include=` / `cargo:rerun-if-changed=` directives for the spec
  file (and the rfs binary path), and propagates parse/validation/codegen
  errors as build failures with the rfs diagnostics in the message.
- Toolchain location: overridable via the `RFS_BIN` environment variable
  (user requirement); default = `rfs` on PATH. In-repo development uses the
  workspace-built binary (`CARGO_BIN_EXE_rfs` when built as a workspace member
  is NOT available to build.rs of OTHER crates - document using RFS_BIN or
  PATH for consumer projects).
- Generated code stays self-contained single files (design invariant) -
  rfs-build only orchestrates the toolchain, it adds no dependencies.

## Scope

### Depends on tasks

(none - first task of the phase)

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 10. | ProtocolFsm trait + adapter crate + Rust example crates | the new `crates/rfs-adapters` and example crates live in the workspace created here |
| 12. | C example projects + fuzzing harness examples | example-crate layout established here |

### Affected source files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `Cargo.toml` | change | becomes the workspace root (`[workspace] members = ["crates/rfs-build", ...]`, package section kept for the `rfs` toolchain) |
| `crates/rfs-build/Cargo.toml` | new | rfs-build package manifest (no external deps; edition 2021) |
| `crates/rfs-build/src/lib.rs` | new | RfsBuild API + rfs invocation + rerun directives |

### Affected documentation files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `BUILD.md` | change | workspace layout note: how to build/test the workspace vs the toolchain package; alias re-verification record |

### Affected test files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `crates/rfs-build/tests/` (new dir) | new | unit tests for the API + an integration test that builds a real spec through the helper (using a stub rfs binary via RFS_BIN to keep it hermetic, plus one real end-to-end check) |

## Planned actions

### Action 1: Workspace conversion
Move nothing - make the root `Cargo.toml` a workspace root with the existing
package as an implicit member and add `crates/*` members as they appear.
Re-run every cargo alias from `.cargo/config.toml` (build, test, clippy, fmt,
generate-code-tests, test-generated-code, extract-training-snippets,
build-training, test-training) and the full `cargo test` to prove nothing
moved; record any alias that needs a `--package rfs` qualifier and fix it in
`.cargo/config.toml` (aliases are user-facing - keep them working as-is).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `Cargo.toml` | change | `[workspace]` table added; package section unchanged |
| `.cargo/config.toml` | change (if needed) | alias qualifiers only if the conversion breaks them |

### Action 2: rfs-build crate
Implement `RfsBuild` per Context: spec path validation, backend/command
options, RFS_BIN/PATH resolution, invocation of `rfs codegen <backend>
production <spec> --out OUT_DIR/<module>.rs`, stderr capture for error
propagation, and the cargo directives (`rerun-if-changed` on the spec file;
`include` on the generated artifact). Keep the crate dependency-free.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `crates/rfs-build/Cargo.toml` | new | package manifest |
| `crates/rfs-build/src/lib.rs` | new | the API (single module is enough at this size) |

### Action 3: Tests
Unit tests: option validation, RFS_BIN override precedence over PATH, error
propagation message shape (stub rfs binary written to a temp dir). Integration
test: run the helper against `examples/valid/01_*.rfs` with the real workspace
binary and assert the generated file exists + compiles (`rustc --edition 2021
--crate-type lib`).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `crates/rfs-build/tests/` (new dir) | new | the test files above |

### Action 4: BUILD.md note
Short section: workspace layout, how to build rfs-build, RFS_BIN usage for
consumer projects, alias re-verification record.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `BUILD.md` | change | new "Workspace" note in section 1/4 area |

## Task deliverables
- Cargo workspace with the `rfs` package + `crates/rfs-build`.
- `rfs-build` crate (API + tests, RFS_BIN override).
- BUILD.md workspace notes; all cargo aliases verified working.

## Task exit criteria
- [ ] `cargo build`, `cargo test` (230/230), clippy, fmt green at the WORKSPACE root with unchanged results.
- [ ] Every alias in `.cargo/config.toml` re-run and working (recorded in BUILD.md).
- [ ] `rfs-build` unit + integration tests pass; RFS_BIN override pinned by a test.
- [ ] A consumer-style build.rs using rfs-build compiles a real spec (integration test).
- [ ] No dependency added to the `rfs` toolchain package.

## Non-goals
- Procedural macro crate (PHASE_5.md: "later" - not in this phase).
- Adapter/example crates (tasks 11/12) - only the workspace plumbing they need.
- C-side build helper (C examples use plain Makefile/gcc recipes, task 12).

## Other information
- Check `cargo metadata` behavior after conversion; if the implicit root
  package causes member-resolution surprises, make the root package an
  explicit member too (`members = [".", "crates/*"]` is NOT valid - use
  `members = ["crates/*"]` + keep `[package]` at root, which cargo supports).
- Keep `default-run = "rfs"` working for `cargo run`.

## Task corrections (during execution)

| # | Type | Description + rationale |
|---|------|-------------------------|
| T1-1 | change | The planned `.no_std()` builder option is DEFERRED to task 9: the `--no-std` codegen flag does not exist in the current CLI (task 9 adds it), so shipping the option now would generate commands the toolchain rejects. `RfsBuild` currently exposes `.backend()`, `.production()`, `.with_property_monitors(bool)` - all mapped to existing production-codegen flags; `.no_std()` lands together with task 9. |
| T1-2 | change | Exit-criterion baseline "230/230" predates task 2 (executed first, per the dependency graph): final actuals are `cargo test` **257** (root package) and `cargo test --workspace` **264** (+7 rfs-build). Generated-code aggregate unchanged at 537. |
| T1-3 | fix | The "member-resolution surprise" the Other-information note anticipated DID materialize: the generated aggregate crate under `target/rfs_codegen_tests/` is discovered by cargo walking up from its manifest into the new root workspace and refused to build ("current package believes it's in a workspace when it's not"). Fix at the source: `rust_test.rs` now emits an empty `[workspace]` table in the generated manifest, making the generated crate its own workspace root (robust regardless of where cargo finds it). No change to `.cargo/config.toml` aliases was needed. |
