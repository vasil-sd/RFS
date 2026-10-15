# RFS Rust Toolchain

**Read this file only when a build/test command fails, the toolchain seems missing,
or you need exact versions/paths.** For normal work, just use the activation block
below (also reproduced in `BUILD.md`) — do not search for the toolchain repeatedly.

## 1. Where the toolchain is (pinned facts)

| Location | What | Status |
|---|---|---|
| `/workspace/host/rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin` | Project-local stable toolchain: `cargo`, `rustc`, `clippy-driver`, `cargo-clippy`, `rustfmt`, `cargo-fmt`. **Canonical for this project.** | Works, no env setup needed if called by direct path |
| `/workspace/host/.cargo/bin` | `rustup` shim directory (CARGO_HOME) for the project-local install | Shim works only with the env exports in §2; without them it fails with "no default is configured" |
| `$HOME/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin` | Identical stable toolchain under the session's home dir (present in the current sandbox) | Works by direct path; `$HOME` changes between sessions, so prefer the project-local path above |
| `/usr/local/bin/cargo` | System cargo 1.57.0 | **Do not use** — too old to read this repo's `Cargo.lock` (version 4); fails with "lock file version `4` was found" |

Pinned versions (both working toolchains): **cargo 1.99.0, rustc 1.99.0** (stable,
2026-08/09). Components `clippy` and `rustfmt` are installed.

## 2. How to run it (copy-paste activation)

Shell environment does **not** persist between tool calls, so prepend this to every
command that needs cargo/rustc:

```sh
export RUSTUP_HOME=/workspace/host/rustup
export CARGO_HOME=/workspace/host/.cargo
export PATH="$CARGO_HOME/bin:$PATH"
cd /workspace/host/RFS
```

Equivalent without env setup (direct toolchain path, useful for one-off checks):

```sh
/workspace/host/rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/cargo --version
```

Verify with `cargo --version` → must print `cargo 1.99.0`. If it prints anything else
(or a lock-file error), you are using the wrong toolchain — re-check `which cargo`.

## 3. Reinstalling the project-local toolchain

Only if `/workspace/host/rustup` or `/workspace/host/.cargo` disappear (the RFS tree
itself can survive):

```sh
curl -sS https://sh.rustup.rs -o /tmp/rustup-init.sh
RUSTUP_HOME=/workspace/host/rustup \
CARGO_HOME=/workspace/host/.cargo \
sh /tmp/rustup-init.sh -y --profile minimal --default-toolchain stable --no-modify-path

export RUSTUP_HOME=/workspace/host/rustup
export CARGO_HOME=/workspace/host/.cargo
export PATH="$CARGO_HOME/bin:$PATH"

rustup component add clippy
rustup component add rustfmt
```

Important: newer `rustup-init` may reject `--rustup` and `--cargo-home` arguments.
Use environment variables instead (as above). Network access to crates.io/rustup was
available in the current environment; future environments may differ — if the install
fails offline, use the direct toolchain path from §1 if it still exists.

## 4. Environment corner cases (observed)

- The working directory of a new shell is **not** `/workspace/host/RFS`; always `cd` first or use absolute paths.
- If cargo fails with a lock-file version error, the active toolchain is too old or missing: reinstall per §3 before debugging code.
- System tools may be old (see §1); prefer the project-local toolchain.
- Use `tail -n 120` or `grep` on long cargo output to keep tool results readable.
