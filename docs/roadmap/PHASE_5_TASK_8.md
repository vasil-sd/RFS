# Phase 5, Task 8 - Implementation details

> Written for a future LLM session: after reading `START.md`,
> `docs/roadmap/INTRO.md` and this file, the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal
Add the `rfs diagram` command: Graphviz DOT state diagrams (pure Rust, no
dependency), transition tables and variable/event summaries (text or Markdown),
SVG/PNG via the external `dot` binary with graceful fallback - plus
external-tool parameterization (`RFS_DOT`/`--dot`/`--dot-args`) and
`--print-script` mode per the user requirement.

## Context

- DOT rendering is PURE RUST (no system dependency): one node per state
  (label = state name; initial state marked), one edge per transition
  (label = `Event[pattern] [guard] -> next`; on_error fallback edges dashed).
  Deterministic node/edge order (declaration order) - output is unit-pinned.
- Transition table: rows = states, columns = event groups (unit + variants);
  cells = guard(s)/action summary or `-`. Variable/event summary: typed lists
  (reuses `rfs info`'s IR walking where sensible). Markdown rendering for both.
- SVG/PNG: invoke the resolved `dot` binary (`RFS_DOT` env / `--dot PATH` /
  `--dot-args "..."` per task 3's resolver; default bare `dot` on PATH) with
  `-Tsvg`/`-Tpng`. If `dot` is absent: warn on stderr and fall back to DOT
  output (documented optional system dependency). Check whether `dot` exists in
  this environment BEFORE implementing the runtime tests - if absent, gate those
  tests on availability.
- Options: `--format dot|md|svg|png` (default dot), `--out FILE` (default
  stdout), `--log-level`, `--print-script`. Exit 0 on success; 1 on
  parse/validation error or (for svg/png) missing `dot` WITH no fallback
  requested? - NO: the fallback always applies, so missing dot only warns.

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 3. | `rfs info` command + shared CLI infrastructure | ext_tool resolver (dot), logging, --print-script emitter |

### Dependent tasks

(none directly; tasks 14/15 document and train it)

### Affected source files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `src/main.rs` | change | dispatch for the new `diagram` subcommand + usage text |
| `src/diagram.rs` | new | DOT renderer, table/summary renderers (text+md), dot invocation, script mode |

### Affected test files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `tests/frontend_test.rs` | change | DOT output pinned for a small example; all 45 valid examples render without error; md table sanity; --print-script pinning; stub-`dot` parameterization proof; fallback warning when dot absent (env-controlled) |

## Planned actions

### Action 1: DOT renderer + unit pinning
Pure-Rust DOT generation per Context with deterministic ordering; pin the full
DOT output of a small hand-written spec in a unit test.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/diagram.rs` | new | DOT renderer + pinned output test |

### Action 2: transition table + summaries (text & md)
Render the state x event matrix and typed variable/event summaries; markdown
variant shares the data layer. Sanity-test on examples with guards, on_error,
and multi-variant events.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/diagram.rs` | change | table/summary renderers |

### Action 3: svg/png via resolved dot + fallback + --print-script
Invoke the resolved `dot` (resolver from task 3) capturing stderr for
diagnostics; missing binary -> warn + DOT fallback. `--print-script` emits the
dot invocation (or a no-external-tool script for dot/md formats).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/diagram.rs` | change | invocation + fallback + script mode |

### Action 4: command wiring + tests
Dispatch, options, usage text; frontend tests per Scope (pin DOT for the small
spec; sweep all 45 examples; stub-dot argv proof; fallback warning).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/main.rs` | change | dispatch + usage text |
| `tests/frontend_test.rs` | change | the tests above |

## Task deliverables
- `rfs diagram` command: DOT (pure Rust), md/text tables+summaries, svg/png via dot with fallback.
- Parameterization (`RFS_DOT`/`--dot`/`--dot-args`) + `--print-script`.
- Pinned DOT output + 45-example sweep test.

## Task exit criteria
- [ ] DOT output for a pinned spec is byte-stable (unit test); all 45 valid examples render without error.
- [ ] Transition table correctly shows guards/actions/on_error for a pinned multi-variant example.
- [ ] svg/png path proven with a stub `dot` (argv + args reach the process, incl. `--dot-args`); missing dot -> warning + DOT fallback (test).
- [ ] `--print-script` output byte-pinned; full suite green, clippy/fmt clean.

## Non-goals
- Interactive/animated diagrams, layout engines in Rust (v1 uses graphviz for
  raster/vector only).
- Diagrams of the state space INCLUDING monitor states (MC internals stay out
  of user-facing diagrams).
- PNG optimization/compression control (graphviz defaults).

## Other information
- Check `which dot` in this environment first; if absent, install nothing -
  implement + test via stubs and gate real-binary tests on availability.
