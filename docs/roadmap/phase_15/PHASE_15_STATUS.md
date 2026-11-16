# Phase 15 - Implementation status

> Written at the task-6 close-out pass, completed after task 2b landed (commit
> 419616f). The phase's packaging + documentation work (tasks 1-5) is complete and
> committed, every Nix-side exit criterion is met, and the canonical sweep is fully
> green on the final source - **the phase is COMPLETE** (close-out re-sweep 2026-11-16,
> task 6 status file). The one count that was blocked at the first close-out pass -
> `cargo test --features smt` failing on `smt_bounded_backend_parity` under host z3
> 5.3.1 - is resolved by task 2b's backend-symmetric z3 resolution in the parity test
> (diagnosis: the recorded failure was not reproducible at HEAD and task 2a's fold
> changed no bytes for the spec; the latent defect was asymmetric solver resolution,
> see task 2b's status file).

## Added source files

| Name | Description |
|---|---|
| - | none (packaging + documentation phase; the only source changes of the phase were task 2a's, recorded there) |

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| - | - | none in tasks 1, 3-6 (task 2a's four `.rs` changes are its own record) |

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `TOOLCHAIN.md` section 10 | changed | Nix-on-Ubuntu-sandbox: installer + single-user vs multi-user (`--daemon`) with when a daemon is required and how to run one manually without systemd, this sandbox's verified nix state (2.35.6 store path, no-daemon single-user-as-root, `sandbox = false`, flakes on, pre-populated store, cache reachable), the dev.to buildRustPackage reference, and the previous preliminary work (commits dd6e0f4/d1c1b31 + the TASKS.md clang-answering-for-gcc finding) |
| `BUILD.md` section 8 | changed | Building, testing and running RFS with Nix: prerequisites pointer, package build (with/without checks) + custom-tool-argument example, the wrapper's env-var contract + override-wins rule + `--print-script` inspection, the dev-shell per-suite command matrix (incl. the nightly Rust-fuzz pipeline via the unwrapped binary + proxy), and a bare-package-vs-shell capability matrix |

## Other file changes

| Name | Operation | Description |
|---|---|---|
| `nix/package.nix` | changed | 12-argument signature (every tool overridable, nixpkgs defaults; mermaid-cli/plantuml/graphviz optional null); runtime tools moved from build-time-only nativeBuildInputs to propagated buildInputs so the installed closure carries them; a postFixup generates `bin/rfs-env.sh` + wraps all five binaries as thin source-and-exec scripts that set the RFS_* contract ONLY when unset (user redefinition wins) |
| `nix/z3-unified.nix` | new | the shared unified-z3-prefix derivation (design decision D3) - one definition imported by both package.nix and shell.nix; byte-identical store path to the former inline copy |
| `shell.nix` | changed | full Ubuntu-parity buildInputs (rustc cargo gcc clang make unifiedZ3 pkg-config cargo-fuzz graphviz plantuml mermaid-cli rustup python3); raw pkgs.z3 dropped for the shared unifiedZ3; identical tool args passed to the package's callPackage; shellHook prepends `<unifiedZ3>/pkgconfig` to PKG_CONFIG_PATH, idempotently installs nightly via rustup, and echoes the resolved tool matrix |

## Changed functionality
- The installed nix package now resolves every external tool deterministically from its
  own closure (no environment setup): `rfs test ...` works from a bare install.
- SMT mode under the dev shell uses the store's unified z3 prefix via pkg-config
  instead of silently falling back to a host-local install.

## Removed functionality
- None.

## New functionality
- Parameterized nix package: every external tool is an overridable argument with a
  nixpkgs default; the diagram tools are optional and, when passed, surface as
  RFS_DOT/RFS_PLANTUML/RFS_MERMAID in the wrapper.
- Runtime tool closure: rustc/cargo/gcc/clang/make/unified-z3/pkg-config/cargo-fuzz are
  propagated dependencies, so a bare install can compile + run generated code and drive
  the SMT solver.
- RFS_* wrapper contract: each of the five binaries sources `bin/rfs-env.sh`, which
  prepends the closure's tool bin dirs to PATH and sets the contract vars only when the
  user has not already defined them (an already-set value always wins).
- Full-parity dev shell: every Ubuntu-runnable suite runs green under `nix develop`
  (workspace, smt, generated-code, rfs test unit/fuzz both backends, example projects,
  diagram parity with the external half active, docs-sync gates).

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|
| - | - | no RFS test sources changed in tasks 1, 3-6 (task 2a added 9 `simplify_expr` unit tests - its own record) |

## Tests run status

Canonical toolchain (START.md section 4 activation, fresh target):

| Name | Status |
|---|---|
| `cargo test` (root) | OK - 557/0 (= Phase 14's 548 + task 2a's 9) |
| `cargo test --workspace` | OK - 685/0 (= 676 + 9) |
| `cargo test --features smt` | OK - **559** passed / 0 failed (re-run green after task 2b; the first close-out pass had recorded 436/1 here - a fail-fast stop after `smt_bounded_test`) |
| `cargo test-generated-code` | OK - 1683/1683 (unchanged - task 2a's fold changed no canonical generated bytes) |
| `cargo clippy --all-targets` | OK - at the Phase 14 baseline (55 warning lines / 34 unique, clippy 1.99.0) |
| `cargo fmt --check` | OK - clean |

Nix side (all exit criteria met):

| Name | Status |
|---|---|
| `nix build --impure --expr 'import ./nix/package.nix { }'` (doCheck=true) | OK - 42/42 checkPhase suites pass |
| closure proof (`nix path-info -r`) | OK - all tool binaries present; no chromium by default |
| wrapper matrix (bare-install unit both backends, C fuzz, override-wins via --print-script, optional diagram vars set/unset) | OK |
| `nix develop` full-suite matrix (task 4's eight commands) | OK - workspace 685/0, smt 559/0 (nix z3 5.3.2), generated-code 1683/0, unit both backends exit 0, C fuzz exit 0, example_projects 3/0, diagram parity 13/0 external half active, docs gates 9/0 + 5/0 |
| `rfs-dev doc-section TOOLCHAIN.md '10. ...'` + `BUILD.md '8. ...'` | OK - both return their sections (exit 0) |
| `rfs-dev sync-check` + `inventory audit` | OK - clean / EXACT (no source/doc-pointer drift) |

## Found problems/issues/mismatches
- **Task-2a SMT-bounded backend-parity regression (host z3 5.3.1) - RESOLVED by task 2b:**
  the first close-out pass recorded `smt_bounded_backend_parity` failing under the
  canonical host toolchain (rust=PASS-WITHIN-BOUND, c=FAIL on the fail variant; 436/1,
  a fail-fast stop after `smt_bounded_test`). Task 2b's diagnosis: the failure is not
  reproducible at HEAD in any constructible host environment and all four generated
  artifacts for the spec are byte-identical between pre-task-2a source (17d22bc) and
  HEAD - task 2a's fold changed no bytes for it. The real latent defect was the parity
  test's ASYMMETRIC z3 resolution (C pkg-config-first vs Rust z3-sys probe order vs
  SONAME-filename runtime lookup), which could put the two backends on different solvers.
  Task 2b made the resolution backend-symmetric (shared loadability-probed
  `resolved_z3()`); the canonical sweep is now 559/0 and nix stays 42/42 + 559/0.
- **BUILD.md section 5 stale baseline (pre-existing, not fixed here):** its "known-good
  baseline" text still shows 538/666/540 (re-measured 2026-11-12, pre-Phase-14). The true
  Phase 14 close-out numbers are 548/676/550, and with task 2a's 9 tests the current
  canonical counts are 557/685/559. Left as-is per this phase's non-goal; flagged for a
  future docs-refresh task.

## Other important findings/information
- The +9 over Phase 14 in every cargo count is task 2a's nine `simplify_expr` unit tests
  (not gated tests) - they run identically on host and nix.
- Design decisions as implemented: D1/D2 (wrapper mechanism = one shared generated
  `rfs-env.sh` preamble sourced by all five thin wrappers, conditional exports so user
  redefinition wins); D3 (shared `nix/z3-unified.nix`, NOT the fallback duplication).
- The Rust fuzz pipeline works under nix only via the unwrapped `rfs.real` + rustup proxy
  on PATH (the wrapped binary's store-cargo PATH prepend defeats cargo-fuzz's nightly pin)
  - a task-3 wrapper follow-up, documented in BUILD.md section 8.4 and plan correction 4.
- Close-out re-sweep environmental artifacts (recorded in the task 6 status file, not
  code defects): (1) cross-toolchain target-directory contamination - host and nix
  develop sharing one `target/` makes `nostd_minimal.rs` fail E0514 (cargo's fingerprint
  treats both rustc 1.99.0 builds as identical; the test's direct rustc invocation sees
  incompatible metadata); fresh-target discipline resolves it. (2) Chrome "Socket path
  too long" under `nix develop` in this sandbox (deep nix-shell temp dir) breaks the
  external mmd half of `diagram_render_parity_test` unless TMPDIR is short.
