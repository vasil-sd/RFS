# RFS Architecture

Pipeline overview, component diagram, and functionality hierarchy of the RFS
toolchain: spec to parse to validate to IR to codegen / model checking /
diagrams.

<a id="dev:architecture-pipeline"></a>
## Pipeline overview

TBD (task 2.1): the full pipeline from `.rfs`/`.json` input through parsing,
validation, and IR normalization to the three target families (production
code, model-checking executables, tests/diagrams), with a doc-to-code
reference at every stage.

<a id="dev:architecture-components"></a>
## Component diagram

TBD (task 2.1): mermaid component diagram of the toolchain (CLI dispatch,
frontend, IR, codegen core, Rust/C backends, MC engine, monitor compiler,
diagram emitters).

<a id="dev:architecture-hierarchy"></a>
## Functionality hierarchy

TBD (task 2.2): functionality hierarchy tree + relationship map between
components.
