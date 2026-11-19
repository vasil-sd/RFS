# Phase 17, Task 1 - Implementation status

> Written once when all exit criteria of `PHASE_17_TASK_1.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

None (documentation-only task).

## Changed/removed source files

None.

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `docs/user/MODEL_CHECKING.md` | changed | section 2 rewritten for mode dispatch: per-mode flag tables (exact / fuzz / smt-bounded) with defaults from the actual usage text, per-mode exit codes, `--nondet` documented under exact, not-applicable reduced to `--threads`/`--fast`, parity statement covering all three modes vs BOTH backends; section 1's Fuzz + SMT rows gain the mc-ref command forms and the exit-code paragraph is now per-mode (fuzz has no 2) |
| `docs/user/INTEGRATION.md` | changed | one-line Nix installation pointer (`nix build`, BUILD.md section 8, full tool closure) next to the from-source note in the rfs-build toolchain-resolution paragraph |

## Other file changes

None.

## Changed functionality

None - documentation only; no command behaviour changed.

## Removed functionality

None.

## New functionality

- User documentation now describes `rfs mc-ref` as a three-mode in-process MC
  (exact / fuzz / smt-bounded) with per-mode flags, defaults and exit codes,
  consistent with the Phase 16 CLI.
- Users are pointed at Nix as an installation path alongside from-source builds.

## Tests add/change/remove

None - no test code touched; existing gates re-run.

## Tests run status

| Name | Status |
|---|---|
| `cargo test --test docs_md_test` (5) | OK |
| `cargo fmt --check` | OK |

## Found problems/issues/mismatches
- Pre-existing inaccuracy fixed along the way: section 1's SMT row showed the
  RUNTIME invocation as `--mode smt-bounded --smt-bound N`, but `--smt-bound`
  is the CODEGEN-time flag - the runtime flag (both the generated binaries and
  mc-ref, per the Phase 16 CLI unification) is `--bound N`. Corrected in the
  table row.

## Other important findings/information
- The section-2 heading text was kept stable so INTEGRATION.md section 8's
  existing anchor link (`#2-rfs-mc-ref---in-process-reference-model-checker`)
  still resolves - confirmed by the green docs_md_test cross-link check.
