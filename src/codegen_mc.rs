//! Target-language-agnostic concepts for model-checker code generation.
//!
//! Shared by all codegen backends (see `CODEGEN.md`). A backend's `mc` command
//! generates a bounded model checker: an executable that drives the generated
//! FSM with random input sequences up to a bound and reports property
//! violations, unexpected events, and assumption violations as counterexample
//! traces.
//!
//! This module defines the language-independent parts of that process:
//!
//! - the **run configuration** (`McConfig`) and its CLI wire format;
//! - the **forced-violation specification** format `label:prob:percent%`;
//! - the **sequence-generation strategy**: the ordered phases attempted at
//!   every step of a run (`McPhase`);
//! - the **per-run outcome taxonomy** (`McStatus`).
//!
//! Backends render these concepts into executable target-language code. The
//! generated binary must not depend on this crate, so backends embed their own
//! copies of the parsing helpers (kept in sync with the formats defined here).

use crate::codegen::AssumptionSelection;

/// One forced assumption-violation directive for a model-checking run.
#[derive(Debug, Clone)]
pub struct ViolationSpec {
    /// Label of the assumption to force violations of.
    pub label: String,
    /// Probability that this spec is active in a given run (0.0..=1.0).
    pub prob: f64,
    /// Target fraction of steps at which an active spec attempts a forced
    /// violation (percent, 0.0..=100.0).
    pub percent: f64,
}

impl ViolationSpec {
    /// Parse the CLI wire format `label` or `label:prob` or `label:prob:percent%`.
    /// Missing fields default to `prob = 1.0`, `percent = 100.0`.
    pub fn parse(s: &str) -> Self {
        let mut parts = s.splitn(3, ':');
        let label = parts.next().unwrap_or("").to_string();
        let prob = parts
            .next()
            .and_then(|x| x.parse::<f64>().ok())
            .unwrap_or(1.0);
        let percent = parts
            .next()
            .map(|x| x.trim_end_matches('%').parse::<f64>().unwrap_or(100.0))
            .unwrap_or(100.0);
        ViolationSpec {
            label,
            prob,
            percent,
        }
    }

    /// Re-render in the CLI wire format.
    pub fn to_wire(&self) -> String {
        format!("{}:{}:{}", self.label, self.prob, self.percent)
    }
}

/// Phases of the per-step sequence-generation strategy, in the order a model
/// checker must attempt them:
///
/// 1. `ForcedAssumptionViolation`: for each active violation spec, try to pick
///    a representative input that triggers that assumption's violation;
/// 2. `UnexpectedInjection`: with probability `unexpected_prob`, inject an
///    arbitrary representative input (may be unexpected);
/// 3. `AllowedRandomInput`: choose uniformly at random among the representative
///    inputs that are currently allowed by the FSM.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McPhase {
    ForcedAssumptionViolation,
    UnexpectedInjection,
    AllowedRandomInput,
}

impl McPhase {
    /// Strategy order: earlier phases take precedence at each step.
    pub const ALL: [McPhase; 3] = [
        McPhase::ForcedAssumptionViolation,
        McPhase::UnexpectedInjection,
        McPhase::AllowedRandomInput,
    ];
}

/// Outcome taxonomy for one bounded run (shared by all backends).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McStatus {
    /// All `bound` steps completed without failure.
    Completed,
    /// No representative input was allowed at some step (deadlock).
    Stuck,
    /// An unexpected event error occurred.
    UnexpectedEvent,
    /// An active assumption was violated and the handler stopped the run.
    AssumptionViolated,
}

/// Configuration of a bounded model-checking run.
#[derive(Debug, Clone)]
pub struct McConfig {
    /// Maximum sequence length (steps per run).
    pub bound: usize,
    /// Number of random runs.
    pub runs: usize,
    /// RNG seed (deterministic runs for the same configuration).
    pub seed: u64,
    /// Which assumptions are active during runs.
    pub assumes: AssumptionSelection,
    /// Forced violation directives (`--violate label:prob:percent%,...`).
    pub violations: Vec<ViolationSpec>,
    /// Probability of the unexpected-injection phase (0.0..=1.0).
    pub unexpected_prob: f64,
}

impl Default for McConfig {
    fn default() -> Self {
        Self {
            bound: 5,
            runs: 20,
            seed: 12345,
            assumes: AssumptionSelection::All,
            violations: Vec::new(),
            unexpected_prob: 0.0,
        }
    }
}

/// Parse an assumption-selection string (`all`, `none`, or a comma-separated
/// label list filtered against the known labels).
pub fn parse_assume_selection(s: &str, known: &[&str]) -> Vec<String> {
    match s.trim() {
        "all" => known.iter().map(|x| x.to_string()).collect(),
        "none" => Vec::new(),
        other => other
            .split(',')
            .map(|x| x.trim().to_string())
            .filter(|x| !x.is_empty() && known.contains(&x.as_str()))
            .collect(),
    }
}

// ---------------------------------------------------------------------------
// Exact reachable-state mode (Phase 4, D-P4-4/D-P4-7)
// ---------------------------------------------------------------------------

/// Outcome taxonomy of one exact model-checking run (shared by all backends).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExactStatus {
    /// The reachable graph was fully explored and no property was violated.
    PassComplete,
    /// A property violation or deadlock was found (see [`FailReason`]).
    Fail,
    /// The state limit (or timeout) was reached before completion; the result
    /// is incomplete. Never a completeness claim.
    UnknownStateLimit,
}

/// Why an exact run failed (D-P4-4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailReason {
    /// A safety property monitor confirmed a violation.
    PropertyViolation,
    /// A liveness obligation was violated (task 6+).
    LivenessViolation,
    /// A reachable configuration has no successor at all.
    Deadlock,
}

impl FailReason {
    /// Wire name in the summary line (`reason=<r>`).
    pub fn wire(&self) -> &'static str {
        match self {
            FailReason::PropertyViolation => "property-violation",
            FailReason::LivenessViolation => "liveness-violation",
            FailReason::Deadlock => "deadlock",
        }
    }
}

impl ExactStatus {
    /// Wire name in the summary line (`status=<s>`).
    pub fn wire(&self) -> &'static str {
        match self {
            ExactStatus::PassComplete => "pass-complete",
            ExactStatus::Fail => "fail",
            ExactStatus::UnknownStateLimit => "unknown-state-limit",
        }
    }

    /// Human-facing text (the phase's `PASS (complete)` / `FAIL` /
    /// `UNKNOWN (state limit reached)`).
    pub fn human(&self) -> &'static str {
        match self {
            ExactStatus::PassComplete => "PASS (complete)",
            ExactStatus::Fail => "FAIL",
            ExactStatus::UnknownStateLimit => "UNKNOWN (state limit reached)",
        }
    }

    /// Process exit code: 0 = pass-complete, 1 = fail, 2 = unknown.
    pub fn exit_code(&self) -> i32 {
        match self {
            ExactStatus::PassComplete => 0,
            ExactStatus::Fail => 1,
            ExactStatus::UnknownStateLimit => 2,
        }
    }
}

/// The machine-readable summary line (D-P4-4), exact format - parity-checked
/// across backends:
///
/// ```text
/// model-check summary: status=<s> states=N transitions=M max_depth=D
/// ```
/// plus, on fail, ` reason=<r> label=<L> step=S` (deadlock uses `label=-`
/// and the depth of the dead configuration as `step`).
pub fn exact_summary_line(
    status: ExactStatus,
    states: usize,
    transitions: usize,
    max_depth: usize,
    fail: Option<(FailReason, &str, usize)>,
) -> String {
    let mut s = format!(
        "model-check summary: status={} states={} transitions={} max_depth={}",
        status.wire(),
        states,
        transitions,
        max_depth
    );
    if let Some((reason, label, step)) = fail {
        s.push_str(&format!(
            " reason={} label={} step={}",
            reason.wire(),
            label,
            step
        ));
    }
    s
}

// ---------------------------------------------------------------------------
// Counterexample report format (Phase 5 task 10)
// ---------------------------------------------------------------------------

/// Report format of the exact-mode counterexample document. `Text` is the
/// parity-pinned human-readable default; `Json` emits ONE compact JSON
/// document carrying the same data (see [`McJsonReport`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CounterexampleFormat {
    /// The pinned text report (default; byte-identical across backends).
    Text,
    /// A single machine-readable JSON document (byte-identical across
    /// backends; key ordering is part of the contract - see below).
    Json,
}

impl CounterexampleFormat {
    /// Wire name for `--counterexample-format`.
    pub fn wire(&self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Json => "json",
        }
    }

    /// Parse the CLI value. Accepts exactly `text` or `json`.
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "text" => Some(Self::Text),
            "json" => Some(Self::Json),
            _ => None,
        }
    }
}

/// Pinned error message for an invalid `--counterexample-format` value.
pub const COUNTEREXAMPLE_FORMAT_ERROR: &str =
    "--counterexample-format requires a value (json|text)";

// ---------------------------------------------------------------------------
// JSON counterexample report document (Phase 5 task 10)
// ---------------------------------------------------------------------------
//
// KEY ORDERING IS PART OF THE BYTE-PARITY CONTRACT. The generated engines
// (Rust and C) must emit the document with EXACTLY this key sequence, in
// compact form (no whitespace outside strings), so both backends' output is
// byte-identical for the same run:
//
//   top level:      status, metrics, [reason, label, step], statistics,
//                   [counterexample]
//   metrics:        states, transitions, max_depth
//   statistics:     unexpected_terminations, monitor_size_bits,
//                   memory_estimate_bytes, peak_rss_kb
//   counterexample: kind, then per kind (ce_steps LAST - the trace-length
//                   quality metric: number of steps for a trace, prefix +
//                   cycle length for a lasso, depth for a deadlock config):
//     "trace":            initial_state, initial_vars, steps, ce_steps
//     "lasso":            initial_state, initial_vars, prefix, cycle,
//                         cycle_length, ce_steps
//     "deadlock-config":  depth, state, vars, ce_steps
//   step object:    input, outputs, from_state, to_state, vars
//   input/output:   variant, fields (fields in declaration order)
//
// `reason`/`label`/`step` and `counterexample` are present IFF status is
// "fail". Strings use the shared writer's escaping contract (json_rw.rs);
// integers are unsigned decimal (every value in this document is
// non-negative; bit<N> fields render as their unsigned value, matching the
// text report). The toolchain-side builder below renders the canonical
// document; the generated engines embed a minimal copy of the same writer
// and must produce byte-identical output for the same run data - EXCEPT
// `peak_rss_kb`, which is INFORMATIONAL (the actual peak resident set in kB,
// self-reported by the binary: VmHWM from /proc/self/status on Rust,
// ru_maxrss from getrusage(RUSAGE_SELF) on C; 0 on read failure).
// Cross-backend byte comparisons normalize it to 0 (tests/common/mc_json.rs),
// exactly like wall-clock times are never asserted.

/// Top-level field names (declaration order = wire order).
pub const JSON_KEY_STATUS: &str = "status";
pub const JSON_KEY_METRICS: &str = "metrics";
pub const JSON_KEY_STATES: &str = "states";
pub const JSON_KEY_TRANSITIONS: &str = "transitions";
pub const JSON_KEY_MAX_DEPTH: &str = "max_depth";
pub const JSON_KEY_REASON: &str = "reason";
pub const JSON_KEY_LABEL: &str = "label";
pub const JSON_KEY_STEP: &str = "step";
pub const JSON_KEY_STATISTICS: &str = "statistics";
pub const JSON_KEY_UNEXPECTED_TERMINATIONS: &str = "unexpected_terminations";
pub const JSON_KEY_MONITOR_SIZE_BITS: &str = "monitor_size_bits";
pub const JSON_KEY_MEMORY_ESTIMATE_BYTES: &str = "memory_estimate_bytes";
/// Informational (machine-dependent): actual peak RSS in kB, self-reported
/// by the MC binary. See the module docs for the normalization contract.
pub const JSON_KEY_PEAK_RSS_KB: &str = "peak_rss_kb";
pub const JSON_KEY_COUNTEREXAMPLE: &str = "counterexample";
/// Counterexample field names.
pub const JSON_KEY_KIND: &str = "kind";
pub const JSON_CE_KIND_TRACE: &str = "trace";
pub const JSON_CE_KIND_LASSO: &str = "lasso";
pub const JSON_CE_KIND_DEADLOCK_CONFIG: &str = "deadlock-config";
pub const JSON_KEY_INITIAL_STATE: &str = "initial_state";
pub const JSON_KEY_INITIAL_VARS: &str = "initial_vars";
pub const JSON_KEY_STEPS: &str = "steps";
pub const JSON_KEY_PREFIX: &str = "prefix";
pub const JSON_KEY_CYCLE: &str = "cycle";
pub const JSON_KEY_CYCLE_LENGTH: &str = "cycle_length";
pub const JSON_KEY_DEPTH: &str = "depth";
pub const JSON_KEY_STATE: &str = "state";
pub const JSON_KEY_VARS: &str = "vars";
pub const JSON_KEY_INPUT: &str = "input";
pub const JSON_KEY_OUTPUTS: &str = "outputs";
pub const JSON_KEY_FROM_STATE: &str = "from_state";
pub const JSON_KEY_TO_STATE: &str = "to_state";
pub const JSON_KEY_VARIANT: &str = "variant";
pub const JSON_KEY_FIELDS: &str = "fields";
/// Counterexample QUALITY metric (Phase 6 task 6): trace length in steps -
/// `steps.len()` for a trace, prefix + cycle length for a lasso, the depth
/// for a deadlock configuration. Last key of every counterexample object.
pub const JSON_KEY_CE_STEPS: &str = "ce_steps";

/// One input/output value in a JSON counterexample: the variant name plus
/// its fields in declaration order. Field values use the shared `Json`
/// model: bool -> Bool, bit<N> -> Int (unsigned decimal), enum -> Str
/// (variant name) - the same names the text report prints.
#[derive(Debug, Clone)]
pub struct JsonIo {
    pub variant: String,
    pub fields: Vec<(String, crate::json_rw::Json)>,
}

/// One step of a counterexample trace (mirrors one `step N` line of the
/// text report).
#[derive(Debug, Clone)]
pub struct JsonStep {
    pub input: JsonIo,
    pub outputs: Vec<JsonIo>,
    pub from_state: String,
    pub to_state: String,
    pub vars: Vec<(String, crate::json_rw::Json)>,
}

/// The counterexample payload (present iff the run failed).
#[derive(Debug, Clone)]
pub enum JsonCounterexample {
    /// Property violation (incl. sample-0): the full trace.
    Trace {
        initial_state: String,
        initial_vars: Vec<(String, crate::json_rw::Json)>,
        steps: Vec<JsonStep>,
    },
    /// Liveness violation: prefix + one cycle iteration; `cycle_length` is
    /// the number of steps in `cycle`.
    Lasso {
        initial_state: String,
        initial_vars: Vec<(String, crate::json_rw::Json)>,
        prefix: Vec<JsonStep>,
        cycle: Vec<JsonStep>,
        cycle_length: i64,
    },
    /// Deadlock: the terminal configuration (depth = its graph depth).
    DeadlockConfig {
        depth: i64,
        state: String,
        vars: Vec<(String, crate::json_rw::Json)>,
    },
}

/// One exact-mode run result in the JSON report's data model. The builder
/// renders the canonical document; both backends' generated engines must
/// produce byte-identical output for the same run.
#[derive(Debug, Clone)]
pub struct McJsonReport {
    pub status: ExactStatus,
    pub states: i64,
    pub transitions: i64,
    pub max_depth: i64,
    /// (reason, label, step) - present iff `status` is [`ExactStatus::Fail`].
    pub fail: Option<(FailReason, String, i64)>,
    pub unexpected_terminations: i64,
    pub monitor_size_bits: i64,
    pub memory_estimate_bytes: i64,
    /// INFORMATIONAL (machine-dependent): actual peak RSS in kB, self-reported
    /// by the MC binary (0 on read failure / for the in-process reference MC).
    /// Cross-backend byte comparisons normalize it to 0.
    pub peak_rss_kb: i64,
    /// Present iff `status` is [`ExactStatus::Fail`].
    pub counterexample: Option<JsonCounterexample>,
}

fn json_io(io: &JsonIo) -> crate::json_rw::Json {
    use crate::json_rw::Json;
    Json::obj(vec![
        (JSON_KEY_VARIANT.to_string(), Json::str(&io.variant)),
        (JSON_KEY_FIELDS.to_string(), Json::obj(io.fields.clone())),
    ])
}

fn json_step(step: &JsonStep) -> crate::json_rw::Json {
    use crate::json_rw::Json;
    Json::obj(vec![
        (JSON_KEY_INPUT.to_string(), json_io(&step.input)),
        (
            JSON_KEY_OUTPUTS.to_string(),
            Json::arr(step.outputs.iter().map(json_io).collect()),
        ),
        (JSON_KEY_FROM_STATE.to_string(), Json::str(&step.from_state)),
        (JSON_KEY_TO_STATE.to_string(), Json::str(&step.to_state)),
        (JSON_KEY_VARS.to_string(), Json::obj(step.vars.clone())),
    ])
}

impl JsonCounterexample {
    /// Counterexample QUALITY metric (Phase 6 task 6): trace length in steps
    /// - `steps.len()` for a trace, prefix + cycle length for a lasso,
    ///   the depth for a deadlock configuration. Deterministic; part of the
    ///   byte-parity contract (unlike `peak_rss_kb`).
    pub fn ce_steps(&self) -> i64 {
        match self {
            JsonCounterexample::Trace { steps, .. } => steps.len() as i64,
            JsonCounterexample::Lasso { prefix, cycle, .. } => (prefix.len() + cycle.len()) as i64,
            JsonCounterexample::DeadlockConfig { depth, .. } => *depth,
        }
    }

    /// Render the counterexample object (key order per the module docs;
    /// `ce_steps` is always last).
    pub fn to_json(&self) -> crate::json_rw::Json {
        use crate::json_rw::Json;
        match self {
            JsonCounterexample::Trace {
                initial_state,
                initial_vars,
                steps,
            } => Json::obj(vec![
                (JSON_KEY_KIND.to_string(), Json::str(JSON_CE_KIND_TRACE)),
                (JSON_KEY_INITIAL_STATE.to_string(), Json::str(initial_state)),
                (
                    JSON_KEY_INITIAL_VARS.to_string(),
                    Json::obj(initial_vars.clone()),
                ),
                (
                    JSON_KEY_STEPS.to_string(),
                    Json::arr(steps.iter().map(json_step).collect()),
                ),
                (JSON_KEY_CE_STEPS.to_string(), Json::int(self.ce_steps())),
            ]),
            JsonCounterexample::Lasso {
                initial_state,
                initial_vars,
                prefix,
                cycle,
                cycle_length,
            } => Json::obj(vec![
                (JSON_KEY_KIND.to_string(), Json::str(JSON_CE_KIND_LASSO)),
                (JSON_KEY_INITIAL_STATE.to_string(), Json::str(initial_state)),
                (
                    JSON_KEY_INITIAL_VARS.to_string(),
                    Json::obj(initial_vars.clone()),
                ),
                (
                    JSON_KEY_PREFIX.to_string(),
                    Json::arr(prefix.iter().map(json_step).collect()),
                ),
                (
                    JSON_KEY_CYCLE.to_string(),
                    Json::arr(cycle.iter().map(json_step).collect()),
                ),
                (JSON_KEY_CYCLE_LENGTH.to_string(), Json::int(*cycle_length)),
                (JSON_KEY_CE_STEPS.to_string(), Json::int(self.ce_steps())),
            ]),
            JsonCounterexample::DeadlockConfig { depth, state, vars } => Json::obj(vec![
                (
                    JSON_KEY_KIND.to_string(),
                    Json::str(JSON_CE_KIND_DEADLOCK_CONFIG),
                ),
                (JSON_KEY_DEPTH.to_string(), Json::int(*depth)),
                (JSON_KEY_STATE.to_string(), Json::str(state)),
                (JSON_KEY_VARS.to_string(), Json::obj(vars.clone())),
                (JSON_KEY_CE_STEPS.to_string(), Json::int(self.ce_steps())),
            ]),
        }
    }
}

impl McJsonReport {
    /// Build the canonical JSON document (key order per the module docs).
    pub fn to_json(&self) -> crate::json_rw::Json {
        use crate::json_rw::Json;
        let mut pairs: Vec<(String, Json)> = vec![
            (JSON_KEY_STATUS.to_string(), Json::str(self.status.wire())),
            (
                JSON_KEY_METRICS.to_string(),
                Json::obj(vec![
                    (JSON_KEY_STATES.to_string(), Json::int(self.states)),
                    (
                        JSON_KEY_TRANSITIONS.to_string(),
                        Json::int(self.transitions),
                    ),
                    (JSON_KEY_MAX_DEPTH.to_string(), Json::int(self.max_depth)),
                ]),
            ),
        ];
        if let Some((reason, label, step)) = &self.fail {
            pairs.push((JSON_KEY_REASON.to_string(), Json::str(reason.wire())));
            pairs.push((JSON_KEY_LABEL.to_string(), Json::str(label)));
            pairs.push((JSON_KEY_STEP.to_string(), Json::int(*step)));
        }
        pairs.push((
            JSON_KEY_STATISTICS.to_string(),
            Json::obj(vec![
                (
                    JSON_KEY_UNEXPECTED_TERMINATIONS.to_string(),
                    Json::int(self.unexpected_terminations),
                ),
                (
                    JSON_KEY_MONITOR_SIZE_BITS.to_string(),
                    Json::int(self.monitor_size_bits),
                ),
                (
                    JSON_KEY_MEMORY_ESTIMATE_BYTES.to_string(),
                    Json::int(self.memory_estimate_bytes),
                ),
                (
                    JSON_KEY_PEAK_RSS_KB.to_string(),
                    Json::int(self.peak_rss_kb),
                ),
            ]),
        ));
        if let Some(ce) = &self.counterexample {
            pairs.push((JSON_KEY_COUNTEREXAMPLE.to_string(), ce.to_json()));
        }
        Json::obj(pairs)
    }

    /// Compact rendering (the wire form the generated engines print).
    pub fn to_json_string(&self) -> String {
        self.to_json().to_string_compact()
    }
}

/// Codegen-time defaults for the exact-mode runtime flags (D-P4-7). Embedded
/// as constants in the generated binary; overridable at runtime.
pub const EXACT_DEFAULT_MAX_STATES: usize = 1_000_000;
/// `--timeout-sec` default: 0 = off. Wall-clock only, NOT parity-guaranteed,
/// never used by parity/benchmark assertions.
pub const EXACT_DEFAULT_TIMEOUT_SEC: u64 = 0;
/// `--threads` default: 1 (sequential). N > 1 enables level-parallel
/// expansion with canonical commit - results stay byte-identical to N = 1
/// (task 10B); `--fast` is a runtime-only non-parity flag.
pub const EXACT_DEFAULT_THREADS: usize = 1;
/// SMT-mode unrolling bound default (task 11): k steps. The generated binary
/// also accepts runtime `--bound N`. Small by design - SMT mode is bug hunting
/// on larger bitvectors, not a completeness tool.
pub const SMT_DEFAULT_BOUND: usize = 10;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn violation_spec_wire_format() {
        let v = ViolationSpec::parse("no_starve");
        assert_eq!(v.label, "no_starve");
        assert_eq!(v.prob, 1.0);
        assert_eq!(v.percent, 100.0);
        let v = ViolationSpec::parse("no_starve:0.5:25%");
        assert!((v.prob - 0.5).abs() < 1e-9);
        assert!((v.percent - 25.0).abs() < 1e-9);
        assert_eq!(v.to_wire(), "no_starve:0.5:25");
    }

    #[test]
    fn assume_selection_parsing() {
        let known = ["a", "b"];
        assert_eq!(parse_assume_selection("all", &known), vec!["a", "b"]);
        assert!(parse_assume_selection("none", &known).is_empty());
        assert_eq!(parse_assume_selection("b,zzz,a", &known), vec!["b", "a"]);
    }

    #[test]
    fn default_config_matches_documented_defaults() {
        let c = McConfig::default();
        assert_eq!(c.bound, 5);
        assert_eq!(c.runs, 20);
        assert_eq!(c.seed, 12345);
        assert_eq!(McPhase::ALL.len(), 3);
    }

    #[test]
    fn exact_summary_line_format() {
        assert_eq!(
            exact_summary_line(ExactStatus::PassComplete, 123, 456, 7, None),
            "model-check summary: status=pass-complete states=123 transitions=456 max_depth=7"
        );
        assert_eq!(
            exact_summary_line(
                ExactStatus::Fail,
                10,
                20,
                3,
                Some((FailReason::PropertyViolation, "p", 5))
            ),
            "model-check summary: status=fail states=10 transitions=20 max_depth=3 reason=property-violation label=p step=5"
        );
        assert_eq!(
            exact_summary_line(
                ExactStatus::Fail,
                10,
                20,
                4,
                Some((FailReason::Deadlock, "-", 4))
            ),
            "model-check summary: status=fail states=10 transitions=20 max_depth=4 reason=deadlock label=- step=4"
        );
    }

    #[test]
    fn counterexample_format_wire_and_parse() {
        assert_eq!(CounterexampleFormat::Text.wire(), "text");
        assert_eq!(CounterexampleFormat::Json.wire(), "json");
        assert_eq!(
            CounterexampleFormat::parse("text"),
            Some(CounterexampleFormat::Text)
        );
        assert_eq!(
            CounterexampleFormat::parse("json"),
            Some(CounterexampleFormat::Json)
        );
        assert_eq!(CounterexampleFormat::parse("JSON"), None);
        assert_eq!(CounterexampleFormat::parse(""), None);
        assert_eq!(
            COUNTEREXAMPLE_FORMAT_ERROR,
            "--counterexample-format requires a value (json|text)"
        );
    }

    /// One synthetic trace step exercising all three field types.
    fn step(input: &str, seq: i64) -> JsonStep {
        use crate::json_rw::Json;
        JsonStep {
            input: JsonIo {
                variant: input.to_string(),
                fields: vec![("seq".to_string(), Json::int(seq))],
            },
            outputs: vec![JsonIo {
                variant: "Ack".to_string(),
                fields: vec![("ok".to_string(), Json::bool(true))],
            }],
            from_state: "A".to_string(),
            to_state: "B".to_string(),
            vars: vec![
                ("flag".to_string(), Json::bool(false)),
                ("seq".to_string(), Json::int(seq)),
                ("mode".to_string(), Json::str("Idle")),
            ],
        }
    }

    #[test]
    fn json_report_pass_complete_shape() {
        let r = McJsonReport {
            status: ExactStatus::PassComplete,
            states: 4,
            transitions: 3,
            max_depth: 2,
            fail: None,
            unexpected_terminations: 0,
            monitor_size_bits: 16,
            memory_estimate_bytes: 208,
            peak_rss_kb: 1024,
            counterexample: None,
        };
        assert_eq!(
            r.to_json_string(),
            "{\"status\":\"pass-complete\",\"metrics\":{\"states\":4,\"transitions\":3,\"max_depth\":2},\"statistics\":{\"unexpected_terminations\":0,\"monitor_size_bits\":16,\"memory_estimate_bytes\":208,\"peak_rss_kb\":1024}}"
        );
    }

    #[test]
    fn json_report_unknown_shape() {
        let r = McJsonReport {
            status: ExactStatus::UnknownStateLimit,
            states: 100,
            transitions: 99,
            max_depth: 50,
            fail: None,
            unexpected_terminations: 2,
            monitor_size_bits: 8,
            memory_estimate_bytes: 4000,
            peak_rss_kb: 2048,
            counterexample: None,
        };
        assert_eq!(
            r.to_json_string(),
            "{\"status\":\"unknown-state-limit\",\"metrics\":{\"states\":100,\"transitions\":99,\"max_depth\":50},\"statistics\":{\"unexpected_terminations\":2,\"monitor_size_bits\":8,\"memory_estimate_bytes\":4000,\"peak_rss_kb\":2048}}"
        );
    }

    #[test]
    fn json_report_fail_property_shape() {
        let r = McJsonReport {
            status: ExactStatus::Fail,
            states: 7,
            transitions: 6,
            max_depth: 3,
            fail: Some((FailReason::PropertyViolation, "no_bad".to_string(), 2)),
            unexpected_terminations: 1,
            monitor_size_bits: 24,
            memory_estimate_bytes: 504,
            peak_rss_kb: 3072,
            counterexample: Some(JsonCounterexample::Trace {
                initial_state: "A".to_string(),
                initial_vars: vec![("flag".to_string(), crate::json_rw::Json::bool(true))],
                steps: vec![step("Req", 0), step("Ack", 1)],
            }),
        };
        assert_eq!(
            r.to_json_string(),
            "{\"status\":\"fail\",\"metrics\":{\"states\":7,\"transitions\":6,\"max_depth\":3},\"reason\":\"property-violation\",\"label\":\"no_bad\",\"step\":2,\"statistics\":{\"unexpected_terminations\":1,\"monitor_size_bits\":24,\"memory_estimate_bytes\":504,\"peak_rss_kb\":3072},\"counterexample\":{\"kind\":\"trace\",\"initial_state\":\"A\",\"initial_vars\":{\"flag\":true},\"steps\":[{\"input\":{\"variant\":\"Req\",\"fields\":{\"seq\":0}},\"outputs\":[{\"variant\":\"Ack\",\"fields\":{\"ok\":true}}],\"from_state\":\"A\",\"to_state\":\"B\",\"vars\":{\"flag\":false,\"seq\":0,\"mode\":\"Idle\"}},{\"input\":{\"variant\":\"Ack\",\"fields\":{\"seq\":1}},\"outputs\":[{\"variant\":\"Ack\",\"fields\":{\"ok\":true}}],\"from_state\":\"A\",\"to_state\":\"B\",\"vars\":{\"flag\":false,\"seq\":1,\"mode\":\"Idle\"}}],\"ce_steps\":2}}"
        );
    }

    #[test]
    fn json_report_fail_liveness_shape() {
        let r = McJsonReport {
            status: ExactStatus::Fail,
            states: 3,
            transitions: 2,
            max_depth: 1,
            fail: Some((FailReason::LivenessViolation, "ev".to_string(), 0)),
            unexpected_terminations: 0,
            monitor_size_bits: 4,
            memory_estimate_bytes: 120,
            peak_rss_kb: 512,
            counterexample: Some(JsonCounterexample::Lasso {
                initial_state: "A".to_string(),
                initial_vars: vec![],
                prefix: vec![],
                cycle: vec![step("Tick", 0), step("Tick", 1)],
                cycle_length: 2,
            }),
        };
        assert_eq!(
            r.to_json_string(),
            "{\"status\":\"fail\",\"metrics\":{\"states\":3,\"transitions\":2,\"max_depth\":1},\"reason\":\"liveness-violation\",\"label\":\"ev\",\"step\":0,\"statistics\":{\"unexpected_terminations\":0,\"monitor_size_bits\":4,\"memory_estimate_bytes\":120,\"peak_rss_kb\":512},\"counterexample\":{\"kind\":\"lasso\",\"initial_state\":\"A\",\"initial_vars\":{},\"prefix\":[],\"cycle\":[{\"input\":{\"variant\":\"Tick\",\"fields\":{\"seq\":0}},\"outputs\":[{\"variant\":\"Ack\",\"fields\":{\"ok\":true}}],\"from_state\":\"A\",\"to_state\":\"B\",\"vars\":{\"flag\":false,\"seq\":0,\"mode\":\"Idle\"}},{\"input\":{\"variant\":\"Tick\",\"fields\":{\"seq\":1}},\"outputs\":[{\"variant\":\"Ack\",\"fields\":{\"ok\":true}}],\"from_state\":\"A\",\"to_state\":\"B\",\"vars\":{\"flag\":false,\"seq\":1,\"mode\":\"Idle\"}}],\"cycle_length\":2,\"ce_steps\":2}}"
        );
    }

    #[test]
    fn json_report_fail_deadlock_shape() {
        let r = McJsonReport {
            status: ExactStatus::Fail,
            states: 4,
            transitions: 3,
            max_depth: 2,
            fail: Some((FailReason::Deadlock, "-".to_string(), 2)),
            unexpected_terminations: 0,
            monitor_size_bits: 0,
            memory_estimate_bytes: 160,
            peak_rss_kb: 256,
            counterexample: Some(JsonCounterexample::DeadlockConfig {
                depth: 2,
                state: "Term".to_string(),
                vars: vec![("seq".to_string(), crate::json_rw::Json::int(9))],
            }),
        };
        assert_eq!(
            r.to_json_string(),
            "{\"status\":\"fail\",\"metrics\":{\"states\":4,\"transitions\":3,\"max_depth\":2},\"reason\":\"deadlock\",\"label\":\"-\",\"step\":2,\"statistics\":{\"unexpected_terminations\":0,\"monitor_size_bits\":0,\"memory_estimate_bytes\":160,\"peak_rss_kb\":256},\"counterexample\":{\"kind\":\"deadlock-config\",\"depth\":2,\"state\":\"Term\",\"vars\":{\"seq\":9},\"ce_steps\":2}}"
        );
    }

    #[test]
    fn json_report_escaping_contract() {
        // The writer's escaping contract applies to every string field
        // (labels, variant/state names): quote, backslash and control chars.
        let r = McJsonReport {
            status: ExactStatus::Fail,
            states: 1,
            transitions: 0,
            max_depth: 0,
            fail: Some((
                FailReason::PropertyViolation,
                "a\"b\\c\u{01}d".to_string(),
                0,
            )),
            unexpected_terminations: 0,
            monitor_size_bits: 0,
            memory_estimate_bytes: 32,
            peak_rss_kb: 64,
            counterexample: Some(JsonCounterexample::DeadlockConfig {
                depth: 0,
                state: "S\t1".to_string(),
                vars: vec![],
            }),
        };
        assert_eq!(
            r.to_json_string(),
            "{\"status\":\"fail\",\"metrics\":{\"states\":1,\"transitions\":0,\"max_depth\":0},\"reason\":\"property-violation\",\"label\":\"a\\\"b\\\\c\\u0001d\",\"step\":0,\"statistics\":{\"unexpected_terminations\":0,\"monitor_size_bits\":0,\"memory_estimate_bytes\":32,\"peak_rss_kb\":64},\"counterexample\":{\"kind\":\"deadlock-config\",\"depth\":0,\"state\":\"S\\t1\",\"vars\":{},\"ce_steps\":0}}"
        );
    }

    #[test]
    fn exact_status_wire_human_exit() {
        assert_eq!(ExactStatus::PassComplete.wire(), "pass-complete");
        assert_eq!(ExactStatus::Fail.wire(), "fail");
        assert_eq!(ExactStatus::UnknownStateLimit.wire(), "unknown-state-limit");
        assert_eq!(ExactStatus::PassComplete.human(), "PASS (complete)");
        assert_eq!(ExactStatus::Fail.human(), "FAIL");
        assert_eq!(
            ExactStatus::UnknownStateLimit.human(),
            "UNKNOWN (state limit reached)"
        );
        assert_eq!(ExactStatus::PassComplete.exit_code(), 0);
        assert_eq!(ExactStatus::Fail.exit_code(), 1);
        assert_eq!(ExactStatus::UnknownStateLimit.exit_code(), 2);
        assert_eq!(EXACT_DEFAULT_MAX_STATES, 1_000_000);
        assert_eq!(EXACT_DEFAULT_TIMEOUT_SEC, 0);
    }
}
