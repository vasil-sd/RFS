//! Model-checking cluster: the shared exact-mode plan (`plan`), the SMT spec
//! + emitter pair (`smt_plan` / `smt_emit` - inseparable, all of smt_emit's
//! outgoing edges target smt_plan), and the independent brute-force reference
//! engine (`ref_mc`, which imports nothing from within this directory - its
//! independence contract is audit-tested in-file).
//! Docs: dev:code-map-mc

pub mod plan;
pub mod ref_mc;
pub mod smt_emit;
pub mod smt_plan;
