//! Fuzz target: IR JSON serialization stability (Phase 6 task 5, D-P6-4).
//!
//! Invariant: random valid IRs (specgen-backed, optionally mutated through
//! the task-3 `mutate` API) satisfy to_json -> from_json -> to_json
//! byte-stability.
//! Docs: dev:toolchain-fuzz-overview, llm:toolchain-fuzz

#![no_main]

mod common;

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let (head, rest) = common::split_head_rest(data);
    let mut u = arbitrary::Unstructured::new(head);
    let spec = match common::RandSpec::arbitrary(&mut u) {
        Ok(s) => s,
        Err(_) => return, // exhausted input: nothing to round-trip this round
    };
    // Optionally apply a few validity-preserving mutation steps (the first
    // raw byte picks the count 0..4; no raw bytes -> k = 0).
    let k = rest.first().map(|b| (*b % 4) as usize).unwrap_or(0);
    let ir = if k == 0 {
        spec.ir
    } else {
        let seed = u64::from_le_bytes({
            let mut buf = [0u8; 8];
            let n = rest.len().min(8);
            buf[..n].copy_from_slice(&rest[..n]);
            buf
        });
        let mut rng = rfs::specgen::Rng::new(seed.wrapping_add(0x9e37_79b9));
        rfs::specgen::mutate(&spec.ir, &mut rng, k)
    };
    let t1 = ir
        .to_json()
        .expect("valid IR serializes to JSON")
        .to_string();
    let j2 = rfs::json_rw::parse(&t1)
        .unwrap_or_else(|e| panic!("own JSON must strict-parse: {e}\n{t1}"));
    let ir2 = rfs::Ir::from_json(&j2).unwrap_or_else(|e| panic!("round-trip from_json: {e}\n{t1}"));
    let t2 = ir2.to_json().expect("re-serialized IR").to_string();
    assert_eq!(t1, t2, "to_json/from_json/to_json must be byte-stable");
});
