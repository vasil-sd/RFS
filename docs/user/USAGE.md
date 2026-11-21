# RFS CLI Usage Reference

This is the exhaustive reference for every `rfs` command: each meaningful
invocation shape, its options, the files it generates (and where they land),
and the errors it can raise. It complements [MODEL_CHECKING.md](MODEL_CHECKING.md)
(mode semantics and report reading - linked, not re-derived here),
[INTEGRATION.md](INTEGRATION.md) (build.rs / adapters / Makefiles), and
[EXAMPLES.md](EXAMPLES.md) (spec walkthroughs). **Exit-code convention:** every
CLI command exits `0` on success and `1` on any error (usage, parse,
validation, missing tool for a selected operation); the generated model-checking
BINARIES use the `0` pass-complete / `1` fail / `2` unknown-status taxonomy
([MODEL_CHECKING.md section 5](MODEL_CHECKING.md#5-status-taxonomy-and-labeling-guarantees)).

## 1. Command map

| Command | One-line purpose | Section |
|---|---|---|
| `rfs parse` | format conversion rfs<->json through the IR | 3 |
| `rfs validate` | parse + semantic validation, text or --json diagnostics | 4 |
| `rfs info` / `rfs info format` | static introspection; self-describing format docs | 5 |
| `rfs codegen rust\|c` | production/test/mc artifact generation (both backends) | 6 |
| `rfs test` | generate + compile + run unit/fuzz suites | 7 |
| `rfs diagram` | dot/puml/mmd/table/md sources; in-process SVG by default | 8 |
| `rfs mc-ref` | in-process reference MC, all three modes | 9 |
| `rfs gen-spec` | random / base-mutated spec generation | 10 |

## 2. External tool resolution - the RFS_* environment contract

Every external tool rfs may invoke resolves by ONE precedence model, stated
once here and honored identically by the CLI and the test suites
([TOOLCHAIN.md section 8](../../TOOLCHAIN.md#8-portability-phase-12-tool-resolution-on-any-linuxunix-system)):

```
CLI flag  >  RFS_* env var  >  standard Unix var  >  auto-discovery (bare name on PATH; pkg-config for z3)
```

`--print-script` prints the resolved invocation as a self-contained POSIX sh
script whose header carries a `# tools: <name>=<path> ...` line - so any run
can be inspected WITHOUT executing. A real capture (from the
tool_resolution example) shows each tier of the model:

```sh
RFS_GCC=/usr/bin/gcc rfs test --backend c --suite unit --print-script spec.rfs
  -> # tools: gcc=/usr/bin/gcc          (the RFS_* env var wins)
RFS_GCC=/usr/bin/gcc rfs test ... --gcc /bin/sh-fake --print-script spec.rfs
  -> # tools: gcc=/bin/sh-fake          (a CLI flag beats the env var)
rfs test --backend c --suite unit --print-script spec.rfs   # nothing set
  -> # tools: gcc=gcc                   (bare name, resolved on PATH at run time)
```

| Tool | RFS_* variable (+ `*_ARGS`) | Standard var | CLI override flag(s) | Auto-discovery fallback | Used by |
|---|---|---|---|---|---|
| C compiler (gcc) | `RFS_GCC` (+`RFS_GCC_ARGS`) | `CC` | `--gcc PATH`, `--gcc-args "..."` | bare `gcc` on PATH | codegen c, test (c), example Makefiles |
| Rust compiler | `RFS_RUSTC` (+`RFS_RUSTC_ARGS`) | `RUSTC` | `--rustc PATH`, `--rustc-args "..."` | bare `rustc` on PATH | codegen rust, test (rust) |
| clang (libFuzzer host) | `RFS_CLANG` (+`RFS_CLANG_ARGS`) | *(none - deliberate)* | `--clang PATH`, `--clang-args "..."` | bare `clang` on PATH | test --suite fuzz (c) |
| cargo (fuzz scaffold) | `RFS_CARGO_FUZZ` (+`_ARGS`) | `CARGO` | `--cargo-fuzz PATH`, `--cargo-fuzz-args "..."` | bare `cargo` on PATH | test --suite fuzz (rust) |
| graphviz dot | `RFS_DOT` (+`RFS_DOT_ARGS`) | *(none)* | `--dot PATH`, `--dot-args "..."` | bare `dot` on PATH | diagram --renderer external |
| PlantUML | `RFS_PLANTUML` (+`RFS_PLANTUML_ARGS`) | *(none)* | `--plantuml PATH`, `--plantuml-args "..."` | bare `plantuml` on PATH | diagram --renderer external |
| Mermaid CLI (mmdc) | `RFS_MERMAID` (+`RFS_MERMAID_ARGS`) | *(none)* | `--mermaid PATH`, `--mermaid-args "..."` | bare `mmdc` on PATH | diagram --renderer external |
| make (C example projects) | `RFS_MAKE` | *(none)* | - | bare `make` on PATH | C example builds |
| z3 (SMT, feature-gated) | `RFS_Z3_DIR` | `Z3_SYS_Z3_DIR` | - | `pkg-config z3`, then the project-local install | mc-ref --mode smt-bounded, generated SMT binaries |

Notes on the contract:

- **clang deliberately has NO standard tier.** libFuzzer requires clang
  specifically, so a system with `CC=clang` must not silently switch the
  fuzzer compiler through the generic `CC` var - set `RFS_CLANG` instead.
- **Env args come first, CLI args are appended** to them (the `*_ARGS` vars
  prefix the `--*-args` flag values).
- **Special vars outside the table:** `RFS_BIN` (the rfs-build build.rs
  helper's binary override - see [INTEGRATION.md section 1](INTEGRATION.md#1-rust-integration-via-buildrs-rfs-build)),
  `RFS_FUZZ_FLAGS` (extra libFuzzer runtime flags), and
  `RFS_MERMAID_PUPPETEER_ARGS` (the mmdc puppeteer config - the headless-Chrome
  gotcha in [TOOLCHAIN.md section 6.4](../../TOOLCHAIN.md)).
- **A missing tool for a SELECTED operation is an error with a diagnostic** -
  no graceful degradation. Generation-only and in-process operations (parse,
  validate, info, codegen, mc-ref exact/fuzz) need no external tool at all.

## 3. rfs parse - format conversion

Convert between the RFS v0 source language and the IR v1 JSON document through
the versioned IR - the machine-readable form editors and CI pipelines consume.
No model checking, no codegen: a pure load -> IR -> render round-trip.

| CLI pattern | Options - detailed description | Generated files / output | Possible errors (exit codes) |
|---|---|---|---|
| `rfs parse --input rfs --output json spec.rfs` | `--input rfs\|json` selects the source format; `--output rfs\|json` the target. A file input's extension must match `--input` (a mismatch is a usage error) | `spec.json` written NEXT TO the input (the original `.rfs` extension replaced, never stacked) | parse/validation error -> stderr + exit 1; output would overwrite the input -> refused, exit 1 |
| `rfs parse --input json --output rfs spec.json` | the reverse direction: IR JSON back to RFS source (round-trip stability is pinned by the test suite) | `spec.rfs` next to the input | same as above |
| `rfs parse --input rfs --output json specs/ --target-dir out/` | a DIRECTORY input: only files matching `--input` are scanned; `--target-dir DIR` writes outputs into DIR with the input directory structure PRESERVED inside it (without it, each output lands next to its input) | `out/<mirrored-path>.json` for every matching file | any single file's parse/validation error fails the run, exit 1 |

### Naming and placement rules

- The original extension is REPLACED, never stacked (`spec.rfs` -> `spec.json`,
  not `spec.rfs.json`).
- Without `--target-dir`, each output is written next to its input file; with a
  directory input and no `--target-dir`, outputs mirror the input tree in place.
- A conversion whose output path equals its input path is REFUSED (it would
  destroy the source) - exit 1.

### Error details and fixes

- **Parse errors** carry line/column of the offending token; fix the syntax at
  that location ([LANGUAGE.md](LANGUAGE.md)).
- **Semantic errors** (unknown state, duplicate transition, bad expression) are
  reported after parsing; they mean the text is well-formed but inconsistent -
  see [PROPERTIES.md](PROPERTIES.md) for property-specific rules.

## 4. rfs validate - parse + semantic validation

Parse and semantically validate RFS or IR-JSON files. This is the gate that
runs before any codegen/MC: a spec that does not validate will not compile or
check. Text diagnostics go to stderr; `--json` emits structured diagnostics on
stdout (the shape editor integrations consume).

| CLI pattern | Options - detailed description | Generated files / output | Possible errors (exit codes) |
|---|---|---|---|
| `rfs validate spec.rfs` | a FILE input: the extension selects the format (`.rfs` or `.json`); no `--input` needed | nothing written; `valid <file>` on stdout when clean | parse/validation error -> stderr + exit 1 |
| `rfs validate --input rfs specs/` | a DIRECTORY input: `--input` is REQUIRED (which extension to scan); every failing source file gets a `<name>.rfs-validation.err` file written NEXT TO it | one `.err` file per failing spec, beside the source | any failure -> exit 1 (the `.err` files carry the diagnostics) |
| `rfs validate --json spec.rfs` | FILE mode + `--json`: ONE structured diagnostic object on stdout (`{file, ok, errors:[{kind,line,col,message}]}`) - `ok:false` with the error list when invalid | the JSON object on stdout (nothing written to disk) | invalid -> exit 1 with `ok:false`; valid -> exit 0 with `ok:true` |
| `rfs validate --json --input rfs specs/` | DIRECTORY mode + `--json`: an ARRAY of one object per FAILING file on stdout | the JSON array on stdout | any failure -> exit 1 |

### Naming and placement rules

- File mode needs no `--input` (the extension decides); directory mode REQUIRES
  `--input` (there is no single extension to infer).
- In directory text mode, diagnostics are written to `<name>.rfs-validation.err`
  beside each failing source - they are NOT printed to stdout (stderr carries a
  summary). In `--json` mode the diagnostics go to stdout instead.

### Error details and fixes

- Each error has a `kind` (`parse` or `semantic`), a `line`/`col` (0/0 for
  module-level errors that have no single token), and a `message`. The
  invalid_spec_diagnosis example walks one real diagnostic end to end.
- A spec can be syntactically valid yet semantically broken (e.g. a duplicate
  transition priority) - validation catches exactly this class.

## 5. rfs info / rfs info format - introspection + self-describing formats

`rfs info` is static spec introspection: per module, the version, initial
state, enums, events/outputs with variants + field types, variables with types
+ init, the transition COUNT, and every property/assumption with its
monitor-subset status (`monitored` / `skipped: reason`). No model checking.
`rfs info format` renders machine-readable descriptions of rfs's own
input/output formats from in-binary models (so they stay in sync with the code).

| CLI pattern | Options - detailed description | Generated files / output | Possible errors (exit codes) |
|---|---|---|---|
| `rfs info spec.rfs` | static introspection of the FIRST module, human-readable on stdout | text report on stdout (nothing written) | parse/validation error -> exit 1 |
| `rfs info --json spec.rfs` | the same view as a machine-readable document (stable key order, all keys present) - the `info-json` format | the JSON document on stdout | parse/validation error -> exit 1 |
| `rfs info format --list` | list every registered format with its available outputs + a one-line description | the format table on stdout | none (always succeeds) |
| `rfs info format spec-json --output schema` | render ONE format document; `--output md\|plantuml\|svg\|schema` selects the kind (default = the format's first). The five JSON formats (`spec-json`, `ce-trace`, `mc-report`, `validate-json`, `info-json`) support `md` + `schema` (a draft-07 JSON schema) | the document on stdout - for `--output schema`, a standalone JSON Schema file you can validate against WITHOUT the rfs source | unknown format name or unsupported output kind -> exit 1 |

### Naming and placement rules

- `info` and `info format` are READ-ONLY: they print to stdout and write no
  files. To keep a schema, redirect it (`rfs info format mc-report --output
  schema > mc-report.schema.json`).
- The JSON-schema outputs are the contract the json_schema_ci example validates
  real tool output against in CI.

### Error details and fixes

- An unknown `--output` kind for a format (e.g. `--output svg` on a JSON-only
  format) is a usage error, exit 1 - check `rfs info format --list` for each
  format's supported outputs.
- A spec that fails to parse/validate produces the same diagnostics as section 4.

## 6. rfs codegen rust|c - artifact generation (the take-or-emit contract)

Generate production FSMs, test suites, and model-checking binaries for either
backend. Each command runs on the FIRST module of the spec and writes its
artifacts to files named from **BASE** - the `--out` value, or by default the
spec file's stem - in the CWD; per-artifact flags override individual names.
The `mc` and `test` commands REFERENCE the fsm artifact: an existing one is
TAKEN as-is, a missing one is EMITTED new, so one invocation always yields a
complete, self-consistent set. This take-or-emit behavior is the nuance most
likely to surprise - it is stated in the table rows below AND in the naming-rules
subsection.

| CLI pattern | Options - detailed description | Generated files / output | Possible errors (exit codes) |
|---|---|---|---|
| `rfs codegen rust production spec.rfs` | no output options -> BASE = spec stem `spec`. Emits ONLY the fsm artifact | `spec-fsm.rs` in the CWD, exit 0 | parse/validation error exit 1 |
| `rfs codegen rust production spec.rfs --out arq` | `--out BASE` shortcut names the whole set; here just the fsm | `arq-fsm.rs`, exit 0 | parse/validation error exit 1 |
| `rfs codegen c production spec.rfs --out BASE` | C production is a header/source PAIR (the source self-includes its header) | `BASE-fsm.h` + `BASE-fsm.c`, exit 0. Compile: `gcc -Wall -c BASE-fsm.c` | parse/validation error exit 1; `--out -` refused (two artifacts) |
| `rfs codegen rust mc spec.rfs --out BASE` | `--fsm-file NAME[.rs]` (or derived from BASE): names the FSM the generated MC references - an EXISTING file is taken as-is, a missing one is emitted new | `BASE-mc.rs` (always fresh) + `BASE-fsm.rs` (taken or emitted); the mc references the fsm via `include!`. Compile: `rustc --edition 2021 BASE-mc.rs -o mc` | usage error (partial/unknown flag) exit 1; parse/validation error exit 1; stale-reuse warning on stderr (not an error) |
| `rfs codegen c mc spec.rfs --out BASE` | `--fsm-file STEM` (or `--fsm-file-header H --fsm-file-source C`): names the FSM pair the generated MC includes - an EXISTING pair is taken as-is (a PARTIAL pair is a usage error), a missing one is emitted new, so one invocation always yields a complete set; a taken fsm whose line-1 spec hash differs from the current spec prints a stale-reuse warning but is still referenced | `BASE-mc.h` + `BASE-mc.c` (always written fresh); `BASE-fsm.h` + `BASE-fsm.c` (taken or emitted). Compile: `gcc -Wall -c BASE-fsm.c BASE-mc.c && gcc -Wall *.o -o mc` | usage error (partial fsm pair, unknown flag) exit 1; parse/validation error exit 1; stale-reuse warning on stderr (not an error) |
| `rfs codegen rust test spec.rfs --out BASE` | the `test` command co-emits the fsm + a test artifact + a libfuzzer fuzz skeleton (`BASE-fuzz-target.rs`) | `BASE-fsm.rs` + `BASE-test.rs` + `BASE-fuzz-target.rs`, exit 0 | parse/validation error exit 1 |
| `rfs codegen c test spec.rfs --out BASE` | C test: fsm pair + `BASE-test.c` + `BASE-fuzz-target.c` (the skeleton references the fsm through its header) | `BASE-fsm.h/.c` + `BASE-test.c` + `BASE-fuzz-target.c`, exit 0 | parse/validation error exit 1 |
| `rfs codegen rust production spec.rfs --out -` | `--out -` prints the SINGLE artifact to stdout (rust production ONLY - every other command writes multiple files and refuses `-`) | the fsm source on stdout, nothing written | used with a multi-artifact command -> usage error exit 1 |

Generation options (apply across commands where they make sense):

| Option | Meaning |
|---|---|
| `--assumes all\|none\|label1,label2` | which safety assumptions are active (default `all`) - affects generated monitor/assumption code and MC |
| `--steps N` | example-run length K for the `test` command's random-trace test (default 4) |
| `--no-std` | rust production only: emit `#![no_std]` + `extern crate alloc` with an identical public API (a documented no-op for C) |
| `--with-property-monitors` | compile every monitorable property to an online monitor stepped alongside the FSM (debug-gated in Rust; `-DRFS_PROPERTY_MONITORS` in C) |
| `--with-protocol-fsm` | rust only: emit the opt-in `impl ProtocolFsm for Fsm` so rfs-adapters can drive the artifact |
| `--mode exact\|fuzz`, `--max-states N`, `--timeout-sec N`, `--threads N`, `--nondet`, `--counterexample-format json\|text` | the mc-mode family (exact is the default; see section 9 for mode semantics) |

### Artifact naming and take-or-emit rules

- **BASE derivation:** BASE := the `--out` value when given, otherwise the SPEC
  FILE STEM (file name without extension). All default artifact names are
  derived from BASE.
- **Per-artifact override:** an explicit per-artifact flag (`--out-fsm-code`,
  `--out-mc-code`, `--out-test-code`, `--out-fuzz-target-code`, and the C
  `--out-fsm-header`/`--out-fsm-source`/`--out-mc-header`/`--out-mc-code`) overrides
  the corresponding BASE-derived name for that one artifact.
- **Take-or-emit (the key rule):** the fsm-reference flags (`--fsm-file NAME`
  for rust, `--fsm-file STEM` or `--fsm-file-header H --fsm-file-source C` for
  C) override the BASE-derived fsm name. Whatever name results - given by flag
  OR derived from BASE - is **TAKEN if already present on disk, otherwise EMITTED
  new**. For C the pair must be complete: BOTH files present -> taken; NEITHER
  present -> both emitted; exactly ONE present -> a usage error (no silent
  overwrite of a partial pair).
- **Stale-reuse warning:** a TAKEN fsm whose line-1 spec hash (the v1.0 unified
  header) differs from the current spec's hash prints a stderr WARNING (stale
  reuse) but is still referenced - it is not an error, because you explicitly
  named a file that exists.
- **Relative names resolve against the CWD;** `--out DIR/BASE` places the whole
  set under DIR. The Rust mc references its fsm via `include!`, so both files
  must sit where that path resolves (generate them together, or pass a bare
  `--fsm-file` name and compile from that directory).
- **`--out -`** prints the single artifact to stdout and is allowed for rust
  production ONLY; every multi-artifact command refuses it.

### Error details and fixes

- **Usage errors** (a partial C fsm pair, an unknown flag, `--out -` on a
  multi-artifact command) exit 1 with a message naming the problem.
- **Parse/validation errors** exit 1 with the same diagnostics as section 4 -
  fix the spec first.
- **Stale-reuse** is a WARNING, not an error: re-run without the `--fsm-file`
  flag (or delete the stale file) to force a fresh emit.

## 7. rfs test - generate + compile + run suites

The CI entry point for generated tests: in ONE invocation it generates the
selected suite's artifacts, compiles them with the resolved toolchain, and runs
them. Model checking stays `rfs codegen <backend> mc` + the generated binary
(or `rfs mc-ref`) - `rfs test` does not model-check.

| CLI pattern | Options - detailed description | Generated files / output | Possible errors (exit codes) |
|---|---|---|---|
| `rfs test --backend rust spec.rfs` | default suite = unit; compiles the extended test artifact with `rustc --edition 2021 --test` and runs it | per-test results on stdout; exit 0 if all pass | test failure / compile error / missing rustc -> exit 1 |
| `rfs test --backend c spec.rfs` | unit suite for C: compiles with `gcc -Wall` and runs | per-test results on stdout; exit 0 if all pass | test failure / compile error / missing gcc -> exit 1 |
| `rfs test --backend rust --suite fuzz spec.rfs` | the libFuzzer target built from the emitted skeleton in a TEMP cargo-fuzz project (`cargo fuzz build/run`), run bounded with `-max_total_time` (`--fuzz-max-total-time`, default 30 s; extra runtime flags via `RFS_FUZZ_FLAGS` / `--fuzz-flags`) | fuzzer output on stdout; exit 0 if no crash in the budget | missing cargo-fuzz/clang -> exit 1 (only because fuzz was SELECTED); a fuzzer crash -> exit 1 |
| `rfs test --backend c --suite fuzz spec.rfs` | C fuzz: `clang -O1 -fsanitize=fuzzer,address` over the fsm + skeleton, bounded run | fuzzer output on stdout; exit 0 if no crash in the budget | missing clang -> exit 1 (fuzz selected); a crash -> exit 1 |
| `rfs test --suite unit,fuzz --backend c spec.rfs` | `--suite` is repeatable or comma-separated; run several suites in one invocation | combined results; exit 0 only if EVERY selected suite passes | any selected suite failing -> exit 1 |
| `rfs test --backend c --print-script spec.rfs` | print a self-contained POSIX sh script (with the `# tools:` header) INSTEAD of executing - inspect the resolved invocation without running it | the script on stdout, nothing executed or written | usage/parse/validation error -> exit 1 |

Tool paths/args follow section 2: `RFS_RUSTC`/`RFS_GCC`/`RFS_CLANG`/`RFS_CARGO_FUZZ`
(+ `*_ARGS`) env vars, overridden by the matching `--rustc`/`--gcc`/`--clang`/
`--cargo-fuzz` PATH + `--*-args` flags. A missing clang/cargo-fuzz is an error
ONLY when the fuzz suite is selected - a unit-only run needs just rustc or gcc.

### Naming and placement rules

- Artifacts are generated into a scratch area and compiled there; nothing is
  written into your source tree (unlike `rfs codegen`, which writes named files).
- `--print-script` is the inspection mode: it resolves every tool (section 2)
  and prints the exact commands, so you can audit or reproduce a run by hand.

### Error details and fixes

- **Missing tool for a selected suite** -> exit 1 with a diagnostic naming the
  tool; install it or set the matching `RFS_*` var (section 2). A unit-only run
  never needs clang/cargo-fuzz.
- **Compile error** in generated code is a toolchain bug, not a spec problem -
  the spec already validated. Report it with the `--print-script` output.
- **Test failure / fuzzer crash** -> exit 1; the failing test or crashing input
  is the signal (a libFuzzer crash is kept under `crash-*`).

## 8. rfs diagram - state-diagram + table formats

Render the spec's state machine as dot / puml / mmd / table / md sources.
The PRIMARY behavior is pure TEXT generation with NO external tool calls;
`--render svg` renders IN-PROCESS by default (`--renderer internal`, the default
since Phase 7) so no external binaries are needed. `--renderer external`
invokes the format's real tool (dot/plantuml/mmdc) instead.

| CLI pattern | Options - detailed description | Generated files / output | Possible errors (exit codes) |
|---|---|---|---|
| `rfs diagram --list` | list the registered diagram formats | the format list on stdout | none |
| `rfs diagram --format mmd spec.rfs` | generate ONE format's source (dot/puml/mmd/table/md) for the FIRST module, to stdout | the diagram source on stdout, nothing written | parse/validation error -> exit 1 |
| `rfs diagram --format dot --render svg --out fig.svg spec.rfs` | `--render svg` REQUIRES `--out FILE.svg`; the source file is written next to it and rendered IN-PROCESS (no external tool) by default | `fig.svg` + the `.dot` source beside it | parse/validation error -> exit 1; `--render svg` without `--out` -> usage error exit 1 |
| `rfs diagram --format dot --renderer external --render svg --out fig.svg spec.rfs` | invoke the REAL graphviz `dot` (resolved per section 2) instead of the in-process renderer | `fig.svg` via dot | missing/failing dot -> ERROR exit 1 (no graceful degradation) |
| `rfs diagram --format mmd --renderer external --print-script spec.rfs` | print the resolved external-render invocation (with the `# tools:` header) instead of executing | the script on stdout | usage/parse/validation error -> exit 1 |

### Naming and placement rules

- Source-only invocations (`--format X` with no `--render`) print to stdout and
  write nothing - redirect to keep the file.
- `--render svg` always writes the SVG to `--out FILE.svg` AND the intermediate
  source file next to it; it is refused without `--out`.
- The in-process renderers (the default) cover dot/puml/mmd with no external
  dependencies; only `--renderer external` needs dot/plantuml/mmdc installed.

### Error details and fixes

- **Missing or failing external render tool** (only under `--renderer external`)
  is an ERROR, exit 1 - install the tool or set its `RFS_*` var, or drop back to
  the in-process default. The mmdc headless-Chrome requirement is the
  [TOOLCHAIN.md section 6.4](../../TOOLCHAIN.md) gotcha (`RFS_MERMAID_PUPPETEER_ARGS`).
- **`--render svg` without `--out`** is a usage error, exit 1.

## 9. rfs mc-ref - the in-process reference model checker

Run the reference model checker IN-PROCESS (no compilation) - rapid
prototyping in a shell or REPL, in ALL THREE modes. The mode SEMANTICS (what
exact/fuzz/smt-bounded each prove, the status taxonomy) live in
[MODEL_CHECKING.md](MODEL_CHECKING.md) and are linked, not re-derived here;
this section covers the CLI surface and how to READ the reports. Exit codes
mirror the generated exact-mode binaries: `0` pass-complete, `1` fail,
`2` unknown (state-limit/timeout).

| CLI pattern | Options - detailed description | Output | Possible errors (exit codes) |
|---|---|---|---|
| `rfs mc-ref spec.rfs` | `--mode exact` (the default): exhaustive exact reachable-state search. `--max-states N` (default 1000000), `--timeout-sec N`, `--nondet` (fire EVERY enabled transition as its own successor), `--counterexample-format json\|text` (text default) | the unified report: a `model-check summary:` line + statistics, and on failure a counterexample (text trace or a single JSON document) | pass-complete exit 0; fail exit 1; state-limit/timeout exit 2 |
| `rfs mc-ref spec.rfs --mode fuzz` | seeded random testing with the generated fuzz runtimes' exact draw sequence. `--bound N` (5), `--runs N` (20), `--seed N` (12345), `--violate label:prob:percent%,...`, `--unexpected-prob P` (0.0) | exit 0: no property violation (a summary line); exit 1: a violation found (a per-run report, NO summary line) | exact-only flags under `--mode fuzz` are a usage error, exit 1 |
| `rfs mc-ref spec.rfs --mode smt-bounded` | k-step SMT unrolling over z3 IN-PROCESS. `--bound N` (10) + `--full-properties` (check the FULL property class - without it those properties are omitted with a per-property warning and a `properties_omitted=M` summary suffix); specs with assumptions or on_error are REJECTED; requires the `smt` cargo feature | exit 0 pass-within-bound / 1 fail / 2 unknown | every flag but `--bound`/`--full-properties` is a usage error, exit 1 (`--full-properties` under other modes too); an assumption-bearing spec is rejected, exit 1 |

`--assumes all\|none\|labels` applies across modes (which safety assumptions are
active). NOT applicable: `--threads`/`--fast` (ref_mc is single-threaded).
Reports are unified with the generated binaries per mode - byte-comparable where
deterministic.

### Reading a counterexample - real captures from this repository

**A property-violation trace** (from the mc_property_violation_fail example - an
HTTP/2-like stream whose `RstStream` arm is retargeted off `Closed`, breaking
`prop rst_goes_closed : always (input_is(RstStream) -> state == Closed)`):

```text
FAIL
model-check summary: status=fail states=34 transitions=64 max_depth=2 reason=property-violation label=rst_goes_closed step=2
  unexpected terminations: 69
  first unexpected: config 0 input index 0
  monitor size: 10 bits
  memory estimate: 1190 bytes
property violation: rst_goes_closed
  violation step: 2
  vars at violation step: Vars { send_window: 15 }
counterexample trace:
  initial state=State::Idle vars=Vars { send_window: 16 }
  step 0 input=Input::SendData { w: 1 } outputs=[Output::DataSent] from=State::Idle to=State::Open vars=Vars { send_window: 15 }
  step 1 input=Input::RstStream outputs=[Output::StreamReset] from=State::Open to=State::HalfClosedLocal vars=Vars { send_window: 15 }
```

Field by field: `reason=property-violation` + `label=rst_goes_closed` +
`step=2` - the monitor for that property went false after step 2 (steps are
1-based over the trace). The trace is a PREFIX, not a dead end: it stops exactly
at the step where the property breaks. Step 0 opens the stream (`SendData { w: 1 }`
decrements `send_window` 16 -> 15 and emits `DataSent`); step 1 processes
`RstStream` - the machine moves to `HalfClosedLocal`, and THAT is where the
property breaks (after a `RstStream` the state must be `Closed`). The `vars at
violation step` line carries the variable context because properties can depend
on variables. The JSON form (`--counterexample-format json`) uses
`counterexample.kind: "trace"` with one entry per step (input variant + fields,
outputs, from/to state, vars) - the same two steps, machine-readable:

```json
{"status":"fail","metrics":{"states":34,"transitions":64,"max_depth":2},"reason":"property-violation","label":"rst_goes_closed","step":2,"statistics":{"unexpected_terminations":69,"monitor_size_bits":10,"memory_estimate_bytes":1190,"peak_rss_kb":0},"counterexample":{"kind":"trace","initial_state":"State::Idle","initial_vars":{"send_window":16},"steps":[{"input":{"variant":"SendData","fields":{"w":1}},"outputs":[{"variant":"DataSent","fields":{}}],"from_state":"State::Idle","to_state":"State::Open","vars":{"send_window":15}},{"input":{"variant":"RstStream","fields":{}},"outputs":[{"variant":"StreamReset","fields":{}}],"from_state":"State::Open","to_state":"State::HalfClosedLocal","vars":{"send_window":15}}],"ce_steps":2}}
```

**A deadlock counterexample** (from the mc_deadlock_fail example - a TCP-
handshake-like endpoint whose `Established` state has no outgoing transition):

```text
FAIL
model-check summary: status=fail states=4 transitions=3 max_depth=2 reason=deadlock label=- step=2
  unexpected terminations: 51
  first unexpected: config 0 input index 1
  monitor size: 9 bits
  memory estimate: 140 bytes
counterexample trace:
  initial state=State::Idle vars=Vars { seq: 0, timer: 7 }
  step 0 input=Input::Recv_Syn outputs=[Output::Send_SynAck] from=State::Idle to=State::SynSent vars=Vars { seq: 1, timer: 7 }
  step 1 input=Input::Recv_Ack { ack_seq: 1 } outputs=[] from=State::SynSent to=State::Established vars=Vars { seq: 1, timer: 7 }
```

`reason=deadlock` + `step=2` - the search reached depth 2 and found a
configuration with NO enabled transition. Unlike a violation (a prefix that stops
at a broken property), a deadlock trace ENDS at the stuck configuration: step 0
completes the `Syn`/`SynAck` exchange, step 1 the handshake into `Established`, and
there nothing can ever fire again. There is no cycle - a liveness failure would
show one. The JSON form uses `counterexample.kind: "deadlock-config"`: the stuck
configuration itself (`state`, `vars`, `depth`) rather than a step-by-step trace:

```json
{"status":"fail","metrics":{"states":4,"transitions":3,"max_depth":2},"reason":"deadlock","label":"-","step":2,"statistics":{"unexpected_terminations":51,"monitor_size_bits":9,"memory_estimate_bytes":140,"peak_rss_kb":0},"counterexample":{"kind":"deadlock-config","depth":2,"state":"State::Established","vars":{"seq":1,"timer":7},"ce_steps":2}}
```

### Naming and placement rules

- `mc-ref` is IN-PROCESS: it writes no files and needs no compiler - the report
  goes to stdout. Redirect it to keep a record.
- `--counterexample-format json` emits a SINGLE machine-readable document (the
  `mc-report` format) byte-identical across backends where deterministic; the
  text form is the pinned human-readable default.

### Error details and fixes

- **exit 2 (unknown)** means the search hit its state-limit or timeout without
  deciding - raise `--max-states` / `--timeout-sec`, or the spec is too large for
  exact mode (try `--mode fuzz` for a sampled answer).
- **A usage error** (an exact-only flag under `--mode fuzz`, any flag but
  `--bound`/`--full-properties` under `--mode smt-bounded`, `--full-properties`
  under the other modes, an assumption-bearing spec under smt-bounded) exits 1
  naming the offending flag/spec.
- **A parse/validation error** exits 1 with the section-4 diagnostics - validate
  the spec first.

## 10. rfs gen-spec - random / base-mutated spec generation

Write a random spec to a file - either fully random from a seed, or a known base
spec mutated K times. Output is ALWAYS parsed + validated before writing (the
always-valid guarantee), and generation is DETERMINISTIC from the seed (same args
=> byte-identical file). This is the rapid-prototyping entry point: invent a
machine, check it instantly with `rfs mc-ref`, mutate it to probe corner cases.

| CLI pattern | Options - detailed description | Generated files / output | Possible errors (exit codes) |
|---|---|---|---|
| `rfs gen-spec --out base.rfs --seed 17` | fully random mode: `--seed N` (default the pinned constant 12345) + `--size small\|medium`, or explicit caps `--max-states/--max-events/--max-vars/--max-transitions/--max-props` | `base.rfs` - a valid, seed-determined spec | any internal parse/validation failure -> exit 1 with a diagnostic (should not happen: validity is by construction) |
| `rfs gen-spec --base base.rfs --mutations 8 --seed 1 --out mut.rfs` | mutation mode: take an existing base spec and apply K seeded mutations - much better corner-case coverage than fresh random | `mut.rfs` - a valid mutated variant of the base | base fails to parse/validate -> exit 1; any mutation-result validation failure -> exit 1 |

### Naming and placement rules

- The output path is exactly `--out FILE.rfs`; there is no default name (you
  must say where the generated spec goes).
- Determinism is the contract: the same `--seed` (and mode) reproduces the SAME
  bytes. Change the seed to get a different machine; keep it to reproduce a
  transcript exactly.

### Error details and fixes

- gen-spec output is always valid, so you will not normally see a validation
  error - if one appears it is a toolchain bug. The interesting "failure" is a
  mutated spec that `rfs mc-ref` then FAILS: that is the teaching moment (a
  mutation broke a property), not an error in gen-spec itself.

## 11. Example projects

Every integration scenario has a runnable project under `crates/examples/` with
a README that walks it end to end (what it demonstrates, the machine, files,
build/run/test, expected results). The five original projects drive the spec-43
sliding-window ARQ sender; the ten added across Phase 17 cover the remaining
usage scenarios. Start here when you want a WORKING reference for a pattern
rather than a flag description.

| Title | Description + rationale | README |
|---|---|---|
| sync_handler | The generated ARQ sender driven from another thread over std mpsc channels via the channel adapter - the reference Rust integration shape | [README](../../crates/examples/sync_handler/README.md) |
| async_handler | The SAME protocol scenario consumed as a futures Stream (no tokio) - the async integration shape | [README](../../crates/examples/async_handler/README.md) |
| c_sync_handler | The plain-C mirror: generated C FSM + scripted peer, Makefile-driven, with a documented gcc/clang choice (`make CC=clang`) | [README](../../crates/examples/c_sync_handler/README.md) |
| fuzz_rust | The emitted libfuzzer_sys skeleton wired into a detached cargo-fuzz workspace - fuzzing the generated Rust FSM | [README](../../crates/examples/fuzz_rust/README.md) |
| fuzz_c | The C side of the fuzz matrix: the emitted `fuzz_target.c` skeleton built with a libFuzzer-enabled clang | [README](../../crates/examples/fuzz_c/README.md) |
| mc_deadlock_fail (negative) | A valid spec that deadlocks at a terminal state - the counterexample trace, what to look for, and the one-transition fix with re-run proof | [README](../../crates/examples/mc_deadlock_fail/README.md) |
| mc_property_violation_fail (negative) | A valid spec with an unsatisfiable property - step-by-step violation-trace walkthrough + the fix | [README](../../crates/examples/mc_property_violation_fail/README.md) |
| invalid_spec_diagnosis (negative) | An invalid spec: `rfs validate` text + `--json` diagnostics explained field by field, and the corrected spec that validates | [README](../../crates/examples/invalid_spec_diagnosis/README.md) |
| no_std_fsm | Production codegen with `--no-std` compiled in a genuinely `#![no_std]` crate - identical public API to the std build | [README](../../crates/examples/no_std_fsm/README.md) |
| monitors_production | Production build with `--with-property-monitors`: the debug-gated monitor catches an injected violation at the step API; the release build is zero-cost | [README](../../crates/examples/monitors_production/README.md) |
| callback_minimal_adapters | The two rfs-adapters never demonstrated before - the callback driver (all three outcomes) and the no_std minimal wrapper (forwarding + composition) over a real generated spec | [README](../../crates/examples/callback_minimal_adapters/README.md) |
| mc_walkthrough | Generate MC binaries for BOTH backends on a passing spec and a failing variant - reading a report end to end (every summary field, the CE in text + JSON) | [README](../../crates/examples/mc_walkthrough/README.md) |
| json_schema_ci | `rfs info format <name> --output schema` for the five JSON document formats; a CI-style script validating REAL tool output against the schemas | [README](../../crates/examples/json_schema_ci/README.md) |
| specgen_mutation_loop | The rapid-prototyping loop as a runnable thing: gen-spec random + mutation, validate, mc-ref, fix, repeat - with a pinned real transcript | [README](../../crates/examples/specgen_mutation_loop/README.md) |
| tool_resolution | Explicit external-tool path passing end to end: `RFS_*` env vars + `--print-script` `# tools:` proof of the precedence model, plus a gcc/clang Makefile choice | [README](../../crates/examples/tool_resolution/README.md) |
