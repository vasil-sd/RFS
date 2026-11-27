# Phase 18, Task 4A - Implementation status

> Written once when all exit criteria of `PHASE_18_TASK_4A.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

| Name | Description |
|---|---|
| - | none - all changes are to existing files |

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| `src/lang/parser.rs` | changed | new `find_anonymous_payload` scan + three helpers (`whole_word_at`, `anonymous_field_offset`, `skip_ws_bytes`) mirroring the existing `find_eventually_missing_paren` precedent; a dedicated error branch in `parse_source` (after the eventually-parens branch) reports `anonymous event/output payloads are not supported - name the variant: ...` at the field identifier's offset when the file fails AND contains an `event <Name>(<ident> :` / `output <Name>(<ident> :` shape. Three inline unit tests pin message + position + the named-variant negative control |

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `docs/roadmap/phase_18/tasks/PHASE_18_TASK_4A.md` | changed (append) | the "Other information" section now carries the execution record + Table 1 (the ten accepted pattern classes, each with a real corpus/test example or an explicit no-corpus-use note, and per-surface parse/meaning/match behavior) + Table 2 (proposals P1-P5 with description + rationale + effort estimate; none implemented) |
| `docs/llm/SRC_STRUCTURE.md` | changed | four new fn rows (`find_anonymous_payload`, `whole_word_at`, `anonymous_field_offset`, `skip_ws_bytes`); the `parse_source` row gains `find_anonymous_payload` in its depends + an updated description; llm-budget 1840 -> 1844 (via `rfs-dev inventory add/update`) |
| `docs/llm/TEST_STRUCTURE.md` | changed | three new test rows (the two dedicated-diagnostic pins + the named-variant negative control); llm-budget 1504 -> 1507 |

## Other file changes

| Name | Operation | Description |
|---|---|---|
| - | none | no example, capture, or training-set file changed - the parser change is failure-path-only, so every currently-valid spec parses and model-checks byte-identically |

## Changed functionality
- Parse-failure reporting (`parse_source`): a spec with an anonymous event/output payload (`event E(x: bit<8>)` / `output O(v: bit<8>)`) now reports the dedicated `anonymous <kind> payloads are not supported - name the variant: ...` message at the field identifier's exact offset, instead of the generic `trailing input after RFS source`. The generic message is UNCHANGED for every other failure shape (missing brace, etc.) and for files that parse successfully.

## Removed functionality
- None. The swallowed-failure behavior is not removed - it is now PRECEDED by a specific diagnostic for the one shape that triggered it (the same relationship as `eventually` missing its parens).

## New functionality
- `find_anonymous_payload` (+ `whole_word_at`, `anonymous_field_offset`, `skip_ws_bytes`): a whole-word, string-literal-skipping byte scan for the anonymous-payload shape, consulted only after a parse failure. Returns the declaration keyword and the field identifier's offset so `err_at` can point at the exact token the parser misread as a unit-variant name.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|
| `src/lang/parser.rs::tests::anonymous_event_payload_reports_dedicated_diagnostic` | new | `event E(x: bit<8>)` reports the anonymous-event message with the span at the field identifier (`x:`), not offset 0 |
| `src/lang/parser.rs::tests::anonymous_output_payload_reports_dedicated_diagnostic` | new | `output O(v: bit<8>)` reports the anonymous-output message with the span at the field identifier (`v:`) |
| `src/lang/parser.rs::tests::named_variant_payload_still_parses` | new | negative control: naming the variant (`event E(E(x: bit<8>))` / `output O(O(v: bit<8>))`) - the fix the diagnostic points to - keeps parsing on both surfaces |

## Tests run status

| Name | Status |
|---|---|
| `cargo build` | OK |
| `cargo test` (all suites) | OK - every binary `ok`, 0 failed (lib 296 passed incl. the 3 new parser tests; the long example-validation/parity suites that pin "all specs keep validating identically" all pass) |
| `cargo clippy --all-targets` | OK - exit 0, zero warnings in `parser.rs` (the 29 lib warnings are pre-existing elsewhere) |
| `cargo fmt --check` | OK - clean (after reformatting the new `skip_ws_bytes` helper) |
| `rfs-dev inventory audit` | EXACT - SRC 1043 fns / 217 types, TEST 1159 fns / 35 types, zero per-file diffs |
| `rfs-dev sync-check` | OK - clean |
| `cargo test --test dev_docs_test --test docs_md_test` | OK - 9 + 5 passed / 0 failed (after the llm-budget bumps) |

Live verification before pinning (`./target/debug/rfs validate`):
- `event E(x: bit<8>)` -> `anonymous event payloads are not supported - name the variant: event E(E(x: bit<8>))`, exit 1
- `output O(v: bit<8>)` -> `anonymous output payloads are not supported - name the variant: output O(O(v: bit<8>))`, exit 1
- `event E(E(x: bit<8>))` -> `valid`, exit 0

## Found problems/issues/mismatches
- `cargo fmt --check` initially failed on the new `skip_ws_bytes` helper (rustfmt wraps the long `matches!` guard). Fixed by running `cargo fmt`; only that one function was reformatted.
- Adding the seven inventory rows pushed both structure files past their declared `llm-budget` headers (SRC 1840, TEST 1504); `dev_docs_test::sync_gate_passes_on_current_tree` caught it. Bumped to 1844 / 1507.

## Other important findings/information
- **The `Group(_)` trap, reproduced empirically.** A spec whose only difference is `input_is(Wake)` vs `input_is(Wake(_))` under the same requiring assumption `always (bounded_eventually[3] (input_is(...)))` gives `status=pass-complete` (exit 0) for the bare group and `deadlock: configuration at depth 1 has no enabled transition` (exit 1) for the phantom - exactly the task-4 B9 false deadlock. The bare-group form is therefore confirmed to already work as THE payload-agnostic idiom (Table 2 P1).
- **A variant CAN be declared literally named `_`.** `event E(_)` is valid and generates input `Input::E__`; both `on E(_)` (transition) and `input_is(E(_))` (formula) match it. So the "phantom" `_` is only phantom when no `_` variant exists - this refines P2's soundness argument (reinterpreting formula-level `Group(_)` as `None` would broaden a degenerate spec that declares a real `_` variant).
- **Formula patterns are entirely unvalidated.** `validator.rs::check_pattern_group` is a no-op, so formulas skip the V7 variant-existence check, the field-count check, and the V8a literal-fit rule that transitions enforce. This means `input_is(Group(V(wrong_count)))` and non-fitting literals silently never match - the SAME failure class as the `Group(_)` trap, just broader. Table 2 P5 proposes closing this gap generally.
