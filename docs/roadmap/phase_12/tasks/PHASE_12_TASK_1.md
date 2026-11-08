# Phase 12, Task 1 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal
Create the buildable `dev_tools/` workspace-member crate (binary `rfs-dev`) with
the FULL CLI dispatch surface fixed up front - every planned subcommand exists
as a stub returning a pinned "not implemented yet" error - plus the
`dev_tools/README.md` skeleton carrying the two section headers that tasks 3.5
and 4 will fill. Rationale: every later task needs a compiling home, and fixing
the CLI surface now keeps task 5's per-tool subtasks independent of each other.

## Context
- Planning decisions (PHASE_12_PLAN.md Context): WORKSPACE MEMBER at repo root
  `dev_tools/`, crate name `rfs-dev`, binary `rfs-dev`; dependencies exactly
  `syn` (+ its `proc-macro2` layer) and `pulldown-cmark` - a SUBSET of the RFS
  dep set plus these two declared exceptions; nothing else.
- The FIRST build needs network access to crates.io (available in this
  environment per TOOLCHAIN.md section 3); afterwards the registry cache serves
  offline rebuilds. If the fetch fails, fall back to `comrak` (already in the
  lock file as a root dev-dependency) for the markdown side and record a plan
  correction - do not silently add other deps.
- The rfs CLI parses args with plain `std::env::args` (no clap - PRINCIPLES.md
  stack guidance); `rfs-dev` follows the same convention. Exit-code convention
  mirrors the rfs CLI: 0 success, 1 operational failure, 2 usage error.
- The dispatch stub list (the stable CLI surface) is exactly:
  `sync-check`, `sync-fix`, `context`, `edit-rename`, `edit-move`,
  `edit-replace-item`, `doc-section`, `inventory` (subverbs `add`, `update`,
  `remove`, `audit`), `calls` (subverbs `extract`, `classify`). Task 4 may
  refine flag shapes but not subcommand names without a plan correction; task 5
  fills the bodies.
- Stub behavior is pinned: each unimplemented subcommand prints
  `rfs-dev <subcommand>: not implemented yet (Phase 12 task 5)` to stderr and
  exits 2. NO `unimplemented!()`/`todo!()` macros (they are panic paths and
  would trip clippy's `todo` lint policy); the stub returns a typed error.
- `lib.rs` carries only the crate-level doc comment for now - per-tool modules
  are added by task 5, not pre-declared (empty modules would be dead code).

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| - | (none) | first task of the phase; needs only the plan's Context decisions |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 2. | Extract commands.md into a call inventory | writes its script under `dev_tools/analysis/`, which must exist as a crate directory |
| 3. | Classify all tool calls | README section headers (filled in 3.5) must already exist |
| 5. | Implement the proposed tools + tests | implements against this task's dispatch stubs and dep set |

### Affected source files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `dev_tools/Cargo.toml` | new | crate manifest: edition 2021, lib + bin `rfs-dev`, deps syn/pulldown-cmark only |
| `dev_tools/src/main.rs` | new | arg parsing + dispatch stub over the pinned subcommand list |
| `dev_tools/src/lib.rs` | new | crate-level doc comment; no modules yet |

### Affected documentation files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `dev_tools/README.md` | new | title + the two section headers with placeholder text naming the filling tasks (3.5, 4) |

### Other affected files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `Cargo.toml` | change | `[workspace] members` list gains `"dev_tools"` (one entry, alphabetical position per the existing list style) |
| `.gitignore` | change | one line: `dev_tools/analysis/calls.jsonl` (regenerable extractor output - plan Context decision) |

### Affected build/run process
- New: `cargo build -p rfs-dev`, `cargo test -p rfs-dev`, `cargo run -p rfs-dev -- <sub>`; `--workspace` runs now include the crate. Plain root `cargo build`/`cargo test` still select only the `rfs` package (existing workspace note, BUILD.md section 2 - unchanged).

## Planned actions

### Action 1: register the crate in the workspace and write its manifest
Add `"dev_tools"` to the root `[workspace] members` list. Write
`dev_tools/Cargo.toml`: package `rfs-dev` v0.1.0, edition 2021, `[lib]` +
`[[bin]] name = "rfs-dev"`; dependencies `syn = { version = "2", features =
["full"] }` (full parsing needed later for impl/trait items) and
`pulldown-cmark = "0.13"`. Record the exact resolved versions in the status
file after the first build. Rationale: one lock file, `--workspace`
clippy/fmt/test coverage for free; the two deps are the phase's declared
exceptions and nothing else may appear.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `Cargo.toml` | change | members list gains `"dev_tools"` - workspace membership is the planning decision; explicit list style preserved |
| `dev_tools/Cargo.toml` | new | manifest per Action 1 text - pins the dependency constraint mechanically (any later dep addition is a visible diff) |

### Action 2: write the dispatch stub (main.rs + lib.rs)
`lib.rs`: crate-level `//!` doc comment stating the crate's role (maintenance
tooling for the RFS repo; not part of the rfs build/test path). `main.rs`:
parse `std::env::args()` - first arg is the subcommand (plus subverb for
`inventory`/`calls`); unknown subcommand or missing args print a usage line
listing the pinned subcommands to stderr and exit 2; known-but-unimplemented
subcommands print the pinned `rfs-dev <sub>: not implemented yet (Phase 12
task 5)` message to stderr and exit 2. Rationale: the CLI surface is FIXED by
this action - task 4's proposal must match it, task 5 only replaces stub
bodies, so a future session can diff the stub list against the README table as
a consistency check.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `dev_tools/src/lib.rs` | new | doc comment only - no pre-declared modules (dead code until task 5) |
| `dev_tools/src/main.rs` | new | dispatch stub with the pinned subcommand list and pinned error messages/exit codes - the stable CLI contract of the phase |

### Action 3: write the README skeleton and the gitignore line
`dev_tools/README.md`: H1 title (`# dev_tools - RFS maintenance tooling`), a
one-paragraph role statement, then `## Analysis of tool calls patterns` with
placeholder text `(filled by Phase 12 task 3.5)` and `## Proposed tools` with
`(filled by Phase 12 task 4)`. `.gitignore`: append
`dev_tools/analysis/calls.jsonl`. Rationale: the section structure is committed
BEFORE any content lands, so tasks 3.5/4 are pure fills and the freeze of the
analysis section (task 3.5 exit criterion) has a defined target.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `dev_tools/README.md` | new | two section headers + placeholders - fills, not restructures, by later tasks |
| `.gitignore` | change | gitignore the regenerable inventory - plan Context decision (repo bloat vs reproducibility: the SCRIPT is committed, its output is not) |

### Action 4: first build + stub tests + verification
Write `dev_tools/tests/dispatch_test.rs`: for EVERY pinned subcommand (and one
unknown subcommand) run the binary and assert the pinned stderr message and
exit code 2. Then run `cargo build -p rfs-dev && cargo test -p rfs-dev &&
cargo clippy --all-targets && cargo fmt --check` (toolchain activation per
BUILD.md section 2). Rationale: the dispatch test is what makes "CLI surface
fixed" mechanically true - task 4's consistency check and task 5's per-tool
tests both build on it; the first build also warms the registry cache for all
later tasks.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `dev_tools/tests/dispatch_test.rs` | new | pins every stub's message + exit code - the executable form of the CLI contract |

## Task deliverables
- `dev_tools/` workspace-member crate building green with exactly the two
depclared dependencies.
- `rfs-dev` binary with the full pinned subcommand surface as stubs (exit 2,
pinned messages).
- `dev_tools/README.md` skeleton with both section headers.
- `calls.jsonl` gitignored; dispatch test suite passing.

## Task exit criteria
- [ ] `cargo build -p rfs-dev` succeeds; `Cargo.lock` gained only syn/
      proc-macro2/pulldown-cmark (no other new entries).
- [ ] Every pinned subcommand and one unknown subcommand produce the pinned
      stderr message and exit code 2 (dispatch test green).
- [ ] `dev_tools/README.md` carries both section headers with placeholders.
- [ ] `git check-ignore dev_tools/analysis/calls.jsonl` succeeds.
- [ ] `cargo test -p rfs-dev`, `cargo clippy --all-targets`, `cargo fmt
      --check` all green; the rfs package itself is untouched (its test count
      unchanged).

## Non-goals
- No tool logic of any kind (all bodies are stubs until task 5).
- No analysis artifacts, no commands.md reading (task 2).
- No changes to the `rfs` package, its CLI, or its tests.
- No `clap` or other arg-parsing dependency (std::env::args per project stack
  guidance).
