//! `rfs codegen` command plugin (moved from `main.rs` `cmd_codegen`). Thin CLI
//! wrapper over the target-language backend registry in `crate::codegen`.
//! Docs: dev:cli-commands-overview

use std::path::Path;
use std::process::ExitCode;

use crate::cli::utils::{convert_file_to_ir, Format};
use crate::codegen;

pub fn run_cli(args: &[String]) -> ExitCode {
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
            counterexample_format,
            with_protocol_fsm,
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
            let ir = match convert_file_to_ir(Path::new(&input), fmt) {
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
                counterexample_format,
                with_protocol_fsm,
                // Stamped from the whole-file canonical IR JSON by `run`
                // (Phase 6, D-P6-3).
                spec_hash: String::new(),
            };
            match command.run(backend, &ir, &options) {
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
