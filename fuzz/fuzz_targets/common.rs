//! Shared helpers for the toolchain fuzz targets (Phase 6 task 5, D-P6-4).
//!
//! Arbitrary strategy per PHASE_6_TASK_5.md: do NOT derive `Arbitrary` over
//! raw IR types (that would produce invalid IR). Instead this wrapper maps
//! fuzzer bytes -> specgen seed + bounds and builds random VALID specs through
//! the task-3 `rfs::specgen` builder (validity by construction -
//! `SpecGen::generate` asserts it).

// Each target binary compiles this module with only the helpers it uses, so
// the unused ones would warn as dead code per binary.
#![allow(dead_code)]

use arbitrary::{Arbitrary, Unstructured};
use rfs::specgen::{Bounds, SpecGen};

/// One random valid single-module spec derived from fuzzer bytes.
pub struct RandSpec {
    pub ir: rfs::Ir,
}

impl<'a> Arbitrary<'a> for RandSpec {
    fn arbitrary(u: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
        let seed = u64::arbitrary(u)?;
        let bounds = if bool::arbitrary(u)? {
            Bounds::medium()
        } else {
            Bounds::small()
        };
        Ok(RandSpec {
            ir: SpecGen::new(seed, bounds).generate(),
        })
    }
}

/// The byte budget of [`RandSpec`]'s head (8 seed bytes + 1 size flag), so
/// targets that ALSO need raw remaining bytes can split the input
/// deterministically.
pub const HEAD_LEN: usize = 9;

/// Split fuzzer input into the RandSpec head and the raw remainder.
pub fn split_head_rest(data: &[u8]) -> (&[u8], &[u8]) {
    data.split_at(data.len().min(HEAD_LEN))
}

/// Build a random valid spec from fuzzer bytes (the whole input is the head;
/// exhausted input falls back to a fixed seed so the target stays alive).
pub fn rand_spec(data: &[u8]) -> rfs::Ir {
    let mut u = Unstructured::new(data);
    match RandSpec::arbitrary(&mut u) {
        Ok(s) => s.ir,
        Err(_) => SpecGen::new(0, Bounds::small()).generate(),
    }
}
