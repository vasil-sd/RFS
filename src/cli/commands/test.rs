//! The `rfs test` command (Phase 5 task 7, extended by task 17): generate +
//! compile + run the selected test suites for a spec in ONE invocation, for
//! either backend - the CI entry point for generated tests.
//!
//! Suites (`--suite unit|fuzz`, repeatable/comma-separated, DEFAULT unit):
//! - `unit` (task 7): codegen the `test` artifact (existing registry
//!   dispatch) into a temp dir -> compile with the resolved tool (`rustc
//!   --edition 2021 --test` for the rust backend, `gcc -Wall` for C; task 3's
//!   resolver: CLI flag > env var > PATH) -> run the executable.
//! - `fuzz` (task 17): build + bounded-run the libFuzzer target from the
//!   EMITTED skeleton extra file - C: clang `-O1 -fsanitize=fuzzer,address`
//!   over `fuzz_target.c` + the production source under its hardcoded include
//!   name; Rust: a temp cargo-fuzz project (lib root = generated production
//!   source, adapted skeleton in `fuzz/fuzz_targets/`) via `cargo fuzz
//!   build` / `cargo fuzz run`. Missing clang/cargo-fuzz is an error ONLY
//!   when the fuzz suite is selected.
//!
//! Per-test results stream on stdout (the executables print them); tool
//! resolution + compile diagnostics go to stderr via the logging helper.
//! `--print-script` prints a deterministic, self-contained POSIX sh script
//! instead of executing (task 3's Script emitter; no volatile temp paths).
//! Docs: dev:cli-commands-overview, dev:cli-commands-test, llm:cli-commands

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use crate::cli::args::usage;

use crate::cli::ext_tool::{sh_quote, ExtTool, ResolvedTool, Script};
use crate::cli::log::Logger;
use crate::codegen::{self, Backend, BackendOptions, CodegenCommandKind};

/// A test suite selectable with `--suite` (Phase 5 task 17).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestSuite {
    /// The generated unit-test executable (task 7 behavior; the default).
    Unit,
    /// The libFuzzer target built from the emitted skeleton extra file.
    Fuzz,
}

impl TestSuite {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "unit" => Some(Self::Unit),
            "fuzz" => Some(Self::Fuzz),
            _ => None,
        }
    }
}

/// Options for `rfs test`.
pub struct TestOptions {
    /// Backend name (must be a registered codegen backend: "rust" / "c").
    pub backend: String,
    /// Example-run length K passed to codegen (default 4).
    pub steps: usize,
    /// CLI tool overrides - task 3's resolver precedence is flag > env var
    /// (`RFS_RUSTC`/`RFS_GCC`/`RFS_CLANG`/`RFS_CARGO_FUZZ`) > bare name on
    /// PATH; `*_args` values are space-separated extra parameters (env args
    /// first, then CLI args).
    pub rustc_path: Option<String>,
    pub gcc_path: Option<String>,
    pub rustc_args: Option<String>,
    pub gcc_args: Option<String>,
    pub clang_path: Option<String>,
    pub clang_args: Option<String>,
    pub cargo_fuzz_path: Option<String>,
    pub cargo_fuzz_args: Option<String>,
    /// Selected suites (default `[Unit]` - task 7 behavior unchanged).
    pub suites: Vec<TestSuite>,
    /// libFuzzer's `-max_total_time` budget in seconds (default 30).
    pub fuzz_max_total_time: u64,
    /// Extra libFuzzer RUNTIME flags, CLI part; the `RFS_FUZZ_FLAGS` env
    /// value comes first (same convention as the `*_ARGS` pairs).
    pub fuzz_flags: Option<String>,
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
            clang_path: None,
            clang_args: None,
            cargo_fuzz_path: None,
            cargo_fuzz_args: None,
            suites: vec![TestSuite::Unit],
            fuzz_max_total_time: 30,
            fuzz_flags: None,
            print_script: false,
        }
    }
}

/// The `RFS_FUZZ_FLAGS` env value first, then the CLI value (whitespace-
/// split, appended verbatim - the `*_ARGS` convention).
fn resolved_fuzz_flags(opts: &TestOptions) -> Vec<String> {
    let mut v = std::env::var("RFS_FUZZ_FLAGS")
        .ok()
        .map(|t| t.split_whitespace().map(String::from).collect::<Vec<_>>())
        .unwrap_or_default();
    if let Some(a) = &opts.fuzz_flags {
        v.extend(a.split_whitespace().map(String::from));
    }
    v
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
    let Some(fmt) = crate::cli::utils::Format::from_extension(ext) else {
        return Err(format!(
            "input '{}' must have extension .rfs or .json",
            spec.display()
        ));
    };
    let ir = crate::cli::utils::convert_file_to_ir(spec, fmt)
        .map_err(|e| format!("{}: {}", spec.display(), e))?;
    if ir.modules.is_empty() {
        return Err(format!("no modules in {}", spec.display()));
    }
    let errors = match crate::lang::validator::validate_ir(&ir) {
        Ok(()) => Vec::new(),
        Err(e) => e,
    };
    if !errors.is_empty() {
        let msgs: Vec<String> = errors.iter().map(|e| e.message.clone()).collect();
        return Err(format!("{}: {}", spec.display(), msgs.join("; ")));
    }
    // 2. Resolve the backend and the external tools (task 3's resolver).
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
    // duplicated from the renderers). The fuzz skeleton extra file is
    // consumed by the fuzz suite when selected (task 17).
    let options = BackendOptions {
        assumptions: codegen::AssumptionSelection::All,
        steps: opts.steps,
        with_property_monitors: false,
        ..BackendOptions::default()
    };
    let out = CodegenCommandKind::UnitTests
        .run(backend, &ir, &options)
        .map_err(|e| e.join("; "))?;
    let run_unit = opts.suites.contains(&TestSuite::Unit);
    let run_fuzz = opts.suites.contains(&TestSuite::Fuzz);
    if out.fuzz_target.is_some() {
        if run_fuzz {
            log.debug("consuming the emitted fuzz skeleton for the fuzz suite");
        } else {
            log.debug("skipping the emitted fuzz skeleton (select it with `--suite fuzz`)");
        }
    }

    // The fuzz pipeline needs the production source (second registry
    // dispatch, same options - no renderer logic duplicated) and the emitted
    // skeleton. Generated BEFORE --print-script so both modes share them.
    let fuzz_ctx = if run_fuzz {
        Some(build_fuzz_context(opts, backend, &ir, &options, &out)?)
    } else {
        None
    };

    // 4. --print-script: deterministic POSIX sh, no execution.
    if opts.print_script {
        return Ok(TestOutcome {
            script: Some(render_script(
                spec,
                opts,
                &resolved,
                &out,
                fuzz_ctx.as_ref(),
            )),
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

    // Sequential per suite (v1: no parallelism); the first failure is
    // reported, but every SELECTED suite gets to run so one invocation
    // surfaces all problems.
    let mut failures: Vec<String> = Vec::new();
    if run_unit {
        if let Err(e) = run_unit_suite(opts, &resolved, &out, &dir, log) {
            failures.push(e);
        }
    }
    if run_fuzz {
        if let Some(ctx) = &fuzz_ctx {
            if let Err(e) = run_fuzz_suite(opts, ctx, &dir, log) {
                failures.push(e);
            }
        }
    }
    if !failures.is_empty() {
        return Err(failures.join("; "));
    }

    // Success note per executed suite (the unit-only text is task 7's).
    let mut notes = Vec::new();
    if run_unit {
        notes.push(format!(
            "all generated {} tests passed ({} backend)",
            opts.backend, resolved.name
        ));
    }
    if run_fuzz {
        notes.push(format!(
            "fuzz target ran within its budget without crash ({} backend)",
            opts.backend
        ));
    }
    Ok(TestOutcome {
        script: None,
        note: notes.join("; "),
    })
}

/// Unit suite (task 7 pipeline): write the test artifact and, for C, the fsm
/// pair it references; compile with the resolved tool (C: object+link - the
/// test source #includes the fsm header, so both sources are compiled to
/// objects and linked); run the executable.
fn run_unit_suite(
    opts: &TestOptions,
    resolved: &ResolvedTool,
    out: &crate::codegen::CodegenOutput,
    dir: &Path,
    log: &Logger,
) -> Result<(), String> {
    let (src_name, bin_name, compile_args) = artifact_names(opts);
    let src_path = dir.join(src_name);
    let bin_path = dir.join(bin_name);
    std::fs::write(&src_path, out.primary(CodegenCommandKind::UnitTests))
        .map_err(|e| format!("failed to write {}: {e}", src_path.display()))?;

    if opts.backend == "c" {
        // Phase 14 task 4: the test source references the fsm header - write the
        // fsm pair alongside (under the standalone default names, since `rfs
        // test` does not stamp custom reference names) and build via object+link.
        let fsm_header_name = "rfs_fsm.h";
        let fsm_source_name = "rfs_fsm.c";
        let header_path = dir.join(fsm_header_name);
        let source_path = dir.join(fsm_source_name);
        std::fs::write(
            &header_path,
            out.fsm_header
                .as_deref()
                .expect("c test fills the fsm-header slot"),
        )
        .map_err(|e| format!("failed to write {}: {e}", header_path.display()))?;
        std::fs::write(
            &source_path,
            out.fsm_code
                .as_deref()
                .expect("c test fills the fsm-code slot"),
        )
        .map_err(|e| format!("failed to write {}: {e}", source_path.display()))?;

        let fsm_obj = dir.join("rfs_fsm.o");
        let test_obj = dir.join("spec_test.o");
        for (c_src, c_obj) in [(&source_path, &fsm_obj), (&src_path, &test_obj)] {
            log.info(&format!(
                "compiling {} with {}",
                c_src.display(),
                resolved.path
            ));
            let mut args = compile_args.clone();
            args.push("-c".into());
            args.push(c_src.to_string_lossy().into_owned());
            args.push("-I".into());
            args.push(dir.to_string_lossy().into_owned());
            args.push("-o".into());
            args.push(c_obj.to_string_lossy().into_owned());
            let compile = resolved
                .run(&args)
                .map_err(|e| format!("failed to run {}: {e}", resolved.name))?;
            if !compile.success() {
                eprintln!("{}", compile.stderr);
                return Err(format!("{} compilation failed", resolved.name));
            }
            for line in compile.stderr.lines() {
                if line.contains("warning:") {
                    return Err(format!(
                        "{} emitted a warning on {} (generated C must be warning-free): {}",
                        resolved.name,
                        c_src.display(),
                        line
                    ));
                }
                log.warn(line);
            }
        }
        log.info(&format!("linking {}", bin_path.display()));
        let link = resolved
            .run(&[
                fsm_obj.to_string_lossy().into_owned(),
                test_obj.to_string_lossy().into_owned(),
                "-o".into(),
                bin_path.to_string_lossy().into_owned(),
            ])
            .map_err(|e| format!("failed to run {}: {e}", resolved.name))?;
        if !link.success() {
            eprintln!("{}", link.stderr);
            return Err(format!("{} link failed", resolved.name));
        }
    } else {
        // Phase 14 task 5: the Rust test artifact references the fsm file via
        // include! - write the fsm .rs alongside (under the standalone default
        // name, since `rfs test` does not stamp custom reference names) and
        // compile the TEST file; include! resolves relative to it.
        let fsm_path = dir.join("rfs_fsm.rs");
        std::fs::write(
            &fsm_path,
            out.fsm_code
                .as_deref()
                .expect("rust test fills the fsm-code slot"),
        )
        .map_err(|e| format!("failed to write {}: {e}", fsm_path.display()))?;

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
    Ok(())
}

/// Everything the fuzz pipeline needs: the resolved tool, the production
/// source (second registry dispatch) and the emitted skeleton extra file.
struct FuzzContext {
    /// clang (C backend) or cargo-fuzz (rust backend).
    tool: ResolvedTool,
    /// The FSM source the fuzz target links against. C: the fsm pair's source
    /// (Phase 14 task 4 - taken from the test codegen output, so the fuzz
    /// target drives the IDENTICAL fsm the unit tests do). Rust: the generated
    /// production source used as the cargo-fuzz lib root.
    prod_source: String,
    /// C fsm header (the source #includes it; both sources are compiled and
    /// linked). None for the Rust backend (single self-contained lib.rs).
    prod_header: Option<String>,
    /// The header file name the C source #includes (standalone default
    /// `rfs_fsm.h` - `rfs test` does not stamp custom reference names).
    header_name: String,
    /// The C fsm source file name (standalone default `rfs_fsm.c`).
    source_name: String,
    /// The emitted fuzz skeleton content (named slot of the test codegen).
    skeleton: String,
}

fn build_fuzz_context(
    opts: &TestOptions,
    backend: &Backend,
    ir: &crate::lang::ir::Ir,
    options: &BackendOptions,
    test_out: &crate::codegen::CodegenOutput,
) -> Result<FuzzContext, String> {
    let skeleton = test_out
        .fuzz_target
        .clone()
        .ok_or_else(|| "the test codegen emitted no fuzz skeleton (backend bug?)".to_string())?;
    // Task 3's resolver for the fuzz tool: CLI flag > env var > bare name.
    let tool = if backend.name == "rust" {
        ExtTool::CARGO_FUZZ.resolve(
            opts.cargo_fuzz_path.as_deref(),
            opts.cargo_fuzz_args.as_deref(),
        )
    } else {
        ExtTool::CLANG.resolve(opts.clang_path.as_deref(), opts.clang_args.as_deref())
    };
    if backend.name == "c" {
        // Phase 14 task 4: the C fuzz target references the SAME fsm pair the
        // test artifact uses (from the test codegen output) - no separate
        // production dispatch. The skeleton includes the header; both sources
        // are compiled together and linked.
        Ok(FuzzContext {
            tool,
            prod_source: test_out
                .fsm_code
                .clone()
                .expect("c test fills the fsm-code slot"),
            prod_header: test_out.fsm_header.clone(),
            header_name: options
                .fsm_header_name
                .clone()
                .unwrap_or_else(|| "rfs_fsm.h".into()),
            source_name: options
                .fsm_source_name
                .clone()
                .unwrap_or_else(|| "rfs_fsm.c".into()),
            skeleton,
        })
    } else {
        // Rust: the fuzz target is a separate bin crate whose lib root is the
        // generated production source (a second registry dispatch).
        let prod = CodegenCommandKind::Production
            .run(backend, ir, options)
            .map_err(|e| e.join("; "))?;
        Ok(FuzzContext {
            tool,
            prod_source: prod
                .fsm_code
                .clone()
                .expect("production fills the fsm-code slot"),
            prod_header: None,
            header_name: "rfs_fsm.h".into(), // unused for Rust
            source_name: "rfs_fsm.c".into(), // unused for Rust
            skeleton,
        })
    }
}

/// Fuzz suite (task 17): build the libFuzzer target from the emitted
/// skeleton and run it bounded (`-max_total_time`). Dispatches on backend.
fn run_fuzz_suite(
    opts: &TestOptions,
    ctx: &FuzzContext,
    dir: &Path,
    log: &Logger,
) -> Result<(), String> {
    if opts.backend == "rust" {
        run_fuzz_rust(opts, ctx, dir, log)
    } else {
        run_fuzz_c(opts, ctx, dir, log)
    }
}

/// C pipeline (Phase 14 task 4): the skeleton references the fsm through its
/// header, so the fsm source + header + skeleton are all written and BOTH
/// sources are compiled together (no more source-include, no multiple-
/// definition constraint).
fn run_fuzz_c(
    opts: &TestOptions,
    ctx: &FuzzContext,
    dir: &Path,
    log: &Logger,
) -> Result<(), String> {
    let skel_path = dir.join("fuzz_target.c");
    let fsm_path = dir.join(&ctx.source_name);
    std::fs::write(&skel_path, &ctx.skeleton)
        .map_err(|e| format!("failed to write {}: {e}", skel_path.display()))?;
    std::fs::write(&fsm_path, &ctx.prod_source)
        .map_err(|e| format!("failed to write {}: {e}", fsm_path.display()))?;
    // The C source #includes its own header - write it next to the source so
    // the include resolves (Rust has no header; skip).
    if let Some(header) = &ctx.prod_header {
        let header_path = dir.join(&ctx.header_name);
        std::fs::write(&header_path, header)
            .map_err(|e| format!("failed to write {}: {e}", header_path.display()))?;
    }

    let bin_path = dir.join("fuzz_bin");
    log.info(&format!("building fuzz target with {}", ctx.tool.path));
    // Extra args (RFS_CLANG_ARGS / --clang-args) sit right after the tool,
    // before its real arguments - the ext_tool contract shared by every
    // preset (task 3 invariant). BOTH sources are compiled: the skeleton no
    // longer source-includes the fsm, so there is no multiple-definition risk.
    let mut build_args = vec!["-O1".to_string(), "-fsanitize=fuzzer,address".to_string()];
    build_args.push("-o".into());
    build_args.push(bin_path.to_string_lossy().into_owned());
    build_args.push(fsm_path.to_string_lossy().into_owned());
    build_args.push(skel_path.to_string_lossy().into_owned());
    let build = ctx.tool.run(&build_args).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            format!(
                "clang not found at '{}' (required for --suite fuzz): install a libFuzzer-enabled clang per TOOLCHAIN.md or set RFS_CLANG",
                ctx.tool.path
            )
        } else {
            format!("failed to run {}: {e}", ctx.tool.display_name())
        }
    })?;
    if !build.success() {
        eprintln!("{}", build.stderr);
        return Err(format!("{} fuzz build failed", ctx.tool.display_name()));
    }
    for line in build.stderr.lines() {
        log.warn(line);
    }

    run_fuzz_binary(&bin_path, opts, log)
}

/// Rust pipeline: scaffold a temp cargo-fuzz project (the task 12 layout):
/// lib root = generated production source; the emitted skeleton goes to
/// fuzz/fuzz_targets/ with its `use crate::{` import adapted to the lib name
/// (the fuzz target is a separate bin crate).
fn run_fuzz_rust(
    opts: &TestOptions,
    ctx: &FuzzContext,
    dir: &Path,
    log: &Logger,
) -> Result<(), String> {
    let proj = dir.join("fuzz_proj");
    std::fs::create_dir_all(proj.join("src"))
        .map_err(|e| format!("failed to create {}: {e}", proj.display()))?;
    let fuzz_targets = proj.join("fuzz/fuzz_targets");
    std::fs::create_dir_all(&fuzz_targets)
        .map_err(|e| format!("failed to create {}: {e}", fuzz_targets.display()))?;

    std::fs::write(proj.join("Cargo.toml"), RUST_FUZZ_ROOT_MANIFEST)
        .map_err(|e| format!("failed to write root manifest: {e}"))?;
    // cargo-fuzz needs a NIGHTLY compiler (-Zsanitizer=address); scope it to
    // the temp project so the user's default toolchain is untouched.
    std::fs::write(proj.join("rust-toolchain.toml"), RUST_FUZZ_TOOLCHAIN)
        .map_err(|e| format!("failed to write rust-toolchain.toml: {e}"))?;
    std::fs::write(proj.join("src/lib.rs"), &ctx.prod_source)
        .map_err(|e| format!("failed to write lib.rs: {e}"))?;
    std::fs::write(proj.join("fuzz/Cargo.toml"), RUST_FUZZ_HARNESS_MANIFEST)
        .map_err(|e| format!("failed to write fuzz manifest: {e}"))?;

    let adapted = ctx.skeleton.replace("use crate::{", "use rfs_fuzz::{");
    if adapted == ctx.skeleton {
        return Err(
            "emitted skeleton lost its `use crate::{` import line - adaptation failed".into(),
        );
    }
    let skel_path = fuzz_targets.join("fuzz_target.rs");
    std::fs::write(&skel_path, adapted)
        .map_err(|e| format!("failed to write {}: {e}", skel_path.display()))?;

    // `cargo fuzz` resolves its project from the CWD - run from the scaffold.
    // A parent rustup may pin RUSTUP_TOOLCHAIN (e.g. stable under `cargo
    // test`); a non-nightly pin would override the scaffold's nightly pin,
    // so strip it (an explicit nightly pin is respected).
    let mut env_remove: Vec<&str> = Vec::new();
    if let Ok(tc) = std::env::var("RUSTUP_TOOLCHAIN") {
        if !tc.starts_with("nightly") {
            env_remove.push("RUSTUP_TOOLCHAIN");
        }
    }
    log.info(&format!("building fuzz target with {}", ctx.tool.path));
    let build = ctx
        .tool
        .run_in_without(&proj, &env_remove, &["build".to_string()])
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                cargo_fuzz_missing_diagnostic(&ctx.tool)
            } else {
                format!("failed to run {}: {e}", ctx.tool.display_name())
            }
        })?;
    if !build.success() {
        eprintln!("{}", build.stderr);
        return Err(cargo_fuzz_failure_diagnostic(&ctx.tool, &build.stderr));
    }

    // Bounded run through cargo-fuzz: everything after `--` reaches the
    // fuzzer binary. `-max_total_time` first, then the extra runtime flags.
    let mut run_args = vec![
        "run".to_string(),
        "fuzz_target".to_string(),
        "--".to_string(),
    ];
    run_args.push(format!("-max_total_time={}", opts.fuzz_max_total_time));
    run_args.extend(resolved_fuzz_flags(opts));
    log.info(&format!(
        "running fuzz target (bounded: {})",
        run_args.join(" ")
    ));
    let run = ctx
        .tool
        .run_in_without(&proj, &env_remove, &run_args)
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                cargo_fuzz_missing_diagnostic(&ctx.tool)
            } else {
                format!("failed to run {}: {e}", ctx.tool.display_name())
            }
        })?;
    if !run.success() {
        eprintln!("{}", run.stderr);
        if run.stderr.contains("no such command") {
            return Err(cargo_fuzz_missing_diagnostic(&ctx.tool));
        }
        return Err("fuzz target exited non-zero: libFuzzer found a crashing input".into());
    }
    Ok(())
}

/// cargo-fuzz is a subcommand of cargo: a missing install surfaces as cargo's
/// `no such command` error, not as a spawn failure.
fn cargo_fuzz_missing_diagnostic(tool: &ResolvedTool) -> String {
    format!(
        "cargo-fuzz not found (required for --suite fuzz on the rust backend): run `cargo install cargo-fuzz` or set RFS_CARGO_FUZZ to a cargo that has it (tried '{}')",
        tool.path
    )
}

fn cargo_fuzz_failure_diagnostic(tool: &ResolvedTool, stderr: &str) -> String {
    if stderr.contains("no such command") {
        cargo_fuzz_missing_diagnostic(tool)
    } else if stderr.contains("nightly option")
        || stderr.contains("toolchain 'nightly' is not installed")
    {
        format!(
            "cargo-fuzz requires a nightly toolchain (the scaffold pins channel=nightly): run `rustup toolchain install nightly` ({})",
            tool.display_name()
        )
    } else {
        format!("{} fuzz build failed", tool.display_name())
    }
}

/// Bounded libFuzzer run: `-max_total_time` first, then the extra runtime
/// flags (RFS_FUZZ_FLAGS env value, then --fuzz-flags). libFuzzer streams its
/// stats on stderr - inherit both so progress is visible.
fn run_fuzz_binary(bin: &Path, opts: &TestOptions, log: &Logger) -> Result<(), String> {
    let mut args = vec![format!("-max_total_time={}", opts.fuzz_max_total_time)];
    args.extend(resolved_fuzz_flags(opts));
    log.info(&format!(
        "running {} (bounded: {})",
        bin.display(),
        args.join(" ")
    ));
    let mut cmd = std::process::Command::new(bin);
    if opts.backend != "rust" {
        // The generated C FSM has NO free API (process-lifetime buffers, one
        // allocation set per fuzz iteration); ASan's exit-time leak report
        // would fail every run, so leak detection is off by default for the
        // C pipeline. In-flight memory errors are still caught; users can
        // re-enable via ASAN_OPTIONS (detect_leaks=1).
        cmd.env("ASAN_OPTIONS", c_asan_options());
    }
    let status = cmd
        .args(&args)
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()
        .map_err(|e| format!("failed to run {}: {e}", bin.display()))?;
    if !status.success() {
        return Err(format!(
            "fuzz target exited non-zero ({}): libFuzzer found a crashing input",
            bin.display()
        ));
    }
    Ok(())
}

/// Scaffolded cargo-fuzz project manifests (Phase 5 task 17). The
/// libfuzzer-sys version is pinned to the release present in the local
/// registry cache so the temp-project build works OFFLINE; it must not leak
/// into the rfs toolchain or generated code (the skeleton is its only
/// consumer).
const RUST_FUZZ_ROOT_MANIFEST: &str = "\
# Scaffolded by `rfs test --suite fuzz` (Phase 5 task 17). Do not edit.
[package]
name = \"rfs_fuzz\"
version = \"0.1.0\"
edition = \"2021\"

[lib]
path = \"src/lib.rs\"

[workspace]
";

/// cargo-fuzz builds with `-Zsanitizer=address` (nightly-only); pinning the
/// channel in the temp project keeps the user's default toolchain untouched.
const RUST_FUZZ_TOOLCHAIN: &str = "\
# Scaffolded by `rfs test --suite fuzz` (Phase 5 task 17). Do not edit.
[toolchain]
channel = \"nightly\"
";

const RUST_FUZZ_HARNESS_MANIFEST: &str = "\
# Scaffolded by `rfs test --suite fuzz` (Phase 5 task 17). Do not edit.
[package]
name = \"rfs_fuzz-fuzz\"
version = \"0.1.0\"
edition = \"2021\"
publish = false

# cargo-fuzz marker (required by `cargo fuzz` to recognize the harness).
[package.metadata]
cargo-fuzz = true

[workspace]

[dependencies]
libfuzzer-sys = \"=0.4.13\"
rfs_fuzz = { path = \"..\" }

[[bin]]
name = \"fuzz_target\"
path = \"fuzz_targets/fuzz_target.rs\"
harness = false
test = false
doc = false
";

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
/// performs exactly what execution mode would have done, minus the volatile
/// temp dir (CWD-relative names). Unit steps are task 7's byte-pinned four;
/// the fuzz steps (task 17) are appended when the fuzz suite is selected.
fn render_script(
    spec: &Path,
    opts: &TestOptions,
    resolved: &ResolvedTool,
    out: &crate::codegen::CodegenOutput,
    fuzz_ctx: Option<&FuzzContext>,
) -> String {
    // Deterministic command description (non-default options only).
    let mut desc = format!("rfs test --backend {}", opts.backend);
    if opts.suites != vec![TestSuite::Unit] {
        let names: Vec<&str> = opts
            .suites
            .iter()
            .map(|s| match s {
                TestSuite::Unit => "unit",
                TestSuite::Fuzz => "fuzz",
            })
            .collect();
        desc.push_str(&format!(" --suite {}", names.join(",")));
    }
    if opts.steps != 4 {
        desc.push_str(&format!(" --steps {}", opts.steps));
    }
    if fuzz_ctx.is_some() {
        if opts.fuzz_max_total_time != 30 {
            desc.push_str(&format!(
                " --fuzz-max-total-time {}",
                opts.fuzz_max_total_time
            ));
        }
        if let Some(f) = &opts.fuzz_flags {
            desc.push_str(&format!(" --fuzz-flags \"{f}\""));
        }
    }
    desc.push(' ');
    desc.push_str(&spec.display().to_string());

    let run_unit = opts.suites.contains(&TestSuite::Unit);
    let mut script = Script::new(&desc, &spec.display().to_string());
    if run_unit {
        script = script.with_tool(resolved.clone());
    }
    if let Some(ctx) = fuzz_ctx {
        script = script.with_tool(ctx.tool.clone());
        if opts.backend == "rust" {
            script = script.with_note(
                "requires cargo-fuzz and registry access for libfuzzer-sys (RFS_CARGO_FUZZ / RFS_CARGO_FUZZ_ARGS)",
            );
        } else {
            script =
                script.with_note("requires clang + libFuzzer runtime (RFS_CLANG / RFS_CLANG_ARGS)");
        }
    }

    if run_unit {
        let (src_name, bin_name, compile_args) = artifact_names(opts);
        // Heredoc with a quoted delimiter: no shell expansion of the content.
        let src_text = if out.primary(CodegenCommandKind::UnitTests).ends_with('\n') {
            out.primary(CodegenCommandKind::UnitTests).to_string()
        } else {
            format!("{}\n", out.primary(CodegenCommandKind::UnitTests))
        };
        let test_heredoc = format!(
            "cat > {} <<'__RFS_TEST_SOURCE__'\n{}__RFS_TEST_SOURCE__",
            sh_quote(src_name),
            src_text
        );

        if opts.backend == "c" {
            // Phase 14 task 4: the test source references the fsm header - the
            // script writes the fsm pair alongside and builds via object+link.
            let fsm_header_name = "rfs_fsm.h";
            let fsm_source_name = "rfs_fsm.c";
            let header_heredoc = heredoc(
                fsm_header_name,
                "__RFS_TEST_FSM_H__",
                out.fsm_header
                    .as_deref()
                    .expect("c test fills the fsm-header slot"),
            );
            let source_heredoc = heredoc(
                fsm_source_name,
                "__RFS_TEST_FSM_C__",
                out.fsm_code
                    .as_deref()
                    .expect("c test fills the fsm-code slot"),
            );
            script = script
                .step(
                    "write generated test artifact (from `rfs codegen c test`)",
                    &test_heredoc,
                )
                .step(
                    "write the referenced fsm header (co-emitted by the same command)",
                    &header_heredoc,
                )
                .step(
                    "write the referenced fsm source (co-emitted by the same command)",
                    &source_heredoc,
                )
                .step(
                    "compile the fsm source to an object",
                    &resolved.render(&[
                        "-Wall".into(),
                        "-c".into(),
                        fsm_source_name.to_string(),
                        "-I".into(),
                        ".".into(),
                        "-o".into(),
                        "rfs_fsm.o".into(),
                    ]),
                )
                .step(
                    "compile the test source to an object",
                    &resolved.render(&[
                        "-Wall".into(),
                        "-c".into(),
                        src_name.to_string(),
                        "-I".into(),
                        ".".into(),
                        "-o".into(),
                        "spec_test.o".into(),
                    ]),
                )
                .step(
                    "link the test binary",
                    &resolved.render(&[
                        "rfs_fsm.o".into(),
                        "spec_test.o".into(),
                        "-o".into(),
                        bin_name.to_string(),
                    ]),
                )
                .step("run the generated tests", &format!("./{bin_name}"))
                .step(
                    "clean up generated artifacts",
                    &format!(
                        "rm -f {src_name} {bin_name} rfs_fsm.h rfs_fsm.c rfs_fsm.o spec_test.o"
                    ),
                );
        } else {
            let mut args = compile_args;
            args.push(src_name.to_string());
            args.push("-o".into());
            args.push(bin_name.to_string());

            // Phase 14 task 5: the Rust test artifact references the fsm via
            // include! - the script writes the referenced fsm file alongside
            // (standalone default name, since `rfs test` does not stamp custom
            // reference names) so the include! resolves at compile time.
            let fsm_source_name = "rfs_fsm.rs";
            let fsm_heredoc = heredoc(
                fsm_source_name,
                "__RFS_TEST_FSM_RS__",
                out.fsm_code
                    .as_deref()
                    .expect("rust test fills the fsm-code slot"),
            );

            script = script
                .step(
                    "write generated test artifact (from `rfs codegen <backend> test`)",
                    &test_heredoc,
                )
                .step(
                    "write the referenced fsm file (co-emitted by the same command)",
                    &fsm_heredoc,
                )
                .step(
                    "compile the generated test artifact",
                    &resolved.render(&args),
                )
                .step("run the generated tests", &format!("./{bin_name}"))
                .step(
                    "clean up generated artifacts",
                    &format!("rm -f {src_name} {bin_name} rfs_fsm.rs"),
                );
        }
    }

    if let Some(ctx) = fuzz_ctx {
        script = render_fuzz_steps(script, opts, ctx);
    }
    script.render()
}

/// The fuzz-suite steps (task 17): C - heredocs for skeleton + production
/// source under the hardcoded include name, clang build, bounded run;
/// Rust - scaffold via mkdir/cat, cargo-fuzz build + bounded run.
fn render_fuzz_steps(script: Script, opts: &TestOptions, ctx: &FuzzContext) -> Script {
    let mut flags = vec![format!("-max_total_time={}", opts.fuzz_max_total_time)];
    flags.extend(resolved_fuzz_flags(opts));

    if opts.backend == "rust" {
        let adapted = ctx.skeleton.replace("use crate::{", "use rfs_fuzz::{");
        script
            .step(
                "scaffold the temp cargo-fuzz project",
                "mkdir -p fuzz_proj/src fuzz_proj/fuzz/fuzz_targets",
            )
            .step(
                "write the root crate manifest (lib = generated production source)",
                &heredoc(
                    "fuzz_proj/Cargo.toml",
                    "__RFS_FUZZ_ROOT_MANIFEST__",
                    RUST_FUZZ_ROOT_MANIFEST,
                ),
            )
            .step(
                "pin the nightly toolchain for the scaffold (cargo-fuzz needs -Zsanitizer)",
                &heredoc(
                    "fuzz_proj/rust-toolchain.toml",
                    "__RFS_FUZZ_TOOLCHAIN__",
                    RUST_FUZZ_TOOLCHAIN,
                ),
            )
            .step(
                "write the generated production source as the lib root",
                &heredoc(
                    "fuzz_proj/src/lib.rs",
                    "__RFS_FUZZ_LIB_RS__",
                    &ctx.prod_source,
                ),
            )
            .step(
                "write the cargo-fuzz harness manifest",
                &heredoc(
                    "fuzz_proj/fuzz/Cargo.toml",
                    "__RFS_FUZZ_HARNESS_MANIFEST__",
                    RUST_FUZZ_HARNESS_MANIFEST,
                ),
            )
            .step(
                "write the emitted fuzz skeleton (import adapted to the lib name)",
                &heredoc(
                    "fuzz_proj/fuzz/fuzz_targets/fuzz_target.rs",
                    "__RFS_FUZZ_TARGET_RS__",
                    &adapted,
                ),
            )
            .step("enter the scaffolded project", "cd fuzz_proj")
            .step(
                "build the Rust fuzz target",
                &ctx.tool.render(&["build".to_string()]),
            )
            .step(
                "run the Rust fuzz target bounded",
                &ctx.tool.render(&{
                    let mut a = vec![
                        "run".to_string(),
                        "fuzz_target".to_string(),
                        "--".to_string(),
                    ];
                    a.extend(flags);
                    a
                }),
            )
            .step("leave the scaffolded project", "cd ..")
            .step("clean up generated artifacts", "rm -rf fuzz_proj")
    } else {
        // Phase 14 task 4: BOTH sources are compiled (the skeleton references
        // the fsm through its header, so there is no multiple-definition risk).
        let mut build_args = vec!["-O1".to_string(), "-fsanitize=fuzzer,address".to_string()];
        build_args.push("-o".into());
        build_args.push("fuzz_bin".into());
        build_args.push(ctx.source_name.clone());
        build_args.push("fuzz_target.c".into());
        script
            .step(
                "write the emitted fuzz skeleton (from `rfs codegen c test`)",
                &heredoc("fuzz_target.c", "__RFS_FUZZ_TARGET_C__", &ctx.skeleton),
            )
            .step(
                "write the referenced fsm source (co-emitted by the same command)",
                &heredoc(&ctx.source_name, "__RFS_FUZZ_FSM_C__", &ctx.prod_source),
            )
            // The C source #includes its own header - write it so the include
            // resolves (the Rust path has no header).
            .step(
                "write the referenced fsm header (the source's #include target)",
                &heredoc(&ctx.header_name, "__RFS_FUZZ_FSM_H__", ctx.prod_header.as_deref().unwrap_or("")),
            )
            .step(
                "build the C fuzz target (both sources)",
                &ctx.tool.render(&build_args),
            )
            .step(
                "run the C fuzz target bounded (leak detection off - no free API in the generated FSM)",
                &format!(
                    "ASAN_OPTIONS=\"detect_leaks=0${{ASAN_OPTIONS:+:$ASAN_OPTIONS}}\" ./fuzz_bin {}",
                    flags.join(" ")
                ),
            )
            .step(
                "clean up generated artifacts",
                &format!("rm -f fuzz_target.c {} {} fuzz_bin", ctx.source_name, ctx.header_name),
            )
    }
}

/// ASAN_OPTIONS for the C fuzz run: leak detection off (see run_fuzz_binary),
/// preserving any user-set options after ours (first occurrence of a key
/// wins in sanitizer option parsing).
fn c_asan_options() -> String {
    match std::env::var("ASAN_OPTIONS") {
        Ok(existing) if !existing.is_empty() && !existing.contains("detect_leaks=") => {
            format!("detect_leaks=0:{existing}")
        }
        _ => "detect_leaks=0".to_string(),
    }
}

/// Quoted-delimiter heredoc writing `content` to `target` (no shell
/// expansion; the content is newline-terminated before the delimiter).
fn heredoc(target: &str, delimiter: &str, content: &str) -> String {
    let body = if content.ends_with('\n') {
        content.to_string()
    } else {
        format!("{content}\n")
    };
    format!(
        "cat > {} <<'{delimiter}'\n{body}{delimiter}",
        sh_quote(target)
    )
}

pub fn run_cli(args: &[String]) -> ExitCode {
    use crate::cli::log::{LogLevel, Logger};
    let mut opts = TestOptions::default();
    let mut log_level = LogLevel::Info;
    let mut path: Option<String> = None;
    // The first --suite replaces the default [Unit]; later ones append.
    let mut suite_given = false;

    let mut i = 0;
    while i < args.len() {
        let arg = args[i].clone();
        if arg == "--help" || arg == "-h" {
            println!("{}", usage());
            return ExitCode::SUCCESS;
        }
        match arg.as_str() {
            "--" => {}
            "--backend"
            | "--steps"
            | "--rustc"
            | "--gcc"
            | "--rustc-args"
            | "--gcc-args"
            | "--clang"
            | "--clang-args"
            | "--cargo-fuzz"
            | "--cargo-fuzz-args"
            | "--suite"
            | "--fuzz-max-total-time"
            | "--fuzz-flags"
            | "--log-level" => {
                let Some(v) = args.get(i + 1).cloned() else {
                    eprintln!("error: {} requires a value\n{}", arg, usage());
                    return ExitCode::FAILURE;
                };
                i += 1;
                match arg.as_str() {
                    "--backend" => opts.backend = v,
                    "--steps" => match v.parse::<usize>() {
                        Ok(n) => opts.steps = n,
                        Err(_) => {
                            eprintln!(
                                "error: --steps expects a non-negative integer (got '{}')\n{}",
                                v,
                                usage()
                            );
                            return ExitCode::FAILURE;
                        }
                    },
                    "--rustc" => opts.rustc_path = Some(v),
                    "--gcc" => opts.gcc_path = Some(v),
                    "--rustc-args" => opts.rustc_args = Some(v),
                    "--gcc-args" => opts.gcc_args = Some(v),
                    "--clang" => opts.clang_path = Some(v),
                    "--clang-args" => opts.clang_args = Some(v),
                    "--cargo-fuzz" => opts.cargo_fuzz_path = Some(v),
                    "--cargo-fuzz-args" => opts.cargo_fuzz_args = Some(v),
                    "--suite" => {
                        // Repeatable or comma-separated; order-preserving dedupe.
                        if !suite_given {
                            opts.suites.clear();
                            suite_given = true;
                        }
                        for part in v.split(',') {
                            let part = part.trim();
                            match TestSuite::from_str(part) {
                                Some(s) => {
                                    if !opts.suites.contains(&s) {
                                        opts.suites.push(s);
                                    }
                                }
                                None => {
                                    eprintln!(
                                        "error: --suite expects unit|fuzz (got '{}')\n{}",
                                        part,
                                        usage()
                                    );
                                    return ExitCode::FAILURE;
                                }
                            }
                        }
                    }
                    "--fuzz-max-total-time" => match v.parse::<u64>() {
                        Ok(n) if n > 0 => opts.fuzz_max_total_time = n,
                        _ => {
                            eprintln!(
                                "error: --fuzz-max-total-time expects a positive integer (got '{}')\n{}",
                                v,
                                usage()
                            );
                            return ExitCode::FAILURE;
                        }
                    },
                    "--fuzz-flags" => opts.fuzz_flags = Some(v),
                    "--log-level" => match LogLevel::parse(&v) {
                        Some(l) => log_level = l,
                        None => {
                            eprintln!(
                                "error: --log-level expects off|error|warn|info|debug (got '{}')\n{}",
                                v,
                                usage()
                            );
                            return ExitCode::FAILURE;
                        }
                    },
                    _ => unreachable!("flag checked by caller: {arg}"),
                }
            }
            "--print-script" => opts.print_script = true,
            _ if arg.starts_with('-') => {
                eprintln!("unknown option: {}\n{}", arg, usage());
                return ExitCode::FAILURE;
            }
            _ => {
                if path.is_some() {
                    eprintln!("unexpected extra argument: {}\n{}", arg, usage());
                    return ExitCode::FAILURE;
                }
                path = Some(arg);
            }
        }
        i += 1;
    }

    let Some(path) = path else {
        eprintln!("error: rfs test requires a spec file\n{}", usage());
        return ExitCode::FAILURE;
    };

    // Fatal diagnostics bypass the logger (always stderr, drive exit code);
    // the logger carries informational progress output only.
    let logger = Logger::new(log_level);
    match run_test(Path::new(&path), &opts, &logger) {
        Ok(result) => {
            if let Some(script) = result.script {
                // `print!` (not println): the script already ends with a
                // newline and callers may redirect stdout to a file - the
                // bytes must be reproduced exactly (codegen convention).
                print!("{script}");
            } else {
                logger.info(&result.note);
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
