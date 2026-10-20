# Phase 5, Task 2 - Implementation details

> Written for a future LLM session: after reading `START.md`,
> `docs/roadmap/INTRO.md` and this file, the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal
Implement a full hand-rolled JSON READER+WRITER (`src/json_rw.rs`) and migrate
ALL sources to it, removing serde/serde_json from the toolchain entirely. This
guarantees key ordering (declaration order, pinned) and always byte-identical
text output for every JSON the tool produces or consumes - IR round-trips, all
`--json` CLI outputs (task 3 info, task 10 MC counterexample, task 13
validate), and the generated MC JSON (embedded copy).

## Context

- **Why hand-rolled instead of external crates (the decision rationale - must be
  recorded verbatim-quality in DESIGN.md section 23, KB.md, and this module's
  source comments per task 15 / this task's Action 1):**
  - KEY ORDERING GUARANTEE: objects serialize with keys in DECLARATION order
    (the order our code writes them), never alphabetical (serde_json's default
    BTreeMap) nor insertion-order-dependent on a third-party map type. Consumers
    can rely on the exact key sequence as part of the wire contract.
  - ALWAYS BYTE-IDENTICAL TEXT OUTPUT: output depends only on the value, not on
    platform, Rust version, allocator behavior, or dependency updates. External
    crates can (and do) change formatting, escaping edge cases, and number
    rendering across versions - every such change would silently break our
    byte-pinned parity tests (cross-backend MC JSON identity, golden files) and
    any downstream consumer diffing tool output.
  - ZERO EXTERNAL DEPENDENCIES: consistent with the generated-code constraint
    (self-contained binaries) - the SAME module is embedded verbatim in the
    generated MC engines (task 10), so toolchain and generated JSON behavior are
    identical by construction, not by hope.
  - STABLE WIRE FORMAT UNDER OUR CONTROL: the IR JSON shape evolves only when we
    change `to_json`/`from_json` deliberately; a dependency upgrade can never
    reshape our output.
- **Current serde usage inventory (all of it goes away):**
  - `src/ir.rs`: `#[derive(Serialize, Deserialize)]` on every IR type
    (Ir/Module/EnumDecl/EventGroup/OutputGroup/VarDecl/Transition/Property/
    Formula/Expr/EventPattern/OutputPattern/Type/etc.) + serde attributes.
  - `src/frontend.rs`: `.json` input parsing (`serde_json::from_str`) and `.json`
    output writing (pretty) in the parse command; spec.json loading for
    validate/codegen.
  - Tests: `serde_json` used for assertions / JSON construction in a handful of
    places (frontend_test, codegen_test, library_comparison).
  - `Cargo.toml`: `serde` (+ derive feature) and `serde_json` dependencies.
- **Scope = ALL sources**: after this task, `grep -r "serde" src/ tests/` finds
  nothing; the only JSON in the repo is produced/parsed by `src/json_rw.rs` (or
  its embedded copies in generated code).
- **Value model design:**
  - `Json { Null, Bool(bool), Int(i64), Str(String), Arr(Vec<Json>),
    Obj(Vec<(String, Json)>) }` - objects are ORDERED key/value vectors
    (declaration order by construction; duplicate keys rejected on read).
  - Writer: `to_string(&Json) -> String` (compact) and `to_string_pretty(&Json,
    indent=2) -> String`; string escaping pinned exactly: `"` `\` `\n` `\t`
    `\r` `\b` `\f` and control chars U+0000..U+001F as `\u00XX`; integers via
    `i64` Display (no floats in scope - the IR and all planned JSON contain only
    ints/strings/bools/nulls/arrays/objects; document this).
  - Reader: `parse(&str) -> Result<Json, JsonError>` with byte-offset errors
    (feeds diagnostic messages); tolerant of surrounding whitespace; strict on
    syntax (no trailing garbage, no NaN/Infinity, no duplicate keys, depth
    limit e.g. 512 to bound recursion). Typed extraction helpers:
    `obj.get("field")`, `as_str/as_int/as_bool/as_arr/as_obj` with clear errors.
- **IR migration approach:** hand-written `to_json(&T) -> Json` /
  `from_json(&Json) -> Result<T, String>` for every IR type (trait or per-type
  fns in `src/ir.rs`); key order = struct declaration order (pinned by golden
  test). Round-trip property: for every valid example, parse rfs -> to_json ->
  from_json -> equality with the original IR. EXISTING IR JSON TEXT MAY CHANGE
  (serde's default map ordering vs our declaration order) - that is acceptable
  and expected; re-pin any test that asserts on IR JSON text.
- **Formatting of `parse --output json`**: keep pretty-printed (2-space indent)
  to match current behavior's readability; the exact layout is pinned by a
  golden test from this task forward.
- Baseline before this task: 230/230 main tests, 537/537 generated-code tests,
  45 valid + 12 invalid examples (Phase 4 close-out).

## Scope

### Depends on tasks

(none)

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 3. | `rfs info` command + shared CLI infrastructure | `--json` output rendered by this task's writer |
| 10. | MC counterexample JSON format | embeds a copy of this task's writer in the generated engines |
| 13. | IDE support | `validate --json` diagnostics rendered by this task's writer |

### Affected source files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `src/json_rw.rs` | new | value model + ordered-object writer (compact/pretty) + reader with typed extraction; module doc carries the full decision rationale |
| `src/lib.rs` | change | register `pub mod json_rw;` |
| `src/ir.rs` | change | remove all serde derives/attributes; add hand-written to_json/from_json for every IR type (declaration-order keys) |
| `src/frontend.rs` | change | .json input parsing + output writing via json_rw (parse command, validate/codegen spec.json loading) |

### Affected test files

| File | Operation | Brief description |
|------|-----------|-------------------|
| lib unit tests (`src/json_rw.rs` inline `#[cfg(test)]`) | new | writer escaping/numbering/layout pins; reader success + error cases (offsets, dup keys, trailing garbage, depth limit); round-trip property on sample documents |
| `tests/frontend_test.rs` | change | serde_json assertions -> json_rw; re-pin any IR JSON text assertions to the new key order |

### Other affected files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `Cargo.toml` | change | remove `serde` and `serde_json` dependencies (z3 feature untouched) |
| `Cargo.lock` | change | regenerated |

## Planned actions

### Action 1: Value model + writer with pinned output
Implement the `Json` value model and the writer in `src/json_rw.rs`: ordered
objects, compact + pretty layouts, exact escaping table, i64 number rendering.
The MODULE DOC must open with the full decision rationale (why hand-rolled
instead of external crates: key-ordering guarantee, always byte-identical text
output across platforms/Rust versions/dependency updates, zero external deps
consistent with generated-code constraint, wire format under our control) -
this text is the source that task 15 copies into DESIGN.md section 23 and
KB.md. Pin every formatting rule with unit tests (escaping table exhaustively,
int rendering, pretty indentation, empty containers).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/json_rw.rs` | new | value model + writer + module-doc rationale + unit tests |
| `src/lib.rs` | change | register the module |

### Action 2: Reader with typed extraction
Implement `parse` (strict syntax, byte-offset errors, depth limit, duplicate-key
rejection) and typed helpers (`get`, `as_str/as_int/as_bool/as_arr/as_obj`).
Unit tests: valid documents incl. nested/empty, every error class with expected
offsets, round-trip property parse(to_string(x)) == x for a corpus of sample
documents (IR-shaped + CLI-shaped).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/json_rw.rs` | change | reader + extraction helpers + tests |

### Action 3: IR migration - hand-written to_json/from_json
For every IR type, write `to_json` (declaration-order keys) and `from_json`
(strict, informative errors). Remove ALL serde derives/attributes from
`src/ir.rs`. Add the round-trip property test: for all 45 valid examples,
parse rfs -> to_json -> from_json equals the original IR. Re-pin any test that
asserts on IR JSON text (key order may change vs serde's output - expected).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/ir.rs` | change | derives removed; to_json/from_json for all types |
| `tests/frontend_test.rs` | change | round-trip property test added; text assertions re-pinned |

### Action 4: Switch remaining sources + remove serde
Migrate `src/frontend.rs` (.json input parsing, pretty .json output for the
parse command, spec.json loading in validate/codegen) and every test using
serde_json to json_rw. Remove `serde`/`serde_json` from `Cargo.toml`,
regenerate `Cargo.lock`. Full verification: build, all tests (expect 230 + new
json_rw tests), clippy, fmt; `grep -r "serde" src/ tests/ Cargo.toml` returns
nothing.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/frontend.rs` | change | all .json paths via json_rw |
| `Cargo.toml` / `Cargo.lock` | change | serde/serde_json removed |
| (any test still using serde_json) | change | switched to json_rw |

## Task deliverables
- `src/json_rw.rs`: full JSON reader-writer with ordered objects, pinned
  formatting, byte-offset error positions, typed extraction - and the module-doc
  decision rationale.
- IR fully serialized/deserialized by hand-written code; round-trip property
  tested over all valid examples.
- serde/serde_json gone from dependencies and every source/test.
- Golden tests pinning writer output (escaping, layout) and reader behavior.

## Task exit criteria
- [ ] `grep -r "serde" src/ tests/ Cargo.toml` returns nothing (case-insensitive).
- [ ] json_rw unit tests green: writer escaping/layout pins, reader error cases with offsets, round-trip property on sample corpus.
- [ ] IR round-trip property test green over all 45 valid examples (parse rfs -> to_json -> from_json == original IR).
- [ ] `cargo run -- parse --input rfs --output json` and all four format combinations still work; pretty layout pinned by a golden test.
- [ ] Full suite green (230 + new tests), clippy without errors, fmt clean.
- [ ] Module doc contains the full decision rationale (key ordering + byte-identical output + zero deps + wire-format control).

## Non-goals
- Float support in the value model (no current or planned JSON contains floats;
  documented as a scope limit - add only if a future need appears).
- Any change to what data the IR/CLI JSON carries (shape is unchanged except key
  order, which becomes declaration order by design).
- Generated-code changes: task 10 embeds the writer copy in the MC engines; this
  task touches only toolchain sources.

## Other information
- If a test pins IR JSON text written before this task, its failure is EXPECTED
  (key order moves to declaration order) - re-pin with a note, do not try to
  preserve alphabetical order.
- Depth limit (512) and duplicate-key rejection are deliberate strictness: the
  only producers of our input JSON are ourselves and users hand-editing specs;
  both cases should fail loudly.
- Keep `JsonError` messages stable-format (`expected X at byte N`) - task 3's
  diagnostics plumbing may surface them verbatim.
