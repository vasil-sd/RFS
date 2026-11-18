//! `rfs mc-ref` command logic (Phase 6 task 3, action 5 / D-P6-10).
//!
//! Runs the reference model checker (`crate::mc::ref_mc`) in-process - no
//! compilation: rapid prototyping in shell/REPL. The pipeline is exposed as a
//! lib function ([`run_mc_ref`]) so integration tests can call it without
//! spawning the binary; [`run_cli`] is the thin argument-parsing shell for
//! `main.rs`.
//!
//! Flag surface mirrors the generated binaries per mode (D-P16-4):
//! `--mode exact|fuzz` (default exact) selects the engine. EXACT mode:
//! `--max-states N`, `--timeout-sec N`, `--assumes all|none|labels` (same wire
//! format as `mc_concepts::parse_assume_selection`),
//! `--counterexample-format json|text` (default text), `--nondet`, `--log-level`.
//! `--nondet` fires EVERY enabled transition as its own successor (the
//! generated engines' nondet mode); liveness then requires strongly-fair bad
//! cycles (D-P4-11). FUZZ mode: `--bound N` (5), `--runs N` (20), `--seed N`
//! (12345), `--assumes`, `--violate label:prob:percent%,...`,
//! `--unexpected-prob P` (0.0) - the generated fuzz runtimes' flag surface,
//! driving the seeded engine in `ref_mc_fuzz.rs`. Exact-only flags
//! (`--max-states`/`--timeout-sec`/`--counterexample-format`/`--nondet`) are a
//! usage error under `--mode fuzz`. NOT applicable (documented in the usage
//! text): `--threads`/`--fast` (generated-engine tuning; ref_mc is
//! single-threaded).
//!
//! Exit codes MIRROR the generated binaries per mode: exact 0 pass-complete,
//! 1 fail, 2 unknown-state-limit; fuzz 0 no property violation (summary line),
//! 1 violation found (per-run report, NO summary line). Usage/parse/validation
//! errors exit 1 (the existing command convention).
//! Docs: dev:cli-commands-overview, dev:cli-commands-mc-ref-gen-spec, llm:cli-commands

use std::path::Path;
use std::process::ExitCode;

use crate::cli::utils::{self, Format};
use crate::codegen::mc_concepts::ViolationSpec;
use crate::mc::ref_mc::{self, FuzzConfig, RefConfig};
use crate::mc::ref_mc_fuzz::FuzzReport;

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

/// The model-checking mode (`--mode`): exact (default) or seeded fuzz.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McMode {
    Exact,
    Fuzz,
}

impl McMode {
    /// Parse the `--mode` value (case-sensitive).
    pub fn parse(s: &str) -> Option<McMode> {
        match s {
            "exact" => Some(McMode::Exact),
            "fuzz" => Some(McMode::Fuzz),
            _ => None,
        }
    }
}

/// Options for one `mc-ref` run (parsed flag values; `None` = engine default).
#[derive(Debug, Clone)]
pub struct McRefOptions {
    /// `--mode exact|fuzz` (default exact).
    pub mode: McMode,
    /// `--max-states N`; `None` = `mc_concepts::EXACT_DEFAULT_MAX_STATES`.
    /// EXACT-MODE-ONLY (rejected under `--mode fuzz`).
    pub max_states: Option<usize>,
    /// `--timeout-sec N` (wall clock, 0 = off); `None` = engine default.
    /// EXACT-MODE-ONLY (rejected under `--mode fuzz`).
    pub timeout_sec: Option<u64>,
    /// `--assumes` wire string: `all`, `none`, or comma-separated labels
    /// (shared by both modes).
    pub assumes: String,
    /// `--counterexample-format`: text (default) or json. EXACT-MODE-ONLY.
    pub ce_format: CeFormat,
    /// `--nondet`: fire every enabled transition as its own successor
    /// (EXACT-MODE-ONLY).
    pub nondet: bool,
    /// `--bound N`: fuzz steps per run; `None` = `FUZZ_DEFAULT_BOUND`.
    pub bound: Option<usize>,
    /// `--runs N`: fuzz run count; `None` = `FUZZ_DEFAULT_RUNS`.
    pub runs: Option<usize>,
    /// `--seed N`: fuzz LCG seed; `None` = `FUZZ_DEFAULT_SEED`.
    pub seed: Option<u64>,
    /// `--violate label:prob:percent%,...` parsed directives (fuzz only).
    pub violate: Vec<ViolationSpec>,
    /// `--unexpected-prob P`: fuzz unexpected-injection probability
    /// (fuzz only); `None` = 0.0.
    pub unexpected_prob: Option<f64>,
}

impl Default for McRefOptions {
    fn default() -> Self {
        Self {
            mode: McMode::Exact,
            max_states: None,
            timeout_sec: None,
            assumes: "all".to_string(),
            ce_format: CeFormat::Text,
            nondet: false,
            bound: None,
            runs: None,
            seed: None,
            violate: Vec::new(),
            unexpected_prob: None,
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
    let ir = utils::convert_file_to_ir(path, fmt)?;
    if ir.modules.is_empty() {
        return Err(format!("no modules in {}", path.display()));
    }
    let module = &ir.modules[0];
    let errors = crate::lang::validator::validate_module(module);
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
    // Mode dispatch (D-P16-4): exact is the default and byte-unchanged; fuzz
    // routes to the seeded engine. Exact-only flags are a usage error under
    // `--mode fuzz` (the generated fuzz binary has no such flags).
    if opts.mode == McMode::Fuzz {
        if opts.max_states.is_some()
            || opts.timeout_sec.is_some()
            || opts.ce_format != CeFormat::Text
            || opts.nondet
        {
            return Err(
                "--max-states/--timeout-sec/--counterexample-format/--nondet are exact-mode-only"
                    .to_string(),
            );
        }
        let cfg = FuzzConfig {
            bound: opts
                .bound
                .unwrap_or(crate::codegen::mc_concepts::FUZZ_DEFAULT_BOUND),
            runs: opts
                .runs
                .unwrap_or(crate::codegen::mc_concepts::FUZZ_DEFAULT_RUNS),
            seed: opts
                .seed
                .unwrap_or(crate::codegen::mc_concepts::FUZZ_DEFAULT_SEED),
            assume_arg: opts.assumes.clone(),
            violate: opts.violate.clone(),
            unexpected_prob: opts.unexpected_prob.unwrap_or(0.0),
        };
        let report = ref_mc::run_fuzz(module, &cfg)?;
        // Exit 0 on completion (summary line), 1 on a property violation
        // (per-run report, NO summary line) - mirroring the generated binaries.
        return match report {
            // The summary line carries no trailing newline; the engine prints it
            // with println! (one \n).
            FuzzReport::Completed { summary } => Ok((ExitCode::SUCCESS, format!("{summary}\n"))),
            // The per-run report already ends with exactly one \n (the last trace
            // line); the engine's print_trace does the same - no extra newline.
            FuzzReport::Violated { report } => Ok((ExitCode::FAILURE, report)),
        };
    }
    let cfg = RefConfig {
        max_states: opts
            .max_states
            .unwrap_or(crate::codegen::mc_concepts::EXACT_DEFAULT_MAX_STATES),
        timeout_sec: opts
            .timeout_sec
            .unwrap_or(crate::codegen::mc_concepts::EXACT_DEFAULT_TIMEOUT_SEC),
        assume_arg: opts.assumes.clone(),
        nondet: opts.nondet,
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
pub fn exit_code_for(status: crate::codegen::mc_concepts::ExactStatus) -> ExitCode {
    match status {
        crate::codegen::mc_concepts::ExactStatus::PassComplete => ExitCode::SUCCESS,
        crate::codegen::mc_concepts::ExactStatus::Fail => ExitCode::FAILURE,
        crate::codegen::mc_concepts::ExactStatus::UnknownStateLimit => ExitCode::from(2),
    }
}

fn usage() -> String {
    "usage: rfs mc-ref <spec.rfs|spec.json> [--mode exact|fuzz]\n       [exact: --max-states N --timeout-sec N --nondet --counterexample-format json|text]\n       [fuzz: --bound N --runs N --seed N --violate label:prob:percent%,... --unexpected-prob P]\n       [--assumes all|none|labels] [--log-level off|error|warn|info|debug]\n\nRuns the reference model checker in-process (no compilation).\n\n--mode exact (default): exhaustive exact search. Exit codes mirror the\n  generated exact-mode binaries: 0 pass-complete, 1 fail, 2 unknown-state-\n  limit. --nondet fires EVERY enabled transition as its own successor;\n  liveness then requires strongly-fair bad cycles.\n--mode fuzz: seeded random testing (the generated fuzz runtimes' strategy).\n  Exit codes: 0 no property violation (summary line printed), 1 violation\n  found (per-run report printed, NO summary line). --bound steps per run (5),\n  --runs count (20), --seed LCG seed (12345), --violate forced-violation\n  directives, --unexpected-prob injection probability (0.0).\n\n--assumes is shared by both modes. Exact-only flags (--max-states/\n--timeout-sec/--counterexample-format/--nondet) are a usage error under\n--mode fuzz.\n\nNot applicable (and why):\n  --threads/--fast: generated-engine tuning; ref_mc is single-threaded."
        .to_string()
}

/// `rfs mc-ref` argument parsing + pipeline (thin shell for main.rs).
pub fn run_cli(args: &[String]) -> ExitCode {
    use crate::cli::log::{LogLevel, Logger};
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
            "--nondet" => {
                opts.nondet = true;
            }
            "--mode" => match take_value(args, &mut i, &arg) {
                Ok(v) => match McMode::parse(&v) {
                    Some(m) => opts.mode = m,
                    None => {
                        eprintln!(
                            "error: --mode expects exact or fuzz (got '{}')\n{}",
                            v,
                            usage()
                        );
                        return ExitCode::FAILURE;
                    }
                },
                Err(e) => {
                    eprintln!("error: {}\n{}", e, usage());
                    return ExitCode::FAILURE;
                }
            },
            "--bound" => match take_value(args, &mut i, &arg) {
                Ok(v) => match v.parse::<usize>() {
                    Ok(n) => opts.bound = Some(n),
                    Err(_) => {
                        eprintln!(
                            "error: --bound expects a non-negative integer (got '{}')\n{}",
                            v,
                            usage()
                        );
                        return ExitCode::FAILURE;
                    }
                },
                Err(e) => {
                    eprintln!("error: {}\n{}", e, usage());
                    return ExitCode::FAILURE;
                }
            },
            "--runs" => match take_value(args, &mut i, &arg) {
                Ok(v) => match v.parse::<usize>() {
                    Ok(n) => opts.runs = Some(n),
                    Err(_) => {
                        eprintln!(
                            "error: --runs expects a non-negative integer (got '{}')\n{}",
                            v,
                            usage()
                        );
                        return ExitCode::FAILURE;
                    }
                },
                Err(e) => {
                    eprintln!("error: {}\n{}", e, usage());
                    return ExitCode::FAILURE;
                }
            },
            "--seed" => match take_value(args, &mut i, &arg) {
                Ok(v) => match v.parse::<u64>() {
                    Ok(n) => opts.seed = Some(n),
                    Err(_) => {
                        eprintln!(
                            "error: --seed expects a non-negative integer (got '{}')\n{}",
                            v,
                            usage()
                        );
                        return ExitCode::FAILURE;
                    }
                },
                Err(e) => {
                    eprintln!("error: {}\n{}", e, usage());
                    return ExitCode::FAILURE;
                }
            },
            "--violate" => match take_value(args, &mut i, &arg) {
                Ok(v) => opts.violate.extend(
                    v.split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .map(|s| ViolationSpec::parse(&s)),
                ),
                Err(e) => {
                    eprintln!("error: {}\n{}", e, usage());
                    return ExitCode::FAILURE;
                }
            },
            "--unexpected-prob" => match take_value(args, &mut i, &arg) {
                Ok(v) => match v.parse::<f64>() {
                    Ok(p) => opts.unexpected_prob = Some(p),
                    Err(_) => {
                        eprintln!("error: --unexpected-prob expects a floating-point probability (got '{}')\n{}", v, usage());
                        return ExitCode::FAILURE;
                    }
                },
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
        use crate::codegen::mc_concepts::ExactStatus;
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
        let parsed = crate::json::json_rw::parse(doc).expect("strict parse");
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
