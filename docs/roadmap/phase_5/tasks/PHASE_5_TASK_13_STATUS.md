# Phase 5, Task 13 - Implementation status

Completed 2026-10-23. IDE support: TextMate grammar + snippets for .rfs (in-repo under ide/) and structured CLI diagnostics (rfs validate --json) so editors can ingest parse/semantic errors directly. Full LSP remains a non-goal.

## Added source files

| File | Description |
|---|---|
| `ide/textmate/rfs.json` | TextMate grammar (scopeName source.rfs): comments // and #, strings, numbers, booleans, all 28 keywords by class, types + bit<N>, enum paths A::B, formulas (temporal/boolean/comparison/arithmetic, input_is/output_has), module/enum/event/output/var/prop/assume/transition/on_error blocks |
| `ide/snippets.rfs` | VS Code snippet JSON: module, transition, prop-always, prop-eventually, prop-until |
| `ide/README.md` | install notes (VS Code local extension + one-click repo; Sublime .tmLanguage) + validate --json shape |

## Changed source files

| File | Change |
|---|---|
| `src/parser.rs` | NEW pub const KEYWORDS (28 RFS v0 keywords, single source of truth) + drift-guard test keyword_list_matches_parser_sites (bidirectional self-scan vs the parser's own call sites) |
| `src/frontend.rs` | StructuredError {kind,line,col,message} + validate_one_structured (parse errors carry real 1-based line/col from ParseError::line_col; semantic errors 0/0 - ValidationError has no span) + run_validate_structured (same traversal rules as run_validate, no .err side-files in JSON mode) + render_validate_json (file: one doc; directory: array per failing file; pretty like info --json) |
| `src/main.rs` | validate --json flag + usage text; JSON on stdout via print!; exit codes unchanged (0 ok / 1 failures or misuse) |

## Added/changed test files

| File | Change |
|---|---|
| `tests/frontend_test.rs` | +5: validate --json parse-error shape with line/col cross-checked against ParseError::line_col, semantic-error shape pinned (0/0), ok=true on a valid file, directory-mode array (one doc per failing file, exit 1), grammar sanity (valid JSON + every KEYWORDS entry present + snippets valid JSON) |
| `src/parser.rs` (unit) | +1: keyword_list_matches_parser_sites |

## Documentation

| File | Change |
|---|---|
| `BUILD.md` | ide/ file-map row, validate --json command-table row, counts re-pinned (root 369->375, workspace 393->399, smt 371->377; lib 188->189, frontend 88->93), baseline |
| `KB.md` | validate status row + Tests row updated |
| `docs/roadmap/INTRO.md` | tasks 1-13 of 16 |

## Corrections / decisions

- C13-1: semantic errors carry line=0, col=0 - ValidationError has no span and the task forbids diagnostic-logic changes; parse errors carry real 1-based positions. Pinned in tests.
- C13-2: keyword sharing via pub const KEYWORDS + a bidirectional self-scan test instead of refactoring ~35 call sites in the byte-sensitive parser (new keywords must be added to KEYWORDS or the pin fails).
- C13-3: JSON mode writes no .rfs-validation.err side files (editors consume stdout); text path untouched.
- D13-1: pretty-printed output, matching rfs info --json (the task's compact example is documentation style only).

## Verification

cargo test 375/375 root (was 369), 399/399 workspace (was 393), 377/377 --features smt (was 371); text diagnostics + exit codes unchanged (all pre-existing validate pins pass unmodified); clippy identical to the 13-warning baseline; fmt clean. Manual: validate --json on a parse-error file (line/col present), a valid file (ok:true, exit 0), and examples/invalid directory mode (array of 12 docs, exit 1).
