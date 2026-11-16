# Phase 15, Task 4 - Implementation status

> Written once when all exit criteria of `PHASE_15_TASK_4.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

| Name | Description |
|---|---|
| `nix/z3-unified.nix` | the shared unified-z3-prefix derivation (design decision D3): `{ pkgs, z3 }: pkgs.runCommand ...` with the runCommand body + rationale comment moved VERBATIM from package.nix. Exported as a bare lambda (no trailing `;` - see corrections). Imported by BOTH package.nix and shell.nix so there is one definition and zero drift. |

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| `nix/package.nix` | changed (`24:68`) | the inline unifiedZ3 runCommand (comment + body) replaced by `unifiedZ3 = import ./z3-unified.nix { inherit pkgs z3; };` in the let block. Behavior identical - the extracted derivation produces the SAME store path as before (see Verification). |
| `shell.nix` | changed (whole file) | full Ubuntu-parity buildInputs (`rustc cargo gcc clang make unifiedZ3 pkg-config cargo-fuzz graphviz plantuml mermaid-cli rustup python3`); raw `pkgs.z3` dropped in favour of the shared `unifiedZ3`; the tool arguments are bound ONCE as let-vars and passed to BOTH buildInputs and the package's `callPackage` (identical derivations - shell and package agree); shellHook now (a) prepends `<unifiedZ3>/pkgconfig` to PKG_CONFIG_PATH so `pkg-config z3` resolves to the nix store z3 (mkShell does not scan that non-standard location), (b) idempotently installs the nightly toolchain via rustup, and (c) echoes the resolved tool matrix. |

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| - | - | none (BUILD.md's nix section is task 5) |

## Other file changes

| Name | Operation | Description |
|---|---|---|
| - | - | none |

## Changed functionality
- The dev shell now provides EVERY external tool a runnable suite needs, so every
  test suite that runs on Ubuntu also runs green under `nix develop -f shell.nix`.
  Raw `pkgs.z3` (split out/lib/dev outputs, no single prefix) is gone; the shared
  unified prefix backs SMT mode in both the shell and the package.

## Removed functionality
- None. (The old shell's four-tool buildInputs are a strict subset of the new set.)

## New functionality
- Full Ubuntu-parity tool set on PATH: make, graphviz (`dot`), plantuml,
  mermaid-cli (`mmdc`), cargo-fuzz, rustup (+ nightly), python3 - in addition to the
  existing gcc/clang/rustc/cargo/pkg-config. The diagram parity test's external half
  is now ACTIVE (all three renderers resolve; mmdc renders via the store chromium).
- PKG_CONFIG_PATH exposes the unified z3 prefix's generated `z3.pc`, so SMT mode runs
  with the nix store z3 (explicit=false, z3-sys reads the real 5.3.2 from headers)
  instead of silently falling back to a host-local install.
- The Rust fuzz pipeline's nightly is installed idempotently by the shellHook and is
  present after first entry (cached in the profile dir afterwards).

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|
| - | - | no test files changed; verification is the nix-side matrix below |

## Tests run status

All under `nix develop -f shell.nix --impure --command sh -c '...'` (TMPDIR=/tmp;
see corrections for why). Counts are pass/fail.

| Command | Result | Note |
|---|---|---|
| `cargo test --workspace` | **685 / 0** | baseline 676; the +9 are tool-availability-gated tests (diagram external half, clang/gcc compile legs, example-project builds) that skip when their tools are absent and now RUN with the full nix tool set - task 4's explicit goal. Clean `CARGO_TARGET_DIR` required (see corrections: shared target/ had mixed host/nix rlibs -> E0514). |
| `cargo test --features smt` | **559 / 0** | baseline 550; same +9 gated-test delta. SMT tests RUN, not skip. `smt_bounded_test` 2/2 (both backends agree) via pkg-config z3 5.3.2 - the unified prefix. |
| `cargo test-generated-code` | **1683 / 0** | exact match to baseline. |
| `rfs test --suite unit --backend rust` (02_counter.rfs) | exit 0 | 9 generated tests pass (rustc backend). |
| `rfs test --suite unit --backend c` (02_counter.rfs) | exit 0 | all generated C tests pass (gcc backend). |
| `rfs test --suite fuzz --backend c` (5 s budget) | exit 0 | 443,964 runs, no crash - closure clang + libFuzzer runtime. |
| `rfs test --suite fuzz --backend rust` (5 s budget) | exit 0 | via nightly + cargo-fuzz - see the Rust-fuzz correction below for the wrapped-vs-unwrapped nuance. |
| `cargo test --test example_projects_test` | **3 / 0** | c_sync_handler (gcc+make), fuzz_c (clang-gated build), fuzz_rust (cargo-check). |
| `cargo test --test diagram_render_parity_test` | **13 / 0** | EXTERNAL half ACTIVE: parity_external_{mmd,dot,puml}_pinned_subset all pass (dot/plantuml/mmdc resolve; mmdc renders via wired chromium, serialized). |
| `cargo test --test dev_docs_test` | **9 / 0** | pure Rust docs-sync gate. |
| `cargo test --test docs_md_test` | **5 / 0** | pure Rust docs-sync gate. |
| package `doCheck` build (clean sandbox) | **42 / 42** | re-confirmed after the z3-unified extraction; smt_bounded_backend_parity + smt_bounded_runtime_bound_flag both ok under pkg-config z3 5.3.2. |

## Found problems/issues/mismatches
- **Shared `target/` cross-contamination (E0514).** The repo's `target/` held FOUR
  `librfs_adapters-*.rlib` from mixed host/nix builds (different rustc versions).
  `nostd_minimal.rs` picks the first such rlib in non-deterministic `read_dir` order
  and compiles a harness against it with `rustc_path()`; grabbing a mismatched rlib
  yields `E0514: found crate rfs_adapters compiled by an incompatible version of
  rustc`. Not a nix defect - a shared-target artifact. Fixed for verification by
  isolating `CARGO_TARGET_DIR=/tmp/rfs-nix-target` (workspace run) -> clean 685/0.
- **`CARGO_TARGET_DIR` breaks `smt_bounded_test`.** The test's scratch cargo project
  hardcodes its binary path as `<proj>/target/release/smt_gen`, but setting
  `CARGO_TARGET_DIR` redirects the actual output elsewhere -> `run_bin` gets a
  nonexistent path (`NotFound`). So `smt_bounded_test` must run with the DEFAULT
  target dir (no `CARGO_TARGET_DIR`), which is fine because it does not touch the
  rfs-adapters nostd rlib that caused E0514.
- **Stale scratch `smt_gen` masked a real pass.** The first smt run built the scratch
  Rust binary with `RFS_Z3_DIR=<unified>` (explicit=true -> the test's hardcoded
  `Z3_SYS_Z3_VERSION=5.3.1` pin, wrong for nix's 5.3.2). Cargo did not rebuild it on
  later runs (identical source; build-script env vars are not fingerprinted), so the
  wrongly-pinned binary was reused and `smt_bounded_backend_parity` disagreed
  (rust=PASS, c=FAIL). Wiping `target/smt_e2e_*` + running with pkg-config-driven z3
  (explicit=false, no pin) makes it pass - matching the doCheck build.
- **Rust fuzz through the WRAPPED binary is blocked by the wrapper's PATH prepend.**
  cargo-fuzz re-invokes bare `cargo` for its build; task 3's `rfs-env.sh` prepends the
  nix store stable `cargo`/`rustc` to PATH, so that internal call resolves to stable
  and rejects `-Zsanitizer` (the scaffold's nightly pin is only honoured by the rustup
  PROXY). The machinery itself is fully functional under nix: invoking the UNWRAPPED
  `rfs.real` with the rustup proxy bin dir first on PATH gives exit 0 (nightly +
  cargo-fuzz + libFuzzer all present and working). This is a task-3 wrapper concern
  (route the fuzz pipeline's internal cargo through the proxy), not a shell.nix defect;
  recorded as a follow-up. The C fuzz backend works fully through the wrapped binary.

## Other important findings/information
- **The unifiedZ3 extraction is byte-identical.** The extracted derivation produces the
  SAME store path as the pre-extraction inline one (`pfd67fcnr9idd2p48sqkj51y0l5sy6zn-
  z3-unified-5.3.2`), proving the build script is unchanged. The PACKAGE store path did
  change (`yzdgas...` -> `dwx2zs...`) solely because `nix/z3-unified.nix` was added to
  the repo and thus to `cleanSource`'s hash - a source-input change, not a behavioral
  one (the doCheck build still passes 42/42).
- **The shell's callPackage agrees with the package.** Passing the identical tool
  arguments (`inherit rustc cargo gcc clang make z3 pkg-config cargo-fuzz mermaid-cli
  plantuml graphviz`) yields the same store path as the on-PATH `rfs`, and its
  `rfs-env.sh` carries exactly the three diagram vars (RFS_DOT/RFS_PLANTUML/RFS_MERMAID)
  because the optional args were passed - default-args builds carry none.
- **TMPDIR must be short for mmdc.** This sandbox's nix-shell temp dir is deeply nested
  (`/opt/unsloth-studio/sandbox/__LOCALID.../unsloth-tmp/nix-shell.XXXX/`), so chromium's
  `SingletonSocket` path exceeds the 108-char unix-socket limit and mmdc fails to launch.
  A normal Ubuntu nix uses `/tmp/nix-shell.XXXX` (short) and is unaffected. Verification
  ran with `TMPDIR=/tmp`; this is a sandbox artifact, not a shell.nix defect - left out of
  the shellHook deliberately so the definition stays correct for normal systems.
- **First `nix develop` entry needs network** (nightly download ~200 MB); subsequent
  entries are fast (cached in `~/.rustup`). The C-side suites run regardless if the
  nightly download fails offline.
- **Host `/workspace/host/z3` (5.3.1) exists on this machine**, so cargo's env.toml
  non-forcing `RFS_Z3_DIR` default would resolve to it and mask the nix z3. For the SMT
  verification I forced pkg-config discovery (`RFS_Z3_DIR=/nonexistent...`) to prove the
  NIX unified prefix (5.3.2) works end-to-end, not the host install.
