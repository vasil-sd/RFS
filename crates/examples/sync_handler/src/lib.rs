//! sync_handler (Phase 5 task 11 example): drive the generated sliding-window
//! ARQ sender (spec 43) from another thread over std mpsc channels via the
//! rfs-adapters channel adapter.
//!
//! The environment (this thread) wakes the sender with a Tick, then keeps
//! ticking until all 8 frames are sent and acknowledged; every third tick it
//! injects an out-of-band `Ack` that the protocol must drop. The receiver
//! side observes only what crosses the channel: SendFrame/RecvAck outputs.

// The generated spec-43 artifact (build.rs strips its inner attributes and
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
pub mod arq {
    include!(concat!(env!("OUT_DIR"), "/43_sliding_window_body.rs"));
}

use arq::{Fsm, Input, Output};
use rfs_adapters::channel::{ChannelFsm, ChannelOutcome};

/// What the environment/receiver observed over the channel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArqReport {
    /// Sequence numbers of SendFrame outputs, in order.
    pub frames_sent: Vec<u8>,
    /// Sequence numbers of RecvAck outputs, in order.
    pub acks_received: Vec<u8>,
    /// Out-of-band Ack inputs that were dropped (no output).
    pub out_of_band_acks_dropped: usize,
    /// Total Tick inputs issued.
    pub ticks: usize,
    /// Maximum observed window (sent - acked) across the run.
    pub max_window: u8,
}

/// Drive one full ARQ session to completion over the channel adapter and
/// verify the observable protocol behavior.
pub fn run() -> Result<ArqReport, String> {
    let fsm = ChannelFsm::<Fsm>::spawn();
    let mut report = ArqReport {
        frames_sent: Vec::new(),
        acks_received: Vec::new(),
        out_of_band_acks_dropped: 0,
        ticks: 0,
        max_window: 0,
    };

    // Wake the sender: Idle -> Active (no output).
    tick(&fsm, &mut report.ticks)?;
    expect_no_outputs(recv(&fsm)?, "wake tick")?;

    // Run until all 8 frames are sent and acknowledged. Every third tick,
    // inject an out-of-band Ack that the protocol must drop.
    while report.frames_sent.len() < 8 || report.acks_received.len() < 8 {
        if report.ticks % 3 == 0 {
            fsm.send_input(Input::Ack_Seq {
                seq: 9,
                checksum: 0xDEAD_BEEF,
            })
            .map_err(|e| e.to_string())?;
            expect_no_outputs(recv(&fsm)?, "out-of-band ack")?;
            report.out_of_band_acks_dropped += 1;
        }
        tick(&fsm, &mut report.ticks)?;
        let outputs = take_outputs(recv(&fsm)?, "tick")?;
        for o in outputs {
            match o {
                Output::SendFrame_Frame { seq } => report.frames_sent.push(seq),
                Output::RecvAck_Acked { seq } => report.acks_received.push(seq),
            }
        }
        let window = (report.frames_sent.len() - report.acks_received.len()) as u8;
        report.max_window = report.max_window.max(window);
    }

    // Completion tick: next_seq == 8 && acked == 8 -> Done (no output).
    tick(&fsm, &mut report.ticks)?;
    expect_no_outputs(recv(&fsm)?, "completion tick")?;

    // Quiescence: in Done, further ticks produce no output.
    tick(&fsm, &mut report.ticks)?;
    expect_no_outputs(recv(&fsm)?, "quiescence tick")?;

    // Verify the observable protocol invariants.
    let expected: Vec<u8> = (0..8).collect();
    if report.frames_sent != expected {
        return Err(format!(
            "frames sent out of order: {:?}",
            report.frames_sent
        ));
    }
    if report.acks_received != expected {
        return Err(format!(
            "acks received out of order: {:?}",
            report.acks_received
        ));
    }
    if report.max_window > 4 {
        return Err(format!("window bound violated: max {}", report.max_window));
    }
    if report.out_of_band_acks_dropped == 0 {
        return Err("expected at least one dropped out-of-band ack".into());
    }
    Ok(report)
}

fn tick(fsm: &ChannelFsm<Fsm>, ticks: &mut usize) -> Result<(), String> {
    fsm.send_input(Input::Tick).map_err(|e| e.to_string())?;
    *ticks += 1;
    Ok(())
}

fn recv(fsm: &ChannelFsm<Fsm>) -> Result<ChannelOutcome<Fsm>, String> {
    fsm.recv_outcome().map_err(|e| e.to_string())
}

fn take_outputs(outcome: ChannelOutcome<Fsm>, what: &str) -> Result<Vec<Output>, String> {
    match outcome {
        ChannelOutcome::Outputs(o) => Ok(o),
        ChannelOutcome::Error(e) => Err(format!("{what}: FSM error {e:?}")),
    }
}

fn expect_no_outputs(outcome: ChannelOutcome<Fsm>, what: &str) -> Result<(), String> {
    let outputs = take_outputs(outcome, what)?;
    if outputs.is_empty() {
        Ok(())
    } else {
        Err(format!("{what}: unexpected outputs {outputs:?}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The full integration story as a test: the generated spec-43 FSM runs
    /// on its own thread, the driver completes one ARQ session over std mpsc
    /// channels, and every observable protocol invariant holds.
    #[test]
    fn arq_session_completes_over_channels() {
        let r = run().expect("ARQ session must complete");
        assert_eq!(r.frames_sent, (0..8).collect::<Vec<u8>>());
        assert_eq!(r.acks_received, (0..8).collect::<Vec<u8>>());
        assert!(r.max_window <= 4, "window bound: {}", r.max_window);
        assert!(r.out_of_band_acks_dropped >= 1);
    }
}
