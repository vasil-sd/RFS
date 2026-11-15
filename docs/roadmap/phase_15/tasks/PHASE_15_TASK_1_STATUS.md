# Phase 15, Task 1 - Implementation status

> Written once when all exit criteria of `PHASE_15_TASK_1.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up phase plans.

## Added source files

| Name | Description |
|---|---|
| (none) | docs-only task - no source files |

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| (none) | - | no source changes |

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `TOOLCHAIN.md` | changed | new §10 "Nix package manager on this Ubuntu sandbox" appended after §9 (file grew 418 -> 514 lines): 10.1 install (official installer one-liner, multi-user daemon vs single-user `--no-daemon`, what the daemon is + when required + manual `nix-daemon &` in a systemd-less container), 10.2 current sandbox state table (2.35.6 store path, not on PATH, no daemon / single-user-as-root works, nix.conf contents, pre-populated store, cache.nixos.org reachable, nixpkgs 26.11pre via channel profile), 10.3 usage (PATH export + the three project commands with `--impure` and why it is required), 10.4 previous work (dd6e0f4, d1c1b31, the TASKS.md clang-answering-for-gcc finding with pointer to task 3 as its resolution) + the dev.to buildRustPackage reference link |

## Other file changes

| Name | Operation | Description |
|---|---|---|
| (none) | - | - |

## Changed functionality
- None (documentation only).

## Removed functionality
- None.

## New functionality
- `TOOLCHAIN.md` §10: a self-sufficient reference for setting up and driving Nix on this Ubuntu sandbox - future sessions no longer need to re-discover the store-path binary location, the no-daemon single-user mode, the `--impure` requirement, or the history of the preliminary nix work.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|
| (none) | - | no test changes; verification used rfs-dev doc-section/sync-check/inventory audit |

## Tests run status

| Name | Status |
|---|---|
| `rfs-dev doc-section TOOLCHAIN.md '10. Nix package manager on this Ubuntu sandbox'` | OK - returns the section, exit 0 (exact-text heading match) |
| `rfs-dev sync-check` | OK - clean (no stale/missing/orphan pointers) |
| `rfs-dev inventory audit` | OK - EXACT: SRC 955 fns / 200 types, TEST 976 fns / 34 types, zero per-file diffs |

## Found problems/issues/mismatches
- The task file pinned the heading as exactly `10. Nix package manager on this Ubuntu sandbox` for machine verification; my first draft titled it "... (Phase 15)" following the file's mixed phase-suffix style, which broke the pinned doc-section lookup. Fixed by dropping the suffix from the heading and moving the phase attribution into the section's first sentence ("Added in Phase 15...") - no task correction needed, this was a draft-vs-criterion mismatch caught at verification.
- A `result` symlink (nix build artifact from the planning session's baseline build) was left untracked in the repo root and is NOT gitignored. Removed it for this commit; tasks 2-4 will recreate it on every `nix build` - if it becomes a recurring nuisance, adding `result` to `.gitignore` is a one-line plan correction for a later task.

## Other important findings/information
- TOOLCHAIN.md's section style mixes phase suffixes ("(Phase 5 task 17)", "(Phase 12)") with plain titles (§9 has none); §10 follows the plain-title form to keep the pinned verification command stable. The phase attribution lives in the body instead.
