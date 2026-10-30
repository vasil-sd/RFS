# RFS Validator (Class A)

Semantic validation and expression typing for RFS IR: the pass that turns a
syntactically valid parse into a well-typed, well-formed document.

<a id="dev:validator-overview"></a>
## Overview

RFS has a two-stage frontend: the parser (`src/parser.rs`) checks SYNTAX and
structural uniqueness at parse time, and the validator (`src/validator.rs`,
1239 lines including its 24 unit tests) checks SEMANTICS - name resolution,
expression typing, pattern conformance, and formula well-formedness. The
split is deliberate: anything that can be decided while reading the source
(duplicate names inside a block, a missing `next`, u64 literal overflow)
is a parser error with a source span; anything that needs the whole module
(a state name must exist in the State enum, a guard must type-check as bool,
prop/assume names must be unique across both lists) is a validator error.

Entry points:

- `src/validator.rs::validate_ir` - the FATAL pass: runs every check over
  every module and returns ALL errors at once (never fail-fast); a spec is
  usable only when the list is empty;
- `src/validator.rs::module_warnings` - the NON-FATAL pass: independent of
  the fatal one, produces diagnostics that never block code generation.

The error model is deliberately thin: `src/validator.rs::ValidationError`
carries a message and nothing else - no span. Semantic errors therefore carry
line/col 0 in `rfs validate --json` (`src/frontend.rs::StructuredError`),
in contrast to parse errors, which point at the exact byte via
`src/error.rs::ParseError`. The CLI flow is in
`src/frontend.rs::validate_one_with_warnings`: on fatal errors they are all
surfaced for the file and codegen is blocked; on success the warning pass
runs and its output is advisory only. `src/frontend.rs::run_validate`
drives this per file/directory.

What codegen may rely on after validation: every transition's state and
next_state are State variants; every event pattern conforms to its declared
constructor (so generated match arms are exhaustive by construction); guards
and set-values type-check against the `src/validator.rs::Env` environment;
emits match output constructor arity and types; prop/assume names are unique
across both lists (generated monitor struct names cannot collide, D-P3-4);
transition IDs are in declaration order (assigned by the parser's
`src/parser.rs::build_module`). The formal meaning of what a validated spec
means is specified in `docs/SEMANTICS.md`; the diagnostics plan is
DESIGN.md section 13.

Where to go next:

- algorithm levels (L1 abstract / L2 structures + invariants / L3 details):
  `dev:algorithms-parser-validation` in ALGORITHMS.md;
- function call graph: `dev:deps-fn-validator` in DEPENDENCIES.md;
- file entries: `dev:code-map-frontend` in CODE_MAP.md.

Code: `src/validator.rs::validate_ir`, `src/validator.rs::module_warnings`,
`src/validator.rs::ValidationError`, `src/frontend.rs::validate_one_with_warnings`,
`src/frontend.rs::run_validate`, `src/error.rs::ParseError`

<a id="dev:validator-fatal-pass"></a>
## Fatal validation pass

`src/validator.rs::validate_ir` iterates the modules and extends one error
vector - a file with ten problems reports all ten, which is what makes
iterating on a broken spec usable. `src/validator.rs::validate_module` runs
the checks in this order (every message is prefixed with the module name,
and per-transition messages with `transition #<index>`):

1. **State enum non-empty** - a module without State variants cannot run.
2. **Initial state** - if present it must be a State variant; if ABSENT but
   transitions exist, that is an error (a machine with transitions needs a
   start state). A module with neither is left to the warnings pass.
3. **Version header format** (`src/validator.rs::is_valid_version_format`,
   D-P6-1) - `X.Y` or `X.Y.Z` with non-empty all-digit components, checked
   without a regex dependency. The parser accepts ANY quoted string (old
   files still parse); this check is the semantic gate. Specs without a
   header are legacy v0.x and remain valid.
4. **Variable initializers** - each `var` init expression must type-check
   against the module environment (`src/validator.rs::type_expr`). The
   literal-to-type adaptation is checked loosely here: an integer literal
   may adapt to the variable's bit width (the strict rules live in typing,
   see below).
5. **Per transition** (in declaration order):
   - `state` and `next_state` are State variants;
   - the event pattern conforms to its declared constructor
     (`src/validator.rs::validate_event_pattern`) - this also POPULATES the
     environment's pattern bindings left-to-right, so guards can reference
     them (see the typing section);
   - the guard, if present, type-checks AND is bool (`must_bool`);
   - each `set` target exists as a module variable and its value type-checks;
   - each emit conforms to its output constructor
     (`src/validator.rs::validate_output_expr`).
6. **Priority/ambiguity ladder** (per state, pairwise over transitions):
   - duplicate priorities are an error ONLY when the two transitions share
     the identical event pattern - priority numbers may be reused across
     different patterns; and
   - identical pattern + identical guard (both absent or structurally equal)
     with no priorities on either is a SHADOWED-DUPLICATE error: the second
     transition can never fire. Note the split with the warnings pass:
     identical pattern, NO priorities, but DIFFERING guards is only a
     WARNING (order-dependent pair) - legal, just fragile.
7. **on_error block** - `next` must be a State variant and its emits must
   check out WITHOUT pattern bindings (the fallback has no event context).
8. **prop/assume names** - unique across BOTH lists combined; a collision
   would collide generated monitor struct names (D-P3-4), so it is rejected
   early rather than at codegen.
9. **Formulas** - every property and assumption formula type-checks as
   boolean (`src/validator.rs::type_formula`), and
   `src/validator.rs::bounded_always_error` enforces the Phase 3 bound rule:
   `always[N]` requires N >= 1, including NESTED occurrences (the check
   recurses through every temporal/binary operator). `bounded_eventually[0]`
   stays legal and means "the body must hold at the trigger sample itself".

Pattern conformance in detail (`src/validator.rs::validate_event_pattern`):
the group must exist; a named variant must exist in the group; unit/struct
shapes must match (a unit variant cannot be matched with fields and vice
versa); struct field counts must be equal; per field, the pattern expression
must be compatible with the declared field type
(`src/validator.rs::pattern_expr_compatible`): wildcards and bindings always
match; an integer literal fits `bit<N>` only when it is <= 2^N - 1 (widths
>= 64 use u64::MAX as the bound); integers cannot match bool or enum fields;
booleans only match bool; an enum-variant path must start with the field's
enum name and have at least two segments. Named pattern bindings are
registered in the environment left-to-right, and a duplicate binding name is
an error.

Output conformance (`src/validator.rs::validate_output_expr`): the group
must exist; a BARE output (no variant named) is legal only for single-
variant groups - otherwise it is ambiguous; the named variant must exist;
the argument count must equal the variant's field count; each argument must
type-check and be compatible with the corresponding field type.

Code: `src/validator.rs::validate_ir`, `src/validator.rs::validate_module`,
`src/validator.rs::is_valid_version_format`,
`src/validator.rs::validate_event_pattern`,
`src/validator.rs::pattern_expr_compatible`,
`src/validator.rs::validate_output_expr`, `src/validator.rs::bounded_always_error`

<a id="dev:validator-typing"></a>
## Expression and formula typing

**The type lattice.** `src/validator.rs::ExprType` has four shapes:
`Bool`, `Bit(width)`, `IntLit`, and `Enum(name)`. `IntLit` is an UNTYPED
integer literal that adapts to whatever bit width its context provides -
literal + literal defaults to `Bit(8)`. This keeps specs readable (no
width annotations on literals) while generated code gets concrete widths.
IR types map into the lattice via `src/validator.rs::type_to_expr_type`.

**The environment.** `src/validator.rs::Env` holds per-module name
resolution: `vars` (declared variables to types), `bindings` (pattern-
binding names, populated by pattern validation as it walks each transition's
event pattern left-to-right), `binding_idents` (RFS binding name -> the
identifier actually emitted when codegen had to MANGLE a binding that
collides with an unqualified identifier in the generated body - the validator
leaves this EMPTY; codegen fills it), `state_variants`, and `enums` (name to
variant set). Built once per module by `src/validator.rs::for_module` (the
State enum is special-cased into `state_variants`); extended per transition
via `src/validator.rs::with_binding`.

**Path resolution.** `src/validator.rs::resolve_path` handles two shapes:
- a single segment: the literal name `state` resolves to `Enum("State")`
  (the current-state pseudo-variable); then BINDINGS shadow VARIABLES, which
  shadow State variants - so a pattern binding named like a variable refers
  to the binding inside that transition's guard; anything else is an unknown
  identifier;
- a qualified path: exactly TWO segments (`Enum::Variant`), checked against
  the module's enums; longer paths are rejected.

**Unary and binary rules.** `src/validator.rs::type_expr` dispatches:
integers -> IntLit, booleans -> Bool, paths -> resolve_path, unary and
binary as below. Unary: `!` requires a bool operand and yields bool; `~`
and `-` require a bitvector (IntLit adapts to Bit(8)). Binary
(`src/validator.rs::binary_type`):
- logical and/or: both operands must be bool, result bool;
- comparisons (`==`, `!=`, `<`, `<=`, `>`, `>=`): operands must be
  compatible (`src/validator.rs::compatible_with`: IntLit adapts to Bit,
equal types ok, IntLit vs Bool is an error, anything else a mismatch),
  result bool;
- plus/minus/star: `src/validator.rs::bit_op_type` - two bitvectors must
  have EQUAL widths; an IntLit adapts to the other side's width; literal +
  literal -> Bit(8);
- shifts: the left operand is a bitvector (IntLit -> Bit(8)); the right may
  be a bitvector or an integer literal with NO strict width equality -
  shifting by a wider amount is legal and evaluates to 0 at runtime (the
  wide-ops parity pins cover this);
- bitwise and/or: bool + bool is allowed and yields bool (logical use),
  otherwise bit_op_type; bitwise xor: bit_op_type only.

**Formula typing.** `src/validator.rs::type_formula` requires every formula
to be boolean: a bare expression must type-check as bool; `input_is` /
`output_has` delegate to `src/validator.rs::check_pattern_group` /
`src/validator.rs::check_output_pattern_group` - both are Phase 1 stubs:
formula patterns do NOT introduce bindings (a struct field with a name is
rejected), only group/variant plausibility is checked; every temporal
operator (not/next/always/bounded_always/bounded_eventually/eventually and
the binary and/or/implies/until) recurses into its operands. The public
utility `src/validator.rs::is_safety_formula` classifies formulas for model-
checking planning: anything containing `bounded_eventually`, `eventually`,
or `until` is NOT a safety formula (D-P4-8); it has no internal callers in
the validator - MC planning and tests consume it.

Code: `src/validator.rs::ExprType`, `src/validator.rs::Env`,
`src/validator.rs::for_module`, `src/validator.rs::with_binding`,
`src/validator.rs::type_to_expr_type`, `src/validator.rs::type_expr`,
`src/validator.rs::resolve_path`, `src/validator.rs::binary_type`,
`src/validator.rs::bit_op_type`, `src/validator.rs::compatible_with`,
`src/validator.rs::type_formula`, `src/validator.rs::is_safety_formula`

<a id="dev:validator-warnings"></a>
## Non-fatal diagnostics (warnings)

`src/validator.rs::module_warnings` produces five classes of advisory
diagnostics for one module. They NEVER block code generation - the CLI and
codegen surface them as warnings, and a spec with only warnings still
generates and runs.

1. **Unreachable states** - iterative DFS from the initial state over
   transitions (guards IGNORED: reachability is over-approximated, so a
   state behind an impossible guard still counts as reachable). Every State
   variant not reached gets a warning.
2. **Dead events** - an event constructor variant that no transition pattern
   references. A BARE group pattern (no variant named) marks ALL of the
   group's variants as used, so only specifically-orphaned variants warn.
3. **Unused variables** - a variable never read (names collected from guards
   and emit arguments via `src/validator.rs::collect_names`, including
   on_error emits) and never assigned by any `set`.
4. **Order-dependent transition pairs** - same state + identical event
   pattern, no priorities on either, but DIFFERING (or one missing) guard:
   first match wins in source order, so the warning suggests pinning the
   intent with distinct `priority N;` actions. This is the warning-side twin
   of the fatal shadowed-duplicate check (identical guards = error).
5. **Impossible guards** - detected SYNTACTICALLY, without evaluating
   anything: either the guard is the literal `false`
   (`src/validator.rs::is_literal_false`), or
   `src/validator.rs::contradictory_equalities` finds a variable compared
equal to two different integer literals among the TOP-LEVEL `&&` conjuncts
   (only direct `var == literal` atoms count - no arithmetic, no nesting
   below the top conjunction). Deliberately narrow: it is cheap, precise for
   the class it catches, and never produces false positives.

Code: `src/validator.rs::module_warnings`,
`src/validator.rs::collect_names`, `src/validator.rs::is_literal_false`,
`src/validator.rs::contradictory_equalities`

## Changing validation rules

Checklist for developers changing what the validator accepts or reports:

1. Add INVALID examples first where possible (drop into `examples/invalid/` -
   the file-driven suite picks them up without test-code changes); a new
   accepted shape gets a valid example.
2. Update `src/validator.rs::validate_module` (fatal) or
   `src/validator.rs::module_warnings` (advisory) - keep the all-errors-
   collection discipline (never fail-fast).
3. Error MESSAGE SHAPES ARE PINNED: `tests/frontend_test.rs`
   e2e_validate_json_semantic_error_shape_pinned pins exact message text
   (kind "semantic", line/col 0), and the 24 unit tests in `mod tests` pin
   individual behaviors (duplicate-priority split, shadowed duplicates,
   warning-only order-dependent pairs, zero-bound rules incl. the nested
   case). The 13-file invalid corpus must keep failing.
4. Verify: `cargo test --lib validator`, `cargo test --test examples_test`,
   `cargo test --test frontend_test`, then the full BUILD.md section 5
   checklist.
5. Docs sync: DESIGN.md section 13 (diagnostics plan) and KB.md when a rule
   changes; training snippets if the change is user-visible.
