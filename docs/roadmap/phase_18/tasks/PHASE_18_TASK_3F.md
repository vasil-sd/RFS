# Phase 18, Task 3F - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start implementation of the task immediately without
> re-deriving scope.

## Goal
Remove the two OBSOLETE restrictions identified by the `V1_R_R_AUDIT.md` audit:
**V17** - the validator rejection of named fields in formula `input_is` patterns
("formula input patterns do not bind variables in Phase 1"), and **S3** - the dead
"non-power-of-two domain" check in the SMT planner. Both block syntax the downstream
machinery already handles; removing them changes no IR, no codegen, and no MC semantics.
Rationale: they are stale Phase-1-era checks with zero model-checking performance
effect, and V17 also closes a real asymmetry (`output_has` patterns already accept
named fields today).

## Context

### The full audit lives in `V1_R_R_AUDIT.md` (repo root)
Read its Table 2 rows for V17 and S3 before starting. This task implements exactly
those two rows; nothing else.

### V17 - the validator check to delete
File: `src/lang/validator.rs`, function `check_pattern_group` (~line 692). Current body:

```rust
fn check_pattern_group(pattern: &EventPattern, env: &Env, is_input: bool) -> Result<(), String> {
    // Formula patterns do not introduce bindings in Phase 1. We only ensure group/variant names are plausible.
    let _ = is_input;
    if pattern.variant.is_some() {
        if let PatternVariant::Struct { fields, .. } = &pattern.variant.clone().unwrap() {
            for f in fields {
                if f.name.is_some() {
                    return Err("formula input patterns do not bind variables in Phase 1".into());
                }
            }
        }
    }
    let _ = env;
    Ok(())
}
```

It is called from `type_formula` (~line 670) as
`Formula::InputIs { pattern } => check_pattern_group(pattern, env, true)`. Its sibling
`check_output_pattern_group` (~line 708) is ALREADY a documented no-op
(`let _ = (pattern, env); Ok(())`) - which is why `output_has(Send(SynAck(y)))`
validates today while the identical construct under `input_is` does not.

**Why removal is safe (verified empirically with the built binary):** a named field in a
formula pattern is a `PatternExpr::Bind`, and BOTH model-checking backends already render
a `Bind` as a no-constraint wildcard:
- exact mode: `src/mc/ref_mc.rs::input_matches` (~line 584) -
  `PatternExpr::Bind { .. } => true` (comment: "binding names are ignored here (they
  constrain nothing)"); the same in `output_matches` (~line 1492).
- smt-bounded: `src/mc/smt_plan.rs::resolve_input_is` (~line 1108) and the
  `OutputHas` conversion (~line 1101) - `Wildcard | Bind { .. } => {}` (no constraint
  emitted).
The bound name is not referenceable anywhere in a formula (formulas type-check against
the module env, which carries no pattern bindings), so accepting it adds exactly the
semantics of a labeled wildcard. Reproduction before the fix:
`rfs validate` on `prop p : always (input_is(Recv(Ack(x))) -> state == A) ;` fails with
"formula input patterns do not bind variables in Phase 1"; the same spec with
`output_has(Send(SynAck(y)))` passes.

**No existing test pins the old rejection.** `grep -rn "do not bind variables" tests/ src/`
matches only the validator source line itself - there is no unit test, invalid example,
or training snippet to flip. (The validator's inline `#[cfg(test)]` module has no
case for it.) Acceptance coverage is added by task 3I.

### S3 - the dead check to delete
File: `src/mc/smt_plan.rs`, inside `build_smt_spec`'s input-class field loop (~line 470),
the `Some(ValueDomain::Int { max })` arm:

```rust
let w = ceil_log2((*max as usize) + 1);
if (1u64 << w) - 1 != *max {
    return Err(format!(
        "non-power-of-two domain for field '{}' (not a bit<N> type)",
        f.name
    ));
}
(w, Vec::new())
```

**Why it is dead:** `ValueDomain::Int { max }` is produced ONLY by
`src/mc/plan.rs::value_domain` (~line 291) for `Type::Bit { width }`, where
`max = (1u64 << width) - 1` (or `u64::MAX` for width >= 64, which the arm above already
handles separately). Every such max satisfies `(1u64 << ceil_log2(max+1)) - 1 == max`,
so the error branch is unreachable. Deleting the `if` block (keeping `let w = ...;`
and `(w, Vec::new())`) changes no behavior. The adjacent "wider than 64 bits" check in
the same arm is LIVE (audit row S2) and stays.

### Interaction with tasks 3G/3H
All three tasks edit `src/mc/smt_plan.rs` but in DISJOINT regions: 3F deletes the
S3 `if` block in the input-class field loop (~line 478); 3G edits the property kind
match (~line 745) and the assumption conversion match (~line 856); 3H edits the var-init
match (~line 425). Execute them as separate commits so each stays independently
revertible.

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 3D. | Maximize RFS language support in smt-bounded (D8) | already complete - its `resolve_input_is` wildcard/bind handling is what makes V17 removal safe; no dependency on unexecuted work |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 3I. | Tests for the 3F-3H changes + test-suite adoption | adds the acceptance examples/tests for the now-legal `input_is` named fields |
| 3J. | Documentation + README sync | retires the "formula patterns bind nothing" doc wording this task makes false |

### Affected source files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `src/lang/validator.rs` | change | delete the named-field rejection in `check_pattern_group`; turn it into a documented no-op mirroring `check_output_pattern_group` |
| `src/mc/smt_plan.rs` | change | delete the dead non-power-of-two `if` block in the input-class field loop |

## Planned actions

### Delete the V17 named-field rejection
Replace the body of `check_pattern_group` with a documented no-op, keeping the function
(it is still the `InputIs` arm's hook in `type_formula` and keeps the call site stable):

```rust
fn check_pattern_group(pattern: &EventPattern, env: &Env, is_input: bool) -> Result<(), String> {
    // Formula patterns introduce no bindings (task 3F): a named field is a labeled
    // wildcard - both MC backends render PatternExpr::Bind as a no-constraint match
    // (ref_mc.rs::input_matches / smt_plan.rs::resolve_input_is). Mirrors
    // check_output_pattern_group, which never restricted names.
    let _ = (pattern, env, is_input);
    Ok(())
}
```

Rationale: deleting the function instead would ripple into `type_formula`'s match arm;
a no-op with the rationale comment keeps the diff minimal and documents WHY named
fields are legal (a future session must not "restore" the check).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/lang/validator.rs` | change (`check_pattern_group`, ~692-706) | body replaced by the no-op above; the stale "Phase 1" comment goes with it |

### Delete the S3 dead domain check
In `src/mc/smt_plan.rs`'s input-class field loop, remove the
`if (1u64 << w) - 1 != *max { return Err(...) }` block, keeping `let w = ceil_log2(...)`
and the `(w, Vec::new())` result. Update the arm's comment to note that Int domains are
always 2^N-1 by construction (`plan.rs::value_domain`), so no power-of-two test is needed.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/mc/smt_plan.rs` | change (~478-485) | dead `if` block deleted; comment updated |

### Verify no behavior change beyond the two lifts
Run the standard matrix (BUILD.md section 5): `cargo build && cargo test &&
cargo clippy --all-targets && cargo fmt`. Then confirm the two lifts with the debug
binary: (1) a spec with `input_is(Recv(Ack(x)))` now validates AND model-checks under
both `rfs mc-ref --mode exact` and `--mode smt-bounded`; (2) no test output changed for
any pre-existing example (the file-driven suites are the regression net - all 50 valid +
13 invalid examples must behave exactly as before, since none of them uses the lifted
syntax).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none) | - | verification only; scratch specs go in /tmp, not the repo |

## Task deliverables
- `src/lang/validator.rs` with `check_pattern_group` a documented no-op (V17 lifted).
- `src/mc/smt_plan.rs` without the dead power-of-two check (S3 lifted).
- A green full test matrix proving no other behavior moved.

## Task exit criteria
- [ ] `rfs validate` accepts `prop p : always (input_is(Recv(Ack(x))) -> state == A) ;`
      (named field in a formula input pattern) on a scratch spec.
- [ ] The same scratch spec model-checks under BOTH `rfs mc-ref --mode exact` and
      `--mode smt-bounded` with the same verdict as the wildcard form `input_is(Recv(Ack(_)))`.
- [ ] `grep -rn "do not bind variables" src/ tests/` returns nothing.
- [ ] `grep -rn "non-power-of-two domain" src/` returns nothing; the adjacent
      "wider than 64 bits" check is still present and intact.
- [ ] `cargo test`, `cargo clippy --all-targets`, `cargo fmt --check` all green with the
      same test counts as before the change (no test was flipped in this task - 3I adds
      the new coverage).
- [ ] All 50 valid + 13 invalid examples validate/model-check exactly as before.

## Non-goals
- No new tests/examples for the lifted syntax (task 3I owns that, so the test work is
  reviewed and committed as one coherent suite change).
- No doc updates (task 3J).
- No changes to V18 (`always[N] >= 1`), R1-R4, E1-E3, S2, S4, S5, or any HARD-row rule
  from the audit - those stay deferred/kept per `V1_R_R_AUDIT.md`.
- No IR schema change: `PatternField.name` already exists and round-trips; nothing is
  added to or removed from the JSON format.

## Other information
- If clippy flags the now-unused `is_input` parameter after the no-op rewrite, the
  `let _ = (pattern, env, is_input);` binding silences it deliberately - do not delete
  the parameter (it documents the input/output symmetry and keeps the call site stable).
- The audit's empirical reproduction specs are in /tmp of the planning session
  (`/tmp/rule_check.rfs`, `/tmp/rule_check2.rfs`); recreate them from the Context section
  if needed - they are not committed.
