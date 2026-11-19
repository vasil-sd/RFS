# Phase 17, Task 8 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal
Create the four script-driven PLAIN-DIR example projects covering the remaining
usage scenarios: end-to-end MC report reading, JSON-schema CI validation,
the gen-spec mutation prototyping loop, and explicit external-tool path passing
(`RFS_*` env vars + `--print-script` proof + a gcc/clang Makefile choice).

## Context
- Plain dirs (NOT workspace members - the `c_sync_handler` / `fuzz_c`
  convention): each has a `README.md` per the PHASE_17.md template + script(s)
  + spec file(s), and a test leg in `tests/example_projects_test.rs` that runs
  the real script/commands and asserts PINNED output excerpts.
- Availability gating: legs needing a real tool (gcc, clang, rustc, python3 +
  jsonschema) skip with a note via `tests/common/toolchain.rs` helpers - never
  fail on absence. `python3 jsonschema` 4.26.0 IS present in this environment.
- The five JSON document formats with schemas: `spec-json`, `ce-trace`,
  `mc-report`, `validate-json`, `info-json` (`rfs info format --list`).
- Tool-resolution contract (TOOLCHAIN.md section 8.2): CLI flag > RFS_* env var
  > standard Unix var > auto-discovery. `--print-script` prints the resolved
  invocation as a self-contained POSIX sh script whose header carries a
  `# tools: <name>=<path> ...` line - that line is the proof artifact.
- Inventory rule: new test fns get TEST_STRUCTURE rows in the same commit
  (budget extended to 1430 by task 3).
- README template + mermaid (`stateDiagram-v2` only) + relative links
  (`../../../docs/user/...`): as in task 6.

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 3. | Fix stale LLM set + KB.md + extend inventory budgets | TEST_STRUCTURE headroom must exist before new rows are appended |

### Affected test files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `tests/example_projects_test.rs` | change | one leg per plain-dir example (4 new fns + shared helpers if needed) |

### Other affected files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `crates/examples/mc_walkthrough/` | new | specs + run script + README |
| `crates/examples/json_schema_ci/` | new | validation script + README |
| `crates/examples/specgen_mutation_loop/` | new | loop script + README with pinned transcript |
| `crates/examples/tool_resolution/` | new | run script + Makefile + README |
| `docs/llm/TEST_STRUCTURE.md` | change | rows for the new test fns (same commit) |

## Planned actions

### 8.1 - mc_walkthrough
A passing spec (e.g. `examples/valid/03_timeout_retry.rfs`) + a failing variant
(reuse task 6's property-violation variant if it lands first, else derive one).
Run script: generate MC binaries for BOTH backends (`rfs codegen rust|c mc`),
compile (rustc / gcc -Wall), run exact mode on the passing spec (capture the
PASS report), then on the failing variant (capture text + JSON CE documents).
README per the template: walks a report END TO END - the status line, every
summary field (states/transitions/max_depth/monitors_bits/unexpected_terminations/
memory estimate), the CE document structure (kind, prefix/cycle or trace,
ce_fields) - using the real captured output. The automaton gets its mermaid
diagram.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `crates/examples/mc_walkthrough/run.sh` | new | the generate-compile-run pipeline as a script |
| `crates/examples/mc_walkthrough/*.rfs` | new | passing spec + failing variant |
| `crates/examples/mc_walkthrough/README.md` | new | template + end-to-end report walkthrough with real captures |

### 8.2 - json_schema_ci
Script: for each of the five formats, emit the schema (`rfs info format <name>
--output schema`) and generate a REAL document of that kind (parse JSON via
`rfs parse --input rfs --output json`; mc report via a generated MC binary's
`--counterexample-format json` run or mc-ref; validate-json via `rfs validate
--json` on an invalid spec; info-json via `rfs info --json`; ce-trace from the
failing variant). Validate each document against its schema with
`python3 -m jsonschema` (availability-gated). README: what self-describing
formats are for (CI validation without the rfs source), the five formats table,
the script's real output, how to wire it into a CI pipeline.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `crates/examples/json_schema_ci/validate.sh` | new | schema emission + real-document generation + jsonschema validation |
| `crates/examples/json_schema_ci/README.md` | new | template + CI-wiring walkthrough with real output |

### 8.3 - specgen_mutation_loop
Script: the rapid-prototyping loop from docs/user/INTEGRATION.md section 8 as a
runnable thing - `rfs gen-spec --out base.rfs --seed <pinned>` (deterministic),
`rfs validate`, `rfs mc-ref`; then `rfs gen-spec --base base.rfs --mutations K
--out mut.rfs`, `rfs mc-ref mut.rfs` (a failing mutation is a teaching moment -
capture the CE); show the fix-and-repeat idea. README: pinned transcript of the
REAL run, what each step proves (seed determinism, always-valid output,
instant in-process checking), when to use the loop.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `crates/examples/specgen_mutation_loop/loop.sh` | new | the generate-validate-check-mutate loop |
| `crates/examples/specgen_mutation_loop/README.md` | new | template + pinned real transcript |

### 8.4 - tool_resolution
Run script: set EXPLICIT paths (`RFS_GCC=/usr/bin/gcc`, `RFS_RUSTC=<toolchain>
/rustc`, `RFS_CLANG=/usr/bin/clang`) and run `rfs test --suite unit --backend
c --print-script <spec>` + the rust equivalent + a codegen invocation - capture
the `# tools:` header lines proving each resolved to the explicit path. Then
show precedence: a CLI flag (`--gcc /bin/sh-fake`) beats the env var (capture
its `# tools:` line); with nothing set, bare PATH resolution applies. Makefile:
`CC ?= gcc` with a documented `make CC=clang` alternative building a generated
C FSM warning-free under either compiler (clang leg availability-gated).
README: the full RFS_* table (mirroring USAGE.md section 2 - written there in
task 10, referenced here), what `--print-script` shows, the precedence proof
with real captured lines, and the Makefile choice.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `crates/examples/tool_resolution/run.sh` | new | explicit-path + precedence demonstration with --print-script captures |
| `crates/examples/tool_resolution/Makefile` | new | gcc/clang choice building a generated C FSM |
| `crates/examples/tool_resolution/README.md` | new | template + precedence walkthrough with real captured lines |

### Test legs + inventory rows
One leg per example in `tests/example_projects_test.rs`: run the script (or the
Makefile targets), assert exit 0 + PINNED excerpts from the real captures (the
`# tools:` lines, a schema-validation OK line, a transcript line). Gate on tool
availability. Append TEST_STRUCTURE rows for the new fns in the same commit.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `tests/example_projects_test.rs` | change | 4 new legs - the scripts are part of the verified surface |
| `docs/llm/TEST_STRUCTURE.md` | change | one row per new fn (same-commit inventory rule) |

## Task deliverables
- Four plain-dir example projects with real captured output and template READMEs.
- Four pinned test legs + inventory rows.

## Task exit criteria
- [ ] Every README follows the template (all five sections; mermaid diagram
      where an automaton is involved).
- [ ] tool_resolution's README carries REAL `# tools:` lines proving explicit
      paths win, and the Makefile builds under both gcc and clang (clang leg
      gated).
- [ ] json_schema_ci validates all five formats' real documents against their
      schemas (python3+jsonschema gated).
- [ ] specgen_mutation_loop's transcript is a REAL pinned run (seed-deterministic).
- [ ] `cargo test --test example_projects_test` green; `cargo test --test
      docs_md_test` green.
- [ ] TEST_STRUCTURE rows added; `cargo test --test dev_docs_test` green.
- [ ] Full checklist: `cargo build && cargo test && cargo clippy --all-targets
      && cargo fmt --check`.

## Non-goals
- No workspace members (plain dirs by design).
- No changes to the resolver itself (`src/cli/ext_tool.rs`) - this task only
  demonstrates it.
