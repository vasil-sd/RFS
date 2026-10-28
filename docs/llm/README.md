<!-- llm-budget: 70 -->
# RFS LLM-Assistant Documentation (Class B)

Audience: LLM assistants executing tasks in this repo. Compact, task-oriented,
context-budgeted: read exactly the fragment(s) you need, never whole files.
For rationale and algorithm detail, follow the links to Class A (`docs/dev/`)
- this set never restates it.

<a id="llm:index"></a>
## Index and context-budget rules

| File | Use it for |
|------|------------|
| `TASK_MAP.md` | functionality to files/functions: every CLI command, subsystem, test suite |
| `CHANGE_IMPACT.md` | change type to affected source files + tests + doc fragments |
| per-module cards (11 files) | one compact card per core module: responsibility, key items, invariants |

Workflow for a task: (1) find the relevant fragment(s) via `TASK_MAP.md`;
(2) read them; (3) check `CHANGE_IMPACT.md` for what else must change;
(4) after editing source, run `cargo test --test dev_docs_test` - its output
names exactly which doc fragments to update.

Rules (machine-checked by the sync gate):

- every file in this directory declares a line-budget header in its first 10
  lines and stays within it;
- fragments are addressable by stable IDs (`llm:<module>`) so you read exactly
  one fragment instead of a whole file;
- no content duplication with Class A - link to it, never restate.
