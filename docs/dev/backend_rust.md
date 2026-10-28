# RFS Rust Backend (Class A)

The Rust target-language backend: plugin registration plus the three
generators that render the shared codegen plans into standalone Rust source.

<a id="dev:backend-rust-overview"></a>
## Overview

TBD (task 9.2): plugin registration (`src/backends/rust.rs::RUST_BACKEND`),
production FSM generation (`src/backends/rust_fsm.rs::generate_production`),
model-checker executable generation
(`src/backends/rust_mc.rs::generate_model_checker`), and unit-test / fuzz /
aggregate-crate generation (`src/backends/rust_test.rs::generate_unit_tests`) -
parity with the C backend is pinned by the parity test suites.
