# Phase 7, Task 13 - Implementation status

> Written once when all exit criteria of `PHASE_7_TASK_13.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

None (all changes to existing files).

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| `src/info_format.rs` | changed - `MMD_SUBSET_MD` (stateDiagram-v2 header; IFF-initial `[*] -->` line; transition label grammar `<EventLabel>` + optional ` \n <guard>` + one ` \n emits <X>` per emitted output, where `\n` is the literal two-character sequence; `[on_error]` first-label-line prefix on fallback edges with no guard part; verbatim names without escaping - safe by RFS identifier grammar) + `diagram_mmd_render` (md only) + FORMATS entry (9th, after diagram-puml) | |

## Added/changed/extended documentation files

| Name | Operation | Description |
|---|---|---|
| `BUILD.md` | changed - section 3 info-format row (34 -> 36 tests, tasks 5-12 -> 5-13, task 13 description appended); root re-pinned **524** (+2) / workspace **548**; section 5 baseline re-measured (524/548/526-smt) | |

## Other file changes

None.

## Changed functionality

- `rfs info format --list`: the registry now enumerates 9 formats (was 8) -
  the new `diagram-mmd` row. All other rows byte-identical (pin updated).

## Removed functionality

None.

## New functionality

- `rfs info format diagram-mmd --output md`: the generated Mermaid subset
  documented in markdown, derived from `diagrams/mermaid.rs` `generate` (the
  single source of truth). No JSON schema output (D-P7-9).

## Tests add/change/remove

- Added (2): `diagram_mmd_md_pins_sections`,
  `diagram_mmd_generated_output_matches_documented_subset` (coverage sync over
  all 50 valid examples x every module: every generated line must be one of
  the documented shapes filled with IR data - no undocumented tokens; corpus
  pins for guard / struct-event / on_error / emits coverage) + the
  `check_mmd_documented_subset` helper.
- Changed (1): `list_pins_all_registered_formats` - registry byte pin extended
  to 9 entries (+ `diagram-mmd` row).

## Tests run status

| Name | Status |
|---|---|
| `cargo test` (root) | OK - **524 pass / 0 fail** (was 522; +2 new tests) |
| `cargo test --workspace` | OK - **548 pass / 0 fail** (was 546) |
| `cargo test --features smt` | OK - **526 pass / 0 fail** (was 524; z3 env per TOOLCHAIN.md 5.2) |
| `tests/info_format_test.rs` | OK - 36 tests (+2) |
| `cargo test-generated-code` | OK - 1683 pass / 0 fail (unchanged; codegen untouched) |
| `cargo clippy --all-targets` | OK - zero warnings in touched files (pre-existing warnings only elsewhere) |
| `cargo fmt --check` | OK |
| `tests/docs_md_test.rs` | OK - 5 pass / 0 fail |

## Found problems/issues/mismatches

- Environment (not code): mmdc's puppeteer browser cache was missing in this
  session's sandbox HOME (`~/.cache/puppeteer`), so the availability-gated
  external half of `diagram_render_parity_test` failed at browser launch
  ("Could not find chrome-headless-shell (ver. 156.0.8078.47)"). Restored by
  copying the exact version from a previous session's cache
  (`/opt/unsloth-studio/sandbox/__LOCALID_FwM7zte/.cache/puppeteer`). After
  the restore all 13 parity tests pass. Future sessions in a fresh sandbox
  HOME may need the same restore (or `npx puppeteer browsers install
  chrome-headless-shell` with network).

## Other important findings/information

- No generator changes: the documented subset follows `diagrams/mermaid.rs`
  as-is (task non-goal honored). The coverage sync test is the guard that
  keeps the documentation and the generator in lockstep.
- Committed at Phase 7 closeout (task 16) per the plan's single-commit rule.
