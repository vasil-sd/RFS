//! `rfs gen-spec` command logic (Phase 6 task 3, action 6 / D-P6-11).
//!
//! Writes a random spec to FILE.rfs in one of two modes:
//!   * fully random - from a seed + size bounds (`SpecGen`);
//!   * mutation - the base spec (`.rfs` or `.json`) mutated K times.
//!
//! The output is ALWAYS parsed + validated before writing (D-P6-11: validity
//! by construction, guarded anyway); on any failure exit 1 with a diagnostic.
//! The pipeline is exposed as lib functions ([`generate_random`],
//! [`mutate_base`]) so integration tests can call them without spawning the
//! binary; [`run_cli`] is the thin argument-parsing shell for `main.rs`.

use std::path::Path;
use std::process::ExitCode;

use crate::frontend::{self, Format};
use crate::specgen::{self, Bounds, Rng, SpecGen};

/// The pinned default seed (documented in the usage text): deterministic
/// output for bare invocations.
pub const DEFAULT_SEED: u64 = 12345;

/// Parsed fully-random-mode options.
#[derive(Debug, Clone)]
pub struct RandomOptions {
    pub seed: u64,
    /// Base preset (`--size`); explicit caps override individual fields.
    pub bounds: Bounds,
}

impl Default for RandomOptions {
    fn default() -> Self {
        Self {
            seed: DEFAULT_SEED,
            bounds: Bounds::small(),
        }
    }
}

/// Fully random mode: generate from the seed, render through the printer, and
/// re-parse + validate the rendered text before returning it.
pub fn generate_random(opts: &RandomOptions) -> Result<String, String> {
    let ir = SpecGen::new(opts.seed, opts.bounds.clone()).generate();
    verify_rendered(&crate::printer::print_ir(&ir))
}

/// Mutation mode: load + validate the base spec, apply K seeded mutation
/// steps to its first module (RFS modules are independent), render, and
/// re-parse + validate the rendered text before returning it.
pub fn mutate_base(base_path: &Path, k: usize, seed: u64) -> Result<String, String> {
    let ext = base_path.extension().and_then(|e| e.to_str()).unwrap_or("");
    let fmt = Format::from_extension(ext).ok_or_else(|| {
        format!(
            "base '{}' must have extension .rfs or .json",
            base_path.display()
        )
    })?;
    let ir = frontend::convert_file_to_ir(base_path, fmt)?;
    if ir.modules.is_empty() {
        return Err(format!("no modules in {}", base_path.display()));
    }
    let errors = crate::validator::validate_module(&ir.modules[0]);
    if !errors.is_empty() {
        return Err(format!(
            "{}: {} validation error(s): {}",
            base_path.display(),
            errors.len(),
            errors
                .iter()
                .map(|e| e.message.clone())
                .collect::<Vec<_>>()
                .join("; ")
        ));
    }
    let mut rng = Rng::new(seed);
    let out_ir = specgen::mutate(&ir, &mut rng, k);
    verify_rendered(&crate::printer::print_ir(&out_ir))
}

/// D-P6-11 guard: the rendered text must parse back and validate (a failure
/// here is a generator/mutation bug, not user error).
fn verify_rendered(src: &str) -> Result<String, String> {
    let ir = crate::parser::parse_source(src)
        .map_err(|e| format!("generated source failed to parse: {e:?}"))?;
    crate::validator::validate_ir(&ir)
        .map_err(|e| format!("generated source failed validation: {e:?}"))?;
    Ok(src.to_string())
}

fn usage() -> String {
    "usage: rfs gen-spec --out FILE.rfs [--seed N] [--size small|medium]\n       [--max-states N] [--max-events N] [--max-vars N]\n       [--max-transitions N] [--max-props N]\n   |  rfs gen-spec --out FILE.rfs --base SPEC.rfs|SPEC.json --mutations K [--seed N]\n\nWrites a random spec to FILE.rfs. Fully random mode: deterministic from\n--seed (default: the pinned constant 12345) + --size small|medium\n(default small; explicit --max-* caps override individual bounds).\nMutation mode: parses --base, applies K seeded mutation steps to its first\nmodule, and writes the result (K=0 re-renders the base unchanged).\nThe output is always parsed + validated before writing; on any failure\nexit 1 with a diagnostic. Same args => byte-identical file."
        .to_string()
}

/// `rfs gen-spec` argument parsing + pipeline (thin shell for main.rs).
pub fn run_cli(args: &[String]) -> ExitCode {
    use crate::cli_log::{LogLevel, Logger};
    let mut out: Option<String> = None;
    let mut seed: u64 = DEFAULT_SEED;
    let mut size: Option<String> = None;
    let mut caps: [Option<u64>; 5] = [None; 5]; // states, events, vars, transitions, props
    let mut base: Option<String> = None;
    let mut mutations: Option<usize> = None;
    let mut log_level = LogLevel::Info;

    macro_rules! cap {
        ($idx:expr, $name:expr, $v:expr) => {{
            match $v.parse::<u64>() {
                Ok(n) if n > 0 => caps[$idx] = Some(n),
                _ => {
                    eprintln!(
                        "error: --{} expects a positive integer (got '{}')\n{}",
                        $name,
                        $v,
                        usage()
                    );
                    return ExitCode::FAILURE;
                }
            }
        }};
    }

    let mut i = 0;
    while i < args.len() {
        let arg = args[i].clone();
        if arg == "--help" || arg == "-h" {
            println!("{}", usage());
            return ExitCode::SUCCESS;
        }
        match arg.as_str() {
            "--" => {}
            "--out" => match take_value(args, &mut i, &arg) {
                Ok(v) => {
                    if out.is_some() {
                        eprintln!("error: --out given twice\n{}", usage());
                        return ExitCode::FAILURE;
                    }
                    out = Some(v);
                }
                Err(e) => {
                    eprintln!("error: {}\n{}", e, usage());
                    return ExitCode::FAILURE;
                }
            },
            "--seed" => match take_value(args, &mut i, &arg) {
                Ok(v) => match v.parse::<u64>() {
                    Ok(n) => seed = n,
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
            "--size" => match take_value(args, &mut i, &arg) {
                Ok(v) => match v.as_str() {
                    "small" | "medium" => size = Some(v),
                    _ => {
                        eprintln!(
                            "error: --size expects small or medium (got '{}')\n{}",
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
            "--max-states" => match take_value(args, &mut i, &arg) {
                Ok(v) => cap!(0, "max-states", v),
                Err(e) => {
                    eprintln!("error: {}\n{}", e, usage());
                    return ExitCode::FAILURE;
                }
            },
            "--max-events" => match take_value(args, &mut i, &arg) {
                Ok(v) => cap!(1, "max-events", v),
                Err(e) => {
                    eprintln!("error: {}\n{}", e, usage());
                    return ExitCode::FAILURE;
                }
            },
            "--max-vars" => match take_value(args, &mut i, &arg) {
                Ok(v) => cap!(2, "max-vars", v),
                Err(e) => {
                    eprintln!("error: {}\n{}", e, usage());
                    return ExitCode::FAILURE;
                }
            },
            "--max-transitions" => match take_value(args, &mut i, &arg) {
                Ok(v) => cap!(3, "max-transitions", v),
                Err(e) => {
                    eprintln!("error: {}\n{}", e, usage());
                    return ExitCode::FAILURE;
                }
            },
            "--max-props" => match take_value(args, &mut i, &arg) {
                Ok(v) => cap!(4, "max-props", v),
                Err(e) => {
                    eprintln!("error: {}\n{}", e, usage());
                    return ExitCode::FAILURE;
                }
            },
            "--base" => match take_value(args, &mut i, &arg) {
                Ok(v) => {
                    if base.is_some() {
                        eprintln!("error: --base given twice\n{}", usage());
                        return ExitCode::FAILURE;
                    }
                    base = Some(v);
                }
                Err(e) => {
                    eprintln!("error: {}\n{}", e, usage());
                    return ExitCode::FAILURE;
                }
            },
            "--mutations" => {
                match take_value(args, &mut i, &arg) {
                    Ok(v) => {
                        match v.parse::<usize>() {
                            Ok(n) => mutations = Some(n),
                            Err(_) => {
                                eprintln!("error: --mutations expects a non-negative integer (got '{}')\n{}", v, usage());
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
                eprintln!("unexpected extra argument: {}\n{}", arg, usage());
                return ExitCode::FAILURE;
            }
        }
        i += 1;
    }

    let Some(out) = out else {
        eprintln!("error: rfs gen-spec requires --out FILE.rfs\n{}", usage());
        return ExitCode::FAILURE;
    };
    if !out.ends_with(".rfs") {
        eprintln!(
            "error: --out must have a .rfs extension (got '{}')\n{}",
            out,
            usage()
        );
        return ExitCode::FAILURE;
    }

    // Mode selection: --base => mutation mode; otherwise fully random.
    let logger = Logger::new(log_level);
    let result = match base {
        Some(base) => {
            let Some(mutations) = mutations else {
                eprintln!("error: --base requires --mutations K\n{}", usage());
                return ExitCode::FAILURE;
            };
            if size.is_some() || caps.iter().any(|c| c.is_some()) {
                eprintln!(
                    "error: mutation mode (--base/--mutations) is mutually exclusive with --size and the --max-* caps\n{}",
                    usage()
                );
                return ExitCode::FAILURE;
            }
            mutate_base(Path::new(&base), mutations, seed)
        }
        None => {
            if mutations.is_some() {
                eprintln!("error: --mutations requires --base SPEC\n{}", usage());
                return ExitCode::FAILURE;
            }
            let mut bounds = match size.as_deref() {
                Some("medium") => Bounds::medium(),
                _ => Bounds::small(),
            };
            if let Some(n) = caps[0] {
                bounds.max_states = n as usize;
            }
            if let Some(n) = caps[1] {
                bounds.max_events = n as usize;
            }
            if let Some(n) = caps[2] {
                bounds.max_vars = n as usize;
            }
            if let Some(n) = caps[3] {
                bounds.max_transitions = n as usize;
            }
            if let Some(n) = caps[4] {
                bounds.max_props = n as usize;
            }
            generate_random(&RandomOptions { seed, bounds })
        }
    };

    match result {
        Ok(src) => match std::fs::write(&out, &src) {
            Ok(()) => {
                logger.debug(&format!("gen-spec: wrote {} byte(s) to {}", src.len(), out));
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("error: cannot write {}: {e}", out);
                ExitCode::FAILURE
            }
        },
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
    fn default_seed_is_pinned_constant() {
        assert_eq!(DEFAULT_SEED, 12345);
    }

    #[test]
    fn generate_random_deterministic_and_valid() {
        let a = generate_random(&RandomOptions::default()).unwrap();
        let b = generate_random(&RandomOptions::default()).unwrap();
        assert_eq!(a, b);
        let ir = crate::parser::parse_source(&a).unwrap();
        crate::validator::validate_ir(&ir).unwrap();
    }

    #[test]
    fn caps_override_preset() {
        let mut bounds = Bounds::small();
        bounds.max_states = 2;
        let a = generate_random(&RandomOptions { seed: 7, bounds }).unwrap();
        let ir = crate::parser::parse_source(&a).unwrap();
        let states = ir.modules[0]
            .enums
            .iter()
            .find(|e| e.name == "State")
            .map(|e| e.variants.len())
            .unwrap_or(0);
        assert!(states <= 2, "cap not respected: {states} states");
    }

    #[test]
    fn mutate_base_k_zero_rerenders_base() {
        let dir = std::env::temp_dir().join("rfs_gen_spec_cmd_k0");
        std::fs::create_dir_all(&dir).unwrap();
        let base_path = dir.join("base.rfs");
        let src = "module m {\n  enum State { A, B }\n  initial A\n  event E(U)\n  output O(V)\n  transition A on E(U) { next B; emit O(V); }\n}\n";
        std::fs::write(&base_path, src).unwrap();
        let out = mutate_base(&base_path, 0, DEFAULT_SEED).unwrap();
        // k=0 is the identity: the re-rendered base parses back to the same IR.
        let a = crate::parser::parse_source(&out).unwrap();
        let b = crate::parser::parse_source(src).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn mutate_base_missing_file_errors() {
        let err = mutate_base(Path::new("/nonexistent/base.rfs"), 3, DEFAULT_SEED).unwrap_err();
        assert!(!err.is_empty());
    }
}
