# RFS Roadmap - Project Principles and Invariants

Project-wide principles and invariants that apply to ALL phases, tasks and
documentation. Split out of `INTRO.md` on 2026-11-01 (see `INTRO.md` for the
file set and naming).

## 1. Guiding principles

The guiding principles are:

> One constrained specification → one versioned IR → multiple targets.

> **Backend parity:** every codegen backend provides the same functionality -
> the same commands, the same deliverables per command, and exactly the same
> observable behavior for the same spec and inputs. Backends must always be kept
> in sync: a feature lands in ALL existing backends within the same phase, and
> cross-backend differential tests verify that backends behave identically.
> Rationale: testing backends against each other gives maximal confidence in
> their correctness.

Targets include production automata, model-checking executables, tests, diagrams,
and later IDE support.

## 2. General rules for the roadmap document set

- All files here are small by design: read the specific file you need fully; do
  not grep-and-sed them the way you treat `KB.md`/`DESIGN.md`.
- Phase descriptions (`PHASE_N.md`) are frozen once planning starts; anything new
  learned goes into `PHASE_N_PLAN.md` (corrections) or `PHASE_N_STATUS.md`, never
  silently into the phase file.
- Keep `CURRENT_STATE.md` (the phase-state table) current.
- Backend parity (guiding principle) applies to ALL phases and ALL tasks: every plan must
  include per-backend tasks for every backend feature and at least one
  cross-backend differential test comparing backends' behavior on identical
  specs/seeds; a phase may not exit with any backend left behind on in-scope
  functionality.
- The "Recommended v1 scope" and implementation-stack guidance in sections
  3-4 of this file apply to ALL phases; they moved here from the old `ROADMAP.md`
  (and from `INTRO.md` at the 2026-11-01 split).

## 3. Recommended v1 scope

To keep the first production-ready version manageable, v1 should include:

- external `.rfs` files;
- single endpoint module;
- finite states as enums;
- events and outputs with finite payload fields;
- variables of type `bool`, enum, `bit<N>`;
- pure guarded transitions;
- simultaneous variable updates;
- explicit priorities for overlapping transitions;
- deterministic production semantics;
- `Result`-based unexpected event handling;
- optional explicit error transition;
- `Tick` event and timer variables;
- PSL-like safety properties:
  - boolean atoms;
  - `next`;
  - `always`;
  - `always[N]`;
  - `bounded_eventually[N]`;
- assumptions using the same property language;
- exact reachable-state MC for small systems;
- bounded MC mode;
- counterexample JSON/text output;
- production code generation in all backends;
- optional debug property monitors;
- CLI and `build.rs` integration;
- generated unit tests.

v1 should not include:

- multi-party composition;
- real-time timed automata;
- unbounded integers;
- arbitrary Rust function calls in transitions;
- full strong unbounded liveness unless implementation goes smoothly;
- LSP;
- advanced symbolic/SMT MC;
- byte-level protocol codecs.

These can be added later without changing the core architecture.

## 4. Suggested implementation stack

| Component | Recommended technology |
|---|---|
| Parser | `nom` 8 recursive descent parser for Phase 0; `chumsky` remains a future candidate for richer diagnostics/recovery |
| Diagnostics | Custom span-based diagnostics first; `miette` later if useful |
| IR serialization | `serde` + `serde_json` initially; protobuf later if needed |
| CLI | `clap` later; simple `std::env::args` for Phase 0 |
| Code generation | Direct Rust string generation first; `proc-macro2`/`quote` later if useful |
| Formatting generated Rust | `rustfmt` programmatic invocation or `rustfmt` crate |
| Testing | `cargo test`, snapshot tests later, property-based tests later |
| Fuzzing | `cargo-fuzz` / `libfuzzer` later |
| LSP later | `tower-lsp` |

### Model-checking library stack

| Component | Recommendation |
|---|---|
| Exact search core | Custom Rust graph search with hash sets/bitsets |
| State packing | Generated bit-packing for states/vars/monitors |
| Counterexample storage | Parent pointers or trace recording |
| Bounded/SMT later | Optional `z3` bindings behind feature flag |
| Symbolic later | Custom BDD or SAT-based backend, optional |
