# RFS Roadmap - docs/roadmap/ (split documents)

This directory is the split form of the former single `ROADMAP.md` (removed
2026-10-16, see `TASKS.md`). It describes development from Phase 0 parser/IR
baseline to a production-ready DSL and toolchain.

The maintained set splits into four documents plus one directory per phase:

| File | Content |
|---|---|
| `PRINCIPLES.md` | Project-wide principles and invariants: guiding principles (incl. backend parity), document-set rules, recommended v1 scope, suggested implementation stack |
| `FORMATTING.md` | Formatting rules: the templates every phase/task document must follow (§2-§6) |
| `PROTOCOL.md` | The two-stage work protocol: planning stage (§2), implementation stage (§3) |
| `CURRENT_STATE.md` | The phase-state table (which phases exist, their state, files present) |

How to build/test/run this project: `BUILD.md`. Rust toolchain locations and
activation: `TOOLCHAIN.md`. Session entry point for LLM-assisted work: `START.md`.

## 1. File set and naming

| File | Content | Lifecycle |
|---|---|---|
| `INTRO.md` | This file: overview + file set and naming + pointers to the split documents | Maintained; update when the layout or the split changes |
| `PRINCIPLES.md` | Project-wide principles and invariants (see table above) | Maintained; update when a principle/invariant changes |
| `FORMATTING.md` | Templates for all phase/task document types (§2 PHASE_N, §3 PLAN, §4 STATUS, §5 TASK_M, §6 TASK_M_STATUS) | Maintained; update when templates change |
| `PROTOCOL.md` | Two-stage work protocol: planning stage (§2), implementation stage (§3) | Maintained; update when the protocol changes |
| `CURRENT_STATE.md` | Phase-state table | Kept current: updated when a phase is planned, completed, or re-planned |
| `phase_N/PHASE_N.md` (N = 0..8) | Phase description: goal, scope, phase-specific sections, deliverables, exit criteria, non-goals | Written once per phase; frozen during planning/implementation - changes only via plan corrections recorded in `phase_N/PHASE_N_PLAN.md` |
| `phase_N/PHASE_N_PLAN.md` | Detailed implementation plan for the phase (planned-task table + corrections table) | Created in the planning stage and reviewed by the user; during implementation only its "Plan corrections" table is appended to |
| `phase_N/PHASE_N_STATUS.md` | Phase completion report: files added/changed/removed, functionality deltas, tests, findings | Written once when the phase exit criteria are reached (PROTOCOL.md §3 step r); not rewritten afterwards |
| `phase_N/tasks/PHASE_N_TASK_M.md` (M = task number from the plan, including correction numbers like `4a`) | One detailed file per top-level planned task (subtask rows live inside their parent's task file) | Written after user confirmation of the plan (PROTOCOL.md §2 step j), reviewed by the user (step k); frozen afterwards - only its "Task corrections" section is appended to |
| `phase_N/tasks/PHASE_N_TASK_M_STATUS.md` | Task completion report: files added/changed/removed, functionality deltas, tests, findings | Written once when the task exit criteria are reached (PROTOCOL.md §3 step m, FORMATTING.md §6.2) or when an unimplemented task is cancelled (§6.1); not rewritten afterwards |

Layout note (2026-11-01): each phase has its own directory `phase_N/`; task
files (`PHASE_N_TASK_M*`) live in `phase_N/tasks/`. Documents written before
this split may still reference the old flat layout and the old single-file
section numbering (old "template 2.4" = FORMATTING.md §5, old "INTRO.md §3.2"
= PROTOCOL.md §3); `TASKS.md` history entries keep their pre-split paths by
design (append-only record).
