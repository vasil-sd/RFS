//! `rfs mc-ref` command logic (Phase 6 task 3, action 5 / D-P6-10).
//!
//! Runs the reference model checker (`crate::ref_mc`) in-process - no
//! compilation: rapid prototyping in shell/REPL. The pipeline is exposed as a
//! lib function ([`run_mc_ref`]) so integration tests can call it without
//! spawning the binary; [`run_cli`] is the thin argument-parsing shell for
//! `main.rs`.
//!
//! Flag surface mirrors the generated exact-mode binaries where applicable:
//! `--max-states N`, `--timeout-sec N`, `--assumes all|none|labels` (same wire
//! format as `codegen_mc::parse_assume_selection`),
//! `--counterexample-format json|text` (default text), `--log-level`.
//! NOT applicable (documented in the usage text): `--seed`/`--steps`/
//! `--violate`/unexpected-injection (fuzz-mode concepts; exact search is
//! seedless, D-P4-7) and `--threads`/`--fast` (generated-engine tuning; ref_mc
//! is single-threaded). `--nondet`: ref_mc explores all enabled transitions by
//! construction - its liveness semantics match the generated engine's DEFAULT
//! exact mode.
//!
//! Exit codes MIRROR the generated binaries: 0 pass-complete, 1 fail,
//! 2 unknown-state-limit; usage/parse/validation errors exit 1 (the existing
//! command convention).

use std::path::Path;
use std::process::ExitCode;

use crate::frontend::{self, Format};
use crate::ref_mc::{self, RefConfig};

/// Counterexample report format on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CeFormat {
    Text,
    Json,
}

impl CeFormat {
    /// Parse the `--counterexample-format` value (case-sensitive).
    pub fn parse(s: &str) -> Option<CeFormat> {
        match s {
            "text" => Some(CeFormat::Text),
            "json" => Some(CeFormat::Json),
            _ => None,
        }
    }
}

/// Options for one `mc-ref` run (parsed flag values; `None` = engine default).
#[derive(Debug, Clone)]
pub struct McRefOptions {
    /// `--max-states N`; `None` = `codegen_mc::EXACT_DEFAULT_MAX_STATES`.
    pub max_states: Option<usize>,
    /// `--timeout-sec N` (wall clock, 0 = off); `None` = engine default.
    pub timeout_sec: Option<u64>,
    /// `--assumes` wire string: `all`, `none`, or comma-separated labels.
    pub assumes: String,
    /// `--counterexample-format`: text (default) or json.
    pub ce_format: CeFormat,
}

impl Default for McRefOptions {
    fn default() -> Self {
        Self {
            max_states: None,
            timeout_sec: None,
            assumes: "all".to_string(),
            ce_format: CeFormat::Text,
        }
    }
}

/// The full pipeline: load (extension selects the format) -> validate ->
/// `ref_mc::run` on the FIRST module (the command convention) -> render the
/// unified report. Returns the exit code mirroring the generated binaries and
/// the exact stdout text (text mode ends with a newline; JSON mode is the
/// canonical compact document + one trailing newline, as the generated
/// binaries print it).
pub fn run_mc_ref(path: &Path, opts: &McRefOptions) -> Result<(ExitCode, String), String> {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    let fmt = Format::from_extension(ext).ok_or_else(|| {
        format!(
            "input '{}' must have extension .rfs or .json",
            path.display()
        )
    })?;
    let ir = frontend::convert_file_to_ir(path, fmt)?;
    if ir.modules.is_empty() {
        return Err(format!("no modules in {}", path.display()));
    }
    let module = &ir.modules[0];
    let errors = crate::validator::validate_module(module);
    if !errors.is_empty() {
        return Err(format!(
            "{}: {} validation error(s): {}",
            path.display(),
            errors.len(),
            errors
                .iter()
                .map(|e| e.message.clone())
                .collect::<Vec<_>>()
                .join("; ")
        ));
    }
    let cfg = RefConfig {
        max_states: opts
            .max_states
            .unwrap_or(crate::codegen_mc::EXACT_DEFAULT_MAX_STATES),
        timeout_sec: opts
            .timeout_sec
            .unwrap_or(crate::codegen_mc::EXACT_DEFAULT_TIMEOUT_SEC),
        assume_arg: opts.assumes.clone(),
    };
    let res = ref_mc::run(module, &cfg)?;
    let out = match opts.ce_format {
        CeFormat::Text => ref_mc::render_text(&res, module),
        // Wire contract (backend_parity): one compact line + trailing newline.
        CeFormat::Json => format!(
            "{}\n",
            ref_mc::to_json_report(&res, module).to_json_string()
        ),
    };
    Ok((exit_code_for(res.status), out))
}

/// Exit codes mirroring the generated exact-mode binaries.
pub fn exit_code_for(status: crate::codegen_mc::ExactStatus) -> ExitCode {
    match status {
        crate::codegen_mc::ExactStatus::PassComplete => ExitCode::SUCCESS,
        crate::codegen_mc::ExactStatus::Fail => ExitCode::FAILURE,
        crate::codegen_mc::ExactStatus::UnknownStateLimit => ExitCode::from(2),
    }
}

fn usage() -> String {
    "usage: rfs mc-ref <spec.rfs|spec.json> [--max-states N] [--timeout-sec N]\n       [--assumes all|none|labels] [--counterexample-format json|text]\n       [--log-level off|error|warn|info|debug]\n\nRuns the reference model checker in-process (no compilation). Exit codes\nmirror the generated exact-mode binaries: 0 pass-complete, 1 fail,\n2 unknown-state-limit.\n\nNot applicable (and why):\n  --seed/--steps/--violate/--unexpected-prob: fuzz-mode concepts; exact\n    search is seedless (D-P4-7).\n  --threads/--fast: generated-engine tuning; ref_mc is single-threaded.\n  --nondet: ref_mc explores all enabled transitions by construction - its\n    liveness semantics match the generated engine's DEFAULT exact mode."
        .to_string()
}

/// `rfs mc-ref` argument parsing + pipeline (thin shell for main.rs).
pub fn run_cli(args: &[String]) -> ExitCode {
    use crate::cli_log::{LogLevel, Logger};
    let mut opts = McRefOptions::default();
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
            "--max-states" => {
                match take_value(args, &mut i, &arg) {
                    Ok(v) => {
                        match v.parse::<usize>() {
                            Ok(n) => opts.max_states = Some(n),
                            Err(_) => {
                                eprintln!("error: --max-states expects a non-negative integer (got '{}')\n{}", v, usage());
                                return ExitCode::FAILURE;
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("error: {}\n{}", e, usage());
                        return ExitCode::FAILURE;
                    }
                }
            }
            "--timeout-sec" => {
                match take_value(args, &mut i, &arg) {
                    Ok(v) => match v.parse::<u64>() {
                        Ok(n) => opts.timeout_sec = Some(n),
                        Err(_) => {
                            eprintln!("error: --timeout-sec expects a non-negative integer (got '{}')\n{}", v, usage());
                            return ExitCode::FAILURE;
                        }
                    },
                    Err(e) => {
                        eprintln!("error: {}\n{}", e, usage());
                        return ExitCode::FAILURE;
                    }
                }
            }
            "--assumes" => match take_value(args, &mut i, &arg) {
                Ok(v) => opts.assumes = v,
                Err(e) => {
                    eprintln!("error: {}\n{}", e, usage());
                    return ExitCode::FAILURE;
                }
            },
            "--counterexample-format" => {
                match take_value(args, &mut i, &arg) {
                    Ok(v) => match CeFormat::parse(&v) {
                        Some(f) => opts.ce_format = f,
                        None => {
                            eprintln!("error: --counterexample-format expects json or text (got '{}')\n{}", v, usage());
                            return ExitCode::FAILURE;
                        }
                    },
                    Err(e) => {
                        eprintln!("error: {}\n{}", e, usage());
                        return ExitCode::FAILURE;
                    }
                }
            }
            "--log-level" => {
                match take_value(args, &mut i, &arg) {
                    Ok(v) => match LogLevel::parse(&v) {
                        Some(l) => log_level = l,
                        None => {
                            eprintln!("error: --log-level expects off|error|warn|info|debug (got '{}')\n{}", v, usage());
                            return ExitCode::FAILURE;
                        }
                    },
                    Err(e) => {
                        eprintln!("error: {}\n{}", e, usage());
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
        eprintln!("error: rfs mc-ref requires a spec file\n{}", usage());
        return ExitCode::FAILURE;
    };

    // Fatal diagnostics bypass the logger (always stderr, drive exit code).
    let logger = Logger::new(log_level);
    match run_mc_ref(Path::new(&path), &opts) {
        Ok((code, out)) => {
            logger.debug(&format!(
                "mc-ref: {} byte(s) of report for {}",
                out.len(),
                path
            ));
            // `print!` (not println): the rendered report already ends with a
            // newline in both formats.
            print!("{out}");
            code
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

/// Take the value following `flag`, advancing the index; error if missing.
fn take_value(args: &[String], i: &mut usize, flag: &str) -> Result<String, String> {
    let Some(raw) = args.get(*i + 1).cloned() else {
        return Err(format!("{} requires a value", flag));
    };
    *i += 1;
    Ok(raw)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ce_format_parse_pinned() {
        assert_eq!(CeFormat::parse("text"), Some(CeFormat::Text));
        assert_eq!(CeFormat::parse("json"), Some(CeFormat::Json));
        assert_eq!(CeFormat::parse("JSON"), None);
        assert_eq!(CeFormat::parse(""), None);
    }

    #[test]
    fn exit_codes_mirror_generated_binaries() {
        use crate::codegen_mc::ExactStatus;
        assert_eq!(exit_code_for(ExactStatus::PassComplete), ExitCode::SUCCESS);
        assert_eq!(exit_code_for(ExactStatus::Fail), ExitCode::FAILURE);
        assert_eq!(
            exit_code_for(ExactStatus::UnknownStateLimit),
            ExitCode::from(2)
        );
    }

    #[test]
    fn run_mc_ref_pass_spec_exits_zero_with_summary_line() {
        let dir = std::env::temp_dir().join("rfs_mc_ref_cmd_pass");
        std::fs::create_dir_all(&dir).unwrap();
        let spec = dir.join("pass.rfs");
        std::fs::write(
            &spec,
            "module m { enum State { A, B } initial A event Go transition A on Go { next B; } transition B on Go { next A; } }",
        )
        .unwrap();
        let (code, out) = run_mc_ref(&spec, &McRefOptions::default()).expect("pipeline");
        assert_eq!(code, ExitCode::SUCCESS);
        assert!(out.starts_with("PASS (complete)\n"), "{out}");
        assert!(
            out.contains("model-check summary: status=pass-complete "),
            "{out}"
        );
    }

    #[test]
    fn run_mc_ref_json_mode_is_compact_line_plus_newline() {
        let dir = std::env::temp_dir().join("rfs_mc_ref_cmd_json");
        std::fs::create_dir_all(&dir).unwrap();
        let spec = dir.join("pass.rfs");
        std::fs::write(
            &spec,
            "module m { enum State { A, B } initial A event Go transition A on Go { next B; } transition B on Go { next A; } }",
        )
        .unwrap();
        let opts = McRefOptions {
            ce_format: CeFormat::Json,
            ..Default::default()
        };
        let (code, out) = run_mc_ref(&spec, &opts).expect("pipeline");
        assert_eq!(code, ExitCode::SUCCESS);
        assert!(out.ends_with('\n') && !out.ends_with("\n\n"), "{out:?}");
        let doc = out.strip_suffix('\n').unwrap();
        // Strict-parseable + canonical compact form (the wire contract).
        let parsed = crate::json_rw::parse(doc).expect("strict parse");
        assert_eq!(parsed.to_string_compact(), doc);
    }

    #[test]
    fn run_mc_ref_max_states_truncation_exits_two() {
        // A 256-cycle (bit<8> counter): --max-states 10 must truncate.
        let dir = std::env::temp_dir().join("rfs_mc_ref_cmd_counter");
        std::fs::create_dir_all(&dir).unwrap();
        let spec = dir.join("counter.rfs");
        std::fs::write(
            &spec,
            "module m { enum State { S } initial S event Tick var n: bit<8> init 0 transition S on Tick { next S; set n = n + 1; } }",
        )
        .unwrap();
        let opts = McRefOptions {
            max_states: Some(10),
            ..Default::default()
        };
        let (code, out) = run_mc_ref(&spec, &opts).expect("pipeline");
        assert_eq!(code, ExitCode::from(2));
        assert!(out.contains("status=unknown-state-limit"), "{out}");
    }

    #[test]
    fn run_mc_ref_validation_error_is_diagnostic_not_panic() {
        let dir = std::env::temp_dir().join("rfs_mc_ref_cmd_bad");
        std::fs::create_dir_all(&dir).unwrap();
        let spec = dir.join("bad.rfs");
        // Transition to an undeclared state: parses, fails validation.
        std::fs::write(
            &spec,
            "module m { enum State { A } initial A event Go transition A on Go { next Z; } }",
        )
        .unwrap();
        let err = run_mc_ref(&spec, &McRefOptions::default()).expect_err("must fail");
        assert!(err.contains("validation error"), "{err}");
    }

    #[test]
    fn run_cli_usage_errors_exit_one() {
        // Missing spec file.
        assert_eq!(run_cli(&[]), ExitCode::FAILURE);
        // Unknown flag.
        let bad = vec!["--nope".to_string(), "x.rfs".to_string()];
        assert_eq!(run_cli(&bad), ExitCode::FAILURE);
        // Bad --max-states value.
        let bad = vec![
            "--max-states".to_string(),
            "xyz".to_string(),
            "x.rfs".to_string(),
        ];
        assert_eq!(run_cli(&bad), ExitCode::FAILURE);
    }
}
