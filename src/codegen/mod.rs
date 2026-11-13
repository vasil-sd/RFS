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
//! - `crate::codegen::fsm`: FSM model analysis (variant flattening,
//!   representative values, safety-formula subset);
//! - `crate::codegen::test_plan`: unit-test planning (positive/negative run plan);
//! - `crate::codegen::mc_concepts`: bounded model-checking concepts (config, violation
//!   specs, sequence-generation phases, run-status taxonomy).
//!
//! Docs: dev:codegen-core-overview, dev:codegen-plugin-api, dev:algorithms-ir-codegen, llm:codegen-core, dev:code-map-codegen, dev:deps-fn-codegen-rust

pub mod backends;
pub mod fsm;
pub mod ident;
pub mod mc_concepts;
pub mod monitor;
pub mod test_plan;

use crate::codegen::backends::{c, rust};
use crate::lang::ir::{Ir, Module};
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

/// Pinned error message for `--with-protocol-fsm` on the `mc` command
/// (Phase 5 task 11): the trait impl belongs to the production/test surface;
/// the model checker keeps its own runtime. The CLI parser and the Rust mc
/// entry point both reject with this exact text.
pub const PROTOCOL_FSM_MC_ERROR: &str =
    "--with-protocol-fsm is only valid for the production and test commands";

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
    /// Exact-mode counterexample report format (Phase 5 task 10): text
    /// (default, the parity-pinned human-readable report) or json (a single
    /// machine-readable document, byte-identical across backends). Embedded
    /// in the generated binary as the runtime default; overridable at
    /// runtime with `--counterexample-format`. Exact mode only - fuzz and
    /// smt-bounded modes keep their own output.
    pub counterexample_format: crate::codegen::mc_concepts::CounterexampleFormat,
    /// Emit the opt-in `impl ProtocolFsm for Fsm` (Phase 5 task 11): links
    /// the generated artifact to the `rfs-adapters` crate via
    /// `use rfs_adapters::ProtocolFsm;` - the one documented opt-in that
    /// connects generated code to an external crate. OFF by default (the
    /// plain surface is unchanged). Rust backend only: production + test
    /// commands emit the impl, `mc` rejects it ([`PROTOCOL_FSM_MC_ERROR`]),
    /// and the C backend ignores it (Rust-ecosystem surface; the C
    /// equivalent is the plain step API).
    pub with_protocol_fsm: bool,
    /// v1.0 spec hash (Phase 6, D-P6-3): first 16 hex chars of the SHA-256 of
    /// the CANONICAL IR JSON of the whole input file (`Ir::to_json()` text -
    /// deterministic per D-P5-2). The standard entry point
    /// [`CodegenCommandKind::run`] stamps it once per invocation; it is embedded
    /// in line 1 of every generated artifact (both backends, all commands +
    /// fuzz skeletons) so users can detect spec/toolchain drift in checked-in
    /// code. Empty only when a library caller renders without going through
    /// `run` and did not stamp it.
    pub spec_hash: String,
    /// Resolved C fsm-header file name (Phase 14 task 2): the CLI layer and
    /// `rfs test` stamp this from the resolved output path so the generated
    /// source `#include`s its own header by the correct name and the include
    /// guard matches. None = a library caller generating standalone content,
    /// which uses the fixed default `rfs_fsm.h`.
    pub fsm_header_name: Option<String>,
    /// Resolved C fsm-source file name (Phase 14 task 2); carried for symmetry
    /// with the header name. None = standalone default.
    pub fsm_source_name: Option<String>,
}

impl Default for BackendOptions {
    fn default() -> Self {
        Self {
            assumptions: AssumptionSelection::All,
            steps: 4,
            no_std: false,
            with_property_monitors: false,
            mc_mode: McMode::Exact,
            mc_max_states: crate::codegen::mc_concepts::EXACT_DEFAULT_MAX_STATES,
            mc_nondet: false,
            mc_timeout_sec: crate::codegen::mc_concepts::EXACT_DEFAULT_TIMEOUT_SEC,
            mc_threads: crate::codegen::mc_concepts::EXACT_DEFAULT_THREADS,
            mc_smt_bound: crate::codegen::mc_concepts::SMT_DEFAULT_BOUND,
            counterexample_format: crate::codegen::mc_concepts::CounterexampleFormat::Text,
            fsm_header_name: None,
            fsm_source_name: None,
            with_protocol_fsm: false,
            spec_hash: String::new(),
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

/// The named artifact slots a generation command can fill (Phase 14 task 1).
/// A command fills exactly the slots its artifacts occupy: production fills
/// the fsm slots, mc the mc slots, test the test + fuzz-target slots; the C
/// backend additionally fills the header slots (tasks 2-3). The CLI write loop
/// resolves each filled slot to its output path and applies take-or-emit to
/// the referenced fsm slots.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtifactSlot {
    /// C production: the FSM public header (types + API declarations).
    FsmHeader,
    /// Production command: the FSM implementation source (all backends).
    FsmCode,
    /// C mc: the MC engine public header (run entry + report types).
    McHeader,
    /// Mc command: the model-checker executable source (all backends).
    McCode,
    /// Test command: the unit-test artifact source (all backends).
    TestCode,
    /// Test command: the libFuzzer target skeleton (all backends; it needs a
    /// libFuzzer-enabled build, so it is never inlined into the test artifact).
    FuzzTarget,
}

impl ArtifactSlot {
    /// The default file name for this slot under BASE - the documented
    /// expansion table (`BASE-fsm.h`/`BASE-fsm.c`, `BASE-mc.h`/`BASE-mc.c`,
    /// `BASE-test.c`, `BASE-fuzz-target.c`; Rust without headers). Header slots
    /// exist for the C backend only.
    pub fn default_name(&self, base: &str, c_backend: bool) -> String {
        match (*self, c_backend) {
            (ArtifactSlot::FsmHeader, true) => format!("{base}-fsm.h"),
            (ArtifactSlot::FsmCode, true) => format!("{base}-fsm.c"),
            (ArtifactSlot::McHeader, true) => format!("{base}-mc.h"),
            (ArtifactSlot::McCode, true) => format!("{base}-mc.c"),
            (ArtifactSlot::TestCode, true) => format!("{base}-test.c"),
            (ArtifactSlot::FuzzTarget, true) => format!("{base}-fuzz-target.c"),
            (ArtifactSlot::FsmCode, false) => format!("{base}-fsm.rs"),
            (ArtifactSlot::McCode, false) => format!("{base}-mc.rs"),
            (ArtifactSlot::TestCode, false) => format!("{base}-test.rs"),
            (ArtifactSlot::FuzzTarget, false) => format!("{base}-fuzz-target.rs"),
            _ => panic!("header slots are C-only"),
        }
    }
}

/// Result of a backend generation command: named artifact slots instead of a
/// single primary string + unnamed extras (Phase 14 task 1).
#[derive(Debug, Default)]
pub struct CodegenOutput {
    /// C production only: the FSM public header.
    pub fsm_header: Option<String>,
    /// Production command: the FSM implementation source (all backends).
    pub fsm_code: Option<String>,
    /// C mc only: the MC engine public header.
    pub mc_header: Option<String>,
    /// Mc command: the model-checker executable source (all backends).
    pub mc_code: Option<String>,
    /// Test command: the unit-test artifact source (all backends).
    pub test_code: Option<String>,
    /// Test command: the libFuzzer target skeleton (all backends).
    pub fuzz_target: Option<String>,
    /// Non-fatal notes (for example properties outside the translated subset).
    pub warnings: Vec<String>,
}

impl CodegenOutput {
    /// The filled slots in canonical order with their content.
    pub fn filled_slots(&self) -> Vec<(ArtifactSlot, &str)> {
        let mut v = Vec::new();
        if let Some(s) = &self.fsm_header {
            v.push((ArtifactSlot::FsmHeader, s.as_str()));
        }
        if let Some(s) = &self.fsm_code {
            v.push((ArtifactSlot::FsmCode, s.as_str()));
        }
        if let Some(s) = &self.mc_header {
            v.push((ArtifactSlot::McHeader, s.as_str()));
        }
        if let Some(s) = &self.mc_code {
            v.push((ArtifactSlot::McCode, s.as_str()));
        }
        if let Some(s) = &self.test_code {
            v.push((ArtifactSlot::TestCode, s.as_str()));
        }
        if let Some(s) = &self.fuzz_target {
            v.push((ArtifactSlot::FuzzTarget, s.as_str()));
        }
        v
    }

    /// The content of one named slot, if filled.
    pub fn slot_content(&self, slot: ArtifactSlot) -> Option<&str> {
        let s = match slot {
            ArtifactSlot::FsmHeader => &self.fsm_header,
            ArtifactSlot::FsmCode => &self.fsm_code,
            ArtifactSlot::McHeader => &self.mc_header,
            ArtifactSlot::McCode => &self.mc_code,
            ArtifactSlot::TestCode => &self.test_code,
            ArtifactSlot::FuzzTarget => &self.fuzz_target,
        };
        s.as_deref()
    }

    /// The sole filled PRIMARY slot's content (fsm_code / test_code /
    /// mc_code - the header and fuzz-target slots are not primaries).
    /// Every command fills exactly one primary slot, so this is unambiguous;
    /// it panics if that invariant breaks (a generator bug).
    pub fn primary_code(&self) -> &str {
        let filled = [
            self.fsm_code.is_some(),
            self.test_code.is_some(),
            self.mc_code.is_some(),
        ]
        .into_iter()
        .filter(|b| *b)
        .count();
        assert_eq!(filled, 1, "exactly one primary slot must be filled");
        self.fsm_code
            .as_deref()
            .or(self.test_code.as_deref())
            .or(self.mc_code.as_deref())
            .expect("a primary slot is filled")
    }

    /// The command's primary slot content (production -> fsm_code, test ->
    /// test_code, mc -> mc_code). Panics on an empty slot - a generator bug,
    /// not a runtime condition.
    pub fn primary(&self, command: CodegenCommandKind) -> &str {
        let slot = match command {
            CodegenCommandKind::Production => &self.fsm_code,
            CodegenCommandKind::UnitTests => &self.test_code,
            CodegenCommandKind::ModelChecker => &self.mc_code,
        };
        slot.as_deref()
            .expect("primary slot filled by its generator")
    }
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

    /// Standard dispatch (the CLI rule): generate for the FIRST module of
    /// `ir`, stamping the v1.0 spec hash (D-P6-3) - SHA-256 of the canonical
    /// IR JSON of the WHOLE input file, first 16 hex chars - into the options
    /// so every artifact carries it in its line-1 header. One hash per
    /// artifact; multi-module files share the file's hash.
    pub fn run(
        self,
        backend: &Backend,
        ir: &Ir,
        options: &BackendOptions,
    ) -> Result<CodegenOutput, Vec<String>> {
        // Canonical IR JSON text (deterministic key order/format per D-P5-2)
        // is the hash input - the whole file, all modules.
        let json_text = ir.to_json().map_err(|e| vec![e])?.to_string();
        let mut stamped = options.clone();
        stamped.spec_hash = crate::utils::hash::spec_hash16(&json_text);
        self.run_module(backend, &ir.modules[0], &stamped)
    }

    /// Per-module dispatch for library consumers that generate each module of
    /// a multi-module file individually (the CLI rule - first module via
    /// [`Self::run`] - is not applied here). The caller owns
    /// `options.spec_hash`: stamp it with the same whole-file hash when parity
    /// with CLI artifacts matters.
    pub fn run_module(
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
// The Generate variant carries the full flag family (per-artifact output
// names + fsm-reference forms), which makes it far larger than List/Help/
// BackendHelp. Boxing would ripple through every construction and match site
// for a struct that is built once per process invocation and matched once -
// not worth the indirection.
#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
pub enum CodegenRequest {
    /// `rfs codegen --list`.
    List,
    /// `rfs codegen --help`.
    Help,
    /// `rfs codegen <backend> --help`.
    BackendHelp(String),
    /// `rfs codegen <backend> <command> <input> [output options] [options]`.
    Generate {
        backend_name: String,
        command: CodegenCommandKind,
        input: String,
        /// The `--out` value: the BASE name all default artifact names derive
        /// from, or "-" for stdout mode. None = the spec-file-stem default.
        base: Option<String>,
        /// Per-artifact output-name overrides (each command accepts only its
        /// own subset - enforced by the parser).
        out_fsm_code: Option<String>,
        out_fsm_header: Option<String>,
        out_mc_code: Option<String>,
        out_mc_header: Option<String>,
        out_test_code: Option<String>,
        out_fuzz_target: Option<String>,
        /// Fsm-reference flags (mc/test only): Rust `NAME[.rs]`, C form 1
        /// `STEM` (-> STEM.h + STEM.c), C form 2 via the header/source pair.
        fsm_file: Option<String>,
        fsm_file_header: Option<String>,
        fsm_file_source: Option<String>,
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
        counterexample_format: crate::codegen::mc_concepts::CounterexampleFormat,
        with_protocol_fsm: bool,
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

    let mut base = None;
    let mut out_fsm_code = None;
    let mut out_fsm_header = None;
    let mut out_mc_code = None;
    let mut out_mc_header = None;
    let mut out_test_code = None;
    let mut out_fuzz_target = None;
    let mut fsm_file = None;
    let mut fsm_file_header = None;
    let mut fsm_file_source = None;
    let mut assumptions = AssumptionSelection::default();
    let mut steps = 4usize;
    let mut no_std = false;
    let mut with_property_monitors = false;
    let mut mc_mode = McMode::default();
    let mut mc_max_states = crate::codegen::mc_concepts::EXACT_DEFAULT_MAX_STATES;
    let mut mc_nondet = false;
    let mut mc_timeout_sec = crate::codegen::mc_concepts::EXACT_DEFAULT_TIMEOUT_SEC;
    let mut mc_threads = crate::codegen::mc_concepts::EXACT_DEFAULT_THREADS;
    let mut mc_smt_bound = crate::codegen::mc_concepts::SMT_DEFAULT_BOUND;
    let mut counterexample_format = crate::codegen::mc_concepts::CounterexampleFormat::Text;
    let mut with_protocol_fsm = false;
    let mut i = 2;
    while i < rest.len() {
        match rest[i].as_str() {
            "--out" => {
                i += 1;
                match rest.get(i) {
                    Some(v) => base = Some(v.clone()),
                    None => errors.push("--out requires a file argument".into()),
                }
            }
            "--out-fsm-code" => {
                if command != CodegenCommandKind::Production {
                    errors.push(
                        "--out-fsm-code is only valid for the production command; mc and test name their referenced fsm artifact with --fsm-file"
                            .into(),
                    );
                } else {
                    i += 1;
                    match rest.get(i) {
                        Some(v) => out_fsm_code = Some(v.clone()),
                        None => errors.push("--out-fsm-code requires a file argument".into()),
                    }
                }
            }
            "--out-fsm-header" => {
                if backend_name != "c" {
                    errors.push(
                        "--out-fsm-header is only valid for the c backend (the rust backend has no header files)"
                            .into(),
                    );
                } else if command != CodegenCommandKind::Production {
                    errors.push(
                        "--out-fsm-header is only valid for the production command; mc and test name their referenced fsm artifact with --fsm-file"
                            .into(),
                    );
                } else {
                    i += 1;
                    match rest.get(i) {
                        Some(v) => out_fsm_header = Some(v.clone()),
                        None => errors.push("--out-fsm-header requires a file argument".into()),
                    }
                }
            }
            "--out-mc-code" => {
                if command != CodegenCommandKind::ModelChecker {
                    errors.push("--out-mc-code is only valid for the mc command".into());
                } else {
                    i += 1;
                    match rest.get(i) {
                        Some(v) => out_mc_code = Some(v.clone()),
                        None => errors.push("--out-mc-code requires a file argument".into()),
                    }
                }
            }
            "--out-mc-header" => {
                if backend_name != "c" {
                    errors.push(
                        "--out-mc-header is only valid for the c backend (the rust backend has no header files)"
                            .into(),
                    );
                } else if command != CodegenCommandKind::ModelChecker {
                    errors.push("--out-mc-header is only valid for the mc command".into());
                } else {
                    i += 1;
                    match rest.get(i) {
                        Some(v) => out_mc_header = Some(v.clone()),
                        None => errors.push("--out-mc-header requires a file argument".into()),
                    }
                }
            }
            "--out-test-code" => {
                if command != CodegenCommandKind::UnitTests {
                    errors.push("--out-test-code is only valid for the test command".into());
                } else {
                    i += 1;
                    match rest.get(i) {
                        Some(v) => out_test_code = Some(v.clone()),
                        None => errors.push("--out-test-code requires a file argument".into()),
                    }
                }
            }
            "--out-fuzz-target-code" => {
                if command != CodegenCommandKind::UnitTests {
                    errors.push("--out-fuzz-target-code is only valid for the test command".into());
                } else {
                    i += 1;
                    match rest.get(i) {
                        Some(v) => out_fuzz_target = Some(v.clone()),
                        None => {
                            errors.push("--out-fuzz-target-code requires a file argument".into())
                        }
                    }
                }
            }
            "--fsm-file" => {
                if command == CodegenCommandKind::Production {
                    errors.push(
                        "--fsm-file is only valid for the mc and test commands (production IS the fsm artifact; name it with --out-fsm-code/--out-fsm-header or --out)"
                            .into(),
                    );
                } else {
                    i += 1;
                    match rest.get(i) {
                        Some(v) => fsm_file = Some(v.clone()),
                        None => errors.push("--fsm-file requires a file argument".into()),
                    }
                }
            }
            "--fsm-file-header" | "--fsm-file-source" => {
                let is_header = rest[i] == "--fsm-file-header";
                if backend_name != "c" {
                    errors.push(
                        format!(
                            "{} is only valid for the c backend (the rust backend names its referenced fsm file with --fsm-file)",
                            rest[i]
                        ),
                    );
                } else if command == CodegenCommandKind::Production {
                    errors.push(format!(
                        "{} is only valid for the mc and test commands",
                        rest[i]
                    ));
                } else {
                    i += 1;
                    match rest.get(i) {
                        Some(v) => {
                            if is_header {
                                fsm_file_header = Some(v.clone());
                            } else {
                                fsm_file_source = Some(v.clone());
                            }
                        }
                        None => errors.push(format!("{} requires a file argument", rest[i - 1])),
                    }
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
            "--counterexample-format" => {
                i += 1;
                match rest
                    .get(i)
                    .and_then(|v| crate::codegen::mc_concepts::CounterexampleFormat::parse(v))
                {
                    Some(f) => counterexample_format = f,
                    None => errors
                        .push(crate::codegen::mc_concepts::COUNTEREXAMPLE_FORMAT_ERROR.to_string()),
                }
            }
            "--with-protocol-fsm" => {
                if command == CodegenCommandKind::ModelChecker {
                    errors.push(PROTOCOL_FSM_MC_ERROR.to_string());
                } else {
                    with_protocol_fsm = true;
                }
            }
            other if other.starts_with('-') => errors.push(format!("unknown option: {}", other)),
            other => errors.push(format!("unexpected extra argument: {}", other)),
        }
        i += 1;
    }

    // C form 2: the header/source pair must be given together, and it cannot
    // be combined with the form-1 --fsm-file stem.
    if fsm_file_header.is_some() != fsm_file_source.is_some() {
        errors.push(
            "--fsm-file-header and --fsm-file-source must be given together (the c form-2 fsm reference)"
                .into(),
        );
    }
    if fsm_file.is_some() && (fsm_file_header.is_some() || fsm_file_source.is_some()) {
        errors.push(
            "--fsm-file cannot be combined with --fsm-file-header/--fsm-file-source (pick the form-1 stem OR the form-2 pair)"
                .into(),
        );
    }

    finish(
        CodegenRequest::Generate {
            backend_name,
            command,
            input,
            base,
            out_fsm_code,
            out_fsm_header,
            out_mc_code,
            out_mc_header,
            out_test_code,
            out_fuzz_target,
            fsm_file,
            fsm_file_header,
            fsm_file_source,
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
