<!-- llm-budget: 150 -->
# RFS C Backend (Class B)

Compact card for LLM assistants working on the C target-language backend.

<a id="llm:backend-c"></a>
## Card

**Responsibility.** The C target-language backend: plugin registration
(`src/backends/c.rs`) plus the three generators that render the shared
codegen plans into self-contained C source - production FSM (`c_fsm.rs`),
unit tests + fuzz skeleton (`c_test.rs`), model-checker executable
(`c_mc.rs`). Backend parity with Rust is the critical constraint: one plan,
rendered identically in Rust and C.

**Key items.**

| Item | Role |
|---|---|
| `src/backends/c.rs::C_BACKEND` | static plugin value registered in codegen::BACKENDS (3 command descriptors + 3 fn pointers); adding a backend = append ONE registry entry |
| `src/backends/c.rs::c_ident` / `c_binding_ident` | keyword to c_ prefix; pattern bindings colliding with identifiers the generated body uses unqualified are renamed `<name>__rfsb` (reserved sets: {f, in, handler, user} in transition blocks, {state, vars, out, n} in mc_step_all) |
| `src/backends/c.rs::bit_c_type` / `c_type` | bit<N> to uint8_t/16/32/64 ONLY (D8 - other widths are a codegen error; Rust falls back to u128); enum E to E_t typedef |
| `src/backends/c.rs::int_c_literal` / `mask_c_expr` | in-range literals (`(v & mask)u` / `(v)ULL`) + the width masks that wrap every arithmetic/bitwise result |
| `src/backends/c_fsm.rs::generate_core` | core emitter shared by all three commands: up-front D8 width rejection, fixed emission order (header, types, monitors, Fsm impl); test/mc pass Always, production maps --with-property-monitors to Gated/Off |
| `src/backends/c_fsm.rs::emit_types` | tagged unions for inputs/outputs (payload struct + kind enum + union wrapper), MAX_OUTPUTS = max emit count, trace/status/policy types (rfs_status_t, step_result_t, rfs_assumption_handler) |
| `src/backends/c_fsm.rs::gen_expr_c` | expression lowering typed by validator::type_expr: * in a uint64 intermediate, results masked to the type width, shifts widen both operands with an `amount >= w -> 0` guard - must mirror the Rust backend AND the planner's static evaluator EXACTLY |
| `src/backends/c_fsm.rs::render_monitor` / `render_node` | online monitor structs from the shared MonitorPlan: DFS pre-order numbering, post-order evaluation; per-node-kind field sets (pending model for bounded obligations, prev_antecedent for delayed checks); step returns newly-confirmed failures |
| `src/backends/c_fsm.rs::render_signal_expr_c` + match conds | prefix-parameterized sample-signal machinery shared with the c_test invariant tests and the c_mc liveness evaluator (monitor rendering stays byte-identical) |
| `src/backends/c_fsm.rs::emit_fsm_impl` | fsm_t + fsm_new (sample-0 stepping: assumption violation aborts, property violation sticky) + fsm_free + rfs_commit (trace push -> assume handler policy -> first-violation-wins prop check -> state/vars update); fsm_step fires per state in descending-priority stable order; on_error is the final catch-all |
| `src/backends/c_fsm.rs::transition_block_c` | pattern match -> bindings as locals (c_binding_ident) -> guard -> actions; SIMULTANEOUS updates via tmp_<var> from pre-step values; unused bindings get a (void) cast |
| `src/backends/c_fsm.rs::render_mc_step_all_c` | mc_step_all for the exact MC runtime: every enabled transition in firing order (deterministic = first effect only; nondet = all); on_error fallback with transition_id = -1 |
| `src/backends/c_test.rs::generate_unit_tests` | production core + table-driven main() rendering the shared TestPlan: 3 module-level + 4 extended classes (t<id>_<state>_on_<event>[+_not_taken], invariant_<label>, monitor_<label>_fires_on_violation, random_trace_seed<seed>_steps<K>), one PASS/FAIL line per test; rejects no_std |
| `src/backends/c_test.rs::emit_fuzz_target_c` | SEPARATE fuzz_target.c (libFuzzer): single-exit decode (truncated field runs / unknown variants set done and break); calls fsm_free once per input |
| `src/backends/c_mc.rs::generate_model_checker` | two shapes: standalone SMT libz3 binary (shared smt_plan/smt_emit spec section + SMT_ENGINE_C), or FSM core + exact_codegen_c + EXACT_ENGINE_C / LIV_ENGINE_C / JSON_WRITER_C / EXACT_RUN_C / MAIN_DISPATCH_C; rejects no_std + --with-protocol-fsm |
| EXACT_ENGINE_C / EXACT_RUN_C (embedded) | mirror of the Rust exact engine: level-synchronous BFS over packed configs, sorted-vector visited set, pthreads two-phase expansion byte-identical to --threads 1, pinned summary line, exit codes 0/1/2 |
| LIV_ENGINE_C / JSON_WRITER_C (embedded) | liveness mirror (per-leaf progress automata, limit vectors, lasso witness, strong fairness); embedded compact JSON writer - KEY ORDERING IS THE BYTE-PARITY CONTRACT (ce_steps last), peak_rss_kb via getrusage informational |

**Invariants that must not break.**

- Parity: a new feature lands in EVERY backend in the same change + cross-backend differential tests (tests/backend_parity_test.rs, prod_parity_test, wide_ops_parity_test); the Rust twin of each file is src/backends/rust*.rs.
- Firing order everywhere = descending effective priority then source order - mirrors generated fsm_step AND the planner; the pattern-scope reserved sets exist for the shadowing bug class.
- gen_expr_c width semantics mirror the Rust backend and the planner's static evaluator EXACTLY (uint64 intermediate for *, mask to type width, shift saturation) - drift makes planned cases disagree with runtime.
- D8: bit widths outside {8,16,32,64} stay a codegen error (up-front rejection in Gen::new).
- Pinned formats: line-1 artifact header, MAX_OUTPUTS + step_result_t layout, mon_* naming via rust::snake, test names + random-trace line, fuzz decode wire format + LCG constants/seed, config_bytes layout, exact summary line + statistics block, JSON key ordering + escape contract.
- Generated artifacts stay dependency-free (the five standard includes) and deterministic; gcc -Wall at ZERO warnings on every artifact over all valid examples; the fuzz skeleton stays a separate file.
- no_std / protocol-fsm no-ops: --no-std adds only its one comment line; --with-protocol-fsm is ignored by production/test, rejected by mc with the pinned error.

**Change impact.** Placement rule first (CODEGEN.md section 2); verify with
cargo test --test codegen_test + backend_parity_test + prod_parity_test +
wide_ops_parity_test, then the full BUILD.md section 5 checklist. Full table:
llm:change-impact (task 13.2).

**Deep dive.** dev:backend-c-overview + dev:backend-c-plugin /
dev:backend-c-fsm in docs/dev/backend_c.md (dev:backend-c-mc /
dev:backend-c-test are task 10.2); artifact design: src/backends/C.md;
extension guide: CODEGEN.md (repo root); the shared plans it renders:
dev:codegen-core-overview et al. in docs/dev/codegen_core.md; Rust twin:
docs/dev/backend_rust.md.
