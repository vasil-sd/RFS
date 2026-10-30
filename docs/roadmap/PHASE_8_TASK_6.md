# Phase 8, Task 6 - Implementation details

> Written for a future LLM session: after reading `START.md`,
> `docs/roadmap/INTRO.md` and this file, the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal

Fill in `docs/dev/ALGORITHMS.md` (Class A): main-algorithm write-ups at
three levels of detail - L1 abstract high-level view, L2 key data structures
and invariants, L3 optional implementation details (important ones
emphasized) - every level linked to the implementing code. Sections: parser
construction + validation passes (6.1), IR normalization + codegen pipeline
(6.2), MC engine exact/liveness/SMT paths (6.3), monitor compilation / PSL
translation (6.4).

## Context

- **Skeleton state (from task 1.2)**: `docs/dev/ALGORITHMS.md` exists with
  four UNANCHORED sections, each followed by a TBD line naming its filling
  task ("TBD (task 6.x). Fragment anchor added in task 6.x."). Unlike task 5
  (whose fragments were pre-owned by `src/lib.rs`), EACH subtask here must
  add its own `<a id="dev:..."></a>` anchor line immediately above the
  heading AND name the new fragment on the owning source files' `//! Docs:`
  lines (orphan rule: every fragment needs at least one owner).
- **Fragment IDs + owners**: naming convention `dev:algorithms-<topic>`.
  6.1 uses `dev:algorithms-parser-validation` with owners `src/parser.rs`
  and `src/validator.rs` (the files whose changes make the fragment stale).
  For 6.2-6.4 pick the ID + owners at execution time the same way (candidates:
  6.4 `dev:algorithms-monitor-compile` owned by `src/monitor.rs`).
  DECIDED at 6.3 execution: `dev:algorithms-mc-engine` is owned by ALL FOUR
  area files - `src/mc_plan.rs`, `src/ref_mc.rs`, `src/smt_plan.rs`,
  `src/smt_emit.rs` (smt_emit joined the candidate list: its leaf-variable
  convention is part of the fragment's content).
  DECIDED at 6.2 execution: `dev:algorithms-ir-codegen` is owned by ALL
  SEVEN area files - `src/ir.rs`, `src/json_rw.rs`, `src/spec_hash.rs`
  (IR trio) + `src/codegen.rs`, `src/codegen_fsm.rs`, `src/codegen_mc.rs`,
  `src/codegen_test.rs` (codegen quartet) - matching the area-fragment
  pattern of `dev:ir-json-rw-overview` and `dev:codegen-core-overview`:
  every file whose change could stale the fragment names it.
- **PLAN WORDING CORRECTION (recorded in Task corrections)**: plan line 6.1
  says "parser construction (LL(1)/chumsky)". The shipped parser is NOT
  chumsky - it is a hand-written recursive-descent parser on nom 8
  combinators (`src/parser.rs` module doc: "RFS v0 source parser
  (nom-based)"; `Cargo.toml`: `nom = "8.0.0"`). The LL(1) part holds in the
  sense that every grammar decision point is determined by a single
  lookahead token. ALGORITHMS.md must describe what is shipped.
- **Source material for 6.1 (measured 2026-10-30)**:
  - `src/parser.rs` (2091 lines; production code 1-1522, tests 1524-2091,
    29 tests): entry `parse_source` (73) = strip_comments -> rfs_source ->
    trailing-input check -> dedicated eventually diagnostic. Grammar fns:
    rfs_source (221), module (232, optional version header), module_item =
    alt of 9 decls (265: enum/initial/event/output/var/transition/prop/
    assume/on_error), enum_variants (289), event_decl (322) +
    opt_constructor_payload (354) + constructor_variant (365) + field_list
    (377), type_parser (402) / bit_type (414), var_decl (423),
    transition_decl (438) + build_transition (461, requires `next`, rejects
    duplicate set-vars) + action_block (491) + action (512: next/emit/set/
    priority, duplicates -> semantic_err, unknown name -> syntax_err),
    prop_decl (564) / assume_decl (576), on_error_decl (588, requires
    `next`), build_module (638: transition IDs, module-level uniqueness,
    prop/assume shared namespace), merge_group (698), event_pattern (726)
    / group_pattern (736) / pattern_field (783) / pattern_expr (844),
    output_expr (872). Expression levels: expr (924) = logic_or (928) >
    logic_and (946) > equality (964) > relational (990) > bitwise_or (1038)
    > bitwise_xor (1059) > bitwise_and (1077) > shift (1098) > additive
    (1124) > multiplicative (1153, `*` only - no division operator) >
    unary_expr (1171: `!` `~` `-`) > primary_expr (1206). Formula levels:
    formula (1226) = or_formula (1230) > impl_formula (1247, `->`
    right-assoc) > and_formula (1264) > unary_formula (1281, passthrough)
    > until_formula (1291, left-assoc) > neg_operand (1311, `!` tighter
    than until) > primary_formula (1325: next/always[N]/bounded_eventually[
    N]/eventually/input_is/output_has/(formula)/non_boolean_expr fallthrough
    at 1431 = equality level). Helpers: identifier (1435), ident_path
    (1456, `A::B` paths), keyword (1472) / soft_keyword (1481, IDENTICAL
    implementations), int_lit (1490, u64 overflow -> semantic_err). Error
    classes: syntax_err = nom Err::Error backtrackable (214), semantic_err
    = nom Err::Failure non-backtrackable (207); nom_error_to_parse_error
    (139) / err_at (151) compute point offsets; find_eventually_missing_
    paren (161) post-failure dedicated diagnostic. KEYWORDS const (21, 28
    words) pinned by test keyword_list_matches_parser_sites (2053).
  - `src/validator.rs` (1239 lines; production 1-978, tests to 1239, 24
    tests): validate_ir (80, all-errors collection over modules),
    is_valid_version_format (95, X.Y or X.Y.Z without a regex dep),
    validate_module (103: State-enum non-empty, initial checks, version
    header, var-init typing, per-transition state/next_state + event
    pattern + guard (bool) + assignments (var exists, value type-checks) +
    emits, per-state priority/ambiguity ladder, on_error checks, prop/
    assume name uniqueness across both lists, formula typing +
    bounded_always_error (308, N >= 1, nested)), module_warnings (789:
    unreachable states DFS guards-ignored, dead events, unused vars,
    order-dependent guard pairs, impossible guards via is_literal_false
    (737) + contradictory_equalities (743, syntactic top-level `&&`
    equalities)). Typing: Env (29) + for_module (45) + with_binding (66),
    ExprType (21: Bool/Bit(w)/IntLit/Enum(name)), type_to_expr_type (72),
    type_expr (505), resolve_path (537: `state` special-cased, bindings >
    vars > state variants, qualified paths exactly 2 segments), binary_type
    (578), bit_op_type (611: width equality, IntLit adapts, lit+lit ->
    Bit(8)), impl ExprType (626: must_bool, compatible_with). Patterns:
    validate_event_pattern (336, populates env.bindings left-to-right,
    duplicate bindings rejected), pattern_expr_compatible (406, literal
    fits bit<N> via u64::MAX for width >= 64), validate_output_expr (444,
    bare output ok only for single-variant groups). Formulas: type_formula
    (663), check_pattern_group (692, no bindings in Phase 1 formulas),
    is_safety_formula (646, D-P4-8).
  - `src/frontend.rs`: run_validate (566) / validate_one_with_warnings
    (345) = parse -> validate_ir -> module_warnings; errors block,
    warnings never do; all errors per file surfaced (FailedFile.errors is a
    Vec). validate_one_structured (392): parse errors carry line/col from
    ParseError::line_col, semantic errors line/col 0.
  - `src/ebnf_grammar.rs`: GRAMMAR_RULES (579) = 61 rules mirroring
    DESIGN.md section 6 rule-for-rule; sync tests in
    `tests/info_format_test.rs` pin fence<->table structural equality and
    KEYWORDS subset of terminals.
  - `src/error.rs`: Span (8), ParseError (28) with line_col (48) +
    rich_display (source snippet + caret).
- **Entry format (house style, cf. `docs/dev/ARCHITECTURE.md` inline refs
  + `docs/dev/DECISIONS.md` Code: lines)**: per section - short intro
  paragraph; `### L1 - Abstract view`, `### L2 - Key data structures and
  invariants`, `### L3 - Implementation details (important ones
  emphasized)` subsections; bold lead-in per item; doc-to-code references
  inline in the prose AND a trailing `Code:` line listing the level's key
  references (makes the "every level linked" requirement auditable).
- **doc-to-code reference rules (gate check 1)**: a backticked string shaped
  like `path/file.rs` or `path/file.rs::item` MUST resolve to an existing
  file and (for `.rs` + `::item`) to an item declared in that file - the
  gate matches declaration lines (`fn`/`struct`/`enum`/`trait`/`type`/
  `const`/`static`/`mod`/impl headers; private items are fine). VERIFY every
  new reference against a declaration line before writing it (grep). Plain-
  text mentions without backticks are exempt. Backticked strings WITHOUT a
  path separator (`IResult<&str, T>`, `u32::try_from`, `bit<N>`,
  `state == S && output_has(O)`) do not match the reference shape and are
  safe.
- **Docs gate**: `docs_md_test` parses every repo `.md` (comrak GFM +
  mermaid render); ALGORITHMS.md sections are prose - no mermaid blocks
  needed in 6.1 (later sections may add diagrams; keep them renderer-safe).
  Run `cargo test --test dev_docs_test` (fast) after each subtask;
  `docs_md_test` at task end.
- **Bookkeeping (established Phase 8 practice)**: mark the plan row's
  Status column DONE in `PHASE_8_PLAN.md` after each verified subtask;
  check off the matching exit-criteria boxes in this file; one git commit
  per subtask with a detailed message; ONE `TASKS.md` entry when task 6 as
  a whole is complete (after 6.4).
- **Verified doc-to-code references for 6.1** (declaration lines checked
  2026-10-30): parser - `src/parser.rs::parse_source`,
  `src/parser.rs::strip_comments`, `src/parser.rs::rfs_source`,
  `src/parser.rs::module_item`, `src/parser.rs::semantic_err`,
  `src/parser.rs::syntax_err`, `src/parser.rs::build_module`,
  `src/parser.rs::merge_group`, `src/parser.rs::logic_or`,
  `src/parser.rs::relational`, `src/parser.rs::until_formula`,
  `src/parser.rs::neg_operand`, `src/parser.rs::non_boolean_expr`,
  `src/parser.rs::find_eventually_missing_paren`, `src/parser.rs::keyword`,
  `src/parser.rs::KEYWORDS`, `src/parser.rs::int_lit`; validator -
  `src/validator.rs::validate_ir`, `src/validator.rs::validate_module`,
  `src/validator.rs::module_warnings`, `src/validator.rs::Env`,
  `src/validator.rs::ExprType`, `src/validator.rs::type_expr`,
  `src/validator.rs::bit_op_type`, `src/validator.rs::validate_event_pattern`,
  `src/validator.rs::check_pattern_group`,
  `src/validator.rs::contradictory_equalities`,
  `src/validator.rs::is_safety_formula`; pipeline -
  `src/frontend.rs::validate_one_with_warnings`,
  `src/frontend.rs::run_validate`, `src/error.rs::ParseError`,
  `src/ebnf_grammar.rs::GRAMMAR_RULES`.

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 1. | 1.2 | ALGORITHMS.md skeleton with the four TBD sections (anchors added per subtask) |
| 2. | 1.4 | parser/validator area pointers exist, so the new fragment can be owned by `src/parser.rs` / `src/validator.rs` |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 1. | 7.1 | wires REFERENCES.md links INTO the ALGORITHMS parser/validation sections (IDs stable from now) |
| 2. | 14 | pointer/doc sync may cross-link from `START.md`/`KB.md` into the filled ALGORITHMS fragments |

## Affected files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `docs/dev/ALGORITHMS.md` | change | replace the four TBD sections with L1/L2/L3 write-ups (one section per subtask 6.1-6.4) + one new anchor per section |
| owner source files (`src/parser.rs`, `src/validator.rs` for 6.1; others per subtask) | change | add the new fragment ID to the `//! Docs:` line ONLY (doc comment, no code) |

## Planned actions

### Action 6.1: `dev:algorithms-parser-validation` - parser construction + validation passes

Replace the TBD under "Parser construction + validation passes" with:
intro paragraph (two-stage frontend, nom-based recursive descent, EBNF
mirror in GRAMMAR_RULES); L1 abstract view (parse pass + validate pass,
errors vs warnings); L2 key structures/invariants (grammar-as-functions,
LL(1)-style single-lookahead decision points, two error classes
backtrackable/non-backtrackable, offset-preserving comment stripping, IR
assembly inside the parse, Env, ExprType lattice, fatal invariants list,
warning diagnostics list); L3 implementation details with the important
ones emphasized (formula fallthrough truncation to the equality level -
most important subtlety; precedence-climbing tables for expr + formula;
multi-char operator guards; dedicated eventually diagnostic; keyword vs
soft_keyword + KEYWORDS pinning; loop-based list parsing; u64 literals;
all-errors collection; deliberately loose assignment typing; pattern
validation populating bindings; priority/ambiguity ladder; syntactic
impossible-guard detection; is_safety_formula; semantic errors carry no
span). Every level ends with a `Code:` line. Add the anchor above the
heading; add the fragment ID to the `//! Docs:` lines of `src/parser.rs`
and `src/validator.rs`. Run the gate.

### Action 6.2: `dev:algorithms-ir-codegen` - IR normalization + codegen pipeline

Same L1/L2/L3 shape for: IR as the single normalized form (versioning,
canonical JSON, spec hash), codegen pipeline (backend plugin registry,
shared helpers, per-backend emission, test generation). Read `src/ir.rs`,
`src/json_rw.rs`, `src/spec_hash.rs`, `src/codegen.rs`,
`src/codegen_fsm.rs`, `src/codegen_mc.rs`, `src/codegen_test.rs` first.
Pick owners at execution (candidates `src/ir.rs` + `src/codegen.rs`).

### Action 6.3: `dev:algorithms-mc-engine` - MC engine exact / liveness / SMT paths

Same shape for: shared plan computation (level-synchronous BFS, input
abstraction), exact checker codegen, liveness decomposition + fairness,
independent reference MC, SMT bounded path (k-step unrolling, PASS always
incomplete). Read `src/mc_plan.rs`, `src/ref_mc.rs`, `src/smt_plan.rs`,
`src/smt_emit.rs` first. Owners: those files.

### Action 6.4: `dev:algorithms-monitor-compile` - monitor compilation (PSL translation)

Same shape for: formula -> check-tree compilation (compile_formula /
compile_check / compile_signal), R1-R4 subset restrictions, the reference
interpreter semantics (pending model, deadline confirmation). Read
`src/monitor.rs` first. Owner: `src/monitor.rs`. Run the full gate
(dev_docs_test + docs_md_test).

## Task exit criteria

- [x] 6.1: parser+validation section filled (L1/L2/L3, every level with a
      Code: line); anchor added; fragment owned by `src/parser.rs` +
      `src/validator.rs`; no TBD in that section; gate green.
- [x] 6.2: IR+codegen section filled the same way; no TBD; gate green.
- [x] 6.3: MC engine section filled the same way; no TBD; gate green.
- [x] 6.4: monitor compilation section filled the same way; no TBD anywhere
      in ALGORITHMS.md; full gate green (dev_docs_test + docs_md_test).
- [x] Every doc-to-code reference resolves (gate check 1); each new fragment
      owned (no orphans); source edits limited to `//! Docs:` lines.
- [x] Plan rows 6.1-6.4 marked DONE; this file's boxes checked; one git
      commit per subtask; `TASKS.md` entry appended after 6.4.
- [x] Full verification green at task end: cargo build/test/clippy/fmt
      (+ --workspace).

## Non-goals

- No mermaid diagrams in 6.1 (prose section); no Class B changes; no
  restatement of the DESIGN.md section 6 grammar table (link only);
  no REFERENCES.md links yet (task 7.1 wires them); no parser/validator
  behavior changes of any kind.

## Task corrections

| Correction | Description + rationale |
|------------|-------------------------|
| 1 | Plan line 6.1 parenthetical "(LL(1)/chumsky)" is stale: the shipped parser is a nom-based hand-written recursive-descent parser, not chumsky; documented as LL(1)-style single-lookahead recursive descent on nom 8 combinators | The implementation (`src/parser.rs` module doc "RFS v0 source parser (nom-based)", `Cargo.toml` `nom = "8.0.0"`) predates the plan wording; ALGORITHMS.md must describe what is shipped, and the LL(1) characterization holds for the decision points |
