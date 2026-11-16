# Phase 15, Task 2b - Implementation status

> Written once when all exit criteria of `PHASE_15_TASK_2B.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

| Name | Description |
|---|---|
| - | none (new functions were added to the existing test file) |

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| `tests/smt_bounded_test.rs` | changed | z3 resolution made backend-symmetric: new `resolved_z3()` (OnceLock-cached) resolves the usable z3 prefix via the unified resolver and PROBES an explicit install for loadability (`z3_prefix_runs()`: compile + run a no-op against the prefix; exit 0 = usable), falling back to the pkg-config tier when the explicit prefix cannot load in this environment (the host-local RFS_Z3_DIR default under `nix develop`); `z3_compile_args()` now compiles an explicit install against its own prefix (-I include, -L lib + bin) instead of preferring raw pkg-config (which shadowed it with the system z3 4.8.12), keeping pkg-config's own flags only for auto-discovered z3; `build_rust()` sets `Z3_NO_PKG_CONFIG=1` for explicit installs so z3-sys links through Z3_LIBRARY_PATH_OVERRIDE only - the same prefix C uses (z3-sys 0.13.1 no longer honors Z3_SYS_Z3_DIR, and its pkg-config -L otherwise wins the link order); `run_bin()` points LD_LIBRARY_PATH at the SAME resolved prefix instead of a raw z3_dir() |

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `docs/llm/TEST_STRUCTURE.md` | changed | 2 new Function rows (`resolved_z3`, `z3_prefix_runs`) + updated depends/description on the `z3_compile_args`, `build_rust`, `run_bin` rows (all now depend on `resolved_z3`); audit EXACT at TEST 978 fns / 34 types |

## Other file changes

| Name | Operation | Description |
|---|---|---|
| - | - | no example/training/build-file/nix-packaging changes (task non-goal honored) |

## Changed functionality
- The SMT-bounded parity test's z3 resolution: both backends now ALWAYS link and
  run the same solver. On the canonical host that is the project-local z3 5.3.1
  (previously BOTH backends silently ran on the system z3 4.8.12 - pkg-config's
  bare `-lz3` won for C, and z3-sys' pkg-config `-L` outranked
  Z3_LIBRARY_PATH_OVERRIDE for Rust). Under `nix develop` the host-local
  RFS_Z3_DIR default from .cargo/env.toml still exists (host fs mounted) but its
  libz3.so cannot load under the shell's glibc - the new probe catches that and
  falls back to the pkg-config tier (the nix unified z3 5.3.2), restoring the
  pre-task behavior there.

## Removed functionality
- The pkg-config-FIRST preference in `z3_compile_args()` for explicit installs
  (it violated TOOLCHAIN.md section 8.1's single precedence model - RFS_* vars
  must win over auto-discovery).

## New functionality
- `resolved_z3()` + `z3_prefix_runs()`: one shared, loadability-probed z3
  resolution for all three consumers (C compile flags, Rust scratch build, run
  LD_LIBRARY_PATH) - the two backends can no longer end up on different solvers,
  which is what makes status/label/step parity hold under ANY solver version.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|
| `tests/smt_bounded_test.rs` (2 tests) | changed (infrastructure only) | same tests, same pinned expectations - only the z3 resolution they build/run against changed; no test logic or assertion touched |

## Tests run status

Canonical toolchain (START.md section 4 activation):

| Name | Status |
|---|---|
| `cargo test --features smt --test smt_bounded_test` (fresh scratch) | OK - 2/0; ldd proof: BOTH binaries load `/workspace/host/z3/bin/libz3.so` (5.3.1), DT_NEEDED=libz3.so on both |
| `cargo test` (root) | OK - **557** passed / 0 failed |
| `cargo test --features smt` | OK - **559** passed / 0 failed (the count task 6 blocked on) |
| `cargo test --workspace` | OK - **685** passed / 0 failed |
| `cargo test-generated-code` | OK - exactly **1683/1683** (unchanged - no codegen source touched) |
| `cargo clippy --all-targets` | At baseline: 55 warning lines / 0 errors (identical to the Phase 14 close-out measurement, clippy 1.99.0) |
| `cargo fmt --check` | OK - clean (after one rustfmt pass over the new probe code) |
| `rfs-dev inventory audit` | OK - EXACT: SRC 964 fns / 200 types, TEST 978 fns / 34 types |
| `rfs-dev sync-check` | OK - clean (exit 0) |

Nix side:

| Name | Status |
|---|---|
| `nix build --impure --expr 'import ./nix/package.nix { }'` (doCheck=true, final source) | OK - **42/42** checkPhase tests pass; both smt_bounded_test tests green under nix z3 5.3.2 |
| `nix develop -f shell.nix -c 'cargo test --features smt'` (full suite, short TMPDIR) | OK - **559/0**; ldd proof: BOTH binaries load the store's unified `libz3.so.5.3` (5.3.2) via the probe fallback |

## Found problems/issues/mismatches
- **Task 6's recorded failure is not reproducible at HEAD in any constructible
  host environment.** All four generated artifacts for SMT_FAIL_SRC (Rust/C x
  production-FSM/MC-driver) are BYTE-IDENTICAL between pre-task-2a source
  (17d22bc) and HEAD - task 2a's fold changed no bytes for this spec (its guard
  `!(x == 0) || x <= 3` is not a foldable form), so "the fold introduced a
  z3-version-sensitive divergence" is not supported at the encoding level. The
  parity test passed under: canonical env, TOOLCHAIN.md section 5.2 env, fully
  fresh scratch builds, a 30-run flakiness check, and all four link/runtime
  combinations of the two host-usable solvers (system 4.8.12 / project-local
  5.3.1). The 436/1 count in the task-6 record is exactly a fail-fast stop after
  smt_bounded_test (cumulative 435 + 1 pass + 1 fail), consistent with the
  record but not re-producible.
- **The real latent defect (fixed here): asymmetric z3 resolution.** C preferred
  raw pkg-config (system 4.8.12 on this host, ignoring RFS_Z3_DIR - inverting
  TOOLCHAIN.md section 8.1's precedence model); Rust went through z3-sys' own
  probe order where pkg-config's `-L/usr/lib/...` is emitted BEFORE
  Z3_LIBRARY_PATH_OVERRIDE's `-L`; and runtime selection is by SONAME filename
  - the project-local install ships only unversioned `libz3.so`, so any binary
  needing `libz3.so.4` (the system SONAME) falls through to system 4.8.12
  regardless of LD_LIBRARY_PATH. The two backends could silently run on
  DIFFERENT solvers - a parity gate comparing different solvers is unsound, and
  any environment drift (pkg-config state, leaked PKG_CONFIG_PATH/LD_LIBRARY_PATH
  from an adjacent nix session) flips which solver each side gets.
- **z3-sys 0.13.1 no longer honors Z3_SYS_Z3_DIR** (zero references in the
  crate); build_rust set it as a silent no-op. Only Z3_LIBRARY_PATH_OVERRIDE
  (extra -L) and Z3_SYS_Z3_VERSION (version-metadata override, which MASKS the
  pkg-config-detected 4.8.12 as 5.3.1 in build metadata) take effect.
- **The first explicit-first fix broke `nix develop`** (both smt tests exit 127):
  under the shell, RFS_Z3_DIR=/workspace/host/z3 still exists (host fs mounted)
  so the resolver trusted it, but its libz3.so cannot load under the shell's
  glibc. Fixed with the `z3_prefix_runs()` loadability probe + pkg-config-tier
  fallback in `resolved_z3()` - nix develop then runs both backends on the store
  unified z3 5.3.2 exactly as before this task.
- **Pre-existing, unrelated:** `diagram_render_parity_test`'s external mmd half
  fails under `nix develop` in THIS sandbox with Chrome "Socket path too long"
  (the nix-shell temp dir is too deep for Chrome's SingletonSocket); passes with
  a short TMPDIR. Not caused by this task; flagged for the environment owner.

## Other important findings/information
- The canonical host run now ACTUALLY uses the project-local z3 5.3.1 for both
  backends - the documented intent of TOOLCHAIN.md section 5 ("do not pin Rust
  to a different solver build than the C backend links against"). Before this
  task both sides silently ran on system 4.8.12, which happened to agree on all
  three specs.
- Nix behavior is byte-for-byte unchanged: the explicit=false path (pkg-config
  auto-discovery) emits identical flags to before, proven by doCheck 42/42 and
  nix-develop 559/0.
- Task 6 (phase close-out) is unblocked: its only pending criterion was the
  canonical `cargo test --features smt` = 559/0, now met.
