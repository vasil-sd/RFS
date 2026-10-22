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

## 5. z3 (SMT solver) — Phase 4 SMT mode (optional)

Only needed for Phase 4 task 11 (`--mode smt-bounded`, feature-gated). No other
build/test command requires it.

| Location | What | Status |
|---|---|---|
| `/workspace/host/z3` | Project-local z3 **5.3.1** install (prebuilt `x64-glibc-2.39` release): `bin/z3` (CLI), `bin/libz3.so`, `include/` (C headers incl. `z3.h`), `lib/pkgconfig/z3.pc` | Installed 2026-10-18, verified: `z3 --version` → `Z3 version 5.3.1 - 64 bit`; sanity query returns `sat` |
| `/workspace/host/z3-5.3.1-x64-glibc-2.39.zip` | The release archive (kept for reinstall) | Present (68 MB) |

Layout quirk of this release: the shared library lives in **`bin/`** (not
`lib/`); `lib/` holds only `pkgconfig/` and `cmake/`. The zip extracts into a
wrapping subdirectory, and `python3 -m zipfile` drops exec bits — both handled
in §5.1.

### 5.1 Reinstalling z3

```sh
cd /workspace/host
curl -sSL -o z3-5.3.1-x64-glibc-2.39.zip \
  https://github.com/Z3Prover/z3/releases/download/z3-5.3.1/z3-5.3.1-x64-glibc-2.39.zip
rm -rf z3
python3 -m zipfile -e z3-5.3.1-x64-glibc-2.39.zip z3/   # no `unzip` in this env
mv z3/z3-5.3.1-x64-glibc-2.39/* z3/ && rmdir z3/z3-5.3.1-x64-glibc-2.39
chmod +x z3/bin/z3                                        # zipfile drops exec bits
/workspace/host/z3/bin/z3 --version                       # must print Z3 version 5.3.1
```

### 5.2 Using z3 from Rust (the `z3` crate, feature-gated)

Pinned: `z3` crate **0.21.x** / `z3-sys` **0.13.x** tracks the z3 5.x series
(verified 2026-10-18). Use the DEFAULT z3-sys feature ("system install") and
point it at the project-local install:

```sh
export PATH="/workspace/host/z3/bin:$PATH"        # CLI version detection (`z3 --version`)
export Z3_LIBRARY_PATH_OVERRIDE=/workspace/host/z3/bin   # -L search path (libz3.so is in bin/)
```

`z3-sys` requests the library itself via `#[link(name = "z3")]`, so no
pkg-config binary is needed (the system one is absent here). If a pkg-config
binary ever exists, `PKG_CONFIG_PATH=/workspace/host/z3/lib/pkgconfig` works as
an alternative (the release ships `z3.pc`). Do **not** use the `gh-release` or
`vendored` z3-sys features: they would pin Rust to a different solver build than
the C backend links against, breaking SMT result parity.

### 5.3 Using z3 from generated C code

```sh
gcc -Wall out.c -I/workspace/host/z3/include -L/workspace/host/z3/bin -lz3 \
    -Wl,-rpath,/workspace/host/z3/bin -o out
```

`-Wl,-rpath` keeps the binary runnable without `LD_LIBRARY_PATH`; drop it and
export `LD_LIBRARY_PATH=/workspace/host/z3/bin` instead if rpath is unwanted.

## 6. Diagram rendering tools (dot / plantuml / mermaid) — Phase 5 task 8

`rfs diagram` generates diagram SOURCE in pure Rust with no external tools.
Only `--render svg` invokes an external renderer, and it is STRICT: a missing
or failing tool makes rfs exit 1 (no graceful degradation - CI/CD requirement).
All three renderers are OPTIONAL system dependencies; text generation works
without any of them.

### 6.1 Install (verified in this environment, Ubuntu 24.04)

```sh
sudo apt install graphviz plantuml     # dot 2.43.0, plantuml 1.2020.02 (pulls OpenJDK 21)
sudo apt install nodejs npm            # node v18.19.1, npm 9.2.0
npm install -g @mermaid-js/mermaid-cli # mmdc (bundles mermaid 12.2.0)
```

### 6.2 Parameterization (unified with the other external tools)

| Tool | Env PATH var | Env args var | CLI overrides |
|---|---|---|---|
| `dot` | `RFS_DOT` | `RFS_DOT_ARGS` | `--dot PATH`, `--dot-args "..."` |
| `plantuml` | `RFS_PLANTUML` | `RFS_PLANTUML_ARGS` | `--plantuml PATH`, `--plantuml-args "..."` |
| `mmdc` | `RFS_MERMAID` | `RFS_MERMAID_ARGS` | `--mermaid PATH`, `--mermaid-args "..."` |

Precedence: CLI flag > env var > bare name on PATH. Env args come first, CLI
args appended (same as rustc/gcc). `--print-script` prints the exact invocation
as a self-contained POSIX sh script instead of executing it.

### 6.3 Running the tools by hand

```sh
rfs diagram --format dot  --out d.dot spec.rfs && dot -Tsvg d.dot -o d.svg
rfs diagram --format puml --out d.puml spec.rfs && plantuml -tsvg d.puml   # writes d.svg next to the input
rfs diagram --format mmd  --out d.mmd  spec.rfs && mmdc -i d.mmd -o d.svg -q
```

### 6.4 Gotchas (observed)

- **mmdc as root** cannot launch Chrome without `--no-sandbox`; rfs therefore
  always passes a generated puppeteer config (`{"args":["--no-sandbox","--disable-dev-shm-usage"]}`,
  overridable via `RFS_MERMAID_PUPPETEER_ARGS`). `mmdc -o` only accepts
  md/svg/png/pdf extensions (not `/dev/null`).
- **plantuml** exits **200** on a syntax error (and still writes an error SVG),
  so rfs gates on the exit status, not file presence.
- **Two `mmdc` processes must not run concurrently** in this container: under
  parallel Chrome launches one of them fails nondeterministically. The test
  suite serializes real-tool invocations for this reason; do the same in CI.

## 7. clang + libFuzzer + cargo-fuzz - `rfs test --suite fuzz` (Phase 5 task 17)

The fuzz suites of `rfs test` build and bounded-run the libFuzzer targets that
tasks 5/6 emit as skeleton extra files. The UNIT suite (the default) needs
none of this section; the tools are required ONLY when `--suite fuzz` is
selected (missing tool -> exit 1 + diagnostic, no graceful degradation).

### 7.1 Install (verified in this environment, Ubuntu 24.04)

```sh
# clang WITH the libFuzzer runtime (Ubuntu ships libclang_rt.fuzzer inside the
# `clang` package):
sudo apt-get install -y clang          # installed: clang 18.1.3-1ubuntu1 at /usr/bin/clang

# cargo-fuzz for the Rust side (into the project-local CARGO_HOME):
cargo install cargo-fuzz --locked      # installed: cargo-fuzz v0.13.2

# A NIGHTLY toolchain - cargo-fuzz builds with `-Zsanitizer=address`, which is
# nightly-only. The rfs scaffold pins `channel = "nightly"` INSIDE the temp
# project (rust-toolchain.toml), so your default toolchain stays untouched:
rustup toolchain install nightly --profile minimal
```

### 7.2 Verification (copy-paste)

```sh
# C: a trivial libFuzzer target builds, links and runs bounded (exit 0):
printf '#include <stdint.h>\n#include <stddef.h>\nint LLVMFuzzerTestOneInput(const uint8_t* d, size_t s){(void)d;(void)s;return 0;}\n' > /tmp/fz.c
clang -O1 -fsanitize=fuzzer,address -o /tmp/fz /tmp/fz.c && /tmp/fz -max_total_time=2
cargo fuzz --version        # cargo-fuzz present on PATH (project-local CARGO_HOME)
rustc +nightly --version    # the toolchain the Rust scaffold pins
```

### 7.3 Parameterization (unified with the other external tools)

| Env var | Meaning |
|---|---|
| `RFS_CLANG` / `RFS_CLANG_ARGS` | clang PATH override / extra build args (env args first, then `--clang-args`) - C fuzz pipeline only |
| `RFS_CARGO_FUZZ` / `RFS_CARGO_FUZZ_ARGS` | the CARGO PATH used for `cargo fuzz ...` (the `fuzz` subcommand is fixed) / extra args between `fuzz` and the subcommand verb - Rust fuzz pipeline only |
| `RFS_FUZZ_FLAGS` | EXTRA libFuzzer RUNTIME flags appended after `-max_total_time=<N>` (both backends; CLI `--fuzz-flags` appended after the env value) |

CLI overrides: `--clang PATH`, `--clang-args "..."`, `--cargo-fuzz PATH`,
`--cargo-fuzz-args "..."`, `--fuzz-max-total-time SEC` (default 30),
`--fuzz-flags "..."`. Precedence per tool: flag > env var > bare name on PATH.

### 7.4 Gotchas (observed)

- **The C pipeline runs with `ASAN_OPTIONS=detect_leaks=0`** (merged over any
  user value): the generated C FSM has NO free API - process-lifetime buffers,
  one allocation set per fuzz iteration - so ASan's exit-time leak report would
  fail every run. In-flight memory errors are still caught; re-enable leaks via
  `ASAN_OPTIONS=detect_leaks=1` if you know what you are doing.
- **The C build compiles ONLY `fuzz_target.c`**: the skeleton `#include`s the
  production source (`rfs_generated_fsm.c`, written next to it), so passing both
  files to clang is a multiple-definition link error.
- **cargo-fuzz needs nightly** (`-Zsanitizer=address`). The scaffold writes a
  `rust-toolchain.toml` pinning `channel = "nightly"` inside the temp project;
  rfs strips a non-nightly inherited `RUSTUP_TOOLCHAIN` from the cargo-fuzz
  subprocess (a parent `cargo test`/`cargo run` under rustup pins stable, which
  would otherwise override the file). An explicit nightly pin is respected.
- The Rust scaffold's harness manifest carries `[package.metadata]
cargo-fuzz = true` (required for `cargo fuzz` to recognize it) and pins
  `libfuzzer-sys = "=0.4.13"` - the release present in the local registry cache,
  so the temp-project build works OFFLINE.
