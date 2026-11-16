# Phase 15, Task 6 - Implementation status

> Written once when all exit criteria of `PHASE_15_TASK_2B.md`'s dependent task 6
> are reached. The original close-out pass (commit c04a63f) ran the full sweep,
> wrote `PHASE_15_STATUS.md`, and stopped PENDING on plan correction 2b - the
> canonical `--features smt` count was blocked at 436/1. Task 2b landed
> (commit 419616f); this close-out re-ran the complete sweep on the final source
> and marked the phase COMPLETE.

## Added source files

| Name | Description |
|---|---|
| - | none (verification + bookkeeping task) |

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| - | - | none |

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `docs/roadmap/phase_15/PHASE_15_STATUS.md` | changed | PENDING-on-2b state markers updated to COMPLETE: header, the canonical `--features smt` row (now 559/0), the task-2a regression finding marked resolved by task 2b, close-out re-sweep counts + the two environmental artifacts recorded |
| `docs/roadmap/CURRENT_STATE.md` | changed | phase-15 row: IN PROGRESS (PENDING on 2b) -> COMPLETE with the close-out summary |

## Other file changes

| Name | Operation | Description |
|---|---|---|
| `TASKS.md` | change (append) | one entry per its first-paragraph template |

## Changed functionality
- None (verification + bookkeeping).

## Removed functionality
- None.

## New functionality
- None.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|
| - | - | none |

## Tests run status

Canonical toolchain (START.md section 4 activation, final source after task 2b):

| Name | Status |
|---|---|
| `cargo test` (root) | OK - **557** passed / 0 failed |
| `cargo test --features smt` | OK - **559** passed / 0 failed (the previously blocked count; task 2b's fix) |
| `cargo test --workspace` | OK - **685** passed / 0 failed |
| `cargo test-generated-code` | OK - exactly **1683/1683** |
| `cargo clippy --all-targets` | At baseline: 55 warning lines / 0 errors (clippy 1.99.0) |
| `cargo fmt --check` | OK - clean |

Nix side (re-confirmed end to end on the final source):

| Name | Status |
|---|---|
| `nix build --impure --expr 'import ./nix/package.nix { }'` (doCheck=true) | OK - **42/42** checkPhase suites pass |
| closure proof (`nix path-info -r` on the final store path) | OK - rustc/cargo 1.99.0, gcc 16.2.0, clang 21.1.8 (+compiler-rt), gnumake, unified z3 5.3.2 (lib+bin), pkg-config, cargo-fuzz 0.13.2 all present; **no chromium** by default |
| wrapper matrix | OK - bare-install `rfs test --suite unit` both backends exit 0 in a clean env; C fuzz 457k runs / no crash / exit 0; pre-set `RFS_GCC=/usr/bin/gcc` wins via `--print-script`; default build leaves RFS_DOT/RFS_PLANTUML/RFS_MERMAID unset, optional-args build sets all three to store bins |
| `nix develop -f shell.nix` full-suite matrix (fresh target) | OK - workspace **685/0**, smt **559/0** (nix z3 5.3.2), generated-code **1683/0**, unit both backends exit 0, C fuzz exit 0, example_projects **3/0**, diagram parity **13/0** external half active, docs gates **9/0 + 5/0** |
| `rfs-dev doc-section TOOLCHAIN.md '10. ...'` + `BUILD.md '8. ...'` | OK - both return their sections (exit 0) |
| `rfs-dev sync-check` + `inventory audit` | OK - clean / EXACT (SRC 964 fns / 200 types, TEST 978 fns / 34 types) |

## Found problems/issues/mismatches
- **Cross-toolchain target-directory contamination (workflow artifact, not a code
  defect):** running the host canonical sweep and `nix develop` against the SAME
  `target/` makes `rfs-adapters`' `nostd_minimal.rs` fail with E0514 - cargo's
  fingerprint treats both rustc 1.99.0 builds (rustup vs nix source-tarball) as
  identical and skips the rebuild, but the test's direct `rustc` invocation sees
  incompatible crate metadata in the host-built rlib. Resolved with fresh-target
  discipline (`rm -rf target` before switching toolchains); workspace then runs
  685/0 under nix develop. Recorded so a future session does not misread it as a
  regression.
- **Chrome "Socket path too long" under `nix develop` in THIS sandbox** (pre-
  existing, unrelated): the nix-shell temp dir is too deep for Chrome's
  SingletonSocket, so `diagram_render_parity_test`'s external mmd half fails
  unless TMPDIR points at a short path. Passes with `TMPDIR=/tmp/mmd`; flagged
  for the environment owner.

## Other important findings/information
- The phase's only source changes remain task 2a's four `.rs` files (fold +
  driver zero-init) and task 2b's test-infrastructure fix; every canonical count
  is exactly the re-stated baseline (557/685/559/1683), proving nothing else moved.
- BUILD.md section 5's stale "known-good baseline" text (538/666/540,
  pre-Phase-14) was left as-is per this task's non-goal; it is flagged in
  `PHASE_15_STATUS.md` for a future docs-refresh task.
