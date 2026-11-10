//! Shared CLI argument processing for the `rfs` command plugins.
//!
//! Holds the pieces every top-level command has in common: the global `usage()`
//! help text (printed by any command on `--help`), the `--input`/`--output`
//! format taker, a generic `--flag VALUE` taker, and the conventional error
//! helpers. Each plugin's `run_cli` keeps its own command-specific flag arms but
//! calls these for the shared parts.
//! Docs: dev:cli-commands-dispatch

use std::process::ExitCode;

use crate::cli::utils::Format;

pub fn usage() -> String {
    "RFS toolchain\n\nusage:\n  rfs parse --input <rfs|json> --output <rfs|json> <file-or-dir> [--target-dir DIR]\n  rfs validate [--input <rfs|json>] [--json] <file-or-dir>\n  rfs info [--json] [--log-level off|error|warn|info|debug] <spec.rfs|spec.json>\n  rfs info format --list | <name> [--output md|plantuml|svg|schema]\n  rfs test [--backend rust|c] [--suite unit|fuzz ...] [--steps N] [--rustc PATH] [--gcc PATH]\n           [--rustc-args \"...\"] [--gcc-args \"...\"] [--clang PATH] [--clang-args \"...\"]\n           [--cargo-fuzz PATH] [--cargo-fuzz-args \"...\"] [--fuzz-max-total-time SEC] [--fuzz-flags \"...\"]\n           [--print-script] [--log-level off|error|warn|info|debug] <spec.rfs|spec.json>\n  rfs diagram --list | --format dot|puml|mmd|table|md [--renderer internal|external] [options] <spec.rfs|spec.json>\n  rfs codegen --list | --help | <backend> --help\n  rfs codegen <backend> <production|test|mc> <spec.rfs|spec.json> [--out FILE] [--assumes ...] [--steps N]\n  rfs mc-ref <spec.rfs|spec.json> [--max-states N] [--timeout-sec N] [--assumes all|none|labels]\n           [--counterexample-format json|text] [--log-level off|error|warn|info|debug]\n  rfs gen-spec --out FILE.rfs [--seed N] [--size small|medium] [--max-states N] [--max-events N]\n           [--max-vars N] [--max-transitions N] [--max-props N]\n      | rfs gen-spec --base SPEC.rfs|SPEC.json --mutations K [--seed N]\n\nparse: load RFS or IR-JSON files, convert through the IR, and write formatted\n       output. With a directory input, only files matching --input are scanned;\n       without --target-dir each output is written next to its input file, with\n       --target-dir the input directory structure is preserved inside it. The\n       original extension is replaced (never stacked). Conversions whose output\n       would overwrite the input file are refused.\n\nvalidate: parse + semantically validate RFS or IR-JSON files. For a file input\n         the extension selects the format and errors go to stderr; for a\n         directory input --input is required and each failing source file gets\n         a <name>.rfs-validation.err file next to it. `--json` emits structured\n         diagnostics on stdout instead (file mode: one object; directory mode:\n         an array of one object per failing file) - the shape editor\n         integrations consume (Phase 5 task 13).\n\ninfo: static spec introspection - per module: version, initial state, enums,
      events/outputs with variants + field types, variables with types + init,
      transition count, and properties/assumptions each with monitor-subset
      status (monitored / skipped: reason). No model checking. `--json` emits
      a machine-readable document (stable key order, all keys present); 
      `--log-level` filters stderr progress output.

info format: machine-readable descriptions of rfs input/output formats,
      rendered from the in-binary models so they stay in sync with the code
      (Phase 7). `--list` shows registered formats; `<name> [--output KIND]`
      prints one document to stdout (default output = the format's first).
      Formats: spec-rfs (RFS v0 EBNF grammar; md, plantuml, svg), spec-json /
      ce-trace / mc-report / validate-json / info-json (md + draft-07 JSON
      schema), diagram-dot / diagram-puml / diagram-mmd (generated subset,
      md), artifact-header (md).

test: generate + compile + run the selected test suites for the spec in one
      invocation (the CI entry point for generated tests; model checking stays
      `rfs codegen <backend> mc` + the generated binary). Suites (--suite,
      repeatable or comma-separated, default unit): unit = the extended test
      artifact compiled with rustc --edition 2021 --test (backend rust) or
      gcc -Wall (backend c), per-test results on stdout; fuzz = the libFuzzer
      target built from the emitted skeleton (C: clang -O1
      -fsanitize=fuzzer,address; Rust: a temp cargo-fuzz project via cargo
      fuzz build/run) run bounded with -max_total_time (--fuzz-max-total-time,
      default 30 s; extra runtime flags via RFS_FUZZ_FLAGS / --fuzz-flags).
      Tool paths/args: RFS_RUSTC/RFS_GCC/RFS_CLANG/RFS_CARGO_FUZZ (+ *_ARGS)
      env vars, overridden by the matching PATH + --*-args flags; missing
      clang/cargo-fuzz is an error only when the fuzz suite is selected.
      `--print-script` prints a self-contained POSIX sh script instead of
      executing. Exit 0 = every selected suite passes; exit 1 = test failure,
      fuzzer crash, compile error, missing tool for a selected suite, or
      usage/parse/validation error (consistent with the other commands).

diagram: state-diagram + table formats as plugins (one module per format).
         PRIMARY behaviour is pure text generation with NO external tool calls;
         `--render svg` REQUIRES --out FILE.svg (the source file is written
         next to it) and renders IN-PROCESS by default (--renderer internal,
         the default since Phase 7): no external binaries are needed.
         --renderer external invokes the format's tool (dot/plantuml/mmdc)
         instead; a missing or failing render tool is an ERROR (exit 1) - no
         graceful degradation. Tool paths/args: RFS_DOT/RFS_PLANTUML/
         RFS_MERMAID (+ *_ARGS) env vars, overridden by --dot/--plantuml/
         --mermaid PATH + --*-args flags; `--print-script` prints a
         self-contained POSIX sh script instead of executing. Runs on the
         first module of the spec. See BUILD.md/TOOLCHAIN.md for installing
         dot/plantuml/mermaid (only needed for --renderer external).

mc-ref: run the reference model checker in-process (no compilation) - rapid\n         prototyping in shell/REPL. Exhaustive exact search per SEMANTICS.md\n         section 9 with unified reports: the SAME text format and JSON document\n         as the generated exact-mode binaries, byte-comparable where\n         deterministic. Exit codes mirror the generated binaries: 0\n         pass-complete, 1 fail, 2 unknown-state-limit. Flags mirror the\n         generated exact mode where applicable; NOT applicable (documented in\n         `rfs mc-ref --help`): --seed/--steps/--violate (fuzz-mode concepts),\n         --threads/--fast (ref_mc is single-threaded), --nondet (ref_mc explores\n         all enabled transitions by construction).\n\ngen-spec: write a random spec (fully random from a seed, or a base spec mutated\n         K times) to FILE.rfs. Output is always parsed + validated before\n         writing; on any failure exit 1 with a diagnostic. Fully random mode:\n         --seed N (default: the pinned constant 12345) + --size small|medium\n         (or explicit caps --max-states/--max-events/--max-vars/\n         --max-transitions/--max-props). Mutation mode: --base SPEC +\n         --mutations K. Deterministic from the seed (same args => byte-identical\n         file).\n\ncodegen: target-language plugin registry. `--list` shows registered backends;\n         `<backend> --help` lists that backend's commands. Each command runs on\n         the first module of the spec and writes generated source to --out or\n         stdout. The rust `test` command additionally emits fuzz_target.rs\n         (a libfuzzer_sys skeleton) next to --out, so it requires --out.\n         See CODEGEN.md for the backend plugin API.\n\nexamples:\n  cargo run -- parse --input rfs --output json specs/handshake.rfs\n  cargo run -- parse --input rfs --output rfs specs/ --target-dir out/\n  cargo run -- validate specs/handshake.rfs\n  cargo run -- validate --input rfs specs/\n".to_string()
}

pub fn take_format(args: &[String], i: &mut usize, flag: &str) -> Result<Option<Format>, String> {
    let Some(raw) = args.get(*i + 1) else {
        return Err(format!("{} requires a value ('rfs' or 'json')", flag));
    };
    *i += 1;
    raw.parse::<Format>()
        .map(Some)
        .map_err(|e| format!("{}: {}", flag, e))
}

/// Take a `--flag VALUE` pair, advancing the index. Generic value taker shared
/// by every plugin (replaces the hand-rolled "requires a value" arms).
pub fn take_value(args: &[String], i: &mut usize, flag: &str) -> Result<String, String> {
    let Some(v) = args.get(*i + 1).cloned() else {
        return Err(format!("{} requires a value", flag));
    };
    *i += 1;
    Ok(v)
}

/// Conventional "unknown option" error: message + usage, exit 1.
pub fn unknown_option(arg: &str) -> ExitCode {
    eprintln!("unknown option: {}\n{}", arg, usage());
    ExitCode::FAILURE
}

/// Conventional "unexpected extra argument" error: message + usage, exit 1.
pub fn unexpected_arg(arg: &str) -> ExitCode {
    eprintln!("unexpected extra argument: {}\n{}", arg, usage());
    ExitCode::FAILURE
}

/// Conventional "missing required argument(s)" error: message + usage, exit 1.
pub fn missing_required(msg: &str) -> ExitCode {
    eprintln!("error: {}\n{}", msg, usage());
    ExitCode::FAILURE
}
