# Phase 6, Task 6 - Implementation details

> Written for a future LLM session: after reading `START.md`,
> `docs/roadmap/INTRO.md` and this file, the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal
Deliver PHASE_6.md's "Benchmark suite": three new protocol-like benchmark specs
(HTTP/2-like stream state machine, USB-like control transfer FSM, CAN-like
arbitration/state machine) as valid examples, an extended benchmark harness with
the full tracked-metric set (parse time, codegen time, MC states explored, MC
time, memory usage, counterexample quality) per backend, and CI wiring.

## Context

- **Existing suite**: `tests/benchmark_mc_test.rs` drives specs 43/44/45 x both
  backends in exact mode: pass-complete statuses, pinned deterministic metrics
  (`Pin { states, transitions, max_depth, monitors_bits }` - values recorded in
  Phase 4 tasks 6/8), inline FAIL variants (safety + liveness) with counterexample
  parity, `--threads 1/2/4` byte-identity gate + `--fast` smoke (task 10B), JSON
  report at `target/mc_benchmarks.json` + per-backend summary table. Wall-clock
  times are INFORMATIONAL only - never asserted (the suite runs inside
  `cargo test` and must not become a flaky timing gate). Keep that discipline.
- **Benchmark set decision**: PHASE_6.md lists simple handshake + timeout/retry
  as benchmark specs - those exist as examples 04/03 but carry NO properties
  (verified: no `prop `/`assume ` lines), so MC metrics on them would be bare
  reachability. Decision: 03/04 join the suite for PARSE/CODEGEN-TIME metrics
  only; MC metrics apply to the property-bearing set {43, 44, 45, 47, 48, 49}.
  Do NOT modify existing examples (they are pinned everywhere).
- **New spec sketches** (design guidance - keep payload widths MINIMAL so exact
  MC stays tractable; target <= ~150k states per the `44_token_ring` precedent of
  129,985; MEASURE with both backends before pinning and shrink if exceeded):
  - `47_http2_stream.rfs` - HTTP/2-like stream state machine: states
    Idle/Open/HalfClosedRemote/HalfClosedLocal/Closed; events SEND_DATA(w:
    bit<4>), WINDOW_UPDATE(w: bit<4>), END_STREAM, RST_STREAM, ACK; vars
    send_window: bit<5>, recv_window: bit<5>; safety: no SEND_DATA when
    send_window = 0, no transitions out of Closed; liveness: bounded_eventually
    Closed after END_STREAM (with a window-drain bound).
  - `48_usb_control_transfer.rfs` - USB-like control transfer: states
    Idle/SetupSent/DataStage/StatusStage/Stalled/Error; events SETUP,
    DATA(n: bit<4>), ACK, NAK, STALL, RESET; vars data_left: bit<3>, retries:
    bit<2>; safety: no StatusStage before DataStage complete, no double SETUP;
    liveness: bounded_eventually (StatusStage or Stalled) after SETUP.
  - `49_can_arbitration.rfs` - CAN-like arbitration/state machine: states
    Idle/Arbitration/Data/Ack/ErrorPassive/ErrorActive; events TX_REQ(id:
    bit<4>), ARB_WIN, ARB_LOSE, ACK_OK, ACK_ERR, BUS_OFF, RECOVER; vars
    error_counter: bit<5>, tx_id: bit<4>; safety: no Data without ARB_WIN,
    ErrorActive only when error_counter exceeds the pinned threshold;
    liveness: bounded_eventually Idle after a completed frame.
  Each spec gets a module version header `version "1.0"` (task 2's formalized
  mechanism) - they are the first v1.0-versioned examples.
- **New report fields** (codegen change - re-pins required):
  - `peak_rss_kb`: ACTUAL peak resident set, self-reported by the MC binary,
    added to the JSON report's `statistics` object next to
    `memory_estimate_bytes` (resource accounting lives there). Rust generated
    binary: read `VmHWM` from `/proc/self/status` before exit (std::fs only,
    Linux; on read failure emit 0 - documented, keeps both backends emitting the
    field unconditionally so parity holds). C generated binary:
    `getrusage(RUSAGE_SELF)` via `#include <sys/resource.h>` + extern-free POSIX
    call (gcc -Wall clean). Add `JSON_KEY_PEAK_RSS_KB` const + `McJsonReport`
    field + builder rendering in `src/codegen_mc.rs`; both backends' embedded
    writers emit it. Re-pin the D-P5-10 document-shape byte-pins + golden
    full-document pin (lib codegen_mc tests) - the parity JSON suite picks up the
    new field automatically (both backends emit identical documents).
  - `ce_steps`: counterexample QUALITY metric, added inside the `counterexample`
    object: trace -> number of steps; lasso -> prefix + cycle length;
    deadlock-config -> depth. Add const + rendering in `src/codegen_mc.rs` +
    both backends; same re-pins as above.
  - Parse time + codegen time: harness-measured (in-process `rfs` lib calls
    wrapped in `Instant`), INFORMATIONAL only, written to the JSON report +
    printed table - never asserted.
- **FAIL variants + CE quality pins**: for each NEW spec craft inline FAIL
  variants the Phase 4 way (mutated property bound / injected bug) with pinned
  label + step + `ce_steps`; parity across backends (byte-identical JSON).
- **CI wiring (D-P6-9)**: no `.github/workflows` exists in this environment -
  "runs in CI" = the standard verification pipeline (`cargo test --test
  benchmark_mc_test`, one command, writes `target/mc_benchmarks.json` + prints
  the per-backend table) plus a GitHub Actions snippet documented in BUILD.md so
  any external CI can run it.

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 2. | Stability and versioning (v1.0) | artifacts carry the final headers; the new examples use `version "1.0"` |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 7. | External user documentation set | EXAMPLES.md walks through the new benchmark specs |
| 8./9. | Training / docs sync | train on and document the new specs + metrics |

### Affected source files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `src/codegen_mc.rs` | change | `McJsonReport` fields (`peak_rss_kb`, `ce_steps`) + key consts + builder rendering |
| `src/backends/rust_mc.rs` | change | VmHWM reader + new report fields in the embedded writer |
| `src/backends/c_mc.rs` | change | getrusage + new report fields in the embedded writer |

### Affected test files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `tests/benchmark_mc_test.rs` | change | extended benchmark set, new metrics, FAIL variants + CE quality pins |
| lib codegen_mc JSON tests | change | D-P5-10 document-shape byte-pins + golden full-document pin re-pinned for the two new fields |
| `tests/examples_test.rs` | change | count re-pin 50 valid / 13 invalid (plan correction 2a adds example 50 before this task) |

### Other affected files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `examples/valid/47_http2_stream.rfs` | new | HTTP/2-like stream state machine benchmark spec |
| `examples/valid/48_usb_control_transfer.rfs` | new | USB-like control transfer benchmark spec |
| `examples/valid/49_can_arbitration.rfs` | new | CAN-like arbitration/state machine benchmark spec |
| `BUILD.md` | change | "Benchmarks in CI" section (single command + JSON report + GitHub Actions snippet) |

## Planned actions

### Action 1: new benchmark specs (subtask 6.1)
Write the three specs per the sketches (module version header `version "1.0"`,
safety + liveness properties each). Iterate until exact-mode MC on BOTH backends
completes within the state budget (measure states/transitions/max_depth; shrink
payload widths if > ~150k states). Re-pin `tests/examples_test.rs` to 50 valid /
13 invalid (plan correction 2a adds example 50 before this task). Regenerate the aggregate generated-code crate (`cargo
generate-code-tests`) and keep `cargo test-generated-code` green (three new
modules appear).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `examples/valid/47_http2_stream.rfs` | new | benchmark spec |
| `examples/valid/48_usb_control_transfer.rfs` | new | benchmark spec |
| `examples/valid/49_can_arbitration.rfs` | new | benchmark spec |
| `tests/examples_test.rs` | change | count re-pin 50/13 (plan correction 2a) |

### Action 2: report field extension (subtask 6.2a)
Add `peak_rss_kb` (statistics) + `ce_steps` (counterexample) per Context:
`src/codegen_mc.rs` model + consts + builder; `rust_mc.rs` VmHWM reader +
emitted field; `c_mc.rs` getrusage + emitted field. Re-pin the D-P5-10
document-shape byte-pins + golden full-document pin (lib codegen_mc tests) and
verify the parity JSON suite still passes byte-identical across backends. Verify
`gcc -Wall` stays clean on the modified C MC output.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/codegen_mc.rs` | change | report model + rendering |
| `src/backends/rust_mc.rs` | change | VmHWM + fields |
| `src/backends/c_mc.rs` | change | getrusage + fields |
| lib codegen_mc JSON tests | change | shape pins + golden pin re-pins |

### Action 3: harness extension (subtask 6.2b)
Extend `tests/benchmark_mc_test.rs`: BENCH_SPECS += 47/48/49 with fresh Pins
(measured values, recorded in the test file like Phase 4's); parse + codegen
time metrics for {03, 04, 43, 44, 45, 47, 48, 49} (in-process lib calls,
informational); `peak_rss_kb` + `ce_steps` consumed from the JSON report into the
per-backend table; FAIL variants for each new spec with pinned label + step +
ce_steps and cross-backend parity. Keep wall-clock/RSS informational (order-of-
magnitude sanity bound only, e.g. peak_rss_kb < 1_000_000).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `tests/benchmark_mc_test.rs` | change | extended suite |

### Action 4: CI wiring (subtask 6.3)
BUILD.md "Benchmarks in CI" section: the single command (`cargo test --test
benchmark_mc_test`), what it writes (`target/mc_benchmarks.json`) and prints
(per-backend table), the metric list + which are asserted vs informational, and
a GitHub Actions snippet (yaml block, documented only - not committed as a
workflow).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `BUILD.md` | change | the section above |

## Task deliverables
- Three new v1.0-versioned benchmark specs (47/48/49) as valid examples.
- MC JSON report extended with `peak_rss_kb` + `ce_steps` (both backends,
  byte-identical parity preserved).
- Extended benchmark suite: six property-bearing specs x both backends, full
  metric set, FAIL variants with CE quality pins.
- BUILD.md "Benchmarks in CI" section with the GitHub Actions snippet.

## Task exit criteria
- [x] Specs 47/48/49 parse+validate, carry `version "1.0"`, exact MC completes on both backends within the state budget; examples_test green at 50 valid / 13 invalid; aggregate crate regenerated + green.
- [x] Report JSON carries `peak_rss_kb` (statistics) + `ce_steps` (counterexample) byte-identically across backends; D-P5-10 pins re-pinned; gcc -Wall clean.
- [x] Benchmark suite green: all Pins measured + recorded, FAIL variants pinned (label + step + ce_steps), parse/codegen time + RSS present as informational data.
- [x] BUILD.md section present with the single-command entry point + Actions snippet.
- [x] Full `cargo test` green; clippy/fmt clean.

## Non-goals
- No wall-clock or RSS ASSERTIONS (informational only, per the Phase 4 discipline); no SMT benchmarks; no modification of existing examples 03/04/43/44/45; no committed CI workflow files (none exist in this environment - documented snippet only).

## Other information
- Measured state counts for 47/48/49 (action 1; identical on both backends -
  these are the Pins in `tests/benchmark_mc_test.rs` and inform task 9's docs):

  | spec | states | transitions | max_depth | monitors_bits |
  |------|--------|-------------|-----------|---------------|
  | 47_http2_stream | 434 | 21857 | 7 | 10 |
  | 48_usb_control_transfer | 104 | 340 | 15 | 11 |
  | 49_can_arbitration | 240 | 448 | 41 | 7 |

  All far below the ~150k state budget; no abstraction shrinkage was needed.
- If a spec cannot be made tractable without losing its protocol character,
  shrink its abstraction (fewer window/retry levels) rather than dropping it -
  record the choice in "Task corrections".

## Task corrections
- **6a** (action 1): pattern bindings that collide with identifiers a generated
  body uses unqualified now get mangled (`<name>__rfsb`) in BOTH backends -
  spec 48's binding `n` shadowed C MC's effect counter, silently dropping
  successors through emitting transitions (out-of-bounds write for wide fields).
  Plan correction 6a.
- **6b** (action 1): the test planner's `expect_progress` heuristic
  (`!transitions.is_empty()`) over-approximated - spec 47's guarded initial arm
  (`w > 0` vs representative w = 0) generated a failing positive drive. Now
  computed exactly as the driver runs (`expect_progress_from_initial`). Plan
  correction 6b.
- **6c** (action 3): `48_usb_control_transfer.rfs` - removed the `Error on Setup`
  absorbing arm. The action-3 FAIL variant (unmodified spec run with
  `--assumes none`) exposed that once the Error state becomes reachable, its
  absorbing Setup arm produces samples with `input_is(Setup)` in a non-SetupSent
  state, violating the spec's own safety invariant `no_double_setup` (reported
  at step 6) and shadowing the intended `completes_after_setup` demonstration.
  Removing the arm makes Error consistent with Stalled (RESET-first
  re-initialization; SETUP there is an unexpected termination) and keeps the
  spec self-consistent under relaxed assumptions. The pass run under
  `env_progress` is unchanged (Error unreachable: 104/340/15 - no other pin
  moved). Plan correction 6c.
- **6d** (action 3, design note): RFS transition priorities fire DESCENDING
  (SEMANTICS 3.4 - higher `priority N` first). Spec 49's AckErr arms are labeled
  P1/P2/P3 as if lower = higher, but their guards are disjoint so behavior was
  correct; the 49 FAIL variant design accounts for the actual firing order (a
  guard-overlap mutation would have been silently preempted by the priority-3
  arm). No spec change needed.
