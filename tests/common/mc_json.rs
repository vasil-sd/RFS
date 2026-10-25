//! Shared helper for byte-comparing the generated engines' MC JSON reports
//! (Phase 6 task 6): the informational `peak_rss_kb` field is zeroed before
//! comparison because it carries the ACTUAL peak resident set self-reported
//! by the binary (machine-dependent - like wall-clock times, never pinned).

/// Zero the value of the `peak_rss_kb` field in one compact MC JSON document.
/// The field is the LAST key of the statistics object in the wire form, so
/// its value runs to the closing brace; the surgery is exact on documents
/// produced by the generated writers (no whitespace outside strings).
pub fn zero_peak_rss(doc: &str) -> String {
    const MARKER: &str = "\"peak_rss_kb\":";
    match doc.find(MARKER) {
        Some(start) => {
            let after = start + MARKER.len();
            let digits = doc[after..]
                .chars()
                .take_while(|c| c.is_ascii_digit())
                .count();
            assert!(digits > 0, "peak_rss_kb value missing in: {doc}");
            let mut out = doc.to_string();
            out.replace_range(after..after + digits, "0");
            out
        }
        None => doc.to_string(),
    }
}
