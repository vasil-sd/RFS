//! callback_minimal_adapters (Phase 17 task 7.3 example): the two
//! rfs-adapters never demonstrated before, driven against a REAL generated
//! spec. The callback driver (`rfs_adapters::callback::drive`) reactively
//! drives the generated spec-03 timeout/retry machine and reaches all three
//! of its outcomes (Completed / Failed / StepLimit); the no_std minimal
//! wrapper (`rfs_adapters::minimal::Minimal`) forwards steps to the same FSM
//! and composes with the callback driver.
//! Docs: dev:workspace-crates-overview, dev:workspace-crates-examples, llm:workspace-crates

// The generated spec-03 artifact (build.rs strips its inner attributes and
// writes the byte-identical body to OUT_DIR). `include!` cannot carry inner
// attributes, so the module-level `#[allow]` mirrors the artifact's own
// `#![allow(...)]` header line exactly.
#[allow(
    dead_code,
    unused_variables,
    unused_mut,
    unused_parens,
    irrefutable_let_patterns,
    non_camel_case_types,
    clippy::all
)]
pub mod timeout_retry {
    include!(concat!(env!("OUT_DIR"), "/03_timeout_retry_body.rs"));
}

use rfs_adapters::callback::{drive, CallbackResult};
use rfs_adapters::minimal::Minimal;
use std::vec::Vec;
use timeout_retry::{Fsm, Input, Output, StepError};

/// What the minimal-wrapper forwarding session observed.
#[derive(Debug)]
pub struct MinimalReport {
    /// Every output the wrapped FSM emitted across the forwarded steps, in order.
    pub outputs: Vec<Output>,
    /// An Ack with nothing in flight was rejected as an unexpected event.
    pub idle_ack_rejected: bool,
}

/// The reactive driver shared by the completion and composition sessions:
/// send once, then tick until a Drop output appears, then stop (None) - the
/// protocol is finished. The closure carries its own `sent` flag because the
/// Send step emits no output: an empty previous-outputs slice alone cannot
/// tell "before the first step" from "after the Send".
fn retry_until_drop() -> impl FnMut(&[Output]) -> Option<Input> {
    let mut sent = false;
    move |prev: &[Output]| {
        if prev.iter().any(|o| matches!(o, Output::Drop)) {
            None
        } else if !sent {
            sent = true;
            Some(Input::Send)
        } else {
            Some(Input::Tick)
        }
    }
}

/// Callback completion: the closure stops after observing Drop -
/// `CallbackResult::Completed(5)` (Send + four Ticks).
pub fn run_callback_completion() -> CallbackResult<StepError> {
    let mut f = Fsm::new();
    drive(&mut f, retry_until_drop(), 10)
}

/// Callback failure: an Ack with nothing in flight - the FSM rejects it and
/// the driver reports `Failed(UnexpectedEvent, 0)` (zero completed steps).
pub fn run_callback_failure() -> CallbackResult<StepError> {
    let mut f = Fsm::new();
    drive(&mut f, |_| Some(Input::Ack), 10)
}

/// Callback step limit: the closure never stops; the driver halts at
/// max_steps - `StepLimit(4)` (Send + three retry Ticks).
pub fn run_callback_step_limit() -> CallbackResult<StepError> {
    let mut f = Fsm::new();
    drive(&mut f, retry_until_drop(), 4)
}

/// Minimal forwarding: the wrapper passes steps straight through to the
/// wrapped FSM - same outputs, same unexpected-event rejection - and hands
/// the inner FSM back.
pub fn run_minimal_forwarding() -> Result<MinimalReport, String> {
    let mut m = Minimal::<Fsm>::new();
    let mut report = MinimalReport {
        outputs: Vec::new(),
        idle_ack_rejected: false,
    };

    for inp in [Input::Send, Input::Tick, Input::Tick, Input::Tick] {
        match m.step(inp) {
            Ok(outputs) => report.outputs.extend(outputs),
            Err(e) => return Err(format!("forwarded step must not error: {e:?}")),
        }
    }
    match m.step(Input::Tick) {
        Ok(outputs) => report.outputs.extend(outputs), // the expiry Tick emits Drop
        Err(e) => return Err(format!("expiry tick must not error: {e:?}")),
    }
    match m.step(Input::Ack) {
        Ok(outputs) => {
            return Err(format!(
                "idle ack must be rejected, got outputs {outputs:?}"
            ))
        }
        Err(StepError::UnexpectedEvent) => report.idle_ack_rejected = true,
        Err(e) => return Err(format!("wrong error kind: {e:?}")),
    }
    // The wrapper hands the inner FSM back untouched.
    let _inner: Fsm = m.into_inner();
    Ok(report)
}

/// Minimal composition: `Minimal<Fsm>` is itself a `ProtocolFsm`, so the
/// callback driver can drive it - the wrapper adds no behavior of its own
/// (the same Completed(5) as driving the bare FSM).
pub fn run_minimal_composition() -> CallbackResult<StepError> {
    let mut m = Minimal::<Fsm>::new();
    drive(&mut m, retry_until_drop(), 10)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The reactive driver completes the protocol: Send + four Ticks, stopping
    /// when it observes the Drop output.
    #[test]
    fn callback_completion_stops_after_drop() {
        assert_eq!(run_callback_completion(), CallbackResult::Completed(5));
    }

    /// An out-of-band Ack is rejected by the FSM; the driver reports the error
    /// with the number of steps completed before it (zero).
    #[test]
    fn callback_failure_reports_the_fsm_error() {
        assert_eq!(
            run_callback_failure(),
            CallbackResult::Failed(StepError::UnexpectedEvent, 0)
        );
    }

    /// A closure that never stops is halted at the step limit.
    #[test]
    fn callback_step_limit_halts_at_max_steps() {
        assert_eq!(run_callback_step_limit(), CallbackResult::StepLimit(4));
    }

    /// The minimal wrapper forwards every output and every error of the
    /// wrapped FSM, then hands the inner FSM back.
    #[test]
    fn minimal_forwards_outputs_and_errors() {
        let r = run_minimal_forwarding().expect("forwarding must complete");
        assert_eq!(
            r.outputs,
            [Output::Retry, Output::Retry, Output::Retry, Output::Drop]
        );
        assert!(r.idle_ack_rejected);
    }

    /// Adapters compose: driving a Minimal<Fsm> through the callback driver
    /// yields the same outcome as driving the bare FSM.
    #[test]
    fn minimal_composes_with_the_callback_driver() {
        assert_eq!(run_minimal_composition(), CallbackResult::Completed(5));
    }
}
