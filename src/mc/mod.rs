//! Model-checking cluster: the shared exact-mode plan (`plan`), the SMT spec
//! with its emitter pair (`smt_plan` / `smt_emit` - inseparable, all of
//! smt_emit's outgoing edges target smt_plan), and the independent brute-force
//! reference engine. The reference engine is split per mode (Phase 16,
//! D-P16-1): `ref_mc` holds the common core + entry points, `ref_mc_exact`
//! the exact-mode engine. Independence contract scoped per file: exact/fuzz
//! import nothing from within this directory beyond the `ref_mc` common core;
//! the SMT mode (task 4) may additionally consume the plan + encoding model
//! chain (`plan` + `smt_plan`, D-P16-3). Audit-tested in `ref_mc.rs` over all
//! `ref_mc_*.rs` files.
//! Docs: dev:code-map-mc

pub mod plan;
pub mod ref_mc;
pub mod ref_mc_exact;
pub mod ref_mc_fuzz;
#[cfg(feature = "smt")]
pub mod ref_mc_smt;
pub mod smt_emit;
pub mod smt_plan;
