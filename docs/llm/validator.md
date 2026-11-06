<!-- llm-budget: 120 -->
# RFS Validator (Class B)

Compact card for LLM assistants working on validation, typing, or error
diagnostics.

<a id="llm:validator"></a>
## Card

**Responsibility.** Semantic validation + expression typing for RFS IR: the
pass that turns a syntactically valid parse into a well-typed, well-formed
document. One file: `src/lang/validator.rs`. Fatal errors block codegen; warnings
never do.

**Key items.**

| Item | Role |
|---|---|
| `src/lang/validator.rs::validate_ir` | fatal pass entry: ALL errors over all modules, never fail-fast |
| `src/lang/validator.rs::validate_module` | the per-module check sequence (states, version header, vars, transitions, priority ladder, on_error, props) |
| `src/lang/validator.rs::is_valid_version_format` | X.Y or X.Y.Z format gate for the version header (D-P6-1; parser accepts any quoted string) |
| `src/lang/validator.rs::validate_event_pattern` | pattern conformance vs declared constructor; populates env bindings left-to-right |
| `src/lang/validator.rs::validate_output_expr` | emit conformance: bare output ok only for single-variant groups, arity + arg types |
| `src/lang/validator.rs::type_expr` | expression typing entry (ExprType lattice) |
| `src/lang/validator.rs::resolve_path` | name resolution: state special-cased; bindings > vars > State variants; qualified paths exactly 2 segments |
| `src/lang/validator.rs::binary_type` / `src/lang/validator.rs::bit_op_type` | operator rules: width equality, IntLit adapts (lit+lit -> Bit(8)), shifts need no strict right width |
| `src/lang/validator.rs::type_formula` | formula typing: everything must be boolean; temporal recursion |
| `src/lang/validator.rs::is_safety_formula` | safety vs liveness classification for MC planning (D-P4-8) |
| `src/lang/validator.rs::bounded_always_error` | always[N] requires N >= 1, nested included; bounded_eventually[0] stays legal |
| `src/lang/validator.rs::module_warnings` | 5 advisory classes: unreachable states, dead events, unused vars, order-dependent pairs, impossible guards |
| `src/lang/validator.rs::Env` / `src/lang/validator.rs::ExprType` | name-resolution environment (vars/bindings/binding_idents/state_variants/enums) and the type lattice (Bool/Bit/IntLit/Enum) |

**Invariants that must not break.**

- All-errors collection: validate_ir never fail-fast - a file with N problems reports all N.
- Errors block codegen; warnings never do. Do not promote/demote a diagnostic without updating the pins.
- Pattern bindings populate left-to-right and are unique per transition; guards may reference them, on_error emits may NOT (no pattern context).
- prop/assume share one namespace (generated monitor struct names would collide - D-P3-4).
- Error message shapes are pinned by tests (frontend_test semantic-shape pin + 24 unit tests in mod tests) - changing a message breaks pins.

**Change impact (validation rules).** Add invalid examples FIRST (file-driven suite), then `src/lang/validator.rs` (+ its mod tests); verify with cargo test --lib validator, cargo test --test example_corpus_test, cargo test --test frontend_test; keep the 13-file invalid corpus failing. Full table: llm:change-impact.

**Deep dive.** dev:validator-overview + dev:validator-fatal-pass / dev:validator-typing / dev:validator-warnings in docs/dev/validator.md; algorithm levels: dev:algorithms-parser-validation; call graph: dev:deps-fn-validator.
