use std::env;
use std::path::Path;
use std::process::ExitCode;

use rfs::{codegen, frontend, Format};

fn usage() -> String {
    "RFS toolchain\n\nusage:\n  rfs parse --input <rfs|json> --output <rfs|json> <file-or-dir> [--target-dir DIR]\n  rfs validate [--input <rfs|json>] <file-or-dir>\n  rfs info [--json] [--log-level off|error|warn|info|debug] <spec.rfs|spec.json>\n  rfs test [--backend rust|c] [--steps N] [--rustc PATH] [--gcc PATH] [--rustc-args \"...\"]\n           [--gcc-args \"...\"] [--print-script] [--log-level off|error|warn|info|debug]\n           <spec.rfs|spec.json>\n  rfs diagram --list | --format dot|puml|mmd|table|md [options] <spec.rfs|spec.json>\n  rfs codegen --list | --help | <backend> --help\n  rfs codegen <backend> <production|test|mc> <spec.rfs|spec.json> [--out FILE] [--assumes ...] [--steps N]\n\nparse: load RFS or IR-JSON files, convert through the IR, and write formatted\n       output. With a directory input, only files matching --input are scanned;\n       without --target-dir each output is written next to its input file, with\n       --target-dir the input directory structure is preserved inside it. The\n       original extension is replaced (never stacked). Conversions whose output\n       would overwrite the input file are refused.\n\nvalidate: parse + semantically validate RFS or IR-JSON files. For a file input\n         the extension selects the format and errors go to stderr; for a\n         directory input --input is required and each failing source file gets\n         a <name>.rfs-validation.err file next to it.\n\ninfo: static spec introspection - per module: version, initial state, enums,
      events/outputs with variants + field types, variables with types + init,
      transition count, and properties/assumptions each with monitor-subset
      status (monitored / skipped: reason). No model checking. `--json` emits
      a machine-readable document (stable key order, all keys present); 
      `--log-level` filters stderr progress output.

test: generate + compile + run the extended test suite for the spec in one
      invocation (the CI entry point for generated tests; model checking stays
      `rfs codegen <backend> mc` + the generated binary). The artifact is
      compiled with rustc --edition 2021 --test (backend rust) or gcc -Wall
      (backend c) and the executable's per-test results stream on stdout.
      Tool paths/args: RFS_RUSTC/RFS_GCC (+ *_ARGS) env vars, overridden by
      --rustc/--gcc PATH + --*-args flags; `--print-script` prints a
      self-contained POSIX sh script instead of executing. Exit 0 = all tests
      pass; exit 1 = test failure, compile error, or usage/parse/validation
      error (consistent with the other commands).

diagram: state-diagram + table formats as plugins (one module per format).
         PRIMARY behaviour is pure text generation with NO external tool calls;
         `--render svg` invokes the format's renderer (dot/plantuml/mmdc) and
         REQUIRES --out FILE.svg (the source file is written next to it). A
         missing or failing render tool is an ERROR (exit 1) - no graceful
         degradation. Tool paths/args: RFS_DOT/RFS_PLANTUML/RFS_MERMAID
         (+ *_ARGS) env vars, overridden by --dot/--plantuml/--mermaid PATH +
         --*-args flags; `--print-script` prints a self-contained POSIX sh
         script instead of executing. Runs on the first module of the spec.
         See BUILD.md/TOOLCHAIN.md for installing dot/plantuml/mermaid.

codegen: target-language plugin registry. `--list` shows registered backends;\n         `<backend> --help` lists that backend's commands. Each command runs on\n         the first module of the spec and writes generated source to --out or\n         stdout. The rust `test` command additionally emits fuzz_target.rs\n         (a libfuzzer_sys skeleton) next to --out, so it requires --out.\n         See CODEGEN.md for the backend plugin API.\n\nexamples:\n  cargo run -- parse --input rfs --output json specs/handshake.rfs\n  cargo run -- parse --input rfs --output rfs specs/ --target-dir out/\n  cargo run -- validate specs/handshake.rfs\n  cargo run -- validate --input rfs specs/\n".to_string()
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("parse") => cmd_parse(&args[1..]),
        Some("validate") => cmd_validate(&args[1..]),
        Some("info") => cmd_info(&args[1..]),
        Some("test") => cmd_test(&args[1..]),
        Some("diagram") => cmd_diagram(&args[1..]),
        Some("codegen") => cmd_codegen(&args[1..]),
        Some(first) => {
            eprintln!(
                "error: unknown subcommand '{}'; expected parse, validate, info, test, diagram or codegen\n{}",
                first,
                usage()
            );
            ExitCode::FAILURE
        }
        None => {
            println!("{}", usage());
            ExitCode::FAILURE
        }
    }
}

/// Parse a `--input`/`--output`-style format flag, advancing the index.
fn take_format(args: &[String], i: &mut usize, flag: &str) -> Result<Option<Format>, String> {
    let Some(raw) = args.get(*i + 1) else {
        return Err(format!("{} requires a value ('rfs' or 'json')", flag));
    };
    *i += 1;
    raw.parse::<Format>()
        .map(Some)
        .map_err(|e| format!("{}: {}", flag, e))
}

fn cmd_parse(args: &[String]) -> ExitCode {
    let mut input_format: Option<Format> = None;
    let mut output_format: Option<Format> = None;
    let mut path: Option<String> = None;
    let mut target_dir: Option<String> = None;

    let mut i = 0;
    while i < args.len() {
        let arg = args[i].clone();
        if arg == "--help" || arg == "-h" {
            println!("{}", usage());
            return ExitCode::SUCCESS;
        }
        match arg.as_str() {
            "--" => {}
            "--input" => match take_format(args, &mut i, "--input") {
                Ok(f) => input_format = f,
                Err(e) => {
                    eprintln!("error: {}", e);
                    return ExitCode::FAILURE;
                }
            },
            "--output" => match take_format(args, &mut i, "--output") {
                Ok(f) => output_format = f,
                Err(e) => {
                    eprintln!("error: {}", e);
                    return ExitCode::FAILURE;
                }
            },
            "--target-dir" => match args.get(i + 1).cloned() {
                Some(v) => {
                    i += 1;
                    target_dir = Some(v);
                }
                None => {
                    eprintln!("error: --target-dir requires a directory argument");
                    return ExitCode::FAILURE;
                }
            },
            _ if arg.starts_with('-') => {
                eprintln!("unknown option: {}\n{}", arg, usage());
                return ExitCode::FAILURE;
            }
            _ => {
                if path.is_some() {
                    eprintln!("unexpected extra argument: {}\n{}", arg, usage());
                    return ExitCode::FAILURE;
                }
                path = Some(arg);
            }
        }
        i += 1;
    }

    let (Some(input_format), Some(output_format), Some(path)) = (input_format, output_format, path)
    else {
        eprintln!(
            "error: rfs parse requires --input <rfs|json>, --output <rfs|json> and a file or directory\n{}",
            usage()
        );
        return ExitCode::FAILURE;
    };

    let (written, errors) = frontend::run_conversion(
        input_format,
        output_format,
        Path::new(&path),
        target_dir.as_ref().map(Path::new),
    );
    for e in &errors {
        eprintln!("error: {}", e);
    }
    if !errors.is_empty() {
        return ExitCode::FAILURE;
    }
    for out in &written {
        println!("wrote {}", out.display());
    }
    println!(
        "converted {} file(s): {} -> {}",
        written.len(),
        input_format.name(),
        output_format.name()
    );
    ExitCode::SUCCESS
}

fn cmd_validate(args: &[String]) -> ExitCode {
    let mut input_format: Option<Format> = None;
    let mut path: Option<String> = None;

    let mut i = 0;
    while i < args.len() {
        let arg = args[i].clone();
        if arg == "--help" || arg == "-h" {
            println!("{}", usage());
            return ExitCode::SUCCESS;
        }
        match arg.as_str() {
            "--" => {}
            "--input" => match take_format(args, &mut i, "--input") {
                Ok(f) => input_format = f,
                Err(e) => {
                    eprintln!("error: {}", e);
                    return ExitCode::FAILURE;
                }
            },
            _ if arg.starts_with('-') => {
                eprintln!("unknown option: {}\n{}", arg, usage());
                return ExitCode::FAILURE;
            }
            _ => {
                if path.is_some() {
                    eprintln!("unexpected extra argument: {}\n{}", arg, usage());
                    return ExitCode::FAILURE;
                }
                path = Some(arg);
            }
        }
        i += 1;
    }

    let Some(path) = path else {
        eprintln!(
            "error: rfs validate requires a file or directory\n{}",
            usage()
        );
        return ExitCode::FAILURE;
    };

    match frontend::run_validate(input_format, Path::new(&path)) {
        Err(misuse) => {
            for m in &misuse {
                eprintln!("error: {}", m);
            }
            ExitCode::FAILURE
        }
        Ok(result) => {
            let mut failed = 0usize;
            for f in &result.ok_files {
                println!("valid {}", f.display());
            }
            for (f, warns) in &result.warnings {
                for msg in warns {
                    eprintln!("warning: {}: {}", f.display(), msg);
                }
            }
            for ff in &result.failed {
                failed += 1;
                if let Some(err_path) = &ff.err_file {
                    println!(
                        "invalid {} ({} error(s), written to {})",
                        ff.path.display(),
                        ff.errors.len(),
                        err_path.display()
                    );
                } else {
                    // File mode, or the err file could not be written:
                    // report on stderr.
                    for msg in &ff.errors {
                        eprintln!("{}: {}", ff.path.display(), msg);
                    }
                }
            }
            for w in &result.write_errors {
                eprintln!("error: {}", w);
            }
            if failed > 0 || !result.write_errors.is_empty() {
                ExitCode::FAILURE
            } else {
                ExitCode::SUCCESS
            }
        }
    }
}

fn cmd_codegen(args: &[String]) -> ExitCode {
    match codegen::parse_codegen_args(args) {
        Err(errors) => {
            for e in &errors {
                eprintln!("error: {}", e);
            }
            ExitCode::FAILURE
        }
        Ok(codegen::CodegenRequest::List) => {
            println!("Available codegen backends:");
            for (name, desc) in codegen::list_backends() {
                println!("  {:<10} {}", name, desc);
            }
            ExitCode::SUCCESS
        }
        Ok(codegen::CodegenRequest::Help) => {
            println!("{}", codegen::common_help());
            ExitCode::SUCCESS
        }
        Ok(codegen::CodegenRequest::BackendHelp(name)) => match codegen::get_backend(&name) {
            Some(b) => {
                println!("{}", codegen::backend_help(b));
                ExitCode::SUCCESS
            }
            None => {
                eprintln!(
                    "error: unknown backend '{}'; see `rfs codegen --list`",
                    name
                );
                ExitCode::FAILURE
            }
        },
        Ok(codegen::CodegenRequest::Generate {
            backend_name,
            command,
            input,
            out,
            assumptions,
            steps,
            no_std,
            with_property_monitors,
            mc_mode,
            mc_max_states,
            mc_nondet,
            mc_timeout_sec,
            mc_threads,
            mc_smt_bound,
        }) => {
            let Some(backend) = codegen::get_backend(&backend_name) else {
                eprintln!(
                    "error: unknown backend '{}'; see `rfs codegen --list`",
                    backend_name
                );
                return ExitCode::FAILURE;
            };
            let ext = Path::new(&input)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("");
            let Some(fmt) = Format::from_extension(ext) else {
                eprintln!("error: input '{}' must have extension .rfs or .json", input);
                return ExitCode::FAILURE;
            };
            let ir = match frontend::convert_file_to_ir(Path::new(&input), fmt) {
                Ok(ir) => ir,
                Err(e) => {
                    eprintln!("error: {}", e);
                    return ExitCode::FAILURE;
                }
            };
            if ir.modules.is_empty() {
                eprintln!("error: no modules in {}", input);
                return ExitCode::FAILURE;
            }
            let options = codegen::BackendOptions {
                assumptions,
                steps,
                no_std,
                with_property_monitors,
                mc_mode,
                mc_max_states,
                mc_nondet,
                mc_timeout_sec,
                mc_threads,
                mc_smt_bound,
            };
            match command.run(backend, &ir.modules[0], &options) {
                Err(errors) => {
                    for e in &errors {
                        eprintln!("codegen error: {}", e);
                    }
                    ExitCode::FAILURE
                }
                Ok(result) => {
                    for w in &result.warnings {
                        eprintln!("warning: {}", w);
                    }
                    match out {
                        Some(path) => {
                            if let Some(parent) = Path::new(&path).parent() {
                                let _ = std::fs::create_dir_all(parent);
                            }
                            if std::fs::write(&path, &result.code).is_err() {
                                return ExitCode::FAILURE;
                            }
                            println!("wrote {}", path);
                            // Additional files (e.g. the rust `test` command's
                            // fuzz_target.rs) are written next to --out.
                            let dir = Path::new(&path).parent().unwrap_or_else(|| Path::new("."));
                            for extra in &result.extra_files {
                                let epath = dir.join(&extra.name);
                                if std::fs::write(&epath, &extra.content).is_err() {
                                    return ExitCode::FAILURE;
                                }
                                println!("wrote {}", epath.display());
                            }
                        }
                        None => {
                            // Strict: a command that emits additional files
                            // cannot run in stdout mode (the extra files would
                            // be lost) - no graceful degradation.
                            if !result.extra_files.is_empty() {
                                let names = result
                                    .extra_files
                                    .iter()
                                    .map(|e| e.name.as_str())
                                    .collect::<Vec<_>>()
                                    .join(", ");
                                eprintln!(
                                    "error: this command emits additional files ({names}) that cannot be written to stdout; use --out FILE (they are written next to it)"
                                );
                                return ExitCode::FAILURE;
                            }
                            print!("{}", result.code);
                        }
                    }
                    ExitCode::SUCCESS
                }
            }
        }
    }
}

fn cmd_info(args: &[String]) -> ExitCode {
    use rfs::cli_log::{LogLevel, Logger};
    let mut json = false;
    let mut log_level = LogLevel::Info;
    let mut path: Option<String> = None;

    let mut i = 0;
    while i < args.len() {
        let arg = args[i].clone();
        if arg == "--help" || arg == "-h" {
            println!("{}", usage());
            return ExitCode::SUCCESS;
        }
        match arg.as_str() {
            "--" => {}
            "--json" => json = true,
            "--log-level" => {
                let Some(v) = args.get(i + 1).cloned() else {
                    eprintln!(
                        "error: --log-level requires a value (off|error|warn|info|debug)\n{}",
                        usage()
                    );
                    return ExitCode::FAILURE;
                };
                i += 1;
                match LogLevel::parse(&v) {
                    Some(l) => log_level = l,
                    None => {
                        eprintln!(
                            "error: --log-level expects off|error|warn|info|debug (got '{}')\n{}",
                            v,
                            usage()
                        );
                        return ExitCode::FAILURE;
                    }
                }
            }
            _ if arg.starts_with('-') => {
                eprintln!("unknown option: {}\n{}", arg, usage());
                return ExitCode::FAILURE;
            }
            _ => {
                if path.is_some() {
                    eprintln!("unexpected extra argument: {}\n{}", arg, usage());
                    return ExitCode::FAILURE;
                }
                path = Some(arg);
            }
        }
        i += 1;
    }

    let Some(path) = path else {
        eprintln!("error: rfs info requires a spec file\n{}", usage());
        return ExitCode::FAILURE;
    };

    // Fatal diagnostics bypass the logger (always stderr, drive exit code);
    // the logger carries informational progress output only.
    let logger = Logger::new(log_level);
    let opts = rfs::info::InfoOptions { json };
    match rfs::info::run_info(Path::new(&path), &opts) {
        Ok(out) => {
            logger.debug(&format!(
                "info: rendered {} byte(s) for {}",
                out.len(),
                path
            ));
            println!("{out}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

/// `rfs test`: generate + compile + run the extended test suite for a spec in
/// one invocation (Phase 5 task 7). Thin wrapper over `rfs::test_cmd`.
fn cmd_test(args: &[String]) -> ExitCode {
    use rfs::cli_log::{LogLevel, Logger};
    let mut opts = rfs::test_cmd::TestOptions::default();
    let mut log_level = LogLevel::Info;
    let mut path: Option<String> = None;

    let mut i = 0;
    while i < args.len() {
        let arg = args[i].clone();
        if arg == "--help" || arg == "-h" {
            println!("{}", usage());
            return ExitCode::SUCCESS;
        }
        match arg.as_str() {
            "--" => {}
            "--backend" | "--steps" | "--rustc" | "--gcc" | "--rustc-args" | "--gcc-args"
            | "--log-level" => {
                let Some(v) = args.get(i + 1).cloned() else {
                    eprintln!("error: {} requires a value\n{}", arg, usage());
                    return ExitCode::FAILURE;
                };
                i += 1;
                match arg.as_str() {
                    "--backend" => opts.backend = v,
                    "--steps" => match v.parse::<usize>() {
                        Ok(n) => opts.steps = n,
                        Err(_) => {
                            eprintln!(
                                "error: --steps expects a non-negative integer (got '{}')\n{}",
                                v,
                                usage()
                            );
                            return ExitCode::FAILURE;
                        }
                    },
                    "--rustc" => opts.rustc_path = Some(v),
                    "--gcc" => opts.gcc_path = Some(v),
                    "--rustc-args" => opts.rustc_args = Some(v),
                    "--gcc-args" => opts.gcc_args = Some(v),
                    "--log-level" => match LogLevel::parse(&v) {
                        Some(l) => log_level = l,
                        None => {
                            eprintln!(
                                "error: --log-level expects off|error|warn|info|debug (got '{}')\n{}",
                                v,
                                usage()
                            );
                            return ExitCode::FAILURE;
                        }
                    },
                    _ => unreachable!("flag checked by caller: {arg}"),
                }
            }
            "--print-script" => opts.print_script = true,
            _ if arg.starts_with('-') => {
                eprintln!("unknown option: {}\n{}", arg, usage());
                return ExitCode::FAILURE;
            }
            _ => {
                if path.is_some() {
                    eprintln!("unexpected extra argument: {}\n{}", arg, usage());
                    return ExitCode::FAILURE;
                }
                path = Some(arg);
            }
        }
        i += 1;
    }

    let Some(path) = path else {
        eprintln!("error: rfs test requires a spec file\n{}", usage());
        return ExitCode::FAILURE;
    };

    // Fatal diagnostics bypass the logger (always stderr, drive exit code);
    // the logger carries informational progress output only.
    let logger = Logger::new(log_level);
    match rfs::test_cmd::run_test(Path::new(&path), &opts, &logger) {
        Ok(result) => {
            if let Some(script) = result.script {
                // `print!` (not println): the script already ends with a
                // newline and callers may redirect stdout to a file - the
                // bytes must be reproduced exactly (codegen convention).
                print!("{script}");
            } else {
                logger.info(&result.note);
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn cmd_diagram(args: &[String]) -> ExitCode {
    use rfs::cli_log::{LogLevel, Logger};
    let mut opts = rfs::diagram::DiagramOptions::default();
    let mut log_level = LogLevel::Info;
    let mut path: Option<String> = None;

    let mut i = 0;
    while i < args.len() {
        let arg = args[i].clone();
        if arg == "--help" || arg == "-h" {
            println!("{}", usage());
            return ExitCode::SUCCESS;
        }
        match arg.as_str() {
            "--" => {}
            "--list" => {
                for (name, desc) in rfs::diagram::list_diagrams() {
                    println!("{name:<6} {desc}");
                }
                return ExitCode::SUCCESS;
            }
            "--format" => {
                let Some(v) = args.get(i + 1).cloned() else {
                    eprintln!(
                        "error: --format requires a value ({} )\n{}",
                        rfs::diagram::list_diagrams()
                            .iter()
                            .map(|(n, _)| *n)
                            .collect::<Vec<_>>()
                            .join("|"),
                        usage()
                    );
                    return ExitCode::FAILURE;
                };
                i += 1;
                opts.format = v;
            }
            "--out" => {
                let Some(v) = args.get(i + 1).cloned() else {
                    eprintln!("error: --out requires a value (FILE)\n{}", usage());
                    return ExitCode::FAILURE;
                };
                i += 1;
                opts.out = Some(std::path::PathBuf::from(v));
            }
            "--render" => {
                let Some(v) = args.get(i + 1).cloned() else {
                    eprintln!("error: --render requires a value (svg)\n{}", usage());
                    return ExitCode::FAILURE;
                };
                i += 1;
                if v != "svg" {
                    eprintln!("error: --render expects svg (got '{}')\n{}", v, usage());
                    return ExitCode::FAILURE;
                }
                opts.render_svg = true;
            }
            "--print-script" => opts.print_script = true,
            "--log-level" => {
                let Some(v) = args.get(i + 1).cloned() else {
                    eprintln!(
                        "error: --log-level requires a value (off|error|warn|info|debug)\n{}",
                        usage()
                    );
                    return ExitCode::FAILURE;
                };
                i += 1;
                match LogLevel::parse(&v) {
                    Some(l) => log_level = l,
                    None => {
                        eprintln!(
                            "error: --log-level expects off|error|warn|info|debug (got '{}')\n{}",
                            v,
                            usage()
                        );
                        return ExitCode::FAILURE;
                    }
                }
            }
            "--dot" | "--plantuml" | "--mermaid" => {
                let Some(v) = args.get(i + 1).cloned() else {
                    eprintln!("error: {} requires a value (PATH)\n{}", arg, usage());
                    return ExitCode::FAILURE;
                };
                i += 1;
                tool_override_mut(&mut opts, &arg).path = Some(v);
            }
            "--dot-args" | "--plantuml-args" | "--mermaid-args" => {
                let Some(v) = args.get(i + 1).cloned() else {
                    eprintln!("error: {} requires a value (ARGS)\n{}", arg, usage());
                    return ExitCode::FAILURE;
                };
                i += 1;
                tool_override_mut(&mut opts, &arg).args = Some(v);
            }
            _ if arg.starts_with('-') => {
                eprintln!("unknown option: {}\n{}", arg, usage());
                return ExitCode::FAILURE;
            }
            _ => {
                if path.is_some() {
                    eprintln!("unexpected extra argument: {}\n{}", arg, usage());
                    return ExitCode::FAILURE;
                }
                path = Some(arg);
            }
        }
        i += 1;
    }

    let Some(path) = path else {
        eprintln!("error: rfs diagram requires a spec file\n{}", usage());
        return ExitCode::FAILURE;
    };

    // Fatal diagnostics bypass the logger (always stderr, drive exit code);
    // the logger carries informational progress output only.
    let logger = Logger::new(log_level);
    match rfs::diagram::run_diagram(std::path::Path::new(&path), &opts) {
        Ok(result) => {
            logger.debug(&result.note);
            if let Some(text) = result.stdout {
                // `print!` (not println): the generated text already ends with
                // a newline, and callers may redirect stdout to a file - the
                // bytes must be reproduced exactly (codegen convention).
                print!("{text}");
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

/// Route a tool flag (`--dot`/`--plantuml`/`--mermaid`, with or without the
/// `-args` suffix) to its override slot.
fn tool_override_mut<'a>(
    opts: &'a mut rfs::diagram::DiagramOptions,
    flag: &str,
) -> &'a mut rfs::diagram::ToolOverride {
    match flag.strip_suffix("-args").unwrap_or(flag) {
        "--dot" => &mut opts.dot,
        "--plantuml" => &mut opts.plantuml,
        "--mermaid" => &mut opts.mermaid,
        _ => unreachable!("flag checked by caller: {flag}"),
    }
}
