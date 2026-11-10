# RFS Roadmap - Formatting Rules (Templates)

## 1. General formatting rules

All phase documents MUST follow the templates below exactly: same section names,
same order, same table columns. Sections marked "(optional)" may be omitted when
empty; all others are mandatory. Descriptions in tables must include explanation
and rationale in a short form (what + why, 1-2 lines).

File locations: `PHASE_N.md`, `PHASE_N_PLAN.md` and `PHASE_N_STATUS.md` live in
`docs/roadmap/phase_N/`; task files (`PHASE_N_TASK_M*.md`) live in
`docs/roadmap/phase_N/tasks/`. See `INTRO.md` section 1 for the full file set.

## 2. Template: PHASE_N.md

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
implementation stage (PROTOCOL.md §3 step r) checks this list verbatim.

## Non-goals
Bulleted list of explicitly excluded scope: what this phase deliberately does NOT
do, and which later phase covers it, if known.
```

## 3. Template: PHASE_N_PLAN.md

```markdown
# Phase N - Implementation plan

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start
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
  rationale of the cancellation (FORMATTING.md §6.1).
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

## 4. Template: PHASE_N_STATUS.md

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

## 5. Template: PHASE_N_TASK_M.md

One file per top-level planned task from `PHASE_N_PLAN.md` (subtask rows, e.g.
`1.1`, are covered inside their parent's task file). Written after user
confirmation of the plan (PROTOCOL.md §2 step j) and reviewed by the user (step k)
before any execution.

```markdown
# Phase N, Task M - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start
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
implementation stage (PROTOCOL.md §3 steps l-m) checks this list verbatim.

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

## 6. Template: PHASE_N_TASK_M_STATUS.md

Two forms: 6.1 for a task cancelled before execution, 6.2 for a successfully
finished task.

#### 6.1 For a cancelled task

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

#### 6.2 For a successfully finished task

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

## 7. Template: git commit message

A commit message is a one-line subject, a blank line, then a body of three
plain-text sections. The section labels are bare text, NOT `#`-prefixed - git
strips `#` lines as comments, so a `# Changes` header would silently vanish from
the stored message:

```markdown
<one-line short description of the change>

Changes:
- <what was actually changed (files, API, functionality, etc), one bullet per item>

Issues and findings:
- <issues or findings discovered during the work, one bullet each; omit this section if there are none>

Verification results:
- <the tests passed / checks run that confirm the goals were reached>
```

Rules:
- The subject is a single line (imperative mood, no trailing period); it must stand
  alone in `git log --oneline`.
- Each section is a bare label followed by a bullet list; drop a whole section only
  if it is genuinely empty.
- Keep bullets factual and specific - name the files, commands and counts, not
  narrative.
