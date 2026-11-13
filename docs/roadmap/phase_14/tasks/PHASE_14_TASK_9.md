# Phase 14, Task 9 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal
Fix the two C example projects to the new artifact model: c_sync_handler's
handler.c includes ONLY the generated header and its Makefile builds explicit
object files (arq_fsm.o + handler.o) linked into the final executable;
fuzz_c's Makefile generates under the new flags and clang-builds the fuzz
target together with the separately generated fsm source. Scenario outputs stay
byte-identical.

## Context

- `crates/examples/c_sync_handler/` (Makefile-driven, NOT a cargo member):
  - `src/handler.c` currently does `#include "arq_fsm.c"` (whole-source
    inclusion - the exact pattern this phase eliminates) and is compiled as a
    single translation unit with `-I$(BUILD_DIR)`.
  - The Makefile generates `build/arq_fsm.c` via
    `$(RFS) codegen c production $(SPEC) --out $@` (old single-file semantics -
    under the new CLI, `--out build/arq_fsm.c` would derive the names
    `build/arq_fsm.c-fsm.h/.c`, so the generation rule MUST be rewritten to the
    new flags), then compiles only handler.c.
  - Toolchain resolution: `RFS ?= $(if $(RFS_BIN),$(RFS_BIN),rfs)`,
    `CC ?= gcc` (honours the standard CC variable for NixOS/nix-shell).
- `crates/examples/fuzz_c/` (Makefile-driven): generates
  `build/rfs_generated_fsm.c` (old flag) + the skeleton via
  `$(RFS) codegen c test $(SPEC) --out $(BUILD_DIR)/spec_test.c` then deletes
  spec_test.c (keeping only the extra-file skeleton - under the new CLI the
  skeleton is a NAMED slot, so this delete-dance disappears); clang builds ONLY
  the skeleton (`$(CLANG) -O1 -fsanitize=fuzzer,address -o $@ $(BUILD_DIR)/
  fuzz_target.c`) relying on the skeleton's .c-inclusion.
- `tests/example_projects_test.rs` pins both projects: it runs `make clean &&
  make` with RFS_BIN pinned, asserts zero gcc diagnostics (`has_gcc_diagnostic`
  greps stderr for `handler.c:` / `arq_fsm.c:`), compares the C handler's
  scenario data lines against the Rust sync_handler's (byte-identical), and for
  fuzz_c asserts the skeleton contains `#include "rfs_generated_fsm.c"` (line
  ~182) and reads `build/rfs_generated_fsm.c` (line ~184).
- Tasks 2-4 have landed: the fsm header/source pair exists, the fuzz skeleton
  includes the header, and the `rfs test` C pipelines prove the object+link /
  both-source clang shapes compile clean.

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 2. | C production header/source split | The arq_fsm.h handler.c will include. |
| 4. | C test + fuzz-target reference; `rfs test` C pipelines | The new skeleton shape + the clang build shape fuzz_c mirrors. |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 11. | Close-out | The example-project suite is part of full verification. |

### Other affected files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `crates/examples/c_sync_handler/src/handler.c` | change | `#include "arq_fsm.c"` -> `#include "arq_fsm.h"`; comment updated (no longer a single translation unit). |
| `crates/examples/c_sync_handler/Makefile` | change | Generation rule uses the new flags (explicit header/source names under build/); EXPLICIT object-file rules (arq_fsm.o, handler.o) + a link rule producing c_sync_handler from both objects. |
| `crates/examples/c_sync_handler/README.md` | change | Build description updated to the object+link shape. |
| `crates/examples/fuzz_c/Makefile` | change | Generation rules use the new flags (the test command now writes the skeleton as a named file - no delete-dance); clang compiles fsm.c + fuzz_target.c together (or their objects). |
| `crates/examples/fuzz_c/README.md` | change | Build description updated. |

### Affected test files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `tests/example_projects_test.rs` | change | Migrate the fuzz_c pins (skeleton-include assertion -> header include; generated-file reads -> new names); keep the c_sync_handler scenario-parity + zero-diagnostic bars. |

## Planned actions

### Action 9.1 - c_sync_handler: header include + explicit objects

Rewrite `src/handler.c`'s include to `"arq_fsm.h"` and update its header comment
(it is no longer a single translation unit - the FSM lives in arq_fsm.o). In
the Makefile: the generation rule becomes
`$(RFS) codegen c production $(SPEC) --out-fsm-header $(BUILD_DIR)/arq_fsm.h
--out-fsm-code $(BUILD_DIR)/arq_fsm.c`; add explicit object rules
(`$(CC) -Wall -I$(BUILD_DIR) -c $(BUILD_DIR)/arq_fsm.c -o $(BUILD_DIR)/
arq_fsm.o` and the same for handler.c) and a link rule
(`$(CC) -o c_sync_handler $(BUILD_DIR)/handler.o $(BUILD_DIR)/arq_fsm.o`);
update `clean` to remove the objects. The scenario output must stay byte-
identical (the test compares data lines against the Rust example).

Rationale: the user's concrete requirement - handler.c includes only the
header, arq_fsm.o is linked with handler.o, object generation is explicit in
the Makefile.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `crates/examples/c_sync_handler/src/handler.c` | change (include + comment) | Header-only include. |
| `crates/examples/c_sync_handler/Makefile` | change (generate rule) | New flags, explicit pair names. |
| `crates/examples/c_sync_handler/Makefile` | change (build rules) | Explicit .o rules + link. |
| `crates/examples/c_sync_handler/README.md` | change | Build-shape description. |

### Action 9.2 - fuzz_c: new generation flags + both-source clang build

Rewrite the Makefile's generation rules to the new CLI (the test command writes
fuzz_target.c as a named slot plus co-emits the fsm pair - the old
generate-then-delete spec_test.c dance is gone); the build rule becomes
`$(CLANG) -O1 -fsanitize=fuzzer,address -o $@ $(BUILD_DIR)/fuzz_target.c
$(BUILD_DIR)/<fsm source>` (both sources compiled together - the skeleton no
longer source-includes the .c). Update README.md accordingly.

Rationale: mirrors task 4's proven clang shape; the example is the documented
manual equivalent of `rfs test --suite fuzz --backend c`.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `crates/examples/fuzz_c/Makefile` | change (generate + build rules) | New flags; both-source clang build. |
| `crates/examples/fuzz_c/README.md` | change | Build-shape description. |

### Action 9.3 - Migrate example_projects_test

Update the fuzz_c pins: the skeleton-include assertion becomes a header-include
assertion; the generated-file reads use the new names; the c_sync_handler leg
keeps its zero-gcc-diagnostic bar (the diagnostic grep already covers
`arq_fsm.c:` - now emitted from compiling arq_fsm.o) and its scenario-parity
comparison. Run the suite.

Rationale: the suite is the standing proof that the examples build and behave
identically under the new model.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `tests/example_projects_test.rs` | change (fuzz_c pins ~182-184) | New include assertion + file names. |

## Task deliverables
- c_sync_handler: header-only include, explicit object files, linked
  executable, byte-identical scenario output.
- fuzz_c: new-flag generation, both-source clang build, bounded smoke run
  exits 0.
- Migrated example-project suite, green.

## Task exit criteria
- [ ] `make clean && make` in c_sync_handler succeeds with ZERO gcc -Wall
      diagnostics; the binary's scenario data lines are byte-identical to the
      Rust sync_handler's (existing parity assertion passes unmodified).
- [ ] handler.c contains `#include "arq_fsm.h"` and no `.c` include; the
      Makefile builds arq_fsm.o + handler.o explicitly and links them.
- [ ] `make clean && make` in fuzz_c succeeds (clang present); a bounded smoke
      run (`./fuzz_target -max_total_time=10`) exits 0.
- [ ] `cargo test --test example_projects_test` green.

## Non-goals
- No Rust example crate changes (sync_handler/async_handler/fuzz_rust are cargo
  members whose build model is untouched by artifact separation).
- No new examples.
- No changes to the spec files the examples consume.

## Other information

- The c_sync_handler zero-diagnostic bar must hold for BOTH object compiles -
  a warning in arq_fsm.o's compile fails the test exactly as a warning in
  handler.c's would.
- If clang is absent in the environment, fuzz_c's build leg is skipped by the
  suite (existing availability gating) - generation still runs and is asserted.
