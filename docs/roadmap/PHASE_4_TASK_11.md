# Phase 4, Task 11 - Implementation details (OPTIONAL)

> Written for a future LLM session: after reading `START.md`,
> `docs/roadmap/INTRO.md` and this file, the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal
Optional `--mode smt-bounded`: SMT-based bounded model checking with z3 for larger
bitvector exploration, clearly labeled incomplete, with identical CLI semantics +
results in both backends. Gated by a feasibility check; deferral is a first-class
outcome (D-P4-12).

## Context

- Environment (verified 2026-10-18): z3 **5.3.1** installed project-locally at
  `/workspace/host/z3` (prebuilt x64-glibc-2.39 release; `bin/z3`, `bin/libz3.so`,
  `include/`, `lib/pkgconfig/z3.pc`). Install/reinstall procedure: TOOLCHAIN.md
  section 5.1. Rust integration env vars (TOOLCHAIN.md 5.2):
  `PATH=/workspace/host/z3/bin:$PATH` + `Z3_LIBRARY_PATH_OVERRIDE=/workspace/host/z3/bin`
  with the DEFAULT z3-sys feature (do NOT use `gh-release`/`vendored` - they would
  pin Rust to a different solver build than C links, breaking parity). C integration
  (TOOLCHAIN.md 5.3): `-I/workspace/host/z3/include -L/workspace/host/z3/bin -lz3
  -Wl,-rpath,/workspace/host/z3/bin`.
- Pinned crate versions: `z3` 0.21.x / `z3-sys` 0.13.x (tracks the z3 5.x series;
  verified 2026-10-18).
- Feasibility gate (D-P4-12): scratch cargo project OUTSIDE the RFS tree (e.g.
  `/workspace/host/z3_scratch`) with `z3 = "0.21"`, built with the env vars above;
  a small bitvector query must solve correctly. If it fails -> plan correction
  deferring SMT to Phase 5/6 with rationale; the exit criterion "bounded MC works
  on larger bitvector specs" is then satisfied by bounded exact MC + input
  abstraction (honest `unknown-state-limit` for used large bitvectors, full checking
  when abstractable), documented in PHASE_4_STATUS.md. STOP the task after recording
  the deferral - do not half-implement.
- If proceeding: feature-gated dependency (`[features] smt = ["dep:z3"]` or the
  current cargo idiom) so the DEFAULT build is unaffected; `--mode smt-bounded` as a
  third mode alongside exact/fuzz (D-P4-7 flag surface).
- Semantics: k-step unrolling of the transition system over bitvector variables
  (guards as constraints, variable updates as next-state equations); liveness via
  k-liveness; results CLEARLY LABELED INCOMPLETE - SMT mode can report a violation
  found within bound k (sound FAIL) or inconclusive-within-bound (unknown), never a
  completeness claim. Identical CLI semantics + results in both backends: same
  encoding from the shared plan tables, same summary-line extension (e.g.
  `mode=smt-bounded bound=k` marker - exact wire format pinned during
  implementation and parity-tested).
- C backend: generated C links the SAME libz3 (TOOLCHAIN.md 5.3) so both backends
  share one solver build - parity.

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 2. | Exact engine - Rust | mode surface + summary-line conventions being extended |
| 3. | Exact engine - C | same, C side |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 13. | Docs sync | records the SMT outcome (implemented or deferred) in all docs |
| 14. | Close-out | PHASE_4_STATUS.md documents the gate result |

### Affected source files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `Cargo.toml` | change | feature-gated `z3` dependency (default build unaffected) |
| `src/backends/rust_mc.rs` or `src/backends/rust_smt.rs` | new/change | SMT encoder + `--mode smt-bounded` runtime (Rust) |
| `src/backends/c_mc.rs` | change | SMT mode in generated C, linking libz3 |

### Affected documentation files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `TOOLCHAIN.md` | change | section 5 status line ONLY if a reinstall/version bump was needed |

## Planned actions

### Action 1: Step 0 - verify the z3 install
Run `/workspace/host/z3/bin/z3 --version` (must print 5.3.1) + a sanity query; if
missing/broken, reinstall per TOOLCHAIN.md 5.1 and update its status line.

#### Affected files
| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `TOOLCHAIN.md` | change (only if reinstalled) | section 5 status line refresh |

### Action 2: Feasibility gate - scratch build
Create `/workspace/host/z3_scratch` (outside the RFS tree), add `z3 = "0.21"`,
build with `PATH=/workspace/host/z3/bin:$PATH` +
`Z3_LIBRARY_PATH_OVERRIDE=/workspace/host/z3/bin`; run a small bitvector query.
On failure: record a plan correction deferring SMT to Phase 5/6 with the exact
error + rationale, document the fallback (bounded exact MC + abstraction) in
PHASE_4_STATUS.md input notes, and STOP.

#### Affected files
| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (scratch project outside the repo) | new | feasibility probe only; never checked in |

### Action 3 (if gate passed): feature + dependency
Add the feature-gated `z3` dependency to `Cargo.toml`; verify the default build
(`cargo build && cargo test`) is behavior-identical without the feature.

#### Affected files
| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `Cargo.toml` | change | `[features] smt` + `z3` dep (feature-gated) |

### Action 4 (if gate passed): SMT encoder + Rust mode
Shared encoding spec (bitvector variables, guard constraints, next-state equations,
k-step unrolling, k-liveness for liveness obligations) computed from the plan
tables; `--mode smt-bounded` runtime in the generated Rust binary with the pinned
summary-line extension + incomplete labeling; sound FAIL within bound / unknown
otherwise.

#### Affected files
| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/backends/rust_mc.rs` or `src/backends/rust_smt.rs` | new/change | SMT mode (Rust) |
| `src/codegen.rs` | change | `--mode smt-bounded` accepted (feature-gated codegen path) |

### Action 5 (if gate passed): C mode + parity
Mirror the encoding into generated C linking libz3 (TOOLCHAIN.md 5.3); identical
CLI semantics + results: parity test on a demo spec where exact mode is infeasible
or slow (large bitvector field) - same status/label/bound behavior both backends;
plus cross-checks on small specs (SMT-found violations must also be found by exact
mode).

#### Affected files
| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/backends/c_mc.rs` | change | SMT mode (C) |
| `tests/benchmark_mc_test.rs` or new test module | change/new | SMT parity + cross-check tests (feature-gated) |

## Task deliverables
Either:
- SMT bounded mode in both backends (feature-gated), parity-tested, labeled
  incomplete; OR
- A recorded plan correction deferring SMT to Phase 5/6 with rationale + the
  documented fallback satisfying the exit criterion.

## Task exit criteria
Gate outcome A (implemented):
- [ ] Default build (`cargo build && cargo test && cargo clippy --all-targets && cargo fmt --check`) green WITHOUT the smt feature - behavior unchanged.
- [ ] With the feature + TOOLCHAIN.md 5.2 env vars: SMT mode runs on a demo large-bitvector spec in BOTH backends with identical results; incomplete labeling present; sound FAIL / unknown only (no completeness claims).
- [ ] Cross-check: every SMT-found violation is also found by exact mode on small specs.
Gate outcome B (deferred):
- [ ] Plan correction recorded with the exact gate failure + rationale.
- [ ] Fallback documented (bounded exact MC + abstraction meets the exit criterion); default build green.

## Non-goals
- Replacing exact finite MC (SMT is bug hunting / larger-data exploration only -
  phase file non-goal), completeness guarantees, k-liveness fairness tricks beyond
  the documented policy, making SMT the default mode.

## Other information
- The scratch project and any z3 build artifacts stay OUTSIDE the RFS tree; nothing
  z3-related is checked in except the feature-gated Cargo.toml entry + generated
  code paths.
- If the `z3` crate API (0.21.x) makes k-liveness awkward, prefer a simpler
  bounded liveness encoding and document the limitation - do not expand scope.
