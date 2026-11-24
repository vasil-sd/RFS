# Phase 18, Task 3H - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start implementation of the task immediately without
> re-deriving scope.

## Goal
Lift audit rows **E4/S1**: make smt-bounded mode accept exactly the variable
initializers exact mode accepts - constant expressions (literals, state-variant paths,
arithmetic/comparison/boolean combinations over them) - instead of only bare literals.
The SMT planner evaluates each init in the same context exact mode uses (`state` =
initial state, NO vars bound) and emits the resulting constant. Rationale: smt-bounded
is needlessly stricter than exact mode for a one-time initialization with zero
formula-size or performance effect; unifying removes the last cross-mode asymmetry in
spec acceptance.

## Context

### The full audit lives in `V1_R_R_AUDIT.md` (repo root)
Read its Table 2 rows for E4/S1 before starting. This task implements exactly that row.

### Verified empirical baseline (reproduce before editing)
With the debug binary, on scratch specs:
- exact mode ACCEPTS `var a: bit<8> init 5 + 1 ;` (constant expression - it folds it);
- exact mode REJECTS `var b: bit<8> init a + 1 ;` (references another var) with
  "variable 'b' initializer is not a constant expression (unsupported)";
- smt-bounded REJECTS BOTH with "smt-bounded mode requires literal initial values
  (var 'a' has Binary { op: Plus, ... })".
After this task, smt-bounded must accept/reject the SAME set as exact mode.

### The exact-mode model to mirror
`src/mc/ref_mc_exact.rs` (~line 336): for each var it builds
`EvalCtx { module, state: Val::Enum(states[q0].clone()), vars: HashMap::new(),
binds: HashMap::new(), bind_types: HashMap::new() }` and calls
`eval_expr(&v.init, &ctx)` - note the EMPTY `vars` map is rebuilt per var, so a later
var's init can NEVER reference an earlier var (that is exact mode's actual semantics,
and it is what "constant expression" means here). On `Err` it returns the
"not a constant expression" error. `eval_expr` and `EvalCtx` live in
`src/mc/ref_mc.rs` (~line 270 / ~line 292) as `pub struct EvalCtx` / `pub(crate) fn
eval_expr` - both reachable from `smt_plan.rs` (sibling module, same crate).

### The SMT site to change
`src/mc/smt_plan.rs::build_smt_spec`, the var loop's init match (~line 425-446). Today:

```rust
let init = match &v.init {
    Expr::Int { value } => *value,
    Expr::Bool { value } => u64::from(*value),
    Expr::Path { path } if is_enum => { /* enum-variant index, ~line 415-437 */ }
    other => return Err(format!(
        "smt-bounded mode requires literal initial values (var '{}' has {:?})",
        v.name, other)),
};
if init >= (1u64 << width.min(64)) {
    return Err(format!("initial value of var '{}' does not fit in {} bits", v.name, width));
}
```

The fix: replace the `other => Err(...)` arm with an evaluation via
`crate::mc::ref_mc::eval_expr(&v.init, &EvalCtx { module, state: Val::Enum(initial_state_name), vars: HashMap::new(), binds: HashMap::new(), bind_types: HashMap::new() })`,
then convert the resulting `Val` to the init constant the same way the existing arms do
(`Val::Bit(n) => n`, `Val::Bool(b) => u64::from(b)`, `Val::Enum(name) => variant index`
for enum-typed vars - reuse the enum-index lookup the `Expr::Path` arm already performs).
On `Err`, return an error with the SAME message exact mode uses ("variable '{}' 
initializer is not a constant expression (unsupported)") so both modes reject
identically-worded. The existing "does not fit in N bits" width check stays and now also
guards evaluated constants (a folded `5 + 1` into a `bit<1>` must still fail there).
Keep the fast literal arms (`Expr::Int`/`Expr::Bool`/enum `Expr::Path`) as-is - they are
the common case and avoid building an `EvalCtx` for them; only genuinely non-literal
inits fall through to the evaluator. (Equivalently: route EVERY init through the
evaluator - it handles literals identically - if that is cleaner; both are correct, but
the fast-path version keeps the diff smaller.)

### Why this is safe for the SMT encoding
An init constant is a single `u64` stored in `SmtVar.init` and asserted as
`var_0 == <init>` at step 0 by every engine (in-process `ref_mc_smt.rs` and both
generated runtimes). Folding `5 + 1` to `6` at plan time changes no formula structure -
the assertion is the same shape with a different constant. Enum inits already encode as
variant indices; the evaluator returns `Val::Enum(name)` for a state-variant path, which
the existing index lookup converts. No engine, emitter, or wire-format change.

### Interaction with tasks 3F/3G
Disjoint regions of `src/mc/smt_plan.rs`: 3H rewrites the var-init match (~425-446);
3F deletes the S3 `if` block (~478); 3G edits the property kind match (~745) and
`convert_assumption` (~888). Separate commits.

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| (none) | - | `eval_expr`/`EvalCtx` have existed since Phase 6; no unexecuted dependency |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 3I. | Tests for the 3F-3H changes + test-suite adoption | adds the exact-vs-smt init-acceptance parity pins |
| 3J. | Documentation + README sync | retires any "smt-bounded requires literal inits" wording |

### Affected source files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `src/mc/smt_plan.rs` | change | var-init `other` arm replaced by an `eval_expr`-based constant fold with exact-mode-identical rejection |

## Planned actions

### Replace the literal-only init arm with a constant-expression fold
In `build_smt_spec`'s var loop, keep the `Expr::Int` / `Expr::Bool` / enum-`Expr::Path`
arms as fast paths and replace the final `other => Err("requires literal initial
values")` arm with: build the exact-mode init context (`state` = the module's initial
state as `Val::Enum`, empty `vars`/`binds`/`bind_types`), call
`crate::mc::ref_mc::eval_expr(&v.init, &ctx)`, convert the `Val` to the init constant
(`Bit(n) => n`, `Bool(b) => u64::from(b)`, `Enum(name) =>` the variant-index lookup the
enum path arm already uses), and on `Err` return "variable '{}' initializer is not a
constant expression (unsupported)" - byte-identical to exact mode's message. The
subsequent width check (`init >= 1 << width`) then guards folded constants too.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/mc/smt_plan.rs` | change (~425-446) | `other` arm becomes the evaluator fold; error message unified with exact mode; a short comment cites task 3H + the audit row |

### Verify identical acceptance across modes
Scratch specs (in /tmp): (1) `var a: bit<8> init 5 + 1 ;` - now ACCEPTED by smt-bounded
(same verdict as exact mode); (2) `var b: bit<8> init a + 1 ;` - REJECTED by BOTH modes
with the identical "not a constant expression" message; (3) an enum var initialized from
a state-variant path - accepted by both, encoding to the same index. Then the full
matrix: `cargo build && cargo test && cargo clippy --all-targets && cargo fmt`. Every
pre-existing pin stays byte-identical (no committed spec uses a non-literal init that
smt-bounded previously rejected - if one surfaces, it is a finding to record, not a
reason to keep the old rejection).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none) | - | verification only |

## Task deliverables
- `src/mc/smt_plan.rs` accepting exactly the var initializers exact mode accepts, with
  byte-identical rejection messages.
- A green full test matrix with all pre-existing pins unchanged.

## Task exit criteria
- [ ] `rfs mc-ref --mode smt-bounded` on a scratch spec with `var a: bit<8> init 5 + 1 ;`
      runs (no "requires literal initial values" error) and matches exact mode's verdict.
- [ ] A var init referencing another var is rejected by BOTH modes with the identical
      message "variable '...' initializer is not a constant expression (unsupported)".
- [ ] `grep -rn "requires literal initial values" src/` returns nothing.
- [ ] The "does not fit in N bits" width check still fires for an out-of-range folded
      constant (scratch: `var x: bit<1> init 1 + 1 ;` -> rejected by both modes).
- [ ] `cargo test`, `cargo clippy --all-targets`, `cargo fmt --check` green; all
      pre-existing pins byte-identical.

## Non-goals
- No threading of earlier vars into later inits - exact mode does NOT allow that (empty
  vars map per init), and faithfulness to exact mode is the point; "declaration-order
  evaluation" would DIVERGE from exact mode and is deliberately excluded.
- No new tests/examples committed (task 3I).
- No doc updates (task 3J).
- No change to transition assignments or guard expressions - only var INITIALIZERS.

## Other information
- `eval_expr` first calls `expr_type(e, ctx.module, &ctx.bind_types)` for typing; with
  empty binds that is the module-level typing, which the validator already guaranteed -
  so a validated spec's init either folds or fails on an unknown identifier, exactly as
  in exact mode.
- If the enum-index lookup needs the var's declared enum variants, they are already in
  scope in that loop (`enum_variants` from the `Type::Enum` match a few lines above) -
  reuse it rather than re-looking-up.
