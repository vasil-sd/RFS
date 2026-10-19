# Phase 4, Task 11 - Status

**State:** COMPLETE (gate outcome A: implemented) - all exit criteria met
**Date:** 2026-10-19

## Feasibility gate (D-P4-12)
PASSED. z3 **5.3.1** verified at `/workspace/host/z3` (`z3 --version` + sanity query); scratch cargo project outside the tree built `z3 = "0.21"` (resolved 0.21.1 / z3-sys 0.13.1) with `Z3_LIBRARY_PATH_OVERRIDE=/workspace/host/z3/bin` + `Z3_SYS_Z3_VERSION=5.3.1` and solved a bitvector query correctly. No reinstall needed - TOOLCHAIN.md section 5 status line unchanged.

## Files added / changed / removed
- `Cargo.toml` (changed): `[features] smt = ["dep:z3"]` + optional `z3 = "0.21"` dep. Default build unaffected (verified: full suite green without the feature).
- `src/smt_plan.rs` (new, from earlier task-11 session): backend-neutral `SmtSpec` (states/vars/classes/transitions/properties with resolved indices) + IR->spec conversion (`build_smt_spec`) with shared v1-scope rejections: no assumptions, no on_error blocks, no enum-typed vars; properties = top-level `always`/`eventually` over boolean combinations of expressions (no input/output refs, no nested temporal ops).
- `src/smt_emit.rs` (new + this session): shared emitter rendering one `SmtSpec` into BOTH backends' spec sections (constants + per-expression builder functions + function-pointer tables + ASGN_MAP). This session: full expression renderer rewrite (`render_se_rust`/`render_se_c` with `bool_as_bv` contexts), corrected shift encoding, comparison widening, and 5 unit tests incl. a structural Rust-vs-C op-sequence parity test.
- `src/backends/rust_mc.rs` (changed): `--mode smt-bounded` branch in `generate_model_checker` + `SMT_ENGINE_RUST` runtime (step variables with fixed names, k-step unrolling, first-match firing, ITE next-state/next-var chains, alive predicate, safety/liveness queries, pinned summary lines, exit codes 0/1/2).
- `src/backends/c_mc.rs` (changed): mirror branch + `SMT_ENGINE_C` - same constraint construction order and z3 operations in the C API; compiles warning-free under `gcc -Wall`.
- `tests/smt_bounded_test.rs` (new, feature-gated): e2e parity gate - 3 inline wide-bitvector specs (safety FAIL at step 0 / liveness cycle / PASS) x both backends: gcc -Wall compile (zero warnings), offline scratch-cargo Rust build under `target/`, byte-identical stdout + exit codes, pinned summary lines; plus runtime `--bound` override check.
- `docs/roadmap/PHASE_4_TASK_11_STATUS.md` (new): this file.

## Design notes / deviations
- **Parity by construction (D-P4-12):** both backends render the SAME `SmtSpec` with the SAME traversal and the SAME z3 operations; step variables are z3 CONSTANTS with fixed names (`s_<i>`, `v_<i>_<j>`, `ev_<i>`, `fld_<i>_<g>`) - Rust builders recreate them by name (hash-consing returns the same node), C builders receive them as pointers. The unit test normalizes every builder function to an op-token sequence and asserts Rust == C.
- **Bool vars are BV(1) leaves** (witness display uses `VAR_IS_BOOL`); z3 Bool results arise from `eq(v, 1)`; assignment top level wraps bool results as `ite(b, 1, 0)` (`bool_as_bv`) since targets are BVs.
- **Shift semantics match exact MC exactly** (`amount >= wl -> 0`): the amount is zero-extended to `max(amount width, wl)` (concat), guarded by `amount < wl` (ite), and truncated to `wl` bits (extract) for the z3 shift op - lossless under the guard. Oversized literal amounts keep enough bits to stay detectable (`shift_amt_width`).
- **Comparisons see FULL literal values** (exact-MC casts both sides to u128): if a literal does not fit the sibling's width, both sides widen (non-literal side zero-extended). Arithmetic/bitwise ops need no widening - truncating the literal is equivalent there (ring homomorphism).
- **z3 5.3.1 API deltas** vs newer versions: `Z3_L_UNDEF` (no `Z3_L_UNKNOWN`; FALSE=-1, UNDEF=0, TRUE=1), no `Z3_del_solver` (solvers die with the context), `Z3_del_context`, `Z3_model_eval` returns bool + out-ast, `Z3_get_bool_value` returns lbool.
- **v1 scope rejections are shared** (same error text both backends): assumptions, on_error blocks, enum-typed vars, property formulas beyond the boolean-over-expressions subset.

## Verification (all in final state)
- Default build WITHOUT the feature: `cargo build && cargo test` **227/227** (was 222 after task 10B; +5 new unit tests); clippy at the 11 lib + 1 codegen_test baseline (zero from this task); fmt clean.
- WITH `--features smt`: **229/229** (+2 integration tests, ~4 s).
- **E2E backend parity sweep:** all 31 assumption-free valid examples -> 22 SMT-compatible specs generated for BOTH backends, compiled (gcc -Wall zero warnings; offline cargo), run at bound 3: **byte-identical stdout + exit codes on all 22** (0 FAILs). The 9 skips are the documented v1 limitations (input_is/next properties x5, enum-typed vars x3, on_error blocks x2 - one spec hits two).
- Wide-bitvector demo: stress spec with bit<16> var + field, shifts (literal/narrow/wide amounts), oversized-literal comparison runs identically in both backends; SMT-found safety violation (`flag_or_counter`, step 0) and liveness violation (`reach_b`) are BOTH also found by exact mode on the same specs (cross-check).
- Incomplete labeling present on every PASS: `PASS WITHIN BOUND k (incomplete - no completeness claim)` + `status=pass-within-bound`; sound FAIL / unknown only, never a completeness claim.

## Bugs found and fixed during development
1. **Shift amount width mismatch (NULL from z3):** literal amounts rendered at default width 8 regardless of the left operand's width; narrow bitvector amounts were "truncated" instead of zero-extended. Fixed with the unified extend/guard/truncate encoding above (both backends).
2. **Oversized-literal comparison wrap:** `counter < 256` on a bit<8> rendered the literal as `from_u64(256, 8)` = 0 -> property always false (spurious FAIL at step 0). Fixed with comparison widening (both backends); unit-test-pinned.
3. **Double-widening of literal sides** in the first widening draft (literal rendered at final width AND extended) - caught by the generated-code compile; fixed (`ext = 0` for literal sides).
4. **z3 5.3.1 API mismatches** in the C engine draft: `Z3_L_UNKNOWN`/`Z3_del_solver`/`Z3_close_context` do not exist in 5.3.1 - compile errors caught on first gcc run; fixed per the header.
5. **Test-harness race:** the two integration tests shared one scratch cargo project and overwrote each other's `main.rs` (parallel test threads) - one test ran the other spec's binary; fixed with per-test project directories.

## Findings / notes for later tasks
- **For task 13 (docs sync):** document `--mode smt-bounded` + runtime `--bound N` in CODEGEN/DESIGN: k-step unrolling over bitvectors, first-match firing, k-liveness cycle encoding, exit codes 0 pass-within-bound / 1 fail / 2 unknown; the v1 scope rejections (assumptions, on_error, enum vars, property subset); env requirements (TOOLCHAIN.md section 5.2/5.3) for building generated SMT binaries; "incomplete - no completeness claim" labeling.
- **For task 14 (close-out):** full verification = `cargo test` at **227** default / **229** with `--features smt`; the parity gate is `tests/smt_bounded_test.rs` (byte-identity across backends on 3 specs) plus the 22-spec sweep methodology recorded above.
- Possible future work (NOT in scope): assumptions in SMT mode, `input_is`/`next` in property formulas, enum-typed vars, solver tuning (tactics/timeouts), and a completeness-oriented k-induction step.

## Exit criteria
- [x] Default build (`cargo build && cargo test && cargo clippy --all-targets && cargo fmt --check`) green WITHOUT the smt feature - behavior unchanged (227/227; clippy/fmt at baseline).
- [x] With the feature + TOOLCHAIN.md 5.2 env vars: SMT mode runs on a demo large-bitvector spec in BOTH backends with identical results (byte-identical stdout, 22-spec sweep + pinned integration specs); incomplete labeling present; sound FAIL / unknown only (no completeness claims).
- [x] Cross-check: every SMT-found violation is also found by exact mode on small specs (safety step-0 violation + liveness cycle, both verified).
