//! dev_tools - RFS maintenance tooling (Phase 12).
//!
//! Library half of the `rfs-dev` binary: per-tool modules land here as Phase
//! 12 task 5 implements them. Implemented so far: `sync` (task 5.1),
//! `context` (task 5.2), `edit` (task 5.3), `doc_section` (task 5.4),
//! `inventory` (task 5.5), `calls` (task 5.6). The crate operates on the
//! repository's sources and docs as data; nothing in the `rfs` build/test path
//! invokes it.

pub mod calls;
pub mod context;
pub mod doc_section;
pub mod edit;
pub mod inventory;
pub mod sync;
