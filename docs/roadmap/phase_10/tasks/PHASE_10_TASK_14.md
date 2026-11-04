# Phase 10, Task 14 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start implementation of the task immediately without re-deriving scope.

## Goal
Propose the project's component and functionality category systems by reviewing
the finished inventory, and fill the `## Components` and `## Functionality`
tables of `SRC_STRUCTURE.md`. These categories are the classification axes for
tasks 15-16 and the justification basis for the naming scheme (task 17).

## Context
- The full non-test inventory exists after task 13 (938 function rows +
  197 type rows = 1,135 rows per plan correction 13a) with complete Depends edges - that edge picture is the input
  to this task.
- Start from the seed proposal in PHASE_10_PLAN.md Context (Components:
  parser, IR model, JSON I/O, printer, validator, monitor compiler, codegen
  core, shared test planning, shared MC concepts, MC plan, SMT plan+emitter,
  Rust backend, C backend, CLI dispatch, frontend CLI logic, external-tool
  resolution, rfs-test command, info command, info-format registry, EBNF
  grammar model, JSON-schema emitter, diagram core + plugins, reference MC,
  spec generation, spec hashing, training pipeline, rfs-build crate,
  rfs-adapters crate, example crates. Functionality: parsing, semantic
  validation/typing, IR serialization, codegen-production, codegen-tests,
  model-checking, property monitoring, PSL translation, diagnostics, CLI
  surface, self-describing formats/documentation, provenance, fuzzing,
  training data, build integration, protocol adapters/integration).
- Refine the seed against the actual rows: merge categories that the Depends
  edges show are inseparable, split ones that hide two responsibilities, rename
  for clarity. Every category needs a depends-on list (other categories) and a
  1-2 line description + rationale.
- A function/type may belong to SEVERAL components and functionalities - the
  category tables define the vocabulary; assignment happens in tasks 15-16.
- Backend parity is a project invariant: the Rust and C backends stay separate
  categories (they are separate code), but their functionality assignments must
  end up symmetric - check this explicitly while designing the vocabulary.

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 2.-13. | All enumeration chunks | the complete inventory + dependency edges are the input to category design |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 15., 16. | Classification fills | use this task's vocabulary |
| 17. | Naming scheme | justified against these categories |
| 31. | Test categories | reuses this vocabulary where applicable |

### Affected documentation files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `SRC_STRUCTURE.md` | change | `## Components` + `## Functionality` tables filled (name / depends on / description + rationale) |

## Planned actions

### Review the dependency picture
Scan all inventory rows: which files call into which, where the Depends edges
concentrate, which clusters are self-contained. Note observations that conflict
with the seed proposal.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none) | - | analysis only |

### Design and write the category tables
Fill both tables: one row per category with its depends-on categories and a
1-2 line description + rationale. Keep the sets minimal-but-complete: every
source file must be assignable to at least one component and one functionality
(verify by walking the file list, not by guessing).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `SRC_STRUCTURE.md` | change (`## Components`, `## Functionality`) | the two category tables - the classification vocabulary for the rest of the phase |

### Gate check
docs_md_test green.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none) | - | verification only |

## Task deliverables
- Filled `## Components` and `## Functionality` tables in SRC_STRUCTURE.md.

## Task exit criteria
- [ ] Both tables filled: every row has name, depends-on list, description +
      rationale (1-2 lines).
- [ ] Every non-test source file is assignable to >= 1 component and >= 1
      functionality under the new vocabulary (walk the file list to verify).
- [ ] Rust/C backend categories are separate but their intended functionality
      assignments are symmetric (parity invariant respected in the design).
- [ ] Deviations from the seed proposal are explained in-row (rationale column).
- [ ] docs_md_test green.

## Non-goals
- No row-level classification yet (tasks 15-16), no naming scheme (task 17),
  no source changes.
