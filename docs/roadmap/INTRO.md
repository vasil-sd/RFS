# RFS Roadmap - docs/roadmap/ (split documents)

This directory is the split form of the former single `ROADMAP.md` (removed
2026-10-16, see `TASKS.md`). It describes development from Phase 0 parser/IR
baseline to a production-ready DSL and toolchain.

The guiding principles are:

> One constrained specification → one versioned IR → multiple targets.

> **Backend parity:** every codegen backend provides the same functionality -
> the same commands, the same deliverables per command, and exactly the same
> observable behavior for the same spec and inputs. Backends must always be kept
> in sync: a feature lands in ALL existing backends within the same phase, and
> cross-backend differential tests verify that backends behave identically.
> Rationale: testing backends against each other gives maximal confidence in
> their correctness.

Targets include production automata, model-checking executables, tests, diagrams,
and later IDE support.

How to build/test/run this project: `BUILD.md`. Rust toolchain locations and
activation: `TOOLCHAIN.md`. Session entry point for LLM-assisted work: `START.md`.

## 1. File set and naming

| File | Content | Lifecycle |
|---|---|---|
| `INTRO.md` | This file: overview, phase-state table, formatting rules (templates), two-stage work protocol | Maintained; update when the protocol or templates change |
| `PHASE_N.md` (N = 0..6) | Phase description: goal, scope, phase-specific sections, deliverables, exit criteria, non-goals | Written once per phase; frozen during planning/implementation - changes only via plan corrections recorded in `PHASE_N_PLAN.md` |
| `PHASE_N_PLAN.md` | Detailed implementation plan for the phase (planned-task table + corrections table) | Created in the planning stage and reviewed by the user; during implementation only its "Plan corrections" table is appended to |
| `PHASE_N_STATUS.md` | Phase completion report: files added/changed/removed, functionality deltas, tests, findings | Written once when the phase exit criteria are reached (implementation stage step r); not rewritten afterwards |
| `PHASE_N_TASK_M.md` (M = task number from `PHASE_N_PLAN.md`, including correction numbers like `4a`) | One detailed file per top-level planned task (subtask rows live inside their parent's task file) | Written after user confirmation of `PHASE_N_PLAN.md` (planning step j), reviewed by the user (step k); frozen afterwards - only its "Task corrections" section is appended to |
| `PHASE_N_TASK_M_STATUS.md` | Task completion report: files added/changed/removed, functionality deltas, tests, findings | Written once when the task exit criteria are reached (implementation step m, template 2.5.2) or when an unimplemented task is cancelled (template 2.5.1); not rewritten afterwards |

Phase state (2026-10-21):

| Phase | State | Files present |
|---|---|---|
| 0 - Semantics, IR, parser baseline | COMPLETE | `PHASE_0.md`, `PHASE_0_STATUS.md` |
| 1 - Minimal end-to-end demonstrator | COMPLETE | `PHASE_1.md`, `PHASE_1_STATUS.md` |
| 2 - Core DSL and production code generation | COMPLETE | `PHASE_2.md`, `PHASE_2_STATUS.md` |
| 3 - PSL-like properties and monitors | COMPLETE | `PHASE_3.md`, `PHASE_3_PLAN.md`, `PHASE_3_TASK_{13,15-24}.md` (+ `_STATUS` files), `PHASE_3_STATUS.md` |
| 4 - Scalable model checking | COMPLETE (close-out 2026-10-20: full verification green, `PHASE_4_STATUS.md`) | `PHASE_4.md`, `PHASE_4_PLAN.md`, `PHASE_4_TASK_{1..14}.md` + `PHASE_4_TASK_10B.md` (+ `_STATUS` files), `PHASE_4_STATUS.md` |
| 5 - Full toolchain and production integration | COMPLETE (close-out 2026-10-23: full verification green, `PHASE_5_STATUS.md`) | `PHASE_5.md`, `PHASE_5_PLAN.md`, `PHASE_5_TASK_{1..17}.md` (+ `_STATUS` files), `PHASE_5_STATUS.md` |
| 6 - Production hardening and v1.0 | not started | `PHASE_6.md` |

Keep this table current: update it when a phase is planned, completed, or re-planned.
Note (2026-10-16): Phase 3 implementation began before the per-task protocol was
introduced - its already-executed work has no task/status files and is recorded
only in `PHASE_3_PLAN.md` corrections; `PHASE_N_TASK_M.md` files are created for
the remaining tasks.


## 2. Formatting rules (templates)

All phase documents MUST follow the templates below exactly: same section names,
same order, same table columns. Sections marked "(optional)" may be omitted when
empty; all others are mandatory. Descriptions in tables must include explanation
and rationale in a short form (what + why, 1-2 lines).

### 2.1 Template: PHASE_N.md

```markdown
# Phase N: <short name>

## Goal
1-3 sentences: what the phase achieves and why it matters. A code block is allowed
for pipeline statements (as in Phase 1).

## Scope
Bulleted list of everything in scope for this phase (language features, tools,
backends, docs). Keep items short; details go to the phase-specific sections below.

## <Phase-specific sections>
Zero or more sections specific to this phase (e.g. "v0 language subset",
"Property subset", "Monitor code generation"), placed between Scope and
Deliverables. Use `###` subheadings inside them when needed.

## Deliverables
Bulleted list of concrete artifacts the phase produces (files, commands, features,
docs). Paths relative to the repo root.

## Exit criteria
Checklist. The phase is complete only when ALL items are satisfied; the
implementation stage (step r) checks this list verbatim.

## Non-goals
Bulleted list of explicitly excluded scope: what this phase deliberately does NOT
do, and which later phase covers it, if known.
```

### 2.2 Template: PHASE_N_PLAN.md

```markdown
# Phase N - Implementation plan

> Written for a future LLM session: after reading `START.md`,
> `docs/roadmap/INTRO.md` and this file, the assistant must be able to start
> implementation immediately without re-deriving scope.

## Context

List of important notes, issues and findings discovered during the planning
process.
All important information for a future LLM session, that may be needed for
an implementation process, should be placed here.

## Planned tasks

| # | Task | Affected files (operation) | Description + rationale |
|---|------|----------------------------|--------------------------|
| 1. | <task name> | `path` (new/change/remove); ... | what will be done, why, how - short form |
| 1.1 | <subtask> | ... | ... |

Rules:
- Task numbers are hierarchical: `1.`, `1.1`, `1.1.1` for subtasks; tasks are
  ordered by execution order.
- Affected files: EVERY file the task touches, each with operation type
  (`new`, `change`, `remove`) - including documentation files, valid/invalid
  examples, tests, and training-set files where applicable.
- Description states what will be done, why (rationale), and how (explanation),
  in short form. Where applicable the plan must include examples update, tests
  update, and training-sets update as explicit tasks.

## Plan corrections (during execution)

| # | Task | Correction type | Description + rationale |
|---|------|-----------------|--------------------------|
| 1.2 | <task name> | new/change/remove | what changed in the plan during execution and why - short form |

Rules:
- The "Planned tasks" table above is NEVER modified after the user approves the
  plan. All deviations discovered during implementation are appended to THIS
  table only, with correction type (`new` = task added, `change` = task modified,
  `remove` = task dropped) and rationale. New task numbers must not collide with
  the numbering of the "Planned tasks" table (e.g. `4a`, `4b`).
- Every correction is reflected in the corresponding `PHASE_N_TASK_M.md` files:
  `new` adds a new task file, `change` edits the task file, `remove` keeps the
  task file but gets a `PHASE_N_TASK_M_STATUS.md` with detailed description and
  rationale of the cancellation (template 2.5.1).
- Task files of already implemented tasks (those that have a
  `PHASE_N_TASK_M_STATUS.md`) are NOT changed during plan execution. If
  previously implemented work must be undone, add a NEW task to this table with
  a detailed description of what it undoes and why.
- Rationale: these rules keep the planning process fully analyzable (what was
  planned initially, what was implemented, what was corrected, what was removed
  before implementation, what was undone after implementation).

## Other information
(optional.) Free-form notes for the implementing session: open questions,
assumptions to re-check, commands to run, risks.
```

### 2.3 Template: PHASE_N_STATUS.md

```markdown
# Phase N - Implementation status

> Written once when all exit criteria of `PHASE_N.md` are reached. Not rewritten
> afterwards; later fixes go into `TASKS.md` entries and follow-up phase plans.

## Added source files

| Name | Description |
|---|---|
| `src/...` | what it is + why it was added - short form |

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| `src/...` | changed / removed | what changed, or why removed - short form |

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `*.md` | new / changed / removed | what the doc change is and why - short form |

## Other file changes

| Name | Operation | Description |
|---|---|---|
| `examples/...`, `training/...`, build files, ... | new / changed / removed | examples, training sets, config, etc. - short form |

## Changed functionality
Hierarchical bullet list (nested bullets for sub-features) of behavior that
existed before the phase and was modified.

## Removed functionality
Hierarchical bullet list of behavior deliberately dropped by the phase.

## New functionality
Hierarchical bullet list of behavior introduced by the phase.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|
| `tests/...` or test name | new / changed / removed | what it covers and why - short form |

## Tests run status

| Name | Status |
|---|---|
| `cargo test` (125) | OK / SKIPPED / FAILED |

One row per suite actually run, with exact counts where known.

## Found problems/issues/mismatches
- bullet list of problems found during implementation (or "None recorded.")

## Other important findings/information
- bullet list of anything else a future session should know (omit if empty).
```

### 2.4 Template: PHASE_N_TASK_M.md

One file per top-level planned task from `PHASE_N_PLAN.md` (subtask rows, e.g.
`1.1`, are covered inside their parent's task file). Written after user
confirmation of the plan (planning step j) and reviewed by the user (step k)
before any execution.

```markdown
# Phase N, Task M - Implementation details

> Written for a future LLM session: after reading `START.md`,
> `docs/roadmap/INTRO.md` and this file, the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal
1-3 sentences: what the task achieves and why it matters. A code block is allowed
for pipeline statements.

## Context
List of important notes, issues and findings discovered during the planning
process. All important information a future LLM session may need for the
implementation should be placed here.

## Scope

### Depends on tasks (optional)

| # | Task | Description + explanation |
|---|------|----------------------------|
| 1. | <task name> | why the current task depends on it - short form |

### Dependent tasks (optional)

| # | Task | Description + explanation |
|---|------|----------------------------|
| 1. | <task name> | why that task depends on the current one - short form |

### Affected source files (optional)

| File | Operation | Brief description |
|------|-----------|-------------------|
| `path` | new / change / remove | what will be done - short form |

### Affected documentation files (optional)

| File | Operation | Brief description |
|------|-----------|-------------------|
| `path` | new / change / remove | what will be done - short form |

### Affected test files (optional)

| File | Operation | Brief description |
|------|-----------|-------------------|
| `path` | new / change / remove | what will be done - short form |

### Affected training files (optional)

| File | Operation | Brief description |
|------|-----------|-------------------|
| `path` | new / change / remove | what will be done - short form |

### Other affected files (optional)

| File | Operation | Brief description |
|------|-----------|-------------------|
| `path` | new / change / remove | what will be done - short form |

### Affected build/run process (optional)
- list of changed/added/removed commands, flags, etc.

## Planned actions
All planned actions needed to reach the task goal, in execution order. Each
action is a subsection with a detailed description and rationale:

### <Action title>
Detailed description and rationale of the action: what will be done, why, how.

#### Action design decisions (optional)
- comparison tables for the design decisions taken in this action

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `path` | new | what will be added and why |
| `path` | remove | why the file will be removed |
| `path` | change (`<start line>:<end line>` optional) | what will be changed and why |

Rules:
- Affected files: EVERY file the action touches, each with operation type
  (`new`, `change`, `remove`) - including documentation files, valid/invalid
  examples, tests, and training-set files where applicable.
- A file with multiple changes gets several entries in its action's "Affected
  files" table, one entry per semantic block of text (two contiguous but
  semantically different blocks - e.g. different functions or documentation
  paragraphs - get two entries).

## Task deliverables
Bulleted list of concrete artifacts the task produces (files, commands, features,
docs). Paths relative to the repo root.

## Task exit criteria
Checklist. The task is complete only when ALL items are satisfied; the
implementation stage (steps l-m) checks this list verbatim.

## Non-goals
Bulleted list of explicitly excluded scope: what this task deliberately does NOT
do, and which later phase/tasks cover it, if known.

## Task corrections (optional)

| Correction | Description + rationale |
|------------|-------------------------|
| <correction title> | what changed in the task during execution and why - detailed form (with references to "Changes in design decisions", if needed) |

### Changes in design decisions (optional)
- list of changed design-decision comparison tables

Rules:
- The sections Goal, Context, Scope, Planned actions, Task deliverables, Task
  exit criteria and Non-goals are NEVER modified after the user approves the
  task during the implementation protocol. All deviations discovered during
  implementation are appended to this "Task corrections" section only, with a
  detailed description and rationale. Rationale: the planning process must stay
  analyzable (what was planned vs what was actually done) so it can be corrected
  in future phases.

## Other information
(optional.) Free-form notes for the task execution session: open questions,
assumptions to re-check, commands to run, risks.
```

### 2.5 Template: PHASE_N_TASK_M_STATUS.md

Two forms: 2.5.1 for a task cancelled before execution, 2.5.2 for a successfully
finished task.

#### 2.5.1 For a cancelled task

```markdown
# Phase N, Task M - Implementation status

Status: CANCELLED

## Brief explanation
Why the task was cancelled.

## Influenced tasks
List of tasks and the findings from their execution that led to cancelling the
current task.

## Tasks affected
List of tasks affected by the cancellation and how they are affected.
```

#### 2.5.2 For a successfully finished task

```markdown
# Phase N, Task M - Implementation status

> Written once when all exit criteria of `PHASE_N_TASK_M.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

| Name | Description |
|---|---|
| `src/...` | what it is + why it was added - short form |

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| `src/...` | changed / removed | what changed, or why removed - short form |

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `*.md` | new / changed / removed | what the doc change is and why - short form |

## Other file changes

| Name | Operation | Description |
|---|---|---|
| `examples/...`, `training/...`, build files, ... | new / changed / removed | examples, training sets, config, etc. - short form |

## Changed functionality
Hierarchical bullet list (nested bullets for sub-features) of behavior that
existed before the task and was modified.

## Removed functionality
Hierarchical bullet list of behavior deliberately dropped by the task.

## New functionality
Hierarchical bullet list of behavior introduced by the task.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|
| `tests/...` or test name | new / changed / removed | what it covers and why - short form |

## Tests run status

| Name | Status |
|---|---|
| `cargo test` (125) | OK / SKIPPED / FAILED |

One row per suite actually run, with exact counts where known.

## Found problems/issues/mismatches
- bullet list of problems found during implementation (or "None recorded.")

## Other important findings/information
- bullet list of anything else a future session should know (omit if empty).
```

## 3. Work protocol (two main stages)

Phase work is split into two strictly separated stages, each triggered by its own
user request. Never merge the stages: do not implement before the plan is written
and user-reviewed, do not execute a task before its `PHASE_N_TASK_M.md` file is
written and user-confirmed, and do not start planning for a phase whose
`PHASE_N.md` does not exist.

### 3.1 Planning stage (trigger: user says "let's make a plan for Phase N")

a) Thoroughly read `INTRO.md`.
b) Thoroughly read `PHASE_N.md` for the phase about to be implemented.
c) Think deeply about the implementation; read all necessary files as needed.
d) Identify ALL affected files, including all documentation files.
e) Re-check assumptions about affected files; read them fully if needed.
f) Re-check goals and deliverables of the corresponding phase (`PHASE_N.md`).
g) Write a detailed plan into `PHASE_N_PLAN.md` following template 2.2. The plan
   must be written for a future LLM session: after reading `START.md`, `INTRO.md`
   and this `PHASE_N_PLAN.md`, the assistant must be able to immediately start
   work on implementation. Where applicable, necessarily include examples update,
   tests update, and training-sets update as explicit tasks.
h) Review the new plan thoroughly: re-check all affected files, goals,
   deliverables, and the formatting rules from this file.
i) STOP here and ask the user to review the new plan. Do not implement anything.
j) After user confirmation of the plan, write one detailed `PHASE_N_TASK_M.md`
   per top-level planned task (template 2.4); subtask rows are covered inside
   their parent's task file.
k) STOP here and ask the user to review the task files. Do not implement
   anything.

### 3.2 Implementation stage (trigger: user says "let's implement Phase N")

a) Thoroughly read `START.md` and `INTRO.md` if needed.
b) Thoroughly read `PHASE_N.md` for the phase about to be implemented.
c) Check that `PHASE_N_PLAN.md` exists; if not, warn the user and STOP.
d) Re-check `PHASE_N_PLAN.md`: all affected files, goals, deliverables; re-read
   all needed files; take into account the "Plan corrections" table from previous
   attempts of planned-task execution, the "Task corrections" tables of the
   `PHASE_N_TASK_M.md` files of already implemented tasks of the current phase,
   and the "Found problems/issues/mismatches" + "Other important
   findings/information" sections of their `PHASE_N_TASK_M_STATUS.md` files.
e) Re-check all `PHASE_N_TASK_M.md` files of not-yet-implemented tasks of the
   current phase; make edits if needed, strictly following template 2.4. Add new
   tasks (with corresponding `PHASE_N_TASK_M.md` files and "Plan corrections"
   entries) if needed.
f) Execute the planned tasks in a loop, one by one:
g) Take the next not-yet-implemented task from its `PHASE_N_TASK_M.md`, taking
   task dependencies into account.
h) Print a message to the user with the task number, a brief description (what
   is planned to be done, list of actions, what will be achieved) and a brief
   scope (which files will be changed).
i) STOP here and ask the user to review the `PHASE_N_TASK_M.md` file. This gate
   applies to EVERY task, including tasks added during execution (steps e, q).
j) Continue only after user confirmation (trigger: user says "let's execute the
   task").
k) Execute the actions from the `PHASE_N_TASK_M.md` file.
l) Check the task exit criteria (`PHASE_N_TASK_M.md`). If not reached, correct
   the task ("Task corrections" table) and re-execute the actions (goto k),
   repeat until the exit criteria are reached.
m) After reaching the task exit criteria, write `PHASE_N_TASK_M_STATUS.md`
   according to template 2.5.2.
n) Reflect the task corrections introduced during work in the "Task corrections"
   table of the `PHASE_N_TASK_M.md`; strictly follow the formatting rules
   (template 2.4).
o) Reflect the plan-level corrections introduced during work in the "Plan
   corrections" table of `PHASE_N_PLAN.md`; NEVER touch the initial "Planned
   tasks" table; follow the formatting rules from this file. If an unimplemented
   task was cancelled, write its `PHASE_N_TASK_M_STATUS.md` according to template
   2.5.1.
p) Check for remaining not-cancelled tasks (consult `PHASE_N_PLAN.md`); if any
   remain, repeat the loop (goto g).
q) If no tasks remain, check the phase exit criteria (`PHASE_N.md`). If not
   reached, correct the plan ("Plan corrections" table), add new tasks (new
   `PHASE_N_TASK_M.md` files) if needed, and re-execute the loop (goto d), repeat
   until the phase exit criteria are reached.
r) After reaching the phase exit criteria, write `PHASE_N_STATUS.md` according to
   template 2.3.
s) Update all related documentation files; re-check that docs are aligned with the
   current state of the project (`KB.md`, `DESIGN.md`, `BUILD.md`, `CODEGEN.md`,
   backend docs, and the phase-state table in section 1 of this file).
t) Update `TASKS.md` (entry per its first-paragraph template) and commit.

## 4. General rules for this document set

- All files here are small by design: read the specific file you need fully; do
  not grep-and-sed them the way you treat `KB.md`/`DESIGN.md`.
- Phase descriptions (`PHASE_N.md`) are frozen once planning starts; anything new
  learned goes into `PHASE_N_PLAN.md` (corrections) or `PHASE_N_STATUS.md`, never
  silently into the phase file.
- Keep the phase-state table in section 1 current.
- Backend parity (guiding principle) applies to ALL phases and ALL tasks: every plan must
  include per-backend tasks for every backend feature and at least one
  cross-backend differential test comparing backends' behavior on identical
  specs/seeds; a phase may not exit with any backend left behind on in-scope
  functionality.
- The "Recommended v1 scope" and implementation-stack guidance in sections 5-6
  apply to ALL phases; they moved here from the old `ROADMAP.md`.

## 5. Recommended v1 scope

To keep the first production-ready version manageable, v1 should include:

- external `.rfs` files;
- single endpoint module;
- finite states as enums;
- events and outputs with finite payload fields;
- variables of type `bool`, enum, `bit<N>`;
- pure guarded transitions;
- simultaneous variable updates;
- explicit priorities for overlapping transitions;
- deterministic production semantics;
- `Result`-based unexpected event handling;
- optional explicit error transition;
- `Tick` event and timer variables;
- PSL-like safety properties:
  - boolean atoms;
  - `next`;
  - `always`;
  - `always[N]`;
  - `bounded_eventually[N]`;
- assumptions using the same property language;
- exact reachable-state MC for small systems;
- bounded MC mode;
- counterexample JSON/text output;
- production code generation in all backends;
- optional debug property monitors;
- CLI and `build.rs` integration;
- generated unit tests.

v1 should not include:

- multi-party composition;
- real-time timed automata;
- unbounded integers;
- arbitrary Rust function calls in transitions;
- full strong unbounded liveness unless implementation goes smoothly;
- LSP;
- advanced symbolic/SMT MC;
- byte-level protocol codecs.

These can be added later without changing the core architecture.

## 6. Suggested implementation stack

| Component | Recommended technology |
|---|---|
| Parser | `nom` 8 recursive descent parser for Phase 0; `chumsky` remains a future candidate for richer diagnostics/recovery |
| Diagnostics | Custom span-based diagnostics first; `miette` later if useful |
| IR serialization | `serde` + `serde_json` initially; protobuf later if needed |
| CLI | `clap` later; simple `std::env::args` for Phase 0 |
| Code generation | Direct Rust string generation first; `proc-macro2`/`quote` later if useful |
| Formatting generated Rust | `rustfmt` programmatic invocation or `rustfmt` crate |
| Testing | `cargo test`, snapshot tests later, property-based tests later |
| Fuzzing | `cargo-fuzz` / `libfuzzer` later |
| LSP later | `tower-lsp` |

### Model-checking library stack

| Component | Recommendation |
|---|---|
| Exact search core | Custom Rust graph search with hash sets/bitsets |
| State packing | Generated bit-packing for states/vars/monitors |
| Counterexample storage | Parent pointers or trace recording |
| Bounded/SMT later | Optional `z3` bindings behind feature flag |
| Symbolic later | Custom BDD or SAT-based backend, optional |
