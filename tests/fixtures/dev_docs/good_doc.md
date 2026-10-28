# Fixture: good Class A doc

Positive control for the sync gate (Phase 8 task 1.1). Every doc→code
reference below resolves, and every fragment defined here is owned by
`tests/fixtures/dev_docs/good_code.rs`.

<a id="dev:fixture-good"></a>
## Good section

The parser entry point lives in `src/parser.rs`, the IR root type is
`src/ir.rs::Ir`, and `src/mc_plan.rs::boolean_over_atoms` is a real
top-level function.

<a id="dev:fixture-second"></a>
## Second section

File-level references without an item anchor are fine too:
`crates/rfs-build/src/lib.rs`.
