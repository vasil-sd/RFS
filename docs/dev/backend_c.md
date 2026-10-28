# RFS C Backend (Class A)

The C target-language backend: plugin registration plus the three generators
that render the shared codegen plans into self-contained C source.

<a id="dev:backend-c-overview"></a>
## Overview

TBD (task 10.1): plugin registration (`src/backends/c.rs::C_BACKEND`),
production FSM generation (`src/backends/c_fsm.rs::generate_production`),
model-checker executable generation with the same runtime wire format as the
Rust checker (`src/backends/c_mc.rs::generate_model_checker`), and unit-test /
fuzz-target generation (`src/backends/c_test.rs::generate_unit_tests`) - parity
with the Rust backend is pinned by the parity test suites.
