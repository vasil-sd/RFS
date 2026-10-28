//! External-tool resolution, invocation, and deterministic script emission.
//!
//! Phase 5 task 3 introduces the shared CLI infrastructure that every later
//! external-tool command reuses (`rfs test` tasks 7/8 compile/run with
//! rustc/gcc/dot). User requirement (2026-10-20): every tool PATH and its
//! extra parameters must be overridable, with ENVIRONMENT VARIABLES as the
//! preferred mechanism:
//!
//! - Paths: `RFS_RUSTC`, `RFS_GCC`, `RFS_DOT`, `RFS_PLANTUML`,
//!   `RFS_MERMAID`, `RFS_CLANG`, `RFS_CARGO_FUZZ` (rfs-build uses
//!   `RFS_BIN`).
//! - Extra parameters: `RFS_RUSTC_ARGS`, `RFS_GCC_ARGS`, `RFS_DOT_ARGS`,
//!   `RFS_PLANTUML_ARGS`, `RFS_MERMAID_ARGS`, `RFS_CLANG_ARGS`,
//!   `RFS_CARGO_FUZZ_ARGS` (space-separated, appended verbatim to the
//!   invocation).
//! - NOT a tool preset: `RFS_FUZZ_FLAGS` (Phase 5 task 17) carries EXTRA
//!   libFuzzer RUNTIME flags for `rfs test --suite fuzz` (both backends) -
//!   it has no tool path; the CLI `--fuzz-flags` value is appended after the
//!   env value (same convention as the `*_ARGS` pairs).
//!
//! Precedence for the tool PATH: **CLI flag > environment variable > bare
//! name on PATH**. For extra args both sources are used: env-var args first,
//! then CLI args appended (unit-test-pinned).
//!
//! `Script` renders the `--print-script` mode: a deterministic POSIX sh
//! script (no timestamps, no volatile temp paths) that performs exactly what
//! the command would have executed.
//! Docs: dev:cli-commands-overview, llm:cli-commands

use std::process::ExitStatus;

/// Preset for one external tool: its bare name and its env-var pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExtTool {
    /// Bare tool name (PATH lookup default) and display name.
    pub name: &'static str,
    /// Env var holding an explicit tool PATH (preferred override).
    pub env_path_var: &'static str,
    /// Env var holding space-separated extra parameters.
    pub env_args_var: &'static str,
    /// Fixed subcommand inserted right after the resolved path, before the
    /// extra args (Phase 5 task 17). Empty for plain tools; "fuzz" for
    /// CARGO_FUZZ, which is a subcommand of `cargo`.
    pub subcommand: &'static str,
}

impl ExtTool {
    pub const RUSTC: ExtTool = ExtTool {
        name: "rustc",
        env_path_var: "RFS_RUSTC",
        env_args_var: "RFS_RUSTC_ARGS",
        subcommand: "",
    };
    pub const GCC: ExtTool = ExtTool {
        name: "gcc",
        env_path_var: "RFS_GCC",
        env_args_var: "RFS_GCC_ARGS",
        subcommand: "",
    };
    pub const DOT: ExtTool = ExtTool {
        name: "dot",
        env_path_var: "RFS_DOT",
        env_args_var: "RFS_DOT_ARGS",
        subcommand: "",
    };
    /// PlantUML (diagram SVG rendering, Phase 5 task 8).
    pub const PLANTUML: ExtTool = ExtTool {
        name: "plantuml",
        env_path_var: "RFS_PLANTUML",
        env_args_var: "RFS_PLANTUML_ARGS",
        subcommand: "",
    };
    /// Mermaid CLI `mmdc` (diagram SVG rendering, Phase 5 task 8).
    pub const MERMAID: ExtTool = ExtTool {
        name: "mmdc",
        env_path_var: "RFS_MERMAID",
        env_args_var: "RFS_MERMAID_ARGS",
        subcommand: "",
    };
    /// clang with the libFuzzer runtime (C fuzz targets, Phase 5 task 17).
    pub const CLANG: ExtTool = ExtTool {
        name: "clang",
        env_path_var: "RFS_CLANG",
        env_args_var: "RFS_CLANG_ARGS",
        subcommand: "",
    };
    /// cargo-fuzz (Rust fuzz targets, Phase 5 task 17): a SUBCOMMAND of
    /// `cargo`, so the resolved path is the cargo binary and `fuzz` is
    /// inserted right after it.
    pub const CARGO_FUZZ: ExtTool = ExtTool {
        name: "cargo",
        env_path_var: "RFS_CARGO_FUZZ",
        env_args_var: "RFS_CARGO_FUZZ_ARGS",
        subcommand: "fuzz",
    };

    /// Resolve the tool PATH and extra parameters.
    ///
    /// PATH precedence: `cli_path` (non-empty), then `$env_path_var`
    /// (non-empty), then the bare name on PATH. Args: `$env_args_var`
    /// (split on whitespace) first, then `cli_args` (same split) appended
    /// verbatim.
    pub fn resolve(&self, cli_path: Option<&str>, cli_args: Option<&str>) -> ResolvedTool {
        let path = match non_empty(cli_path) {
            Some(p) => p.to_string(),
            None => match non_empty(std::env::var(self.env_path_var).as_deref().ok()) {
                Some(p) => p.to_string(),
                None => self.name.to_string(),
            },
        };
        let mut extra_args = split_args(std::env::var(self.env_args_var).as_deref().ok());
        if let Some(a) = cli_args {
            extra_args.extend(split_args(Some(a)));
        }
        ResolvedTool {
            name: self.name.to_string(),
            path,
            subcommand: self.subcommand.to_string(),
            extra_args,
        }
    }
}

fn non_empty(s: Option<&str>) -> Option<&str> {
    s.map(str::trim).filter(|t| !t.is_empty())
}

fn split_args(s: Option<&str>) -> Vec<String> {
    s.map(|t| t.split_whitespace().map(String::from).collect())
        .unwrap_or_default()
}

/// A resolved tool: concrete PATH + the extra parameters to append.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedTool {
    /// The preset's bare/display name (e.g. "rustc").
    pub name: String,
    /// What to actually execute (may be a bare PATH-lookup name).
    pub path: String,
    /// Fixed subcommand right after the path, before the extra args
    /// ("fuzz" for cargo-fuzz; empty for plain tools).
    pub subcommand: String,
    /// Extra parameters appended after the tool, before its real arguments.
    pub extra_args: Vec<String>,
}

impl ResolvedTool {
    /// Display name for diagnostics ("cargo-fuzz" when a subcommand is set).
    pub fn display_name(&self) -> String {
        if self.subcommand.is_empty() {
            self.name.clone()
        } else {
            format!("{}-{}", self.name, self.subcommand)
        }
    }

    /// Render one logical command line with POSIX sh quoting.
    pub fn render(&self, args: &[String]) -> String {
        let mut parts = vec![sh_quote(&self.path)];
        if !self.subcommand.is_empty() {
            parts.push(self.subcommand.clone());
        }
        for a in &self.extra_args {
            parts.push(sh_quote(a));
        }
        for a in args {
            parts.push(sh_quote(a));
        }
        parts.join(" ")
    }

    /// Execute the tool, capturing stdout/stderr for diagnostics.
    pub fn run(&self, args: &[String]) -> std::io::Result<ToolOutput> {
        self.run_impl(None, &[], args)
    }

    /// Execute the tool from a specific working directory (Phase 5 task 17:
    /// `cargo fuzz` resolves its project from the CWD, so the temp scaffold
    /// must be it).
    pub fn run_in(&self, cwd: &std::path::Path, args: &[String]) -> std::io::Result<ToolOutput> {
        self.run_impl(Some(cwd), &[], args)
    }

    /// Like `run_in`, but with the listed environment variables REMOVED from
    /// the child (Phase 5 task 17: a parent rustup may pin
    /// RUSTUP_TOOLCHAIN=stable, which would override the scaffold's
    /// rust-toolchain.toml nightly pin).
    pub fn run_in_without(
        &self,
        cwd: &std::path::Path,
        remove: &[&str],
        args: &[String],
    ) -> std::io::Result<ToolOutput> {
        self.run_impl(Some(cwd), remove, args)
    }

    fn run_impl(
        &self,
        cwd: Option<&std::path::Path>,
        remove: &[&str],
        args: &[String],
    ) -> std::io::Result<ToolOutput> {
        let mut cmd = std::process::Command::new(&self.path);
        if !self.subcommand.is_empty() {
            cmd.arg(&self.subcommand);
        }
        if let Some(c) = cwd {
            cmd.current_dir(c);
        }
        for r in remove {
            cmd.env_remove(r);
        }
        let out = cmd.args(&self.extra_args).args(args).output()?;
        Ok(ToolOutput {
            status: out.status,
            stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
        })
    }
}

/// Captured result of an external-tool invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolOutput {
    pub status: ExitStatus,
    pub stdout: String,
    pub stderr: String,
}

impl ToolOutput {
    pub fn success(&self) -> bool {
        self.status.success()
    }
}

/// POSIX sh quoting for one word: bare if it consists only of safe
/// characters, otherwise single-quoted with embedded `'` escaped as `\'\'`.
pub fn sh_quote(s: &str) -> String {
    let safe = !s.is_empty()
        && s.bytes().all(|b| {
            matches!(
                b,
                b'A'..=b'Z'
                    | b'a'..=b'z'
                    | b'0'..=b'9'
                    | b'_'
                    | b'.'
                    | b'/'
                    | b':'
                    | b'-'
                    | b'+'
                    | b'='
                    | b'@'
                    | b'^'
                    | b'%'
                    | b','
            )
        });
    if safe {
        s.to_string()
    } else {
        format!("'{}'", s.replace('\'', "'\\''"))
    }
}

/// Deterministic POSIX sh script builder (`--print-script` mode).
///
/// Layout (byte-pinned by unit tests):
/// ```sh
/// #!/bin/sh
/// # rfs <version> - generated by `<command>` on <spec>
/// # tools: rustc=/path/to/rustc (args: --x)
/// set -e
/// # step 1: compile
/// /path/to/rustc ...
/// ```
/// No timestamps, no volatile temp paths: only the paths the user will
/// actually use, artifact names relative.
#[derive(Debug, Clone)]
pub struct Script {
    /// The invoking command line, e.g. `rfs test --backend rust spec.rfs`.
    command_desc: String,
    /// The spec path the command operates on (header comment only).
    spec: String,
    tools: Vec<ResolvedTool>,
    /// Header notes emitted after the `# tools:` lines (e.g. required
    /// external runtimes for the fuzz suites, Phase 5 task 17).
    notes: Vec<String>,
    /// (comment, rendered command line) pairs, executed in order.
    steps: Vec<(String, String)>,
}

impl Script {
    pub fn new(command_desc: &str, spec: &str) -> Self {
        Self {
            command_desc: command_desc.to_string(),
            spec: spec.to_string(),
            tools: Vec::new(),
            notes: Vec::new(),
            steps: Vec::new(),
        }
    }

    /// Record a header note (emitted as `# note: ...` after the tools).
    pub fn with_note(mut self, note: &str) -> Self {
        self.notes.push(note.to_string());
        self
    }

    /// Record a resolved tool (emits one `# tools:` header line).
    pub fn with_tool(mut self, tool: ResolvedTool) -> Self {
        self.tools.push(tool);
        self
    }

    /// Append an execution step.
    pub fn step(mut self, comment: &str, cmdline: &str) -> Self {
        self.steps.push((comment.to_string(), cmdline.to_string()));
        self
    }

    /// Render the full script text (deterministic).
    pub fn render(&self) -> String {
        let mut out = String::new();
        out.push_str("#!/bin/sh\n");
        out.push_str(&format!(
            "# rfs {} - generated by `{}` on {}\n",
            env!("CARGO_PKG_VERSION"),
            self.command_desc,
            self.spec
        ));
        for t in &self.tools {
            if t.subcommand.is_empty() {
                if t.extra_args.is_empty() {
                    out.push_str(&format!("# tools: {}={}\n", t.name, t.path));
                } else {
                    let args = t.extra_args.clone();
                    out.push_str(&format!(
                        "# tools: {}={} (args: {})\n",
                        t.name,
                        t.path,
                        args.join(" ")
                    ));
                }
            } else if t.extra_args.is_empty() {
                out.push_str(&format!(
                    "# tools: {}-{}={} (subcommand: {})\n",
                    t.name, t.subcommand, t.path, t.subcommand
                ));
            } else {
                let args = t.extra_args.clone();
                out.push_str(&format!(
                    "# tools: {}-{}={} (subcommand: {}, args: {})\n",
                    t.name,
                    t.subcommand,
                    t.path,
                    t.subcommand,
                    args.join(" ")
                ));
            }
        }
        for n in &self.notes {
            out.push_str(&format!("# note: {n}\n"));
        }
        out.push_str("set -e\n");
        for (i, (comment, cmdline)) in self.steps.iter().enumerate() {
            out.push_str(&format!("# step {}: {}\n", i + 1, comment));
            out.push_str(cmdline);
            out.push('\n');
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Env-mutating tests share process-global state - serialize them.
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn set_env(pairs: &[(&str, Option<&str>)]) {
        for (k, v) in pairs {
            match v {
                Some(val) => std::env::set_var(k, val),
                None => std::env::remove_var(k),
            }
        }
    }

    #[test]
    fn resolve_precedence_flag_beats_env_beats_default() {
        let _g = ENV_LOCK.lock().unwrap();
        set_env(&[
            ("RFS_RUSTC", Some("/opt/custom/rustc")),
            ("RFS_RUSTC_ARGS", None),
        ]);

        // 1. CLI flag wins over env var.
        let r = ExtTool::RUSTC.resolve(Some("/flag/rustc"), None);
        assert_eq!(r.path, "/flag/rustc");

        // 2. Env var wins over the bare default.
        let r = ExtTool::RUSTC.resolve(None, None);
        assert_eq!(r.path, "/opt/custom/rustc");

        // 3. Bare name when neither is set (empty string counts as unset).
        set_env(&[("RFS_RUSTC", None), ("RFS_RUSTC", Some(""))]);
        let r = ExtTool::RUSTC.resolve(Some(""), None);
        assert_eq!(r.path, "rustc");

        // Unknown tool preset shape: GCC/DOT use their own vars.
        set_env(&[("RFS_GCC", Some("/usr/bin/gcc-custom"))]);
        assert_eq!(ExtTool::GCC.resolve(None, None).path, "/usr/bin/gcc-custom");
        assert_eq!(ExtTool::DOT.resolve(None, None).path, "dot");
        set_env(&[("RFS_RUSTC", None), ("RFS_GCC", None)]);
    }

    #[test]
    fn resolve_args_env_first_cli_appended() {
        let _g = ENV_LOCK.lock().unwrap();
        set_env(&[("RFS_DOT", None), ("RFS_DOT_ARGS", Some("-Tsvg -Gdpi=150"))]);

        // Env args only.
        let r = ExtTool::DOT.resolve(None, None);
        assert_eq!(
            r.extra_args,
            vec!["-Tsvg".to_string(), "-Gdpi=150".to_string()]
        );

        // CLI args appended AFTER env args (order pinned).
        let r = ExtTool::DOT.resolve(None, Some("-x -y"));
        assert_eq!(
            r.extra_args,
            vec![
                "-Tsvg".to_string(),
                "-Gdpi=150".to_string(),
                "-x".to_string(),
                "-y".to_string(),
            ]
        );

        set_env(&[("RFS_DOT_ARGS", None)]);
    }

    #[test]
    fn sh_quoting_rules() {
        assert_eq!(sh_quote("rustc"), "rustc");
        assert_eq!(sh_quote("/usr/bin/rustc"), "/usr/bin/rustc");
        assert_eq!(sh_quote("-Clink-arg=-lz3"), "-Clink-arg=-lz3");
        // Space forces quoting.
        assert_eq!(sh_quote("/opt/my tools/rustc"), "'/opt/my tools/rustc'");
        // Embedded single quote escaped as '\' inside single quotes.
        assert_eq!(sh_quote("a'b"), "'a'\\''b'");
        // Empty string is quoted (bare would vanish).
        assert_eq!(sh_quote(""), "''");
    }

    #[test]
    fn render_command_line_with_extra_args() {
        let t = ResolvedTool {
            name: "rustc".to_string(),
            path: "/opt/my tools/rustc".to_string(),
            subcommand: String::new(),
            extra_args: vec!["-C".to_string(), "debuginfo=2".to_string()],
        };
        assert_eq!(
            t.render(&["--edition 2021".to_string(), "in.rs".to_string()]),
            "'/opt/my tools/rustc' -C debuginfo=2 '--edition 2021' in.rs"
        );
    }

    #[test]
    fn script_output_byte_pinned() {
        let rustc = ResolvedTool {
            name: "rustc".to_string(),
            path: "/usr/local/bin/rustc".to_string(),
            subcommand: String::new(),
            extra_args: vec!["--edition 2021".to_string()],
        };
        let script = Script::new("rfs test --backend rust spec.rfs", "spec.rfs")
            .with_tool(rustc)
            .step(
                "compile",
                "/usr/local/bin/rustc --edition 2021 in.rs -o out.rlib",
            )
            .step("run", "./out");
        assert_eq!(
            script.render(),
            "#!/bin/sh\n# rfs 1.0.0 - generated by `rfs test --backend rust spec.rfs` on spec.rfs\n# tools: rustc=/usr/local/bin/rustc (args: --edition 2021)\nset -e\n# step 1: compile\n/usr/local/bin/rustc --edition 2021 in.rs -o out.rlib\n# step 2: run\n./out\n"
        );
    }

    #[test]
    fn script_without_tools_or_args() {
        let script = Script::new("rfs diagram --format dot spec.rfs", "spec.rfs")
            .step("emit dot", "echo done");
        assert_eq!(
            script.render(),
            "#!/bin/sh\n# rfs 1.0.0 - generated by `rfs diagram --format dot spec.rfs` on spec.rfs\nset -e\n# step 1: emit dot\necho done\n"
        );
    }

    /// Phase 5 task 17 presets: CLANG follows the standard precedence;
    /// CARGO_FUZZ resolves the CARGO path (the `fuzz` subcommand is fixed).
    #[test]
    fn clang_and_cargo_fuzz_presets_resolve() {
        let _g = ENV_LOCK.lock().unwrap();
        set_env(&[
            ("RFS_CLANG", Some("/opt/custom/clang")),
            ("RFS_CARGO_FUZZ", Some("/opt/custom/cargo")),
        ]);

        // CLANG: flag > env var > bare name.
        assert_eq!(
            ExtTool::CLANG.resolve(Some("/flag/clang"), None).path,
            "/flag/clang"
        );
        assert_eq!(ExtTool::CLANG.resolve(None, None).path, "/opt/custom/clang");
        set_env(&[("RFS_CLANG", None)]);
        assert_eq!(ExtTool::CLANG.resolve(None, None).path, "clang");

        // CARGO_FUZZ: the env var overrides the CARGO path; subcommand fixed.
        let r = ExtTool::CARGO_FUZZ.resolve(None, None);
        assert_eq!(r.path, "/opt/custom/cargo");
        assert_eq!(r.subcommand, "fuzz");
        set_env(&[("RFS_CARGO_FUZZ", None)]);
        let r = ExtTool::CARGO_FUZZ.resolve(None, None);
        assert_eq!(r.path, "cargo");
        assert_eq!(r.display_name(), "cargo-fuzz");
    }

    /// The fixed subcommand is rendered/invoked right after the path, before
    /// the extra args (env first, then CLI), then the real arguments.
    #[test]
    fn cargo_fuzz_subcommand_placement_pinned() {
        let _g = ENV_LOCK.lock().unwrap();
        set_env(&[("RFS_CARGO_FUZZ_ARGS", Some("--env-arg"))]);
        let t = ExtTool::CARGO_FUZZ.resolve(Some("/opt/cargo"), Some("--cli-arg"));

        // Render: path, subcommand, env args, CLI args, real args.
        assert_eq!(
            t.render(&["run".to_string(), "fuzz_target".to_string()]),
            "/opt/cargo fuzz --env-arg --cli-arg run fuzz_target"
        );

        // Run: the stub sees the same argv order.
        let dir = std::env::temp_dir().join(format!("rfs_ext_tool_{}_cfz", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let stub = dir.join("stub_cargo");
        std::fs::write(&stub, "#!/bin/sh\necho \"ARGS:$@\"\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut p = std::fs::metadata(&stub).unwrap().permissions();
            p.set_mode(0o755);
            std::fs::set_permissions(&stub, p).unwrap();
        }
        let t2 = ExtTool::CARGO_FUZZ.resolve(Some(stub.to_str().unwrap()), None);
        let out = t2.run(&["build".to_string()]).expect("stub runs");
        assert_eq!(out.stdout.trim(), "ARGS:fuzz --env-arg build");

        // Script header for a subcommand tool (byte-pinned).
        let script = Script::new("rfs test --backend rust spec.rfs", "spec.rfs")
            .with_tool(t.clone())
            .step("build", &t.render(&["build".to_string()]));
        assert_eq!(
            script.render(),
            "#!/bin/sh\n# rfs 1.0.0 - generated by `rfs test --backend rust spec.rfs` on spec.rfs\n# tools: cargo-fuzz=/opt/cargo (subcommand: fuzz, args: --env-arg --cli-arg)\nset -e\n# step 1: build\n/opt/cargo fuzz --env-arg --cli-arg build\n"
        );

        set_env(&[("RFS_CARGO_FUZZ_ARGS", None)]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// End-to-end with a stub tool: proves resolution actually reaches the
    /// invoked process (path + extra args in the pinned order).
    #[test]
    fn stub_tool_receives_resolved_path_and_args() {
        let _g = ENV_LOCK.lock().unwrap();
        let dir = std::env::temp_dir().join(format!("rfs_ext_tool_{}_stub", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let stub = dir.join("stub_rustc");
        std::fs::write(&stub, "#!/bin/sh\necho \"ARGS:$@\"\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut p = std::fs::metadata(&stub).unwrap().permissions();
            p.set_mode(0o755);
            std::fs::set_permissions(&stub, p).unwrap();
        }

        // Env-var path + env args + CLI args appended.
        set_env(&[
            ("RFS_RUSTC", Some(stub.to_str().unwrap())),
            ("RFS_RUSTC_ARGS", Some("-env1 -env2")),
        ]);
        let t = ExtTool::RUSTC.resolve(None, Some("-cli"));
        let out = t.run(&["real-arg".to_string()]).expect("stub runs");
        assert!(out.success(), "stderr: {}", out.stderr);
        assert_eq!(out.stdout.trim(), "ARGS:-env1 -env2 -cli real-arg");

        // CLI flag path beats the env var.
        let other = dir.join("other_stub");
        std::fs::write(&other, "#!/bin/sh\necho \"OTHER\"\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut p = std::fs::metadata(&other).unwrap().permissions();
            p.set_mode(0o755);
            std::fs::set_permissions(&other, p).unwrap();
        }
        let t = ExtTool::RUSTC.resolve(Some(other.to_str().unwrap()), None);
        let out = t.run(&[]).expect("stub runs");
        assert_eq!(out.stdout.trim(), "OTHER");

        set_env(&[("RFS_RUSTC", None), ("RFS_RUSTC_ARGS", None)]);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
