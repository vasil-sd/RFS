//! monitors_production (Phase 17 task 7.2 example): production codegen with
//! `--with-property-monitors`. The generated spec-41 FSM carries online
//! monitors for all five properties; in debug builds a violating environment
//! sequence surfaces the violation at the step API (`StepError::
//! PropertyViolation`), and in release builds the monitor code is compiled
//! out entirely (zero runtime cost).
//! Docs: dev:workspace-crates-overview, dev:workspace-crates-examples, llm:workspace-crates

// The generated spec-41 artifact (build.rs strips its inner attributes and
// writes the byte-identical body to OUT_DIR). `include!` cannot carry inner
// attributes, so the module-level `#[allow]` mirrors the artifact's own
// `#![allow(...)]` header line exactly - plus `unused_comparisons`, which
// the generated bounded-window monitor code (`always[10]`) trips on with an
// always-true `idx as u64 >= 0` comparison.
#[allow(
    dead_code,
    unused_variables,
    unused_mut,
    unused_parens,
    irrefutable_let_patterns,
    non_camel_case_types,
    clippy::all,
    unused_comparisons
)]
pub mod property_monitors {
    include!(concat!(env!("OUT_DIR"), "/41_property_monitors_body.rs"));
}

use property_monitors::{Fsm, Input, Output};

/// The outcome of driving one session with the online monitors active.
#[derive(Debug)]
pub struct MonitorReport {
    /// Steps that completed without a monitor violation.
    pub ok_steps: usize,
    /// Send(Ack) outputs emitted by the protocol (the Acked transition).
    pub acks_emitted: usize,
    /// The violated property's label (None when the monitors stayed quiet).
    pub violation_label: Option<String>,
    /// The sample at which the violation was confirmed (None when quiet).
    pub violation_step: Option<usize>,
}

/// Drive the protocol's happy path: Send(Fin), Recv(Ack), Tick -> Closed.
/// All five monitors must stay quiet, and the Acked transition must emit
/// Send(Ack).
pub fn run_correct_session() -> Result<MonitorReport, String> {
    let mut f = Fsm::new();
    let mut report = MonitorReport {
        ok_steps: 0,
        acks_emitted: 0,
        violation_label: None,
        violation_step: None,
    };

    for inp in [Input::Send_Fin, Input::Recv_Ack, Input::Tick] {
        match f.step(inp) {
            Ok(outputs) => {
                report.ok_steps += 1;
                report.acks_emitted += outputs
                    .iter()
                    .filter(|o| matches!(o, Output::Send_Ack))
                    .count();
            }
            Err(e) => return Err(format!("happy path must not error: {e:?}")),
        }
    }
    Ok(report)
}

/// Drive the violating environment sequence: Send(Fin) then four Ticks with
/// no Ack ever arriving. `fin_acked_bounded` (every Fin answered by an Ack
/// within 5 samples) triggers at sample 1 and expires at sample 5 - steps
/// 1-4 are Ok, step 5 surfaces the violation at the step API. Debug builds
/// only: the `PropertyViolation` variant is `#[cfg(debug_assertions)]`.
#[cfg(debug_assertions)]
pub fn run_violating_session() -> MonitorReport {
    use property_monitors::StepError;

    let mut f = Fsm::new();
    let mut report = MonitorReport {
        ok_steps: 0,
        acks_emitted: 0,
        violation_label: None,
        violation_step: None,
    };

    for inp in [
        Input::Send_Fin,
        Input::Tick,
        Input::Tick,
        Input::Tick,
        Input::Tick,
    ] {
        match f.step(inp) {
            Ok(outputs) => {
                report.ok_steps += 1;
                report.acks_emitted += outputs
                    .iter()
                    .filter(|o| matches!(o, Output::Send_Ack))
                    .count();
            }
            Err(StepError::PropertyViolation { label, step }) => {
                report.violation_label = Some(label);
                report.violation_step = Some(step);
                break;
            }
            Err(e) => panic!("expected a property violation, got {e:?}"),
        }
    }
    report
}

/// Release-build twin of the violating sequence: with the monitors compiled
/// out, the same five inputs all step Ok - zero runtime cost for the
/// properties.
#[cfg(not(debug_assertions))]
pub fn run_unmonitored_session() -> MonitorReport {
    let mut f = Fsm::new();
    let mut report = MonitorReport {
        ok_steps: 0,
        acks_emitted: 0,
        violation_label: None,
        violation_step: None,
    };

    for inp in [
        Input::Send_Fin,
        Input::Tick,
        Input::Tick,
        Input::Tick,
        Input::Tick,
    ] {
        match f.step(inp) {
            Ok(outputs) => {
                report.ok_steps += 1;
                report.acks_emitted += outputs
                    .iter()
                    .filter(|o| matches!(o, Output::Send_Ack))
                    .count();
            }
            Err(e) => panic!("release build must not surface monitor violations: {e:?}"),
        }
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The happy path keeps all five monitors quiet and emits the protocol's
    /// Send(Ack).
    #[test]
    fn correct_session_keeps_monitors_quiet() {
        let r = run_correct_session().expect("happy path must not violate");
        assert_eq!(r.ok_steps, 3);
        assert_eq!(r.acks_emitted, 1);
        assert_eq!(r.violation_label, None);
        assert_eq!(r.violation_step, None);
    }

    /// The violating environment sequence (Fin, then four Ticks with no Ack)
    /// surfaces `fin_acked_bounded` at sample 5 - the same label/step the MC
    /// counterexample reports.
    #[cfg(debug_assertions)]
    #[test]
    fn violating_session_surfaces_property_violation() {
        let r = run_violating_session();
        assert_eq!(r.ok_steps, 4);
        assert_eq!(r.violation_label.as_deref(), Some("fin_acked_bounded"));
        assert_eq!(r.violation_step, Some(5));
    }

    /// Release build: the same violating sequence steps all Ok - the monitors
    /// are compiled out.
    #[cfg(not(debug_assertions))]
    #[test]
    fn release_build_compiles_monitors_out() {
        let r = run_unmonitored_session();
        assert_eq!(r.ok_steps, 5);
        assert_eq!(r.violation_label, None);
        assert_eq!(r.violation_step, None);
    }
}
