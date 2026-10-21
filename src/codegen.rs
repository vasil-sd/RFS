//! Code generation backend API and plugin registry.
//!
//! This module is the language-agnostic core of the RFS code generator. It
//! defines the `Backend` plugin interface, the static registry of all backends
//! (`BACKENDS`), the shared option/output types, and the argument parsing for
//! the `rfs codegen` CLI subcommand. Target-language specific generation lives
//! in `crate::backends` (one module tree per language; see `CODEGEN.md` for
//! the full plugin API documentation).
//!
//! Shared language-agnostic building blocks used by backends:
//!
//! - `crate::codegen_fsm`: FSM model analysis (variant flattening,
//!   representative values, safety-formula subset);
//! - `crate::codegen_test`: unit-test planning (positive/negative run plan);
//! - `crate::codegen_mc`: bounded model-checking concepts (config, violation
//!   specs, sequence-generation phases, run-status taxonomy).

use crate::backends::{c, rust};
use crate::ir::Module;
use std::fmt::Write as _;

// ---------------------------------------------------------------------------
// Shared option and output types
// ---------------------------------------------------------------------------

/// Which assumptions a backend should activate in generated code.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum AssumptionSelection {
    #[default]
    All,
    None,
    Labels(Vec<String>),
}

/// Model-checking mode for the `mc` command (Phase 4, D-P4-7).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum McMode {
    /// Exact reachable-state search (the Phase 4 default).
    #[default]
    Exact,
    /// Legacy seeded-random bounded runs (kept behind `--mode fuzz`).
    Fuzz,
    /// SMT-based bounded model checking with z3 (Phase 4 task 11, optional):
    /// k-step unrolling over bitvectors; sound FAIL within bound / otherwise
    /// incomplete (clearly labeled). Generated binaries link libz3.
    SmtBounded,
}

impl std::str::FromStr for McMode {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim() {
            "exact" => Ok(Self::Exact),
            "fuzz" => Ok(Self::Fuzz),
            "smt-bounded" => Ok(Self::SmtBounded),
            other => Err(format!(
                "unknown mc mode '{}'; expected exact, fuzz or smt-bounded",
                other
            )),
        }
    }
}

impl std::str::FromStr for AssumptionSelection {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim() {
            "all" => Ok(Self::All),
            "none" => Ok(Self::None),
            other => Ok(Self::Labels(
                other
                    .split(',')
                    .map(|x| x.trim().to_string())
                    .filter(|x| !x.is_empty())
                    .collect(),
            )),
        }
    }
}

/// Pinned error message for `--no-std` on a non-production command (Phase 5
/// task 9): the CLI parser and the backend entry points both reject with this
/// exact text, so tests can pin one string for every path.
pub const NO_STD_PRODUCTION_ONLY_ERROR: &str =
    "--no-std is only supported for the production command";

/// Options shared by all backend commands. A backend may ignore fields that do
/// not apply to a specific command (for example `steps` is only meaningful for
/// the `test` command).
#[derive(Debug, Clone)]
pub struct BackendOptions {
    /// Assumptions to activate (production code and model checker).
    pub assumptions: AssumptionSelection,
    /// Example-run length K for test generation.
    pub steps: usize,
    /// no_std production codegen (Phase 5 task 9): the generated Rust becomes
    /// `#![no_std]` + `extern crate alloc` with an IDENTICAL public API shape
    /// (only the file header changes; the body is byte-identical to the std
    /// variant). Only the `production` command accepts this - test/mc entry
    /// points reject it with [`NO_STD_PRODUCTION_ONLY_ERROR`]. The C backend
    /// accepts it as a documented no-op (plain C is already dependency-free)
    /// and notes that in the generated header.
    pub no_std: bool,
    /// Emit property monitors in production output (D-P3-6). Only the
    /// `production` command accepts this option; `test` and `mc` always
    /// include property monitors, so they ignore it.
    pub with_property_monitors: bool,
    /// MC mode (Phase 4, D-P4-7): exact reachable-state search (default)
    /// or legacy fuzz. Only the `mc` command uses this; embedded in the
    /// generated binary as the runtime default.
    pub mc_mode: McMode,
    /// Exact-mode state limit default (`--max-states`, D-P4-7).
    pub mc_max_states: usize,
    /// Exact-mode nondeterministic successor enumeration default (`--nondet`).
    pub mc_nondet: bool,
    /// Exact-mode wall-clock timeout default in seconds (`--timeout-sec`; 0 =
    /// off, NOT parity-guaranteed).
    pub mc_timeout_sec: u64,
    /// Exact-mode parallel level expansion default (`--threads`, task 10B;
    /// 1 = sequential. N > 1 keeps results byte-identical to 1 via canonical
    /// commit; `--fast` is runtime-only and non-parity).
    pub mc_threads: usize,
    /// SMT-mode unrolling bound default (`--smt-bound`, task 11; the generated
    /// binary also accepts runtime `--bound N`).
    pub mc_smt_bound: usize,
}

impl Default for BackendOptions {
    fn default() -> Self {
        Self {
            assumptions: AssumptionSelection::All,
            steps: 4,
            no_std: false,
            with_property_monitors: false,
            mc_mode: McMode::Exact,
            mc_max_states: crate::codegen_mc::EXACT_DEFAULT_MAX_STATES,
            mc_nondet: false,
            mc_timeout_sec: crate::codegen_mc::EXACT_DEFAULT_TIMEOUT_SEC,
            mc_threads: crate::codegen_mc::EXACT_DEFAULT_THREADS,
            mc_smt_bound: crate::codegen_mc::SMT_DEFAULT_BOUND,
        }
    }
}

/// How property monitors are activated in a generated artifact (D-P3-6).
/// Backends map [`Gated`] to their compile-time debug switch: Rust renders
/// `#[cfg(debug_assertions)]`, C renders `#ifdef RFS_PROPERTY_MONITORS`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropertyMonitorMode {
    /// No property monitors are emitted (default production output).
    Off,
    /// Property monitors are emitted but compile-time gated (zero cost when
    /// the debug switch is off).
    Gated,
    /// Property monitors are always active (`test` and `mc` commands).
    Always,
}

/// Result of a backend generation command.
#[derive(Debug)]
pub struct CodegenOutput {
    /// The generated source text.
    pub code: String,
    /// Non-fatal notes (for example properties outside the translated subset).
    pub warnings: Vec<String>,
}

// ---------------------------------------------------------------------------
// Backend plugin API
// ---------------------------------------------------------------------------

/// Descriptor of one command a backend supports (used for help output).
#[derive(Debug, Clone, Copy)]
pub struct BackendCommandInfo {
    /// Command name as used on the CLI, e.g. `production`.
    pub name: &'static str,
    /// One-line summary shown in help.
    pub summary: &'static str,
    /// Full usage line, e.g. `rfs codegen rust production <spec.rfs> [--out FILE]`.
    pub usage: &'static str,
    /// Free-form option documentation for this command.
    pub options: &'static str,
}

/// A registered code-generation backend (plugin).
///
/// Backends are static data: a name, a description, help descriptors for each
/// supported command, and one function pointer per command kind. Adding a new
/// target language means adding a module under `crate::backends` that defines
/// such a value and appending it to `BACKENDS`. See `CODEGEN.md`.
pub struct Backend {
    /// CLI name of the backend, e.g. `rust`.
    pub name: &'static str,
    /// One-line description shown by `rfs codegen --list`.
    pub description: &'static str,
    /// The commands this backend supports (all three kinds are supported by
    /// every current backend; the descriptors document each one).
    pub commands: &'static [BackendCommandInfo],
    /// Generate production FSM source code.
    pub generate_production: fn(&Module, &BackendOptions) -> Result<CodegenOutput, Vec<String>>,
    /// Generate production FSM source code plus unit tests.
    pub generate_unit_tests: fn(&Module, &BackendOptions) -> Result<CodegenOutput, Vec<String>>,
    /// Generate a bounded model-checker executable source file.
    pub generate_model_checker: fn(&Module, &BackendOptions) -> Result<CodegenOutput, Vec<String>>,
}

/// The registry of all code-generation backends (the plugin table).
const ALL_BACKENDS: &[&Backend] = &[&c::C_BACKEND, &rust::RUST_BACKEND];

pub static BACKENDS: &[&Backend] = ALL_BACKENDS;

/// All registered backends as `(name, description)` pairs.
pub fn list_backends() -> Vec<(&'static str, &'static str)> {
    BACKENDS.iter().map(|b| (b.name, b.description)).collect()
}

/// Look up a backend by CLI name.
pub fn get_backend(name: &str) -> Option<&'static Backend> {
    BACKENDS.iter().find(|b| b.name == name).copied()
}

/// Common help text for `rfs codegen --help`.
pub fn common_help() -> String {
    let mut s = String::new();
    let _ = writeln!(s, "RFS code generation");
    let _ = writeln!(s);
    let _ = writeln!(s, "usage:");
    let _ = writeln!(s, "  rfs codegen --list");
    let _ = writeln!(s, "  rfs codegen --help");
    let _ = writeln!(s, "  rfs codegen <backend> --help");
    let _ = writeln!(
        s,
        "  rfs codegen <backend> <command> <spec.rfs|spec.json> [--out FILE] [options]"
    );
    let _ = writeln!(s);
    let _ = writeln!(s, "backends:");
    for (name, description) in list_backends() {
        let _ = writeln!(s, "  {:<10} {}", name, description);
    }
    let _ = writeln!(s);
    let _ = writeln!(s, "common options:");
    let _ = writeln!(
        s,
        "  --assumes all|none|label1,label2   assumptions to activate (default: all)"
    );
    let _ = writeln!(
        s,
        "  --steps N                          example-run length K for the test command (default: 4)"
    );
    let _ = writeln!(
        s,
        "  --with-property-monitors           production only: emit online property monitors\n                               (debug-gated; zero cost in release builds)"
    );
    let _ = writeln!(
        s,
        "  --no-std                         production only: Rust output becomes #![no_std] + extern crate alloc\n                               (identical public API); C accepts the flag as a documented no-op"
    );
    let _ = writeln!(
        s,
        "  --mode exact|fuzz|smt-bounded   mc only: model-checking mode (default: exact;\n                               smt-bounded generates a z3-based bounded checker - incomplete by design)"
    );
    let _ = writeln!(
        s,
        "  --max-states N                   mc only: state limit for exact mode (default: 1000000)"
    );
    let _ = writeln!(s, "  --nondet                         mc only: every enabled transition is a successor (exact mode)");
    let _ = writeln!(
        s,
        "  --threads N                      mc only: parallel level expansion for exact mode\n                               (default: 1). Deterministic - results are byte-identical to\n                               --threads 1 for any N. The generated binary also accepts\n                               runtime --threads N and --fast (non-parity early abort)."
    );
    let _ = writeln!(s, "  --smt-bound N                    mc only: unrolling bound k for smt-bounded mode (default: 10); the generated binary also accepts runtime --bound N");
    let _ = writeln!(
        s,
        "  --timeout-sec N                  mc only: wall-clock limit, 0 = off (default: 0).\n                               Wall-clock ONLY - NOT parity-guaranteed (the same spec can\n                               time out on one machine and not another); never used by\n                               parity or benchmark assertions."
    );
    let _ = writeln!(
        s,
        "  --out FILE                         write generated source to FILE (default: stdout)"
    );
    let _ = writeln!(s);
    let _ = writeln!(
        s,
        "run `rfs codegen <backend> --help` for backend-specific commands."
    );
    s
}

/// Help text for one backend (`rfs codegen <backend> --help`).
pub fn backend_help(backend: &Backend) -> String {
    let mut s = String::new();
    let _ = writeln!(s, "RFS codegen backend: {}", backend.name);
    let _ = writeln!(s, "{}", backend.description);
    let _ = writeln!(s);
    let _ = writeln!(s, "commands:");
    for c in backend.commands {
        let _ = writeln!(s, "  {:<12} {}", c.name, c.summary);
        let _ = writeln!(s, "              usage:   {}", c.usage);
        for line in c.options.lines() {
            let _ = writeln!(s, "              {}", line);
        }
    }
    s
}

// ---------------------------------------------------------------------------
// `rfs codegen` CLI request parsing
// ---------------------------------------------------------------------------

/// The three generation commands every backend supports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodegenCommandKind {
    /// Production FSM source.
    Production,
    /// Production FSM source plus unit tests.
    UnitTests,
    /// Bounded model-checker executable source.
    ModelChecker,
}

impl CodegenCommandKind {
    /// CLI names: `production`, `test`, `mc`.
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "production" => Some(Self::Production),
            "test" => Some(Self::UnitTests),
            "mc" => Some(Self::ModelChecker),
            _ => None,
        }
    }

    /// The command descriptor registered by `backend` for this kind.
    pub fn info(&self, backend: &Backend) -> &'static BackendCommandInfo {
        let name = match self {
            Self::Production => "production",
            Self::UnitTests => "test",
            Self::ModelChecker => "mc",
        };
        backend
            .commands
            .iter()
            .find(|c| c.name == name)
            .expect("backend registers all commands")
    }

    /// Dispatch to the backend's function pointer for this command.
    pub fn run(
        self,
        backend: &Backend,
        module: &Module,
        options: &BackendOptions,
    ) -> Result<CodegenOutput, Vec<String>> {
        match self {
            Self::Production => (backend.generate_production)(module, options),
            Self::UnitTests => (backend.generate_unit_tests)(module, options),
            Self::ModelChecker => (backend.generate_model_checker)(module, options),
        }
    }
}

/// A parsed `rfs codegen` CLI request.
#[derive(Debug)]
pub enum CodegenRequest {
    /// `rfs codegen --list`.
    List,
    /// `rfs codegen --help`.
    Help,
    /// `rfs codegen <backend> --help`.
    BackendHelp(String),
    /// `rfs codegen <backend> <command> <input> [--out FILE] [options]`.
    Generate {
        backend_name: String,
        command: CodegenCommandKind,
        input: String,
        out: Option<String>,
        assumptions: AssumptionSelection,
        steps: usize,
        no_std: bool,
        with_property_monitors: bool,
        mc_mode: McMode,
        mc_max_states: usize,
        mc_nondet: bool,
        mc_timeout_sec: u64,
        mc_threads: usize,
        mc_smt_bound: usize,
    },
}

/// Parse `rfs codegen` arguments into a request. Returns all errors found.
pub fn parse_codegen_args(args: &[String]) -> Result<CodegenRequest, Vec<String>> {
    let mut errors = Vec::new();
    if args.is_empty() {
        return Err(vec![
            "missing subcommand; usage: rfs codegen --list | --help | <backend> ...".into(),
        ]);
    }
    match args[0].as_str() {
        "--list" => {
            if args.len() > 1 {
                errors.push("--list takes no arguments".into());
            }
            return finish(CodegenRequest::List, errors);
        }
        "--help" | "-h" => {
            if args.len() > 1 {
                errors.push("--help takes no arguments".into());
            }
            return finish(CodegenRequest::Help, errors);
        }
        _ => {}
    }

    let backend_name = args[0].clone();
    let rest = &args[1..];
    if rest.first().map(|s| s.as_str()) == Some("--help")
        || rest.first().map(|s| s.as_str()) == Some("-h")
    {
        return finish(CodegenRequest::BackendHelp(backend_name), errors);
    }
    if rest.len() < 2 {
        return Err(vec![format!(
            "usage: rfs codegen {} <command> <spec.rfs|spec.json> [--out FILE] [options]; run `rfs codegen {} --help` for details",
            backend_name, backend_name
        )]);
    }

    let command = match CodegenCommandKind::from_str(&rest[0]) {
        Some(c) => c,
        None => {
            return Err(vec![format!(
                "unknown command '{}'; expected production, test or mc",
                rest[0]
            )]);
        }
    };
    let input = rest[1].clone();

    let mut out = None;
    let mut assumptions = AssumptionSelection::default();
    let mut steps = 4usize;
    let mut no_std = false;
    let mut with_property_monitors = false;
    let mut mc_mode = McMode::default();
    let mut mc_max_states = crate::codegen_mc::EXACT_DEFAULT_MAX_STATES;
    let mut mc_nondet = false;
    let mut mc_timeout_sec = crate::codegen_mc::EXACT_DEFAULT_TIMEOUT_SEC;
    let mut mc_threads = crate::codegen_mc::EXACT_DEFAULT_THREADS;
    let mut mc_smt_bound = crate::codegen_mc::SMT_DEFAULT_BOUND;
    let mut i = 2;
    while i < rest.len() {
        match rest[i].as_str() {
            "--out" => {
                i += 1;
                match rest.get(i) {
                    Some(v) => out = Some(v.clone()),
                    None => errors.push("--out requires a file argument".into()),
                }
            }
            "--assumes" => {
                i += 1;
                match rest.get(i) {
                    Some(v) => assumptions = v.parse().unwrap_or_default(),
                    None => {
                        errors.push("--assumes requires a value (all|none|label1,label2)".into())
                    }
                }
            }
            "--steps" => {
                i += 1;
                match rest.get(i).and_then(|v| v.parse::<usize>().ok()) {
                    Some(n) => steps = n,
                    None => errors.push("--steps requires a numeric argument".into()),
                }
            }
            "--with-property-monitors" => {
                if command != CodegenCommandKind::Production {
                    errors.push(
                        "--with-property-monitors is only valid for the production command; test and mc always include property monitors"
                            .into(),
                    );
                } else {
                    with_property_monitors = true;
                }
            }
            "--no-std" => {
                if command != CodegenCommandKind::Production {
                    errors.push(NO_STD_PRODUCTION_ONLY_ERROR.to_string());
                } else {
                    no_std = true;
                }
            }
            "--mode" => {
                i += 1;
                match rest.get(i) {
                    Some(v) => mc_mode = v.parse().unwrap_or_default(),
                    None => errors.push("--mode requires a value (exact|fuzz|smt-bounded)".into()),
                }
            }
            "--max-states" => {
                i += 1;
                match rest.get(i).and_then(|v| v.parse::<usize>().ok()) {
                    Some(n) => mc_max_states = n,
                    None => errors.push("--max-states requires a numeric argument".into()),
                }
            }
            "--nondet" => mc_nondet = true,
            "--timeout-sec" => {
                i += 1;
                match rest.get(i).and_then(|v| v.parse::<u64>().ok()) {
                    Some(n) => mc_timeout_sec = n,
                    None => errors.push("--timeout-sec requires a numeric argument".into()),
                }
            }
            "--threads" => {
                i += 1;
                match rest.get(i).and_then(|v| v.parse::<usize>().ok()) {
                    Some(n) if n > 0 => mc_threads = n,
                    _ => errors.push("--threads requires a positive integer argument".into()),
                }
            }
            "--smt-bound" => {
                i += 1;
                match rest.get(i).and_then(|v| v.parse::<usize>().ok()) {
                    Some(n) if n > 0 => mc_smt_bound = n,
                    _ => errors.push("--smt-bound requires a positive integer argument".into()),
                }
            }
            other if other.starts_with('-') => errors.push(format!("unknown option: {}", other)),
            other => errors.push(format!("unexpected extra argument: {}", other)),
        }
        i += 1;
    }

    finish(
        CodegenRequest::Generate {
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
        },
        errors,
    )
}

fn finish(request: CodegenRequest, errors: Vec<String>) -> Result<CodegenRequest, Vec<String>> {
    if errors.is_empty() {
        Ok(request)
    } else {
        Err(errors)
    }
}
