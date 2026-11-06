# RFS Toolchain: Fuzz + Training (Class A)

The development tooling layer: libFuzzer fuzz targets, the training-data
pipeline, and the generated-code test harness.

<a id="dev:toolchain-fuzz-overview"></a>
## Overview

The development tooling layer has three parts, all driven by the Phase 6
two-layer fuzzing decision (D-P6-4) and the training/code-test pipelines:

1. **Toolchain fuzzing** - a root-level DETACHED cargo-fuzz workspace
   `fuzz/` (package `rfs-fuzz`, never built by the outer cargo test
   --workspace) with four in-process libFuzzer targets + shared helpers, and
   the slow CI-runnable half: `tests/toolchain_fuzz_test.rs` in plain cargo
   test. Random VALID specs come from `src/lang/specgen.rs` through a shared
   wrapper - raw IR types are deliberately NOT made Arbitrary (validity by
   construction; deep dive dev:cli-commands-mc-ref-gen-spec).
2. **Training-data pipeline** - `src/training.rs` + two bin tools that
   aggregate the per-category JSONL sets and extract/validate the snippet
   trees under `training/`.
3. **Generated-code test harness** - two bin tools that generate an aggregate
   Rust unit-test crate from every valid example + valid training snippet and
   compile/run it.

Target-to-module map:

| Area | Files | Deep-dive fragment |
|---|---|---|
| Fuzz targets (fast layer) | `fuzz/fuzz_targets/{common,parser,validator,ir_roundtrip,codegen}.rs` + detached `fuzz/Cargo.toml` | dev:toolchain-fuzz-targets |
| Slow CI suite (same fragment - D-P6-4's two layers) | `tests/toolchain_fuzz_test.rs` | dev:toolchain-fuzz-targets |
| Training pipeline | `src/training.rs`, `src/bin/build_training.rs`, `src/bin/extract_training_snippets.rs` | dev:toolchain-fuzz-training |
| Generated-code tests | `src/bin/generate_code_tests.rs`, `src/bin/test_generated_code.rs` | dev:toolchain-fuzz-code-tests |

The new fuzz targets found two real bugs during Phase 6, recorded as plan
corrections 5a (planner binding-name keying - codegen FAIL on a fully valid
spec; regression seed pinned in the CI suite) and 5b (C MC fuzz-mode OOB read
at trigger step 0). Current training-set counts (categories, examples,
valid/invalid snippets) live in [KB.md](../../KB.md) section 1 - this doc describes the
machinery, not the numbers.

Code: `fuzz/fuzz_targets/common.rs::RandSpec`,
`src/training.rs::aggregate_training_sets`


<a id="dev:toolchain-fuzz-targets"></a>
## Fuzz targets + slow CI suite (`fuzz/`, `tests/toolchain_fuzz_test.rs`)

D-P6-4's two layers: fast in-process libFuzzer (manual/nightly, long-running
sessions) + a seeded deterministic CI suite. Both consume the SAME specgen
builder.

**Shared wrapper.** `fuzz/fuzz_targets/common.rs::RandSpec` implements
Arbitrary by mapping fuzzer bytes -> a u64 seed + a size flag (small vs
medium Bounds) and building a random VALID single-module spec through
`src/lang/specgen.rs::SpecGen::generate` - the deliberate alternative to deriving
Arbitrary over raw IR types, which would produce invalid IR. `HEAD_LEN` = 9
(8 seed bytes + 1 size flag); `fuzz/fuzz_targets/common.rs::split_head_rest`
splits input so targets that ALSO need raw remaining bytes can do so
deterministically; `fuzz/fuzz_targets/common.rs::rand_spec` consumes the whole
input as the head and falls back to a fixed seed (0, small) when the input is
exhausted so the target stays alive. The module carries #![allow(dead_code)]
because each target binary compiles only the helpers it uses.

**The four targets.** All are #![no_main] libfuzzer_sys targets in the
detached `fuzz/` workspace (empty [workspace] table + [package.metadata]
cargo-fuzz = true; libfuzzer-sys 0.4 + arbitrary 1 + rfs by path; harness =
false per bin):

- `fuzz/fuzz_targets/parser.rs`: arbitrary bytes -> `src/lang/parser.rs::parse_source`
  must NOT panic. Non-UTF-8 input is skipped (the CLI rejects such files
  before parsing, so a &str API cannot see it). On success: semantic
  validation must not panic (valid or invalid result), and the parsed IR must
  serialize, strict-parse, and re-serialize BYTE-STABLY (to_json -> from_json
  -> to_json equality - the D-P5-2 contract on parser output).
- `fuzz/fuzz_targets/validator.rs`: two parts over a head/rest split - random
  VALID IRs must not panic `src/lang/validator.rs::validate_ir`; the remaining raw
  bytes as JSON text must not panic `src/lang/ir.rs::Ir::from_json` (strict parse
  first; malformed JSON is a clean Err).
- `fuzz/fuzz_targets/ir_roundtrip.rs`: random valid IRs, optionally mutated
  through the task-3 `src/lang/specgen.rs::mutate` API (the first raw byte picks k
  in 0..4; the mutation seed derives from the remaining bytes + the golden-
  ratio constant 0x9e37_79b9) - the result must satisfy to_json -> from_json
  -> to_json byte-stability. Exhausted input = nothing to round-trip this
  round (clean return).
- `fuzz/fuzz_targets/codegen.rs`: a random valid spec generates ALL SIX
  artifacts (2 backends x production/test/mc) via the
  `src/codegen.rs::CodegenCommandKind` registry without error, and every
  artifact passes cheap structural checks (string scans only - they run on
  every fuzzer iteration): line 1 is the task-2 header (`// Generated by RFS
  v...` + `spec sha256:` field); delimiters are balanced under a scanner that
  skips line/block comments, string literals, and char literals (both the
  escape form - exactly opening quote + backslash + one char + closing quote
  - and the one-char form, which lifetimes deliberately do NOT match); no
  placeholder tokens (TODO/FIXME/PLACEHOLDER/unimplemented! in Rust output,
  TODO in C); core symbols present (`pub struct Fsm` / `fsm_t` + `fsm_new`).
  NO compilation inside the fuzzer - that is the slow layer's job.

**The slow CI suite.** `tests/toolchain_fuzz_test.rs` (plain cargo test, the
CI-runnable half): pinned seeds x small specgen specs (3 seeds x 10 specs per
the stateful sequence; seed 24099 is the plan-correction-5a regression seed -
its first spec is the one the codegen target originally found) are rendered to
source and re-parsed + validated (exercising printer + parser), then ALL SIX
artifacts are generated and COMPILED (rustc / gcc -Wall - compilation success
IS the "no invalid target-language code" check). The MC binaries run in exact
mode (shared small --max-states cap) AND fuzz mode; per spec the suite asserts:
exit codes mirror the status taxonomy {0 pass-complete, 1 fail, 2 unknown-
state-limit} and fuzz-mode runs stay in {0,1,2}; NO FALSE PASS - where
`src/ref_mc.rs` (independent ground truth, D-P6-5) says Fail, a generated
engine must NOT say pass-complete, with full status agreement per the task
4.1 semantics; STABLE DIAGNOSTICS - each MC binary run twice yields byte-
identical stderr. One documented asymmetry is handled like Phase 6 task 4.1:
an assumption already violated at the initial configuration makes ref_mc
report a clean error while the generated binaries panic/abort - both naming
the SAME label, and that pinned behavior is asserted instead of a status
comparison.

Code: `fuzz/fuzz_targets/common.rs::RandSpec`,
`fuzz/fuzz_targets/common.rs::HEAD_LEN`, `fuzz/fuzz_targets/common.rs::split_head_rest`,
`fuzz/fuzz_targets/common.rs::rand_spec`, `fuzz/fuzz_targets/parser.rs`,
`fuzz/fuzz_targets/validator.rs`, `fuzz/fuzz_targets/ir_roundtrip.rs`,
`fuzz/fuzz_targets/codegen.rs`, `tests/toolchain_fuzz_test.rs`


<a id="dev:toolchain-fuzz-training"></a>
## Training-data pipeline (`src/training.rs`, two bin tools)

**Tree layout.** `training/<category>/rfs_training.jsonl` holds the category's
SFT examples; `training/<category>/snippets/{valid,invalid}/*.rfs` are the
extracted snippets; `all_rfs_training.jsonl` at the project root is the
GENERATED aggregate of all category JSONL files. Category discovery is sorted
for determinism.

**Core API.** `src/training.rs::SnippetCounts { valid, invalid }` + total();
`training_root` / `category_dirs` (sorted; missing training/ = empty) /
`category_jsonl_path`; `read_jsonl_lines` (skips blank lines).
`src/training.rs::validate_training_line` pins the JSONL contract: each line
is a JSON object with a `messages` array of EXACTLY two entries - first role
user, second role assistant; user content non-empty; assistant content must
use the `< think> ... < /think>` chain-of-thought format. (The literal tags
carry spaces - that is the pinned format, not a typo.)
`src/training.rs::aggregate_training_sets` walks the categories in order,
VALIDATES every line (a bad line fails the whole build), writes the aggregate
(parent dirs created), and returns the total count.

**Snippet extraction.** `src/training.rs::extract_rfs_snippets` pulls ```rfs
fenced blocks out of message content (CRLF/LF tolerant, trimmed, empty
blocks dropped). `src/training.rs::extract_category_snippets` dedupes snippets
across the whole category (HashSet), classifies each by parse + validate
(parse error OR validation failure = invalid), then REWRITES the category's
snippets/ directory from scratch (remove_dir_all + recreate) with sorted,
1-based `snippet_NNNN.rfs` names in valid/ and invalid/. `src/training.rs::extract_all_snippets`
runs it over every category.

**Bin tools.** `src/bin/build_training.rs`: aggregate all categories into
all_rfs_training.jsonl (or an explicit output path argument); prints the count;
exit 1 with a diagnostic on any validation failure. `src/bin/extract_training_
snippets.rs`: extract per category and print valid/invalid counts + totals.
Both resolve the project root from CARGO_MANIFEST_DIR.

**Pinning.** tests/training_test.rs: categories exist and every JSONL line is
valid; snippets parse exactly as classified (valid/ parses + validates,
invalid/ does not); the aggregate matches the sum of its parts. Current
counts: [KB.md](../../KB.md) section 1.

Code: `src/training.rs::SnippetCounts`, `src/training.rs::category_dirs`,
`src/training.rs::validate_training_line`, `src/training.rs::aggregate_training_sets`,
`src/training.rs::extract_rfs_snippets`, `src/training.rs::extract_category_snippets`,
`src/training.rs::extract_all_snippets`, `src/bin/build_training.rs`,
`src/bin/extract_training_snippets.rs`, `tests/training_test.rs`


<a id="dev:toolchain-fuzz-code-tests"></a>
## Generated-code test harness (two bin tools)

The aggregate unit-test crate proves the generated Rust code COMPILES and its
tests PASS for every well-formed spec in the repo - a slow, exhaustive
complement to the fuzz layers.

**Source collection.** Both tools collect `examples/valid/*.rfs` plus each
training category's `snippets/valid/` (the invalid snippets are EXPECTED to
fail parsing and are deliberately excluded), then sort + dedupe. An empty
collection is an error.

**Generation.** `src/bin/generate_code_tests.rs`: flags --test-steps N
(default 4) and --assumes all|none|labels (default All); builds
`src/codegen.rs::BackendOptions` and calls
`src/backends/rust_test.rs::generate_aggregate_test_crate` (the extended shared
test planner rendered by the Rust test backend - deep dive
[backend_rust.md](backend_rust.md)), then `src/backends/rust_test.rs::write_generated_crate`
under target/rfs_codegen_tests. Warnings stream to stderr; codegen errors fail
the build.

**Compile + run.** `src/bin/test_generated_code.rs`: the same collection with
fixed options (steps 4, assumes All), regenerates the crate, then runs
cargo test --manifest-path target/rfs_codegen_tests/Cargo.toml - the exit code
mirrors the generated suite's. The aggregate crate is a GENERATED artifact
under target/ (never checked in); its size grows with spec + valid-snippet
coverage (current counts: [KB.md](../../KB.md) section 1).

Code: `src/bin/generate_code_tests.rs`, `src/bin/test_generated_code.rs`,
`src/backends/rust_test.rs::generate_aggregate_test_crate`,
`src/backends/rust_test.rs::write_generated_crate`
