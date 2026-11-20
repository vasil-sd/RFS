# Phase 17, Task 10 - Implementation status

> Written once when all exit criteria of `PHASE_17_TASK_10.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added documentation files

| Name | Description |
|---|---|
| `docs/user/USAGE.md` | the exhaustive CLI usage reference (sections 1-11): intro + exit-code convention; command map; the RFS_* tool-resolution contract (nine-tool table matching TOOLCHAIN.md section 8.2 tool-for-tool, + CLI-override and used-by columns, special vars, clang/z3 notes, a real `--print-script` `# tools:` proof); one section per command set (parse / validate / info+info format / codegen rust\|c / test / diagram / mc-ref / gen-spec) each with a four-column pattern table + naming-and-placement rules + error-details-and-fixes; the codegen take-or-emit contract stated in BOTH the table rows and a dedicated naming-rules subsection (BASE derivation, taken-vs-emitted, partial-pair usage error, stale-reuse warning, `--out -` stdout mode); the mc-ref section with all three modes' flag tables + exit codes and field-by-field CE-trace walkthroughs using task 6's REAL captured traces (deadlock-config + property-violation trace, text and JSON); and the 15-row example-projects table linking every `crates/examples/*/README.md` |

## Changed documentation files

| Name | Operation | Description |
|---|---|---|
| `docs/user/README.md` | changed | USAGE.md row added to "Which doc for which question?" ("Run any CLI command with confidence: every pattern, option, generated artifact and error") + a pointer in the toolchain-at-a-glance section after the eight-command list |

## Changed functionality
None - documentation only (no source changes, per the non-goals).

## Removed functionality
None.

## New functionality
- A single authoritative CLI reference that complements MODEL_CHECKING.md (mode semantics + report reading), INTEGRATION.md (build.rs/adapters/Makefiles) and EXAMPLES.md (spec walkthroughs) without re-deriving their content - it links to them where semantics belong.

## Tests add/change/remove
None new. USAGE.md is machine-checked by the existing `tests/docs_md_test.rs` gate (it collects every repo `.md`; mermaid must parse, cross-links must resolve) - no test-code change was needed.

## Tests run status

| Name | Status |
|---|---|
| `cargo test --test docs_md_test` (5) | OK (all_repo_docs passes: USAGE.md mermaid renders + all 15 example links + TOOLCHAIN.md/LANGUAGE/PROPERTIES/MODEL_CHECKING/INTEGRATION/EXAMPLES links resolve) |
| `cargo test --test dev_docs_test` (9) | OK (sync gate unaffected - no new .rs files) |
| `cargo test` (root pkg, 584) | OK |
| `cargo clippy --all-targets` | 0 errors |
| `cargo fmt --check` | clean |

## Found problems/issues/mismatches
- Three TOOLCHAIN.md references were initially written as `../TOOLCHAIN.md`, which from `docs/user/` resolves to the non-existent `docs/TOOLCHAIN.md` - the repo root is two levels up, so the correct path is `../../TOOLCHAIN.md`. Caught by the dead-link gate before it could ship.

## Other important findings/information
- Every nuanced claim in the pattern tables was verified by RUNNING the command, not from memory: take-or-emit (an existing `--fsm-file` is reused and only the mc artifact is written), the C partial-pair usage error (exit 1, "a lone file is never overwritten"), the stale-reuse warning (a taken fsm from a different spec prints a stderr warning but is still used - exit 0, not an error), and the mc-ref exit-code taxonomy (pass=0, fail=1, state-limit=2). The section-2 `# tools:` proof lines are task 8.4's real captures.
