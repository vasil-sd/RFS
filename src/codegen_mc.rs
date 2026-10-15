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
}
