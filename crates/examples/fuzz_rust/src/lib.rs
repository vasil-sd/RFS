//! `fuzz_rust` root crate (Phase 5 task 12): the generated spec-43 ARQ
//! sender as a plain lib, consumed by the cargo-fuzz harness in `fuzz/`.
//! Docs: dev:workspace-crates-overview, llm:workspace-crates

// The generated artifact body (build.rs strips its inner attributes and
// writes it to OUT_DIR). The module-level #[allow] mirrors the artifact's
// own #![allow(...)] header line exactly.
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

// Re-export the generated surface at the crate root: the emitted fuzz
// skeleton imports `use fuzz_rust::{Fsm, Input, StopAssumptionHandler};`.
pub use arq::*;
