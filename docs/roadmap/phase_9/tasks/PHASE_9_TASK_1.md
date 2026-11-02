# Phase 9, Task 1 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start implementation of the task immediately
> without re-deriving scope.

## Goal
Replace the root `README.md` - currently an outdated copy of
`docs/user/README.md` - with a proper repository entry point: short project
intro with the main principles, a high-level RFS pipeline overview, and clear
routing sections for users (-> `docs/user/README.md`) and developers (->
`docs/dev/README.md`). Rationale: the root README is the first thing any
visitor sees; it must orient readers to the right documentation set instead of
duplicating the user docs.

## Context
- The current root `README.md` (4788 B) differs from `docs/user/README.md`
  (5156 B): it carries repo-root-relative links (`docs/user/LANGUAGE.md`),
  lacks the Phase 7 updates (in-process SVG rendering note, the `info format`
  mode in the command list) and the `docs/dev/` + `docs/llm/` rows. It is an
  outdated copy, not a live document.
- Nothing in the repo links to the root README: the only
  `[README.md](README.md)` hit is `docs/user/LANGUAGE.md:4`, which links its
  OWN directory's file. Deletion breaks no link (re-verify with grep at
  execution time).
- The new README is parsed by `tests/docs_md_test.rs` (part of plain
  `cargo test`): comrak GFM parse, every relative link target must resolve
  from the repo root, and EVERY ```mermaid block must render through
  mermaid-rs-renderer's full pipeline. Model the pipeline diagram on the one
  in `docs/user/README.md` section "The toolchain at a glance" - that exact
  shape already passes the gate. If a diagram variant fails the renderer,
  simplify toward that shape rather than fighting the renderer subset.
- Content spec (from the phase request): (a) short intro about the project,
  its purpose and main principles - one IR for all targets, parity of
  backends, cross-checking of MC; (b) high-level overview of the RFS pipeline
  (spec -> IR -> backends/MC/etc); (c) section for users with a link to
  `docs/user/README.md`; (d) section for developers with a link to
  `docs/dev/README.md`.
- The main principles are stated canonically in
  `docs/roadmap/PRINCIPLES.md` section 1 (one constrained specification -> one
  versioned IR -> multiple targets; backend parity with cross-backend
  differential tests) and the MC cross-checking layer is described in
  `BUILD.md` section 5 (independent reference MC, differential suites,
  two-layer toolchain fuzzing). Phrase the README intro from these; do not
  invent new claims.

## Scope

### Affected documentation files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `README.md` | remove | delete the outdated copy of docs/user/README.md |
| `README.md` | new | write the new root README per the content spec above |

### Other affected files

None. No source, test, example, or training files are touched.

## Planned actions

### Action 1: Delete the outdated root README
Remove `README.md` from the repo root. Rationale: it is a stale duplicate;
the new file (action 2) replaces its role entirely.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `README.md` | remove | outdated copy of docs/user/README.md; nothing links to it (verified at planning: only docs/user/LANGUAGE.md:4 matches `[README.md](README.md)` and that is a same-directory link) |

### Action 2: Write the new root README
Create `README.md` with, in order:
1. **Title + short intro** (a few sentences): RFS is a finite-state protocol
   specification language; you describe one endpoint of a protocol as a finite
   input/output machine and the toolchain compiles that ONE description to
   production FSMs (Rust AND C), generated test suites, model-checking
   binaries, diagrams, and static introspection. Then the main principles,
   each in one sentence: **one constrained specification -> one versioned IR
   -> multiple targets** (the IR is the single source of truth for every
   generator); **backend parity** (Rust and C backends provide the same
   commands, deliverables, and observable behavior, verified by cross-backend
   differential tests); **MC cross-checking** (generated model checkers are
   soundness-checked against an independent in-process reference MC,
   hand-written monitor references, an independent PSL evaluator, and
   two-layer toolchain fuzzing - see `BUILD.md` section 5 for the suite list).
2. **"The RFS pipeline" section**: a ```mermaid flowchart modeled on
   `docs/user/README.md`'s "The toolchain at a glance" diagram (Authoring /
   Compilation / Verification / Ecosystem subgraphs: `.rfs` spec ->
   `rfs validate` / `rfs info` / `rfs diagram` / `rfs gen-spec` -> versioned
   IR -> `codegen rust|c production/test/mc` -> `rfs test`, Rust/C MC binaries
   with byte-parity, `rfs mc-ref` in-process reference MC without compilation,
   production apps via rfs-build + ProtocolFsm adapters or C Makefiles). A
   short paragraph under the diagram: spec -> parse/validate -> one versioned
   IR -> every target consumes that same IR, which is what makes parity and
   cross-checking possible.
3. **"For users" section**: one or two sentences + a link
   `[docs/user/README.md](docs/user/README.md)` - the self-contained user
   documentation set (language, properties, model checking, integration,
   examples, FAQ, migration, normative semantics).
4. **"For developers" section**: one or two sentences + a link
   `[docs/dev/README.md](docs/dev/README.md)` - the Class A developer deep-
   dive set with stable fragment anchors; plus one line pointing LLM-assisted
   sessions to `docs/llm/` (TASK_MAP / CHANGE_IMPACT / budgeted per-module
   cards) and to `START.md` as the session entry point.
Keep the whole file short (~60-90 lines). No images other than the mermaid
block. Every relative link must resolve from the repo root.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `README.md` | new | repository entry point per the phase content spec (a)-(d); mermaid pipeline diagram modeled on the gate-passing docs/user/README.md shape so tests/docs_md_test.rs stays green |

### Action 3: Verify (subtask 1.1)
Run `cargo test --test docs_md_test` (must pass: parse, link resolution,
mermaid render of the new diagram). Grep the repo for any reference to the old
root README content that would now dangle (expected: none outside TASKS.md
history, which is append-only and never edited). Rationale: the doc-quality
gate is machine-checked; a broken mermaid block or dead link fails plain
`cargo test`.

#### Affected files

None (test run + grep only).

## Task deliverables
- New `README.md` at the repo root with the four required parts (intro +
  principles, pipeline overview with mermaid diagram, users section ->
  docs/user/README.md, developers section -> docs/dev/README.md).
- Old outdated copy deleted.
- `cargo test --test docs_md_test` green.

## Task exit criteria
- [ ] `README.md` no longer matches the old copy; it contains: (a) short intro
      with the three main principles (one IR for all targets, backend parity,
      MC cross-checking), (b) a pipeline overview section with a rendering
      mermaid flowchart, (c) a users section linking `docs/user/README.md`,
      (d) a developers section linking `docs/dev/README.md`.
- [ ] No file outside `TASKS.md` references the removed README content in a
      way that dangles (grep-verified).
- [ ] `cargo test --test docs_md_test` passes (5 pass).

## Non-goals
- No changes to `docs/user/README.md` or any other documentation file.
- No source/test/training changes.
- No new sections beyond the four specified (no contribution guide, no badge
  clutter, no duplication of user-doc content).

## Task corrections

| Correction | Description + rationale |
|------------|-------------------------|
| Planning docs must pass the repo-wide doc gate | Action 3's `cargo test --test docs_md_test` run FAILED on first execution: 6 of 255 docs failed - all in `docs/roadmap/phase_9/` (PHASE_9_PLAN.md + task files 2, 3, 4, 6, 7) because their BARE illustrative markdown link examples (e.g. `[DESIGN.md](../DESIGN.md)` shown to specify the conversion style) are parsed by comrak as real relative links that must resolve from docs/roadmap/phase_9/. Fixed by wrapping every bare illustrative link in a code span (backticks) - inert to link parsing, example stays readable. The new README itself passed on first render. Recorded here because the fix touched planning-stage documents outside this task's planned file list; see the plan's corrections table. |

## Other information
- Toolchain activation for the test run: the standard block from START.md
  section 4 (RUSTUP_HOME/CARGO_HOME/PATH exports + cd into the repo root).
