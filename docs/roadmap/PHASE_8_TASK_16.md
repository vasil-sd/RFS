# Phase 8 Task 16 - Full verification + phase close-out

Plan row 16 (single-line task, no subtasks; depends on 2-15). One task file per
plan group; task files are created at execution.

## Goal

Close out Phase 8: run the full verification suite on the final tree, write
`docs/roadmap/PHASE_8_STATUS.md`, update the `INTRO.md` phase-state row to
COMPLETE, append the `TASKS.md` entry.

## Context

- All 50 other plan rows (1.1-15) are DONE; this is the only PENDING row and
  the last task of Phase 8.
- Baseline to verify against (BUILD.md section 5, re-dated by task 14):
  535/535 root, 559 workspace, 537 smt, 1683/1683 generated (192 modules),
  examples 50 valid + 13 invalid, aggregate 267 lines / snippets 138+128.
- Task 14's scope note deferred one item to this close-out review: KB.md
  section 1 "Tests" row still carries Phase 6 counts (453/477/455).
- Phase 8 house pattern: task file created at execution with checked exit
  criteria + corrections table; no per-task `_STATUS.md` file.

## Planned actions

1. **Full verification** on the final tree: `cargo build`; `cargo test` (root);
   `--workspace`; `--features smt` (z3 env per TOOLCHAIN.md 5.2);
   `cargo test-generated-code`; `examples_test`; training pipeline (extract +
   build + test-training); `dev_docs_test` + `docs_md_test` explicit;
   `clippy --all-targets`; `fmt --check`.
2. **Close-out fixes** if verification surfaces phase-introduced issues.
3. **PHASE_8_STATUS.md** per template 2.3 (file census from the git diff over
   the Phase 8 range + measured counts).
4. **Docs sync**: INTRO.md row 8 → COMPLETE; KB.md Tests row re-measured (the
   task-14 deferral).
5. **Bookkeeping**: plan row 16 → DONE; this task file; TASKS.md entry;
   commits per action.

## Exit criteria (task 16)

- [x] Full verification green on the final tree: build OK; 535/535 root;
      559 workspace; 537 smt; 1683/1683 generated (192 modules); examples 3
      pass (50 valid + 13 invalid); training pipeline OK (aggregate 267 lines,
      snippets 138+128 unchanged, test-training 5 pass); dev_docs_test 9/9;
      docs_md_test 5/5; clippy --all-targets exit 0 with zero warnings in
      Phase 8's new files; fmt clean.
- [x] Phase-introduced clippy warnings fixed: 5 `doc_lazy_continuation` on the
      `//! Docs:` lines (codegen/frontend/json_rw/printer/smt_plan) - blank
      `//!` separator, doc-comment-only, re-verified by gate + full suite.
- [x] `docs/roadmap/PHASE_8_STATUS.md` written per template 2.3.
- [x] INTRO.md phase-state row 8 → COMPLETE (close-out 2026-11-01).
- [x] KB.md section 1 Tests row re-measured at close-out (453/477/455 →
      535/559/537, dev_docs_test named) - the item task 14 deferred.
- [x] Plan row 16 marked DONE; this task file written; TASKS.md entry
      appended.

## Non-goals (task 16)

- No new features, no algorithm/CLI/IR changes (documentation-only phase).
- No per-task `_STATUS.md` files (Phase 8 house pattern).
- The untracked `nix/` directory (unrelated hipfire/ROCm project) is left
  untouched.

## Task corrections

| Correction | Description + rationale |
|------------|-------------------------|
| Task file created at execution (2026-11-01) for plan row 16 | Single-line plan task: one section per the house pattern, like PHASE_8_TASK_14/15.md. |
| KB.md Tests-row update added to scope | The plan row lists only PHASE_8_STATUS.md + INTRO.md + TASKS.md, but task 14 explicitly deferred the stale KB.md Tests row (Phase 6 counts) to "task 16's close-out review" and protocol step s requires docs aligned with the current state at close-out. |
| Close-out clippy fix added to scope | Verification surfaced 5 phase-introduced `doc_lazy_continuation` warnings on the mandatory `//! Docs:` lines (each directly after an existing bullet list); fixing them (blank `//!` separator, doc-comment-only) keeps the close-out record "zero phase-introduced warnings" and stays within the phase's documentation-only scope. |
