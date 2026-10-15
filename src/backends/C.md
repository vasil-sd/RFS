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
| `c_mc.rs` | `mc` command: production code + model-checker runtime (`main`) with the same CLI wire format and LCG as the Rust MC. |

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
- **Translated safety subset**: same formula subset as Rust (via
  `codegen_fsm::safety_formula_inner`), rendered over a dynamic trace array;
  out-of-subset formulas are skipped with warnings, exactly like Rust.
- **Version header**: emitted as `#define RFS_SPEC_VERSION "..."`.

## 5. Model checker wire format

The generated MC executable accepts the identical CLI as the Rust one
(`--bound`, `--runs`, `--seed`, `--assumes`, `--violate label:prob:percent`,
`--unexpected-prob`) and uses the same LCG constants
(6364136223846793005 / 1442695040888963407, top-53-bit double mapping), the
same three-phase step strategy (forced violation -> unexpected injection ->
allowed random), and prints the same summary line format. A run that violates
a property prints a counterexample trace and exits 1.

## 6. Known limitations

- `bit<N>` widths outside {8,16,32,64} are rejected (the Rust backend accepts
  any width up to 128 via u128 intermediates).
- No aggregate C test crate: each module's tests are a standalone executable
  (the Rust aggregate-crate tooling is Rust-specific by design).
