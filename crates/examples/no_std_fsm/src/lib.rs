//! no_std_fsm (Phase 17 task 7.1 example): production codegen with
//! `--no-std`. The crate lib is `#![no_std]` + `extern crate alloc` and
//! includes the generated spec-03 timeout/retry machine byte-identically;
//! a test drives a full session through that no_std artifact, proving the
//! public API is identical to the std build.
//! Docs: dev:workspace-crates-overview, dev:workspace-crates-examples, llm:workspace-crates

#![no_std]

extern crate alloc;

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

use alloc::format;
use alloc::string::String;
use timeout_retry::{Fsm, Input, Output, StepError};

/// What the driver observed over one full session.
#[derive(Debug)]
pub struct SessionReport {
    /// Retry outputs emitted while the timer counted down.
    pub retries: usize,
    /// Drop outputs emitted when the timer expired.
    pub drops: usize,
    /// An Ack with nothing in flight was rejected as an unexpected event.
    pub idle_ack_rejected: bool,
    /// The resend + ack success path completed without error.
    pub ack_after_resend_accepted: bool,
}

/// Drive one full timeout/retry session through the no_std artifact using
/// only its inherent API (`Fsm::new()` + `step`): Send, three retry Ticks,
/// the expiring Tick (Drop), an out-of-band Ack that must be rejected, and a
/// resend + Ack success path.
pub fn run_session() -> Result<SessionReport, String> {
    let mut f = Fsm::new();
    let mut report = SessionReport {
        retries: 0,
        drops: 0,
        idle_ack_rejected: false,
        ack_after_resend_accepted: false,
    };

    expect_outputs(&mut f, Input::Send, &[], "send")?;
    for _ in 0..3 {
        expect_outputs(&mut f, Input::Tick, &[Output::Retry], "retry tick")?;
        report.retries += 1;
    }
    expect_outputs(&mut f, Input::Tick, &[Output::Drop], "expiry tick")?;
    report.drops += 1;

    // An Ack with nothing in flight has no enabled transition: the no_std
    // artifact rejects it at the step API exactly like the std build.
    expect_unexpected_event(&mut f, Input::Ack, "idle ack")?;
    report.idle_ack_rejected = true;

    // Resend and complete: the success path works through the no_std artifact.
    expect_outputs(&mut f, Input::Send, &[], "resend")?;
    expect_outputs(&mut f, Input::Ack, &[], "ack")?;
    report.ack_after_resend_accepted = true;

    Ok(report)
}

fn expect_outputs(
    fsm: &mut Fsm,
    input: Input,
    expected: &[Output],
    what: &str,
) -> Result<(), String> {
    match fsm.step(input) {
        Ok(outputs) => {
            if outputs.as_slice() == expected {
                Ok(())
            } else {
                Err(format!(
                    "{what}: expected outputs {expected:?}, got {outputs:?}"
                ))
            }
        }
        Err(e) => Err(format!("{what}: unexpected step error {e:?}")),
    }
}

fn expect_unexpected_event(fsm: &mut Fsm, input: Input, what: &str) -> Result<(), String> {
    match fsm.step(input) {
        Ok(outputs) => Err(format!(
            "{what}: expected an unexpected-event error, got outputs {outputs:?}"
        )),
        Err(StepError::UnexpectedEvent) => Ok(()),
        Err(e) => Err(format!("{what}: wrong step error kind {e:?}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A full session through the `#![no_std]` artifact: the public API is
    /// identical to the std build (same outputs, same unexpected-event
    /// rejection), driven here from a plain test context.
    #[test]
    fn full_session_through_nostd_artifact() {
        let r = run_session().expect("session must complete");
        assert_eq!(r.retries, 3);
        assert_eq!(r.drops, 1);
        assert!(r.idle_ack_rejected);
        assert!(r.ack_after_resend_accepted);
    }
}
