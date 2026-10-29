# RFS Algorithms

Main-algorithm write-ups at three levels of detail - L1 abstract high-level
view, L2 key data structures and invariants, L3 optional implementation
details (important ones emphasized) - every level linked to the implementing
code. External references are collected in `REFERENCES.md` and linked from
the relevant sections.

<a id="dev:algorithms-parser-validation"></a>
## Parser construction + validation passes

The frontend turns spec text into a validated IR in two stages: a
hand-written recursive-descent parser built on nom 8 combinators
(`src/parser.rs`) followed by whole-module semantic validation and
expression typing (`src/validator.rs`). The EBNF grammar the parser
implements is mirrored as data in `src/ebnf_grammar.rs::GRAMMAR_RULES`
(61 rules, same names and order as DESIGN.md section 6) and pinned to
the parser by sync tests. Pipeline entry:
`src/frontend.rs::validate_one_with_warnings`.

### L1 - Abstract view

**Parse.** One pass over a comment-stripped copy of the source: the
entry rule parses zero or more modules; each module is a sequence of
nine declaration kinds (enum, initial, event, output, var, transition,
prop, assume, on_error) assembled into an `Ir` document. There is no
separate lexer - tokens are recognized inline by the grammar functions,
and structural duplicates (a second `initial`, repeated names) are
rejected during assembly. The parse must consume the whole input;
otherwise a "trailing input" error is reported at the first unconsumed
byte.

**Validate.** For each module, one pass collects ALL fatal errors -
name resolution against the module's own declarations, expression
typing in a per-module environment, pattern/constructor conformance,
priority-conflict detection, formula typing and bound rules - never
fail-fast. A second, independent pass computes non-fatal warnings
(unreachable states, dead events, unused variables, order-dependent
guard pairs, impossible guards). Errors block code generation;
warnings never do.

Code: `src/parser.rs::parse_source`, `src/validator.rs::validate_ir`,
`src/validator.rs::module_warnings`,
`src/frontend.rs::validate_one_with_warnings`

### L2 - Key data structures and invariants

**Grammar-as-functions.** Every EBNF rule is a Rust function returning
`IResult<&str, T>`; the entry is `rfs_source = many0(module)`. The
machine-readable mirror of the same grammar is
`src/ebnf_grammar.rs::GRAMMAR_RULES`; sync tests enforce that the
DESIGN.md section 6 fence and the parser's keyword list agree with it.

**LL(1)-style decision points.** Every choice in the grammar is decided
by a single lookahead token: the nine `module_item` alternatives each
start with a distinct keyword; `type_parser` (bool / bit<N> / enum
name), `pattern_field` (_ / int literal / true / false / identifier)
and `primary_formula` (temporal keywords / "(" / expression
fallthrough) likewise. Multi-character operators are disambiguated by
inspecting the byte after a one-character match (see L3).

**Two error classes.** `syntax_err` produces nom's backtrackable
`Err::Error` (an enclosing `alt` tries the next alternative);
`semantic_err` produces non-backtrackable `Err::Failure` (a definitive
rejection - duplicate names, a missing required action - that must not
be re-attributed to another alternative). Invariant: anything about the
structure of an already-recognized construct is a semantic error.

**Offset preservation.** `strip_comments` replaces comment bytes with
spaces instead of deleting them, so byte offsets in the cleaned text
equal those in the original file; every parse error carries a point
span that maps 1:1 to user coordinates (`src/error.rs::ParseError`).

**IR assembly inside the parse.** `build_module` assigns sequential
transition IDs and enforces module-level uniqueness (enum/var names,
one initial state, one on_error block, prop/assume names shared across
both lists); `merge_group` merges repeated `event Name(...)` /
`output Name(...)` declarations into single constructor groups.

**Validation environment.** `Env` holds per-module name resolution:
`vars` (declared variables to types), `bindings` (pattern-binding
names to types, populated by the validator while walking each
transition's event pattern so guards can reference them),
`binding_idents` (codegen mangle map: RFS binding name to the
identifier actually emitted when a binding collides with an
unqualified identifier in the generated body), `state_variants`, and
`enums` (name to variant set). Built once per module by
`Env::for_module`; the validator extends `bindings` in place per
transition, codegen fills `binding_idents`.

**Expression type lattice.** `ExprType = Bool | Bit(width) | IntLit |
Enum(name)`. `IntLit` is an untyped literal that adapts to any bit
width in arithmetic (literal + literal -> Bit(8)); the compatibility
rules live in `compatible_with` / `bit_op_type`.

**Fatal invariants (errors).** State enum non-empty; initial state is a
State variant (and required when transitions exist); version header, if
present, matches X.Y or X.Y.Z; every transition's state and next_state
are State variants; event patterns conform to their declared
constructor (variant exists, field counts equal, per-field literal
compatibility, no duplicate bindings); guards type-check and are Bool;
`set` targets exist and values type-check; emit arguments match the
output constructor's arity and types; on_error's next state is a State
variant and its emits check out without pattern bindings; prop/assume
names unique across both lists; every formula type-checks as boolean;
`always[N]` has N >= 1 (nested occurrences included).

**Non-fatal diagnostics (warnings).** Unreachable states (DFS from the
initial state over transitions, guards ignored - an over-approximation);
dead events (constructor never referenced by any pattern); unused
variables (never read or written); order-dependent pairs (same state +
pattern, no priorities, different guards - first match wins in source
order); impossible guards (literal false, or a variable equal to two
different integer literals under top-level `&&`).

Code: `src/parser.rs::rfs_source`, `src/parser.rs::module_item`,
`src/parser.rs::semantic_err`, `src/parser.rs::strip_comments`,
`src/parser.rs::build_module`, `src/parser.rs::merge_group`,
`src/validator.rs::Env`, `src/validator.rs::ExprType`,
`src/validator.rs::validate_module`,
`src/validator.rs::module_warnings`,
`src/ebnf_grammar.rs::GRAMMAR_RULES`, `src/error.rs::ParseError`

### L3 - Implementation details (important ones emphasized)

**Parser.**

1. **The formula fallthrough is deliberately truncated (the most
   important parser subtlety).** A bare operand in a formula position
   is parsed by `non_boolean_expr` - the equality level of the
   expression grammar, NOT the full `expr`. Using `logic_or` /
   `logic_and` there would mis-consume identifier-looking formula atoms
   as variable paths: in `state == S && output_has(O)` the old
   fallthrough absorbed `output_has` as an `Expr::Path` and left `(O)`
   as trailing input. Formula-level `&&` / `||` are handled by
   `and_formula` / `or_formula`, each operand parsed independently via
   `primary_formula`, so `output_has(...)` reaches its keyword branch.
   Pinned by the mixed-conjunction/disjunction tests in
   `src/parser.rs`.
2. **Precedence climbing.** Expression levels, loosest to tightest:
   logic_or > logic_and > equality > relational > bitwise_or >
   bitwise_xor > bitwise_and > shift > additive > multiplicative >
   unary (`!` `~` `-`) > primary; each binary level left-folds in a
   loop (left-associative). Formula levels: or_formula > impl_formula
   (`->`, right-associative) > and_formula > until_formula (left-
   associative) > neg_operand (`!` chains bind TIGHTER than `until` -
   standard PSL/CTL convention, so `!a until b` is `(!a) until b`) >
   primary_formula.
3. **Multi-character operator guards.** After matching a one-character
   tag, the next byte decides whether a longer operator owned by
   another level was actually present: `<` followed by `<` stops
   relational (it is `<<`), `-` followed by `>` stops additive (it is
   `->`), `|` / `&` followed by themselves stop bitwise_or /
   bitwise_and (they are `||` / `&&`).
4. **Dedicated `eventually` diagnostic.** `eventually` without a
   parenthesized body can never parse and would otherwise surface as
   "trailing input"; after any failure, `find_eventually_missing_paren`
   scans for a whole-word `eventually` whose next non-whitespace byte is
   not `(` (skipping string literals, so version headers are safe) and
   reports the exact position.
5. **Keywords.** `keyword` and `soft_keyword` are implemented
   identically (parse an identifier, compare); the naming marks words
   that double as identifiers in other positions (`next`, `always`,
   `eventually`, `input_is`, `output_has`, `until`, `true`, `false`).
   `KEYWORDS` (28 words) is the single source of truth for reserved
   words; the test `keyword_list_matches_parser_sites` pins it against
   the parser's call sites in both directions.
6. **Loop-based list parsing.** Repeated-element constructs
   (`enum_variants`, `field_list`, `pattern_field_list`, `expr_list`,
   `action_block`) parse in a loop until the closing token with
   optional comma separators, detecting duplicates inline as semantic
   errors.
7. **Numeric literals.** `int_lit` is `many1(digit)` parsed to u64
   (overflow -> semantic error); `bit<N>` widths go through
   `u32::try_from` for the same reason.

**Validator.**

1. **All-errors collection.** `validate_ir` / `validate_module`
   accumulate every error in a Vec instead of failing fast - one run
   reports the whole problem set, and the CLI surfaces them all.
2. **Deliberately loose assignment typing.** `set x = e` checks that x
   exists and that e type-checks, but does NOT check e's type against
   x's declared type; variable initializers are checked the same way
   (the initializer literal may adapt to the var type). Width
   adaptation is the design: integer literals conform to the target
   width.
3. **Pattern validation populates the environment.**
   `validate_event_pattern` walks the pattern fields left-to-right and
   inserts each named field into `env.bindings`, so the guard (checked
   afterwards) can reference bindings; duplicate binding names are
   rejected. Formula patterns (`input_is` / `output_has`) do NOT bind in
   Phase 1 - `check_pattern_group` rejects named fields there.
4. **Priority/ambiguity ladder** (per state, pairwise over transitions):
   identical pattern + equal explicit priorities -> error; identical
   pattern + both without priority + structurally equal guards -> error
   (the second transition is unreachable); identical pattern + both
   without priority + different guards -> warning only (order-dependent;
   suggests pinning with `priority N;`). Priority numbers may be reused
   across different patterns.
5. **Impossible-guard detection is syntactic.** Literal false, or a
   variable bound to two different integer literals by equality atoms
   joined at the top level with `&&` (`contradictory_equalities`) - no
   evaluation, no SAT.
6. **Safety classification for the MC engine.** `is_safety_formula`:
   any occurrence of Eventually / BoundedEventually / Until makes the
   formula liveness; everything else is safety (D-P4-8).
7. **Semantic errors carry no span.** The validator reports messages
   only (structured JSON uses line/col 0); point spans exist for parse
   errors, where the position is known.

Code: `src/parser.rs::non_boolean_expr`, `src/parser.rs::logic_or`,
`src/parser.rs::relational`, `src/parser.rs::until_formula`,
`src/parser.rs::neg_operand`,
`src/parser.rs::find_eventually_missing_paren`,
`src/parser.rs::keyword`, `src/parser.rs::KEYWORDS`,
`src/parser.rs::int_lit`, `src/validator.rs::validate_module`,
`src/validator.rs::validate_event_pattern`,
`src/validator.rs::check_pattern_group`,
`src/validator.rs::contradictory_equalities`,
`src/validator.rs::is_safety_formula`

## IR normalization + codegen pipeline

TBD (task 6.2). Fragment anchor added in task 6.2.

## MC engine - exact / liveness / SMT paths

TBD (task 6.3). Fragment anchor added in task 6.3.

## Monitor compilation (PSL translation)

TBD (task 6.4). Fragment anchor added in task 6.4.
