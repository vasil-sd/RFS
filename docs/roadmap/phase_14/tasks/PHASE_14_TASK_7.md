# Phase 14, Task 7 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal
Sync every living document with goal 1's rule (only the `rfs` binary is
referenced for invoking rfs commands - no `cargo run -- <subcommand>`
pattern) and with the new codegen CLI (flag family, spec-stem default naming,
artifact pairs, object+link builds). Legitimate cargo usages stay untouched.

## Context

- Verified occurrence map (2026-11-13, grep over living docs only):
  - `KB.md` section 4.5 (CLI behavior): ~20 example lines of the form
    `cargo run -- <command> ...` (parse/validate/info/info format/codegen/
    test/diagram blocks, plus two inline mentions around lines ~529/534).
  - `BUILD.md`: ~14 lines in the command tables (sections 3-4) + quick-start
    references; also legitimate `cargo run -p rfs-dev` / workspace usages that
    STAY.
  - `README.md`: mermaid node labels (`cargo run -- codegen rust production/
    test/mc`, `cargo run -- codegen c ...`) + surrounding text.
  - `START.md` section 7: one sanity line (`cargo run -- validate --input rfs
    examples/valid`).
  - `docs/user/README.md`: mermaid labels + the two-minute tour examples
    (~7 lines) + the conventions note ("via `cargo run --`; an installed binary
    is just `rfs`").
  - `docs/user/INTEGRATION.md` (~9), `docs/user/LANGUAGE.md` (~10),
    `docs/user/MODEL_CHECKING.md` (~2), `docs/user/PROPERTIES.md` (~1),
    `docs/user/EXAMPLES.md` (~3) - command examples throughout.
- Legitimate cargo usages that must NOT change (grep will still find `cargo
  run` after this task - that is correct): workspace build/test commands in
  BUILD.md, `cargo run -p sync_handler` / `-p async_handler` (example crates
  are cargo members), the cargo-fuzz pipeline documentation, `cargo run -p
  rfs-dev`.
- The codegen walkthroughs must be updated to the NEW CLI wherever they show
  generation: the two-minute tour (docs/user/README.md), integration examples
  (INTEGRATION.md - e.g. the `--no-std` production example, the gen-spec +
  mc-ref workflow), the MC workflow (MODEL_CHECKING.md - now shows the fsm/mc
  file pair + object linking), and EXAMPLES.md walkthroughs.
- `tests/docs_md_test.rs` renders EVERY mermaid block in docs/user/* with the
  mdr-stack parser - diagram edits must keep all blocks syntactically valid.
- Historical records are frozen: `docs/roadmap/phase_0..13/*`, `TASKS.md`
  history entries, `commands.md` keep their `cargo run --` text (non-goal).

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 6. | Help-text fix | The help text is the reference the docs mirror; docs must not describe a CLI the binary no longer prints. |
| 1.-5. | Codegen artifact tasks (as completed) | Walkthroughs show the final artifact model (pairs, references, object+link). |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 8. | Codegen architecture docs + dev/llm re-sync | Maintainer-facing docs; this task covers user-facing + root docs. |
| 11. | Close-out | Verification incl. docs_md_test. |

### Affected documentation files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `KB.md` | change | Section 4.5: ~20 example lines -> `rfs ...`; codegen rows updated to the new flag family + default policy. |
| `BUILD.md` | change | Command tables + quick-start: `cargo run -- <cmd>` -> `rfs <cmd>`; codegen rows updated; legitimate cargo usages untouched. |
| `README.md` | change | Mermaid node labels + text: `rfs codegen ...`. |
| `START.md` | change | Section 7 sanity line -> `rfs validate --input rfs examples/valid`. |
| `docs/user/README.md` | change | Mermaid labels, two-minute tour examples, conventions note ("every command example is copy-paste runnable ... as the installed `rfs` binary"). |
| `docs/user/INTEGRATION.md` | change | ~9 examples -> `rfs ...`; codegen walkthroughs updated to the new CLI. |
| `docs/user/LANGUAGE.md` | change | ~10 examples -> `rfs ...`. |
| `docs/user/MODEL_CHECKING.md` | change | ~2 examples -> `rfs ...`; MC workflow shows the fsm/mc file pair + object linking. |
| `docs/user/PROPERTIES.md` | change | 1 example -> `rfs ...`. |
| `docs/user/EXAMPLES.md` | change | ~3 examples -> `rfs ...`; walkthroughs updated where they show codegen. |

## Planned actions

### Action 7.1 - Root + maintainer-adjacent docs (KB.md, BUILD.md, README.md, START.md)

Rewrite every `cargo run -- <rfs-subcommand>` example to `rfs <subcommand>`
(same arguments); update the codegen rows/examples to the new flag family and
spec-stem default policy; leave all legitimate cargo usages (workspace builds,
`-p` example crates, cargo-fuzz, rfs-dev) byte-for-byte untouched. In
README.md's mermaid diagram, rename the two codegen node labels to `rfs
codegen ...`.

Rationale: these four files are what a new reader (human or LLM session) sees
first - they must not teach the removed invocation pattern.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `KB.md` | change (section 4.5 example blocks) | Pattern rewrite + codegen rows. |
| `BUILD.md` | change (command tables, quick-start) | Pattern rewrite + codegen rows. |
| `README.md` | change (mermaid + text) | Node labels + surrounding prose. |
| `START.md` | change (section 7) | Sanity line. |

### Action 7.2 - User documentation set (docs/user/*)

Same pattern rewrite across the six user docs; then update every codegen
walkthrough to the new CLI: the two-minute tour generates with the new flags
and shows the resulting file names; INTEGRATION.md's production/MC examples
show header/source pairs for C and include!-referenced files for Rust;
MODEL_CHECKING.md's workflow compiles fsm.c + mc.c to objects and links (the
shape tasks 3-4 made real). Keep every mermaid block valid.

Rationale: the user set is the primary consumer-facing documentation - it must
describe the CLI as it now behaves, including the file-separation model that
changes how users build generated C.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `docs/user/README.md` | change | Tour + conventions + mermaid. |
| `docs/user/INTEGRATION.md` | change | Examples + codegen walkthroughs. |
| `docs/user/LANGUAGE.md` | change | Examples. |
| `docs/user/MODEL_CHECKING.md` | change | Examples + MC build workflow. |
| `docs/user/PROPERTIES.md` | change | Example. |
| `docs/user/EXAMPLES.md` | change | Examples + walkthroughs. |

### Action 7.3 - Verify the doc set

Run `cargo test --test docs_md_test` (mermaid validity + doc quality gate);
grep-verify zero `cargo run -- <rfs-subcommand>` occurrences across the living
doc set (the remaining `cargo run` hits are all legitimate `-p`/workspace/
cargo-fuzz usages - list them in the task status file as the verified-untouched
set).

Rationale: the docs gate is machine-checked; the grep bar proves goal 1's doc
half is complete.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (verification only - no file changes beyond 7.1/7.2 fixes) | - | docs_md_test + grep bar. |

## Task deliverables
- Zero `cargo run -- <rfs-subcommand>` examples in any living document.
- Codegen walkthroughs describing the new CLI + artifact model.
- Verified-untouched list of legitimate cargo usages (recorded in the status
  file).

## Task exit criteria
- [ ] `grep -rn "cargo run --" KB.md BUILD.md README.md START.md docs/user/`
      returns zero hits.
- [ ] Every codegen walkthrough in the user set uses the new flag family and
      shows the correct artifact file names for its backend.
- [ ] `cargo test --test docs_md_test` green (all mermaid blocks valid).
- [ ] Legitimate cargo usages unchanged (the verified-untouched list is
      accurate).

## Non-goals
- No historical-record rewrites (roadmap phase files, TASKS.md history,
  commands.md).
- No maintainer deep-dive docs (docs/dev/*, docs/llm/* - task 8).
- No CODEGEN.md / backend architecture docs (task 8).
- No source changes.

## Other information

- When rewriting BUILD.md's command tables, keep the table column widths sane -
  the `rfs` prefix is shorter than `cargo run --`, so rows get shorter, not
  longer.
- The docs/user conventions note currently says examples run "via `cargo run
  --`; an installed binary is just `rfs`" - invert it: examples ARE the
  installed-binary form; building from source is a maintainer concern (BUILD.md).
