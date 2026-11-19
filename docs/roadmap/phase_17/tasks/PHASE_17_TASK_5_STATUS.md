# Phase 17, Task 5 - Implementation status

> Written once when all exit criteria of `PHASE_17_TASK_5.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

None (documentation-only task).

## Changed/removed source files

None.

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `TOOLCHAIN.md` | changed | section 6.4 gains the "mmdc present is NOT mmdc working" gotcha (observed 2026-11-19): the npm package does not bundle its browser; chrome-headless-shell must be installed into a PERSISTENT PUPPETEER_CACHE_DIR (sandbox $HOME is ephemeral per session); Node 18 cannot run the latest npx-fetched puppeteer - install the version mermaid-cli bundles (25.16.0) via npm install --prefix + its installer; symlink ~/.cache/puppeteer to the persistent dir; the verified recipe is included as a copy-paste block; root still needs the --no-sandbox config from the adjacent bullet |

## Other file changes

None.

## Changed functionality

None - documentation only.

## Removed functionality

None.

## New functionality

- The mmdc browser-binary finding is recorded with a verified install recipe,
  so a future session does not rediscover it.

## Tests add/change/remove

None - existing gates re-run.

## Tests run status

| Name | Status |
|---|---|
| `cargo test --test docs_md_test` (5) | OK |
| `cargo fmt --check` | OK |
| live mmdc render (end-to-end proof of the recipe) | OK - fresh SVG produced |

## Found problems/issues/mismatches
- None recorded.
