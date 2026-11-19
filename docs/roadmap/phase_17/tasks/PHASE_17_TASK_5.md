# Phase 17, Task 5 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal
Record the mmdc/puppeteer browser-binary finding in TOOLCHAIN.md section 6.4 so
a future session does not rediscover it: `mmdc` being on PATH is NOT mmdc
working - the mermaid-cli npm package does not bundle its browser.

## Context
- Discovered during Phase 17 preparation (2026-11-19): `mmdc --version` worked
  but every render failed with "Could not find chrome-headless-shell (ver.
  156.0.8078.47)" - puppeteer's default cache was the EPHEMERAL sandbox home
  (`/opt/unsloth-studio/sandbox/__LOCALID_*/.cache/puppeteer`, a new dir per
  session).
- The verified fix (already applied in this environment, recorded here for the
  record): install the browser into a PERSISTENT cache dir using the puppeteer
  version mermaid-cli BUNDLES - the latest npx-fetched puppeteer needs Node 20+
  and crashes on this env's Node 18.19.1 with "Invalid regular expression
  flags". Recipe that worked:
  `npm install --prefix /tmp/pptr puppeteer@25.16.0` then run its installer
  (`node /tmp/pptr/node_modules/puppeteer/lib/esm/puppeteer/node/install.js`) with
  `PUPPETEER_CACHE_DIR=/workspace/host/.cache/puppeteer` exported - it fetched
  both `chrome` and `chrome-headless-shell`. Then symlink the session home:
  `ln -sfn /workspace/host/.cache/puppeteer ~/.cache/puppeteer` (re-do per
  session, or export PUPPETEER_CACHE_DIR instead).
- Running as root still requires the `--no-sandbox` puppeteer config - rfs
  already passes one (existing 6.4 gotcha, unchanged).
- After the fix, `mmdc -i t.mmd -o t.svg -q -p pup.json` renders end-to-end
  (verified: fresh SVG produced).

## Scope

### Affected documentation files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `TOOLCHAIN.md` | change | section 6.4 gains the browser-binary gotcha + verified install recipe |

## Planned actions

### Add the gotcha to section 6.4
A new bullet in the existing "Gotchas (observed)" list, matching its style:
mmdc present is NOT mmdc working - the npm package does not bundle its
browser; `chrome-headless-shell` must be installed separately into a PERSISTENT
`PUPPETEER_CACHE_DIR` (the sandbox `$HOME` is ephemeral per session); on Node
18 the latest npx-fetched puppeteer crashes (needs Node 20+) - install the
version mermaid-cli bundles (25.16.0 here) via `npm install --prefix` + its
installer script; symlink `~/.cache/puppeteer` to the persistent dir or export
`PUPPETEER_CACHE_DIR`; root still needs the `--no-sandbox` config rfs passes.
Include the exact verified commands from Context. Mark it "observed
2026-11-19" like the other entries carry their provenance.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `TOOLCHAIN.md` | change (section 6.4) | a future session seeing mmdc on PATH will assume it works - this gotcha cost real debugging time in preparation |

## Task deliverables
- TOOLCHAIN.md section 6.4 carrying the browser-binary finding + recipe.

## Task exit criteria
- [ ] The new bullet names: the missing `chrome-headless-shell`, the ephemeral
      sandbox HOME, the Node 18 vs latest-puppeteer incompatibility, the
      persistent-cache + symlink fix, and the verified commands.
- [ ] `cargo test --test docs_md_test` green (TOOLCHAIN.md is in its file set).
- [ ] `cargo fmt --check` clean.

## Non-goals
- No changes to section 6.1's install block (the apt/npm lines remain correct
  for a fresh machine - the browser step is an ADDITIONAL gotcha, not a
  replacement).
- No source or test changes.
