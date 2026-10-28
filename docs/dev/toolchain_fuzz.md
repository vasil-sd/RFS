# RFS Toolchain: Fuzz + Training (Class A)

The development tooling layer: libFuzzer fuzz targets, the training-data
pipeline, and the generated-code test harness.

<a id="dev:toolchain-fuzz-overview"></a>
## Overview

TBD (task 12.3): the five fuzz targets in `fuzz/fuzz_targets/` (`parser.rs`,
`validator.rs`, `ir_roundtrip.rs`, `codegen.rs`, shared helpers in
`common.rs`), the training-data pipeline (`src/training.rs` + the four
`src/bin/` tools), and how generated code tests are produced and run.
