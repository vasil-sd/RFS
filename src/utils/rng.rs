//! Docs: dev:code-map-utils, dev:codegen-test-plan
// The project's single hand-rolled LCG generator: `state = state * LCG_MULT +
// LCG_ADD` (wrapping), with the top-53-bit double mapping used by the generated
// fuzz runtimes. All toolchain-side random draws go through this type; the two
// GENERATED-runtime copies (the Rust FUZZ_RUNTIME string in
// codegen/backends/rust/mc.rs and the C seed function in
// codegen/backends/c/mc.rs) interpolate these same constants so all four stay
// in one RNG family by construction rather than by copy-paste.

/// LCG multiplier (shared by every copy of the generator).
pub const LCG_MULT: u64 = 6364136223846793005;
/// LCG addend (shared by every copy of the generator).
pub const LCG_ADD: u64 = 1442695040888963407;

#[derive(Debug, Clone)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn step(&mut self) -> u64 {
        self.0 = self.0.wrapping_mul(LCG_MULT).wrapping_add(LCG_ADD);
        self.0
    }

    pub fn next_u64(&mut self) -> u64 {
        self.step()
    }

    /// Same top-53-bit double mapping as the fuzz wire format.
    pub fn next_f64(&mut self) -> f64 {
        ((self.step() >> 11) as f64) / ((u64::MAX >> 11) as f64)
    }

    /// Uniform integer in `[lo, hi)` (panics on an empty range).
    pub fn range(&mut self, lo: usize, hi: usize) -> usize {
        assert!(hi > lo, "empty range");
        lo + (self.next_u64() % (hi - lo) as u64) as usize
    }

    /// A random element (`None` for an empty slice).
    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> Option<&'a T> {
        if items.is_empty() {
            None
        } else {
            Some(&items[self.range(0, items.len())])
        }
    }

    /// True with probability `p`.
    pub fn chance(&mut self, p: f64) -> bool {
        self.next_f64() < p
    }

    /// Same index selection as the fuzz wire format.
    pub fn pick_index(&mut self, len: usize) -> Option<usize> {
        if len == 0 {
            None
        } else {
            Some((self.next_f64() * len as f64) as usize % len)
        }
    }
}
