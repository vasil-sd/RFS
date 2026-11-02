# Phase 9, Task 1 - Implementation status

> Written once when all exit criteria of `PHASE_9_TASK_1.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

(none - docs-only task)

| Name | Description |
|---|---|

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `README.md` | removed | the outdated copy of docs/user/README.md (repo-root-relative links, missing Phase 7 updates and the dev/llm rows); nothing linked to it |
| `README.md` | new | repository entry point: short intro + the three main principles (one IR for all targets, backend parity, MC cross-checking), "The RFS pipeline" section with a mermaid flowchart (modeled on the gate-passing docs/user/README.md diagram), "For users" -> docs/user/README.md, "For developers" -> docs/dev/README.md + docs/llm/ and START.md pointer. 76 lines |
| `docs/roadmap/phase_9/PHASE_9_PLAN.md` | changed | bare illustrative markdown links in the task-table rows (tasks 1, 6, 7 descriptions) wrapped in code spans so the repo-wide docs_md_test gate does not parse them as real relative links from docs/roadmap/phase_9/ |
| `docs/roadmap/phase_9/tasks/PHASE_9_TASK_{2,3,4,6,7}.md` | changed | same code-span wrapping for their bare illustrative link examples (TASK_2 x2, TASK_3 x1, TASK_4 x4, TASK_6 x2, TASK_7 x4) |

## Other file changes

| Name | Operation | Description |
|---|---|---|

## Changed functionality

- None (no toolchain behavior changed).

## Removed functionality

- The outdated root README content (stale duplicate of the user docs with
  repo-root-relative links) - replaced, not merely deleted.

## New functionality

- Root `README.md` as the repository orientation page: principles summary,
  pipeline overview diagram, and routing to the user / developer / LLM-assistant
  documentation sets.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|

## Tests run status

| Name | Status |
|---|---|
| `cargo test --test docs_md_test` (5) | OK (first run FAILED - 6 phase_9 planning docs with bare illustrative links; after code-span wrapping: 5 passed, 0 failed) |

## Found problems/issues/mismatches
- The repo-wide doc-quality gate (`tests/docs_md_test.rs`) covers EVERY `.md`
  file in the repository, including `docs/roadmap/phase_9/` planning and task
  documents. Bare illustrative markdown link syntax in those documents (e.g.
  `[DESIGN.md](../DESIGN.md)` shown as an example of the conversion style) is
  parsed by comrak as a real relative link and must resolve from the document's
  own directory - it does not, so the gate failed on 6 of 255 docs. Fix:
  wrap every illustrative link in a code span (backticks), which makes it inert
  to link parsing while keeping the example readable. Multi-line code spans
  (TASK_7 line 39) are handled correctly by comrak and need no change.
- The old root README was not byte-identical to docs/user/README.md but an
  outdated copy - confirmed during execution, matching the planning finding.

## Other important findings/information
- Dangling-reference sweep (grep for root-README references outside TASKS.md):
  only two hits, both allowed exceptions - `docs/user/LANGUAGE.md:4` links its
  own directory's README; `BUILD.md:19` mentions `ide/README.md` in backticks.
- The mermaid pipeline diagram in the new README is the exact shape of the one
  in docs/user/README.md (verified rendering through mermaid-rs-renderer by the
  gate) - if it ever needs to diverge, re-run docs_md_test after the change.
