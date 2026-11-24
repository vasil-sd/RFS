# Phase 18, Task 3I - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start implementation of the task immediately without
> re-deriving scope.

## Goal
Add positive AND negative test coverage for every restriction lifted by tasks 3F-3H,
and refactor the existing test suite so it pins the NEW accepted-syntax behavior in both
directions (accepts what it used to reject; still rejects what must stay rejected).
Rationale: 3F-3H change the accepted-syntax surface; without suite adoption a future
regression (someone "restoring" a deleted check) would be invisible, and the parity
suites would not prove the lifted forms behave identically across modes.

## Context

### The full audit lives in `V1_R_R_AUDIT.md` (repo root)
This task covers its Table 2 rows V17, S3, S4, E4/S1 - exactly what 3F/3G/3H lift.
Read the three task files (`PHASE_18_TASK_3F.md`, `_3G.md`, `_3H.md`) first: their
"Context" sections carry the exact code sites, error strings, and empirical baselines
this task's tests must pin.

### Depends on 3F + 3G + 3H being complete
The new tests assert behavior that does not exist until those tasks land. Do not start
3I before all three are done and their exit criteria met (the lifted syntax must
validate/model-check first, then this task pins it).

### Existing test surfaces (where the new coverage goes)
- `tests/smt_bounded_test.rs` - end-to-end smt-bounded legs with per-task named tests
  (`smt_bounded_task3c_on_error_and_assumptions`, `smt_bounded_task3d_enum_and_wildcards`
  are the pattern to follow: generate a spec, run the in-process AND generated engines,
  assert verdict + summary-line shape). Add ONE new leg
  `smt_bounded_task3f_3h_lifted_restrictions` covering all three lifts end-to-end.
- `tests/ref_mc_smt_test.rs` - in-process smt-bounded unit-level tests (planner errors,
  conversion shapes). Add assertions that the two retired error strings
  ("requires literal initial values", "top-level always/always[N]/eventually/...") are
  gone and that the lifted forms plan successfully.
- `src/lang/validator.rs` inline `#[cfg(test)]` module - add a unit test that a formula
  `input_is` pattern with named fields now type-checks (the mirror of the existing
  `bounded_eventually_zero_bound_is_allowed` style tests already there).
- `examples/valid/` + `examples/invalid/` - the file-driven corpus (50 valid + 13
  invalid, asserted by `tests/example_corpus_test.rs`). Add ONE new valid example that
  exercises all three lifts together, and ONE new invalid example proving a type
  mismatch INSIDE a formula pattern is still caught (the lift must not have weakened
  field typing).
- Training set - if the new valid example is fenced into a training category as a
  snippet, regenerate: `cargo extract-training-snippets && cargo build-training &&
  cargo test-training` (KB.md section 4.6 rules; the generated-code suite grows by the
  new module - expected and recorded, per the Phase 17 close-out reasoning).

### What "still rejects what must stay rejected" means concretely
The lifts are NARROW. These must STILL be errors after 3I (pin them):
- `input_is(Recv(Ack(300)))` where Ack's field is `bit<8>` - literal does not fit
  (V8a stays);
- `input_is(Recv(Ack(true)))` on a non-bool field - cross-type literal (V8b stays);
- `input_is(Recv(Ack(x, y)))` where Ack has one field - arity mismatch (V7 stays);
- a var init referencing another var (`init a + 1`) - rejected by BOTH modes with the
  unified "not a constant expression" message (3H's faithfulness guarantee);
- an out-of-range folded init (`var x: bit<1> init 1 + 1`) - width check stays (3H).

### No existing test pins the OLD rejections
`grep -rn "do not bind variables\|requires literal initial values\|top-level always/always" tests/`
returns nothing - the old behavior was only pinned by the source checks themselves, so
there is no test to FLIP; 3I purely ADDS coverage. (If a grep during execution finds a
pin, flip it and record the flip in the task status.)

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 3F. | Remove obsolete validator/MC restrictions (V17 + S3) | the named-field `input_is` must validate before it can be tested |
| 3G. | Accept bare/`next` top-level properties in smt-bounded (S4) | the lifted top forms must plan before parity can be pinned |
| 3H. | Unify variable-initializer handling across MC modes (E4/S1) | the folded inits must run before acceptance can be pinned |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 3J. | Documentation + README sync | documents exactly the behavior this task pins; the new example's README is written here and finalized there |

### Affected test files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `tests/smt_bounded_test.rs` | change | new leg `smt_bounded_task3f_3h_lifted_restrictions`: end-to-end parity for all three lifts (in-process + generated engines, verdict + summary pins) |
| `tests/ref_mc_smt_test.rs` | change | planner-level assertions: lifted forms plan OK; retired error strings absent; FULL-class gating intact for the wrapped obligation form |
| `src/lang/validator.rs` | change (inline tests only) | unit test: named-field formula `input_is` type-checks; field-typing errors inside formula patterns still fire |
| `examples/valid/<new>.rfs` + README | new | one spec exercising all three lifts together, model-checking PASS under exact AND smt-bounded |
| `examples/invalid/<new>.rfs` | new | a type mismatch inside a formula pattern (proves V7/V8a/V8b survived the lift) |
| `tests/example_corpus_test.rs` | change (only if the corpus test needs the new files registered - check first; the file-driven suite may pick them up automatically) | register the two new examples if not auto-discovered |

### Affected training files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `training/<cat>/rfs_training.jsonl` + `snippets/` + `all_rfs_training.jsonl` | change (regenerated) | only if the new valid example is fenced as a snippet; regenerate + `cargo test-training` |

## Planned actions

### Add the validator unit tests for V17
In `src/lang/validator.rs`'s inline test module, add: (a) a spec with
`prop p : always (input_is(Recv(Ack(x))) -> state == A) ;` asserting
`validate_module` is EMPTY (the lift); (b) the same spec with an out-of-range literal
(`input_is(Recv(Ack(300)))` on a `bit<8>` field) and a cross-type literal, asserting the
field-typing errors STILL fire (the narrowness). Follow the existing test style in that
module (`module_from` helper + message assertions).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/lang/validator.rs` | change (inline `#[cfg(test)]` module) | two new tests: acceptance of named fields + survival of field typing |

### Add the smt planner-level tests for S4 + E4/S1
In `tests/ref_mc_smt_test.rs`, add assertions that: (a) a bare boolean top-level property
and a top-level `next p` plan without error and are NOT flagged FULL class (default
gate); (b) `input_is(E) -> bounded_eventually[5] (q)` as a top form plans but IS flagged
FULL (omitted without `--full-properties`, checked with it - the gate was not weakened);
(c) a var init `5 + 1` plans to the constant 6; (d) a var init referencing another var
fails with the exact-mode-identical message. Use the existing test helpers in that file
for building modules and calling `build_smt_spec`.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `tests/ref_mc_smt_test.rs` | change | four new planner-level assertions per the list above |

### Add the end-to-end parity leg for all three lifts
In `tests/smt_bounded_test.rs`, add `smt_bounded_task3f_3h_lifted_restrictions` following
the `smt_bounded_task3d_enum_and_wildcards` pattern: a spec using named-field
`input_is`, a bare/`next` top-level property, and a constant-expression var init; run it
under exact mode AND smt-bounded (in-process + generated engines where the leg's helpers
allow); assert identical verdicts and pin the summary-line shape. This is the regression
net that proves the lifts are cross-mode-faithful end-to-end.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `tests/smt_bounded_test.rs` | change | one new end-to-end leg covering 3F+3G+3H together |

### Add the two corpus examples
Write `examples/valid/<new>.rfs` (a small, complete module exercising all three lifts,
model-checking PASS under both modes - verify by running before committing) with its
README (the Phase 17/18 example-README pattern), and `examples/invalid/<new>.rfs`
(a formula-pattern type mismatch that must fail validation). Confirm
`tests/example_corpus_test.rs` picks them up (file-driven; register only if needed).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `examples/valid/<new>.rfs` + `README.md` | new | positive corpus example for the lifted syntax |
| `examples/invalid/<new>.rfs` | new | negative corpus example proving field typing survived |
| `tests/example_corpus_test.rs` | change (conditional) | register the new files only if the suite is not auto-discovering them |

### Regenerate the training set (conditional)
If the new valid example is fenced into a training category as a snippet, run
`cargo extract-training-snippets && cargo build-training && cargo test-training` and
record the generated-code growth (the new module's tests) in the task status. If no
snippet is fenced, skip this action and say so in the status.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `training/<cat>/rfs_training.jsonl`, `snippets/`, `all_rfs_training.jsonl` | change (regenerated, conditional) | keep the SFT set in step with the new accepted syntax |

### Run the full verification matrix
`cargo build && cargo test && cargo clippy --all-targets && cargo fmt` plus
`cargo test-generated-code` (the generated suite grows by the new valid example's
module - expected) and `rfs validate --input rfs examples/valid` (exit 0). Record exact
counts in the task status.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none) | - | verification only |

## Task deliverables
- Validator unit tests pinning V17 acceptance + field-typing survival.
- SMT planner tests pinning S4 default/FULL classification + E4/S1 init folding and
  unified rejection.
- One end-to-end parity leg covering all three lifts.
- Two new corpus examples (valid + invalid) with README.
- Regenerated training set (if a snippet was fenced).
- A green full matrix with exact counts recorded.

## Task exit criteria
- [ ] Named-field `input_is` validates and model-checks identically under exact and
      smt-bounded (unit + end-to-end pins).
- [ ] Bare/`next` top-level properties plan as DEFAULT class; the wrapped obligation form
      plans as FULL class (gate intact) - pinned in `ref_mc_smt_test.rs`.
- [ ] Constant-expression var inits fold to the same constant in both modes; var-ref and
      out-of-range inits are rejected identically - pinned.
- [ ] The new invalid example fails validation with a field-typing error (V7/V8a/V8b
      survived); the new valid example PASSes under both modes.
- [ ] `cargo test`, `cargo clippy --all-targets`, `cargo fmt --check`,
      `cargo test-generated-code` all green; exact counts recorded in the status file.
- [ ] No pre-existing pin changed except any old-rejection pin that a grep surfaces
      (recorded as a flip in the status).

## Non-goals
- No source changes to `src/lang/validator.rs` logic, `src/mc/smt_plan.rs`, or any engine
  - this task is TESTS ONLY; if a test exposes a bug in 3F-3H, fix it via a task
  correction to THAT task, not here.
- No doc updates beyond the new example's own README (task 3J owns the doc set).
- No changes to the R1-R4 monitor subset, E1-E3 liveness limits, or any HARD-row rule.

## Other information
- The end-to-end leg must run the spec under exact mode FIRST and confirm PASS before
  pinning the smt-bounded verdict (the Phase 18 authoring discipline from the plan's
  "Other information" section) - a spec that fails exact mode is not a valid parity pin.
- Keep the new valid example's state space small (tens of states) so exact MC is fast,
  per the plan's spec-authoring guidance.
