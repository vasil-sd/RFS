//! The sync channel-based adapter (Phase 5 task 11): the FSM runs on its
//! own thread; inputs and outcomes cross std mpsc channels. FIFO ordering is
//! preserved. Requires the `std` feature.

use crate::ProtocolFsm;
use alloc::vec::Vec;
use std::sync::mpsc;

/// One outcome delivered over the channel.
pub enum ChannelOutcome<F: ProtocolFsm> {
    /// The step succeeded: its outputs.
    Outputs(Vec<F::Output>),
    /// The step failed: the FSM error. A failed step does NOT stop the FSM
    /// thread - subsequent inputs are still processed (the generated FSM's
    /// state is unchanged by a rejected input).
    Error(F::Error),
}

// Manual impls bounded on the ASSOCIATED types (not `F`): the generated
// `Fsm` struct itself derives neither Debug nor PartialEq.
impl<F: ProtocolFsm> core::fmt::Debug for ChannelOutcome<F>
where
    F::Output: core::fmt::Debug,
    F::Error: core::fmt::Debug,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ChannelOutcome::Outputs(o) => f.debug_tuple("Outputs").field(o).finish(),
            ChannelOutcome::Error(e) => f.debug_tuple("Error").field(e).finish(),
        }
    }
}

impl<F: ProtocolFsm> PartialEq for ChannelOutcome<F>
where
    F::Output: PartialEq,
    F::Error: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (ChannelOutcome::Outputs(a), ChannelOutcome::Outputs(b)) => a == b,
            (ChannelOutcome::Error(a), ChannelOutcome::Error(b)) => a == b,
            _ => false,
        }
    }
}

/// A protocol FSM driven over std mpsc channels from another thread.
pub struct ChannelFsm<F: ProtocolFsm + Send + 'static> {
    tx: mpsc::Sender<F::Input>,
    rx: mpsc::Receiver<ChannelOutcome<F>>,
    /// Keeps the FSM thread joinable; dropping this handle does not stop the
    /// thread (it exits when the input channel closes, i.e. when `tx` drops).
    _handle: std::thread::JoinHandle<()>,
}

impl<F> ChannelFsm<F>
where
    F: ProtocolFsm + Send + 'static,
    F::Input: Send,
    F::Output: Send,
    F::Error: Send,
{
    /// Spawn the FSM thread in its initial configuration.
    pub fn spawn() -> Self {
        let (in_tx, in_rx) = mpsc::channel();
        let (out_tx, out_rx) = mpsc::channel();
        let handle = std::thread::spawn(move || {
            let mut fsm = F::new();
            while let Ok(input) = in_rx.recv() {
                let outcome = match fsm.step(input) {
                    Ok(outputs) => ChannelOutcome::Outputs(outputs),
                    Err(e) => ChannelOutcome::Error(e),
                };
                if out_tx.send(outcome).is_err() {
                    break; // receiver dropped - stop
                }
            }
        });
        Self {
            tx: in_tx,
            rx: out_rx,
            _handle: handle,
        }
    }

    /// Submit one input (FIFO order preserved).
    pub fn send_input(&self, input: F::Input) -> Result<(), mpsc::SendError<F::Input>> {
        self.tx.send(input)
    }

    /// Wait for the next outcome.
    pub fn recv_outcome(&self) -> Result<ChannelOutcome<F>, mpsc::RecvError> {
        self.rx.recv()
    }
}
