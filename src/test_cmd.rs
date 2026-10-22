//! The `rfs test` command (Phase 5 task 7): generate + compile + run the
//! extended test suite for a spec in ONE invocation, for either backend -
//! the CI entry point for generated tests.
//!
//! Pipeline: parse+validate spec -> codegen the `test` artifact (existing
//! registry dispatch) into a temp dir -> compile with the resolved tool
//! (`rustc --edition 2021 --test` for the rust backend, `gcc -Wall` for C;
//! task 3's resolver: CLI flag > env var > PATH) -> run the executable.
//! Per-test results stream on stdout (the executables print them); tool
//! resolution + compile diagnostics go to stderr via the logging helper.
//! `--print-script` prints a deterministic, self-contained POSIX sh script
//! instead of executing (task 3's Script emitter; no volatile temp paths).

use std::path::{Path, PathBuf};

use crate::cli_log::Logger;
use crate::codegen::{self, BackendOptions, CodegenCommandKind};
use crate::ext_tool::{sh_quote, ExtTool, ResolvedTool, Script};

/// Options for `rfs test`.
pub struct TestOptions {
    /// Backend name (must be a registered codegen backend: "rust" / "c").
    pub backend: String,
    /// Example-run length K passed to codegen (default 4).
    pub steps: usize,
    /// CLI tool overrides - task 3's resolver precedence is flag > env var
    /// (`RFS_RUSTC`/`RFS_GCC`) > bare name on PATH; `*_args` values are
    /// space-separated extra parameters (env args first, then CLI args).
    pub rustc_path: Option<String>,
    pub gcc_path: Option<String>,
    pub rustc_args: Option<String>,
    pub gcc_args: Option<String>,
    /// Print the compile+run script instead of executing.
    pub print_script: bool,
}

impl Default for TestOptions {
    fn default() -> Self {
        Self {
            backend: "rust".into(),
            steps: 4,
            rustc_path: None,
            gcc_path: None,
            rustc_args: None,
            gcc_args: None,
            print_script: false,
        }
    }
}

/// Outcome of a successful `rfs test` invocation.
pub struct TestOutcome {
    /// The `--print-script` text (None in execution mode).
    pub script: Option<String>,
    /// Informational note for the logger (stderr, info level).
    pub note: String,
}

/// Run the full pipeline. Ok on all-pass (or script printed); Err carries a
/// one-line diagnostic - details are already on stderr.
pub fn run_test(spec: &Path, opts: &TestOptions, log: &Logger) -> Result<TestOutcome, String> {
    // 1. Load + validate (same contract as `rfs info` / `rfs diagram`).
    let ext = spec.extension().and_then(|e| e.to_str()).unwrap_or("");
    let Some(fmt) = crate::frontend::Format::from_extension(ext) else {
        return Err(format!(
            "input '{}' must have extension .rfs or .json",
            spec.display()
        ));
    };
    let ir = crate::frontend::convert_file_to_ir(spec, fmt)
        .map_err(|e| format!("{}: {}", spec.display(), e))?;
    if ir.modules.is_empty() {
        return Err(format!("no modules in {}", spec.display()));
    }
    let errors = match crate::validator::validate_ir(&ir) {
        Ok(()) => Vec::new(),
        Err(e) => e,
    };
    if !errors.is_empty() {
        let msgs: Vec<String> = errors.iter().map(|e| e.message.clone()).collect();
        return Err(format!("{}: {}", spec.display(), msgs.join("; ")));
    }
    let module = &ir.modules[0];

    // 2. Resolve the backend and the external tool (task 3's resolver).
    let backend = codegen::get_backend(&opts.backend).ok_or_else(|| {
        format!(
            "unknown backend '{}' (registered: {})",
            opts.backend,
            codegen::list_backends()
                .iter()
                .map(|(n, _)| *n)
                .collect::<Vec<_>>()
                .join(", ")
        )
    })?;
    let preset = if opts.backend == "rust" {
        ExtTool::RUSTC
    } else {
        ExtTool::GCC
    };
    let (cli_path, cli_args) = if opts.backend == "rust" {
        (opts.rustc_path.as_deref(), opts.rustc_args.as_deref())
    } else {
        (opts.gcc_path.as_deref(), opts.gcc_args.as_deref())
    };
    let resolved = preset.resolve(cli_path, cli_args);

    // 3. Codegen the test artifact via the registry dispatch (no logic
    // duplicated from the renderers). The fuzz skeleton extra file is NOT
    // part of this command (building/running it needs cargo-fuzz/clang -
    // task 12 wires the examples).
    let options = BackendOptions {
        assumptions: codegen::AssumptionSelection::All,
        steps: opts.steps,
        with_property_monitors: false,
        ..BackendOptions::default()
    };
    let out = CodegenCommandKind::UnitTests
        .run(backend, module, &options)
        .map_err(|e| e.join("; "))?;
    if !out.extra_files.is_empty() {
        log.debug(&format!(
            "skipping {} extra file(s) (fuzz targets need cargo-fuzz/clang - see the Phase 5 task 12 examples)",
            out.extra_files.len()
        ));
    }

    // 4. --print-script: deterministic POSIX sh, no execution.
    if opts.print_script {
        return Ok(TestOutcome {
            script: Some(render_script(spec, opts, &resolved, &out.code)),
            note: "printed script (no execution)".into(),
        });
    }

    // 5. Execution pipeline in a temp dir (cleaned on EVERY path via Drop).
    let dir = make_temp_dir()?;
    struct Guard(PathBuf);
    impl Drop for Guard {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    let _guard = Guard(dir.clone());

    let (src_name, bin_name, compile_args) = artifact_names(opts);
    let src_path = dir.join(src_name);
    let bin_path = dir.join(bin_name);
    std::fs::write(&src_path, &out.code)
        .map_err(|e| format!("failed to write {}: {e}", src_path.display()))?;

    log.info(&format!(
        "compiling {} with {}",
        src_path.display(),
        resolved.path
    ));
    let mut args = compile_args;
    args.push(src_path.to_string_lossy().into_owned());
    args.push("-o".into());
    args.push(bin_path.to_string_lossy().into_owned());
    let compile = resolved
        .run(&args)
        .map_err(|e| format!("failed to run {}: {e}", resolved.name))?;
    if !compile.success() {
        eprintln!("{}", compile.stderr);
        return Err(format!("{} compilation failed", resolved.name));
    }
    for line in compile.stderr.lines() {
        log.warn(line);
    }

    log.info(&format!("running {}", bin_path.display()));
    let run = std::process::Command::new(&bin_path)
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::piped())
        .output()
        .map_err(|e| format!("failed to run {}: {e}", bin_path.display()))?;
    if !run.status.success() {
        eprintln!("{}", String::from_utf8_lossy(&run.stderr));
        return Err("generated tests failed".to_string());
    }
    Ok(TestOutcome {
        script: None,
        note: format!(
            "all generated {} tests passed ({} backend)",
            opts.backend, resolved.name
        ),
    })
}

/// Temp workspace for one invocation: unique per process + call counter.
static COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

fn make_temp_dir() -> Result<PathBuf, String> {
    let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("rfs_test_{}_{}", std::process::id(), n));
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("failed to create temp dir {}: {e}", dir.display()))
        .map(|_| dir)
}

/// Artifact names + compile arguments per backend (shared by execution and
/// script modes so the two can never drift).
fn artifact_names(opts: &TestOptions) -> (&'static str, &'static str, Vec<String>) {
    if opts.backend == "rust" {
        // `--test` enables cfg(test) + links the libtest harness - the
        // artifact is production code plus a #[cfg(test)] module.
        (
            "spec_test.rs",
            "spec_test_bin",
            vec!["--edition".into(), "2021".into(), "--test".into()],
        )
    } else {
        ("spec_test.c", "spec_test_bin", vec!["-Wall".into()])
    }
}

/// Build the `--print-script` output: a self-contained POSIX sh script that
/// writes the generated test source (quoted heredoc), compiles it with the
/// resolved tool, runs the tests, and cleans up - exactly what execution
/// mode would have done, minus the volatile temp dir (CWD-relative names).
fn render_script(spec: &Path, opts: &TestOptions, resolved: &ResolvedTool, source: &str) -> String {
    let mut desc = format!("rfs test --backend {}", opts.backend);
    if opts.steps != 4 {
        desc.push_str(&format!(" --steps {}", opts.steps));
    }
    desc.push(' ');
    desc.push_str(&spec.display().to_string());

    let (src_name, bin_name, compile_args) = artifact_names(opts);
    // Heredoc with a quoted delimiter: no shell expansion of the content.
    let src_text = if source.ends_with('\n') {
        source.to_string()
    } else {
        format!("{source}\n")
    };
    let heredoc = format!(
        "cat > {} <<'__RFS_TEST_SOURCE__'\n{}__RFS_TEST_SOURCE__",
        sh_quote(src_name),
        src_text
    );

    let mut args = compile_args;
    args.push(src_name.to_string());
    args.push("-o".into());
    args.push(bin_name.to_string());

    Script::new(&desc, &spec.display().to_string())
        .with_tool(resolved.clone())
        .step(
            "write generated test artifact (from `rfs codegen <backend> test`)",
            &heredoc,
        )
        .step(
            "compile the generated test artifact",
            &resolved.render(&args),
        )
        .step("run the generated tests", &format!("./{bin_name}"))
        .step(
            "clean up generated artifacts",
            &format!("rm -f {src_name} {bin_name}"),
        )
        .render()
}
