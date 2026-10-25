//! Shared helpers for the Phase 6 differential suites (task 4: prod parity,
//! monitor references, PSL translation; task 6: MC JSON report comparison).
//! See `prod_driver` for the production-FSM driver contract and `mc_json`
//! for the peak_rss_kb normalization used by byte comparisons.
pub mod mc_json;
pub mod prod_driver;
