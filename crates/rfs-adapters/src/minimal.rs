//! The no_std-minimal adapter (Phase 5 task 11): a pure step-forwarding
//! wrapper with no state of its own and no allocations beyond the trait's
//! own `Vec` return. Core-only - works with the `#![no_std]` production
//! artifacts of Phase 5 task 9.
//! Docs: dev:workspace-crates-overview, llm:workspace-crates

use crate::ProtocolFsm;
use alloc::vec::Vec;

/// A transparent wrapper around any [`ProtocolFsm`].
pub struct Minimal<F>(pub F);

impl<F: ProtocolFsm> Minimal<F> {
    /// Construct the wrapped FSM in its initial configuration.
    pub fn new() -> Self
    where
        Self: Sized,
    {
        Minimal(F::new())
    }

    /// Forward one step to the wrapped FSM.
    pub fn step(&mut self, input: F::Input) -> Result<Vec<F::Output>, F::Error> {
        self.0.step(input)
    }

    /// Unwrap the inner FSM.
    pub fn into_inner(self) -> F {
        self.0
    }
}

/// `Minimal` is itself a protocol FSM, so adapters compose (e.g. drive a
/// `Minimal<F>` through [`crate::callback::drive`]).
impl<F: ProtocolFsm> ProtocolFsm for Minimal<F> {
    type Input = F::Input;
    type Output = F::Output;
    type Error = F::Error;

    fn new() -> Self
    where
        Self: Sized,
    {
        Minimal(F::new())
    }

    fn step(&mut self, input: Self::Input) -> Result<Vec<Self::Output>, Self::Error> {
        self.0.step(input)
    }
}
