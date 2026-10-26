# Task 5 - `rfs info format` infrastructure + spec-rfs (EBNF)

## Subtasks

### 5.1 Format registry + CLI dispatch (`src/info_format.rs`, `src/main.rs`)
- [x] `FormatInfo { name, summary, outputs, render }` + static `FORMATS` registry in new `src/info_format.rs` (registry pattern mirrors `DIAGRAMS`/`BACKENDS`; 4 unit tests: unique names, every entry renders its default output, unknown-format error, unsupported-output error)
- [x] `run_info_format`: `--list` (markdown table), name lookup with clear error on unknown name (points at `--list`), `--output` validated against the entry's supported list (default = first listed); `--help` prints usage; duplicate/missing values rejected
- [x] `cmd_info` dispatches bare first arg `"format"` BEFORE any path handling (D-P7-6 - a file literally named `format` cannot collide); usage text gained the `rfs info format --list | <name> [--output md|plantuml|svg|schema]` line + an `info format:` description block

### 5.2 EBNF grammar model + renderers (`src/ebnf_grammar.rs`, `Cargo.toml`, `Cargo.lock`)
- [x] `RuleNode` enum (const-constructible: slices + references instead of the plan's `Vec`/`Box`) + `GRAMMAR_RULES` - 61 rules mirroring the ACTUAL parser (`src/parser.rs`), including lexical rules (`identifier`, `letter`, `digit`, `unsigned_integer_literal`, `string_literal`). Verified rule-by-rule against the parser entry points (module header w/ version, on_error body = exactly one `next` among any `emit`s, output_call_payload two-level form, pattern_field/pattern_expr literal alternatives, full expression ladder incl. BOTH the bitwise and arithmetic levels - see plan corrections)
- [x] `to_md()` (intro + ebnf code block + rule table with per-rule notes), `to_plantuml_ebnf()` (@startebnf/@endebnf, quoted terminals, `[X]`/`{X}`/`{X}-` - verified against the current PlantUML tokenizer: no range suffixes needed), `to_ebnf_text()` (the canonical one-rule-per-line text DESIGN.md section 6 now carries)
- [x] Added `railroad = { version = "0.3", default-features = false }` (SVG strings only; the `resvg` PNG feature stays off); SVG via RuleNode->railroad mapping (Sequence->Sequence, Choice->Choice, Optional->Optional, ZeroOrMore->Optional(Repeat), OneOrMore->Repeat) on a `Stylesheet::Light` diagram, one LabeledBox per rule in a VerticalGrid

### 5.3 Tests (`tests/info_format_test.rs`, new)
- [x] `--list` pin (every registered name as a row + row count; current registry byte-pinned to exactly spec-rfs)
- [x] spec-rfs md pin (all 61 rules in the ebnf block AND the rule table, every `parser::KEYWORDS` entry a quoted terminal, 61-rule count guard)
- [x] plantuml pin (@startebnf/@endebnf wrapper, every rule line terminated with `;`, all rule names, quoted terminals)
- [x] svg pin (standalone `<svg`...`</svg>`, every rule name drawn as a text element, one comment label per rule)
- [x] DESIGN.md section 6 rule-name parity test (parse the ebnf fence, compare name sets in BOTH directions + no duplicates) - plus library-level dangling-nonterminal and keyword-terminal model checks, and CLI error cases (unknown format with `--list` hint, unsupported output, missing name)

### 5.4 Docs (`BUILD.md`)
- [x] Section 3: new dedicated row for `cargo test --test info_format_test` (10 pass + coverage description); root count re-pinned **493 pass** (+14 = 4 lib unit + 10 integration) with the suite added to the breakdown; workspace re-pinned **517 pass**; stale "part of the 273" parser row fixed to the current 242 lib tests
- [x] Section 5: known-good baseline paragraph re-measured (493/517/495-smt; clippy 36 pre-existing warnings, zero in task 5 files)

## Status

COMPLETE (2026-10-26).

Verification: new suite **10 pass / 0 fail**; full root suite **493 pass / 0 fail** (was 479); workspace **517 pass / 0 fail** (was 503); `--features smt` **495 pass / 0 fail** (z3 env per TOOLCHAIN.md 5.2); clippy zero warnings in the new/changed files (36 pre-existing elsewhere, unchanged); `cargo fmt --check` clean; docs gate (`docs_md_test`, parses every repo `.md` incl. the rewritten DESIGN.md section 6) green; `rfs diagram ... --render svg` (internal) and legacy `rfs info <spec>` regression-checked.

Notes for later tasks:
- The registry is library-driven: tasks 6-14 each add ONE `FormatInfo` entry + their render fns; the `--list` pin stays green as long as it asserts against `FORMATS` (the byte pin of the current registry will need extending, not fixing).
- `src/ebnf_grammar.rs` is now the single source of truth for the RFS v0 grammar: DESIGN.md section 6's fence is its canonical text (sync test enforces rule-name parity). If the parser language ever changes, update `parser.rs`, `GRAMMAR_RULES`, and DESIGN.md together - the sync test + KEYWORDS coverage fail otherwise.
- The stray untracked `examples/valid/40_on_error_recovery.json` left by an interrupted earlier attempt was DELETED (D-P7-11: no new example files in this phase; it was referenced nowhere).
