//! Fuzz target: the RFS v0 parser (Phase 6 task 5, D-P6-4).
//!
//! Invariant: arbitrary bytes -> `parse_source` must NOT panic. On success
//! the IR validates without panicking and round-trips through the hand-rolled
//! JSON serialization byte-stably (D-P5-2 contract on parser output).
//! Docs: dev:toolchain-fuzz-overview, llm:toolchain-fuzz

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // `parse_source` is a `&str` API: non-UTF-8 input cannot reach it
    // (the CLI rejects such files before parsing), so skip it here.
    let Ok(source) = std::str::from_utf8(data) else {
        return;
    };
    match rfs::parse_source(source) {
        Err(_) => {} // malformed input: a clean parse error is the goal.
        Ok(ir) => {
            // Semantic validation must not panic (valid or invalid result).
            let _ = rfs::validator::validate_ir(&ir);
            // A parsed IR must serialize, strict-parse and re-serialize
            // byte-stably.
            let t1 = ir
                .to_json()
                .expect("parsed IR serializes to JSON")
                .to_string();
            let j2 = rfs::json_rw::parse(&t1)
                .unwrap_or_else(|e| panic!("own JSON must strict-parse: {e}\n{t1}"));
            let ir2 = rfs::Ir::from_json(&j2)
                .unwrap_or_else(|e| panic!("round-trip from_json: {e}\n{t1}"));
            let t2 = ir2.to_json().expect("re-serialized IR").to_string();
            assert_eq!(t1, t2, "to_json/from_json/to_json must be byte-stable");
        }
    }
});
