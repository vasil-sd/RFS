# Phase 7, Task 5 - Implementation details

> Written for a future LLM session: after reading `START.md`,
> `docs/roadmap/INTRO.md` and this file, the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal

The `rfs info format` infrastructure (format registry + CLI plumbing) plus the
flagship format: the RFS v0 language EBNF grammar in md / plantuml / svg
(D-P7-6/D-P7-7). This is the foundation for the JSON-schema format entries of
tasks 6-10 and the diagram-subset entries of tasks 11-14.

## Context

- Phase 7 plan: rows 5, 5.1-5.4; decisions D-P7-6 (CLI shape), D-P7-7 (grammar
  model mirrors the ACTUAL parser, including lexical rules), D-P7-9 (sync
  guarantees: docs must not drift from code).
- An interrupted earlier attempt had left 5.1 + most of 5.2 uncommitted (a WIP
  `GRAMMAR_RULES`); this task verified it rule-by-rule against `parser.rs`,
  fixed a real model gap (see corrections), wrote the tests, updated DESIGN.md
  section 6, and finished docs/bookkeeping.
- The in-binary grammar model is the source of truth; DESIGN.md follows it.

## Scope

### In scope
- Format registry + CLI dispatch (`src/info_format.rs`, `src/main.rs`).
- EBNF grammar model + md/plantuml/svg renderers (`src/ebnf_grammar.rs`;
  `railroad` dependency for SVG).
- Tests (`tests/info_format_test.rs`, new) incl. the DESIGN.md section 6
  rule-name parity test.
- DESIGN.md section 6 fence re-synced to the actual parser.
- BUILD.md section 3 row; TASKS.md entry; INTRO.md row update.

### Out of scope
- JSON format entries (tasks 6-10), diagram-subset entries (tasks 11-14).

## Planned actions

1. **Format registry + CLI dispatch** (`src/info_format.rs`, `src/main.rs`):
   `FormatInfo { name, summary, outputs: &[&str], render: fn(&str /*output*/,
   &mut String) }` + static `FORMATS`; `run_info_format`: `--list` (markdown
   table), name lookup with clear error on unknown name, `--output` validated
   against the entry's supported list (default md). `cmd_info` dispatches bare
   first arg `"format"` to it (before path handling, so a file named `format`
   cannot collide); usage text updated (D-P7-6).
2. **EBNF grammar model + renderers** (`src/ebnf_grammar.rs`, `Cargo.toml`,
   `Cargo.lock`): `RuleNode` (const-constructible: slices + references instead
   of the plan's `Vec`/`Box`) + `GRAMMAR_RULES` - 61 rules mirroring the ACTUAL
   parser (`src/parser.rs`), including lexical rules (`identifier`, `letter`,
   `digit`, `unsigned_integer_literal`, `string_literal`). Verified rule-by-rule
   against the parser entry points (module header w/ version, on_error body =
   exactly one `next` among any `emit`s, output_call_payload two-level form,
   pattern_field/pattern_expr literal alternatives, full expression ladder incl.
   BOTH the bitwise and arithmetic levels - see plan corrections). `to_md()`
   (intro + ebnf code block + rule table with per-rule notes),
   `to_plantuml_ebnf()` (@startebnf/@endebnf, quoted terminals, `[X]`/`{X}`/
   `{X}-` - verified against the current PlantUML tokenizer: no range suffixes
   needed), `to_ebnf_text()` (the canonical one-rule-per-line text DESIGN.md
   section 6 now carries). Add `railroad = { version = "0.3",
   default-features = false }` (SVG strings only; the `resvg` PNG feature stays
   off); SVG via RuleNode->railroad mapping (Sequence->Sequence, Choice->Choice,
   Optional->Optional, ZeroOrMore->Optional(Repeat), OneOrMore->Repeat) on a
   `Stylesheet::Light` diagram, one LabeledBox per rule in a VerticalGrid.
3. **Tests** (`tests/info_format_test.rs`): `--list` pin (every registered name
   as a row + row count; at this task the registry is byte-pinned to exactly
   spec-rfs); spec-rfs md pin (all 61 rules in the ebnf block AND the rule
   table, every `parser::KEYWORDS` entry a quoted terminal, 61-rule count
   guard); plantuml pin (@startebnf/@endebnf wrapper, every rule line
   terminated with `;`, all rule names, quoted terminals); svg pin (standalone
   `<svg>...</svg>`, every rule name drawn as a text element, one comment label
   per rule); DESIGN.md section 6 rule-name parity test (parse the ebnf fence,
   compare name sets in BOTH directions + no duplicates) - plus library-level
   dangling-nonterminal and keyword-terminal model checks, and CLI error cases
   (unknown format with `--list` hint, unsupported output, missing name).
   Plus 4 library unit tests in `src/info_format.rs`: unique names, every entry
   renders its default output, unknown-format error, unsupported-output error.
4. **Docs** (`BUILD.md`): section 3 new dedicated row for
   `cargo test --test info_format_test` (10 pass + coverage description); root
   count re-pinned with the suite added to the breakdown (+14 = 4 lib unit +
   10 integration); stale "part of the 273" parser row fixed to the current 242
   lib tests; section 5 known-good baseline paragraph re-measured.

#### Affected files

| File | Operation |
|---|---|
| `src/info_format.rs` | new - `FormatInfo` registry + static `FORMATS` + `run_info_format` (spec-rfs entry first) |
| `src/ebnf_grammar.rs` | new - const-constructible `RuleNode` model + 61-rule `GRAMMAR_RULES` mirroring the actual parser; `to_md()` / `to_plantuml_ebnf()` / `to_svg()` (railroad) |
| `Cargo.toml` / `Cargo.lock` | changed - `railroad = { version = "0.3", default-features = false }` added |
| `src/lib.rs` | changed - `pub mod ebnf_grammar;` + `pub mod info_format;` (test access) |
| `src/main.rs` | changed - `cmd_info` dispatches bare first token `"format"` to `run_info_format`; usage text updated |
| `tests/info_format_test.rs` | new - 10 tests: list pin, spec-rfs md/plantuml/svg pins, DESIGN.md section 6 rule-name parity, no-dangling-nonterminals, keywords-as-terminals, 3 CLI error tests |
| `DESIGN.md` | changed - section 6 ebnf fence REPLACED with the canonical text of `to_ebnf_text()` (one rule per line, no in-fence comments; the two precedence notes moved to prose after the fence) |
| `BUILD.md` | changed - section 3 row for `rfs info format`; baseline re-pin |

## Task deliverables

- `rfs info format --list` / `rfs info format spec-rfs --output md|plantuml|svg`
  work; unknown names/outputs produce clear errors.
- DESIGN.md section 6 is byte-canonical against the in-binary model (sync test
  enforces rule-name parity).
- Full test matrix green.

## Task exit criteria

- [x] Registry + CLI dispatch: `--list` markdown table, unknown-name error, `--output` validated (default md) (DONE: `list_pins_all_registered_formats`, `unknown_format_errors_with_list_hint`, `unsupported_output_errors`, `no_name_and_no_list_errors`)
- [x] EBNF model mirrors the actual parser incl. lexical rules; 61 rules (DONE: verified rule-by-rule against `parser.rs`; `no_dangling_nonterminals`, `keywords_appear_as_terminals_in_model`)
- [x] md / plantuml / svg renderers work and are pinned (DONE: `spec_rfs_md_pins_all_rules_and_keywords`, `spec_rfs_plantuml_pins_structure`, `spec_rfs_svg_pins_labels`)
- [x] DESIGN.md section 6 rule-name parity enforced by test (DONE: `design_md_section6_rule_names_match_model`; fence replaced with canonical `to_ebnf_text()` text)
- [x] Full test matrix green; BUILD.md baseline re-pinned (DONE: root 493 / workspace 517 / smt 495, info_format_test 10, lib 242)

## Non-goals

- No JSON-schema entries yet (task 6 introduces the shared emitter).
- No language/grammar changes - the model mirrors the parser as-is.

## Task corrections

- **5.2** (2026-10-26): DESIGN.md section 6 ebnf fence found STALE vs the actual
  parser: missing `version` in module, `on_error_decl`, `priority_action`,
  `output_call_payload`, the bitwise ladder levels, literal alternatives in
  `pattern_field`/`pattern_expr`, and the lexical rules (`letter`/`digit`/
  `unsigned_integer_literal`/`string_literal` were referenced but never defined);
  it still carried the old arithmetic-only placement. The fence was REPLACED
  with the canonical text of `to_ebnf_text()` (one rule per line, no in-fence
  comments - the two precedence notes moved to prose after the fence), and the
  sync test now enforces rule-name parity. Phase 7's whole rationale is docs
  staying in sync with the code; the plan assumed section 6 was accurate. The
  in-binary model (verified rule-by-rule against `parser.rs`) is the source of
  truth; DESIGN.md follows it.
- **5.2** (2026-10-26): the WIP `GRAMMAR_RULES` from an interrupted earlier
  attempt was MISSING `add_expr`/`mul_expr`: the parser's expression ladder
  contains BOTH the bitwise levels (`| ^ &`) AND the arithmetic levels (`+ - *`)
  between shift and unary (verified in `parser.rs` `shift`->`additive`->
  `multiplicative`). Added both rules + notes; rule count 59 -> 61. Caught by
  cross-checking the model against the parser BEFORE writing the sync test -
  exactly the drift class D-P7-9 exists to catch. The old DESIGN.md fence had
  add/mul but no bitwise levels; neither document was right on its own.

## Other information

- This file was reconstructed retroactively on 2026-10-27 after a protocol
  violation (work notes existed as `docs/roadmap/tasks/task_5.md`, since removed).
  Sources: commit `8803f87` message + diff, TASKS.md entry for task 5,
  `PHASE_7_PLAN.md` corrections table, BUILD.md baseline re-pins.
- Committed as `8803f87` (2026-10-26); the commit also carries the TASKS.md
  entry and the INTRO.md row update (tasks 1-5 of 16 complete).
