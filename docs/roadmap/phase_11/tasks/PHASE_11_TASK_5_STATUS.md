# Phase 11, Task 5 - Status

> Written when the task's exit criteria are reached.

## Added source files

| Name | Description |
|---|---|
| `src/codegen/ident.rs` | NEW: identifier-mangling utilities (snake, rust_ident, rust_binding_ident + RUST_KEYWORDS) extracted from the Rust backend - the neutral home resolving the flagged C->Rust coupling; //! Docs: dev:codegen-core-overview |

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| `src/backends/mod.rs` -> `src/codegen/backends/mod.rs` | moved + changed | declarations reduced to the two language dirs (pub mod c; pub mod rust;) |
| `src/backends/rust.rs` -> `src/codegen/backends/rust/mod.rs` | moved + changed | ident trio + RUST_KEYWORDS extracted out; 3 sibling pub mod decls added; use of codegen::ident |
| `src/backends/c.rs` -> `src/codegen/backends/c/mod.rs` | moved + changed | 3 sibling pub mod decls added |
| `src/backends/{rust_fsm,rust_test,rust_mc}.rs` -> `src/codegen/backends/rust/{fsm,test,mc}.rs` | moved + changed | import-line rewrites only (incl. cross-sibling c_fsm-style refs to nested form) |
| `src/backends/{c_fsm,c_test,c_mc}.rs` -> `src/codegen/backends/c/{fsm,test,mc}.rs` | moved + changed | import-line rewrites; ALL former crate::backends::rust::{snake,rust_ident} refs now point at codegen::ident - zero c/* imports from rust/ remain |
| `src/lib.rs` | change | pub mod backends; removed (path is now crate::codegen::backends) |
| `src/codegen/mod.rs` | change | gains pub mod backends; + pub mod ident;; own use updated |
| `src/bin/{generate_code_tests,test_generated_code}.rs` | change | grouped root import rfs::{backends::rust_test, ...} -> aliased codegen::backends::rust::test as rust_test (bodies untouched) |
| `tests/codegen_test.rs` | change | 18 external rfs::backends::<lang>_<artifact> sites -> nested rfs::codegen::backends::<lang>::<artifact> form |

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `tests/dev_docs_manifest.txt` | changed | 9 old backends paths replaced by the 10 new sorted entries (incl. codegen/backends/mod.rs) + new src/codegen/ident.rs entry |
| `docs/dev/CODE_MAP.md` | changed | section 18->19 files; Codegen core 4->5 (new ident.rs entry); backends/mod.rs entry moved to the section root; Rust backend 5->4 + C backend 4 entries rewritten to the nested paths |
| `docs/llm/TASK_MAP.md` | changed | file-list paths rewritten + new ident.rs coverage row |
| `docs/dev/backend_{c,rust}.md`, `docs/dev/codegen_core.md`, `docs/llm/backend_{c,rust}.md`, `docs/llm/CHANGE_IMPACT.md`, `docs/dev/DEPENDENCIES.md`, `docs/dev/ALGORITHMS.md` (+ other live docs) | changed | every src/backends/* reference rewritten to the new nested paths; directory-level mentions (src/backends/, rust*.rs globs, {rust,c}_mc.rs brace form) updated; C.md/RUST.md refs re-pointed to codegen/backends/; ident-function doc refs re-pointed from rust/mod.rs to ident.rs (22 gate problems) |
| `src/codegen/backends/{C,RUST}.md` | moved | the two backend architecture docs moved with their code (the proposal tree covered .rs files only - recorded here); module doc comments + docs/dev relative links updated |

## Other file changes

(none)

## Changed functionality

- None - pure moves/renames + the ident extraction (same three functions, same
  bodies, new neutral home). Codegen output byte-identical (determinism suites green).

## Removed functionality

- The cross-backend coupling: no C-backend file imports anything from the
  Rust-backend module anymore (grep-verified: 0 hits).

## New functionality

- `codegen/ident.rs` as the shared identifier-mangling home (the proposal's
  flagged resolution, executed).

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|
| `tests/codegen_test.rs` | changed | import paths only - no test logic touched |

## Tests run status

| Name | Status |
|---|---|
| `cargo build` | OK |
| `cargo test` (full) | OK - 535 passed / 0 failed, the documented baseline exactly |
| `cargo test --test dev_docs_test` (9) | OK - green after the manifest + doc-ref re-sync |

## Found problems/issues/mismatches

- The plan's "3 external import sites (2 bin files + 1 test file)" re-verified TRUE, but the bins' form was a grouped crate-root import (use rfs::{backends::rust_test, ...}) - fixed with an aliased import so their bodies stay untouched.
- Cross-sibling calls (c/mc.rs -> c_fsm::generate_core) needed a SECOND rewrite pass: the bulk crate::backends:: substitution produced backends::c_fsm, but the nested tree requires backends::c::fsm.
- The absorbed rust/mod.rs + c/mod.rs each needed three new pub mod declarations (task-4 lesson); codegen/mod.rs needed BOTH pub mod backends; and pub mod ident; (the first was compiler-caught).
- Doc refs to the ident functions were bulk-rewritten to rust/mod.rs::snake etc. by the path sed, then failed the gate (item not declared there) - re-pointed to ident.rs (22 problems).

## Other important findings/information

- C.md/RUST.md moved with the code: a deliberate small scope extension beyond
  the proposal's .rs-only tree (leaving src/backends/ alive for two doc files
  would be wrong); all references updated.
