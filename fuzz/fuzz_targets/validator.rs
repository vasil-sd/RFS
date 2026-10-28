//! Fuzz target: semantic validation + IR JSON deserialization (Phase 6 task
//! 5, D-P6-4).
//!
//! Invariants:
//!   * random VALID IRs (specgen-backed) -> `validate_ir` must not panic;
//!   * arbitrary raw bytes as JSON text -> `Ir::from_json` returns Err (or Ok),
//!     never panics.
//! Docs: dev:toolchain-fuzz-overview, llm:toolchain-fuzz

#![no_main]

mod common;

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let (head, rest) = common::split_head_rest(data);
    // Part 1: random valid IR -> validate must not panic.
    let mut u = arbitrary::Unstructured::new(head);
    if let Ok(spec) = common::RandSpec::arbitrary(&mut u) {
        let _ = rfs::validator::validate_ir(&spec.ir);
    }
    // Part 2: the remaining bytes as raw JSON text -> from_json must not
    // panic (strict parse first; malformed JSON is a clean Err).
    if let Ok(text) = std::str::from_utf8(rest) {
        if let Ok(json) = rfs::json_rw::parse(text) {
            let _ = rfs::Ir::from_json(&json);
        }
    }
});
