# RFS Codegen Core (Class A)

The language-agnostic core of the code generator: the backend plugin API and
the shared planning/concept modules that all backends render from.

<a id="dev:codegen-core-overview"></a>
## Overview

TBD (task 9.1): the `Backend` plugin interface and registry (`src/codegen.rs`),
shared production-FSM helpers (`src/codegen_fsm.rs::build_variants`), bounded
model-checker concepts (`src/codegen_mc.rs::McConfig`, violation/status
taxonomy), and unit-test planning (`src/codegen_test.rs::TestPlan`) - the
parity principle: one plan, rendered identically in Rust and C.
