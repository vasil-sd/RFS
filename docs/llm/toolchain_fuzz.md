<!-- llm-budget: 200 -->
# RFS Toolchain: Fuzz + Training (Class B)

Compact card for LLM assistants working on fuzz targets, training data, or
the generated-code test harness.

<a id="llm:toolchain-fuzz"></a>
## Card

**Responsibility.** The development tooling layer: D-P6-4's TWO-LAYER
toolchain fuzzing (fast in-process libFuzzer targets in the DETACHED root
`fuzz/` workspace + the slow seeded CI suite `tests/toolchain_fuzz_test.rs`),
the training-data pipeline (`src/training.rs` + 2 bin tools over the
`training/<category>/` JSONL trees), and the generated-code test harness (2
bin tools that build an aggregate Rust unit-test crate from every valid spec).
All random specs come from `src/specgen.rs` - raw IR types are deliberately
NOT Arbitrary (validity by construction).

**Target-to-module map.**

| Area | Files |
|---|---|
| Fast fuzz layer | fuzz/fuzz_targets/common.rs (RandSpec wrapper), parser.rs, validator.rs, ir_roundtrip.rs, codegen.rs; detached fuzz/Cargo.toml (never built by the outer cargo test --workspace) |
| Slow CI layer | tests/toolchain_fuzz_test.rs (plain cargo test; 2 tests) |
| Training pipeline | src/training.rs, src/bin/build_training.rs, src/bin/extract_training_snippets.rs |
| Generated-code tests | src/bin/generate_code_tests.rs, src/bin/test_generated_code.rs |

**Key items.**

| Item | Role |
|---|---|
| `fuzz/fuzz_targets/common.rs::RandSpec` | fuzzer bytes -> u64 seed + small/medium flag -> SpecGen random VALID spec; HEAD_LEN = 9 so targets needing raw extra bytes split deterministically (split_head_rest); rand_spec falls back to fixed seed 0 on exhausted input |
| `fuzz/fuzz_targets/parser.rs` | arbitrary bytes -> parse_source must NOT panic; non-UTF-8 skipped (CLI rejects first); on success: validate_ir no-panic + to_json/from_json/to_json BYTE-STABLE (D-P5-2) |
| `fuzz/fuzz_targets/validator.rs` | head = random valid IR -> validate_ir no-panic; rest = raw JSON text -> Ir::from_json never panics (strict parse first, malformed = clean Err) |
| `fuzz/fuzz_targets/ir_roundtrip.rs` | random valid IR, optionally specgen::mutate'd (first raw byte picks k in 0..4; seed from remaining bytes + 0x9e37_79b9) -> JSON round-trip byte-stable |
| `fuzz/fuzz_targets/codegen.rs` | ALL SIX artifacts (2 backends x production/test/mc) generate without error + cheap structural checks: line-1 header with spec sha256, balanced delimiters (comment/string/char-literal-aware scanner), no placeholder tokens, core symbols (pub struct Fsm / fsm_t+fsm_new); NO compilation in the fuzzer |
| `tests/toolchain_fuzz_test.rs` | seeded CI half: 3 pinned seeds x 10 small specs (24099 = plan-correction-5a regression seed) -> render + re-parse + validate, all six artifacts COMPILED (rustc / gcc -Wall), MC exact + fuzz mode vs ref_mc ground truth |
| `src/training.rs::validate_training_line` | pins the JSONL contract: exactly 2 messages (user then assistant), non-empty user content, assistant in `< think> ... < /think>` CoT format (the spaced tags are the pinned format) |
| `src/training.rs::extract_category_snippets` | dedupes ```rfs snippets across a category, classifies by parse+validate, REWRITES snippets/{valid,invalid}/ from scratch with sorted 1-based snippet_NNNN.rfs names |
| `src/bin/generate_code_tests.rs` / `test_generated_code.rs` | collect examples/valid + every category's snippets/valid (invalid deliberately excluded), sort+dedupe; generate_aggregate_test_crate -> target/rfs_codegen_tests; the second tool then cargo test --manifest-path it - exit code mirrors the generated suite |

**Invariants that must not break.**

- Two-layer split (D-P6-4): libFuzzer sessions are long-running and stay
  manual/nightly (Rust-side needs nightly per TOOLCHAIN.md section 7.5);
  tests/toolchain_fuzz_test.rs is the CI-runnable layer - keep it seeded,
  deterministic, and compile-and-run based.
- No false pass: where ref_mc (independent ground truth, D-P6-5) says Fail,
  a generated engine must NOT say pass-complete; MC exit codes mirror
  {0 pass-complete, 1 fail, 2 unknown-state-limit}; stable diagnostics = each
  MC binary run twice yields byte-identical stderr.
- Documented asymmetry: an assumption violated at the initial configuration
  makes ref_mc report a clean error while generated binaries panic/abort -
  both naming the SAME label; that pinned behavior is asserted instead of a
  status comparison.
- Fuzz targets must never PANIC (that is the whole point); malformed input is
  a clean Err; exhausted fuzzer input falls back, not fails.
- Training: a bad JSONL line fails the WHOLE aggregate build (no silent
  skipping); snippet classification is parse+validate based and the snippets
  tree is regenerated from scratch on every extraction.
- Generated-code harness: only WELL-FORMED sources feed the aggregate crate
  (snippets/invalid are expected to fail parsing); the crate lives under
  target/ and is never checked in.

**Change impact.** A new fuzz target = one file + a [[bin]] entry in
fuzz/Cargo.toml (harness = false) + reuse of common.rs helpers; verify with
cargo fuzz build (nightly) or at minimum cargo check in the detached workspace.
A training-format change must update validate_training_line +
tests/training_test.rs together. Verify with cargo test --workspace (559
baseline - includes toolchain_fuzz_test + training_test), then the full
BUILD.md section 5 checklist. Full table: llm:change-impact.

**Deep dive.** dev:toolchain-fuzz-overview + dev:toolchain-fuzz-targets /
dev:toolchain-fuzz-training / dev:toolchain-fuzz-code-tests in
docs/dev/toolchain_fuzz.md; specgen engine: dev:cli-commands-mc-ref-gen-spec;
reference MC ground truth: dev:mc-engine-exact; Rust test backend (aggregate
crate renderer): docs/dev/backend_rust.md; current training counts: KB.md
section 1.
