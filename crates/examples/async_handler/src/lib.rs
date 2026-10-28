//! async_handler (Phase 5 task 11 example): drive the generated sliding-window
//! ARQ sender (spec 43) as a futures Stream via the rfs-adapters async adapter.
//!
//! The environment submits inputs with `AsyncFsm::submit` and consumes
//! outcomes from the stream; the same protocol scenario as sync_handler -
//! wake tick, then ticks until all 8 frames are sent and acknowledged, an
//! out-of-band `Ack` injected every third tick, completion + quiescence.
//! Docs: dev:workspace-crates-overview, llm:workspace-crates

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
use futures::stream::StreamExt;
use rfs_adapters::async_adapter::{AsyncFsm, AsyncOutcome};

/// What the environment/receiver observed from the stream.
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

/// Drive one full ARQ session to completion over the futures stream adapter
/// and verify the observable protocol behavior. The FSM is stepped
/// synchronously inside `poll_next`, so the plain futures executor suffices
/// (no tokio).
pub fn run() -> Result<ArqReport, String> {
    futures::executor::block_on(async {
        let mut fsm = AsyncFsm::<Fsm>::new();
        let mut report = ArqReport {
            frames_sent: Vec::new(),
            acks_received: Vec::new(),
            out_of_band_acks_dropped: 0,
            ticks: 0,
            max_window: 0,
        };

        // Wake the sender: Idle -> Active (no output).
        report.ticks += 1;
        expect_no_outputs(step(&mut fsm, Input::Tick).await?, "wake tick")?;

        // Run until all 8 frames are sent and acknowledged. Every third
        // tick, inject an out-of-band Ack that the protocol must drop.
        while report.frames_sent.len() < 8 || report.acks_received.len() < 8 {
            if report.ticks % 3 == 0 {
                expect_no_outputs(
                    step(
                        &mut fsm,
                        Input::Ack_Seq {
                            seq: 9,
                            checksum: 0xDEAD_BEEF,
                        },
                    )
                    .await?,
                    "out-of-band ack",
                )?;
                report.out_of_band_acks_dropped += 1;
            }
            report.ticks += 1;
            let outputs = step(&mut fsm, Input::Tick).await?;
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
        report.ticks += 1;
        expect_no_outputs(step(&mut fsm, Input::Tick).await?, "completion tick")?;

        // Quiescence: in Done, further ticks produce no output.
        report.ticks += 1;
        expect_no_outputs(step(&mut fsm, Input::Tick).await?, "quiescence tick")?;

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
    })
}

/// Submit one input and await exactly the outcome it produces (the stream
/// steps one queued input per `poll_next`).
async fn step(fsm: &mut AsyncFsm<Fsm>, input: Input) -> Result<Vec<Output>, String> {
    fsm.submit(input);
    let outcome = fsm
        .next()
        .await
        .ok_or_else(|| "stream ended unexpectedly".to_string())?;
    take_outputs(outcome, "step")
}

fn take_outputs(outcome: AsyncOutcome<Fsm>, what: &str) -> Result<Vec<Output>, String> {
    match outcome {
        AsyncOutcome::Outputs(o) => Ok(o),
        AsyncOutcome::Error(e) => Err(format!("{what}: FSM error {e:?}")),
    }
}

fn expect_no_outputs(outputs: Vec<Output>, what: &str) -> Result<(), String> {
    if outputs.is_empty() {
        Ok(())
    } else {
        Err(format!("{what}: unexpected outputs {outputs:?}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The full integration story as a test: the generated spec-43 FSM is
    /// exposed as a futures Stream, the driver completes one ARQ session,
    /// and every observable protocol invariant holds.
    #[test]
    fn arq_session_completes_over_the_stream() {
        let r = run().expect("ARQ session must complete");
        assert_eq!(r.frames_sent, (0..8).collect::<Vec<u8>>());
        assert_eq!(r.acks_received, (0..8).collect::<Vec<u8>>());
        assert!(r.max_window <= 4, "window bound: {}", r.max_window);
        assert!(r.out_of_band_acks_dropped >= 1);
    }
}
