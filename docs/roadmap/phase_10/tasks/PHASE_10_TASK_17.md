# Phase 10, Task 17 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start implementation of the task immediately without re-deriving scope.

## Goal
Propose a new naming scheme for source files and a target directory tree, in a
new `## Proposed naming scheme` section of `SRC_STRUCTURE.md`, justified against
the task-14 component/functionality categories. This is PROPOSAL only - nothing
is moved or renamed in this phase.

## Context
- Input: the fully classified inventory (tasks 15-16) + the category tables
  (task 14). The scheme must make the category structure visible in the file
  tree: files grouped by component, named by responsibility.
- Hard constraints the proposal MUST respect (violating any of them makes the
  later refactoring phase much more expensive):
  - Cargo workspace layout: root package `rfs` (lib + CLI + bins),
    `crates/rfs-build`, `crates/rfs-adapters`, example crates; the C example
    projects are Makefile-driven non-members and stay put.
  - The detached root `fuzz/` cargo-fuzz workspace keeps its own layout
    (it is test infrastructure - its reorganization belongs to the TEST side,
    task 33).
  - `src/bin/*.rs` bin names are cargo aliases' targets (`cargo build-training`
    etc. via `.cargo/config.toml`) - renaming a bin file requires updating the
    alias; the proposal must say so explicitly per renamed bin.
  - The 71-file `//! Docs:` manifest + dev_docs_test: every moved/renamed file
    keeps its pointer line; the refactoring phase re-syncs docs/dev + docs/llm
    (out of scope here, but the proposal should note which doc fragments would
    need retargeting).
  - Generated code is standalone - no generated artifact references toolchain
    source paths, so codegen output is unaffected by any rename.
  - `rfs-build`'s RFS_BIN resolution and example build.rs files reference the
    toolchain binary, not source paths - unaffected, but note it.
- Naming conventions to propose (start here, adjust against the categories):
  snake_case module files mirroring module paths; command files named after
  their subcommand (`mc_ref_cmd.rs` -> e.g. `commands/mc_ref.rs` style);
  backend files keep the `<lang>_<artifact>.rs` split (fsm/test/mc) since the
  three artifacts are genuinely separate responsibilities; shared-plan files
  grouped under a codegen/ or mc/ subdirectory per the category picture.
- The section must contain: (a) the naming conventions as rules, (b) the target
  directory tree in text form (every current file mapped to its proposed new
  path, or explicitly kept), (c) per-group rationale tied to component names,
  (d) a risk/effort note per non-trivial move (what breaks: aliases, manifest,
  doc pointers).

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 14. | Categories proposal | the scheme is justified against these categories |
| 15., 16. | Classification fills | the per-file component/functionality picture drives the grouping |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 18. | File mapping table + mermaid | renders this scheme as the mapping table + diagram |

### Affected documentation files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `SRC_STRUCTURE.md` | change | new `## Proposed naming scheme` section (conventions, target tree in text, rationale, risk notes) |

## Planned actions

### Derive the grouping from the categories
Walk the classified file picture: which files share a component, where the
Depends edges cross proposed group boundaries. Draft the target tree so that
intra-component edges stay inside one directory and cross-component edges go
through clearly named module roots.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none) | - | design work, output goes into the section below |

### Write the section
Add `## Proposed naming scheme` to SRC_STRUCTURE.md with parts (a)-(d) from
Context. Every current non-test source file appears exactly once in the target
tree (moved or explicitly kept).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `SRC_STRUCTURE.md` | change (`## Proposed naming scheme`) | the proposed scheme - the design input for task 18's mapping table and the later refactoring phase |

### Constraint audit
Check the proposal against every hard constraint in Context (workspace layout,
fuzz workspace, bin aliases, //! Docs: manifest, generated-code independence);
fix violations in the section text.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `SRC_STRUCTURE.md` | change (`## Proposed naming scheme`) | constraint-audit fixes |

### Gate check
docs_md_test green.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none) | - | verification only |

## Task deliverables
- `## Proposed naming scheme` section in SRC_STRUCTURE.md: conventions, target
tree (text), per-group rationale, per-move risk notes.

## Task exit criteria
- [ ] Section exists with all four parts (a)-(d).
- [ ] Every current non-test source file appears exactly once in the target tree.
- [ ] All hard constraints from Context satisfied (workspace, fuzz/, bin
      aliases, //! Docs: manifest, generated-code independence) - each checked
      and noted.
- [ ] Each proposed group's rationale names the component(s) it realizes.
- [ ] docs_md_test green.

## Non-goals
- No actual moves/renames (later refactoring phase), no mermaid yet (task 18),
  no test-tree proposal (task 33).
