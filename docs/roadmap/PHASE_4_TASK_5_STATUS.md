# Phase 4, Task 5 - Status

**State:** COMPLETE (all exit criteria met)
**Date:** 2026-10-18

## Files added / changed / removed
- `src/parser.rs` (changed): new `until_formula` level between unary and primary (`neg_operand { "until" neg_operand }`, left-associative) + `eventually "(" formula ")"` as a new `primary_formula` alternative; the old unary body moved to `neg_operand` below the until level so `!` binds tighter than `until` (see Task corrections in the task file). 8 unit tests pin the precedence table, left-associativity, unary interaction, parenthesized compound operands, `eventually` nesting, malformed-shape rejection, and `until` not shadowing identifiers in expression contexts.
- `src/ir.rs` (changed): additive `Formula::Eventually { inner }` / `Formula::Until { left, right }`; NO `IR_VERSION` bump (stays 0).
- `src/printer.rs` (changed): new `Fctx::UntilLeft`/`Fctx::UntilRight` contexts; `needs_parens` updated (And/Or/Implies/boolean-carrying Expr parenthesized in until positions; Until bare on the left, parenthesized on the right; Eventually safe bare everywhere); `Until` printing arm (`a until b`, minimal parens); the `Not` arm now also parenthesizes an inner `Until` (bare `!a until b` re-parses as `(!a) until b`). 3 tests: both-direction round-trip for 13 canonical shapes, exact-printing spot checks, and pre-liveness IR JSON still deserializing.
- `src/validator.rs` (changed): `type_formula` types both liveness operands as boolean (clear "must be bool" diagnostics); `is_safety_formula` returns false for `Eventually`/`Until`; the `always[N] >= 1` bound check recurses through liveness operators. 4 unit tests (accept/reject/bound-recursion/safety-classification).
- `src/monitor.rs` (changed): explicit out-of-subset arm - any formula containing `eventually`/`until` fails `compile_formula` with a dedicated message, so codegen in every backend emits the identical "outside the monitor subset; skipped" warning (MC-only). 1 unit test covering top-level and nested positions.
- `tests/codegen_test.rs` (changed): new test pins the IDENTICAL skip-warning text (same strings, same order) from production codegen of a liveness module in both backends.
- No example files added (task 6 lands the first liveness example); `examples_test` unchanged at 42 valid / 12 invalid.

## Tests
- `cargo test`: **201/201** (was 184; +17 new: 8 parser, 3 printer, 4 validator, 1 monitor, 1 codegen).
- Precedence pinned: `a && b until c` = `a && (b until c)`; `a until b until c` = `(a until b) until c`; `!a until b` = `(!a) until b`; `(a && b) until c` / `a until (b || c)` require parens; `eventually p` (no parens), `a until` (missing operand), unclosed body all rejected; `until_b` stays an identifier in expression contexts (`a == until_b`, `a && until_b`).
- Round-trip stable both directions (rfs->json->rfs and json->rfs->json) for all 13 canonical liveness shapes; pre-liveness IR JSON deserializes unchanged (no version bump).
- Validator rejects non-boolean operands (`eventually (x)`, `x until flag`, `flag until x`) with "must be bool" diagnostics; accepts boolean shapes.
- Identical skip warning in both backends' production codegen, exact text pinned: `property '<name>' is outside the monitor subset; skipped` / `assume '<name>' ...`.
- clippy: baseline 12 warnings (11 lib + 1 codegen_test), zero from this task; `cargo fmt --check` clean.

## Findings / notes for later tasks
- **Task correction (see task file):** the EBNF block in Context contradicted its own pinned consequence for `!a until b`; implemented per the pinned consequence + ladder + standard PSL/CTL (negation below the until level). The OPEN QUESTION (`a && b until c`) was resolved with the stated default (pinned production: `a && (b until c)`); user informed at task start, no objection.
- **Environment (this session):** working toolchain = rustc 1.99.0 at `/workspace/host/rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin` with `CARGO_HOME=/opt/unsloth-studio/sandbox/__LOCALID_nkBnQA0/.cargo`. Hit a PERSISTENT variant of the documented stale-stdin probe incident: cargo's target-info probe (`rustc - --crate-name ___ ...`) read stale session text even with cargo's own stdin redirected to `/dev/null` (the probe's stdin is not reliably cargo's fd 0 in this sandbox). Working workaround: `RUSTC_WRAPPER` script that execs the real compiler with `< /dev/null` forced for `-` probes, handling both arg shapes (with and without the extra compiler-name arg cargo prepends when `RUSTC_WORKSPACE_WRAPPER` is set). Verified via `RUSTC_WRAPPER=... cargo clippy --all-targets`.
- **For task 6 (liveness engine - Rust):** the grammar deliberately allows temporal operators inside liveness operands at parse level (e.g. `eventually (always (a))` parses fine); D-P4-9's scope restriction (top-level obligations with boolean subformulas over atomic propositions) must be enforced as a CODEGEN diagnostic in task 6, not by parser/validator. `is_safety_formula` already classifies `Eventually`/`Until` as non-safety for MC mode selection.
- **For task 13 (docs sync):** DESIGN section 6's grammar ladder becomes or > impl > and > until > not(!) > primary at the parse level, with the pinned consequences above; the D-P4-8 illustrative example in PHASE_4_PLAN.md (`(a && b) until c`) is wrong and should be corrected there too.
- **Log gap (pre-existing):** the root `TASKS.md` has no task 4 entry - task 4 was recorded only in `docs/roadmap/TASKS.md`. Left as-is; noted here and in the root log.
