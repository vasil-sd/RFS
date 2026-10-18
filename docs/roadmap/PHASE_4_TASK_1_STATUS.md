# Phase 4, Task 1 - Implementation status

> Written once when all exit criteria of `PHASE_4_TASK_1.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

| Name | Description |
|---|---|
| `src/mc_plan.rs` | Codegen-time MC plan (D-P4-1..3): input-abstraction analysis (referenced vs abstracted payload fields), configuration bit-encoding layout (state index + packed vars + monitor transition states), traversal spec (canonical value domains, per-state transition firing order, assumption labels) + 10 unit tests |

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| `src/lib.rs` | changed | declare `pub mod mc_plan;` (alphabetical position) |

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| - | - | none (docs sync is task 13) |

## Other file changes

| Name | Operation | Description |
|---|---|---|
| - | - | none |

## Changed functionality
- None. The plan module is not consumed by any backend yet; no generated output
  changed (`cargo test-generated-code` still 513 pass without regeneration).

## Removed functionality
- None.

## New functionality
- `McPlan::compute(module, assumptions)` - pure, deterministic codegen-time plan:
  - Input abstraction (D-P4-3): per event variant, per field - concrete (full
    canonical domain: bool false<true, bit<N> 0..max ascending, enum declaration
    order) vs abstracted (single representative value). Reference kinds: guard,
    emit args, assignment values (binding used), property/assumption `input_is`
    patterns with constraining field literals. Binding alone does NOT reference;
    wildcards/binds in `input_is` do not reference.
  - Configuration encoding (D-P4-2): state index bits + per-var bit slots +
    per-monitor persistent-state layout (Obligation bound>=1: pending 1 bit +
    remaining ceil(log2(bound+1)) bits; DelayedCheck: antecedent 1 bit; node ids
    preorder matching backend `n{id}` naming). Sticky failed/trigger_step flags
    excluded by invariant (live configs have no confirmed failure).
  - Traversal spec: per-variant value domains, per-state transition firing order
    (priority desc, declaration order for ties), assumption labels.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|
| `mc_plan::tests` (10 tests in `src/mc_plan.rs`) | new | each reference kind keeps a field concrete; unreferenced field abstracted with representative; bound-but-unused field abstracted; wildcard/bind in input_is do not reference; encoding layout deterministic + exact bit counts; obligation/delayedcheck state bits; assumption selection affects encoding; canonical value domains and order; state transition firing order; assumption labels in declaration order |

## Tests run status

| Name | Status |
|---|---|
| `cargo build` | OK |
| `cargo test` (180: 96 lib incl. 10 new mc_plan, 41 frontend CLI logic, 23 codegen, 3 examples, 4 training, 4 parser-library comparison, 2 Phase 2 integration, 4 monitor integration, 3 cross-backend parity) | OK |
| clippy `--all-targets` (via `RUSTC_WORKSPACE_WRAPPER=clippy-driver cargo check --all-targets`, see findings) | OK - back to the 12 pre-existing warnings (11 lib + 1 codegen_test), zero from `mc_plan.rs`, no errors |
| `cargo fmt --check` | OK |
| `cargo test-generated-code` (513, no regeneration needed) | OK |

## Found problems/issues/mismatches
- `cargo clippy --all-targets` is BROKEN in this environment: cargo's
  clippy-driver probe (`clippy-driver <rustc> - --print=...`) receives another
  process's error text on stdin (observed: a rustc E0554 message / a `cat`
  stderr line) and fails with "unknown start of token". Pre-existing: reproduces
  identically on the pristine tree (verified via `git stash`). Workaround used:
  `RUSTC_WORKSPACE_WRAPPER=<toolchain>/clippy-driver cargo check --all-targets`
  (plain-rustc probe works; clippy lints workspace crates). A future session
  should use this workaround until the environment is fixed.
- RFS syntax gotchas hit while writing test specs (parser, not plan bugs):
  pattern bindings are bare identifiers (`on E(V(x))`, NOT `$x`); output
  declarations need a variant inside the outer parens (`output O(P(y: bool))`),
  and emits use `emit O(P(y));`.

## Other important findings/information
- Monitor tree shapes (verified against `src/monitor.rs`): `p -> be[N](q)`
  compiles to a SINGLE root Obligation node; `p -> next q` to a single root
  DelayedCheck; bare `always(signal)` to a RangeCheck (0 persistent bits).
  Task 6's liveness work should not assume wrapper nodes.
- The plan's monitor set mirrors `rust_fsm::compile_monitors`: properties
  (declaration order) then SELECTED assumptions (declaration order); out-of-
  subset formulas are skipped (no state) - backends keep emitting their own
  identical skip warnings.
- Task 2 can consume the plan directly: `McPlan` is `Clone + PartialEq`
  (byte-determinism testable) and every field is a plain table renderable as
  constants in Rust or C.
