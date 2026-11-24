# RFS v1.0 - Validator Rules & Restrictions Audit

> Date: 2026-11-24. Scope: every rule the toolchain enforces that limits the
> expressible RFS subset - parser-level syntax rejections, validator semantic
> errors, monitor-subset restrictions (R1-R4), exact-mode MC planner
> rejections, smt-bounded mode rejections, and the C-backend width limit.
> Method: read of `src/lang/validator.rs` (all 1239 lines), `src/lang/parser.rs`
> (pattern/action/module builders), `src/codegen/monitor.rs` (R1-R4),
> `src/mc/plan.rs` + `src/mc/ref_mc_exact.rs` (exact planner),
> `src/mc/smt_plan.rs` + `src/mc/ref_mc_smt.rs` (SMT planner/engine),
> `src/codegen/backends/c/fsm.rs` (widths), cross-checked against
> `DESIGN.md` sections 4-5 + 21, `KB.md` section 7, and `docs/SEMANTICS.md`.
> Every "obsolete" claim below was verified empirically with the built binary
> (`rfs validate` / `rfs mc-ref --mode exact|smt-bounded`) before being made.
>
> Rationale for this audit: validator rules and restrictions have not been
> reviewed since they were written (most in Phases 1-4). Several now block
> syntax the downstream machinery already handles, i.e. they are obsolete and
> can be removed cheaply. The design decisions about the supported subset of
> DSL syntax/semantics are re-reviewed here against the current state of the
> project.

## Effort / impact legend

- **Removing effort** - `OBSOLETE`: the downstream machinery already copes with
  the general case; removal is a small deletion (check + its tests/docs), no
  semantic design needed. `MODERATE`: real refactoring across one or more
  backends, or a small semantic decision to make first. `HARD`: a semantic
  necessity (name resolution, arity, typing, determinism) or a large
  language/algorithm extension; not a removal candidate.
- **MC impact** - the cost of *lifting* the restriction (more specs accepted):
  `NONE` / `MINOR` / `MODERATE` / `HUGE`, stated separately for exact mode and
  smt-bounded mode because the two engines encode formulas differently.

## Table 1 - All validator rules & restrictions (by enforcement layer)

| # | Rule / restriction | RFS example (rejected form) | Enforced in | Description & rationale |
|---|---|---|---|---|
| **Parser layer** (won't parse) |||||
| P1 | Anonymous payloads | `event E(x: bit<8>)` | `parser.rs::opt_constructor_payload` | Variant name must be written: `event E(E(x: bit<8>))`. Some natural syntax rejected (KB section 7). |
| P2 | Transition body shape | `transition A on E { next B; next C; }` / duplicate `set x` / duplicate `priority` | `parser.rs::action` + `build_transition` | Exactly one `next` (required); no duplicate `set var`; no duplicate `priority`. Semantic necessity. |
| P3 | `on_error` actions | `on_error { set x = 0; next A; }` | `parser.rs::on_error_decl` | Only `next`+`emit`; no `set`; `next` mandatory; at most one per module. |
| P4 | Module name uniqueness | two `prop p1 : ...` / dup enum / dup var / dup initial / dup variant in a merged group | `parser.rs::build_module` + `merge_group` | Name-resolution safety; cross-list prop/assume collision rejected at parse. |
| P5 | `eventually` parens | `eventually p` | `parser.rs` formula | Body must be parenthesized: `eventually (p)`. Precedence disambiguation. |
| P6 | No division | `x / y` | grammar | Division is not in the expression language (DESIGN section 5.9). |
| P7 | Path length <= 2 | `a.b.c` | `validator.rs::resolve_path`, `ref_mc.rs` | Max `Enum::Variant`; no record field access. |
| **Validator layer** (semantic errors) |||||
| V1 | State enum non-empty | `module m { initial A }` (no `enum State`) | `validate_module` | Finite-state requirement. |
| V2 | Initial state is a variant | `initial Nowhere` | `validate_module` | Must name a `State` variant. |
| V3 | Transitions need an initial state | transitions present, no `initial` | `validate_module` | Exploration needs a start configuration. |
| V4 | Version header format | `version "1.x"` | `validate_module` (`is_valid_version_format`) | D-P6-1 provenance: `X.Y` or `X.Y.Z`. Legacy (absent) still valid. |
| V5 | Var init typing | `var x: bit<8> init true` | `validate_module` -> `type_expr` | Initializer must type-check. |
| V6 | Transition states are variants | `transition Nowhere on E { ... }` | `validate_module` | `state`/`next_state` must be `State` variants. |
| V7 | Event pattern structure | `on Recv(Ack(1,2))` (Ack has 1 field) / unknown group or variant / unit<->struct mismatch / duplicate binding | `validate_event_pattern` | Positional arity + name checks. |
| V8a | Int literal fits bit<N> | `on E(V(300))` with `V(x: bit<8>)` | `pattern_expr_compatible` | "literal 300 does not fit bit<8>". |
| V8b | Literal/type cross-matches | `on E(V(1))` on a bool field; `on E(V(1))` on an enum field; `on E(V(true))` on a bit field; `on E(V(Color::Red))` on a `Mode` field | `pattern_expr_compatible` | No cross-type literal matching; an enum path must name the field's enum. |
| V9 | Guard is bool | `where x` (x: bit<8>) | `validate_module` -> `must_bool` | Guards are boolean. |
| V10 | Set a known variable | `set y = 0` (no var y) | `validate_module` | Assignment target must exist. |
| V11a | Output structure | unknown group/variant; arg-count mismatch; arg type mismatch | `validate_output_expr` | Arity + typing of emit args. |
| V11b | Bare output ambiguity | `emit O` where `O` has 2 variants | `validate_output_expr` | "bare output 'O' is ambiguous; name a variant". Single-variant groups may be bare. |
| V12 | Duplicate priority, same pattern | two `priority 5` on an identical pattern in one state | `validate_module` | Error only when the patterns are identical; different patterns may reuse numbers. |
| V13 | Shadowed duplicate transition | two transitions, same pattern + same guard, no priority | `validate_module` | The second is unreachable -> hard error. |
| V14 | on_error next is a variant | `on_error { next Nowhere; }` | `validate_module` | Fallback target must be a `State` variant. |
| V15 | Dup prop/assume name (IR) | JSON-loaded IR with duplicate names | `validate_module` | Catches IR that bypasses the parser pass. |
| V16 | Formula expr is bool | `prop p : x ;` (x: bit<8>) | `type_formula` | Atomic formula expressions must be boolean. |
| V17 | **Named fields in formula input patterns** | `input_is(Recv(Ack(x)))` | `check_pattern_group` | "formula input patterns do not bind variables in Phase 1". **Asymmetric**: `output_has(Send(SynAck(y)))` is already allowed. |
| V18 | `always[N]` N >= 1 | `always[0] (p)` | `bounded_always_error` + monitor | D-P3-2. `bounded_eventually[0]` stays legal ("hold at the trigger sample"). |
| **Monitor subset R1-R4** (production monitors **+** exact-mode MC monitor) |||||
| M1 | R1: no `be` under negation | `!bounded_eventually[5] (p)` | `codegen/monitor.rs`, `ref_mc.rs::compile_check` | Negating a temporal formula is unbounded liveness -> not monitorable. |
| M2 | R2: `be[N]` body temporal-free + placement | `next (bounded_eventually[3] (p))`; a `be` under another `be` | same | Body must be signal-level; only top-level or inside `always`/`always[N]`. Trigger = the implication antecedent. |
| M3 | R3: `always[N]` not under temporal | `always (always[2] (p))` | same | Only top-level or under boolean connectives. |
| M4 | R4: no `next` in a `be` body | `bounded_eventually[3] (next p)` | same | A delayed body inside an obligation is unsupported. |
| M5 | `eventually`/`until` outside the monitor subset | `prop p : eventually (flag)` | same | MC-only (D-P4-8/9); every backend's codegen skips it with an identical warning. |
| **Exact-mode MC planner** |||||
| E1 | Liveness body boolean-over-atoms | `eventually (always (p))` | `plan.rs::decompose_top` | "no temporal operators inside `eventually`/`until` operands". |
| E2 | No liveness under temporal | `always (eventually p)` | `plan.rs::decompose_top` | "liveness operators nested under a temporal operator are not supported (deeper nesting)". |
| E3 | Split safety from liveness | a liveness prop with a stray `next`/`always` | `plan.rs::decompose_top` | "temporal operators are not supported in liveness properties; split the safety part". |
| E4 | Var init is a constant expr | `var x: bit<8> init y + 1` (refs another var) | `ref_mc_exact.rs` | May use literals + state variants, not other vars. |
| **SMT-bounded mode** |||||
| S1 | Var init is a literal | `var x: bit<8> init 5 + 1` | `smt_plan.rs::build_smt_spec` | "requires literal initial values" - *stricter* than exact mode (which folds constants). |
| S2 | Concrete payload <= 64 bits | a constraining pattern on a `bit<128>` field | `smt_plan.rs` | "does not support concrete payload fields wider than 64 bits". |
| S3 | Power-of-two domains | (unreachable today) | `smt_plan.rs` | "non-power-of-two domain" - only `bit<N>` yields Int domains, always 2^N-1. **Dead code.** |
| S4 | Top-level property form | `prop p : next (state == A) ;` / a bare boolean prop | `smt_plan.rs` | Only `always`/`always[N]`/`eventually`/`bounded_eventually`/`until`. Exact mode accepts bare/`next` top forms. |
| S5 | `--full-properties` gate | top-level `until` / nested temporals | `ref_mc_smt.rs` | Omitted by default: "greatly affect solver time/memory at large bounds (O(k*N) formula growth)". A performance guardrail, not a language rule. |
| **C backend** |||||
| C1 | Bit widths 8/16/32/64 | `var x: bit<128>` | `codegen/backends/c/fsm.rs` | D8 hard limit; Rust keeps u128 intermediates. |

## Table 2 - Audit: removal effort x MC-performance impact

| Rule / restriction | Removing effort | Impact on exact-mode MC | Impact on smt-bounded MC | Verdict / notes |
|---|---|---|---|---|
| **V17** named fields in formula `input_is` | **OBSOLETE** | MINOR (binds already = wildcards; no constraint added) | MINOR (same) | **Remove.** Delete the `check_pattern_group` check + its tests/docs. Closes the input/output asymmetry. Both MC backends already treat a `Bind` pattern field as a no-constraint wildcard (`ref_mc.rs::input_matches` -> `Bind {..} => true`; `smt_plan.rs::resolve_input_is` -> `Wildcard \| Bind => {}`). Verified: `output_has(Send(SynAck(y)))` validates and runs in both modes today. |
| **S3** power-of-two domain check | **OBSOLETE** | NONE | NONE | **Remove.** Dead code for the current type system (`value_domain` only emits Int domains from `bit<N>`, always 2^N-1). Delete the check. |
| **V4** version-header format | OBSOLETE (cheap) | NONE | NONE | Keep - deliberate D-P6-1 provenance formalization, not a stale subset restriction. Listed for completeness. |
| **V12** dup-priority-same-pattern error | OBSOLETE (cheap) | NONE | NONE | Keep - a determinism guard (D1-D5), not an expressiveness restriction. Listed for completeness. |
| **V13** shadowed-duplicate error | OBSOLETE (cheap) | NONE | NONE | Keep - an unreachability diagnostic, not an expressiveness restriction. Listed for completeness. |
| **S4** SMT top-level property form | MODERATE (cheap half) | NONE | NONE | **Remove.** Desugar a bare `p` / `next p` top form to `always(p)` / `always(next p)` to match exact-mode semantics. Removes a real cross-mode asymmetry. |
| **E4 / S1** var-init constant vs literal | MODERATE (cheap half) | NONE (one-time init) | NONE | **Unify.** Evaluate initializers symbolically at step 0 / declaration order so SMT accepts the same constant expressions exact mode already folds. SMT is needlessly stricter. |
| **V18** `always[N]` N >= 1 | MODERATE | MINOR (`always[0]` = a single-sample check) | MINOR | Defer - must fix the `to: Some(n-1)` underflow in the monitor + both MC paths and decide `always[0]` semantics first. Not cheap. |
| **P5** `eventually` parens | MODERATE | NONE | NONE | Defer - grammar/precedence interaction (`eventually p && q` is ambiguous). |
| **P1** anonymous payloads | MODERATE | NONE | NONE | Defer - parser + IR flattening + codegen naming. Syntax sugar only. |
| **P3** no `set` in `on_error` | MODERATE | MINOR (fallback gains var updates) | MINOR | Defer - thread assignments through the error path in all backends + MC. |
| **V8a** int-fits-bit<N> | MODERATE | MINOR | MINOR | Defer - requires defining truncation/wrap semantics for pattern literals everywhere. |
| **P6** no division | MODERATE-HARD | MINOR-MODERATE | MINOR | Defer - needs total div-by-zero semantics + BV division in Rust/C/SMT encodings. |
| **S2** payload <= 64 bits | MODERATE | NONE | MINOR | Defer - multi-word BV encoding + witness decoding. z3 handles wide BVs fine. |
| **C1** C widths 8/16/32/64 | MODERATE-HARD | NONE (MC is not C) | NONE | Defer - multi-word arithmetic in generated C. |
| **M1** R1 no-`be`-under-negation | MODERATE | MINOR (coverage gain - currently *skipped*) | NONE (already converts it) | Defer - `!be[N](p)` is desugarable to a windowed `always(!p)`. |
| **M4** R4 no-`next` in `be` body | MODERATE (cheap) | MINOR (coverage gain) | NONE | Defer - a delayed obligation body. |
| **M2** R2 `be` body/placement | MODERATE-HARD | MINOR-MODERATE (coverage gain) | NONE | Defer - composing nested obligations in the monitor state machine. |
| **M3** R3 `always[N]` under temporal | MODERATE | MINOR (coverage gain) | NONE | Defer - generalize the anchor-range machinery. |
| **M5** `eventually`/`until` not monitorable | HARD | n/a (MC already checks them) | n/a | Keep - fundamental: unbounded liveness cannot be an online per-sample monitor. |
| **E1-E3** exact-mode liveness decomposition | HARD | **HUGE** (full LTL -> automata product => state-space blowup) | NONE (SMT already nests; gated by S5) | Keep - lifting means real LTL model checking in exact mode. Document as an exact-mode-only limit. |
| **S5** `--full-properties` default-off | guardrail, not a language rule | n/a | **HUGE** at large bounds if made default (O(k*N) formula growth) | Keep - a deliberate performance lever, not an obsolete restriction. |
| **P2, P4, P7, V1-V3, V5-V7, V8b, V9-V11, V14-V16** (core type/structure checks) | HARD | NONE | NONE | Keep - semantic necessities (name resolution, arity, typing, determinism). Not removal candidates. |

## Summary of actionable items

- **Remove now (obsolete, zero MC cost):** V17 (the named-fields-in-formula-
  `input_is` rule) and S3 (the dead power-of-two check). These are the only two
  restrictions where the downstream machinery already fully handles the general
  case and a stale check is the sole blocker.
- **Cheap wins that remove real cross-mode asymmetries:** S4 (SMT top-level
  property form - desugar to `always(...)`) and unifying E4/S1 (var-init
  handling - exact mode already accepts what SMT rejects).
- **Defer (genuinely moderate, need a semantic decision or multi-engine work):**
  V18 (`always[0]`), P5, P1, P3, V8a, P6, S2, C1, and the monitor-subset R1-R4
  (M1-M4) - the latter are coverage gains for exact mode, not obsolete blockers.
- **Keep (hard / fundamental):** the core typing & structure rules, M5 (online
  liveness monitoring), E1-E3 (exact-mode LTL), S5 (the performance gate), and
  the deliberate design decisions V4/V12/V13.

## Implementation tasks (Phase 18 plan corrections)

The cheap removals above are scheduled as Phase 18 plan-correction tasks
(`docs/roadmap/phase_18/PHASE_18_PLAN.md`, "Plan corrections" table):

- **3F** - remove the OBSOLETE restrictions (V17 + S3).
- **3G** - desugar the SMT top-level property form (S4).
- **3H** - unify var-init handling across exact and smt-bounded (E4/S1).
- **3I** - tests for all of the above + refactor the existing test suite to adopt
  the new accepted syntax.
- **3J** - update all documentation and the example/test README files.

Each task file carries enough context to start in a fresh session without
re-reading large parts of the source tree.
