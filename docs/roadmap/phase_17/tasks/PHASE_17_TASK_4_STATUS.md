# Phase 17, Task 4 - Implementation status

> Written once when all exit criteria of `PHASE_17_TASK_4.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

None.

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| `src/cli/args.rs` | changed | the usage() string literal only: the mc-ref signature line is now the three-mode form (`--mode exact\|fuzz\|smt-bounded` + per-mode flag groups, matching `src/cli/commands/mc_ref.rs`'s own usage text re-flowed to the global style); the mc-ref help paragraph describes all three modes with per-mode exit codes and reduces not-applicable to `--threads`/`--fast`. Every other command's line and paragraph is byte-identical |

## Added/changed/extended documentation files

None (the usage text IS the documentation change).

## Other file changes

None.

## Changed functionality

- Printed usage text only - no parsing, dispatch or exit-code behaviour changed
  (verified: 573/0 tests, identical to baseline).

## Removed functionality

None.

## New functionality

- The global usage (no-argument `rfs`, unknown subcommand) now matches the
  command's own three-mode usage text - no more contradiction between the two
  surfaces.

## Tests add/change/remove

None - no test pins this string (verified in preparation); the full suite was
re-run as the behaviour-preservation proof.

## Tests run status

| Name | Status |
|---|---|
| `cargo test` (573) | OK - exactly the Phase 16 baseline |
| `cargo clippy --all-targets` | OK - no warnings in args.rs (56 pre-existing baseline lines elsewhere) |
| `cargo fmt --check` | OK |

## Found problems/issues/mismatches
- None recorded.

## Other important findings/information
- Spot-checks performed: no-argument `rfs` prints the new three-mode line;
  unknown subcommand exits 1; `rfs mc-ref` with no spec prints its OWN usage,
  unchanged.
