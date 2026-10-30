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

<a id="dev:algorithms-ir-codegen"></a>
## IR normalization + codegen pipeline

The IR is the single normalized form of a spec: every downstream stage
(codegen, model checking, diagrams, info) consumes a validated `Ir`
document and there is no intermediate on-disk representation between
stages. Normalization means a versioned document with a PINNED wire
format (hand-rolled JSON), a deterministic canonical rendering, and a
spec hash stamped into every generated artifact. Code generation is a
language-agnostic core (`src/codegen.rs`) plus shared planning modules,
rendered by per-language backends registered in a static plugin table.

### L1 - Abstract view

**IR normalization.** The parser (section above) produces an `Ir`
document; `.json` input loads straight into the same type via
`from_json`. The document carries `IR_VERSION`; its wire format is a
pinned byte-level contract (declaration-order keys, internally-tagged
enums with a leading "kind" key), rendered deterministically by the
hand-rolled JSON writer. The canonical rendering of the WHOLE file is
the input to the spec hash - first 16 hex chars of SHA-256
(`spec_hash16`) - which every generated artifact carries in its line-1
header so users can detect spec/toolchain drift in checked-in code.

**Codegen pipeline.** A `Backend` plugin value (name, command
descriptors, one function pointer per command kind) is registered in
the static table `BACKENDS`; the three command kinds are production FSM
source, unit tests, and model checker. The standard dispatch stamps the
spec hash into a shared options bag and generates for the FIRST module
of the file. Language-agnostic planning modules decide WHAT to emit -
FSM variant flattening + representative values (`src/codegen_fsm.rs`),
the static unit-test plan (`src/codegen_test.rs`), model-checker run
concepts and report formats (`src/codegen_mc.rs`) - and backends render
that data into target syntax. Parity principle: one plan, rendered
identically in Rust and C.

Code: `src/ir.rs::Ir`, `src/ir.rs::IR_VERSION`,
`src/spec_hash.rs::spec_hash16`, `src/codegen.rs::Backend`,
`src/codegen.rs::BACKENDS`, `src/codegen.rs::CodegenCommandKind`

### L2 - Key data structures and invariants

**Versioned document.** `Ir { ir_version, modules }`; the read policy
(D-P6-2) accepts the current version and ALL previous minor versions
(0..=IR_VERSION), rejects higher versions with a pinned error - forward
compatibility belongs to newer toolchains. Breaking schema changes bump
`IR_VERSION`; additive optional fields do not (D10 evolution rule).

**Module as codegen unit.** `Module` holds name, optional version
header, optional on_error fallback, enums, initial state, event/output
constructor groups, vars, transitions (sequential IDs), properties and
assumptions. The CLI generates for the first module; multi-module files
share the file's spec hash.

**Pinned wire format.** Structs serialize keys in DECLARATION order;
`Option` fields render as `null` EXCEPT `Module.version` /
`Module.on_error`, which are omitted when absent (both optional on
read); internally-tagged enums put "kind" FIRST with the snake_case
variant name; unit op variants are bare strings. Integers: unsigned IR
fields render as non-negative JSON numbers, `to_json` FAILS (rather
than clamps) above i64::MAX because the value model is 64-bit signed,
and `from_json` rejects negatives for those fields.

**JSON value model.** `Json` objects are ORDERED key/value vectors -
serialization order is exactly declaration order and the reader rejects
duplicate keys; integers only (no float variant); strict reader (no
trailing garbage, no leading zeros, nesting depth capped at
`MAX_DEPTH`). The same writer is embedded VERBATIM in the generated MC
engines, so toolchain and generated JSON behavior are identical by
construction.

**Spec hash.** `spec_hash16` = first 16 hex chars of hand-rolled
SHA-256 (FIPS 180-4) over the canonical IR JSON text of the whole file;
stable across runs and platforms, sensitive to any spec change.

**Backend plugin + registry.** `Backend` is static data: name,
description, per-command help descriptors, and one function pointer per
command kind (`generate_production` / `generate_unit_tests` /
`generate_model_checker`). `BACKENDS` currently holds the C and Rust
backends; adding a target language means adding a module under
`crate::backends` and appending to the table.

**Shared options bag.** `BackendOptions` carries everything a command
may need: assumption selection, test-run length, no_std, property-
monitor mode, MC mode + limits (max states, nondet, timeout, threads,
SMT bound), counterexample format, protocol-FSM opt-in, and the stamped
spec hash. Backends ignore fields that do not apply to a command.

**Shared planning invariants.** Variant flattening assigns collision-
free names (single-variant group keeps the bare name, otherwise
group_variant, duplicates get numeric suffixes); representative values
are bool -> false, bit<N> -> 0, enum -> first declared variant; the
translatable safety subset is `always(phi)` with phi liveness-free. The
test planner is STATIC: it emits data (cases, expected outcomes,
labels), never code, and records documented fallbacks in notes where a
case cannot be decided statically. MC concepts are shared by all
backends (generated engines embed copies kept in sync): per-step phase
order (forced assumption violation -> unexpected injection -> allowed
random input), the exact-mode status taxonomy (PassComplete / Fail /
UnknownStateLimit with exit codes 0/1/2 and a parity-pinned summary
line), and the counterexample report whose key ORDERING is part of the
byte-parity contract.

Code: `src/ir.rs::Module`, `src/json_rw.rs::Json`,
`src/json_rw.rs::parse`, `src/spec_hash.rs::sha256_hex`,
`src/codegen.rs::BackendOptions`,
`src/codegen_fsm.rs::build_variants`,
`src/codegen_fsm.rs::representative_values`,
`src/codegen_test.rs::TestPlan`, `src/codegen_mc.rs::ExactStatus`,
`src/codegen_mc.rs::McJsonReport`

### L3 - Implementation details (important ones emphasized)

**IR normalization.**

1. **Hand-rolled JSON: four contractual properties (the most important
   IR decision).** No external crate can provide: (a) key-ordering
   guarantee - objects are ordered vectors, so the exact key sequence is
   part of the wire contract and golden tests pin it byte-for-byte;
   (b) always byte-identical text output - rendering depends only on the
   value, never on platform or a dependency update; (c) zero external
   dependencies - consistent with self-contained generated artifacts,
   and the module is embedded verbatim in generated MC engines;
   (d) stable wire format under our control - a dependency upgrade can
   never reshape output or break inputs. Scope limits are documented:
   integers only, strict reader.
2. **Version read policy.** `from_json` accepts 0..=IR_VERSION and
   rejects higher versions with a pinned message; the parser always
   writes the current version. Additive variants and optional fields
   land WITHOUT a version bump (D10 evolution rule): within one version
   the schema only grows, and a document carrying a kind an older reader
   does not know fails with a pinned unknown-kind error rather than a
   version error.
3. **u64 value model.** Unsigned IR fields (u32/u64/usize) render as
   non-negative JSON numbers; `to_json` fails above i64::MAX instead of
   clamping, and `from_json` rejects negatives for those fields - the
   value model is 64-bit signed end to end.
4. **Hand-rolled SHA-256.** FIPS 180-4 implemented without dependencies
   (consistent with the dependency-light stance); NIST test vectors are
   pinned, including the 56-byte padding boundary and a multi-block
   message; `spec_hash16` is a prefix of `sha256_hex`, so it is
   deterministic and content-sensitive by construction.

**Codegen pipeline.**

1. **Parity principle (the most important codegen invariant).** The
   planning modules emit DATA, never code: one test plan / variant table
   / MC concept set is rendered by each backend into its own syntax, and
   cross-backend parity tests enforce byte-identical semantics for the
   same input. Identifier mangling, type mapping, and expression
   lowering stay in the backends.
2. **Static guard evaluation mirrors the backend lowering exactly.**
   The test planner evaluates guards with concrete values using the SAME
   rules as generated code: simultaneous assignment updates (all values
   computed from pre-step variables, like the temporary-next-variable
   lowering), u128-wide bit masks, and width resolution identical to the
   validator's typing. Where a case cannot be decided statically,
   `TestPlan.notes` records the documented fallback instead of guessing.
3. **Deterministic LCG everywhere.** The random-trace test uses an LCG
   with the SAME constants and top-53-bit double mapping as the fuzz
   wire format (fixed seed 12345), precomputed at plan time and
   byte-pinned - so the unit-test random trace and the fuzz input stream
   are mutually reproducible.
4. **Spec-hash stamping.** `CodegenCommandKind::run` renders the
   canonical IR JSON once per invocation, stamps `options.spec_hash`,
   and every artifact (both backends, all commands + fuzz skeletons)
   carries it in line 1; the hash is empty only when a library caller
   renders via `run_module` without stamping.
5. **JSON report key order is contractual.** The exact-mode JSON
   counterexample document has a fixed key sequence (top level: status,
   metrics, [reason, label, step], statistics, [counterexample];
   ce_steps LAST as the trace-length quality metric); both backends'
   generated engines embed a minimal copy of the shared writer and must
   produce byte-identical output - EXCEPT `peak_rss_kb`, which is
   informational (self-reported VmHWM/getrusage) and normalized to 0 in
   cross-backend comparisons, exactly like wall-clock times.
6. **Pinned cross-path error strings.** `--no-std` on a non-production
   command and `--with-protocol-fsm` on the mc command are rejected with
   ONE exact text at both the CLI parser and the backend entry points,
   so tests pin one string for every path; C accepts `--no-std` as a
   documented no-op (plain C is already dependency-free).

Code: `src/ir.rs::to_json`, `src/ir.rs::from_json`,
`src/json_rw.rs::MAX_DEPTH`, `src/codegen.rs::run`,
`src/codegen_test.rs::Lcg`, `src/codegen_test.rs::RANDOM_TRACE_SEED`,
`src/codegen_mc.rs::exact_summary_line`,
`src/codegen_mc.rs::CounterexampleFormat`

<a id="dev:algorithms-mc-engine"></a>
## MC engine - exact / liveness / SMT paths

The model-checking subsystem has three engines over one codegen-time plan:
the generated EXACT engine (Rust and C binaries rendered from `McPlan`),
an INDEPENDENT reference checker (`src/ref_mc.rs`, ground truth for
differential testing and the `rfs mc-ref` command), and an optional SMT
BOUNDED path (z3 k-step unrolling). All three report through the shared
status taxonomy and report formats of `src/codegen_mc.rs` (section above),
so their outputs are directly comparable - byte-for-byte where
deterministic.

### L1 - Abstract view

**Exact.** Breadth-first search over reachable configurations from the
initial configuration, where a configuration is (state, variable values,
monitor persistent states). Transition firing is deterministic first-
match in effective-priority order; every enabled input is tried at every
configuration - seedless exhaustive search, not random. Safety violations
are detected online by monitors while stepping successors; deadlock is a
configuration with no successor at all; liveness properties are checked
AFTER the BFS on the completed graph.

**Liveness.** Properties containing `eventually` / `until` are decomposed
into obligations plus a boolean skeleton, then checked by SCC analysis of
a product graph (position x automaton state); a violation yields a lasso
witness (prefix + one cycle iteration).

**SMT bounded.** k-step unrolling over z3 bitvectors: first-match firing
is encoded as prefix-OR constraints, properties are asserted per live
step. Sound FAIL within the bound; PASS is always incomplete ("pass-
within-bound"); solver-inconclusive is a distinct unknown status.

**Reference MC.** A deliberate reimplementation with different data
structures and algorithms (structured configuration keys instead of bit-
packing, its own evaluator / monitor compiler / input enumeration /
SCC), sharing ONLY the report data types and formatting helpers - so it
can confirm or refute the generated engines' verdicts independently.

Code: `src/mc_plan.rs::McPlan`, `src/ref_mc.rs::run`,
`src/smt_plan.rs::build_smt_spec`, `src/codegen_mc.rs::ExactStatus`

### L2 - Key data structures and invariants

**The shared plan (D-P4-1).** `McPlan::compute` is a PURE function of the
validated module + assumption selection - no environment reads, no
randomness - computed once in the tool and embedded as constants/tables
in both generated binaries, so the Rust and C engines are identical by
construction. It has three parts:
- **Input abstraction (D-P4-3).** Per event variant, per field: CONCRETE
  (enumerate the full canonical domain) iff the field is referenced by any
  transition guard, output payload expression, variable update expression,
  or property/assumption formula (`input_is` field values count); otherwise
  ABSTRACTED to a single representative value. Value order is canonical:
  bool false < true, bit<N> ascending, enum in declaration order.
- **Configuration encoding (D-P4-2).** config = (state index, packed
  variables, monitor transition states), bit-packed little-endian into
  `total_bytes` - the buffer BOTH engines hash for the visited set, so
  they must pack identically. Only monitor nodes with persistent state
  appear; the sticky failed/trigger flags are deliberately excluded
  (invariant on all live configurations - a safety failure terminates the
  search before enqueue).
- **Traversal spec (D-P4-2).** Canonical input order (events in
  declaration order, fields in declaration order, values ascending) and
  per-state firing order = effective priority (`priority N` descending,
  declaration order for ties) - the same order det mode uses for first-
  match and nondet mode uses to enumerate successors.

**Exact-engine invariants (SEMANTICS.md).** Level-synchronous BFS with
visited-at-enqueue; a safety failure terminates the search with a
counterexample before enqueue; unexpected inputs terminate that path
(counted as terminations, never successors); assumption-violating
successors are PRUNED and never reported as property violations
(assumptions step first at every sample); deadlock = no successor at all;
liveness is checked only when no safety failure / deadlock was found; a
liveness witness inside a truncated subgraph is SOUND (all its edges are
real).

**Liveness decomposition invariants (D-P4-9).** An obligation is
`eventually p` or `p until q` whose operands are BOOLEAN over atomic
propositions (no temporal operators inside); the skeleton is the top-
level boolean structure with obligations replaced by indices. Uncheckable
shapes (deeper nesting, non-boolean operands, temporal safety parts)
yield codegen diagnostics - never parse errors.

**SMT spec invariants (D-P4-12).** `build_smt_spec` renders a backend-
neutral encoding (`Se`/`Sb`/`Sf` over resolved indices) from the module +
the SAME `McPlan` (reusing its input abstraction): transitions in firing
order (same as exact first-match), concrete fields become free z3
variables per unrolled step, abstracted fields are substituted as their
representative literals. v1 scope is rejected at codegen with shared
errors: no assumptions, no on_error; top-level `always(phi)` /
`eventually(phi)` only with phi boolean over state/vars; bit<N>/bool vars
only; concrete payload fields N <= 64.

**Report invariants.** All paths report through the shared status
taxonomy (exit codes 0/1/2) and the parity-pinned summary line; SMT adds
mode=smt-bounded with statuses pass-within-bound / fail / unknown.

Code: `src/mc_plan.rs::InputAbstraction`,
`src/mc_plan.rs::ConfigEncoding`, `src/mc_plan.rs::TraversalSpec`,
`src/mc_plan.rs::LivenessDecomposition`,
`src/mc_plan.rs::boolean_over_atoms`, `src/ref_mc.rs::RefResult`,
`src/smt_plan.rs::SmtSpec`, `src/codegen_mc.rs::exact_summary_line`

### L3 - Implementation details (important ones emphasized)

**Exact + reference.**

1. **Independence by construction (the most important reference-MC
   design decision).** `ref_mc` reimplements everything: structured
   configuration-key tuples instead of bit-packing, its own expression
   evaluator and its own online monitor compiler (same documented
   semantics as `src/monitor.rs`, different code), its own representative-
   input rule (SEMANTICS 9.3, not imported from `src/mc_plan.rs`), and
   iterative Tarjan SCC instead of the generated engine's BFS + stay-edge
   subgraph + Kosaraju construction. It imports only std, the IR types,
   and the shared report data types / formatting helpers - enforced by
   the independence-import-audit test.
2. **Level-synchronous BFS with fine-grained deadline.** Frontier /
   next-frontier iteration; the wall-clock check runs PER CONFIGURATION
   (one level can exceed the budget on its own); the state cap yields
   UnknownStateLimit; parent / depth / adjacency are recorded for EVERY
   success (including edges to already-visited nodes) so the graph is
   complete for liveness analysis.
3. **Monitor stepping order at every sample.** Assumptions FIRST (a
   pruned successor is never reported as a property violation), then
   properties; sample 0 is stepped before the BFS starts (assumption
   violated at the initial configuration -> clean error; property
   violated -> FAIL with the empty trace).
4. **Liveness via product-graph SCC.** A position graph (position 0 =
   initial sample, one position per explored edge - edge-based labeling),
   a mixed-radix automaton state (radix 2 for safety/eventually leaves,
   radix 3 for until leaves), product reachability BFS over (position,
   state) pairs, iterative Tarjan SCC, on-cycle pairs = non-trivial SCC
   members + self-loop singletons; the falsifying limit vector is searched
   in ascending state order - a liveness property is violated iff some
   reachable cycle's limit falsifies the skeleton.
5. **Deadlock carries monitor state.** The result records the persistent
   monitor projection of the deadlocked terminal configuration, because
   the "no surviving successor" judgment depends on assumption-pruning
   state that is not printed in the report - an external verifier needs it
   to re-derive the deadlock verdict.
6. **Counterexample verification.** `verify_counterexample` replays a
   generated engine's JSON counterexample through ref_mc's independent
   evaluators (Ok = a valid execution that genuinely exhibits the claimed
   violation at the claimed step); `step_trace` drives a fixed input
   sequence with first-match firing + on_error fallback (the production-
   parity tests' driver).

**SMT bounded.**

7. **First-match encoded as prefix-OR.** At each unrolled step, fires[t]
   = enabled[t] AND NOT(enabled[0] OR ... OR enabled[t-1]) in firing
   order - the same order the exact engine uses; alive[i+1] = alive[i] AND
   any-fire, so deadlock paths die within the unrolling.
8. **Property encoding.** `always(phi)`: violation = OR over live steps of
   NOT phi(i). `eventually(phi)`: never = AND over live steps of NOT
   (alive AND phi), PLUS a repeated-configuration constraint (some pair of
   equal configurations - a cycle that can be iterated forever), so "never
   within k" becomes a genuine liveness violation; SAT yields the first
   violating step / first repeated pair (the lasso).
9. **PASS is always incomplete.** All checks UNSAT -> "PASS WITHIN BOUND k
   (incomplete - no completeness claim)" with status pass-within-bound
   and exit 0; solver-inconclusive -> status unknown, exit 2. Sound FAIL
   within the bound, clearly labeled.
10. **Parity-critical leaf-variable convention.** Step variables are z3
    CONSTANTS with fixed names (s_<i>, v_<i>_<j>, ev_<i>, fld_<i>_<g>);
    the C builders receive them as pointers, the Rust builders recreate
    them by name - z3 hash-consing returns the SAME node, so both
    backends build structurally identical formulas.

Code: `src/ref_mc.rs::bnode_key`, `src/ref_mc.rs::match_effect`,
`src/ref_mc.rs::mon_step`, `src/ref_mc.rs::liv_analyze`,
`src/ref_mc.rs::verify_counterexample`, `src/ref_mc.rs::step_trace`,
`src/smt_plan.rs::SmtTrans`, `src/smt_emit.rs::emit_rust_spec`,
`src/smt_emit.rs::emit_c_spec`,
`src/backends/rust_mc.rs::SMT_ENGINE_RUST`

## Monitor compilation (PSL translation)

TBD (task 6.4). Fragment anchor added in task 6.4.
