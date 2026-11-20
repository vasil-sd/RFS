# Phase 17, Task 9 - Implementation status

> Written once when all exit criteria of `PHASE_17_TASK_9.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files
None (this task restructures existing READMEs + adds one test leg).

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| `crates/examples/c_sync_handler/Makefile` | changed (header comment only) | documents `make CC=clang` as a supported compiler choice (the generated C is compiler-agnostic - warning-free under both gcc and clang, per tests/c_compiler_agnostic_test.rs) + the RFS_GCC / RFS_CLANG equivalents for driving rfs itself |
| `tests/example_projects_test.rs` | changed | + `c_sync_handler_builds_under_clang_with_same_outcome` leg (availability-gated on clang + make): rebuilds under CC=clang, asserts warning-free + the SAME pinned scenario outcome as the gcc leg |

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `crates/examples/sync_handler/README.md` | changed | restructured to the template (five sections in order) + mermaid stateDiagram of spec 43 + Files layout table; CORRECTED the stale `#[path]`/`src/arq_generated.rs` claim - the real mechanism is build.rs stripping inner attributes + lib.rs `include!` from OUT_DIR |
| `crates/examples/async_handler/README.md` | changed | same template restructure + diagram; preserves the no-tokio / futures-confined facts |
| `crates/examples/c_sync_handler/README.md` | changed | same + the documented gcc/clang choice (make / make CC=clang) + RFS_GCC/RFS_CLANG equivalents in Build-run-test |
| `crates/examples/fuzz_rust/README.md` | changed | same template restructure + diagram noting the fuzz-target relationship; preserves the detached-workspace / emitted-skeleton / decode-plan facts |
| `crates/examples/fuzz_c/README.md` | changed | same template restructure + diagram; preserves the generation-vs-build clang split + bounded-smoke-run facts |
| `docs/llm/TEST_STRUCTURE.md` | changed | +1 row for the new clang test fn (same-commit rule); Example-projects-suite row + example_projects_test file row extended (22 -> 23 fns); 1403/1430 |

## Changed functionality
None - README restructure + one test leg + a Makefile header comment.

## Removed functionality
None. (One STALE fact was corrected, not removed: sync_handler's `#[path]`/
`arq_generated.rs` description did not match the source - the real mechanism is
the shared include!-from-OUT_DIR strip pattern used by every example crate.)

## New functionality
- All 15 example projects now read uniformly to the PHASE_17.md template (five
  sections in order, a mermaid stateDiagram of the automaton, a Files layout
  table).
- c_sync_handler's compiler choice is documented and TESTED: `make CC=clang`
  builds the same scenario warning-free under clang with the identical pinned
  outcome (the generated-C compiler-agnosticism guarantee, made concrete for
  this project).

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|
| `example_projects_test::c_sync_handler_builds_under_clang_with_same_outcome` | new | rebuild c_sync_handler from scratch under clang (make CC=clang); assert warning-free (rfs codegen warnings excluded via has_gcc_diagnostic) AND the SAME pinned scenario outcome as the gcc leg (19 ticks / [0..8] frames+acks / 5 dropped out-of-band acks / max window 4); availability-gated on clang + make |

## Tests run status

| Name | Status |
|---|---|
| `cargo test --test example_projects_test` (14 = 13 baseline + 1 new) | OK |
| `cargo test --test dev_docs_test` (9) | OK |
| `cargo test --test docs_md_test` (5) | OK (all five restructured READMEs' mermaid + links render/resolve) |
| `rfs-dev inventory audit` | EXACT: SRC 1013 fns / 213 types, TEST 1055 fns / 35 types - zero per-file diffs |

## Found problems/issues/mismatches
- sync_handler's README described the generated artifact as a `#[path]` module (`src/arq_generated.rs`, gitignored) - STALE. The source (lib.rs + build.rs) actually uses the shared strip pattern: build.rs strips inner attributes and writes a byte-identical body to OUT_DIR, lib.rs pulls it in via `include!`. No `arq_generated.rs` exists on disk. Corrected during the restructure.

## Other important findings/information
- `make CC=clang` on c_sync_handler prints two rfs CODEGEN diagnostics ("property 'terminates'/'bounded_until_done' is outside the monitor subset; skipped") - these are NOT compiler warnings. The existing has_gcc_diagnostic helper (which matches only handler.c: / arq_fsm.c: prefixes) correctly excludes them, so the clang leg's zero-warning assertion is sound.
- The clang-built binary's header line still says "plain gcc" (a hardcoded string in handler.c) - it is language-specific and not compared; only the data lines are pinned, and they are byte-identical under both compilers.
