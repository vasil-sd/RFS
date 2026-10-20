# C Codegen Backend

Architecture and design decisions of the C target-language backend
(`src/backends/c*.rs`). For the generic plugin API see `CODEGEN.md` at the
repository root; for semantics see `DESIGN.md`; for the reference implementation
see `RUST.md`.

## 1. File layout

| File | Role |
|---|---|
| `c.rs` | Plugin entry: `C_BACKEND` static plus shared C helpers: `c_ident` (keyword-safe snake_case identifier), `bit_c_type` (fixed-width mapping), `c_type` (IR type -> C type). |
| `c_fsm.rs` | `production` command: emits the standalone FSM source. Core reused by `test` and `mc`. |
| `c_test.rs` | `test` command: production code + plain-assert test harness with its own `main`. |
| `c_mc.rs` | `mc` command: (default exact / legacy fuzz) production core + mirror of the Rust exact engine (pthreads for `--threads`, C11 atomics, gcc -Wall clean) + fuzz runtime + generated `main` dispatching on `--mode`; (SMT mode) standalone binary linking libz3. |

Dependency direction mirrors the Rust backend: `c_mc -> c_fsm`,
`c_test -> c_fsm`, all three -> shared helpers in `c.rs` and the
language-agnostic core modules (`codegen_fsm`, `codegen_test`, `codegen_mc`).

## 2. Generated artifact design

Every command emits **one self-contained `.c` file**:

- includes only C standard headers (`stdint.h`, `stdbool.h`, `string.h`,
  `stdio.h`, `stdlib.h`); no project or third-party dependencies;
- compiles warning-clean with `gcc -Wall`; deterministic output.

### Type mapping (decision D8, DESIGN.md section 20)

| RFS type | C type |
|---|---|
| `bool` | `bool` (stdbool.h) |
| `bit<8/16/32/64>` | `uint8_t` / `uint16_t` / `uint32_t` / `uint64_t` |
| other `bit<N>` | codegen error: "C backend supports bit widths 8, 16, 32, 64" |
| enum `E` | `E_t` (typedef enum) |

Arithmetic on `bit<N>` wraps natively in the fixed-width type (no wider
intermediate; results are identical to the Rust backend's u128+mask strategy).

## 3. Value representation

C has no algebraic data types, so inputs and outputs use the classic
**tagged union**: one payload struct per flattened variant plus a `kind` enum:

```c
typedef struct { uint16_t src_port; } input_recv_syn_payload_t;
typedef enum { INPUT_RECV_SYN, INPUT_TICK, ... } input_kind_t;
typedef struct { input_kind_t kind; union { input_recv_syn_payload_t input_recv_syn; ... } payload; } input_t;
```

Outputs use the same shape (`output_t`, `OUTPUT_*` kinds). The step result
carries a fixed-size output array: `#define RFS_MAX_OUTPUTS n` where n is the
maximum emit count over all transitions and the on_error block.

## 4. Semantics parity with the Rust backend

- **Priority-ordered arms**: per state, transition blocks are emitted in
  descending effective priority (default 0), same ordering as Rust (D1/D5).
- **Simultaneous updates**: all assignment RHS evaluate into `tmp_*` locals
  from the old vars before any write.
- **on_error fallback**: when the module declares one, it is the final
  catch-all in `fsm_step` instead of `RFS_UNEXPECTED_EVENT`.
- **Assumption handling**: function-pointer handler
  (`rfs_policy_t (*handler)(void* user, const char* label)`) with
  Continue/Stop/Panic policies - the C analogue of the Rust trait.
- **Online monitors (Phase 3, D-P3-5)**: the SAME shared `MonitorPlan` as the
  Rust backend is rendered in C - per-label monitor structs (`mon_<label>_t`
  with `_in_N` input-match helpers, `_sig_N` signal expressions, and a `_step`
  function) mirroring the Rust rendering, evaluated over `rfs_sample_t`
  (the C analogue of Rust's `Sample`: `input`, `outputs`, `from_state`,
  `to_state`, `vars`). Property monitors are gated by
  `#ifdef RFS_PROPERTY_MONITORS` (defined when `--with-property-monitors` is
  used for the production command); assumption monitors are always active and
  go through the existing handler path. A confirmed property violation sets
  `RFS_PROPERTY_VIOLATION` in `step_result_t` with `prop_label`, `prop_step`,
  and `prop_trigger_step` (the C analogue of Rust's
  `StepError::PropertyViolation`). Out-of-subset formulas (R1-R4) are skipped
  with the IDENTICAL warning text as Rust.
- **Version header**: emitted as `#define RFS_SPEC_VERSION "..."`.

## 5. Model checker (Phase 4)

The generated MC executable is a MIRROR of the Rust exact engine: the same
embedded plan tables, the same level-synchronous BFS over packed configurations
with canonical successor order, the same three-status taxonomy and pinned
summary line (`model-check summary: status=<s> states=N transitions=M
max_depth=D` [+ ` reason=<r> label=<L> step=S`]), and byte-identical
counterexample fields (property violation, deadlock, liveness lasso). The C
implementation uses a sorted vector + binary search for the visited set -
behavior must not depend on the container choice (D-P4-2), and parity is
verified by `tests/backend_parity_test.rs` (field identity over all
property-bearing examples) and `tests/liveness_mc_test.rs` (lasso fields).

- Liveness engine mirror: per-leaf progress automata, product limit vectors,
lasso reconstruction, `--nondet` strong-fairness filter, truncated-mode
  soundness; per-property edge-label cache (a shared cache mislabels edges from
  the second property on - found and fixed in task 7).
- `--threads N > 1`: pthreads mirror of the Rust two-phase expansion (workers
  over disjoint frontier ranges + sequential canonical commit) with a C11
  `atomic_int` abort flag; inline sequential fallback if `pthread_create`
  fails (results stay canonical). Byte-identical to `--threads 1` for any N;
  ASan/UBSan-clean (TSan unavailable in this environment - recorded).
- Legacy fuzz mode (`--mode fuzz`) retains the Phase 1/3 CLI wire format
  (`--bound/--runs/--seed/--assumes/--violate/--unexpected-prob`) and LCG
  constants (6364136223846793005 / 1442695040888963407, top-53-bit double
  mapping).
- SMT mode: links the SAME libz3 5.3.1 as the Rust backend (one solver build,
  parity); the generated C code targets the z3 5.3.1 API surface
  (`Z3_L_UNDEF`, no `Z3_del_solver`, `Z3_model_eval` bool + out-ast).
- Every generated MC binary must compile with `gcc -Wall` at ZERO warnings;
  the test suites' `compile_c` helper asserts this.

## 6. Known limitations

- `bit<N>` widths outside {8,16,32,64} are rejected (the Rust backend accepts
  any width up to 128 via u128 intermediates).
- No aggregate C test crate: each module's tests are a standalone executable
  (the Rust aggregate-crate tooling is Rust-specific by design).
