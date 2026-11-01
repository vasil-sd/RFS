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

## Task corrections (during execution)

| Date | What changed | Why |
|------|--------------|-----|
| 2026-10-28 | Scope expanded by one file: `src/main.rs` - the usage-text `info format` paragraph (written in task 5) was stale ("Currently: spec-rfs ... JSON-schema outputs land with the JSON format entries") although tasks 6-14 registered nine more formats; replaced with a one-line summary of the current 10-format registry | The training entries document `rfs info format` against the REAL CLI surface; shipping SFT data that teaches a stale usage line would be exactly the drift class D-P7-9 exists to catch. Help text only - no behavior change, no test pinned the line |
| 2026-10-28 | The three new entries quote verbatim tool output captured in this session (rfs 1.0.0, spec `examples/valid/04_tcp_handshake_like.rfs`): internal SVG sizes dot 3766 / puml 4391 / mmd 6366 bytes with every external tool unresolvable; external dot 3335 / puml 7077 / mmdc 189407 bytes; `mc-ref` on spec 04 exits 1 (the known pinned deadlock) and its JSON report validates against the emitted mc-report schema | KB.md protocol: entries must contain REAL tool output, no fiction - so the numbers in the data are this session's captures, not placeholders |
| 2026-10-28 | Entry 2 carries an honest note that the installed plantuml (apt 1.2020.02) predates PlantUML EBNF support and answers "No diagram found" (exit 100) even for a minimal `@startebnf` file, so real-tool acceptance of `spec-rfs --output plantuml` is pinned structurally by the test suite instead | The entry documents what the tool ACTUALLY does in this environment; task 16 should consider noting in TOOLCHAIN.md that real-tool acceptance requires a newer PlantUML release |
