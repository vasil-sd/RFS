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
