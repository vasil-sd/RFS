<!-- llm-budget: 120 -->
# RFS Parser (Class B)

Compact card for LLM assistants working on parsing, grammar, or reserved words.

<a id="llm:parser"></a>
## Card

**Responsibility.** Turns RFS v0 source text into an IR document in one pass
(nom-based recursive descent; no separate lexer) and assembles the IR while
parsing. Two files: `src/parser.rs` (grammar + entry points) and
`src/ebnf_grammar.rs` (EBNF model mirroring the parser, rendered by
`rfs info format spec-rfs`).

**Key items.**

| Item | Role |
|---|---|
| `src/parser.rs::parse_source` | entry: strip comments -> parse -> trailing-input check -> dedicated eventually diagnostic on failure |
| `src/parser.rs::parse_file` | read a file, delegate to parse_source |
| `src/parser.rs::strip_comments` | offset-preserving comment removal (error positions map 1:1 to the source) |
| `src/parser.rs::syntax_err` / `src/parser.rs::semantic_err` | two-tier errors: backtrackable vs non-backtrackable nom failures |
| `src/parser.rs::KEYWORDS` | 28 reserved words; single source of truth (parser call sites + IDE grammar pinned to it) |
| `src/parser.rs::module_item` | alt over the 9 declaration kinds (one lookahead token decides) |
| `src/parser.rs::build_transition` | requires exactly one next, rejects duplicate set vars |
| `src/parser.rs::build_module` | transition IDs = declaration order; module-level uniqueness; shared prop/assume namespace |
| `src/parser.rs::merge_group` | merges repeated event/output group decls into one ConstructorGroup |
| `src/parser.rs::group_pattern` / `src/parser.rs::pattern_field` | event patterns: bare name, group(variant), field bindings/literals/wildcards |
| `src/parser.rs::expr` | expression ladder entry (12 levels, loosest: logical or) |
| `src/parser.rs::formula` | formula ladder entry (implies right-assoc; until left-assoc; negation tighter than until) |
| `src/parser.rs::primary_formula` | temporal atoms + fallthrough to non-boolean expr at the equality level |
| `src/parser.rs::int_lit` | u64 literals; overflow is a semantic error |
| `src/ebnf_grammar.rs::GRAMMAR_RULES` | 61-rule EBNF model, pinned rule-for-rule to DESIGN.md section 6 + parser keywords |
| `src/ebnf_grammar.rs::to_md` / `src/ebnf_grammar.rs::to_plantuml_ebnf` / `src/ebnf_grammar.rs::to_svg` | renderers behind rfs info format spec-rfs |

**Invariants that must not break.**

- strip_comments preserves offsets (spaces, keeps newlines) - error line/col depend on it.
- Error tiers: recognized-but-ill-formed = semantic_err (aborts); unrecognized token = syntax_err (backtrackable). Do not swap them.
- KEYWORDS is pinned in both directions against parser call sites (keyword_list_matches_parser_sites) and the IDE grammar - a new keyword goes into KEYWORDS AND ide/textmate/rfs.json.
- GRAMMAR_RULES mirrors DESIGN.md section 6 rule-for-rule; sync tests fail on drift - change parser + model + fence together.
- Transition IDs are declaration order; prop/assume share one namespace; one initial + one on_error per module.

**Change impact (grammar/language).** Add valid+invalid examples FIRST, then:
`src/parser.rs` (+ unit tests in its mod tests),
`src/ebnf_grammar.rs::GRAMMAR_RULES`, DESIGN.md section 6 fence; verify with
cargo test --lib parser, cargo test --test examples_test,
cargo test --test info_format_test. Full table: llm:change-impact (task 13.2).

**Deep dive.** dev:parser-overview + dev:parser-entry-error /
dev:parser-grammar / dev:parser-expressions / dev:parser-ebnf-model in
docs/dev/parser.md; algorithm levels: dev:algorithms-parser-validation;
call graph: dev:deps-fn-parser.
