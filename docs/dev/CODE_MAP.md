# RFS Code Map

File-by-file map of the 72 hand-written source files: `src/` (49), `crates/`
(18), `fuzz/fuzz_targets/` (5). Each entry: responsibility, key items, doc
links both ways. The pointer scope is driven by `tests/dev_docs_manifest.txt`.

<a id="dev:code-map-frontend"></a>
## Frontend area - root + parser + validator/IR (13 files)

TBD (task 3.1): root (6), parser (2), validator+IR (5).

<a id="dev:code-map-codegen"></a>
## Codegen core + backends (18 files)

TBD (task 3.2): codegen core (4) + Rust backend (5) + C backend (4); both
backends at equal depth.

<a id="dev:code-map-cli"></a>
## CLI + diagrams area (18 files)

TBD (task 3.3): CLI/diagrams (13) + bins/training (5).

<a id="dev:code-map-workspace"></a>
## Workspace crates + fuzz (23 files)

TBD (task 3.4): `crates/` (18) + `fuzz/` (5).
