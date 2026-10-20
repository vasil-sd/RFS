# Phase 5, Task 15 - Implementation details

> Written for a future LLM session: after reading `START.md`,
> `docs/roadmap/INTRO.md` and this file, the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal
Sync all design-relevant documentation to the FINAL Phase 5 state: new CLI
surface + option table, workspace layout, external-tool parameterization +
--print-script semantics, rfs-build/adapters/no_std/diagram/JSON decisions -
including a NEW DESIGN.md section 23 "Phase 5 toolchain decisions" recording
D-P5-1..N as shipped.

## Context

- Docs per the plan: `DESIGN.md` (sections 5/8/9/10 updates + NEW section 23),
  `KB.md`, `BUILD.md`, `CODEGEN.md`, `src/backends/RUST.md`,
  `src/backends/C.md`, `START.md` (section 1 snapshot).
- Ground truth = the implemented behavior after tasks 1-13 + this task's own
  verification runs. Read targeted chunks only (grep headers first) - do not
  load whole large docs.
- Decisions to record as D-P5-x (as shipped, with any plan corrections):
  workspace layout; the hand-rolled JSON reader-writer (full rationale per task 2's module doc: key-ordering guarantee + always byte-identical output, no external crates); rfs-build API + RFS_BIN; info/test/diagram command
  semantics + exit codes; external-tool parameterization (env-var-preferred,
  precedence flag > env > PATH, the RFS_* variable names); --print-script
  format; no_std scope (production-only, core+alloc, identical API shape);
  counterexample JSON document shape + generated-code hand-rolled writer
  constraint; ProtocolFsm approach (optional impl emission, generic adapters,
  Rust-ecosystem-only per parity principle); diagram external-dot dependency +
  fallback; IDE grammar placement.
- Stale-content sweep checklist: "three commands" CLI descriptions (now six),
  single-package layout references, generated-test counts (grown in tasks 5/6),
  any pre-Phase-5 option tables missing the new flags, "later" markers that
  Phase 5 resolved (no_std, adapters).

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 1.-14. | All implementation + training tasks | docs describe the FINAL state incl. final counts |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 16. | Close-out | verifies everything, writes PHASE_5_STATUS.md |

### Affected documentation files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `DESIGN.md` | change | sections 5 (language unchanged - verify), 8/9/10 (CLI + codegen decisions updated for new commands/options), NEW section 23 D-P5-1..N as shipped |
| `KB.md` | change | state table (Phase 5 row, counts), CLI behavior section (six commands + options incl. RFS_* env vars), source map (workspace, crates/, ide/), test-generation description |
| `BUILD.md` | change | workspace build notes, new command examples (info/test/diagram with flags), final suite counts, external-tool env var table, dot/clang availability notes |
| `CODEGEN.md` | change | new codegen flags (--no-std, --with-protocol-fsm, --counterexample-format), test-command rendering description, fuzz skeleton emission |
| `src/backends/RUST.md` | change | no_std production rendering, extended test rendering, fuzz skeleton, optional ProtocolFsm impl emission |
| `src/backends/C.md` | change | extended test rendering mirror, libFuzzer skeleton + clang recipe, --no-std no-op note, JSON renderer |
| `START.md` | change | section 1 snapshot (Phase 5 complete at implementation level + final baseline numbers); source tree update (crates/, ide/) |

## Planned actions

### Action 1: read the final state
Collect ground truth: current flag surface (`src/codegen.rs`, command modules),
final test counts from this session's runs, crate layout. Check task status
files for plan corrections to record in section 23.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none - reading only) | - | ground-truth collection |

### Action 2: DESIGN.md
Update sections 8/9/10 for the new CLI/codegen surface; write NEW section 23
"Phase 5 toolchain decisions (as shipped)" with D-P5-1..N + plan/task
corrections (pointers, not silent overwrites).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `DESIGN.md` | change | sections 8/9/10 + new section 23 |

### Action 3: KB.md + BUILD.md + CODEGEN.md
Per the Scope table: state table, CLI behavior (six commands, RFS_* env vars,
--print-script), source map, final counts; workspace + command examples;
codegen flag surface.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `KB.md`, `BUILD.md`, `CODEGEN.md` | change | per Scope table |

### Action 4: backend docs + START.md
RUST.md/C.md per Scope; START.md section 1 snapshot (Phase 5 complete at
implementation level, final baseline - task 15 confirms) + source tree.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/backends/RUST.md`, `src/backends/C.md`, `START.md` | change | per Scope table |

### Action 5: stale sweep
Grep all docs for the Context checklist items; fix any remainder. Cross-check
that every section number cited exists (references must resolve).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (any doc with stale claims) | change | sweep fixes |

## Task deliverables
- All listed docs consistent with the shipped Phase 5 behavior; DESIGN section
  23 records D-P5-1..N as shipped.

## Task exit criteria
- [ ] DESIGN.md sections 8/9/10 current + new section 23 present (D-P5-1..N as shipped, incl. corrections).
- [ ] KB.md, BUILD.md, CODEGEN.md, RUST.md, C.md, START.md updated per the tables above.
- [ ] Stale sweep: no doc describes the CLI as three-command, the repo as a single package, or pre-Phase-5 counts (unless describing history explicitly).
- [ ] Cross-references between docs resolve (section numbers cited actually exist).

## Non-goals
- Roadmap bookkeeping (INTRO.md state table row + PHASE_5_STATUS.md are task 16).
- Training data (task 14).

## Other information
- If a plan correction changed a pinned decision, section 23 records the FINAL
  value with a pointer to the correction - do not silently overwrite history.
