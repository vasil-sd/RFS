# Phase 6, Task 7 - Implementation details

> Written for a future LLM session: after reading `START.md`,
> `docs/roadmap/INTRO.md` and this file, the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal
Write the external-user documentation set (`docs/user/`): a self-contained doc
collection for people who have never seen this repository - language reference,
property reference, MC modes reference, production integration guide, examples
walkthrough, FAQ on abstraction and assumptions, plus an index. This is
PHASE_6.md's "Documentation" deliverable and the exit criterion "documentation
is sufficient for external users".

## Context

- **Sources** (derive from these; do not invent): `docs/SEMANTICS.md` (task 1 -
  normative semantics), `KB.md` section 3 (RFS v0 syntax reference) + section 7
  (known limitations), `DESIGN.md` sections 5-6 (language subset + EBNF grammar),
  `BUILD.md` (commands/flags as shipped after tasks 2-6), `docs/user/MIGRATION.md`
  (task 2.4), the example specs in `examples/valid/` (incl. new 47/48/49 from
  task 6 and `tla_adapted/`), `crates/examples/*` READMEs (integration patterns).
- **Style rules**: no internal jargon without a one-line definition; every
  command example must be copy-paste runnable in this repository (verify a
  representative sample by actually running it); cross-link between the user
  docs, SEMANTICS.md, and examples; short pages with a table of contents at the
  top; no images (project rule).
- **Execution order**: this task runs AFTER tasks 1, 2 and 6 - the docs describe
  the FINAL v1.0 state (8-command CLI, version headers, new benchmark specs,
  extended MC report). If a referenced feature is missing, that is a bug in an
  earlier task - record it, do not document fiction.
- **Section outlines** (per file; adjust headings as needed, keep the coverage):
  - `LANGUAGE.md`: quick start (write -> validate -> generate a spec); module +
    version header (v1.0 format, legacy no-header = v0.x); states/enums +
    initial; events/outputs with finite payloads; variables (`bool`/enum/
    `bit<N>`, C-backend width limit 8/16/32/64); transitions: guards, explicit
    priorities, simultaneous updates, `on_error`; `Tick` + timer variables;
    expression language (operators, wrap semantics); a full minimal example with
    line-by-line walkthrough; pointer to SEMANTICS.md for formal semantics.
  - `PROPERTIES.md`: atoms (`state_is`, `input_is`, `output_has`, expr atoms);
    operators with bounds (`next`, `always`, `always[N]`,
    `bounded_eventually[N]`, `eventually`/`until`) each with a one-line meaning
    + a tiny example; sampling semantics (sample 0 = initial configuration);
    assumptions + handler policies (continue/stop/panic); monitor subset + skip
    reasons (what `rfs info` reports as monitored vs skipped); strong vs weak
    liveness; fairness. Worked examples: specs 13/15/16/42.
  - `MODEL_CHECKING.md`: the three modes (exact = default, bounded/fuzz, SMT
    feature-gated) with when-to-use guidance; FULL flag tables for the generated
    MC binary (exact: `--max-states`, `--nondet`, `--timeout-sec`, `--threads`,
    `--fast`, `--counterexample-format json|text`, `--assumes`; fuzz: `--mode
    fuzz`, `--bound`, `--runs`, `--seed`, `--violate label:prob:percent%`,
    `--unexpected-prob`) and for `rfs mc-ref` (task 3's unified flags + exit
    codes 0/1/2); status taxonomy + LABELING GUARANTEES (when a PASS is
    trustworthy - the no-false-PASS invariant; what UnknownStateLimit means);
    counterexample formats: text report anatomy + JSON document example (from an
    actual run); forced violations explained; how to read a report step by step;
    when MC results transfer to the real system (abstraction caveats -> FAQ).
  - `INTEGRATION.md`: Rust via build.rs with rfs-build (API options table:
    `.backend()`, `.production()`, `.test_command()`, `.with_property_monitors()`,
    `.no_std()`, `.with_protocol_fsm()`; RFS_BIN); `ProtocolFsm` + the four
    adapters (sync channel / async futures / callback / no_std-minimal) with a
    minimal code sample; `--no-std` production codegen; C projects (Makefile
    pattern from `c_sync_handler`); running generated tests in CI (`rfs test`,
    both suites); external-tool parameterization (`RFS_*` env vars, CLI override,
    `--print-script`); verifying checked-in artifacts (spec hash + toolchain
    version in headers); the `rfs gen-spec` rapid-prototyping workflow (random +
    base-mutation modes) and `rfs mc-ref` for shell/REPL prototyping.
  - `EXAMPLES.md`: curated walkthroughs: 01 trivial two-state; 03 timeout/retry;
    04 TCP-handshake-like; 43 sliding window (deep dive - the protocol behind
    the example crates); 44 token ring; 45 elevator; 47/48/49 benchmark specs
    (what each abstracts + its properties); note on `tla_adapted/`.
  - `FAQ.md`: abstraction and assumptions Q&A: what representative inputs are
    and what they soundness-ify; assumption policies and their effect on MC;
    nondeterministic MC vs deterministic production; deadlock semantics; when is
    a PASS trustworthy (labeling guarantees); C-backend bit-width limits; version
    headers + migration (pointer to MIGRATION.md); known limitations (from KB.md
    section 7 - spans in semantic errors, anonymous payloads, ambiguity warning,
    error recovery).
  - `README.md`: index + a "which doc for which question" table + links to
    SEMANTICS.md and examples/.

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 1. | Formal semantics document | PROPERTIES.md / MODEL_CHECKING.md derive their semantics from it |
| 2. | Stability and versioning (v1.0) | docs describe the v1.0 version header, spec hash headers, MIGRATION.md |
| 6. | Benchmark suite | EXAMPLES.md walks through the new benchmark specs; MC report fields are documented |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 8. | Training data update | training entries quote these docs/commands in their final form |
| 9. | Docs sync | START.md doc map gets the `docs/user/` row; stale sweep verifies consistency |

### Affected documentation files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `docs/user/README.md` | new | index + question-routing table |
| `docs/user/LANGUAGE.md` | new | language reference |
| `docs/user/PROPERTIES.md` | new | property language reference |
| `docs/user/MODEL_CHECKING.md` | new | MC modes reference (incl. `rfs mc-ref`) |
| `docs/user/INTEGRATION.md` | new | production integration guide (incl. `rfs gen-spec` workflow) |
| `docs/user/EXAMPLES.md` | new | curated examples walkthrough |
| `docs/user/FAQ.md` | new | FAQ on abstraction and assumptions + known limitations |

## Planned actions

### Action 1: language reference (subtask 7.1)
Write `docs/user/LANGUAGE.md` per the outline. Verify every syntax example by
parsing it (`cargo run -- validate <file>` in a temp dir) before committing it
to the doc.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `docs/user/LANGUAGE.md` | new | the reference |

### Action 2: property reference (subtask 7.2)
Write `docs/user/PROPERTIES.md` per the outline, semantics quoted from
SEMANTICS.md sections 4-8; worked examples from specs 13/15/16/42.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `docs/user/PROPERTIES.md` | new | the reference |

### Action 3: MC modes reference (subtask 7.3)
Write `docs/user/MODEL_CHECKING.md` per the outline; capture REAL report outputs
(text + JSON) from actual runs on a pinned spec and embed them as examples;
document `rfs mc-ref` (flags, exit codes, when to use it vs a generated MC).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `docs/user/MODEL_CHECKING.md` | new | the reference |

### Action 4: integration guide (subtask 7.4)
Write `docs/user/INTEGRATION.md` per the outline; code samples adapted from
`crates/examples/*` (verify they compile/run as written where practical);
include the `rfs gen-spec` prototyping workflow + `rfs mc-ref` shell usage.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `docs/user/INTEGRATION.md` | new | the guide |

### Action 5: examples walkthrough + FAQ (subtask 7.5)
Write `docs/user/EXAMPLES.md` (walkthroughs per outline, incl. 47/48/49) and
`docs/user/FAQ.md` (Q&A per outline; limitations quoted from KB.md section 7 as
shipped after task 9's updates are NOT yet available - use the current state and
task 9 re-verifies).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `docs/user/EXAMPLES.md` | new | walkthroughs |
| `docs/user/FAQ.md` | new | FAQ |

### Action 6: index (subtask 7.6)
Write `docs/user/README.md`: index + question-routing table + links to
SEMANTICS.md, examples/, ide/README.md.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `docs/user/README.md` | new | the index |

## Task deliverables
- Seven files under `docs/user/`: README, LANGUAGE, PROPERTIES,
  MODEL_CHECKING, INTEGRATION, EXAMPLES, FAQ - each self-contained, cross-
  linked, with runnable examples.

## Task exit criteria
- [ ] All seven files exist with the outline coverage (every topic in Context's per-file outlines present).
- [ ] A representative sample of command/syntax examples verified runnable (record which were run in "Other information").
- [ ] Cross-links resolve (no dead relative links within docs/user/ + to SEMANTICS.md/examples/ide).
- [ ] No stale claims: version header format, CLI surface (8 commands), report fields, and benchmark specs match the shipped state after tasks 2/3/6.
- [ ] `cargo test` unaffected (docs-only) - sanity run green.

## Non-goals
- Not the formal semantics document (that is SEMANTICS.md, task 1); no LSP/IDE documentation beyond a pointer to `ide/README.md`; no translations; no changes to any source file or example.

## Other information
- Record here after action 5: which examples were actually executed for
  verification.
- Keep pages focused: if a doc grows past ~400 lines, split by topic rather than
  padding - the set must stay navigable.
