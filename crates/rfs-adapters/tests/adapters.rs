//! Adapter tests over a small in-crate mock FSM (no generated code needed).
//! Docs: dev:workspace-crates-overview, llm:workspace-crates

use rfs_adapters::callback::{drive, CallbackResult};
use rfs_adapters::minimal::Minimal;
use rfs_adapters::ProtocolFsm;
use std::vec::Vec;

/// A tiny protocol FSM: a counter. `Add(k)` adds k and emits `Tick(count)`;
/// `Reset` clears (no output); `Boom` fails with `MockError::Boom`.
#[derive(Debug)]
struct MockFsm {
    count: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MockInput {
    Add(u64),
    Reset,
    Boom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MockOutput {
    Tick(u64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MockError {
    Boom,
}

impl ProtocolFsm for MockFsm {
    type Input = MockInput;
    type Output = MockOutput;
    type Error = MockError;

    fn new() -> Self {
        MockFsm { count: 0 }
    }

    fn step(&mut self, input: MockInput) -> Result<Vec<MockOutput>, MockError> {
        match input {
            MockInput::Add(k) => {
                self.count += k;
                Ok(vec![MockOutput::Tick(self.count)])
            }
            MockInput::Reset => {
                self.count = 0;
                Ok(Vec::new())
            }
            MockInput::Boom => Err(MockError::Boom),
        }
    }
}

// ---------------------------------------------------------------------------
// no_std-minimal (core-only wrapper)
// ---------------------------------------------------------------------------

#[test]
fn minimal_forwards_steps_and_unwraps() {
    let mut m = Minimal::<MockFsm>::new();
    assert_eq!(
        m.step(MockInput::Add(1)).unwrap(),
        vec![MockOutput::Tick(1)]
    );
    assert_eq!(
        m.step(MockInput::Add(2)).unwrap(),
        vec![MockOutput::Tick(3)]
    );
    assert_eq!(m.step(MockInput::Reset).unwrap(), Vec::<MockOutput>::new());
    let inner = m.into_inner();
    assert_eq!(inner.count, 0);
}

#[test]
fn minimal_is_itself_a_protocol_fsm() {
    // Composition: drive a Minimal<MockFsm> through the callback driver.
    let mut m = Minimal::<MockFsm>::new();
    let r = drive(&mut m, |_| Some(MockInput::Add(1)), 3);
    assert_eq!(r, CallbackResult::StepLimit(3));
}

// ---------------------------------------------------------------------------
// callback driver (reactive: the closure sees the previous outputs)
// ---------------------------------------------------------------------------

#[test]
fn callback_drives_until_the_closure_stops() {
    let mut fsm = MockFsm::new();
    // Add(1) while the previous output says count < 5 (empty on the first
    // call), stop once it says 5.
    let r = drive(
        &mut fsm,
        |last| (!matches!(last.first(), Some(MockOutput::Tick(5)))).then_some(MockInput::Add(1)),
        100,
    );
    assert_eq!(r, CallbackResult::Completed(5));
    assert_eq!(fsm.count, 5);
}

#[test]
fn callback_reports_fsm_errors_with_steps_done() {
    let mut fsm = MockFsm::new();
    // Add twice, then Boom.
    let mut n = 0;
    let r = drive(
        &mut fsm,
        |_| {
            n += 1;
            match n {
                1 | 2 => Some(MockInput::Add(1)),
                _ => Some(MockInput::Boom),
            }
        },
        100,
    );
    assert_eq!(r, CallbackResult::Failed(MockError::Boom, 2));
}

#[test]
fn callback_respects_the_step_limit() {
    let mut fsm = MockFsm::new();
    let r = drive(&mut fsm, |_| Some(MockInput::Add(1)), 4);
    assert_eq!(r, CallbackResult::StepLimit(4));
    assert_eq!(fsm.count, 4);
}

// ---------------------------------------------------------------------------
// sync channel adapter (FSM on its own thread)
// ---------------------------------------------------------------------------

#[cfg(feature = "std")]
mod channel_tests {
    use super::*;
    use rfs_adapters::channel::{ChannelFsm, ChannelOutcome};

    #[test]
    fn channel_round_trip_preserves_order() {
        let fsm = ChannelFsm::<MockFsm>::spawn();
        for input in [
            MockInput::Add(1),
            MockInput::Add(2),
            MockInput::Reset,
            MockInput::Add(5),
        ] {
            fsm.send_input(input).unwrap();
        }
        let got: Vec<ChannelOutcome<MockFsm>> =
            (0..4).map(|_| fsm.recv_outcome().unwrap()).collect();
        assert_eq!(got[0], ChannelOutcome::Outputs(vec![MockOutput::Tick(1)]));
        assert_eq!(got[1], ChannelOutcome::Outputs(vec![MockOutput::Tick(3)]));
        assert_eq!(got[2], ChannelOutcome::Outputs(Vec::new()));
        assert_eq!(got[3], ChannelOutcome::Outputs(vec![MockOutput::Tick(5)]));
    }

    #[test]
    fn channel_error_outcome_keeps_the_fsm_running() {
        let fsm = ChannelFsm::<MockFsm>::spawn();
        fsm.send_input(MockInput::Boom).unwrap();
        assert_eq!(
            fsm.recv_outcome().unwrap(),
            ChannelOutcome::Error(MockError::Boom)
        );
        // The FSM thread survived the error and still processes inputs.
        fsm.send_input(MockInput::Add(1)).unwrap();
        assert_eq!(
            fsm.recv_outcome().unwrap(),
            ChannelOutcome::Outputs(vec![MockOutput::Tick(1)])
        );
    }
}

// ---------------------------------------------------------------------------
// futures stream adapter
// ---------------------------------------------------------------------------

#[cfg(feature = "async")]
mod async_tests {
    use super::*;
    use core::task::{Context, Poll};
    use futures::stream::{Stream, StreamExt};
    use rfs_adapters::async_adapter::{AsyncFsm, AsyncOutcome};
    use std::pin::Pin;

    #[test]
    fn async_stream_yields_outcomes_in_order() {
        let out = futures::executor::block_on(async {
            let mut fsm = AsyncFsm::<MockFsm>::new();
            fsm.submit(MockInput::Add(1));
            fsm.submit(MockInput::Add(2));
            fsm.submit(MockInput::Reset);
            let a = fsm.next().await.unwrap();
            let b = fsm.next().await.unwrap();
            let c = fsm.next().await.unwrap();
            (a, b, c)
        });
        assert_eq!(out.0, AsyncOutcome::Outputs(vec![MockOutput::Tick(1)]));
        assert_eq!(out.1, AsyncOutcome::Outputs(vec![MockOutput::Tick(3)]));
        assert_eq!(out.2, AsyncOutcome::Outputs(Vec::new()));
    }

    #[test]
    fn async_stream_is_pending_when_empty_and_ends_after_close() {
        let mut fsm = AsyncFsm::<MockFsm>::new();
        // Empty queue + not closed: Pending (checked without an executor so
        // the test cannot hang).
        let waker = futures::task::noop_waker();
        let mut cx = Context::from_waker(&waker);
        {
            let pinned = Pin::new(&mut fsm);
            assert!(matches!(pinned.poll_next(&mut cx), Poll::Pending));
        }
        // close(): the stream ends after the queue drains.
        fsm.close();
        {
            let pinned = Pin::new(&mut fsm);
            assert!(matches!(pinned.poll_next(&mut cx), Poll::Ready(None)));
        }
    }

    #[test]
    fn async_stream_carries_errors_and_keeps_going() {
        let out = futures::executor::block_on(async {
            let mut fsm = AsyncFsm::<MockFsm>::new();
            fsm.submit(MockInput::Boom);
            fsm.submit(MockInput::Add(1));
            (fsm.next().await.unwrap(), fsm.next().await.unwrap())
        });
        assert_eq!(out.0, AsyncOutcome::Error(MockError::Boom));
        assert_eq!(out.1, AsyncOutcome::Outputs(vec![MockOutput::Tick(1)]));
    }
}
