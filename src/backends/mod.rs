//! Code generation backend plugins.
//!
//! Each target language is a self-contained module here: one registration
//! entry (e.g. [`rust::RUST_BACKEND`]) plus the modules that implement its
//! `production`, `test`, and `mc` commands. See `CODEGEN.md` for the plugin
//! API and how to add a new backend.
//! Docs: dev:backend-rust-overview, llm:backend-rust

pub mod c;
pub mod c_fsm;
pub mod c_mc;
pub mod c_test;
pub mod rust;
pub mod rust_fsm;
pub mod rust_mc;
pub mod rust_test;
