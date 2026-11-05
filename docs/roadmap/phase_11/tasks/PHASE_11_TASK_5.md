# Phase 11, Task 5 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start implementation of the task immediately without re-deriving scope.

## Goal
Move the language backends into `src/codegen/backends/{rust,c}/` (one directory
per language, one file per artifact) and resolve the flagged cross-backend
coupling by extracting the identifier utilities into a new neutral module
`src/codegen/ident.rs`.

## Context
- Mapping: backends/mod.rs -> codegen/backends/mod.rs (re-export list updated to
  the new sub-paths); rust.rs -> codegen/backends/rust/mod.rs and c.rs ->
  codegen/backends/c/mod.rs (absorbed as mod.rs); rust_fsm/rust_test/rust_mc ->
  rust/{fsm,test,mc}.rs; c_fsm/c_test/c_mc -> c/{fsm,test,mc}.rs. The three
  artifacts stay separate files - merging would destroy the split the parity
  invariant is checked against.
- Ident extraction (proposal risk notes): `snake`, `rust_ident`,
  `rust_binding_ident` currently live in backends/rust.rs and are borrowed by
  the C emitters - the one deliberate cross-backend coupling. They move to NEW
  src/codegen/ident.rs; rust/mod.rs re-exports or imports from there as needed;
  c/* emitters import from codegen::ident. After this task NO c/* file imports
  anything from the rust/ module.
- External import sites of backends::*: 3 (2 bin files + 1 test file), paths
  `rfs::backends::<lang>_<artifact>` -> `rfs::codegen::backends::<lang>::<artifact>`
  - re-verify by grep at task start.

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 4. | codegen/ shared layer | backends/ nests under it; its import lines are final |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 6.-8. | cli, mc | their backend import lines are already final |

### Affected source files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `src/backends/mod.rs` | remove (moved) | git mv to codegen/backends/mod.rs + re-export update |
| `src/backends/rust.rs` | remove (moved) | git mv to codegen/backends/rust/mod.rs; ident fns leave |
| `src/backends/c.rs` | remove (moved) | git mv to codegen/backends/c/mod.rs |
| the six emitters | remove (moved) | git mv per mapping |
| `src/codegen/ident.rs` | new | snake / rust_ident / rust_binding_ident |
| c/* emitters + rust/mod.rs | change | ident imports from codegen::ident |
| 3 external backends::* import sites | change | path updates, scripted |

## Planned actions

### Move the eight files into the two language directories
git mv per mapping; update codegen/backends/mod.rs's re-export list to the new
sub-paths; script the 3 external import-site rewrites. Rationale: the Rust/C
parity axis becomes two sibling directories, visible in the tree.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/backends/mod.rs` | remove | git mv to codegen/backends/mod.rs; re-exports updated |
| `src/backends/rust.rs`, `src/backends/c.rs` | remove | git mv to the language dirs' mod.rs |
| `src/backends/{rust_fsm,rust_test,rust_mc,c_fsm,c_test,c_mc}.rs` | remove | git mv per mapping |
| 3 external backends::* import sites | change | scripted path rewrite |

### Extract the identifier utilities into codegen/ident.rs
Create src/codegen/ident.rs with snake / rust_ident / rust_binding_ident (moved
verbatim from rust/mod.rs); update rust/mod.rs and every c/* emitter to import
from codegen::ident. Rationale: executes the proposal's flagged resolution - no
C file may depend on a Rust-backend module.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/codegen/ident.rs` | new | the three identifier utilities, moved verbatim |
| `src/codegen/backends/rust/mod.rs` | change | fns removed (now in ident.rs) or re-exported |
| `src/codegen/backends/c/{fsm,test,mc}.rs` | change | ident imports from codegen::ident |

### Gate
cargo build && cargo test green; grep proves no c/* file imports from the rust/
module.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none) | - | verification only |

## Task deliverables
- `src/codegen/backends/{rust,c}/` per the proposal; `src/codegen/ident.rs`;
  zero cross-backend imports.

## Task exit criteria
- [ ] cargo build && cargo test green (535 baseline).
- [ ] No c/* file imports anything from codegen::backends::rust (grep-verified).
- [ ] snake/rust_ident/rust_binding_ident defined exactly once, in codegen/ident.rs.
- [ ] No file references the old src/backends/* paths (grep-verified).

## Non-goals
- No emitter behavior change; generated output byte-identical.
