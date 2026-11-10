//! `rfs parse` command plugin: RFS/JSON format conversion (moved from `main.rs`
//! `cmd_parse`). Calls the shared conversion engine in `crate::cli::utils`.
//! Docs: dev:cli-commands-overview

use std::path::Path;
use std::process::ExitCode;

use crate::cli::args::{take_format, usage};
use crate::cli::utils::{run_conversion, Format};

pub fn run_cli(args: &[String]) -> ExitCode {
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

    let (written, errors) = run_conversion(
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
