# RFS Code Map

File-by-file map of the 71 hand-written source files: `src/` (49), `crates/`
(17), `fuzz/fuzz_targets/` (5). Each entry: responsibility, key items, doc
links both ways. The pointer scope is driven by `tests/dev_docs_manifest.txt`.

<a id="dev:code-map-frontend"></a>
## Frontend area - root + parser + validator/IR (13 files)

### Root (6)

- `src/lib.rs` - crate root: declares the 30-module tree and re-exports the
  public API (`src/parser.rs::parse_source`, `src/parser.rs::parse_file`,
  `src/printer.rs::print_ir`, `src/ir.rs::Ir`, `src/error.rs::ParseError`,
  `src/error.rs::Span`, plus the frontend conversion/validation surface).
  Co-owner of all cross-cutting fragments (`dev:index`, `llm:index`,
  architecture, decisions, task map, change impact).
- `src/main.rs` - CLI entry point: argument parsing and dispatch to the eight
  commands (`parse`, `validate`, `info`, `test`, `diagram`, `codegen`,
  `mc-ref`, `gen-spec`) + usage text; key item `src/main.rs::main`. Owner of
  `dev:code-map-cli`.
- `src/error.rs` - shared source spans and parse-error types:
  `src/error.rs::Span` (byte ranges attached to every IR node) and
  `src/error.rs::ParseError`.
- `src/cli_log.rs` - minimal stderr logging for the CLI commands:
  `src/cli_log.rs::LogLevel` + `src/cli_log.rs::Logger`.
- `src/frontend.rs` - frontend CLI logic behind `rfs parse` / `rfs validate`:
  format detection (`src/frontend.rs::Format`), file collection, conversion
  through the IR (`src/frontend.rs::convert_file_to_ir`,
  `src/frontend.rs::render_ir`, `src/frontend.rs::run_conversion`) and
  validation dispatch (`src/frontend.rs::run_validate`,
  `src/frontend.rs::validate_one`).
- `src/printer.rs` - IR-to-RFS source printer (unparser): `print_ir`,
  `print_module`, `print_expr`, `print_formula`, plus round-trip helpers
  `roundtrip_rfs` / `roundtrip_json`.

### Parser (2)

- `src/parser.rs` - RFS v0 source parser (nom-based): spec text to IR with
  spans; entry points `src/parser.rs::parse_source` /
  `src/parser.rs::parse_file`, `src/parser.rs::KEYWORDS` table. Deep dive:
  `dev:parser-overview` / `llm:parser`.
- `src/ebnf_grammar.rs` - EBNF grammar model + renderers for IDE/tooling
  output: `src/ebnf_grammar.rs::RuleNode`, `to_ebnf_text`, `to_md`,
  `to_plantuml_ebnf`, `to_svg`.

### Validator + IR (5)

- `src/validator.rs` - semantic validation and expression typing for RFS IR:
  `src/validator.rs::validate_ir` returns the full error list
  (`src/validator.rs::ValidationError`), expression types via
  `src/validator.rs::ExprType`. Deep dive: `dev:validator-overview` /
  `llm:validator`.
- `src/ir.rs` - IR types + pinned JSON wire format: `src/ir.rs::Ir`,
  `src/ir.rs::Module`, `src/ir.rs::IR_VERSION`. Deep dive:
  `dev:ir-json-rw-overview` / `llm:ir-json-rw`.
- `src/json_rw.rs` - hand-rolled JSON reader/writer (the only JSON impl in the
  toolchain): ordered objects, byte-identical output contract;
  `src/json_rw.rs::Json`, `src/json_rw.rs::parse`.
- `src/json_schema.rs` - draft-07 JSON-Schema emitter + markdown renderer +
  sync walker for the IR wire format: `src/json_schema.rs::SchemaDoc`.
- `src/spec_hash.rs` - hand-rolled SHA-256; `src/spec_hash.rs::spec_hash16`
  stamps every generated artifact header.

<a id="dev:code-map-codegen"></a>
## Codegen core + backends (18 files)

TBD (task 3.2): codegen core (4) + Rust backend (5) + C backend (4); both
backends at equal depth.

<a id="dev:code-map-cli"></a>
## CLI + diagrams area (18 files)

TBD (task 3.3): CLI/diagrams (13) + bins/training (5).

<a id="dev:code-map-workspace"></a>
## Workspace crates + fuzz (23 files)

TBD (task 3.4): `crates/` (18) + `fuzz/` (5).
