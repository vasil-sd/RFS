//! RFS adapter layer (Phase 5 task 11): the [`ProtocolFsm`] trait + generic
//! adapters for driving generated FSMs from application code.
//!
//! The trait mirrors the shape of every generated Rust FSM (`new()` +
//! `step(input) -> Result<Vec<Output>, Error>`); codegen emits the impl
//! opt-in via `--with-protocol-fsm` (the one documented opt-in that links a
//! generated artifact to an external crate). All adapters are GENERIC over
//! `F: ProtocolFsm` and never depend on generated code.
//!
//! # Features
//!
//! - `std` (default): the sync channel adapter ([`channel`]) - the FSM runs
//!   on its own thread, inputs/outcomes cross std mpsc channels.
//! - `async`: the futures-based stream adapter ([`async_adapter`]).
//! - The callback driver ([`callback`]) and the no_std-minimal wrapper
//!   ([`minimal`]) are always available (core-only; they work with the
//!   `#![no_std]` production artifacts of Phase 5 task 9).

#![no_std]

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

use alloc::vec::Vec;

#[cfg(feature = "async")]
pub mod async_adapter;
pub mod callback;
#[cfg(feature = "std")]
pub mod channel;
pub mod minimal;

/// The protocol-FSM surface shared by all generated Rust FSMs (Phase 5
/// task 11). Generated code implements this opt-in via `--with-protocol-fsm`;
/// the emitted impl is a mechanical forward to the inherent `Fsm` methods.
pub trait ProtocolFsm {
    /// The FSM's input type (the generated `Input` enum).
    type Input;
    /// The FSM's output type (the generated `Output` enum).
    type Output;
    /// The FSM's step error type (the generated `StepError` enum).
    type Error;

    /// Construct the FSM in its initial configuration.
    fn new() -> Self
    where
        Self: Sized;

    /// Apply one input; returns the outputs emitted by the taken transition.
    fn step(&mut self, input: Self::Input) -> Result<Vec<Self::Output>, Self::Error>;
}
