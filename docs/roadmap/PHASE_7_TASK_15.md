# Phase 7, Task 15 - Implementation details

> Written for a future LLM session: after reading `START.md`,
> `docs/roadmap/INTRO.md` and this file, the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal

Training-set update for the new Phase 7 CLI surface: category-local entries in
the two-message format per KB.md covering `rfs diagram --renderer
internal|external` usage (incl. the zero-external-deps rationale) and `rfs info
format` usage (`--list`, per-format `--output` choices, CI/CD use of JSON
schemas). No `.rfs` snippets added (D-P7-11/D-P7-12); regenerate snippets +
aggregate anyway per protocol; `cargo test-training` green.

## Context

- Phase 7 plan: row 15; decisions D-P7-11 (no language/algorithm changes - no
  new examples), D-P7-12 (training entries only for the new CLI surface).
- Training protocol per KB.md: category-local, append-only, two-message format
  with `< think>` numbered-reasoning block and verbatim REAL tool output; no
  entry documents fiction.
- The five categories live in `training/<cat>/rfs_training.jsonl`;
  snippets are auto-extracted from `examples/` (untouched this phase, so the
  snippet set is unchanged - regeneration is a consistency step only);
  `all_rfs_training.jsonl` is the regenerated aggregate.
- Baseline at planning time: aggregate 261 lines (snippets 138 valid +
  128 invalid).

## Scope

### In scope
- New entries in `training/frontend_cli/rfs_training.jsonl` (the category for
  CLI usage) - and any other category where an entry genuinely belongs.
- Snippet regeneration + aggregate regeneration.
- `cargo test-training` green.

### Out of scope
- No new `.rfs` examples (D-P7-11).
- No rewording of existing entries (append-only rule).
- Closeout docs (task 16).

## Planned actions

1. **Category-local entries** (`training/frontend_cli/rfs_training.jsonl`):
   `rfs diagram --renderer internal|external` usage incl. the zero-external-
   deps rationale (in-process default for all three formats; external tools now
   optional; availability-gated parity checks), and `rfs info format` usage
   (`--list`, per-format `--output` choices, CI/CD use of JSON schemas - e.g.
   validating IR v1 documents / MC reports against the emitted draft-07
   schemas). Two-message format per KB.md with REAL tool output captured in
   this session.
2. **Regenerate** snippets + aggregate per protocol (`cargo
   extract-training-snippets`, `cargo build-training`) even though no `.rfs`
   snippets are added (D-P7-11/D-P7-12).
3. **Verify**: `cargo test-training` green; counts recorded for task 16's
   BUILD.md re-pin.

#### Affected files

| File | Operation |
|---|---|
| `training/frontend_cli/rfs_training.jsonl` | change - new category-local entries (diagram --renderer + info format usage) |
| `all_rfs_training.jsonl` | change - regenerated aggregate |
| `training/*/snippets/*` | regenerated - consistency step only (no new `.rfs` snippets) |

## Task deliverables

- SFT coverage of the Phase 7 CLI surface in the established category-local
  format.
- Regenerated snippets + aggregate; `cargo test-training` green.

## Task exit criteria

- [ ] Category-local entries added for `rfs diagram --renderer internal|external` (incl. zero-external-deps rationale) and `rfs info format` (`--list`, per-format `--output`, CI/CD JSON-schema use)
- [ ] No `.rfs` snippets added (D-P7-11/D-P7-12); snippets + aggregate regenerated anyway per protocol
- [ ] `cargo test-training` green; counts recorded for task 16's BUILD.md re-pin

## Non-goals

- No new examples, no language/algorithm changes.
- No rewording of existing entries (append-only).
