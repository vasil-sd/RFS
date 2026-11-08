//! Fixture crate root for the edit-tool integration tests.
pub mod alpha;
pub mod beta;

/// Drives both modules. The string literal and the comment below carry the
/// old name `target_fn` on purpose - phantom negative control: a rename must
/// leave them byte-identical.
pub fn drive() -> u32 {
    let note = "target_fn is documented here";
    // target_fn: see the alpha module
    crate::alpha::target_fn(1) + beta::helper() + crate::beta::helper()
}
