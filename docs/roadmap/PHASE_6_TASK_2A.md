# Phase 6, Task 2a - Implementation details

> Written for a future LLM session: after reading `START.md`,
> `docs/roadmap/INTRO.md` and this file, the assistant must be able to start
> implementation of the task immediately without re-deriving scope.
>
> Added by plan correction (Phase 6 task 1 cross-check finding; see
> `PHASE_6_PLAN.md` "Plan corrections", row 2a). Runs AFTER task 2, BEFORE
> task 3, so every later task builds on corrected C output.

## Goal
Make the generated C code WELL-DEFINED and exactly parity with Rust for all
inputs: (a) shifts by amounts `>= width` evaluate to `0` (the RFS semantics,
as implemented by the Rust backend and the SMT encoder) instead of relying on
C integer-promotion behavior that is undefined when the variable shift amount
reaches the promoted LHS width; (b) bitvector multiplication computes in a
uint64_t-wide intermediate so `bit<16>` products above INT_MAX are not signed
overflow. Add a file-driven example + targeted cross-backend differential test
that exercise both cases.

## Context

- **Finding** (verified during task 1, recorded in `docs/SEMANTICS.md`
  section 2.2 footnote c-ub): `src/backends/c_fsm.rs` renders bitvector
  arithmetic as `({type})(((l) op (r)) & mask)` with NO widening cast on the
  operands. C integer promotion then applies:
  - `bit<8>`/`bit<16>` operands promote to `int`; `bit<32>` stays unsigned int;
    `bit<64>` stays uint64_t.
  - **Shifts:** `(l) << (uint64_t)(r)` is UB when `r >= width of promoted l`
    (>= 32 for bit<8..32> LHS, >= 64 for bit<64>). Verified on a scratch spec
    (`set window = window << k_in;`, `k_in: bit<8>`): Rust emits
    `if (b as u32) >= 8 { 0 } else { ... }`; C emits
    `(uint8_t)(((f->vars.window) << (uint64_t)(k_in)) & mask)` - UB for
    k_in >= 32, and on x86-64 gcc it effectively wraps the shift count mod 32,
    so e.g. `window=5, k_in=32` gives C=5 vs Rust=0.
  - **Multiplication:** two promoted `int` operands overflow signed when the
    product > INT_MAX (reachable for bit<16>: 50000 * 50000 = 2.5e9).
    Unsigned widths (bit<32>/bit<64>) wrap definedly, so only the cast is
    needed for uniformity.
  - **Not affected:** `+`/`-` (max promoted sum 131070 < INT_MAX), bitwise
    ops (no overflow), `~`/unary `-` (already ULL-based in C rendering).
- **Latency proof:** no valid example uses `*`; all variable shift amounts in
  examples are `bit<2>` (max 3) - so NO existing pin breaks from the finding
  itself, but the LOWERING CHANGE alters bytes of every C artifact containing
  `<<`/`>>`/`*`: examples 20_shifts, 44_token_ring, 45_elevator (shifts with
  literal and variable amounts) - plus their aggregate-crate modules.
- **RFS semantics to implement** (`docs/SEMANTICS.md` section 2.2): result =
  `(l op r) mod 2^w`; shift: `amount >= w -> 0`, logical (unsigned) shifts
  only. The Rust backend is the reference; the SMT encoder already implements
  `amount >= width -> 0` faithfully (DESIGN section 11, D-P4-12).
- **Parity principle** (INTRO.md): same observable behavior in ALL backends -
  UB violates this contract and Phase 6's "no invalid target-language code
  from valid specs" goal.
- Example numbering: 47/48/49 are reserved by task 6.1; this task uses
  **50** (`examples/valid/50_wide_ops.rfs`). After task 2 the count is
  46 valid / 13 invalid; after this task it is **47 valid / 13 invalid**
  (task 6.1's final re-pin becomes 50/13 - see plan correction row 6.1).

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 2. | Stability and versioning (v1.0) | task 2.3's header change lands first; this task's re-pin sweep then covers headers AND the new lowering in one pass, so no later task re-pins C arithmetic bytes |

### Affected source files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `src/backends/c_fsm.rs` | change | shift lowering: uint64-wide operands + `amount >= width -> 0` guard (uniform for literal and variable amounts); multiplication: uint64-wide operands before `*`; `+`/`-`/bitwise unchanged |

### Affected test files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `tests/wide_ops_parity_test.rs` | new | targeted cross-backend differential: generate production code for the wide-ops spec in BOTH backends, compile (rustc / gcc -Wall), drive identical input sequences hitting k >= width shifts and bit<16> products > INT_MAX, assert byte-identical per-step outputs; plus exact-MC status parity on the spec |
| `tests/examples_test.rs` | change | count re-pin 47 valid / 13 invalid |
| `tests/codegen_test.rs` | change | re-pin C byte assertions for shift-bearing examples (20/44/45) where pinned |

### Other affected files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `examples/valid/50_wide_ops.rfs` | new | small spec exercising variable wide shifts (`w << k`, `w >> k`, `k: bit<8>`) and a guarded bit<16> multiplication with product > INT_MAX; exact-MC-tractable (a few states, no liveness - keep it a safety property) |
| aggregate generated-code crate | regenerated | `cargo generate-code-tests && cargo test-generated-code` after the lowering change |

## Planned actions

### Action 1: C lowering fix
Change `gen_expr_c` in `src/backends/c_fsm.rs`:
- Shifts: render `(uint64_t)((r)) < W ? ((uint64_t)((l)) << (uint64_t)((r))) : (uint64_t)0` (and `>>`) masked to the result type - one uniform form for literal and variable amounts, matching Rust's `if amount >= width { 0 } else { ... }`.
- Multiplication: render `(type)(((uint64_t)((l)) * (uint64_t)((r))) & mask)`.
Rationale: uint64 is wide enough for every RFS width (<= 128 only exists in
the Rust backend - the C backend rejects widths outside {8,16,32,64}, so
uint64 intermediates cover all C-legal widths; document this in the comment).
`+`/`-` stay as-is (no overflow possible after promotion for width <= 16;
unsigned wrap defined for 32/64) - minimal byte churn.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/backends/c_fsm.rs` | change | the two renderings above; keep `gcc -Wall` zero-warning (check: no new "conversion" warnings from the explicit casts) |

### Action 2: example + tests
- Add `examples/valid/50_wide_ops.rfs`: one state, vars `w: bit<8> init 1`,
  `a: bit<16> init 50000`, `b: bit<16> init 50000`; event
  `Go(Value(k: bit<8>))`; transitions setting `w = w << k` / `w = w >> k` /
  `a = a * b` (split across a couple of states or guarded by k so the spec
  stays small and exact-MC-tractable); one safety property (e.g.
  `always (w >= 0)` style trivially-true atom is NOT allowed - use a real
  invariant like `always (b == 50000)` if b is never written... choose atoms
  that are actually checkable; the point is exercising monitors on this spec,
  not inventing semantics).
- Add `tests/wide_ops_parity_test.rs` per Scope: targeted sequences must
  include k = 8, 31, 32, 64, 255 (shift boundaries) and the wide multiply;
  assert per-step variable values byte-identical across backends.
- Re-pin `tests/examples_test.rs` (47/13) and any C byte pins in
  `tests/codegen_test.rs` for 20/44/45; regenerate + re-run the aggregate
generated-code crate (expect 1501+ tests green).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `examples/valid/50_wide_ops.rfs` | new | file-driven coverage of the fixed cases in ALL future sweeps |
| `tests/wide_ops_parity_test.rs` | new | deterministic regression pin for the exact divergent inputs (random fuzzing alone may not hit k >= 32 quickly) |
| `tests/examples_test.rs` | change | count re-pin 47/13 |
| `tests/codegen_test.rs` | change | C byte re-pins where shift/mul artifacts are asserted |

### Action 3: verification + docs sync
- Full checklist for a codegen change (`BUILD.md` section 5): build/test/
  clippy/fmt, `cargo generate-code-tests && cargo test-generated-code`,
  `examples_test`, gcc -Wall spot check of the new C output.
- `src/backends/C.md`: update the type-mapping/semantics note (shifts guarded,
multiplication uint64-wide) so the backend doc matches the rendering.
- `docs/SEMANTICS.md` footnote c-ub: mark the divergence RESOLVED by task 2a
  (keep the historical note; adjust the "Until then" sentence).
- Training snippets: run the extract/build/test-training trio; re-pin counts
  if the new example flows into snippet extraction.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/backends/C.md` | change | document the corrected lowering |
| `docs/SEMANTICS.md` | change | footnote c-ub: resolved status |
| `training/<cat>/snippets/*`, `all_rfs_training.jsonl` | change (if applicable) | regenerate if extraction picks up the new example |

## Task deliverables
- Well-defined generated C for shifts and multiplication, byte-parity with
  Rust on all inputs (targeted differential test green).
- `examples/valid/50_wide_ops.rfs` + `tests/wide_ops_parity_test.rs`.
- Re-pinned C artifacts / example counts; aggregate crate regenerated.
- `C.md` + `SEMANTICS.md` footnote updated.

## Task exit criteria
- [ ] Generated C for a variable shift renders the guarded uint64 form; no promotion-based UB remains for `*`/`<<`/`>>` (review the rendering in the generated artifact of 50_wide_ops).
- [ ] `tests/wide_ops_parity_test.rs` green: byte-identical per-step outputs across backends on the boundary sequences (k = 8/31/32/64/255, wide multiply); gcc -Wall zero warnings.
- [ ] `examples_test` re-pinned 47 valid / 13 invalid and green; aggregate generated-code crate regenerated and fully green.
- [ ] Full BUILD.md section 5 checklist green (build/test/clippy/fmt + generated-code tests).
- [ ] `C.md` and `docs/SEMANTICS.md` footnote c-ub reflect the fix.

## Non-goals
- No change to Rust lowering, SMT encoder, or MC engine (they already define
  the semantics).
- No change to `+`/`-`/bitwise C rendering (no defect there).
- Not a general C codegen rewrite - surgical two-expression fix.

## Other information
- If the uniform shift guard turns out to trip `gcc -Wall` (e.g. comparison
  always-false for literal amounts < width), allow a literal-only fast path:
  constant-fold literal amounts at codegen time (`amount >= width` -> emit
  `0`, else plain shift) and keep the runtime guard only for variable
  amounts - record as a task correction if taken.
- Re-pin sweep must be ONE pass (task 2.3's risk note applies here too):
  after this task, no later Phase 6 task may need to re-pin C arithmetic bytes.