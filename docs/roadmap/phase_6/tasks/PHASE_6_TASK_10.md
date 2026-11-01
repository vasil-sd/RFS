# Phase 6, Task 10 - Implementation details

> Written for a future LLM session: after reading `START.md`,
> `docs/roadmap/INTRO.md` and this file, the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal
Close out Phase 6: run the FULL verification matrix, check every exit criterion
of `PHASE_6.md` verbatim with evidence, write `PHASE_6_STATUS.md` (template
2.3), update the INTRO.md phase-state table, append the TASKS.md entry, and tag
the release `v1.0.0`.

## Context

- **PHASE_6.md exit criteria (checked verbatim at this step)**:
  1. v1.0 syntax is stable -> evidence: task 2 shipped (version-header format
     check, IR_VERSION 1 with {0,1} policy, crate 1.0.0, MIGRATION.md) + the git
     tag below.
  2. toolchain passes fuzzing and differential testing (including cross-backend
     Rust-vs-C comparisons) -> evidence: tasks 4/5 suites green in this run
     (ref_mc_diff, backend_parity incl. forced violations, prod_parity,
     monitor_ref, psl_translation, toolchain_fuzz + bounded libFuzzer smoke).
  3. model checker reports are trustworthy and clearly labeled -> evidence:
     task 4.1's no-false-PASS negative control + labeling guarantees documented
     (SEMANTICS.md section 9, docs/user/MODEL_CHECKING.md).
  4. production code generation is used in at least one real or realistic
     protocol project -> evidence: the Phase 5 example-crate matrix re-verified
     GREEN in this run (sync_handler + async_handler session tests + demo
     binaries exit 0; c_sync_handler rebuilt via Makefile, gcc -Wall clean,
     scenario parity pinned) driving the spec-43 sliding-window ARQ end to end
     via rfs-build + adapters. Record the satisfaction explicitly in the status
     file.
  5. documentation is sufficient for external users -> evidence: task 7's seven
     docs/user/ files + SEMANTICS.md present, cross-linked, examples verified
     runnable (task 7's record).
  6. benchmark suite runs in CI -> evidence: `cargo test --test
     benchmark_mc_test` green in this run (single command, JSON report +
     per-backend table) + BUILD.md "Benchmarks in CI" section with the Actions
     snippet.
- **Verification matrix** (run ALL; record exact counts):
  - `cargo build && cargo test && cargo clippy --all-targets && cargo fmt --check`
  - `cargo test --workspace` (all members incl. example crates)
  - `cargo test --features smt` (z3 env per TOOLCHAIN.md section 5.2)
  - `cargo generate-code-tests && cargo test-generated-code` (final count)
  - `cargo test --test examples_test` (49 valid / 13 invalid)
  - each new Phase 6 suite individually: ref_mc_diff_test, backend_parity_test,
    prod_parity_test, monitor_ref_test, psl_translation_test,
    toolchain_fuzz_test, benchmark_mc_test
  - gcc -Wall spot checks of the NEW C artifacts (47/48/49 production + mc)
  - example projects: `cargo run -p sync_handler` + `cargo run -p async_handler`
    (exit 0), c_sync_handler Makefile rebuild, fuzz_rust `cargo check` in the
    detached workspace, fuzz_c generation without clang
  - benchmark suite run: `target/mc_benchmarks.json` written + table printed
  - bounded fuzz smoke: C-side libFuzzer targets (`-max_total_time 10`); Rust-
    side if nightly available (task 5's recorded path)
- **Status file**: `docs/roadmap/PHASE_6_STATUS.md` per template 2.3 - every
  section filled: added source files (spec_hash/specgen/ref_mc/mc_ref_cmd/
  gen_spec_cmd + fuzz/ workspace), changed/removed, documentation (SEMANTICS,
  docs/user/*, MIGRATION, all synced docs), other file changes (new examples,
  training regeneration, .gitignore), changed functionality (generated header
  line 1, MC JSON report fields, validator version check, IR read policy),
  removed functionality (none expected - verify), new functionality (hierarchical
  list: mc-ref, gen-spec, versioning surface, ref_mc/specgen lib modules,
  differential suites, fuzz layer, benchmark metrics, user docs set), tests
  add/change/remove table, tests run status table (one row per suite with exact
  counts), found problems/issues/mismatches (from all task status files + this
  run), other important findings.
- **INTRO.md**: phase-state table row 6 -> COMPLETE (close-out date,
  `PHASE_6_STATUS.md` in files-present); keep the table's note style consistent
  with rows 4/5.
- **TASKS.md**: append ONE entry per its first-paragraph template (read ONLY the
  first paragraph for the template - never the whole history).
- **Git**: single close-out commit with a clean tree; tag `v1.0.0` on it
  (annotated: "RFS v1.0.0 - production hardening and stable syntax/IR/codegen
  contract"). Verify no untracked files remain (target/, fuzz/target are
  gitignored).

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 1.-9. | all previous Phase 6 tasks | close-out runs only after everything is shipped and docs-synced |

### Affected documentation files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `docs/roadmap/PHASE_6_STATUS.md` | new | phase completion report (template 2.3) |
| `docs/roadmap/INTRO.md` | change | phase-state table row 6 -> COMPLETE |
| `TASKS.md` | change | close-out entry (first-paragraph template) |

## Planned actions

### Action 1: full verification matrix
Run the complete matrix from Context; collect exact counts for every suite;
fix nothing yet - if anything fails, diagnose and record it (a failing exit
criterion means the phase is NOT done: add plan corrections + fix tasks and
re-run per the implementation protocol before writing the status file).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none - verification only) | - | counts collected for the status file |

### Action 2: exit-criteria evidence pass
Check the six PHASE_6.md exit criteria verbatim against action 1's results +
the task records; write the evidence lines into the status file draft.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `docs/roadmap/PHASE_6_STATUS.md` | new | exit-criteria evidence section content |

### Action 3: status file + INTRO + TASKS.md
Finish `PHASE_6_STATUS.md` (all template sections, exact counts); update the
INTRO.md row; append the TASKS.md entry.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `docs/roadmap/PHASE_6_STATUS.md` | new | the full report |
| `docs/roadmap/INTRO.md` | change | row 6 COMPLETE |
| `TASKS.md` | change | close-out entry |

### Action 4: commit + tag
Single close-out commit (clean tree verified) + annotated tag `v1.0.0`.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (git state) | change | commit + tag v1.0.0 |

## Task deliverables
- Full verification matrix results with exact counts.
- `docs/roadmap/PHASE_6_STATUS.md` (complete, template 2.3).
- INTRO.md phase-state row 6 COMPLETE; TASKS.md entry.
- Git tag `v1.0.0` on a clean close-out commit.

## Task exit criteria
- [x] Every suite in the verification matrix green with exact counts recorded (or failures resolved via plan corrections first).
- [x] All six PHASE_6.md exit criteria checked verbatim with evidence in the status file - including the explicit "realistic protocol project" satisfaction record.
- [x] `PHASE_6_STATUS.md` complete per template 2.3 (all sections, no placeholders).
- [x] INTRO.md row 6 COMPLETE; TASKS.md entry appended.
- [x] Tag `v1.0.0` exists on the close-out commit; working tree clean.

## Non-goals
- No post-v1 work (multi-party composition, timed automata, LSP, etc. remain non-goals per PHASE_6.md); no crates.io publishing (not a phase deliverable - note it as a future candidate in the status file's findings if relevant); no rewrites of completed task files/statuses.

## Other information
- If the verification matrix surfaces failures that need code fixes, follow the
  implementation protocol: plan corrections + new tasks first, then re-run this
  close-out - the status file is written ONLY when all exit criteria hold.
