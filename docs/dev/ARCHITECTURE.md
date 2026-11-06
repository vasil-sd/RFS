# RFS Architecture

Pipeline overview, component diagram, and functionality hierarchy of the RFS
toolchain: spec to parse to validate to IR to codegen / model checking /
diagrams.

<a id="dev:architecture-pipeline"></a>
## Pipeline overview

RFS is an in-memory pipeline: a command loads a spec, normalizes it to one
versioned IR, and derives its output from that IR alone - there is no
intermediate on-disk representation between stages.

1. **Input** - a spec arrives as RFS source text (`.rfs`) or as IR JSON
   (`.json`); `src/cli/frontend.rs::Format` maps extensions to formats and
   `src/cli/frontend.rs::convert_file_to_ir` loads either form into the IR.
2. **Parse** - RFS source is parsed by the nom-based parser,
   `src/lang/parser.rs::parse_source` (or `src/lang/parser.rs::parse_file` for a path),
   producing an `Ir` with source spans attached to every node
   (`src/lang/error.rs::Span`).
3. **Validate** - `src/lang/validator.rs::validate_ir` runs the semantic checks
   (well-formed states/events/transitions, expression typing) and returns a
   list of validation errors; a spec is usable only when that list is empty.
4. **IR** - the normalized form is `src/lang/ir.rs::Ir`, versioned by
   `src/lang/ir.rs::IR_VERSION`. Its wire format is hand-rolled JSON
   (`src/json/json_rw.rs`); every generated artifact carries the spec hash
   `src/utils/hash.rs::spec_hash16` (first 16 hex chars of SHA-256 over the
   canonical IR JSON) in its header line. The IR can be printed back to RFS
   source by `src/lang/printer.rs::print_ir`.
5. **Targets** - everything downstream consumes the validated IR:
   - *Production FSM code*, *model-checker executables* and *unit/fuzz tests*
     are generated through the backend plugin registry
     `src/codegen/mod.rs::BACKENDS` (lookup: `src/codegen/mod.rs::get_backend`), with
     the Rust and C backends registered in `src/codegen/backends/rust/mod.rs` /
     `src/codegen/backends/c/mod.rs`; shared codegen helpers live in
     `src/codegen/fsm.rs`, `src/codegen/mc_concepts.rs` and `src/codegen/test_plan.rs`.
   - *Model checking* plans are computed once per spec by
     `src/mc/plan.rs::McPlan` and embedded in both backends' checker
     binaries; the SMT path is rendered from `src/mc/smt_plan.rs` by
     `src/mc/smt_emit.rs`, and `src/mc/ref_mc.rs::run` provides the in-process
     reference checker behind `rfs mc-ref`.
   - *Monitors* for PSL properties are compiled by
     `src/codegen/monitor.rs::compile_formula` into `src/codegen/monitor.rs::MonitorPlan`s
     embedded in the generated checkers.
   - *Diagrams* (state diagrams, transition tables) come from the diagram
     plugin registry `src/cli/commands/diagram/mod.rs::DIAGRAMS`, one plugin per format under
     `src/cli/commands/diagram/`; external renderers are resolved through
     `src/cli/ext_tool.rs::ExtTool`.

The CLI (`src/main.rs`) exposes this pipeline as eight commands: `parse` and
`validate` (frontend logic in `src/cli/frontend.rs::run_conversion` /
`src/cli/frontend.rs::run_validate`), `info`, `test`, `diagram`, `codegen`,
`mc-ref`, and `gen-spec` (the one producer-side command: it writes a random
spec, then parses + validates it before writing).

<a id="dev:architecture-components"></a>
## Component diagram

```mermaid
flowchart TD
    IN[".rfs source / IR .json"]

    subgraph CLI["CLI dispatch - src/main.rs"]
        CMD["parse, validate, info, test, diagram, codegen, mc-ref, gen-spec"]
    end

    subgraph FE["Frontend - src/cli/frontend.rs"]
        CONV["run_conversion / run_validate"]
    end

    subgraph CORE["Spec core"]
        PARSER["src/lang/parser.rs (nom)"]
        VALID["src/lang/validator.rs"]
        IR["src/lang/ir.rs (Ir, IR_VERSION) + json_rw / spec_hash / printer"]
    end

    subgraph GEN["Codegen - src/codegen/mod.rs registry"]
        FSM["codegen_fsm -> rust_fsm / c_fsm"]
        MC["codegen_mc -> rust_mc / c_mc"]
        TEST["codegen_test -> rust_test / c_test"]
    end

    subgraph MCE["MC engine support"]
        MCP["src/mc/plan.rs (McPlan)"]
        SMT["src/mc/smt_plan.rs + src/mc/smt_emit.rs"]
        MON["src/codegen/monitor.rs (compile_formula)"]
        REF["src/mc/ref_mc.rs (rfs mc-ref)"]
    end

    subgraph DIAG["Diagrams - src/cli/commands/diagram/mod.rs registry"]
        PLUG["dot / mermaid / plantuml / tables (src/cli/commands/diagram/)"]
        EXT["src/cli/ext_tool.rs (external renderers)"]
    end

    IN --> CONV
    CMD --> CONV
    CONV --> PARSER
    PARSER --> VALID
    VALID --> IR
    IR --> FSM
    IR --> MC
    IR --> TEST
    IR --> PLUG
    MCP --> MC
    SMT --> MC
    MON --> MC
    IR --> REF
    PLUG --> EXT
```

<a id="dev:architecture-hierarchy"></a>
## Functionality hierarchy

```mermaid
flowchart TD
    RFS["RFS toolchain"]

    subgraph AUTHOR["Spec authoring"]
        GEN["rfs gen-spec (src/lang/specgen.rs)"]
    end

    subgraph FRONT["Spec frontend"]
        PARSE["parse (src/lang/parser.rs)"]
        VALIDATE["validate (src/lang/validator.rs)"]
        CONVERT["format conversion (src/cli/frontend.rs)"]
        INFO["rfs info (src/cli/commands/info.rs, src/cli/info_format/mod.rs)"]
    end

    subgraph IRCORE["IR core"]
        IR["Ir + IR_VERSION (src/lang/ir.rs)"]
        JSON["JSON wire format (src/json/json_rw.rs)"]
        HASH["spec hash (src/utils/hash.rs)"]
        PRINT["print to RFS (src/lang/printer.rs)"]
    end

    subgraph CODEGEN["Codegen (src/codegen/mod.rs registry)"]
        PROD["production FSM (codegen_fsm -> rust_fsm / c_fsm)"]
        MCBIN["MC executables (codegen_mc -> rust_mc / c_mc)"]
        TESTS["unit/fuzz tests (codegen_test -> rust_test / c_test)"]
    end

    subgraph MCENG["Model checking"]
        PLAN["McPlan (src/mc/plan.rs)"]
        SMT["SMT path (src/mc/smt_plan.rs, src/mc/smt_emit.rs)"]
        MON["monitors (src/codegen/monitor.rs)"]
        REF["reference checker (src/mc/ref_mc.rs)"]
    end

    subgraph DIAGRAMS["Diagrams (src/cli/commands/diagram/mod.rs registry)"]
        FORMATS["dot / mermaid / plantuml / tables (src/cli/commands/diagram/)"]
        RENDER["rendering (src/cli/ext_tool.rs)"]
    end

    subgraph TOOLING["Tooling"]
        TESTCMD["rfs test (src/cli/commands/test.rs)"]
        TRAIN["training data (src/training.rs + src/bin/)"]
        FUZZ["fuzz targets (fuzz/fuzz_targets/)"]
        CRATES["workspace crates (crates/)"]
    end

    RFS --> AUTHOR
    RFS --> FRONT
    RFS --> IRCORE
    RFS --> CODEGEN
    RFS --> MCENG
    RFS --> DIAGRAMS
    RFS --> TOOLING
```

### Relationship map

- **The IR is the only shared currency.** Every consumer - codegen, model
  checking, diagrams, `rfs info` - reads the validated `src/lang/ir.rs::Ir`; no
  component passes live data structures to another outside the shared helper
  modules. All file-based commands load their spec through the frontend
  gateway `src/cli/frontend.rs::convert_file_to_ir`; `gen-spec`
  (`src/lang/specgen.rs`) is the only producer-side command.
- **Two plugin registries, one pattern.** Codegen
  (`src/codegen/mod.rs::BACKENDS`, lookup `src/codegen/mod.rs::get_backend`) and
  diagrams (`src/cli/commands/diagram/mod.rs::DIAGRAMS`) both use a static registry of
  struct-plus-function-pointer entries; adding a backend or a diagram format
  means registering one entry, not touching the dispatch.
- **Parity by construction.** The model-checking plan is computed once in
  Rust (`src/mc/plan.rs::McPlan`) and embedded in BOTH backends' checker
  binaries, and the SMT spec is rendered from one backend-neutral source
  (`src/mc/smt_plan.rs`) into Rust (z3) and C (libz3) by `src/mc/smt_emit.rs` - so
  Rust/C checker behavior cannot drift apart at the plan level.
- **Independent ground truth.** The reference checker `src/mc/ref_mc.rs::run`
  deliberately shares no code with the generated checkers; it is the oracle
  that cross-checks them (`rfs mc-ref`).
- **Monitors are embedded, not linked.** `src/codegen/monitor.rs::compile_formula`
  runs at codegen time; the resulting `src/codegen/monitor.rs::MonitorPlan` is baked
  into the generated artifacts rather than being a runtime dependency.
- **Shared external-tool layer.** Optional external tools (diagram renderers,
  rustc/gcc for compiling generated tests) are resolved and invoked through
  `src/cli/ext_tool.rs::ExtTool`, which can also emit a deterministic POSIX sh
  script instead of executing (`--print-script`).
- **Workspace crates wrap the pipeline.** `crates/rfs-build` drives the same
  compile path from dependent crates' `build.rs`; `crates/rfs-adapters`
  provides runtime adapters around generated FSMs. The fuzz targets in
  `fuzz/fuzz_targets/` feed random inputs into the same parser/validator/IR/
codegen functions the pipeline uses.
