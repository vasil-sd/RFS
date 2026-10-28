# RFS Model-Checking Engine (Class A)

The model-checking engine family: the exact bounded checker plan, the shared
SMT encoding + emitter for the z3-based mode, and the brute-force reference
checker used as ground truth.

<a id="dev:mc-engine-overview"></a>
## Overview

TBD (task 11.1): the codegen-time `McPlan` computed once and embedded in both
generated binaries (`src/mc_plan.rs::McPlan`), the backend-neutral SMT spec
(`src/smt_plan.rs`) rendered by the shared emitter
(`src/smt_emit.rs::emit_rust_spec`, `src/smt_emit.rs::emit_c_spec`) for parity
by construction, and the independent brute-force reference checker
(`src/ref_mc.rs::run`) behind `rfs mc-ref` and differential testing.
