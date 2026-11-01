# Phase 7, Task 5 - Implementation status

> Written once when all exit criteria of `PHASE_7_TASK_5.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

| Name | Description |
|---|---|
| `src/info_format.rs` | `FormatInfo { name, summary, outputs, render }` registry + static `FORMATS` (spec-rfs entry first) + `run_info_format`: `--list` markdown table, unknown-name error with list hint, `--output` validated against the entry's supported list (default md) |
| `src/ebnf_grammar.rs` | const-constructible `RuleNode` model + 61-rule `GRAMMAR_RULES` mirroring the ACTUAL parser (incl. lexical rules); renderers `to_md()` / `to_plantuml_ebnf()` (plantuml.com/en/ebnf "All EBNF Elements": quoted terminals, `[0..n]`/`[1..n]`) / `to_svg()` (railroad mapping on a stylesheet diagram) |
| `tests/info_format_test.rs` | 10 tests: `--list` pin, spec-rfs md/plantuml/svg pins, DESIGN.md section 6 rule-name parity, no-dangling-nonterminals, keywords-as-terminals, 3 CLI error tests |

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| `Cargo.toml` / `Cargo.lock` | changed | `railroad = { version = "0.3", default-features = false }` added (SVG rendering for spec-rfs) |
| `src/lib.rs` | changed | `pub mod ebnf_grammar;` + `pub mod info_format;` (test access) |
| `src/main.rs` | changed | `cmd_info` dispatches bare first token `"format"` to `run_info_format` (before path handling, so a file named `format` cannot collide); usage text updated (D-P7-6) |

## Added/changed/extended documentation files

| Name | Operation | Description |
|---|---|---|
| `DESIGN.md` | changed | section 6 ebnf fence REPLACED with the canonical text of `to_ebnf_text()` (one rule per line, no in-fence comments; the two precedence notes moved to prose after the fence) - the fence was stale vs the actual parser (correction 5.2) |
| `BUILD.md` | changed | section 3 row for `rfs info format`; baseline re-pin (root 493 / workspace 517 / smt 495) |

## Other file changes

| Name | Operation | Description |
|---|---|---|
| `TASKS.md` | changed | task 5 entry recorded (2026-10-26 21:42) |
| `docs/roadmap/INTRO.md` | changed | Phase 7 row -> implementing (tasks 1-5 of 16 complete) |
| `examples/valid/40_on_error_recovery.json` | removed (untracked) | stray generated artifact left by an interrupted earlier attempt; DELETED (D-P7-11: no new example files in this phase; it was referenced nowhere) - untracked, so absent from the commit diff |

## Changed functionality

- DESIGN.md section 6 now matches the actual parser (was stale - see corrections).

## Removed functionality

None.

## New functionality

- `rfs info format` command: `--list` (markdown table of registered formats),
  per-format rendering with `--output` validation, clear errors for unknown
  names/outputs.
- First registered format: `spec-rfs` - the RFS v0 EBNF grammar in md /
  plantuml / svg (railroad).
- D-P7-9 sync guarantees: DESIGN.md section 6 rule-name parity test,
  no-dangling-nonterminals, keywords-as-terminals.

## Tests add/change/remove

- Added (10, all in the new `tests/info_format_test.rs`):
  `list_pins_all_registered_formats`, `spec_rfs_md_pins_all_rules_and_keywords`,
  `spec_rfs_plantuml_pins_structure`, `spec_rfs_svg_pins_labels`,
  `design_md_section6_rule_names_match_model`, `no_dangling_nonterminals`,
  `keywords_appear_as_terminals_in_model`, `unknown_format_errors_with_list_hint`,
  `unsupported_output_errors`, `no_name_and_no_list_errors`.
- Changed: none.

## Tests run status

| Name | Status |
|---|---|
| `cargo test` (root) | OK - 493 pass / 0 fail |
| `cargo test --workspace` | OK - 517 pass / 0 fail |
| `cargo test --features smt` | OK - 495 pass / 0 fail (z3 env per TOOLCHAIN.md 5.2) |
| `tests/info_format_test.rs` | OK - 10 tests |
| lib unit tests | OK - 242 |
| `cargo clippy --all-targets` | OK - zero warnings in new/changed files (36 pre-existing elsewhere, unchanged) |
| `cargo fmt --check` | OK |
| docs gate (`docs_md_test`, parses every repo `.md` incl. the rewritten DESIGN.md section 6) | OK |
| regression: `rfs diagram ... --render svg` (internal) + legacy `rfs info <spec>` | OK |

## Found problems/issues/mismatches

- DESIGN.md section 6 ebnf fence was STALE vs the actual parser (missing
  `version`, `on_error_decl`, `priority_action`, `output_call_payload`, bitwise
  ladder levels, literal alternatives in `pattern_field`/`pattern_expr`,
  undefined lexical rules; old arithmetic-only placement) - replaced with the
  canonical `to_ebnf_text()` text (plan correction 5.2).
- The WIP `GRAMMAR_RULES` from an interrupted earlier attempt was missing
  `add_expr`/`mul_expr` (the parser's expression ladder has BOTH bitwise and
  arithmetic levels between shift and unary) - added both rules; rule count
  59 -> 61. Caught by cross-checking the model against the parser BEFORE writing
  the sync test (plan correction 5.2).

## Other important findings/information

- This status file was reconstructed retroactively on 2026-10-27 (protocol
  violation: no per-task status file was written at execution time). Sources:
  commit `8803f87` message + diff, TASKS.md entry for task 5, BUILD.md baseline
  re-pins.
- Notes for later tasks: the registry is library-driven - tasks 6-14 each add
  ONE `FormatInfo` entry + their render fns; the `--list` pin stays green as
  long as it asserts against `FORMATS` (the byte pin of the current registry
  gets extended, not fixed). `src/ebnf_grammar.rs` is now the single source of
  truth for the RFS v0 grammar: DESIGN.md section 6's fence is its canonical
  text (sync test enforces rule-name parity). If the parser language ever
  changes, update `parser.rs`, `GRAMMAR_RULES`, and DESIGN.md together - the
  sync test + KEYWORDS coverage fail otherwise.
- Committed as `8803f87` (2026-10-26), 13 files changed (+1671/-132).
