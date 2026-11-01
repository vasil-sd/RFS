# RFS Roadmap - Work Protocol (Two Main Stages)

## 1. Two main stages

Phase work is split into two strictly separated stages, each triggered by its own
user request. Never merge the stages: do not implement before the plan is written
and user-reviewed, do not execute a task before its `PHASE_N_TASK_M.md` file is
written and user-confirmed, and do not start planning for a phase whose
`PHASE_N.md` does not exist.

File locations: `docs/roadmap/phase_N/` holds the phase description, plan and
status; `docs/roadmap/phase_N/tasks/` holds the task files. Templates:
`FORMATTING.md`. Binding principles: `PRINCIPLES.md`. Phase state:
`CURRENT_STATE.md`.

## 2. Planning stage (trigger: user says "let's make a plan for Phase N")

a) Thoroughly read `INTRO.md`, `PROTOCOL.md`, `FORMATTING.md` and
   `PRINCIPLES.md`.
b) Thoroughly read `docs/roadmap/phase_N/PHASE_N.md` for the phase about to
   be implemented.
c) Think deeply about the implementation; read all necessary files as needed.
d) Identify ALL affected files, including all documentation files.
e) Re-check assumptions about affected files; read them fully if needed.
f) Re-check goals and deliverables of the corresponding phase
   (`docs/roadmap/phase_N/PHASE_N.md`).
g) Write a detailed plan into `docs/roadmap/phase_N/PHASE_N_PLAN.md` following
   FORMATTING.md §3. The plan must be written for a future LLM session: after
   reading `START.md`, the `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` +
   `FORMATTING.md`) and this `PHASE_N_PLAN.md`, the assistant must be able to
   immediately start
   work on implementation. Where applicable, necessarily include examples update,
   tests update, and training-sets update as explicit tasks.
h) Review the new plan thoroughly: re-check all affected files, goals,
   deliverables, and the formatting rules in `FORMATTING.md`.
i) STOP here and ask the user to review the new plan. Do not implement anything.
j) After user confirmation of the plan, write one detailed
   `docs/roadmap/phase_N/tasks/PHASE_N_TASK_M.md` per top-level planned task
   (FORMATTING.md §5); subtask rows are covered inside their parent's task file.
k) STOP here and ask the user to review the task files. Do not implement
   anything.

## 3. Implementation stage (trigger: user says "let's implement Phase N")

a) Thoroughly read `START.md` and the `docs/roadmap/` set (`INTRO.md`,
   `PROTOCOL.md`, `FORMATTING.md`) if needed.
b) Thoroughly read `docs/roadmap/phase_N/PHASE_N.md` for the phase about to
   be implemented.
c) Check that `docs/roadmap/phase_N/PHASE_N_PLAN.md` exists; if not, warn the
   user and STOP.
d) Re-check `docs/roadmap/phase_N/PHASE_N_PLAN.md`: all affected files, goals,
   deliverables; re-read all needed files; take into account the "Plan
   corrections" table from previous attempts of planned-task execution, the
   "Task corrections" tables of the `phase_N/tasks/PHASE_N_TASK_M.md` files of
   already implemented tasks of the current phase, and the "Found
   problems/issues/mismatches" + "Other important findings/information"
   sections of their `PHASE_N_TASK_M_STATUS.md` files.
e) Re-check all `phase_N/tasks/PHASE_N_TASK_M.md` files of not-yet-implemented
   tasks of the current phase; make edits if needed, strictly following
   FORMATTING.md §5. Add new tasks (with corresponding task files and "Plan
   corrections" entries) if needed.
f) Execute the planned tasks in a loop, one by one:
g) Take the next not-yet-implemented task from its task file
   (`phase_N/tasks/PHASE_N_TASK_M.md`), taking task dependencies into account.
h) Print a message to the user with the task number, a brief description (what
   is planned to be done, list of actions, what will be achieved) and a brief
   scope (which files will be changed).
i) STOP here and ask the user to review the task file. This gate applies to
   EVERY task, including tasks added during execution (steps e, q).
j) Continue only after user confirmation (trigger: user says "let's execute the
   task").
k) Execute the actions from the task file.
l) Check the task exit criteria (task file). If not reached, correct the task
   ("Task corrections" table) and re-execute the actions (goto k), repeat until
   the exit criteria are reached.
m) After reaching the task exit criteria, write
   `phase_N/tasks/PHASE_N_TASK_M_STATUS.md` according to FORMATTING.md §6.2.
n) Reflect the task corrections introduced during work in the "Task
   corrections" table of the task file; strictly follow the formatting rules
   (FORMATTING.md §5).
o) Reflect the plan-level corrections introduced during work in the "Plan
   corrections" table of `phase_N/PHASE_N_PLAN.md`; NEVER touch the initial
   "Planned tasks" table; follow the formatting rules in FORMATTING.md. If an
   unimplemented task was cancelled, write its `PHASE_N_TASK_M_STATUS.md`
   according to FORMATTING.md §6.1.
p) Check for remaining not-cancelled tasks (consult
   `phase_N/PHASE_N_PLAN.md`); if any remain, repeat the loop (goto g).
q) If no tasks remain, check the phase exit criteria
   (`phase_N/PHASE_N.md`). If not reached, correct the plan ("Plan corrections"
   table), add new tasks (new task files) if needed, and re-execute the loop
   (goto d), repeat until the phase exit criteria are reached.
r) After reaching the phase exit criteria, write
   `docs/roadmap/phase_N/PHASE_N_STATUS.md` according to FORMATTING.md §4.
s) Update all related documentation files; re-check that docs are aligned with
   the current state of the project (`KB.md`, `DESIGN.md`, `BUILD.md`,
   `CODEGEN.md`, backend docs, and `CURRENT_STATE.md`).
t) Update `TASKS.md` (entry per its first-paragraph template) and commit.
