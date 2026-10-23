# RFS IDE support (Phase 5 task 13)

TextMate grammar + snippets for `.rfs` files, and structured CLI diagnostics
for editor integrations. A full LSP is a phase non-goal - this is the initial
IDE surface.

## Contents

- `textmate/rfs.json` - VS Code-compatible TextMate grammar (`source.rfs`).
  Covers: comments (`//` and `#`), the module/enum/event/output/var/transition/
  on_error structure, property & assumption formulas (temporal operators
  `always` / `eventually` / `bounded_eventually[N]` / `until`, boolean,
  comparison and arithmetic operators), string literals (version header),
  numbers, booleans, primitive types (`bool`, `bit<N>`), enum constructor
  paths (`Mode::Normal`) and identifiers.
- `snippets.rfs` - snippet set (VS Code JSON format): module skeleton,
  transition block, property with `always` / `eventually`, assumption.

## Editor diagnostics

Editors can ingest parse/semantic errors directly:

```sh
rfs validate --json <file.rfs>            # one document on stdout
rfs validate --json --input rfs <dir>     # array of one document per failing file
```

Document shape (stable - pinned by tests):

```json
{ "file": "<path>", "ok": false,
  "errors": [ { "kind": "parse|semantic", "line": L, "col": C, "message": "..." } ] }
```

`line`/`col` are 1-based; they are `0` when unknown (semantic errors carry no
span). Exit codes are unchanged: 0 = valid, 1 = invalid or misuse.

## Installing in VS Code

The grammar is shipped in-repo (no published extension - a phase non-goal).
Two routes:

### Route A: one-click repo (simplest)

Make this `ide/` directory the root of a git repository (or copy it into one)
with ONE extra file at the root - `package.json`:

```json
{
  "name": "rfs-language",
  "displayName": "RFS Language",
  "version": "0.1.0",
  "engines": { "vscode": "^1.75.0" },
  "contributes": {
    "languages": [{
      "id": "rfs",
      "extensions": [".rfs"],
      "configuration": { "comments": { "lineComment": ["//", "#"] } }
    }],
    "grammars": [{
      "language": "rfs",
      "scopeName": "source.rfs",
      "path": "./textmate/rfs.json"
    }],
    "snippets": [{ "language": "rfs", "path": "./snippets.rfs" }]
  }
}
```

Then either open the repo in a VS Code development host
(`Ctrl/Cmd+Shift+P` -> "Developer: Open Extensions Development Host") or
package it (`vsce package`) and install the `.vsix`. With the extension
active, every `.rfs` file gets syntax highlighting + snippets.

## Installing in Sublime Text

1. Copy `textmate/rfs.json` to `Packages/User/RFS.tmLanguage` (Sublime reads
   TextMate JSON directly).
2. Associate the extension in `Preferences -> Settings`:

   ```json
   { "extensions": ["rfs"], "hidden_extensions": [], "syntax": "Packages/User/RFS.tmLanguage" }
   ```

3. Snippets: Sublime uses its own snippet format - copy the bodies from
   `snippets.rfs` into a `Packages/User/RFS.sublime-snippet` file (each JSON
   `body` array maps 1:1 to `<content>` lines; `${n}` placeholders become `$n`).

## Keeping the grammar in sync

The parser's keyword list (`KEYWORDS` in `src/parser.rs`) is the single source
of truth; a test asserts every keyword appears in this grammar (and that the
list matches the parser's call sites). When the language grows, update the
grammar and the test will tell you what is missing.
