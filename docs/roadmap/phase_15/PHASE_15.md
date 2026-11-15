# Phase 15: Full Nix development support

## Goal
Make Nix a first-class, fully operational way to build, test and run RFS on any
Linux system - not just a partial dev shell. The `rfs` binary produced by
`nix/package.nix` must carry its complete external-tool closure (compilers,
z3, make, cargo-fuzz) so that `rfs test ...` works out of the box from the
installed package; the package exports a wrapper script that sets every
required `RFS_*` environment variable while still honoring user redefinition;
and `shell.nix` covers the ENTIRE test suite that runs on Ubuntu (unit + fuzz
tests, generated-code tests, SMT mode, example projects, diagram checks,
docs-sync gates), so nothing that passes here is skipped under Nix.

```
nix build  nix/package.nix  ->  rfs binary + full tool closure + RFS_* wrapper
nix develop -f shell.nix    ->  every Ubuntu-runnable test suite, green
```

## Scope
1. `TOOLCHAIN.md`: a new explicit section documenting how to set up the Nix
   package manager on this Ubuntu sandbox (installer, single-user vs
   multi-user/`--daemon` modes and when a daemon is needed, the current
   sandbox state, usage commands), referencing the dev.to buildRustPackage
   guide and the previous preliminary nix work recorded in `TASKS.md`
   (commits dd6e0f4 / d1c1b31) and its known findings.
2. `nix/package.nix`: parameterize every external tool as a package argument
   (`{ pkgs ? import <nixpkgs> {}, rustc ? pkgs.rustc, cargo ? pkgs.cargo,
   gcc ? pkgs.gcc, clang ? pkgs.clang, make ? pkgs.gnumake, z3 ? pkgs.z3,
   pkg-config ? pkgs.pkg-config, cargo-fuzz ? pkgs.cargo-fuzz,
   mermaid-cli ? null, plantuml ? null, graphviz ? null }`) so users can pass
   their own tool versions; move the tools that `rfs test ...` needs at RUN
   TIME (rustc, cargo, gcc, clang with the libFuzzer runtime, make, z3 via the
   unified prefix, pkg-config, cargo-fuzz) from build-time-only
   `nativeBuildInputs` to propagated dependencies, so the built package's
   closure contains them; keep `mermaid-cli`, `plantuml` and `graphviz`
   OPTIONAL - when passed (non-null) they become dependencies AND the wrapper
   sets `RFS_MERMAID` / `RFS_PLANTUML` / `RFS_DOT` accordingly.
3. `nix/package.nix`: export a wrapper script for the package binaries that
   sets all required `RFS_*` variables (`RFS_RUSTC`, `RFS_GCC`, `RFS_CLANG`,
   `RFS_CARGO_FUZZ`, `RFS_MAKE`, `RFS_Z3_DIR` + the z3-sys build vars and
   `PKG_CONFIG_PATH`) - each set ONLY when not already defined by the user,
   so user redefinition always wins (this also eliminates the recorded
   "clang answering for gcc" tool-resolution artifact).
4. `shell.nix`: extend the dev shell to cover every test suite that runs on
   Ubuntu - add make, graphviz, plantuml, mermaid-cli (with its wired
   chromium), cargo-fuzz, rustup + a nightly toolchain (for the Rust fuzz
   pipeline), python3, and the unified z3 prefix (the current shell passes raw
   split `pkgs.z3`, which the resolvers cannot use for SMT mode); keep the
   built `rfs` binary + full toolchain closure via `inputsFrom`.
5. `BUILD.md`: a new explicit section - how to build, test and run RFS on Nix
   (package build with/without checks, dev shell usage, what the wrapper sets,
   how to override tools by passing package arguments, what works from the
   bare package vs what needs the dev shell).

## Tool-resolution contract (what the wrapper must satisfy)
The resolvers (`src/cli/ext_tool.rs`, `tests/common/toolchain.rs`) follow one
precedence: CLI flag > `RFS_*` env var > standard Unix var > bare name on PATH
(pkg-config for z3). The wrapper therefore sets, conditionally on being unset:

| Variable | Value (store path) | Purpose |
|---|---|---|
| `RFS_RUSTC` | rustc bin | Rust codegen/test compilation |
| `RFS_GCC` | gcc bin | C codegen/test compilation, example Makefiles |
| `RFS_CLANG` | clang bin | libFuzzer C fuzz pipeline |
| `RFS_CARGO_FUZZ` | cargo bin (with cargo-fuzz on PATH) | Rust fuzz pipeline scaffold |
| `RFS_MAKE` | make bin | C example projects |
| `RFS_Z3_DIR` | unified z3 prefix | SMT mode discovery (first tier) |
| `Z3_SYS_Z3_DIR`, `Z3_LIBRARY_PATH_OVERRIDE` | unified z3 prefix / its bin | z3-sys linking at user build time |
| `PKG_CONFIG_PATH` | unified z3 pkgconfig dir (prepended) | `pkg-config z3` discovery tier |
| `RFS_DOT` / `RFS_PLANTUML` / `RFS_MERMAID` | graphviz / plantuml / mermaid-cli bins, ONLY when the optional args are passed | external diagram rendering |

## Deliverables
- `TOOLCHAIN.md` new section (Nix on the Ubuntu sandbox: install, daemon
  modes, current state, usage) - changed file.
- `nix/package.nix` - parameterized tool arguments, propagated runtime tool
  dependencies, optional diagram-tool arguments, RFS_* wrapper script -
  changed file.
- `shell.nix` - full-test-suite dev shell (all Ubuntu-runnable suites) -
  changed file.
- `BUILD.md` new section (build/test/run RFS on Nix) - changed file.
- `docs/roadmap/phase_15/PHASE_15_STATUS.md` - phase completion report.

## Exit criteria
- [ ] `TOOLCHAIN.md` contains the explicit Nix-on-Ubuntu-sandbox section:
      installer command, single-user vs multi-user (`--daemon`) explanation
      with when a daemon is required and how to run one manually in a
      container without systemd, this sandbox's current nix state (version,
      store path, no-daemon single-user mode, `sandbox = false`, flakes on),
      the dev.to buildRustPackage reference, and the previous preliminary nix
      work (commits + TASKS.md findings) recorded.
- [ ] `nix/package.nix` accepts every external tool as an overridable package
      argument with a nixpkgs default; `mermaid-cli`, `plantuml`, `graphviz`
      are optional (default null).
- [ ] The built package's runtime closure contains the `rfs` binary AND the
      rustc, cargo, gcc, clang, make, z3 (unified prefix) and pkg-config tool
      binaries (`nix path-info -r` proof); `nix build` with `doCheck = true`
      passes all checkPhase suites.
- [ ] The package exports a wrapper that sets every variable in the contract
      table above; a pre-set user `RFS_*` variable is NOT overridden (verified
      by running the wrapped binary with and without pre-set vars and checking
      `--print-script` output); when the optional diagram args are passed,
      `RFS_DOT`/`RFS_PLANTUML`/`RFS_MERMAID` are set, and when they are null
      the variables stay unset.
- [ ] From the nix-built package alone (no dev shell): `rfs test --suite unit`
      passes for both backends on an example spec; `rfs test --suite fuzz`
      C backend passes with the closure's clang; SMT-mode compilation of
      generated C works against the unified z3.
- [ ] `shell.nix` provides every tool needed by every Ubuntu-runnable suite:
      `cargo test`, `cargo test --workspace`, `cargo test --features smt`,
      `cargo test-generated-code`, `rfs test` unit+fuzz (both backends),
      `example_projects_test`, `diagram_render_parity_test` with the external
      half ACTIVE (dot/plantuml/mmdc all resolve), and the docs-sync gates
      (`dev_docs_test`, `docs_md_test`) - verified by running them inside
      `nix develop`.
- [ ] `BUILD.md` has the explicit Nix section: build (package, with/without
      checks), test (checkPhase + dev-shell suites), run (wrapped binary,
      wrapper env vars, tool overrides via package arguments).
- [ ] Canonical-toolchain verification unchanged and green (no RFS source
      changes in this phase): `cargo test` 548, `--workspace` 676,
      `--features smt` 550, generated-code 1683 - the Phase 14 close-out
      baselines (BUILD.md section 5's 538/666/540 predate Phase 14's 13 new
      tests) - fmt clean, clippy at baseline.
- [ ] `TASKS.md` entry appended; `CURRENT_STATE.md` phase-15 row updated;
      `PHASE_15_STATUS.md` written per FORMATTING.md §4.

## Non-goals
- No `flake.nix` / flake outputs - the project stays on non-flake Nix
  (`--impure`, `import <nixpkgs>`); a flake is a possible later phase.
- No RFS source, language, IR or codegen changes - this phase is packaging +
  documentation only; any required source fix enters via plan corrections.
- No hermetic nightly Rust toolchain in the package closure - the Rust fuzz
  pipeline's nightly requirement is met by rustup inside the dev shell and is
  documented as such (the C fuzz pipeline needs no nightly).
- No expansion of the package `checkPhase` test selection beyond the current
  10 MC suites + `smt_bounded_test`.
- No CI system integration (GitHub Actions etc.) - future phase.
- No changes to `.cargo/env.toml` or the resolver precedence in source.
