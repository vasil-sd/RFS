# Phase 14, Task 2 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal
Split the C `production` artifact into an idiomatic header/source pair: the
header carries the include guard, the line-1 artifact header, all type
definitions and the public API declarations; the source includes its own
header and carries the pragma, static helpers and all function bodies. Both
files carry their own line-1 artifact header (D-P6-3).

```
rfs codegen c production spec.rfs --out arq
  -> arq-fsm.h   (types + API declarations)
  -> arq-fsm.c   (#include "arq-fsm.h" + implementation)
```

## Context

- The C production generator (`src/codegen/backends/c/fsm.rs`) builds ONE string
  via `Gen`: `emit_header` (line-1 artifact header, std includes
  `<stdint.h>/<stdbool.h>/<string.h>/<stdio.h>/<stdlib.h>`, the
  `-Wunused-function` pragma block, `RFS_SPEC_VERSION` define when the spec has
  a version, `RFS_PROPERTY_MONITORS 1` define in Gated mode), then
  `emit_types` (non-State enums as `<Name>_t` typedefs, the State enum as
  `state_t`, input/output unions, `step_result_t`, `fsm_t`, policy/monitor
  types), then `emit_monitors` and `emit_fsm_impl` (static helpers +
  `fsm_new` / `fsm_step` + trace/debug helper API).
- The "shared helper API" comment in `emit_header` explains why the pragma
  exists: a given program uses only a subset of the static helpers, so
  `-Wunused-function` on the rest is expected - the pragma must stay with the
  IMPLEMENTATION file (where the statics live), not the header.
- Public surface that consumers (handler.c, test/mc/fuzz artifacts) need from
  the header: all typedefs/enums/unions, `fsm_new`, `fsm_step`, the policy enum
  (`rfs_policy_t` + `RFS_CONTINUE`/... constants), and the monitor/assumption
  constructor surface that is NOT static. Everything declared `static` / `static
  inline` stays in the .c.
- Task 1 has already landed: `generate_production` returns the slot struct with
  the current single .c in `fsm_code`; this task fills `fsm_header` as well and
  re-splits the content. The CLI write loop already writes both slots when
  present (C production derives `BASE-fsm.h` + `BASE-fsm.c`).
- Compile-bar tests: generated C must compile warning-free under `gcc -Wall`
  (and clang where available - see `tests/c_compiler_agnostic_test.rs` for the
  dual-toolchain pattern).

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 1. | Codegen output model + CLI flag family | Provides the fsm-header/fsm-code slots and the write loop that emits both files. |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 3. | C mc artifact pair + fsm reference | mc.c includes the header this task produces. |
| 4. | C test + fuzz-target reference; `rfs test` C pipelines | test.c / fuzz_target.c include the header. |
| 9. | Example fixes | c_sync_handler links arq_fsm.o built from this task's source file. |

### Affected source files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `src/codegen/backends/c/fsm.rs` | change | Split the single-string emission into header part + source part; `generate_production` fills both slots. |

### Affected test files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `tests/codegen_test.rs` | change | C production legs pin the PAIR: header self-containedness, source includes its own header, content assertions re-pointed at the right slot, `gcc -Wall -c` on EACH file. |

## Planned actions

### Action 2.1 - Split the emission into header and source parts

Refactor `Gen` in `src/codegen/backends/c/fsm.rs` to build TWO strings.
Header part: line-1 artifact header (same text as today's, so D-P6-3 drift
detection works on both files), an include guard (`#ifndef <BASE>_FSM_H` /
`#define` / `#endif` - the guard macro derived from the module name,
mangled like other generated identifiers), the std includes the TYPES need
(`<stdint.h>`, `<stdbool.h>`, `<string.h>`), the `RFS_SPEC_VERSION` and
`RFS_PROPERTY_MONITORS` defines, ALL type definitions (enums, unions,
`step_result_t`, `fsm_t`, policy/monitor types), and the public API
declarations (`fsm_new`, `fsm_step`, non-static monitor constructors). Source
part: line-1 artifact header, `#include "<own header name>"` (the name is
resolved by the CLI layer - pass it in via `BackendOptions`, see Action 2.2),
the remaining std includes the implementation needs (`<stdio.h>`,
`<stdlib.h>`), the `-Wunused-function` pragma block, all static helpers and
function bodies. The property-monitor `#ifdef RFS_PROPERTY_MONITORS` wrapping
(`prop_wrap`) applies exactly as today - declarations that are monitor-only go
into the header inside the same guard so consumers see them only when the
define is active.

Rationale: the split follows the existing emission order (header/types first,
impl after), so this is a re-partitioning of already-correct code, not a
rewrite; keeping the pragma with the statics preserves the warning-free
invariant under both gcc and clang.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/codegen/backends/c/fsm.rs` | change (`emit_header` / `emit_types` region) | Re-partitioned into header-string emission. |
| `src/codegen/backends/c/fsm.rs` | change (`emit_monitors` / `emit_fsm_impl` region) | Re-partitioned into source-string emission; statics + pragma stay here. |

### Action 2.2 - Plumb the own-header name into the generator

The source file must `#include` its own header by the RESOLVED name (which may
differ from the default, e.g. `--out-fsm-header include/arq.h`). Add the
resolved fsm reference names to `BackendOptions` (fields such as
`fsm_header_name: Option<String>` / `fsm_source_name: Option<String>` - the
CLI layer and `rfs test` set them from their resolved paths; library callers
that generate standalone content use a fixed default like `rfs_fsm.h`).
`generate_production` uses the header name for the include guard AND for the
source's `#include` line.

Rationale: without this, an explicitly named header would produce a source
file that includes the WRONG name - the reference must be a data input to the
generator, not a CLI-side string rewrite.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/codegen/mod.rs` | change (`BackendOptions`) | New resolved-name fields + Default values. |
| `src/cli/commands/codegen.rs` | change (options construction) | Stamp the resolved fsm names into options before dispatch. |
| `src/cli/commands/test.rs` | change (options construction) | Same for the `rfs test` pipelines. |

### Action 2.3 - Pin the pair in tests

Update the C production legs of `tests/codegen_test.rs`: assert BOTH slots are
filled; the header compiles ALONE (`gcc -Wall -c` on a translation unit that
only includes it - self-containedness); the source compiles with `-I` pointing
at the header; the union of emitted declarations is unchanged (existing
content assertions re-pointed at the correct slot); the include guard and both
line-1 headers are present. Keep the `--with-property-monitors` toggle test
working against the split output (the define lands in the header).

Rationale: the pair is only as good as its compile bar - a header that needs
the .c to compile, or a source that includes a nonexistent name, would break
every dependent task silently.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `tests/codegen_test.rs` | change (C production legs) | Pair assertions + per-file compile checks. |

## Task deliverables
- C `production` emits a header/source pair with correct include guard,
  dual line-1 headers, and the pragma in the source.
- `BackendOptions` carries the resolved fsm reference names; CLI + `rfs test`
  stamp them.
- Tests pinning the pair's compile bar and content split.

## Task exit criteria
- [ ] `rfs codegen c production <spec> --out arq` writes `arq-fsm.h` +
      `arq-fsm.c`; each compiles warning-free under `gcc -Wall -c` (header
      alone, source with `-I.`).
- [ ] No `static` symbol is declared in the header; no type definition lives
      in the source.
- [ ] Both files carry the line-1 artifact header with the current spec hash.
- [ ] Existing C production content assertions pass against the re-partitioned
      slots; `cargo test` (root) green; fmt clean; clippy at baseline.

## Non-goals
- No change to the Rust backend (task 5).
- No change to what mc/test/fuzz artifacts contain (tasks 3-4) - they still
  emit their current self-contained content in this task.
- No Makefile/example changes (task 9).

## Task corrections

| Correction | Description + rationale |
|------------|-------------------------|
| Action 2.3 affected-test-files list was incomplete | The task file listed only `tests/codegen_test.rs`, but the split makes the generated source non-self-contained, so EVERY test leg that writes `primary_code()` (now the source) as a standalone `.c` and compiles it broke with "header not found". Re-pointed/fixed beyond the listed file: `tests/info_format_test.rs` (C production now fills 2 slots + header line-1 stamp), `tests/common/prod_driver.rs` (shared helper writes the header into the per-tag dir - fixes all prod_driver-based suites), `tests/toolchain_fuzz_test.rs`, `tests/monitor_integration_test.rs`, `tests/wide_ops_parity_test.rs` (each writes the header alongside the source), and `tests/utils_test.rs` (`--print-script` pins regenerated for the new three-heredoc script). Rationale: the compile bar is only meaningful if the header is present wherever the source is compiled. |

## Other information

- When deciding which monitor/assumption constructors are public vs static,
  follow the EXISTING visibility in today's single file - do not change any
  symbol's linkage in this task (that would be a functionality change).
- The include-guard macro must survive identifier mangling (module names can
  contain characters illegal in macros) - derive it from the same mangled form
  used for other generated identifiers, uppercased.
