# RFS Migration Guide and Compatibility Policy

User-facing compatibility policy for the RFS toolchain, plus the record of
what changed between released versions and how to migrate. Written as part of
the v1.0 stabilization (Phase 6, decisions D-P6-1..D-P6-3 in DESIGN.md;
task files `docs/roadmap/PHASE_6_TASK_2.md` action 6).

Companion documents: `docs/SEMANTICS.md` (normative language semantics),
`DESIGN.md` (toolchain decisions, incl. the D10 IR evolution rule this policy
implements), and - from Phase 6 task 7 onward - the rest of this `docs/user/`
set (`PROPERTIES.md`, `MODEL_CHECKING.md`).

---

## 1. Versioning scheme

The toolchain uses calendar-free semver on the crate version (`rfs` in
`Cargo.toml`, surfaced as `CARGO_PKG_VERSION`): `MAJOR.MINOR.PATCH`.

- **MAJOR bump**: something in the STABLE surface (section 2) changed in a
  breaking way. Existing specs, generated code, or scripts may stop working.
  A new entry is added to section 4 describing the migration.
- **MINOR bump**: additive changes only - new commands, new flags with
  defaults, new optional IR JSON fields, new examples. Everything that worked
  before keeps working byte-for-byte (except where a spec itself changed).
- **PATCH bump**: bug fixes and internal changes with no surface effect.

Two more version numbers travel with the toolchain and are independent of it:

- The **IR schema version** (`ir_version` in IR JSON; `IR_VERSION` in
  `src/ir.rs`). See section 2.3 for its evolution rule.
- The optional **syntax version header** inside a `.rfs` file
  (`version "X.Y"` / `version "X.Y.Z"`). This is the spec author's own
  declaration, NOT the toolchain version; see section 2.4.

## 2. What is STABLE within a major version

### 2.1 Generated-code public API (names and behavior)

Everything a user program compiles against in a generated artifact keeps its
name and behavior across MINOR and PATCH bumps of the toolchain, per module:

**Rust backend** (`rfs codegen rust production|test|mc`): the `State`,
`Input`, `Output` enums; the `Vars` struct; `Fsm` with `Fsm::new` / `Fsm::step`
/ `Fsm::step_with_handler`; the `StepError` enum; the trace types `Trace`,
`TraceStep`, `Sample`; the assumption machinery `AssumptionPolicy`,
`AssumptionHandler`, `ContinueAssumptionHandler`, `StopAssumptionHandler`,
`PanicAssumptionHandler`; and, when properties are present, the monitor types
`PropMonitors`, `PropertyViolation`, `AssumeMonitors`. Step semantics follow
`docs/SEMANTICS.md` - a generated machine is a faithful rendering of it.

**C backend** (`rfs codegen c production|test|mc`): the per-module typedefs
(`state_t`, `input_kind_t`, the vars struct, `step_result_t`, ...), and the
function surface `fsm_new` / `fsm_step` / `fsm_free` plus the assumption-hook
types (`rfs_policy_t`, `rfs_assumption_handler`). C artifacts are rendered to
be warning-free under `gcc -Wall`; that is part of the contract.

What is NOT covered: anything below the public names - internal helper
functions, layout of structs, code shape, and (since v1.0) line 1 of the
header (section 2.5).

### 2.2 CLI surface

Every command present at a MAJOR tag - its name, its flags and their
meaning, its exit codes, and what it writes to stdout/stderr - is stable until
the next MAJOR bump. As of this writing the commands are `parse`, `validate`,
`info`, `test`, `diagram`, `codegen`; Phase 6 adds `mc-ref` and `gen-spec`
before the v1.0 tag, so the v1.0 surface is those eight; Phase 7 adds the
`rfs diagram --renderer` flag and the additive `rfs info format` sub-mode
before the v1.1 tag (recorded in section 4). Rules:

- Adding a command or a flag (with a default that leaves existing behavior
  unchanged) is MINOR.
- Changing a flag's meaning, an exit code, or stdout bytes for an existing
  invocation is MAJOR.
- `--print-script` output is deterministic and byte-stable for a given
  toolchain version + spec + resolved tools; it embeds the toolchain version
  in its header (section 2.5), so a different rfs version changes that line -
  that is expected, not a break.
- External-tool resolution (`RFS_RUSTC` / `RFS_GCC` / `RFS_CLANG` /
  `RFS_CARGO_FUZZ` / `RFS_DOT` / ... env vars and their per-command flags,
  precedence CLI flag > env var > bare name on PATH) is stable; see
  `TOOLCHAIN.md`.

### 2.3 IR JSON schema (evolution rule)

The IR JSON document (`Ir::to_json`, hand-rolled deterministic writer in
`src/json_rw.rs` - stable key order and formatting) carries an
`ir_version` field. The evolution rule (DESIGN.md D10, implemented in
`src/ir.rs`):

- `from_json` accepts `ir_version` in `0..=IR_VERSION` - the current version
  AND all previous minor versions keep loading; a HIGHER version is rejected
  with the pinned error `unsupported IR version {n} at {path}.ir_version
  (this toolchain reads 0..={IR_VERSION})`.
- Therefore schema evolution must stay backward-readable: new fields are
  ADDITIVE and OPTIONAL (or have defaults). The liveness property variants
  were added this way. A change that would make old documents unloadable is a
  MAJOR toolchain bump, not an in-place edit.
- `to_json` writes the current `IR_VERSION`; golden JSON pins in the test
  suite re-pin on every bump.

### 2.4 The `.rfs` language and the syntax version header

Specs that parsed and validated under a given MAJOR version keep parsing and
validating until the next MAJOR bump. Since v1.0 (D-P6-1) the optional
module-level `version "..."` header is FORMALIZED: when present it must match
`X.Y` or `X.Y.Z` (all-digit components), otherwise validation fails with the
pinned error `module '<name>' invalid version header '<v>' (expected X.Y or
X.Y.Z)`. The parser still accepts any quoted string - a malformed header is a
semantic error, consistent with the parse/validate split. Specs WITHOUT a
header remain valid; they are treated as legacy v0.x syntax.

### 2.5 Artifact headers: stable CONTENT, movable TEXT

Every generated artifact (both backends, all commands, fuzz skeletons) starts
with a line-1 comment carrying three fields - since v1.0 in the unified form

```text
// Generated by RFS v{toolchain_version} | spec sha256:{hash16} | module {module_name}
```

The STABLE contract is that these three fields are present and correct:
the toolchain version that generated the file, the spec hash (section 3),
and the module name. The exact wording/format of the line may change in a
MINOR bump (it already changed in v1.0 - section 4); do not byte-pin the
format, pin the fields.

## 3. How users verify compatibility (spec hash + toolchain version)

The v1.0 mechanism for detecting drift between a checked-in generated file and
the spec it was built from:

1. **Toolchain version** - the `v{...}` field in line 1 tells you which rfs
   wrote the file. Regenerate with your current rfs; if only that field (and
   the hash, below) changed, the difference is toolchain drift.
2. **Spec hash** - `spec sha256:{hash16}` is the first 16 hex characters of
   SHA-256 over the CANONICAL IR JSON of the WHOLE input file (`Ir::to_json()`
   text; deterministic key order/format, so the hash is stable across runs
   and platforms). Any change to any part of the spec file changes it.
3. **Multi-module files** - the hash covers ALL modules of the file, but every
   codegen command generates for the FIRST module only (the CLI rule; see
   `docs/SEMANTICS.md` section 1). So one hash per artifact: editing a module
   that is not generated changes the hash even though the generated body is
   unchanged - the hash is a file fingerprint, not a per-module one.
4. **Check**: `rfs codegen <backend> production spec.rfs --out /tmp/new.rs`
   and diff against the checked-in artifact. Line 1 differs only when the
   toolchain version or the spec changed; any body difference on an
   unchanged spec + toolchain is a bug to report.

## 4. Migration record

### v0 -> v1 (toolchain 0.1.0 -> 1.0.0, Phase 6)

**What changed for users.**

- **IR schema version 0 -> 1** (D-P6-2): `to_json` now writes
  `"ir_version": 1`. Documents with `ir_version: 0` STILL LOAD - the read
  policy accepts `0..=1`; only `>= 2` is rejected. No action required for
  users of IR JSON; golden/byte pins re-pin to 1.
- **New line-1 header on every generated artifact** (D-P6-3): the old per-
  backend lines (`// Generated by RFS Phase 1 codegen.` / `// Generated by
  RFS Phase 3 codegen (C backend). Do not edit.` and the fuzz-skeleton
  variants) are replaced by the unified v1.0 line with toolchain version +
  spec hash + module name (section 2.5). Line 2 onward is unchanged. If you
  byte-pinned generated headers, re-pin them; if you pinned on the substring
  `// Generated by RFS`, your pins still hold.
- **Syntax version header formalized** (D-P6-1): a PRESENT `version "..."`
  must match `X.Y` or `X.Y.Z` (pinned semantic error otherwise). Specs without
  a header are unchanged and remain valid (legacy v0.x). If your spec carried
  a non-semver string as a free-form label, change it to `X.Y[.Z]` or remove
  the line.
- **Crate version 0.1.0 -> 1.0.0**: `--print-script` headers and other
  byte-pinned outputs that embed the version now say `rfs 1.0.0`.

**What did NOT change.** The `.rfs` language surface (everything that parsed
and validated under 0.x still does), generated-code public API names and
behavior, CLI commands/flags/exit codes, IR JSON field set (the version number
is the only new content), and step semantics (`docs/SEMANTICS.md`).

**Migration steps for a project pinning generated code.**

1. Update your toolchain to 1.0.0.
2. Regenerate all checked-in artifacts; review the diff - expected: line 1 of
   every artifact (and, for `rfs test` scripts, the script header version).
3. If you have IR JSON fixtures, regenerate them (`ir_version` 0 -> 1) or keep
   the old ones - both load.
4. Re-pin any byte-pinned headers to the new line-1 format (pin the FIELDS,
   not the wording - section 2.5).

### v1 -> v1.1 (Phase 7, pending minor bump)

**What changes for users.**

- **`rfs diagram --render svg` renders IN-PROCESS by default** (D-P7-1): a new
  `--renderer internal|external` flag defaults to `internal`, where each
  format's pure-Rust SVG renderer runs inside the rfs binary - NO external
  binaries (dot/plantuml/mmdc) are needed. `--renderer external` restores the
  pre-v1.1 behaviour byte-for-byte (resolved tool, strict errors, no fallback).
  The generated SOURCE files (`.dot`/`.puml`/`.mmd`) are unchanged by this.
- **Generated PlantUML source normalized** (task 2): `note right on link`
  became bare `note on link`, and emitted outputs now print ONE
  `note on link : emits X` line per output instead of a single multi-line note
  block. Both forms are valid PlantUML with identical meaning; the change was
  required by the in-process renderer's parser. If you byte-pin generated
  `.puml` sources, re-pin them.
- **New additive surface: `rfs info format`** (D-P7-6): self-describing
  documentation for the toolchain's own formats - `--list`, per-format
  `--output md|plantuml|svg|schema`. The five JSON document formats
  (`spec-json`, `ce-trace`, `mc-report`, `validate-json`, `info-json`) emit
  draft-07 JSON Schemas you can drop into CI to validate tool output; sync
  tests keep the models pinned to the code.

**What did NOT change.** The `.rfs` language, IR JSON schema (still v1),
generated-code public API, all other CLI commands/flags/exit codes, MC report
bytes, and step semantics (`docs/SEMANTICS.md`). No new example specs.

**Migration steps for a project pinning diagram output.**

1. Update your toolchain to the v1.1 release.
2. If you render SVG in CI, you can now DROP the dot/plantuml/mmdc
   dependencies - the default path needs none. Keep `--renderer external`
   only if you specifically want the official renderers' output.
3. Re-pin any byte-pinned `.puml` sources (note-on-link form change above);
   `.dot`/`.mmd` sources and all other artifacts are unchanged.

## 5. Process: how future migrations are made (maintainer template)

When a change touches the STABLE surface, the maintainer:

1. **Classify** it against section 2: breaking (MAJOR) or additive (MINOR).
   When in doubt, break nothing - ship additively and defer the break.
2. **Bump the crate version** in `Cargo.toml` (and let `Cargo.lock` follow);
   re-pin every byte-pinned output that embeds `CARGO_PKG_VERSION`
   (grep for the old `rfs {version}` string across `src/`, `tests/`,
   `training/`).
3. **Bump `IR_VERSION`** in `src/ir.rs` ONLY if the IR JSON schema changed;
   keep the read policy `0..=IR_VERSION` (old documents must still load),
   re-pin golden JSON, and add a "v{old} document still loads" test.
4. **Update THIS file**: extend section 2 if the stable surface grew
   (new commands, new generated-code types), and add a dated entry under
   section 4 with: what changed for users, what did NOT change, and concrete
   migration steps.
5. **Record the decision** in `DESIGN.md` (next free D-number of the phase)
   and sync `KB.md` / `BUILD.md` counts per the docs-sync task of the phase.
6. **Verify**: full BUILD.md section 5 checklist - `cargo build/test/clippy/
   fmt`, `--workspace`, `--features smt`, `cargo generate-code-tests && cargo
   test-generated-code` (any header change regenerates the aggregate crate),
   plus the file-driven example suites.
7. **Tag** the release at the phase close-out task (git tag `v{version}`).
