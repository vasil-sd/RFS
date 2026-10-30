# Phase 8, Task 8 - Implementation details

> Written for a future LLM session: after reading `START.md`,
> `docs/roadmap/INTRO.md` and this file, the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal

Fill in `docs/dev/parser.md` (Class A deep dive): full content on the nom-based
parser (`src/parser.rs`) and the EBNF grammar model that mirrors it
(`src/ebnf_grammar.rs`) - architecture, entry points + two-tier error model,
module/declaration/pattern parsing, expression + formula ladders, IR assembly,
KEYWORDS, renderers + sync tests. Also fill in the matching Class B card
`docs/llm/parser.md` (within its 120-line budget) - per the skeleton's own TBD
assignment ("TBD (task 8.1)").

## Context

- **Skeleton state (from task 1.3)**: `docs/dev/parser.md` exists with one
  anchored fragment `dev:parser-overview` (owned by BOTH `src/parser.rs` and
  `src/ebnf_grammar.rs` - both files' `//! Docs:` lines already name it)
  followed by a TBD line naming this task; `docs/llm/parser.md` exists with
  budget header `<!-- llm-budget: 120 -->`, fragment `llm:parser` (same two
  owners), and a TBD line. The existing anchor lines + headings stay
  byte-stable; this task replaces the TBD bodies.
- **Scope decision (confirm at review)**: plan line 8.1 lists only
  `docs/dev/parser.md`; the Class B cards are listed under plan line 13.3
  ("11 per-module compact cards ... do after tasks 2-12"). But every llm-card
  skeleton's TBD text assigns the card to its deep-dive task (parser -> "TBD
  (task 8.1)", validator -> 8.2, ir_json_rw -> 8.3, codegen_core -> 9.1,
  backend_rust -> 9.2, backend_c -> 10.1, mc_engine -> 11.1, monitor -> 11.3,
  cli -> 12.1, workspace -> 12.2, fuzz -> 12.3) - a consistent scheme from
  task 1. DECIDED: this task fills BOTH files; plan line 13.3 remains the
  final verification/budget pass over all 11 cards once tasks 2-12 are done
  (its rationale "one-line roles match final Class A content" is satisfied:
  each module's deep dive is final when its card is written). If confirmed,
  record a one-line entry in `PHASE_8_PLAN.md`'s Corrections table at
  execution.
- **Fragment structure**: keep `dev:parser-overview` for the Overview section;
  add four new sub-fragments (house pattern: one fragment per major `##`
  section, cf. ALGORITHMS.md):

  | Fragment | Section | Owner(s) to extend |
  |---|---|---|
  | `dev:parser-entry-error` | Entry points and error model | `src/parser.rs` |
  | `dev:parser-grammar` | Module and declaration parsing | `src/parser.rs` |
  | `dev:parser-expressions` | Expression and formula ladders | `src/parser.rs` |
  | `dev:parser-ebnf-model` | EBNF grammar model (`src/ebnf_grammar.rs`) | `src/ebnf_grammar.rs` + `src/parser.rs` (KEYWORDS lives in parser.rs and is part of the model's sync) |

  ORPHAN RULE: each new anchor MUST be added to the owner file's `//! Docs:`
  line in the same action (single line; the gate reads IDs off that line only).
  Exact target lines:
  - `src/parser.rs` line 2 becomes:
    `//! Docs: dev:parser-overview, dev:parser-entry-error, dev:parser-grammar, dev:parser-expressions, dev:deps-fn-parser, dev:algorithms-parser-validation, dev:references-theory, llm:parser`
  - `src/ebnf_grammar.rs` line 10 becomes:
    `//! Docs: dev:parser-overview, dev:parser-ebnf-model, llm:parser`

- **doc-to-code reference rules (gate check 1)**: a backticked string shaped
  like `path/file.rs` or `path/file.rs::item` MUST resolve to an existing file
  and (for `.rs` + `::item`) to an item declared in that file - the gate
  matches declaration lines (`fn`/`struct`/`enum`/`trait`/`type`/`const`/
  `static`/`mod`/impl headers; private items are fine). VERIFY every new
  reference against a declaration line before writing it (grep). Plain-text
  mentions without backticks are exempt. Backticked strings WITHOUT a path
  separator or with other extensions (`.json`, `.rfs`, shell commands) do not
  match the reference shape and are safe. Fenced code blocks (incl. mermaid)
  are EXEMPT from reference resolution.
- **No duplication with existing fragments**: ALGORITHMS.md
  `dev:algorithms-parser-validation` already carries the L1/L2/L3 write-up of
  parser + validation; DEPENDENCIES.md `dev:deps-fn-parser` has the call
  graph; CODE_MAP.md has the file entries; REFERENCES.md has the external
  refs. The deep dive goes DEEPER, not parallel: per-item detail (what each
  grammar function does, what it rejects and with which error tier), worked
  examples of tricky parses (mixed atom kinds, `until` associativity,
  constructor-group merging, the dedicated eventually diagnostic), invariants,
  and change guidance. Cross-link to those fragments; never restate them.
- **Verified source material (measured 2026-10-30; line numbers for
  navigation only - doc references use item names, NEVER line numbers)**:
  - `src/parser.rs` (2091 lines; production 1-1522, `mod tests` 1524-2091 with
    29 tests): entry `parse_source` (73) = strip_comments -> rfs_source ->
    trailing-input check -> dedicated eventually diagnostic on failure;
    `parse_file` (100). `strip_comments` (110) replaces comment bytes (`#` and
    `//`) with spaces keeping newlines - offsets in the cleaned text map 1:1
    to the original. Error tier constructors: `semantic_err` (207) = nom
    Err::Failure non-backtrackable, `syntax_err` (214) = nom Err::Error
    backtrackable (both ErrorKind::Verify). Offset mapping:
    `nom_error_to_parse_error` (139) / `err_at` (151) compute
    clean.len() - residual.len(); `ParseError` carries Span + source for
    line/col (`src/error.rs::line_col`) + snippet/caret (`rich_display`).
    `find_eventually_missing_paren` (161): whole-word `eventually` whose body
    position is not `(`, skips string literals, excludes
    `bounded_eventually`; consulted only after a parse failure. Grammar fns:
    `rfs_source` (221) = many0(module) -> Ir { ir_version: IR_VERSION };
    `module` (232, optional quoted version header); `string_lit` (258);
    `module_item` (265) = alt of 9 decls; `enum_decl` (280) +
    `enum_variants` (289, trailing comma allowed); `initial_decl` (313);
    `event_decl` (322) / `output_decl` (338) + `opt_constructor_payload`
    (354) + `constructor_variant` (365, Unit | Struct) + `field_list` (377);
    `type_parser` (402) / `bit_type` (414); `var_decl` (423, type + init
    expr); `transition_decl` (438) + `build_transition` (461: requires
    `next`, rejects duplicate set-vars) + `action_block` (491) + `action`
    (512: next/emit/set/priority into TransitionBody; duplicates ->
    semantic_err, unknown action name -> syntax_err); `prop_decl` (564) /
    `assume_decl` (576); `on_error_decl` (588, requires `next`);
    `build_module` (638: transition IDs = declaration order; module-level
    uniqueness - enum names, single initial, var names, prop/assume SHARED
    namespace, single on_error); `merge_group` (698: repeated event/output
    group decls merge into one ConstructorGroup; duplicate variant ->
    semantic_err). Patterns: `event_pattern` (726) / `output_pattern` (731)
    / `group_pattern` (736: bare name | group(variant)) /
    `pattern_field_list` (764) / `pattern_field` (783: name | name(expr)
    binding) / `pattern_expr` (844); `output_expr` (872). Expression ladder
    (12 levels, loosest first): `expr` (924) = logic_or (928) > logic_and
    (946) > equality (964) > relational (990) > bitwise_or (1038) >
    bitwise_xor (1059) > bitwise_and (1077) > shift (1098) > additive (1124)
    > multiplicative (1153, `*` only - no division operator) > unary_expr
    (1171: `!` `~` `-`) > primary_expr (1206). Formula ladder (8 levels):
    `formula` (1226) = or_formula (1230) > impl_formula (1247, `->`
    right-assoc) > and_formula (1264) > unary_formula (1281, passthrough) >
    until_formula (1291, left-assoc) > neg_operand (1311, `!` tighter than
    until) > primary_formula (1325: next / always[N] /
    bounded_eventually[N] / eventually(input_is(E)) / input_is / output_has /
    parenthesized formula / non-boolean fallthrough at 1431 =
    `non_boolean_expr`, the EQUALITY level - so mixed atom kinds like
    `state == S && output_has(O)` parse). Lexical helpers: `identifier`
    (1435), `ident_start`/`ident_cont`/`digit` (1444-1452), `ident_path`
    (1456, `A::B` paths), `keyword` (1472) / `soft_keyword` (1481 - IDENTICAL
    implementations), `int_lit` (1490, u64 overflow -> semantic_err),
    `opt_semi`/`p_tag`/`opt_p_tag`/`p_space0`/`p_space1` (1499-1520). KEYWORDS
    const (21, 28 words) pinned by test `keyword_list_matches_parser_sites`
    (2053) in both directions + the IDE grammar sanity test in
    `tests/frontend_test.rs`. Notable unit tests: mixed-conjunction parsing
    (1564/1592), formula precedence pins (1670, 1824-1883),
    eventually-missing-paren diagnostic (1966), liveness shapes (1943).
  - `src/ebnf_grammar.rs` (902 lines): `RuleNode` enum (23: Terminal /
    NonTerminal / Sequence / Choice / Optional / ZeroOrMore / OneOrMore - all
    const-constructible); `GRAMMAR_RULES` (579) = 61 rules mirroring
    DESIGN.md section 6 exactly (same names, order, structure); `RULE_NOTES`
    (645) one-line descriptions for the md rule table; renderers:
    `to_ebnf_text` (775 - the canonical text of the DESIGN.md section 6 ebnf
    fence), `to_md` (787: intro + ebnf block + rule table + formula-
    precedence note), `to_plantuml_ebnf` (816: @startebnf/@endebnf, quoted
    terminals, `[X]`/`{X}`/`{X}-` suffixes, every rule line ends with `;`),
    `to_svg` (866: railroad diagram via the `railroad` crate). Sync tests in
    `tests/info_format_test.rs`: DESIGN.md section 6 fence <-> model
    structural equality both directions + no duplicates, every KEYWORDS entry
    a terminal, rule-name parity. `rfs info format spec-rfs` prints from this
    model (`src/info_format.rs`).
- **Entry format (house style, cf. ARCHITECTURE.md / ALGORITHMS.md)**: short
  intro paragraph per section; bold lead-in per item; doc-to-code references
  inline in the prose AND a trailing `Code:` line listing the section's key
  references (auditable). One small mermaid flowchart is allowed in the
  entry-error section (parse_source pipeline); house style: `flowchart TD`,
  quoted node labels, unquoted diamond decision nodes without parentheses,
  `-->|label|` edge labels, no `<br/>` - it must render under docs_md_test.
- **Docs gate**: run `cargo test --test dev_docs_test` (fast) after each
  action; `docs_md_test` at task end (it parses every repo `.md` + renders
  every mermaid block).
- **Bookkeeping (established Phase 8 practice)**: mark the plan row's Status
  column DONE in `PHASE_8_PLAN.md` after verification; check off the matching
  exit-criteria boxes in this file; one git commit per action with a detailed
  message; ONE `TASKS.md` entry when the task is complete (template in its
  first paragraph only).
- **Verified doc-to-code references for this task** (declaration lines checked
  2026-10-30): all items listed above under "Verified source material" plus
  `src/error.rs::ParseError`, `src/error.rs::Span`,
  `src/error.rs::line_col`, `src/error.rs::rich_display`,
  `src/ir.rs::IR_VERSION`, `src/ir.rs::Ir`, `src/ir.rs::Module`,
  `src/ir.rs::ConstructorGroup`, `src/ir.rs::Transition`,
  `src/ir.rs::EventPattern`, `src/ir.rs::Formula`, file-level
  `tests/info_format_test.rs`, `tests/frontend_test.rs`,
  `fuzz/fuzz_targets/parser.rs`, `src/info_format.rs`. Non-reference backticks
  (safe): `ide/textmate/rfs.json`, `.rfs`/`.json` paths, shell commands,
  bare identifiers.

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 1. | 1.3 | parser-area skeletons (`docs/dev/parser.md`, `docs/llm/parser.md`) with the fragment anchors + pointers in `src/parser.rs` / `src/ebnf_grammar.rs` |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 1. | 13.3 | final Class B verification/budget pass over all 11 cards (this task's card must be within budget and consistent) |
| 2. | 14 | pointer/doc sync may cross-link from `START.md`/`KB.md` into the filled parser fragments; IDs are stable from now |

### Affected files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `docs/dev/parser.md` | change | replace the TBD with the full deep dive: 5 sections (Overview + 4 new anchored sub-fragments) + unanchored "Changing the grammar" checklist; one mermaid flowchart in the entry-error section |
| `docs/llm/parser.md` | change | replace the TBD with the compact card (responsibility, key-items table, invariants, change-impact pointers, doc links) within the 120-line budget |
| `src/parser.rs` | change | `//! Docs:` line extended with the three new parser fragments (doc comment only, no code) |
| `src/ebnf_grammar.rs` | change | `//! Docs:` line extended with `dev:parser-ebnf-model` (doc comment only, no code) |

## Planned actions

### Action 1: Class A deep dive - `docs/dev/parser.md`

Replace the TBD under `dev:parser-overview` and add the four new sections
(anchor line immediately above each heading). Content per section:

1. **Overview** (`dev:parser-overview`, existing anchor): what the parser is
   (nom-based hand-written recursive descent, single pass - parsing AND IR
   assembly happen together; no separate lexer, lexical helpers are inline);
   entry points `src/parser.rs::parse_source` / `src/parser.rs::parse_file`;
   file map (`src/parser.rs` 2091 lines incl. 29 unit tests, `src/ebnf_grammar.rs`
   902 lines); position in the pipeline (input -> parse -> validate, cf.
   `dev:architecture-pipeline`); cross-links to `dev:algorithms-parser-validation`
   (L1/L2/L3), `dev:deps-fn-parser` (call graph), `dev:code-map-frontend`,
   DESIGN.md sections 5/6/8, `docs/SEMANTICS.md` for what the IR means; the
   parser is fuzzed as a toolchain-fuzz target (`fuzz/fuzz_targets/parser.rs`).
2. **Entry points and error model** (new `dev:parser-entry-error`): the
   parse_source pipeline with the small mermaid flowchart (strip_comments ->
   rfs_source -> trailing-input check -> on failure: dedicated eventually
   diagnostic, else mapped nom error); offset-preserving comment stripping
   (why it matters: error line/col map 1:1 to the original source);
   the two-tier error model (`syntax_err` backtrackable = alt tries other
   branches; `semantic_err` non-backtrackable = aborts immediately) with a
   table of which constructs raise which tier (duplicate next/set-var/action
   -> semantic; unknown action name / malformed syntax -> syntax);
   offset computation (`nom_error_to_parse_error` / `err_at`) and ParseError
   rendering (`line_col`, `rich_display` snippet + caret); the dedicated
   eventually diagnostic (whole-word match, string-literal skipping,
   bounded_eventually exclusion, consulted only after failure).
3. **Module and declaration parsing** (new `dev:parser-grammar`): rfs_source
   -> module (optional quoted version header) -> module_item alt of the 9
   kinds; enum/initial decls; constructor groups (event/output with Unit |
   Struct variants, field lists, types incl. bit<N>) + `merge_group` merging
   repeated group decls (worked example: two `event Recv(...)` lines become
   one group); var decls; transition parsing (`build_transition` requires
   next, rejects duplicate set-vars; action block = next/emit/set/priority);
   prop/assume/on_error decls; `build_module` invariants (transition IDs =
   declaration order, module-level uniqueness, shared prop/assume namespace,
   single initial + single on_error); event patterns (group_pattern bare |
   group(variant), pattern_field bindings, pattern_expr) and output exprs;
   the deliberate leniency (missing commas between list items / missing
   semicolons after actions are tolerated - the canonical printer never emits
   them; cf. the to_md intro note).
4. **Expression and formula ladders** (new `dev:parser-expressions`): the
   12-level expression ladder as a table (level, operator(s), function,
   associativity) from logic_or down to primary_expr (note: multiplicative is
   `*` only - no division); the 8-level formula ladder table (or -> and until
   neg, with the right-assoc `->`, left-assoc `until`, `!` tighter than
   until); the MOST IMPORTANT subtlety emphasized: primary_formula's
   fallthrough to `non_boolean_expr` at the EQUALITY level - each operand of a
   formula-level operator is parsed independently, so mixed atom kinds like
   `state == S && output_has(O)` parse (worked examples from the unit tests);
   keyword vs soft_keyword (identical implementations) + KEYWORDS as the
   single source of truth (28 words; pinned against parser call sites by
   `keyword_list_matches_parser_sites` and against the IDE grammar by the
   frontend test); int_lit u64 overflow -> semantic error; ident_path for
   `A::B` qualified names.
5. **EBNF grammar model** (new `dev:parser-ebnf-model`, in
   `src/ebnf_grammar.rs`): RuleNode's 7 const-constructible variants (why
   slices/references instead of Vec/Box); GRAMMAR_RULES = 61 rules mirroring
   DESIGN.md section 6 rule-for-rule; RULE_NOTES; the four renderers
   (to_ebnf_text = the canonical fence text, to_md, to_plantuml_ebnf with its
   PlantUML-specific emission, to_svg railroad); the sync tests in
   `tests/info_format_test.rs` (fence <-> model both directions + no
   duplicates, KEYWORDS terminal coverage, rule-name parity) and how
   `rfs info format spec-rfs` prints from the model (`src/info_format.rs`).
6. **Changing the grammar** (unanchored closing section): developer checklist
   - add valid + invalid examples FIRST (START.md protocol); change parser +
   GRAMMAR_RULES + DESIGN.md section 6 fence together (sync test fails on
   drift); a new keyword goes into KEYWORDS AND `ide/textmate/rfs.json` (the
   IDE grammar sanity test asserts coverage); verify with the parser unit
   tests (`mod tests`), `cargo test --test examples_test`,
   `cargo test --test info_format_test`; training snippets only if the change
   is language-visible. Every new fragment anchor added in this action gets
   its owner's `//! Docs:` line extended in the SAME action (exact target
  lines in Context).

Run `cargo test --test dev_docs_test` after writing.

### Action 2: Class B card - `docs/llm/parser.md`

Replace the TBD under `llm:parser` with the compact card (keep the budget
header, title, intro line and anchor byte-stable): responsibility (1-2 lines);
key-items table (item -> one-line role) covering parse_source / parse_file /
strip_comments / syntax_err + semantic_err / KEYWORDS / module_item /
build_transition / build_module / merge_group / group_pattern + pattern_field
/ expr / formula / primary_formula + non_boolean_expr fallthrough / int_lit /
GRAMMAR_RULES / the renderers; invariants that must not break (offset
preservation, error-tier semantics, KEYWORDS triple pinning parser+IDE+test,
GRAMMAR_RULES <-> DESIGN.md section 6 sync, transition-ID order, shared
prop/assume namespace, single initial/on_error); change-impact pointers
(grammar change -> examples first, then parser + GRAMMAR_RULES + fence +
KEYWORDS/IDE grammar; verify with examples_test + info_format_test + lib
parser tests; full table in `llm:change-impact` once task 13.2 lands); doc
links to the Class A fragments (link, never restate). Avoid literal pipe
characters inside table cells (use words like "logical or"). Total file must
stay <= 120 lines. Run `cargo test --test dev_docs_test` after writing.

### Action 3: verification + bookkeeping

Run the full gate: `cargo test --test dev_docs_test` (9/9),
`cargo test --test docs_md_test` (5/5 - the new mermaid block must render),
then full verification: `cargo build && cargo test && cargo clippy
--all-targets && cargo fmt --check` (+ `--workspace`). Source edits are doc-
comment lines only, so all cargo counts must be UNCHANGED from the task-7.2
tree (535/535 root, 559/559 workspace). Mark plan row 8.1 DONE in
`PHASE_8_PLAN.md`; check off this file's exit-criteria boxes; append the
scope-decision entry to `PHASE_8_PLAN.md`'s Corrections table (if confirmed at
review); append ONE `TASKS.md` entry; commit per action (Action 1: deep dive +
owner lines; Action 2+3: card + bookkeeping).

## Task deliverables

- `docs/dev/parser.md` filled: Overview + Entry/error model (+ flowchart) +
  Module/declaration parsing + Expression/formula ladders + EBNF model +
  Changing-the-grammar checklist; four new anchored fragments, all owned.
- `docs/llm/parser.md` card filled within its 120-line budget.
- `//! Docs:` lines of `src/parser.rs` + `src/ebnf_grammar.rs` extended
  (doc comments only).
- Plan row 8.1 DONE; Corrections entry; TASKS.md entry.

## Task exit criteria

- [x] `docs/dev/parser.md`: all five anchored sections + the unanchored
      "Changing the grammar" checklist filled, no TBD remains; every
      technical statement carries a resolving doc-to-code reference (gate
      check 1); the existing `dev:parser-overview` anchor + heading unchanged;
      the four new fragments anchored AND owned in the same action.
- [x] `docs/llm/parser.md`: card filled, no TBD, total line count <= 120,
      links to Class A without restating it.
- [x] Source edits limited to the two `//! Docs:` lines (no code changes);
      all cargo counts unchanged from the task-7.2 tree.
- [x] Full verification green: dev_docs_test 9/9, docs_md_test 5/5 (new
      mermaid renders), cargo build/test/clippy/fmt (+ --workspace).
- [x] Plan row 8.1 marked DONE; this file's boxes checked; TASKS.md entry
      appended.

## Non-goals

- No parser/validator behavior changes of any kind; no new dependencies.
- No restatement of ALGORITHMS L1/L2/L3 or DEPENDENCIES graph content (link
  only); no edits to other docs (TASK_MAP full map = task 13.1,
  CHANGE_IMPACT = task 13.2, other cards = their own tasks; START.md/KB.md/
  BUILD.md pointer sync = task 14).
- No DESIGN.md edits (section 6 fence is already in sync - the deep dive
  references it); no new example `.rfs` files.
- No line-number-based anchors anywhere; stable item-name anchors only.

## Subtask 8.2 - validator deep dive (added at execution, see Task corrections)

### Goal

Fill in `docs/dev/validator.md` (Class A deep dive): full content on the
semantic validation + expression typing pass (`src/validator.rs`) -
architecture and the parser/validator/codegen split, the fatal pass with all
per-item checks, the typing system (ExprType lattice, Env, path resolution,
binary/bit rules, formula typing), and the non-fatal warning pass. Also fill
in the matching Class B card `docs/llm/validator.md` (within its 120-line
budget) - per the confirmed scope decision (plan Corrections entry 3).

### Context

- **Skeleton state (from task 1.4)**: `docs/dev/validator.md` exists with one
  anchored fragment `dev:validator-overview` (owner: `src/validator.rs` - its
  `//! Docs:` line already names it) followed by a TBD line; `docs/llm/validator.md`
  exists with budget header `<!-- llm-budget: 120 -->`, fragment `llm:validator`
  (same owner), and a TBD line. Existing anchor + heading stay byte-stable.
- **Fragment structure**: keep `dev:validator-overview` for Overview; add
  three new sub-fragments, ALL owned by `src/validator.rs`:
  - `dev:validator-fatal-pass` - Fatal validation pass
  - `dev:validator-typing` - Expression and formula typing
  - `dev:validator-warnings` - Non-fatal diagnostics (warnings)
  ORPHAN RULE: the new IDs go on `src/validator.rs`'s `//! Docs:` line in the
  same action. Exact target line:
  `//! Docs: dev:validator-overview, dev:validator-fatal-pass, dev:validator-typing, dev:validator-warnings, dev:deps-fn-validator, dev:algorithms-parser-validation, llm:validator`
- **No duplication with existing fragments**: ALGORITHMS.md
  `dev:algorithms-parser-validation` L2 already lists the fatal invariants +
  warning classes (one line each); DEPENDENCIES.md `dev:deps-fn-validator` has
  the call graph. The deep dive goes DEEPER: per-check detail with exact error
  message shapes, the full typing rule set, the priority/ambiguity ladder's
  error-vs-warning split, and the warning algorithms. Cross-link; never
  restate.
- **Verified source material (measured 2026-10-30; line numbers for
  navigation only - doc references use item names)**: `src/validator.rs`
  (1239 lines; production 1-935, `mod tests` 937-1239 with 24 tests):
  `ValidationError` (8: message ONLY - no span; semantic errors carry
  line/col 0 in `validate --json`, cf. `src/frontend.rs::StructuredError`);
  `ExprType` (21: Bool | Bit(u32) | IntLit | Enum(String)); `Env` (29: vars,
  bindings, binding_idents - codegen mangle map, EMPTY from the validator's
  side, state_variants, enums) + `for_module` (43: State enum ->
  state_variants; all enums -> variant sets) + `with_binding` (66);
  `type_to_expr_type` (72); `validate_ir` (80: all-errors collection over
  modules, never fail-fast); `is_valid_version_format` (95: X.Y or X.Y.Z,
  non-empty all-digit components, no regex dep - D-P6-1); `validate_module`
  (103) check order: State enum non-empty -> initial state is a variant /
  required when transitions exist -> version header format (parser accepts
  any quoted string; this is the semantic gate) -> var init typing (loose:
  literal may adapt to var type) -> per transition (state + next_state are
  variants; event pattern via `validate_event_pattern` which POPULATES
  env.bindings left-to-right, duplicate bindings rejected; guard types AND is
  bool; set targets exist + values type-check; emits via
  `validate_output_expr`) -> priority/ambiguity ladder (per state, pairwise:
  duplicate priorities are an error ONLY for identical patterns - numbers may
  be reused across different patterns; identical pattern + identical guard
  with no priorities = shadowed-duplicate ERROR) -> on_error (next is a
  variant; emits checked WITHOUT pattern bindings) -> prop/assume names
  unique across BOTH lists (D-P3-4: generated monitor struct names would
  collide) -> per formula: `type_formula` + `bounded_always_error` (308:
  always[N] requires N >= 1, nested occurrences included;
  bounded_eventually[0] stays legal = "body must hold at the trigger sample").
  Patterns: `validate_event_pattern` (336: group exists; variant exists;
  unit/struct shape match; field counts equal; per-field literal
  compatibility via `pattern_expr_compatible` (406: wildcard/bind always ok;
  int fits bit<N> via u64::MAX for width >= 64; int vs bool/enum rejected;
  bool only vs bool; enum variant path must start with the enum name and be
  >= 2 segments)); `validate_output_expr` (444: group exists; bare output ok
  ONLY for single-variant groups; variant exists; arg count = field count;
  args type-check + compatible_with field types). Typing: `type_expr` (505:
  Int -> IntLit, Bool -> Bool, Path -> resolve_path, unary: ! requires bool,
  ~ and - require bitvector with IntLit adapting to Bit(8), binary ->
  binary_type); `resolve_path` (537: `state` special-cased to Enum("State");
  precedence bindings > vars > State variants; qualified paths exactly 2
  segments Enum::Variant, longer rejected); `binary_type` (578: logical ops
  require bool operands -> Bool; comparisons via compatible_with -> Bool;
  Plus/Minus/Star -> bit_op_type; shifts: left Bit(w) or IntLit->8, right may
  be bit or literal - NO strict width equality; BitAnd/BitOr allow bool+bool
  -> Bool, else bit_op_type; BitXor -> bit_op_type); `bit_op_type` (611:
  Bit(a)+Bit(b) requires a == b; IntLit adapts to the other side's width;
  IntLit+IntLit -> Bit(8)); `impl ExprType` (626: must_bool, compatible_with
  - IntLit<->Bit ok, equal ok, IntLit vs Bool error, else mismatch);
  `is_safety_formula` (646: bounded_eventually/eventually/until are NOT safety
  (D-P4-8); public utility for MC planning/tests); `type_formula` (663: Expr
  must type as bool; InputIs/OutputHas -> check_pattern_group (692, Phase 1
  stub: formula patterns do NOT bind variables - struct fields with names are
  rejected) / check_output_pattern_group (708, stub)); warnings:
  `module_warnings` (789) five classes: unreachable states (DFS from initial
  over transitions, guards ignored = over-approximation), dead events
  (variant never referenced by any pattern; a bare group pattern marks ALL
  its variants used), unused variables (never read via collect_names (720)
  in guards/emit args incl. on_error emits, and never assigned),
  order-dependent pairs (same state + identical pattern, no priorities,
  DIFFERING guards - first match wins in source order; suggests `priority N`),
  impossible guards (`is_literal_false` (737) or `contradictory_equalities`
  (743): SYNTACTIC only - top-level `&&` conjuncts, var == int-literal pairs,
  same var bound to two different literals). Notable unit tests:
  duplicate_priority_same_pattern_is_error / _different_patterns_is_allowed
  (997/1008), shadowed_duplicate_transition_is_error (1016),
  order_dependent_guard_pair_is_warning_only (1026), unreachable_state_is_
  warning (1038), dead_event_is_warning (1049), unused_var_is_warning
  (1060), literal_false_guard_is_warning (1071),
  contradictory_equalities_guard_is_warning (1082),
  on_error_unknown_state_is_error (1091), bounded_always_zero_bound_is_error
  + _nested (1103/1113), bounded_eventually_zero_bound_is_allowed (1125).
- **Pinned error messages**: `tests/frontend_test.rs`
  e2e_validate_json_semantic_error_shape_pinned pins the EXACT message text
  "module 'always_zero_bound' property 'bad': always[N] requires N >= 1"
  (kind "semantic", line/col 0) - changing a validator message shape breaks
  that pin; the examples/invalid corpus (13 files) must keep failing.
- **Entry format / gate rules / bookkeeping**: same as subtask 8.1 above
  (house style with per-section Code: lines; doc-to-code reference rules;
  run dev_docs_test after each action, docs_md_test at task end; plan row ->
  DONE, boxes checked, one TASKS.md entry, commit per action).
- **Verified doc-to-code references for this subtask** (declaration lines
  checked 2026-10-30): all items listed above plus `src/frontend.rs::validate_
  one_with_warnings`, `src/frontend.rs::ValidateOneResult`,
  `src/frontend.rs::StructuredError`, `src/frontend.rs::run_validate`,
  `src/ir.rs::Module`, `src/ir.rs::Transition`, `src/ir.rs::Formula`,
  `src/ir.rs::EventPattern`, file-level `tests/frontend_test.rs`.

### Planned actions

1. **Class A deep dive - `docs/dev/validator.md`**: replace the TBD and add
   the three new sections (anchor line immediately above each heading):
   - **Overview** (`dev:validator-overview`): two-stage frontend split - what
     the parser already checked (syntax, structural uniqueness at parse time)
     vs what the validator checks (semantics + typing); entry points
     `validate_ir` (fatal, all errors collected) and `module_warnings`
     (non-fatal, independent pass); the `ValidationError` model (message only,
     no span - semantic errors carry line/col 0 in `validate --json`, unlike
     parse errors which have spans); the error/warning flow through
     `src/frontend.rs::validate_one_with_warnings` (errors block codegen and
     are all surfaced per file; warnings never block); what codegen may rely
     on (well-typed expressions, known states/vars/patterns, unique
     prop/assume names for monitor struct names, transition IDs in declaration
     order); cross-links to `dev:algorithms-parser-validation`,
     `dev:deps-fn-validator`, `dev:code-map-frontend`, DESIGN.md section 13,
     `docs/SEMANTICS.md`.
   - **Fatal validation pass** (`dev:validator-fatal-pass`): the
     `validate_module` check order as a numbered list with per-check detail
     and exact error message shapes (module 'X' ... prefixes); the
     priority/ambiguity ladder's error-vs-warning split explained (duplicate
     priorities only conflict for identical patterns; shadowed duplicates are
     errors; differing guards without priorities are only a warning);
     version-header format check (D-P6-1: parser accepts any quoted string,
     validator is the semantic gate, legacy specs without a header stay
     valid); `bounded_always_error` (N >= 1 nested; bounded_eventually[0]
     legal and what it means).
   - **Expression and formula typing** (`dev:validator-typing`): the ExprType
     lattice with IntLit adaptation rules (lit+lit -> Bit(8)); `Env` fields
     incl. binding_idents (codegen mangle map, empty from the validator's
     side) and how pattern validation populates bindings left-to-right;
     `resolve_path` precedence (`state` special-cased; bindings > vars >
     State variants; qualified paths exactly 2 segments); the full
     `binary_type`/`bit_op_type` rule set (width equality, IntLit adaptation,
     shift rules with no strict width equality on the right, bool+bool for
     BitAnd/BitOr); `compatible_with`; `type_formula` + the Phase 1 stubs
     (formula patterns do not bind variables); `is_safety_formula` (D-P4-8)
     and who consumes it.
   - **Non-fatal diagnostics** (`dev:validator-warnings`): the five warning
     classes with their algorithms (DFS reachability over-approximation,
     dead-event counting incl. bare-group semantics, unused-var collection
     incl. on_error emits, order-dependent pairs, syntactic impossible-guard
     detection and WHY it is syntactic-only); warnings never block codegen.
   - **Changing validation rules** (unanchored closing section): developer
     checklist - add invalid examples FIRST (file-driven suite), update
     `validate_module`/`module_warnings`, error message shapes are pinned by
     tests (frontend_test semantic-shape pin + the 24 unit tests in `mod
     tests`), keep the 13-file invalid corpus failing, docs sync (DESIGN.md
     section 13, KB.md), training snippets if user-visible.
   Run `cargo test --test dev_docs_test` after writing; extend
   `src/validator.rs`'s `//! Docs:` line in the same action.
2. **Class B card - `docs/llm/validator.md`**: replace the TBD (keep budget
   header, title, intro, anchor byte-stable): responsibility; key-items table
   (validate_ir / validate_module / is_valid_version_format /
   validate_event_pattern / validate_output_expr / type_expr / resolve_path /
   binary_type + bit_op_type / type_formula / is_safety_formula /
   bounded_always_error / module_warnings / Env / ExprType); invariants that
   must not break (all-errors collection never fail-fast; errors block /
   warnings never do; bindings populated left-to-right and unique;
   prop/assume shared namespace; message shapes pinned by tests); change-
   impact pointers (invalid examples first; verify with cargo test --lib
   validator, cargo test --test examples_test, cargo test --test
   frontend_test; full table in llm:change-impact at task 13.2); deep-dive
   links to the four Class A fragments. Total <= 120 lines. Run the gate.
3. **Verification + bookkeeping**: dev_docs_test 9/9, docs_md_test 5/5,
   full verification (build/test/clippy/fmt + workspace; counts unchanged:
   535/535 root, 559/559 workspace); plan row 8.2 -> DONE; check off this
   subtask's boxes; TASKS.md entry; commit per action.

### Exit criteria (subtask 8.2)

- [x] `docs/dev/validator.md`: all four anchored sections + the unanchored
      "Changing validation rules" checklist filled, no TBD remains; every
      technical statement carries a resolving doc-to-code reference; the
      existing `dev:validator-overview` anchor + heading unchanged; the three
      new fragments anchored AND owned in the same action.
- [x] `docs/llm/validator.md`: card filled, no TBD, total line count <= 120,
      links to Class A without restating it.
- [x] Source edits limited to `src/validator.rs`'s `//! Docs:` line (no code
      changes); all cargo counts unchanged from the task-8.1 tree.
- [x] Full verification green: dev_docs_test 9/9, docs_md_test 5/5, cargo
      build/test/clippy/fmt (+ --workspace).
- [x] Plan row 8.2 marked DONE; this subtask's boxes checked; TASKS.md entry
      appended.

### Non-goals (subtask 8.2)

- No validator behavior changes of any kind; no new dependencies.
- No restatement of ALGORITHMS L2 invariant lists or DEPENDENCIES graph
  content (link only); no edits to other docs (ir_json_rw deep dive = task
  8.3; START.md/KB.md/BUILD.md pointer sync = task 14).
- No DESIGN.md edits; no new example `.rfs` files.

## Task corrections

| Correction | Description + rationale |
|------------|-------------------------|
| Subtask 8.2 added to this task file at execution (2026-10-30) | The file was initially written for plan row 8.1 only (user request "prepare to execute task8, subtask 8.1"); plan rows 8.1-8.3 are the group-8 deep dives and the user's model is one task file per plan group ("task 8.2" = subtask of task file 8). The 8.2 section above was appended after 8.1 completed, following template 2.4 internally; the frozen 8.1 sections are kept as the historical record. |