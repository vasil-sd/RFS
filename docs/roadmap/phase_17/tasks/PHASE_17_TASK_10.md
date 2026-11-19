# Phase 17, Task 10 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal
Write `docs/user/USAGE.md` - the exhaustive CLI usage reference (the phase's
central new deliverable) - and wire it into the user documentation index.

## Context
- LAYOUT (BINDING, PHASE_17.md "Proposed USAGE.md layout"): intro + exit-code
  convention; section 1 command map; section 2 external-tool resolution (the
  RFS_* table); sections 3-10 one per command set, each = purpose line ->
  pattern table (`| CLI pattern | Options - detailed description | Generated
  files / output | Possible errors (exit codes) |`) -> "Naming and placement
  rules" + "Error details and fixes" subsections; section 11 the example-
  projects table.
- Section 2 content source: TOOLCHAIN.md section 8.2 (the nine-tool table,
  precedence CLI flag > RFS_* env var > standard Unix var > auto-discovery)
  + the used-by column + special vars (`RFS_BIN` for rfs-build, `RFS_FUZZ_FLAGS`,
  `RFS_MERMAID_PUPPETEER_ARGS`) + the clang-no-standard-tier note + the z3
  discovery chain + `--print-script` as the inspection mechanism. The sample
  filled-in row in PHASE_17.md shows the expected depth.
- Per-command content sources (read before writing each section - do not trust
  memory): the command's own usage text (`cargo run -q -- <cmd>` with no args
  prints it), `src/cli/args.rs::usage()` (post-task 4), BUILD.md section 4
  (the command table with full flag lists), and for codegen the Phase 14
  artifact model in `src/codegen/CODEGEN.md` + PHASE_14.md's "Proposed CLI"
  contract table (take-or-emit semantics, BASE derivation, partial-pair usage
  error, stale-reuse warning).
- The mc-ref section (9) reuses the REAL CE traces captured by task 6 (deadlock
  lasso + property violation, text and JSON) - field-by-field walkthrough.
- Section 11 lists ALL 15 example projects (5 existing + 10 new from tasks
  6-8): `| title | description + rationale | ref to README.md |` with relative
  links `../../crates/examples/<name>/README.md`.
- Machine-checked by `tests/docs_md_test.rs`: mermaid must parse (use the
  generated-subset shapes), every link must resolve. The file joins the
  docs/user/ set automatically (the gate collects every repo .md).
- Depends on: task 1 (MODEL_CHECKING.md consistency - USAGE.md links to it for
  mode semantics and does not re-derive them), tasks 6-9 (real traces + real
  example READMEs to reference).

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 1. | Fix stale user docs | USAGE.md must not contradict the rewritten MODEL_CHECKING.md |
| 6.-9. | Example projects (all) | section 11 references their READMEs; the mc-ref section reuses task 6's traces |

### Affected documentation files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `docs/user/USAGE.md` | new | the full CLI usage reference (sections 1-11) |
| `docs/user/README.md` | change | USAGE.md row in "Which doc for which question?" + toolchain-at-a-glance mention |

## Planned actions

### Write sections 1-2 (command map + tool resolution)
Intro (2-3 sentences: what this file is, how it complements MODEL_CHECKING.md /
INTEGRATION.md / EXAMPLES.md; the exit-code convention - 0/1 for CLI commands,
0/1/2 status taxonomy for MC binaries). Section 1: the command map table
(eight rows, section numbers). Section 2: the precedence model stated ONCE, the
nine-tool RFS_* table with the used-by column, the special-vars paragraph,
the clang + z3 notes, and a `--print-script` inspection example (a real captured
`# tools:` line - reuse task 8.4's capture).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `docs/user/USAGE.md` | new (sections 1-2) | orientation + the tool contract users must know before any command section |

### Write sections 3-5 (parse, validate, info + info format)
Per the layout: purpose line, pattern table (every meaningful invocation shape
- file vs dir input, --target-dir, --json, info format --list/<name>/--output
md|plantuml|svg|schema), naming/placement rules (extension replacement never
stacked, overwrite refusal, err-file placement), error details and fixes
(parse errors with line/col, semantic errors, the validate-json document shape
- link PROPERTIES.md/MODEL_CHECKING.md where semantics belong).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `docs/user/USAGE.md` | new (sections 3-5) | the three read-only command sets |

### Write section 6 (codegen rust/c) - the take-or-emit centerpiece
The pattern table covers: production/test/mc x rust/c, `--out BASE` vs no-out
(spec-stem default in CWD), every per-artifact flag (`--out-fsm-code`,
`--out-fsm-header`, `--out-mc-code/-header`, `--out-test-code`,
`--out-fuzz-target-code`), the fsm-reference flags (`--fsm-file NAME[.rs]` /
`--fsm-file STEM` / `--fsm-file-header H --fsm-file-source C`) with TAKEN-if-
present / EMITTED-if-missing semantics stated in the row AND in a dedicated
"Artifact naming and take-or-emit rules" subsection (BASE derivation, per-
artifact override, partial-pair usage error, stale-reuse warning, `--out -`
stdout mode rust-production-only), generation options (`--assumes`, `--steps`,
`--no-std`, `--with-property-monitors`, the mc-mode family,
`--with-protocol-fsm`), and the compile recipes (rustc --edition 2021 / gcc
-Wall -c + link). Error details: usage errors, parse/validation, stale-reuse.
Source of truth: PHASE_14.md's contract table + `src/codegen/CODEGEN.md` -
verify every claim by running the command.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `docs/user/USAGE.md` | new (section 6) | the artifact semantics are the nuance the phase request called out - they must be explicit, not implied |

### Write sections 7-10 (test, diagram, mc-ref, gen-spec)
test: unit/fuzz suites per backend, every tool flag, missing-tool diagnostics,
`--print-script`. diagram: five formats, internal/external renderers, the
puppeteer gotcha pointer (TOOLCHAIN.md 6.4). mc-ref: ALL THREE modes with per-
mode flag tables + exit codes (consistent with task 1's MODEL_CHECKING.md),
then the CE-trace walkthroughs using task 6's real captures (deadlock lasso
field by field; property-violation trace field by field; the JSON document
shape). gen-spec: random vs mutation modes, seed determinism, always-valid
output guarantee.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `docs/user/USAGE.md` | new (sections 7-10) | the remaining command sets + the CE walkthroughs that make failure output readable |

### Write section 11 (example projects) + wire the index
The 15-row table: `| title | description + rationale | ref to README.md |`
with relative links to every `crates/examples/<name>/README.md` (5 existing +
10 new). Then `docs/user/README.md`: add the USAGE.md row to "Which doc for
which question?" ("Run any CLI command with confidence: every pattern, option,
generated artifact and error - USAGE.md") and a mention in the toolchain-at-
a-glance diagram where it fits.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `docs/user/USAGE.md` | new (section 11) | the examples table is the bridge from CLI patterns to runnable projects |
| `docs/user/README.md` | change (~line 20 table) | users must be able to FIND USAGE.md from the index |

## Task deliverables
- `docs/user/USAGE.md` complete (sections 1-11).
- `docs/user/README.md` referencing it.

## Task exit criteria
- [ ] Every command set has a pattern table with all four columns filled for
      every meaningful invocation shape (spot-check at least one row per
      section by RUNNING the command and comparing).
- [ ] The codegen section states take-or-emit explicitly in BOTH the table rows
      and the naming-rules subsection (BASE derivation, taken-vs-emitted,
      partial-pair error, stale-reuse warning).
- [ ] Section 2's RFS_* table matches TOOLCHAIN.md section 8.2 tool for tool.
- [ ] The mc-ref CE walkthroughs use task 6's real captured traces.
- [ ] Section 11 links all 15 example READMEs and every link resolves.
- [ ] `cargo test --test docs_md_test` green (mermaid + cross-links).
- [ ] `cargo fmt --check` clean.

## Non-goals
- No re-documentation of mode SEMANTICS (that is MODEL_CHECKING.md's job -
  link, don't duplicate).
- No source changes.
