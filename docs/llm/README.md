<!-- llm-budget: 70 -->
# RFS LLM-Assistant Documentation (Class B)

Audience: LLM assistants executing tasks in this repo. Compact, task-oriented,
context-budgeted: read exactly the fragment(s) you need, never whole files.
For rationale and algorithm detail, follow the links to Class A (`docs/dev/`)
- this set never restates it.

<a id="llm:index"></a>
## Index and context-budget rules

| File | Budget | Use it for |
|------|--------|------------|
| `TASK_MAP.md` | 400 | functionality to files/functions: every CLI command, subsystem, test suite |
| `CHANGE_IMPACT.md` | 250 | change type to affected source files + tests + doc fragments |
| per-module cards (11) | 120-200 each | one compact card per core module: responsibility, key items, invariants |

The 11 module cards: `parser.md` (120), `validator.md` (120),
`ir_json_rw.md` (150), `codegen_core.md` (150), `backend_rust.md` (150),
`backend_c.md` (150), `mc_engine.md` (150), `monitor_compiler.md` (120),
`cli_commands.md` (200), `workspace_crates.md` (200), `toolchain_fuzz.md`
(200).

Workflow for a task: (1) find the relevant fragment(s) via `TASK_MAP.md`;
(2) read them; (3) check `CHANGE_IMPACT.md` for what else must change;
(4) after editing source, run `cargo test --test dev_docs_test` - its output
names exactly which doc fragments to update.

Rules (machine-checked by the sync gate, tests/dev_docs_test.rs):

- every file in this directory declares a line-budget header (`<!-- llm-
  budget: N -->`) in its first 10 lines and stays within it;
- fragments are addressable by stable IDs (`llm:<module>`) - locate one with
  `grep -n 'id="llm:<module>"' <file>` and read only that section, never the
  whole file;
- no content duplication with Class A - link to it, never restate;
- `TASK_MAP.md` must cover every manifest-listed source file as a doc->code
  reference - when you add a tracked .rs file, extend tests/dev_docs_manifest.
  txt and its //! Docs: line in the same change.
