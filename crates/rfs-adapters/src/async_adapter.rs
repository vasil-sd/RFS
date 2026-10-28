//! The futures-based async adapter (Phase 5 task 11): submit inputs with
//! [`AsyncFsm::submit`], consume outcomes as a [`futures::stream::Stream`].
//! The FSM is stepped synchronously inside `poll_next` (generated `step`
//! does no I/O), so ANY executor works - no tokio dependency. Requires the
//! `async` feature.
//! Docs: dev:workspace-crates-overview, llm:workspace-crates

use crate::ProtocolFsm;
use alloc::collections::VecDeque;
use alloc::vec::Vec;
use core::pin::Pin;
use core::task::{Context, Poll};
use futures::stream::Stream;

/// One outcome produced by the stream.
pub enum AsyncOutcome<F: ProtocolFsm> {
    /// The step succeeded: its outputs.
    Outputs(Vec<F::Output>),
    /// The step failed: the FSM error (the stream keeps producing for
    /// subsequently submitted inputs).
    Error(F::Error),
}

// Manual impls bounded on the ASSOCIATED types (not `F`): the generated
// `Fsm` struct itself derives neither Debug nor PartialEq.
impl<F: ProtocolFsm> core::fmt::Debug for AsyncOutcome<F>
where
    F::Output: core::fmt::Debug,
    F::Error: core::fmt::Debug,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            AsyncOutcome::Outputs(o) => f.debug_tuple("Outputs").field(o).finish(),
            AsyncOutcome::Error(e) => f.debug_tuple("Error").field(e).finish(),
        }
    }
}

impl<F: ProtocolFsm> PartialEq for AsyncOutcome<F>
where
    F::Output: PartialEq,
    F::Error: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (AsyncOutcome::Outputs(a), AsyncOutcome::Outputs(b)) => a == b,
            (AsyncOutcome::Error(a), AsyncOutcome::Error(b)) => a == b,
            _ => false,
        }
    }
}

/// A protocol FSM exposed as a [`Stream`] of outcomes. Each `poll_next`
/// consumes at most one queued input; with an empty queue it is `Pending`
/// until [`close`](Self::close) (then the stream ends).
pub struct AsyncFsm<F: ProtocolFsm> {
    fsm: F,
    pending: VecDeque<F::Input>,
    closed: bool,
}

impl<F: ProtocolFsm> AsyncFsm<F> {
    /// Construct in the initial configuration with an empty input queue.
    pub fn new() -> Self {
        Self {
            fsm: F::new(),
            pending: VecDeque::new(),
            closed: false,
        }
    }

    /// Queue one input; the next `poll_next` steps the FSM on it.
    pub fn submit(&mut self, input: F::Input) {
        self.pending.push_back(input);
    }

    /// End the stream once queued inputs have drained.
    pub fn close(&mut self) {
        self.closed = true;
    }
}

// The Unpin bounds hold for every generated FSM (Copy-field structs and
// enums) and keep `poll_next` free of unsafe: the adapter owns plain data.
impl<F: ProtocolFsm> Stream for AsyncFsm<F>
where
    F: Unpin,
    F::Input: Unpin,
{
    type Item = AsyncOutcome<F>;

    fn poll_next(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.as_mut().get_mut();
        if let Some(input) = this.pending.pop_front() {
            let outcome = match this.fsm.step(input) {
                Ok(outputs) => AsyncOutcome::Outputs(outputs),
                Err(e) => AsyncOutcome::Error(e),
            };
            return Poll::Ready(Some(outcome));
        }
        if this.closed {
            Poll::Ready(None)
        } else {
            Poll::Pending
        }
    }
}
