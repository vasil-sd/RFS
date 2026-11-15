# Phase 15, Task 1 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal
Give `TOOLCHAIN.md` an explicit, self-sufficient section on setting up and using
the Nix package manager on this Ubuntu sandbox - installer command, single-user
vs multi-user (`--daemon`) modes with when a daemon is actually required, the
sandbox's current nix state, usage commands, the dev.to packaging reference, and
the previous preliminary nix work with its recorded findings. Future sessions
must be able to drive Nix here without re-discovering any of it.

## Context
- **Current sandbox state (verified 2026-11-15):** nix **2.35.6** at
  `/nix/store/g3m57m0jcdj84fas49lpiwpisacp20gr-nix-2.35.6/bin/nix` - NOT on PATH
  (no `/usr/bin/nix`, no `/etc/profile.d/nix.sh`). No daemon runs and none is
  needed: no `/nix/var/nix/daemon-socket` exists, yet `nix build` works because
  as root Nix falls back to single-user (direct) mode. `/etc/nix/nix.conf`
  contains exactly `experimental-features = nix-command flakes` and
  `sandbox = false`. The store is pre-populated from the preliminary work; the
  network to `cache.nixos.org` works. `import <nixpkgs> {}` resolves via the
  root channel profile to nixpkgs unstable 26.11pre.
- **Daemon knowledge to document:** the official installer
  (`curl -sS https://nixos.org/nix/install | sh`) offers multi-user mode (installs
  a systemd unit running `nix-daemon`; required for non-root users and for
  remote/daemon-mediated builds) vs single-user mode (`--no-daemon`; every nix
  command talks to the store directly as the invoking user - root works, an
  unprivileged user needs write access to `/nix`). In a container without
  systemd the multi-user unit never starts; the two working options are
  `--no-daemon` (what this sandbox effectively is) or manually running
  `sudo nix-daemon &` from the same store path and letting clients connect via
  `/nix/var/nix/daemon-socket/socket`. The section must state which mode this
  sandbox uses and that no action is needed to "start" nix here.
- **Reference guide:** https://dev.to/misterio/how-to-package-a-rust-app-using-nix-3lh3
  - the buildRustPackage happy path (minimal default.nix, `cargoLock.lockFile`,
  `cleanSource`, flake variant, `shell.nix` with `inputsFrom` + extra
  buildInputs, `importTOML` for name/version). Our `nix/package.nix` already
  follows this pattern; the section links it as the packaging reference.
- **Previous preliminary work to record:** commit **dd6e0f4** (2026-11-12)
  created `nix/package.nix` + `shell.nix` and fixed three C-backend defects the
  nix clang toolchain exposed; commit **d1c1b31** (2026-11-12) switched the
  unified-z3 prefix from copying to plain symlinks. The only nix entry in
  `TASKS.md` (Phase 14 task 8 close-out, line ~2463) records that
  `nix develop -f shell.nix` failed on a "clang answering for gcc"
  tool-resolution artifact in `ref_mc_diff_test` - Phase 15 task 3's wrapper is
  the fix. The section must point at both so nobody re-chases the artifact as a
  codegen bug.
- **Insertion point:** new top-level section `## 10. Nix package manager on this Ubuntu sandbox`
  appended AFTER the end of the current last section `## 9. Building dev_tools
  (`rfs-dev`)` (TOOLCHAIN.md is 418 lines; §9 starts at line 399 and runs to
  EOF). Numbering continues the file's existing `## N.` scheme.
- **Reading tool:** verify the result with
  `./target/debug/rfs-dev doc-section TOOLCHAIN.md '10. Nix package manager on this Ubuntu sandbox'`
  (exact-text heading match - rfs-dev doc-section matches full heading text,
  not prefixes).

## Scope

### Affected documentation files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `TOOLCHAIN.md` | change (append after line 418) | new §10 with four subsections: install, daemon modes, current sandbox state, usage + previous work |

### Affected build/run process
- No commands change; the section documents existing behavior. A future session
  gains a canonical reference for `export PATH=/nix/store/g3m57m0jcdj84fas49lpiwpisacp20gr-nix-2.35.6/bin:$PATH`
  before any nix command.

## Planned actions

### Write the new §10 section
Four subsections, in this order:
1. `### 10.1 Install Nix on Ubuntu` - the official installer one-liner, the
   multi-user vs single-user choice at install time (`--no-daemon`), and a short
   paragraph on what the nix daemon is: a privileged process that mediates store
   writes, required for non-root users and remote builds; in a systemd-less
   container either use `--no-daemon` or start `nix-daemon` manually from the
   store path (`sudo /nix/store/<hash>-nix-<ver>/bin/nix-daemon &`) so clients
   can connect through the daemon socket.
2. `### 10.2 Current state of this sandbox` - the verified facts from Context
   (version + store path, not on PATH, no daemon / single-user-as-root works,
   nix.conf contents, pre-populated store, cache.nixos.org reachable, nixpkgs
   channel version).
3. `### 10.3 Using nix here` - the PATH export line, then the three commands
   this project uses: `nix build --impure --expr 'import ./nix/package.nix { }'`
   (package + checks), the doCheck=false variant, and
   `nix develop -f shell.nix -c '<cmd>'`. Note that `--impure` is required
   because `<nixpkgs>` resolves through the channel profile, not a flake input.
4. `### 10.4 Previous nix work and references` - commits dd6e0f4/d1c1b31 with
   one-line descriptions, the TASKS.md clang-answering-for-gcc finding (with
   pointer to Phase 15 task 3 as its resolution), and the dev.to article link
   as the buildRustPackage packaging reference.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `TOOLCHAIN.md` | change (append at EOF, after line 418) | the new §10 - keeps the file's "read only when you need exact versions/paths" role while making Nix setup self-documenting |

## Task deliverables
- `TOOLCHAIN.md` §10 with all four subsections, retrievable by
  `rfs-dev doc-section` under its exact heading.

## Task exit criteria
- [ ] `TOOLCHAIN.md` contains `## 10. Nix package manager on this Ubuntu sandbox`
      with subsections 10.1-10.4 covering: installer command; single-user vs
      multi-user/`--daemon` explanation including when a daemon is required and
      how to run one manually without systemd; the sandbox's current state
      (2.35.6 store path, no PATH entry, no daemon, `sandbox = false`, flakes
      on, pre-populated store); usage commands with the PATH export;
      dd6e0f4/d1c1b31 + the TASKS.md finding; the dev.to link.
- [ ] `./target/debug/rfs-dev doc-section TOOLCHAIN.md '10. Nix package manager on this Ubuntu sandbox'`
      returns the section (exit 0).
- [ ] No other file changed (`git status` shows only TOOLCHAIN.md); `rfs-dev
      sync-check` still clean; `rfs-dev inventory audit` still EXACT.
- [ ] Committed per FORMATTING.md §7.

## Non-goals
- No changes to any other section of TOOLCHAIN.md (no renumbering - §10 is a
  pure append).
- No flake setup, no nix daemon installation - the sandbox already works in
  single-user mode; the section documents that, not a migration.
- No `BUILD.md` changes (that is task 5).

## Task corrections

| Correction | Description + rationale |
|------------|-------------------------|

## Other information
- Keep the section factual and short-form like the rest of TOOLCHAIN.md - it is
  a reference, not a tutorial. The dev.to article is linked, not reproduced.
