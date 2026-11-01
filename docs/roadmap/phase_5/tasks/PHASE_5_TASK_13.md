# Phase 5, Task 13 - Implementation details

> Written for a future LLM session: after reading `START.md`,
> `docs/roadmap/INTRO.md` and this file, the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal
Deliver the phase's initial IDE support: a TextMate grammar + snippets for
`.rfs` shipped in-repo under `ide/`, and structured CLI diagnostics
(`rfs validate --json`) so editors can ingest parse/semantic errors directly.
Full LSP stays a non-goal.

## Context

- Grammar scope: RFS v0 keywords (module/enum/event/output/var/transition/
  initial/next/emit/set/where/priority/on_error/version), temporal operators
  incl. Phase 4's `eventually`/`until` + `always[N]`/`bounded_eventually[N]`,
  boolean/comparison/arithmetic operators, comments (`//` and `#`), string
  literals (version header), numbers, identifiers, paths (`::`). Snippets for
  the common skeletons (module, transition, prop with always/eventually).
- Placement: `ide/textmate/rfs.json` (VS Code-compatible TextMate grammar) +
  `ide/snippets.rfs` + `ide/README.md` (install notes for VS Code via a local
  extension or one-click repo, and Sublime). A small sanity test asserts the
  grammar file is valid JSON and that every RFS keyword from the parser's
  keyword list appears in the grammar (prevents drift when the language grows).
- `validate --json`: reuses task 3's JSON plumbing, rendered by task 2's json_rw writer. Document shape (pinned):
  ```json
  { "file": "<path>", "ok": false,
    "errors": [ { "kind": "parse|semantic", "line": L, "col": C, "message": "..." } ] }
  ```
  (line/col from the existing ParseError span + line_col helper; ok=true with
  empty errors on success). Exit codes unchanged (0/1). Directory mode: one
  document per failing file in an array. This is what editor integrations
  consume - keep it stable and pinned by tests over examples/invalid.

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 3. | `rfs info` command + shared CLI infrastructure | the JSON plumbing + error-shape conventions this task reuses |

### Dependent tasks

(none directly; tasks 14/15 document and train it)

### Other affected files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `ide/textmate/rfs.json` | new | TextMate grammar for .rfs |
| `ide/snippets.rfs` | new | snippet set (module/transition/prop skeletons) |
| `ide/README.md` | new | install + usage notes (VS Code, Sublime) |

### Affected source files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `src/frontend.rs` | change | `validate --json` option + document rendering |

### Affected test files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `tests/frontend_test.rs` | change | validate --json over examples/invalid (shape pins, line/col correctness for parse + semantic errors), ok=true on a valid file; grammar sanity test (valid JSON + keyword coverage) |

## Planned actions

### Action 1: TextMate grammar + snippets + README
Author the grammar covering Context's scope; snippets; README with install
notes. Sanity test: valid JSON + every parser keyword present in patterns.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `ide/textmate/rfs.json` | new | the grammar |
| `ide/snippets.rfs` | new | snippets |
| `ide/README.md` | new | docs |

### Action 2: validate --json
Add the flag to the validate command; render the pinned document shape from
existing ParseError/validator diagnostics (no diagnostic logic changes - pure
rendering). Directory mode emits an array.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/frontend.rs` | change | flag + rendering |

### Action 3: tests
JSON shape pins over a parse-error example and a semantic-error example from
examples/invalid (line/col asserted against the text diagnostics); ok=true on
examples/valid; grammar sanity test.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `tests/frontend_test.rs` | change | the tests above |

## Task deliverables
- `ide/`: TextMate grammar, snippets, README (VS Code + Sublime notes).
- `rfs validate --json` with pinned document shape (file + directory modes).
- Tests for both.

## Task exit criteria
- [ ] Grammar file is valid JSON and covers every parser keyword (sanity test green); snippet set present.
- [ ] `validate --json` output parses; shape + line/col pinned for at least one parse-error and one semantic-error example; directory mode emits the array form.
- [ ] Text diagnostics and exit codes unchanged (existing pins still pass verbatim).
- [ ] Full main suite green, clippy/fmt clean.

## Non-goals
- Full LSP (go-to-definition, references, inline errors, completion) - explicit
  phase non-goal ("later").
- Hover information from the IR (PHASE_5.md: "maybe later").
- A published VS Code extension (in-repo grammar + install notes only).

## Other information
- When adding keywords to the language in a future phase, the grammar sanity
  test will force the grammar update - keep the keyword list source shared with
  the parser if feasible.
