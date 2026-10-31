<!-- llm-budget: 150 -->
# RFS Rust Backend (Class B)

Compact card for LLM assistants working on the Rust target-language backend.

<a id="llm:backend-rust"></a>
## Card

**Responsibility.** The Rust target-language backend: plugin registration
(`src/backends/rust.rs`) plus the three generators that render the shared
codegen plans into standalone Rust source - production FSM (`rust_fsm.rs`),
unit tests + fuzz skeleton + aggregate crate (`rust_test.rs`), model-checker
executable (`rust_mc.rs`). Backend parity with C is the critical constraint:
one plan, rendered identically in Rust and C.

**Key items.**

| Item | Role |
|---|---|
| `src/backends/rust.rs::RUST_BACKEND` | static plugin value registered in codegen::BACKENDS (3 command descriptors + 3 fn pointers); adding a backend = append ONE registry entry |
| `src/backends/rust.rs::rust_ident` / `rust_binding_ident` | keyword to r# mangling; pattern bindings colliding with identifiers the generated body uses unqualified are renamed `<name>__rfsb` (reserved sets: {input, handler} in the Fsm impl, {out, state, vars} in mc_step_all) |
| `src/backends/rust.rs::bit_rust_type` / `mask_expr` | bit<N> to u8..u128 + width mask; generated arithmetic is u128 wrapping + mask (DSL wraparound semantics) |
| `src/backends/rust_fsm.rs::generate_core` | core emitter shared by all three commands: re-validation, fixed emission order (prelude, enums, Vars, Input/Output, renderers, trace/handlers, monitors, Fsm impl, [ProtocolFsm], [tests]) |
| `src/backends/rust_fsm.rs::render_monitor` / `render_node` | online monitor structs from the shared MonitorPlan: DFS pre-order numbering, post-order evaluation; per-node-kind field sets (pending model for bounded obligations, prev_antecedent for delayed checks); step returns newly-confirmed failures |
| `src/backends/rust_fsm.rs::gen_expr` | expression lowering typed by validator::type_expr: u128 wrapping + mask to type width, shifts saturate at width, bool bitwise = logical - must mirror the planner's static evaluator EXACTLY |
| `src/backends/rust_fsm.rs::render_mc_step_all` | mc_step_all for the exact MC runtime: every enabled transition in firing order (deterministic = first effect only; nondet = all); on_error fallback with transition_id usize::MAX |
| `src/backends/rust_test.rs::emit_test_module` | renders the shared TestPlan into the seven stable test classes (initial_state_is_correct, positive_sequence_runs, negative_unexpected_input_search, t<id>_<state>_on_<event>[+_not_taken], invariant_<label>, monitor_<label>_fires_on_violation, random_trace_seed<seed>_steps<K>) |
| `src/backends/rust_test.rs::emit_fuzz_target` | SEPARATE fuzz_target.rs (libfuzzer_sys dep): first byte = variant selector, bool = one non-zero byte, bit<W> = ceil(W/8) LE bytes masked, enum = constant representative |
| `src/backends/rust_test.rs::generate_aggregate_test_crate` | one pub mod per module (m{:03}_<file>_<module>); whole-file spec hash; empty [workspace] table makes the generated crate its OWN workspace root |
| `src/backends/rust_mc.rs::generate_model_checker` | three shapes: standalone SMT z3 binary (shared smt_plan/smt_emit spec section + SMT_ENGINE_RUST), or FSM core + exact_codegen + FUZZ_RUNTIME + MAIN_DISPATCH |
| `src/backends/rust_mc.rs::exact_codegen` | embeds plan tables as runtime-overridable defaults: canonical all_inputs, config_bytes layout (matches ref_mc's CfgKey byte-for-byte; remaining packed only while pending), the engines |
| EXACT_ENGINE / EXACT_RUN (embedded) | level-synchronous BFS: visited at ENQUEUE time, parent pointers, firing-order successors; --threads N = parallel phase A + sequential canonical commit (byte-identical to --threads 1); stops at first violation (SEMANTICS 9.5); exit codes 0/1/2 |
| LIV_ENGINE (embedded) | liveness: per-leaf progress automata to limit vectors, product BFS, stay-edge SCCs (iterative Kosaraju), strong fairness in nondet mode, lasso witness; truncated analysis never a false pass-complete |
| JSON_WRITER_RUST (embedded) | minimal copy of the toolchain compact writer (u64); KEY ORDERING IS THE BYTE-PARITY CONTRACT (ce_steps last); escape contract pinned; peak_rss_kb informational (normalized to 0 in parity comparisons) |
| FUZZ_RUNTIME / SMT_ENGINE_RUST (embedded) | legacy fuzz engine (SAME LCG constants as codegen_test::Lcg, McConfig defaults, phase-ordered input selection); SMT: fixed variable names s_i/v_i_j/ev_i/fld_i_g, fixed constraint + query order, pinned summary lines |

**Invariants that must not break.**

- Parity: a new feature lands in EVERY backend in the same change + cross-backend differential tests (tests/backend_parity_test.rs, liveness_mc_test, benchmark suite); the C twin of each file is src/backends/c*.rs.
- Firing order everywhere = descending effective priority then source order - mirrors generated fsm_step AND the planner; the pattern-scope reserved sets exist for the shadowing bug class.
- gen_expr width semantics mirror the planner's static evaluator EXACTLY (u128 wrapping + mask, shift saturation, bool bitwise = logical) - drift makes planned cases disagree with runtime.
- Pinned formats: line-1 artifact header, test names + random-trace line, fuzz decode wire format + LCG constants/seed, config_bytes layout, exact summary line + statistics block, JSON key ordering + escape contract, SMT naming/order/witness format.
- Generated artifacts stay dependency-free and deterministic (declaration order everywhere); the fuzz skeleton stays a separate file; the aggregate crate keeps its empty [workspace] table.
- no_std: only the header may change - the body is byte-identical (pinned by the compile sweep over all 50 valid examples x both monitor modes).
- Generated binaries embed their own copies of plan tables / parsing helpers / JSON writer - format changes must update both backends' emitters in the same change (source of truth: src/codegen_mc.rs, src/mc_plan.rs).

**Change impact.** Placement rule first (CODEGEN.md section 2); verify with
cargo test --test codegen_test + backend_parity_test + liveness_mc_test +
benchmark_mc_test, then the full BUILD.md section 5 checklist. Full table:
llm:change-impact (task 13.2).

**Deep dive.** dev:backend-rust-overview + dev:backend-rust-plugin /
dev:backend-rust-fsm / dev:backend-rust-test / dev:backend-rust-mc in
docs/dev/backend_rust.md; artifact design: src/backends/RUST.md; extension
guide: CODEGEN.md (repo root); the shared plans it renders:
dev:codegen-core-overview et al. in docs/dev/codegen_core.md.
