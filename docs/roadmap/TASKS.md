
## Phase 4 - Scalable model checking (task 4 of 14)

### Task 4: Parity suite migration — COMPLETE (2026-10-18)
- `tests/backend_parity_test.rs` + `tests/monitor_integration_test.rs` migrated from same-seed fuzz to seedless exact mode (D-P4-14); values re-pinned from actual deterministic output.
- Re-pin: 6 pass-complete / 9 fail across all 15 property-bearing examples - 8 NEW deadlock findings (terminal states the fuzz walk never reported), 1 property violation (`13_properties_safety`/`no_bad`, step 1). `41_property_monitors` now reports its depth-3 deadlock before any property violation.
- Correct-spec test reinterpreted: exact mode explores the full graph, so example 41's terminal state is a legitimate FAIL(deadlock); the no-false-positive claim is preserved via reason + absence of property violations.
- Verification: cargo test 184/184; field-identity Rust vs C on all 15 examples; no fuzz assertions remain in either suite; clippy/fmt at baseline.
- Details: `docs/roadmap/PHASE_4_TASK_4_STATUS.md` (full old->new diff table).

## Phase 4 - Scalable model checking (task 5 of 14)

### Task 5: Liveness language extension — COMPLETE (2026-10-18)
- `eventually (p)` primary + infix `until` (new level between unary and primary, left-associative) across parser/IR/printer/validator; additive IR variants, no IR_VERSION bump (old JSON deserializes - test-pinned).
- Precedence pinned by tests: `a && b until c` = `a && (b until c)`; `a until b until c` left-assoc; `!a until b` = `(!a) until b` (`!` tighter than `until`, standard PSL/CTL); compound operands need parens; `until` never shadows identifiers in expression contexts.
- Monitor boundary: any formula containing eventually/until is outside the subset -> identical skip warning in both backends' production codegen (exact text test-pinned).
- Task correction: the task file's EBNF block contradicted its own pinned consequence for `!a until b`; implemented per the pinned consequence/ladder (negation below the until level) - see "Task corrections" in PHASE_4_TASK_5.md. OPEN QUESTION resolved with the default (pinned production).
- Verification: cargo test 201/201 (was 184, +17 new); examples_test unchanged 42 valid / 12 invalid; clippy at 12-warning baseline (zero from this task); fmt clean.
- Details: `docs/roadmap/PHASE_4_TASK_5_STATUS.md`.

## Phase 4 - Scalable model checking (task 6 of 14)

### Task 6: Liveness engine - Rust + first liveness example — COMPLETE (2026-10-19)
- Per-obligation bad-set analysis on the completed reachable graph (D-P4-10): per-leaf progress automata (safety ok/bad, eventually wait/done, until ok/sat/bad), product BFS + stay-edge SCC limit-vector enumeration + top-level skeleton evaluation; edge-based labels so `input_is`/`output_has` inside obligation bodies depend on the edge taken.
- **Plan correction (D-P4-10):** the text's weak-until equation (backward reach to gfp(¬q∧¬p)) is the STRONG-until one; it diverges from D-P4-8 semantics on 185/1200 random tiny graphs. Implemented corrected equation: backward reachability staying in ¬q to {pos: ¬p(pos)}. Verified against brute-force lasso enumeration: 1200 equation checks + 2000 full-algorithm checks, 0 mismatches; hand-computed graphs H1-H4 / edge cases E1-E5 recorded.
- Counterexamples: lasso (prefix + cycle) in the Phase 3 trace format; summary line `reason=liveness-violation label=<L> step=<cycle start>`; human fields formula/violation step/cycle length/note (per-falsified-leaf explanation).
- Bounded mode: truncated analysis on the explored subgraph - sound FAIL when a witness lies inside it, else `unknown-state-limit`; never a false pass-complete (both directions test-pinned).
- `--nondet` strong transition fairness (D-P4-11): bad cycle must be fair = every enabled successor of each cycle state is taken on the cycle -> stay-edge restriction to closed, fully expanded configs. No plan correction needed; discriminator test-pinned (det FAIL vs nondet pass-complete).
- Deeper nesting / non-boolean operands -> codegen diagnostic (property skipped, binary still compiles/runs); liveness diagnostics surface as codegen warnings.
- Example 43 (`examples/valid/43_sliding_window.rfs`): sliding-window ARQ sender (window 4 over seq 0..8), Tick-driven + out-of-band `Ack(Seq(seq: bit<4>, checksum: bit<32>))` with both payload fields unused -> input abstraction collapses to ONE representative Ack (spot-checked in generated `all_inputs()`). Safety `always (next_seq - acked <= 4)` + liveness `eventually (state == Done)` under the timing assumption + `(next_seq - acked <= 4) until (state == Done)`. Exact MC -> **pass-complete, states=165 transitions=330 max_depth=22** (pinned in the parity suite; pass/fail split re-pinned 6/9 -> 7/9).
- Verification: cargo test 213/213 (was 204, +9 new liveness integration tests); all pre-existing pins unchanged (BFS edge recording purely additive; safety-only specs never enter the liveness path); clippy at 12-warning baseline; fmt clean.
- Details: `docs/roadmap/PHASE_4_TASK_6_STATUS.md`.

## Phase 4 - Scalable model checking (task 7 of 14)

### Task 7: Liveness engine - C + parity coverage — COMPLETE (2026-10-19)
- `src/backends/c_mc.rs`: the C liveness engine (mirror of task 6's Rust `LIV_ENGINE`): per-leaf progress automata, product BFS + stay-edge SCC limit vectors, skeleton evaluation, lasso witness reconstruction, `--nondet` strong-fairness filter, truncated-mode soundness. Same embedded plan tables as Rust; canonical witness selection mirrored so counterexamples match byte-for-byte.
- **Pre-existing format gap found and closed:** the C `print_trace` (since task 3) was a raw placeholder (`state=0 vars_raw`, `input_kind=`...); the old parity suite compared only summary fields, never trace bodies. Rewrote `shared_printers` to emit byte-identical Rust Debug-format printers (state/input/output name lookups generated per module, `Vars { ... }`, `[...]` output lists) + aligned the sample-0 property-violation report with Rust's dedicated `(initial configuration)` format. FAIL traces are now identical across backends for every counterexample shape (property violation, deadlock, liveness).
- **C engine bug found during parity:** the first draft shared one edge-label cache across all properties; label bit positions depend on the property's leaves, so a shared cache mislabels edges from the second property on. Fixed: per-property cache allocation (Rust engine was already correct).
- `src/backends/c_fsm.rs`: unused pattern bindings now get a `(void)` cast in generated C (gcc -Wall unused-variable fix for specs whose bindings no guard/output/update references).
- `tests/backend_parity_test.rs`: 4 new liveness parity tests - example 43 exact-mode byte-identical stdout (pass-complete, no counterexample); 10 inline variants (same specs as `liveness_mc_test.rs`: pass, eventually/until/skeleton FAILs, input_is edge labels, --nondet det-vs-fair, truncated fail/unknown, safety precedence) with exit code + full stdout byte-identity; pinned lasso fields on the eventually-FAIL variant (step=0, cycle length 2); truncated-mode soundness (sound FAIL / UNKNOWN, never false pass-complete). `compile_c` now asserts zero gcc warnings (was: exit status only), enforcing the `-Wall` bar for every C binary the suite builds.
- Verification: cargo test **217/217** (was 213; +4 new); clippy at the 12-warning HEAD baseline (zero from this task - verified by stash-diff); fmt clean; one-shot check: all 43 generated C MC binaries compile with `gcc -Wall` at zero warnings.
- Details: `docs/roadmap/PHASE_4_TASK_7_STATUS.md`.
