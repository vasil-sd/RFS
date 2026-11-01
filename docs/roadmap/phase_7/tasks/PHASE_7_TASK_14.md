# Phase 7, Task 14 - Implementation details

> Written for a future LLM session: after reading `START.md`,
> `docs/roadmap/INTRO.md` and this file, the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal

The `artifact-header` format entry (md only): line 1 of every generated
artifact - the exact stamp format (toolchain version + spec hash) derived from
`src/codegen.rs` at implementation time, with the precise syntax recorded in
the entry and a sync test comparing the documented format against actually
generated artifacts for all 50 examples.

## Context

- Phase 7 plan: row 14; decisions D-P7-9 (sync guarantees).
- The line-1 artifact header was introduced in Phase 6 (D-P6-3 / D-P5-2 stance:
  hand-rolled SHA-256 spec hash, same hash on both backends); the stamp is
  emitted by the codegen writers in `src/codegen.rs`.
- The entry has outputs: md only (it describes a line format, not a JSON
document).
- Current state at planning time: `src/info_format.rs` `FORMATS` holds 8 entries;
  after task 13 it will hold 9; this task adds the 10th and final one.

## Scope

### In scope
- Artifact-header documentation + entry registration in `src/info_format.rs`
  (outputs: md).
- Sync test: documented format vs actually generated artifacts for all 50
  examples.
- BUILD.md row update.

### Out of scope
- Training-set update (task 15), closeout (task 16).

## Planned actions

1. **Header documentation + entry** (`src/info_format.rs`): derive the exact
   stamp syntax from `src/codegen.rs` at implementation time (toolchain version
   + spec hash); record the precise syntax in the entry (outputs: md).
2. **Sync test** (`tests/info_format_test.rs`): compare the documented format
   against actually generated artifacts for all 50 examples (every artifact
   kind that carries the header).
3. **Docs** (`BUILD.md`): section 3 row update; baseline re-pin.

#### Affected files

| File | Operation |
|---|---|
| `src/info_format.rs` | change - artifact-header documentation + render fn + entry registration (outputs: md) |
| `tests/info_format_test.rs` | change - sync test(s): documented format vs generated artifacts for all 50 examples |
| `BUILD.md` | change - section 3 artifact-header row; baseline re-pin |

## Task deliverables

- `rfs info format artifact-header --output md` works.
- The documented stamp syntax matches every generated artifact line 1 (pinned
  by test over the whole corpus).
- Full test matrix green.

## Task exit criteria

- [ ] Exact stamp format derived from `src/codegen.rs` and recorded in the entry (toolchain version + spec hash; precise syntax)
- [ ] Entry registered (outputs: md)
- [ ] Sync test: documented format compared against actually generated artifacts for all 50 examples
- [ ] Full test matrix green; BUILD.md baseline re-pinned

## Non-goals

- No changes to the emitted header itself.
- No JSON schema output (line format, not a JSON document).
