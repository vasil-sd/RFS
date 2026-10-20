use std::env;
use std::path::Path;
use std::process::ExitCode;

use rfs::{codegen, frontend, Format};

fn usage() -> String {
    "RFS toolchain\n\nusage:\n  rfs parse --input <rfs|json> --output <rfs|json> <file-or-dir> [--target-dir DIR]\n  rfs validate [--input <rfs|json>] <file-or-dir>\n  rfs info [--json] [--log-level off|error|warn|info|debug] <spec.rfs|spec.json>\n  rfs codegen --list | --help | <backend> --help\n  rfs codegen <backend> <production|test|mc> <spec.rfs|spec.json> [--out FILE] [--assumes ...] [--steps N]\n\nparse: load RFS or IR-JSON files, convert through the IR, and write formatted\n       output. With a directory input, only files matching --input are scanned;\n       without --target-dir each output is written next to its input file, with\n       --target-dir the input directory structure is preserved inside it. The\n       original extension is replaced (never stacked). Conversions whose output\n       would overwrite the input file are refused.\n\nvalidate: parse + semantically validate RFS or IR-JSON files. For a file input\n         the extension selects the format and errors go to stderr; for a\n         directory input --input is required and each failing source file gets\n         a <name>.rfs-validation.err file next to it.\n\ninfo: static spec introspection - per module: version, initial state, enums,
      events/outputs with variants + field types, variables with types + init,
      transition count, and properties/assumptions each with monitor-subset
      status (monitored / skipped: reason). No model checking. `--json` emits
      a machine-readable document (stable key order, all keys present); 
      `--log-level` filters stderr progress output.

codegen: target-language plugin registry. `--list` shows registered backends;\n         `<backend> --help` lists that backend's commands. Each command runs on\n         the first module of the spec and writes generated source to --out or\n         stdout. See CODEGEN.md for the backend plugin API.\n\nexamples:\n  cargo run -- parse --input rfs --output json specs/handshake.rfs\n  cargo run -- parse --input rfs --output rfs specs/ --target-dir out/\n  cargo run -- validate specs/handshake.rfs\n  cargo run -- validate --input rfs specs/\n".to_string()
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("parse") => cmd_parse(&args[1..]),
        Some("validate") => cmd_validate(&args[1..]),
        Some("info") => cmd_info(&args[1..]),
        Some("codegen") => cmd_codegen(&args[1..]),
        Some(first) => {
            eprintln!(
                "error: unknown subcommand '{}'; expected parse, validate, info or codegen\n{}",
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
                        }
                        None => print!("{}", result.code),
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
