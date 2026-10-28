<!-- llm-budget: 400 -->
# RFS Task Map (Class B)

Functionality to files/functions for EVERY CLI command, subsystem, and test
suite; the "read this fragment for this task" index extending `START.md`
section 6.

<a id="llm:task-map"></a>
## Map

TBD (task 13.1): full functionality-to-source map over all 72 hand-written
files + every test suite, at file/function granularity.

## Coverage (growing stub)

Sync-gate requirement: every manifest-listed source file must appear below
as a doc→code reference. Chunks 1.3-1.10 append their files here; task 13.1
rewrites this section into the full functionality-to-source map.

- `src/lib.rs` - crate root: module tree, public API surface
- `src/main.rs` - CLI entry point: argument parsing and command dispatch
- `src/error.rs` - shared source spans and parse-error types
- `src/cli_log.rs` - minimal stderr logging for the new CLI commands
- `src/frontend.rs` - frontend CLI logic: RFS/JSON conversion, validation dispatch
- `src/printer.rs` - IR-to-RFS source printer (unparser) and round-trip helpers
