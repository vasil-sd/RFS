//! The callback-based adapter (Phase 5 task 11): a driving loop where a
//! user-supplied closure decides the next input from the PREVIOUS step's
//! outputs - a reactive protocol driver. Core-only.

use crate::ProtocolFsm;
use alloc::vec::Vec;

/// Outcome of a callback-driven run.
pub enum CallbackResult<E> {
    /// The driver returned `None` (protocol finished): steps taken.
    Completed(usize),
    /// The FSM reported an error: the error + steps completed before it.
    Failed(E, usize),
    /// The step limit was reached: steps taken.
    StepLimit(usize),
}

impl<E: core::fmt::Debug> core::fmt::Debug for CallbackResult<E> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            CallbackResult::Completed(n) => f.debug_tuple("Completed").field(n).finish(),
            CallbackResult::Failed(e, n) => f.debug_tuple("Failed").field(e).field(n).finish(),
            CallbackResult::StepLimit(n) => f.debug_tuple("StepLimit").field(n).finish(),
        }
    }
}

impl<E: PartialEq> PartialEq for CallbackResult<E> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (CallbackResult::Completed(a), CallbackResult::Completed(b)) => a == b,
            (CallbackResult::Failed(ea, a), CallbackResult::Failed(eb, b)) => ea == eb && a == b,
            (CallbackResult::StepLimit(a), CallbackResult::StepLimit(b)) => a == b,
            _ => false,
        }
    }
}

/// Drive `fsm` with inputs produced by `decide`: before each step the
/// closure receives the previous step's outputs (empty for the first) and
/// returns the next input, or `None` to stop. Stops on an FSM error or after
/// `max_steps` steps.
pub fn drive<F, C>(fsm: &mut F, mut decide: C, max_steps: usize) -> CallbackResult<F::Error>
where
    F: ProtocolFsm,
    C: FnMut(&[F::Output]) -> Option<F::Input>,
{
    let mut steps = 0usize;
    let mut last: Vec<F::Output> = Vec::new();
    while steps < max_steps {
        let Some(input) = decide(&last) else {
            return CallbackResult::Completed(steps);
        };
        match fsm.step(input) {
            Ok(outputs) => {
                last = outputs;
                steps += 1;
            }
            Err(e) => return CallbackResult::Failed(e, steps),
        }
    }
    CallbackResult::StepLimit(steps)
}
