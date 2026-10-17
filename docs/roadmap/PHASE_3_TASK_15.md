# Phase 3, Task 15 - Implementation details

> Written for a future LLM session: after reading `START.md`,
> `docs/roadmap/INTRO.md` and this file, the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal
Planned task 15: codegen tests pinning `--with-property-monitors` option sensitivity
and monitor presence in generated code for BOTH backends (Rust and C).

## Context

- `BackendOptions.with_property_monitors` exists (`src/codegen.rs:64`, default false),
  parsed from `--with-property-monitors` (`src/codegen.rs:377`), threaded through
  `main.rs` (lines ~258, ~289).
- Existing `tests/codegen_test.rs` tests all construct options with
  `with_property_monitors: false`; NO option-sensitivity test exists yet.
- D-P3-6 (plan, Other information): the flag applies to the PRODUCTION command of
  every backend; mc and test ALWAYS include monitors. Rust gates emission/stepping
  under `#[cfg(debug_assertions)]`; C under `#ifdef RFS_PROPERTY_MONITORS`.
- Generated-code markers to assert (verified present in current codegen output):
  - Rust: `Mon_` monitor structs, `StepError::PropertyViolation`,
    `pub struct PropertyViolation { pub label: String, pub step: usize, pub trigger_step: Option<usize> }`.
  - C: `RFS_PROPERTY_VIOLATION` in `rfs_status_t`, `prop_monitors` state,
    `prop_label`/`prop_step`/`prop_trigger_step` fields, `RFS_PROPERTY_MONITORS` define
    handling for the production command.
- Example 41 (5 prop/assume labels) is the reference spec; requires task 13 green first.

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 8., 9. | Rust monitors + CLI flag | the option and Rust rendering under test |
| 11. | C monitor parity | the C rendering under test |
| 13. | Valid examples fixed | example 42 must parse for per-command coverage over all valid examples |

### Affected test files (optional)

| File | Operation | Brief description |
|------|-----------|-------------------|
| `tests/codegen_test.rs` | change | new tests: option sensitivity (both backends), flag semantics for mc/test, monitor presence, no skip warnings for in-subset properties |

## Planned actions

### Action 1: Option-sensitivity test - Rust production

New test in `tests/codegen_test.rs`: generate example 41 production with the flag off
and on (two `BackendOptions`). Assert: outputs DIFFER; on-output contains `Mon_`
monitor structs + `StepError::PropertyViolation`; off-output contains neither. Rationale:
pins D-P3-6 zero-cost-when-disabled for Rust.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `tests/codegen_test.rs` | change (new test fn after `all_rust_commands_generate_for_valid_examples`) | pins Rust production flag behavior |

### Action 2: Option-sensitivity test - C production

Mirror of Action 1 for the C backend: on-output contains `RFS_PROPERTY_VIOLATION`,
`prop_monitors`, and the `RFS_PROPERTY_MONITORS` gating; off-output contains none of
them. Rationale: parity principle - same option, same semantics per language.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `tests/codegen_test.rs` | change (new test fn after `all_c_commands_generate_for_valid_examples`) | pins C production flag behavior |

### Action 3: mc/test always include monitors (flag not needed)

New test: for example 41, generate mc and test commands in BOTH backends with the flag
off; assert monitor code IS present (monitors unconditional for mc/test per D-P3-6).
Also check CLI-level behavior of passing `--with-property-monitors` to mc/test commands
and assert whatever the current documented behavior is (accepted-and-ignored vs
rejected) - record the observed behavior in the status file; if it contradicts D-P3-6,
stop and record a plan correction instead of changing code here.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `tests/codegen_test.rs` | change (new test fn) | pins mc/test monitor inclusion for both backends |

### Action 4: No skip warnings for in-subset properties, both backends

New test: for every valid example containing `prop `/`assume ` lines (33,
41, 42 - enumerate dynamically like `files()` does), generate production in both
backends and assert ZERO "outside monitor subset" warnings. Rationale: exit criterion
"no skip warnings for in-subset formulas in any backend".

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `tests/codegen_test.rs` | change (new test fn) | pins the no-skip-warning exit criterion dynamically |

## Task deliverables
- 4 new tests in `tests/codegen_test.rs` (exact count recorded in status file).

## Task exit criteria
- [x] All 4 new tests present and passing; `cargo test --no-fail-fast` fully green.
- [x] Rust AND C production flag on/off outputs verified different with correct markers.
- [x] mc/test monitor inclusion asserted for both backends.
- [x] Zero skip warnings asserted for all property-bearing valid examples in both backends.

## Non-goals
- No changes to codegen source or CLI parsing (tests only). If a mismatch between
  behavior and D-P3-6 is found: record a plan correction + new task; do not fix
  backend code inside this task.

## Task corrections

| Correction | Description + rationale |
|---|---|
| Action 1/2 "off has neither" calibrated to the real off-output shape | The flag does NOT remove the per-label `Mon_*` struct definitions from Rust production output (they are emitted in both modes); what it gates is the `StepError::PropertyViolation` variant, the `PropertyViolation` struct, the `PropMonitors` aggregate + stepping calls - all under `#[cfg(debug_assertions)]`. C off-output keeps the stable wire format (`RFS_PROPERTY_VIOLATION` enum value, `prop_label`/`prop_step`/`prop_trigger_step` result fields) but has no `RFS_PROPERTY_MONITORS` define, no `rfs_property_violation_t`, no `prop_monitors` state. Tests assert these exact marker sets (verified against generated output before writing). Behavior matches D-P3-6; no plan correction needed. |
| Action 3 observed CLI behavior: flag REJECTED for mc/test | `parse_codegen_args` returns the error "--with-property-monitors is only valid for the production command; test and mc always include property monitors" for both `mc` and `test`. This is the D-P3-6 "not accepted/needed" path anticipated by the task file's Other information - the test asserts the error message. |
| Action 4 example list was stale: 15 property-bearing files, not (33, 41, 42) | Dynamic enumeration found top-level 04, 13, 14, 15, 16, 24, 37, 41, 42 + tla_adapted 31-36 (the TLA adaptations live in `examples/valid/tla_adapted/`, missed by a top-level-only glob). All 15 x both backends compile with ZERO "outside monitor subset" warnings (empirically verified before writing the test), so the zero-warnings assertion holds as written. |

## Other information

- Test style: follow the existing `parse_module` helper + direct
  `CodegenCommandKind::...run(backend, module, &options)` pattern already used in the file.
- If Action 3 reveals the CLI REJECTS the flag for mc/test (error), that is acceptable
  per D-P3-6 ("not accepted/needed") - assert the error path instead of ignoring it.
