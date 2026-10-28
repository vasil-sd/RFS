# RFS Developer Documentation (Class A)

Audience: engineers driving project development. This set is rich,
diagram-heavy, rationale-focused and educational: after reading it a developer
must be able to understand the source layout, identify which files implement
which functionality, follow any feature from spec syntax through IR to
generated artifacts, and understand WHY each major design decision was made.

For compact task-oriented guidance (functionality-to-source maps, change-impact
tables, per-module cards with line budgets), see the Class B set in
`docs/llm/`.

<a id="dev:index"></a>
## Index

| File | Content | Filled by |
|------|---------|-----------|
| `ARCHITECTURE.md` | pipeline overview, component diagram, functionality hierarchy | tasks 2.1-2.2 |
| `CODE_MAP.md` | file-by-file map of all 72 hand-written source files | tasks 3.1-3.4 |
| `DEPENDENCIES.md` | file-level + function-level dependency graphs (mermaid) | tasks 4.1-4.5 |
| `DECISIONS.md` | rationale index over every decision entry in `DESIGN.md` sections 9, 19-25 | tasks 5.1-5.4 |
| `ALGORITHMS.md` | main-algorithm write-ups at L1/L2/L3 detail levels | tasks 6.1-6.4 |
| `REFERENCES.md` | external articles/books/publications index | tasks 7.1-7.2 |
| per-subsystem deep dives (11 files) | parser, validator, IR, codegen, backends, MC, monitor, CLI, crates, fuzz | tasks 8.1-12.3 |

## Conventions (machine-checked by the sync gate)

The traceability conventions below are enforced by `tests/dev_docs_test.rs`
(part of plain `cargo test`); the suite's module doc is the canonical
statement:

- **Fragment IDs**: `dev:<name>` / `llm:<name>` (kebab-case `<name>`). A
  fragment is defined by an anchor line immediately followed by a heading;
  IDs are unique across `docs/dev/` + `docs/llm/`. The anchor form:

  ```markdown
  <a id="dev:name"></a>
  ## Section heading
  ```

- **doc-to-code references**: inline code spans of the form shown below -
  repo-rooted, at least one path separator, `.rs`/`.c`/`.h` only; `::item`
  for `.rs` only. Anchors are stable identifiers (function/type names), NEVER
  line numbers. A backticked string matching this shape MUST be a real
  reference:

  ```text
  src/mc_plan.rs              file-level reference
  src/mc_plan.rs::some_item   item-level reference
  ```

- **code-to-doc pointers**: every manifest-listed source file carries exactly
  one `//! Docs: dev:name, llm:name` module-doc line; the pointer scope is
  driven by `tests/dev_docs_manifest.txt`.
- **Ownership**: every fragment must be named by at least one source file's
  pointer line (no orphans) - the owner is the file whose changes make the
  fragment stale.

After altering source code, run `cargo test --test dev_docs_test`: its output
alone tells you which doc fragments are stale or missing.
