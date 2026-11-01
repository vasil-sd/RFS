# Phase 6, Task 9 - Implementation details

> Written for a future LLM session: after reading `START.md`,
> `docs/roadmap/INTRO.md` and this file, the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal
Sync ALL design-relevant documentation to the shipped Phase 6 state: the D-P6
decisions in DESIGN.md (new section 24), KB.md state/limitations/roadmap,
BUILD.md suites + commands + baseline, CODEGEN.md + backend docs (header format,
report fields), and START.md (8-command CLI snapshot + doc map). This is the
established pre-close-out docs-sync task (Phase 5 task 15 precedent).

## Context

- **What changed in this phase** (the sync checklist):
  - Two new CLI commands: `rfs mc-ref`, `rfs gen-spec` (task 3) - CLI is now
    EIGHT commands.
  - Versioning surface (task 2): version-header format check, `IR_VERSION = 1`
    with {0,1} read policy, spec hash + toolchain version in every generated
    artifact header (new line-1 format), crate 1.0.0, `docs/user/MIGRATION.md`.
  - New lib modules: `src/spec_hash.rs`, `src/specgen.rs`, `src/ref_mc.rs`,
    `src/mc_ref_cmd.rs`, `src/gen_spec_cmd.rs` (source map updates).
  - MC JSON report extended: `statistics.peak_rss_kb` + counterexample
    `ce_steps` (task 6) - CODEGEN.md + backend docs + the D-P5-10 description in
    DESIGN.md section 23 may need a forward-reference note.
  - New test suites (tasks 4/5/6): `ref_mc_diff_test`, extended
    `backend_parity_test` (forced violations), `prod_parity_test`,
    `monitor_ref_test`, `psl_translation_test`, `toolchain_fuzz_test`, extended
    `benchmark_mc_test` - BUILD.md test table rows with final counts.
  - Root `fuzz/` cargo-fuzz workspace (task 5) - BUILD.md section added in task
    5.3; verify + cross-link here.
  - New docs: `docs/SEMANTICS.md` (task 1), `docs/user/*` set (tasks 2.4/7).
  - New examples: 46 (version header) + 47/48/49 (benchmarks); invalid example
    `invalid_bad_version_format.rfs` - final counts 49 valid / 13 invalid.
  - Training: new counts from task 8's "Other information".
- **Stale sweep**: after the edits, grep the docs for stale facts and fix:
  `0.1.0` (crate version references), `376/376`, `45 valid`, `12 invalid`,
  `244` (training lines), `1501` (generated tests - re-measure), `Phase 1
  codegen` / `Phase 3 codegen (C backend)` header strings, "six-command CLI" /
  "6 subcommands", and any Phase 5-era baseline numbers. Re-measure the final
  test counts by actually running the suites (do not trust stale numbers).
- **DESIGN.md section 24 format**: mirror section 23's style - one `### D-P6-N.
  <title> (task M)` subsection per decision with context + choice + rationale;
  decisions D-P6-1..11 as defined in `PHASE_6_PLAN.md` Context (the plan is the
  source of truth for their wording - adapt to what actually shipped, noting any
drift).

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 1.-8. | all previous Phase 6 tasks | this task documents the SHIPPED state - everything must be complete and green first |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 10. | Close-out | re-verifies docs alignment (implementation stage step s) and writes PHASE_6_STATUS.md |

### Affected documentation files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `DESIGN.md` | change | NEW section 24 "Phase 6 production-hardening decisions (as shipped)" D-P6-1..11; cross-reference pointers verified from task 1 |
| `KB.md` | change | section 1 state (Phase 6 complete, final counts); section 2 pointer to D-P6 decisions; section 4.2 source map (+5 new modules); section 7 limitations table (mark what Phase 6 resolved - e.g. generated-code provenance now verifiable via spec hash; keep genuinely open items); section 8 roadmap (Phase 6 complete + post-v1 candidates); section 12 next work |
| `BUILD.md` | change | test table rows for all new suites with final counts; command table rows for `mc-ref`/`gen-spec`; verify task 5.3/6.3 sections present and consistent; final known-good baseline (re-measured) |
| `CODEGEN.md` | change | artifact header line-1 contract (toolchain version + spec hash), report field additions (`peak_rss_kb`, `ce_steps`), where the hash is computed |
| `src/backends/RUST.md` | change | header line, VmHWM reporting in the MC binary |
| `src/backends/C.md` | change | header line (unified with Rust), getrusage reporting |
| `START.md` | change | section 1 state snapshot (Phase 6 complete pending close-out; 8-command CLI; final baseline counts); section 2 doc map (+ `docs/SEMANTICS.md`, + `docs/user/` row); section 3 source tree (+ new modules, + `fuzz/`) |

## Planned actions

### Action 1: DESIGN.md section 24
Write the D-P6-1..11 decision entries (format per Context), adapting wording to
what shipped; verify task 1's cross-reference pointers are in place.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `DESIGN.md` | change | section 24 + pointer verification |

### Action 2: KB.md sync
Update sections 1/2/4.2/7/8/12 per Context; re-measure the numbers you write.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `KB.md` | change | the six section updates |

### Action 3: BUILD.md sync
Test table rows (new suites, final counts), command table rows (`mc-ref`,
`gen-spec` with full flag summaries), verify the fuzzing + benchmark-CI sections
from tasks 5.3/6.3 are consistent, rewrite the known-good baseline block with
re-measured numbers.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `BUILD.md` | change | table rows + baseline block |

### Action 4: CODEGEN.md + backend docs
Header contract, report fields, hash computation location; RUST.md/C.md header +
RSS reporting notes.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `CODEGEN.md` | change | header + report field sections |
| `src/backends/RUST.md` | change | header + VmHWM note |
| `src/backends/C.md` | change | header + getrusage note |

### Action 5: START.md sync + stale sweep
State snapshot (8-command CLI, final counts), doc map rows, source tree; then
the full stale-sweep grep list from Context across ALL docs (KB/DESIGN/BUILD/
CODEGEN/backend docs/START/docs/user) and fix every hit.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `START.md` | change | snapshot + doc map + source tree |
| all phase-touched docs | change | stale-sweep fixes (record the grep list + hits in "Other information") |

## Task deliverables
- DESIGN.md section 24 with D-P6-1..11 as shipped.
- KB.md / BUILD.md / CODEGEN.md / RUST.md / C.md / START.md fully synced to the
  final Phase 6 state with re-measured counts.
- Stale sweep completed (grep list + results recorded).

## Task exit criteria
- [x] DESIGN.md section 24 present with all 11 decisions, matching what shipped (drift noted). DONE 2026-10-25: D-P6-1..D-P6-11 written as shipped + an "execution notes" subsection recording plan corrections 2a/4b/5a/5b/6a/6b/6c, the final example counts (50/13), and the 2026-10-25 rule change; task-1 cross-reference pointers verified in place (DESIGN sections 2/3/4 -> SEMANTICS).
- [x] Every doc's numbers are RE-MEASURED (test counts, example counts, training counts) - no stale Phase 5 baseline survives the grep sweep. DONE 2026-10-25: all counts re-measured live this task (workspace/smt/generated/training/examples runs); two-pass grep sweep over START/BUILD/KB/CODEGEN/TOOLCHAIN/backend docs/docs/user clean - see Other information for the pattern list + kept-as-historical hits.
- [x] BUILD.md command table documents both new commands; test table has a row per new suite with final counts. DONE 2026-10-25: `mc-ref` + `gen-spec` rows added to section 4; nine new suite rows added to section 3 (ref_mc_sweep/ref_mc_diag/ref_mc_diff/prod_parity/monitor_ref/psl_translation/toolchain_fuzz/wide_ops_parity/docs_md) with measured counts; root row 453, workspace 477, generated 1683/192, baseline block rewritten.
- [x] START.md snapshot says 8-command CLI + Phase 6 state; doc map covers SEMANTICS.md + docs/user/. DONE 2026-10-25: section 1 rewritten (Phase 6 summary + re-measured baseline), section 2 gained the `docs/user/` row (SEMANTICS.md row already present from task 1), section 3 source tree updated (8 subcommands, +5 new src modules, fuzz/, Phase 6 test suites, 50/13 examples, 261 aggregate).
- [x] `cargo build && cargo test && cargo clippy --all-targets && cargo fmt --check` green (docs-only changes - sanity run). DONE 2026-10-25: build exit 0, `cargo test` 453/453 exit 0, clippy exit 0 (no errors; the 36 pre-existing warnings noted in KB/BUILD), fmt clean.

## Non-goals
- No `INTRO.md` phase-state table update (that happens at close-out, task 10); no `TASKS.md` entry (task 10); no `PHASE_6_STATUS.md` (task 10); no source code changes (if the sweep reveals a doc/code mismatch, fix the DOC to match shipped code - or record a plan correction if the code is wrong).

## Other information
- **Stale-sweep grep list** (run twice over START.md, BUILD.md, KB.md, DESIGN.md, CODEGEN.md, TOOLCHAIN.md, src/backends/RUST.md, src/backends/C.md, docs/user/; roadmap PHASE_* files are frozen historical records and were NOT rewritten):
  - FIXED: `v0.1.0` crate ref (BUILD.md section 1 row), `244 examples` / `135/127` / `1601` training+generated counts (BUILD.md sections 4 + alias note), `1501 generated unit tests` (KB.md section 5 pointer, START.md section 7 code block), root/workspace test counts 376/400 -> 453/477 (BUILD.md section 3 rows), "45-example" sweep descriptions -> 50 (BUILD.md root row x2, RUST.md no_std sweep note), `189 modules` -> 192 (BUILD.md alias row).
  - KEPT AS CORRECT HISTORY (verified, not stale): DESIGN.md D-P5-2 "all 45 valid examples" (true at that task's time), MIGRATION.md "0.1.0 -> 1.0.0" + the old header strings it documents as REPLACED, DESIGN.md section titles "Phase 1 codegen ..." (stable names).
- **Final re-measured baseline (2026-10-25 - copy into task 10's status file):** `cargo test` 453/453; `--workspace` 477/477; `--features smt` 455/455 (z3 env per TOOLCHAIN.md 5.2); `test-generated-code` 1683/1683 (192 modules); examples 50 valid / 13 invalid; training aggregate 261 lines, snippets 138 valid + 128 invalid; clippy exit 0 with 36 pre-existing warnings (all in Phase 6 tasks 4-6 files: tests/common/prod_driver.rs x14 cfg-gated dead-code helpers, src/backends/c_fsm.rs x9 monitor rendering + from_str naming, plus single hits in tests/common/mc_json.rs, src/validator.rs, src/test_cmd.rs, src/codegen_test.rs, src/codegen.rs); fmt clean. Per-suite root counts: lib 238, frontend 101, codegen 50, backend_parity 11, liveness_mc 9, monitor_integration 4, training 4, examples 3, example_projects 3, library_comparison 4, benchmark_mc 5, integration_example 2, ref_mc_diff 4, ref_mc_sweep 1, ref_mc_diag 1, prod_parity 1, monitor_ref 2, psl_translation 1, toolchain_fuzz 2, wide_ops_parity 2, docs_md 5 (smt_bounded 0 without the feature, +2 with).
- **DESIGN.md section 24 drift notes**: none material - all eleven decisions shipped as planned; the plan-corrections table entries are summarized in the section's "execution notes" subsection rather than per-decision rewrites.
