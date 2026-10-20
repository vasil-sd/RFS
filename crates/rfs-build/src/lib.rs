//! `rfs-build` - the build.rs helper for RFS.
//!
//! Generates protocol code from an RFS spec at build time by invoking the
//! `rfs` toolchain, so a consumer project can do:
//!
//! ```rust,ignore
//! // consumer/build.rs
//! fn main() -> Result<(), rfs_build::RfsBuildError> {
//!     rfs_build::RfsBuild::new("specs/handshake.rfs")
//!         .backend("rust")
//!         .with_property_monitors(true)
//!         .compile()?;
//!     Ok(())
//! }
//! ```
//!
//! ```rust,ignore
//! // consumer/src/lib.rs
//! pub mod handshake {
//!     include!(concat!(env!("OUT_DIR"), "/handshake.rs"));
//! }
//! ```
//!
//! ## Toolchain location
//!
//! The `rfs` executable is resolved in this order:
//!
//! 1. the `RFS_BIN` environment variable (preferred - pins an exact binary,
//!    e.g. a workspace-built `target/debug/rfs` or a released toolchain);
//! 2. `rfs` on `PATH`.
//!
//! An empty/whitespace `RFS_BIN` is treated as unset. A non-empty `RFS_BIN`
//! that does not point to an existing file is an error. When the binary comes
//! from PATH there is no stable path cargo can watch, so rebuilds are only
//! re-triggered by spec changes and `RFS_BIN` changes (documented limitation;
//! set `RFS_BIN` to pin the toolchain).
//!
//! ## Cargo directives emitted by [`RfsBuild::compile`]
//!
//! - `cargo:rerun-if-changed=<spec>` - the spec file (absolute path);
//! - `cargo:rerun-if-env-changed=RFS_BIN`;
//! - `cargo:rerun-if-changed=<RFS_BIN>` when `RFS_BIN` names an existing file;
//! - `cargo:include=<artifact>` - the generated file under `OUT_DIR`.
//!
//! ## Scope
//!
//! Only the `production` codegen command is supported (that is what build.rs
//! consumers want). The crate adds no dependencies and never modifies the
//! spec; parse/validation/codegen failures from `rfs` become build failures
//! with the rfs diagnostics in the message.

use std::path::{Path, PathBuf};
use std::process::Command;

/// A failure to plan or run RFS code generation from a build script.
#[derive(Debug)]
pub struct RfsBuildError {
    pub message: String,
}

impl std::fmt::Display for RfsBuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for RfsBuildError {}

fn err(message: impl Into<String>) -> RfsBuildError {
    RfsBuildError {
        message: message.into(),
    }
}

/// Resolve the `rfs` executable per the module docs: `RFS_BIN` (when set to a
/// non-empty value naming an existing file) wins over PATH. The returned path
/// is either the exact `RFS_BIN` location or the bare name `rfs` (resolved via
/// PATH at spawn time).
pub fn resolve_rfs_bin() -> Result<PathBuf, RfsBuildError> {
    match std::env::var("RFS_BIN") {
        Ok(raw) if !raw.trim().is_empty() => {
            let p = PathBuf::from(&raw);
            if !p.is_file() {
                return Err(err(format!(
                    "RFS_BIN is set to '{}' but that file does not exist",
                    p.display()
                )));
            }
            Ok(p)
        }
        _ => Ok(PathBuf::from("rfs")),
    }
}

/// Builder for one codegen invocation (see the module docs for usage).
#[derive(Debug, Clone)]
pub struct RfsBuild {
    spec: PathBuf,
    backend: String,
    with_property_monitors: bool,
}

impl RfsBuild {
    /// Start a build for `spec` (a `.rfs` or `.json` file). The default
    /// backend is `rust`; the command is always `production`.
    pub fn new(spec: impl Into<PathBuf>) -> Self {
        Self {
            spec: spec.into(),
            backend: "rust".into(),
            with_property_monitors: false,
        }
    }

    /// Target backend: `"rust"` (default) or `"c"`. Anything else is an error
    /// at [`compile`](Self::compile) time.
    pub fn backend(mut self, backend: &str) -> Self {
        self.backend = backend.to_string();
        self
    }

    /// Select the `production` codegen command (the default and only supported
    /// command). Kept explicit so build scripts read intent clearly.
    pub fn production(self) -> Self {
        self
    }

    /// Emit property monitors in the generated production code
    /// (`--with-property-monitors`). Off by default.
    pub fn with_property_monitors(mut self, on: bool) -> Self {
        self.with_property_monitors = on;
        self
    }

    /// The spec file this build reads.
    pub fn spec(&self) -> &Path {
        &self.spec
    }

    /// Run the code generation. Prints the cargo directives, invokes
    /// `rfs codegen <backend> production <spec> --out OUT_DIR/<stem>.<ext>`
    /// (plus `--with-property-monitors` when enabled), and returns the path of
    /// the generated artifact. Errors carry the rfs diagnostics.
    pub fn compile(&self) -> Result<PathBuf, RfsBuildError> {
        // --- validation (before touching the environment) -----------------
        if !matches!(self.backend.as_str(), "rust" | "c") {
            return Err(err(format!(
                "unknown backend '{}' (expected 'rust' or 'c')",
                self.backend
            )));
        }
        let ext = self.spec.extension().and_then(|e| e.to_str()).unwrap_or("");
        if !matches!(ext, "rfs" | "json") {
            return Err(err(format!(
                "spec '{}' must have extension .rfs or .json",
                self.spec.display()
            )));
        }
        if !self.spec.is_file() {
            return Err(err(format!(
                "spec file '{}' does not exist",
                self.spec.display()
            )));
        }
        let out_dir = std::env::var("OUT_DIR").map_err(|_| {
            err("OUT_DIR is not set - rfs-build must run inside a cargo build script")
        })?;

        // --- plan -----------------------------------------------------------
        let rfs_bin = resolve_rfs_bin()?;
        let stem = self
            .spec
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("rfs");
        let artifact_ext = if self.backend == "rust" { "rs" } else { "c" };
        let artifact = Path::new(&out_dir).join(format!("{stem}.{artifact_ext}"));

        // --- cargo directives ----------------------------------------------
        println!("{}", self.cargo_directives(&rfs_bin, &artifact));
        if rfs_bin == Path::new("rfs") {
            println!("note: rfs-build resolved `rfs` from PATH; set RFS_BIN to pin the toolchain");
        }

        // --- invoke ----------------------------------------------------------
        let mut cmd = Command::new(&rfs_bin);
        cmd.arg("codegen")
            .arg(&self.backend)
            .arg("production")
            .arg(&self.spec)
            .arg("--out")
            .arg(&artifact);
        if self.with_property_monitors {
            cmd.arg("--with-property-monitors");
        }
        let output = cmd.output().map_err(|e| {
            err(format!(
                "failed to spawn rfs binary '{}': {}",
                rfs_bin.display(),
                e
            ))
        })?;
        if !output.status.success() {
            let mut msg = format!(
                "rfs codegen failed (exit {}): {}",
                output.status.code().unwrap_or(-1),
                String::from_utf8_lossy(&output.stderr)
            );
            let stdout = String::from_utf8_lossy(&output.stdout);
            if !stdout.trim().is_empty() {
                msg.push_str("\n--- rfs stdout ---\n");
                msg.push_str(stdout.trim_end());
            }
            return Err(err(msg));
        }
        if !artifact.is_file() {
            return Err(err(format!(
                "rfs exited successfully but the artifact '{}' was not written",
                artifact.display()
            )));
        }
        Ok(artifact)
    }

    /// The `cargo:` directive lines for this build (pure - unit-testable).
    fn cargo_directives(&self, rfs_bin: &Path, artifact: &Path) -> String {
        let spec = self
            .spec
            .canonicalize()
            .unwrap_or_else(|_| self.spec.clone());
        let mut lines = vec![
            format!("cargo:rerun-if-changed={}", spec.display()),
            "cargo:rerun-if-env-changed=RFS_BIN".to_string(),
        ];
        if rfs_bin != Path::new("rfs") {
            lines.push(format!("cargo:rerun-if-changed={}", rfs_bin.display()));
        }
        lines.push(format!("cargo:include={}", artifact.display()));
        lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::sync::Mutex;

    /// Env-mutating tests share process-global state - serialize them.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn temp_dir(tag: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("rfs_build_test_{}_{}", std::process::id(), tag));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_spec(dir: &Path, name: &str) -> PathBuf {
        let p = dir.join(name);
        std::fs::write(&p, "module m { enum S { A } initial A }\n").unwrap();
        p
    }

    /// A stub `rfs` shell script: records its argv, honors --out by writing a
    /// marker file, and fails (exit 3 + stderr) when RFS_STUB_FAIL is set.
    fn write_stub(dir: &Path) -> PathBuf {
        let p = dir.join("stub_rfs");
        let mut f = std::fs::File::create(&p).unwrap();
        writeln!(
            f,
            "#!/bin/sh\necho \"$@\" > \"$(dirname \"$0\")/stub_args.txt\"\nif [ -n \"$RFS_STUB_FAIL\" ]; then echo \"stub: deliberate failure\" >&2; exit 3; fi\nout=\"\"\nprev=\"\"\nfor a in \"$@\"; do if [ \"$prev\" = \"--out\" ]; then out=\"$a\"; fi; prev=\"$a\"; done\nif [ -n \"$out\" ]; then echo 'pub struct Stub;' > \"$out\"; fi\nexit 0"
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        p
    }

    fn set_env(pairs: &[(&str, Option<&str>)]) {
        for (k, v) in pairs {
            match v {
                Some(val) => std::env::set_var(k, val),
                None => std::env::remove_var(k),
            }
        }
    }

    #[test]
    fn resolve_rfs_bin_precedence() {
        let _g = ENV_LOCK.lock().unwrap();
        let dir = temp_dir("resolve");
        let stub = write_stub(&dir);

        // RFS_BIN set to an existing file -> wins over PATH.
        set_env(&[("RFS_BIN", Some(stub.to_str().unwrap()))]);
        assert_eq!(resolve_rfs_bin().unwrap(), stub);

        // RFS_BIN empty/whitespace -> treated as unset -> bare `rfs`.
        set_env(&[("RFS_BIN", Some("   "))]);
        assert_eq!(resolve_rfs_bin().unwrap(), PathBuf::from("rfs"));

        // RFS_BIN unset -> bare `rfs`.
        set_env(&[("RFS_BIN", None)]);
        assert_eq!(resolve_rfs_bin().unwrap(), PathBuf::from("rfs"));

        // RFS_BIN pointing at a missing file -> error.
        let missing = dir.join("nope");
        set_env(&[("RFS_BIN", Some(missing.to_str().unwrap()))]);
        let e = resolve_rfs_bin().unwrap_err();
        assert!(e.message.contains("does not exist"), "{}", e.message);

        set_env(&[("RFS_BIN", None)]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn option_validation_errors_before_environment() {
        // No OUT_DIR / RFS_BIN needed: validation errors come first.
        let dir = temp_dir("validate");
        let spec = write_spec(&dir, "ok.rfs");

        let e = RfsBuild::new(&spec).backend("cobol").compile().unwrap_err();
        assert!(
            e.message.contains("unknown backend 'cobol'"),
            "{}",
            e.message
        );

        let bad_ext = dir.join("ok.txt");
        std::fs::write(&bad_ext, "x").unwrap();
        let e = RfsBuild::new(&bad_ext).compile().unwrap_err();
        assert!(e.message.contains(".rfs or .json"), "{}", e.message);

        let missing = dir.join("ghost.rfs");
        let e = RfsBuild::new(&missing).compile().unwrap_err();
        assert!(e.message.contains("does not exist"), "{}", e.message);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn compile_success_and_error_propagation_with_stub() {
        let _g = ENV_LOCK.lock().unwrap();
        let dir = temp_dir("stub");
        let out = dir.join("out");
        std::fs::create_dir_all(&out).unwrap();
        let stub = write_stub(&dir);
        let spec = write_spec(&dir, "handshake.rfs");

        set_env(&[
            ("RFS_BIN", Some(stub.to_str().unwrap())),
            ("OUT_DIR", Some(out.to_str().unwrap())),
        ]);

        // Success: artifact path returned + file written by the stub.
        let artifact = RfsBuild::new(&spec)
            .backend("rust")
            .production()
            .with_property_monitors(true)
            .compile()
            .expect("stub compile should succeed");
        assert_eq!(artifact, out.join("handshake.rs"));
        assert_eq!(
            std::fs::read_to_string(&artifact).unwrap(),
            "pub struct Stub;\n"
        );

        // The stub recorded the exact argv (monitors flag included).
        let args = std::fs::read_to_string(dir.join("stub_args.txt")).unwrap();
        assert_eq!(
            args.trim(),
            format!(
                "codegen rust production {} --out {} --with-property-monitors",
                spec.display(),
                artifact.display()
            )
        );

        // C backend picks the .c extension.
        let artifact_c = RfsBuild::new(&spec).backend("c").compile().expect("c stub");
        assert_eq!(artifact_c, out.join("handshake.c"));

        // Failure: non-zero exit + stderr propagated into the error message.
        std::env::set_var("RFS_STUB_FAIL", "1");
        let e = RfsBuild::new(&spec).compile().unwrap_err();
        assert!(e.message.contains("exit 3"), "{}", e.message);
        assert!(
            e.message.contains("stub: deliberate failure"),
            "{}",
            e.message
        );

        set_env(&[
            ("RFS_BIN", None),
            ("OUT_DIR", None),
            ("RFS_STUB_FAIL", None),
        ]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn compile_without_out_dir_is_an_error() {
        let _g = ENV_LOCK.lock().unwrap();
        set_env(&[("OUT_DIR", None), ("RFS_BIN", None)]);
        let dir = temp_dir("noout");
        let spec = write_spec(&dir, "m.rfs");
        let e = RfsBuild::new(&spec).compile().unwrap_err();
        assert!(e.message.contains("OUT_DIR"), "{}", e.message);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn cargo_directives_pinned() {
        let dir = temp_dir("directives");
        let spec = write_spec(&dir, "handshake.rfs");
        let rfs_bin = PathBuf::from("/tools/rfs-1.2.3/bin/rfs");
        let artifact = PathBuf::from("/out/handshake.rs");

        // RFS_BIN case: watches the binary too.
        let d = RfsBuild::new(&spec).cargo_directives(&rfs_bin, &artifact);
        let lines: Vec<&str> = d.lines().collect();
        assert_eq!(lines.len(), 4);
        assert!(
            lines[0].starts_with("cargo:rerun-if-changed="),
            "{}",
            lines[0]
        );
        assert!(lines[0].ends_with("handshake.rfs"), "{}", lines[0]);
        assert_eq!(lines[1], "cargo:rerun-if-env-changed=RFS_BIN");
        assert_eq!(
            lines[2],
            format!("cargo:rerun-if-changed={}", rfs_bin.display())
        );
        assert_eq!(lines[3], "cargo:include=/out/handshake.rs");

        // PATH case: no binary watch line.
        let d = RfsBuild::new(&spec).cargo_directives(Path::new("rfs"), &artifact);
        assert_eq!(d.lines().count(), 3);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
