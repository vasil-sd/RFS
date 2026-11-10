//! RFS CLI entry point: pure command dispatcher. Every top-level
//! `rfs <command>` is a plugin registered in `crate::cli::commands::ALL_COMMANDS`;
//! this file only resolves the command name and hands off to its `run_cli`.
//! Common argument processing (usage text, flag takers, error conventions) lives
//! in `crate::cli::args`.
//! Docs: dev:code-map-cli, dev:cli-commands-dispatch

use std::env;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some(name) => match rfs::cli::commands::find(name) {
            Some(cmd) => (cmd.run)(&args[1..]),
            None => {
                eprintln!(
                    "error: unknown subcommand '{}'; expected parse, validate, info, test, diagram, codegen, mc-ref or gen-spec\n{}",
                    name,
                    rfs::cli::args::usage()
                );
                ExitCode::FAILURE
            }
        },
        None => {
            println!("{}", rfs::cli::args::usage());
            ExitCode::FAILURE
        }
    }
}
