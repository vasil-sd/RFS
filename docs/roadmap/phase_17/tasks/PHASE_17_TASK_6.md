# Phase 17, Task 6 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal
Create the three NEGATIVE example projects that teach diagnosis: a deadlock MC
failure, a property-violation MC failure, and an invalid-spec validation
failure - each with real captured output, a step-by-step walkthrough, and the
fix with re-run proof. These are the highest-value user documentation after the
happy path.

## Context
- All three are PLAIN DIRS under `crates/examples/` (NOT workspace members -
  the established convention of `c_sync_handler` / `fuzz_c`): spec file(s) +
  `README.md` per the PHASE_17.md template + a test leg in
  `tests/example_projects_test.rs`.
- README template (BINDING, from PHASE_17.md "Proposed example README.md
  template"): What it demonstrates / The machine (mermaid stateDiagram ALWAYS)
  / Files layout / Build-run-test / Expected results - and for negative
  examples: what the counterexample trace shows, why the spec fails, and the
  fix + re-run result.
- Mermaid: use `stateDiagram-v2` only (the subset rfs generates - guaranteed to
  parse under `docs_md_test`).
- Relative links from `crates/examples/<name>/` to user docs are
  `../../../docs/user/...`.
- Test-leg conventions (from the existing file): `repo_root()` /
  `rfs_bin()` helpers, availability gating via `tests/common/toolchain.rs`
  (skip-with-note, never fail), scratch dirs under `target/`, pinned output
  excerpts asserted with `contains`.
- Inventory rule (PROTOCOL.md section 5): every new test fn gets a
  TEST_STRUCTURE.md row in the SAME commit (budget already extended to 1430 by
  task 3 - use `rfs-dev inventory add --doc TEST ...` or match the existing
  row format exactly).
- Spec selections (from the plan Context):
  - deadlock: `examples/valid/04_tcp_handshake_like.rfs` - known FAIL: deadlock
    at terminal `Established` (the teaching case in docs/user/EXAMPLES.md).
  - property violation: ONE of the pinned FAIL variants in
    `tests/common/fail_variants.rs` - `http2_rst_fail_src()` (spec 47, the RST
    arm removed) or `can_passive_fail_src()` (spec 49). Pick one, extract its
    source string into a `.rfs` file in the example dir, and pin it.
  - invalid spec: `examples/invalid/invalid_duplicate_priority.rfs` (two
    transitions with the same priority - a semantic error with an obvious fix:
    change one priority).
- The FIX for each negative example is a SECOND spec file in the same dir that
  passes - capture its real PASS output too.

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 3. | Fix stale LLM set + KB.md + extend inventory budgets | TEST_STRUCTURE budget headroom must exist before new rows are appended |

### Affected test files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `tests/example_projects_test.rs` | change | one test leg per negative example (3 new fns + shared helpers if needed) |

### Other affected files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `crates/examples/mc_deadlock_fail/` | new | spec + fixed spec + README |
| `crates/examples/mc_property_violation_fail/` | new | extracted FAIL variant + fixed spec + README |
| `crates/examples/invalid_spec_diagnosis/` | new | invalid spec + corrected spec + README |
| `docs/llm/TEST_STRUCTURE.md` | change | rows for the new test fns (same commit) |

## Planned actions

### 6.1 - mc_deadlock_fail
Copy `examples/valid/04_tcp_handshake_like.rfs` into the example dir. Run
`rfs mc-ref` on it: capture the REAL text FAIL report + the deadlock CE (lasso)
and the JSON form (`--counterexample-format json`). Generate BOTH backends' MC
binaries and run them - capture their reports too (they must agree with
mc-ref). Write the fix as a second spec file (derive it from the CE: the
deadlock is at terminal `Established` - the fix adds the missing way out or
adjusts the assumption; verify by re-running to PASS). README per the template:
the automaton (TCP-handshake-like endpoint, drawn from the real protocol),
mermaid diagram, the CE trace walkthrough field by field (what a deadlock lasso
means: prefix + cycle, why the machine is stuck), what to pay attention to,
and the fix + re-run proof.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `crates/examples/mc_deadlock_fail/04_tcp_handshake_like.rfs` | new | the failing spec (copy) |
| `crates/examples/mc_deadlock_fail/fixed.rfs` | new | the corrected spec that passes |
| `crates/examples/mc_deadlock_fail/README.md` | new | template + CE walkthrough + fix proof |

### 6.2 - mc_property_violation_fail
Extract ONE pinned FAIL variant from `tests/common/fail_variants.rs` (read the
function, write its source string to a `.rfs` file - keep the function's header
comment explaining the mutation). Run mc-ref + both backends: capture the REAL
violation CE (trace kind, not lasso). Write the fix (restore the mutated arm)
as a second spec file; re-run to PASS. README per the template: which property
fires, at which step, why the mutation breaks it, field-by-field trace
walkthrough, fix + re-run proof.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `crates/examples/mc_property_violation_fail/<name>_fail.rfs` | new | the extracted FAIL variant |
| `crates/examples/mc_property_violation_fail/fixed.rfs` | new | the corrected spec that passes |
| `crates/examples/mc_property_violation_fail/README.md` | new | template + violation walkthrough + fix proof |

### 6.3 - invalid_spec_diagnosis
Copy `examples/invalid/invalid_duplicate_priority.rfs`. Run `rfs validate`
text mode AND `--json`: capture both REAL outputs. README per the template:
what the duplicate-priority error means (two transitions from the same state on
the same event with equal priority - the firing order is undefined, so the
validator rejects it), the `--json` diagnostic explained field by field (kind,
message, line/col), and the corrected spec (one priority changed) that
validates - capture its exit 0.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `crates/examples/invalid_spec_diagnosis/broken.rfs` | new | the invalid spec (copy) |
| `crates/examples/invalid_spec_diagnosis/fixed.rfs` | new | the corrected spec that validates |
| `crates/examples/invalid_spec_diagnosis/README.md` | new | template + diagnostic walkthrough + fix proof |

### Test legs + inventory rows
Add one test fn per example to `tests/example_projects_test.rs`: run the real
commands via `rfs_bin()`, assert exit code 1 + a PINNED excerpt of the failure
output (CE line / diagnostic message - copied from the real capture, never
hand-written), and assert the fixed variant exits 0. The MC legs are
availability-gated on rustc/gcc like the existing c_sync_handler leg. Append a
TEST_STRUCTURE.md row per new fn in the same commit.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `tests/example_projects_test.rs` | change | 3 new test fns (+ shared helpers if factored) - pin the failures so a regression in CE format is caught |
| `docs/llm/TEST_STRUCTURE.md` | change | one row per new fn (same-commit inventory rule) |

## Task deliverables
- Three negative example projects with real captured traces, walkthroughs and
  fix proofs.
- Three pinned test legs + inventory rows.

## Task exit criteria
- [ ] Every README follows the template (all five sections; mermaid diagram
      present).
- [ ] Every pinned excerpt in a test assertion was copied from a REAL run in
      this environment (no hand-written approximations).
- [ ] The fixed variants actually pass (exit 0) - verified by the test legs.
- [ ] `cargo test --test example_projects_test` green; `cargo test --test
      docs_md_test` green (the new READMEs are in its file set).
- [ ] TEST_STRUCTURE rows added; `cargo test --test dev_docs_test` green.
- [ ] Full checklist: `cargo build && cargo test && cargo clippy --all-targets
      && cargo fmt --check`.

## Non-goals
- No changes to `examples/valid/` or `examples/invalid/` (the corpus stays as
  is - the copies live in the example dirs).
- No new workspace members.
