# RFS Roadmap - Current State (Phase State)

Phase state (2026-11-05):

| Phase | State | Files present |
|---|---|---|
| 0 - Semantics, IR, parser baseline | COMPLETE | `PHASE_0.md`, `PHASE_0_STATUS.md` |
| 1 - Minimal end-to-end demonstrator | COMPLETE | `PHASE_1.md`, `PHASE_1_STATUS.md` |
| 2 - Core DSL and production code generation | COMPLETE | `PHASE_2.md`, `PHASE_2_STATUS.md` |
| 3 - PSL-like properties and monitors | COMPLETE | `PHASE_3.md`, `PHASE_3_PLAN.md`, `PHASE_3_TASK_{13,15-24}.md` (+ `_STATUS` files), `PHASE_3_STATUS.md` |
| 4 - Scalable model checking | COMPLETE (close-out 2026-10-20: full verification green, `PHASE_4_STATUS.md`) | `PHASE_4.md`, `PHASE_4_PLAN.md`, `PHASE_4_TASK_{1..14}.md` + `PHASE_4_TASK_10B.md` (+ `_STATUS` files), `PHASE_4_STATUS.md` |
| 5 - Full toolchain and production integration | COMPLETE (close-out 2026-10-23: full verification green, `PHASE_5_STATUS.md`) | `PHASE_5.md`, `PHASE_5_PLAN.md`, `PHASE_5_TASK_{1..17}.md` (+ `_STATUS` files), `PHASE_5_STATUS.md` |
| 6 - Production hardening and v1.0 | COMPLETE (close-out 2026-10-25: full verification green, tag v1.0.0, `PHASE_6_STATUS.md`) | `PHASE_6.md`, `PHASE_6_PLAN.md`, `PHASE_6_TASK_{1..10}.md` + `PHASE_6_TASK_2A.md` (+ `_STATUS` files), `PHASE_6_STATUS.md` |
| 7 - Documentation and self-describing formats | COMPLETE (close-out 2026-10-28: full verification green, `PHASE_7_STATUS.md`) | `PHASE_7.md`, `PHASE_7_PLAN.md`, `PHASE_7_TASK_{1..16}.md` (+ `_STATUS` files), `PHASE_7_STATUS.md` |
| 8 - Detailed developer documentation and code↔doc traceability | COMPLETE (close-out 2026-11-01: full verification green, `PHASE_8_STATUS.md`) | `PHASE_8.md`, `PHASE_8_PLAN.md`, `PHASE_8_TASK_{1..15}.md`, `PHASE_8_STATUS.md` |
| 9 - Documentation cleanup - structure, deduplication, navigability | COMPLETE (close-out 2026-11-02: full verification green, `PHASE_9_STATUS.md`) | `PHASE_9.md`, `PHASE_9_PLAN.md`, `PHASE_9_TASK_{1..8}.md` (+ `_STATUS` files), `PHASE_9_STATUS.md` |
| 10 - Source structure analysis and refactoring preparation | COMPLETE (close-out 2026-11-05: full verification green, `PHASE_10_STATUS.md`) | `PHASE_10.md`, `PHASE_10_PLAN.md`, `tasks/PHASE_10_TASK_{1..34}.md` (+ `_STATUS` files), `PHASE_10_STATUS.md` |
| 11 - Source refactoring per the Phase 10 proposals (no functionality change) | PLANNED (2026-11-05: description + 14-task plan written, awaiting user review) | `PHASE_11.md`, `PHASE_11_PLAN.md` |
| 12 - LLM assistant development tooling | DESCRIBED (2026-11-05: phase description written; implementation plan to be defined later) | `PHASE_12.md` |

Keep this table current: update it when a phase is planned, completed, or re-planned.
Note (2026-10-16): Phase 3 implementation began before the per-task protocol was
introduced - its already-executed work has no task/status files and is recorded
only in `PHASE_3_PLAN.md` corrections; `PHASE_N_TASK_M.md` files are created for
the remaining tasks.

Layout: each phase has its own directory `docs/roadmap/phase_N/` (description,
plan, status); task files (`PHASE_N_TASK_M*`) live in
`docs/roadmap/phase_N/tasks/`. See `INTRO.md` section 1 for the full file set.
