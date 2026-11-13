//! `rfs codegen` command plugin (moved from `main.rs` `cmd_codegen`). Thin CLI
//! wrapper over the target-language backend registry in `crate::codegen`.
//! Docs: dev:cli-commands-overview

use std::path::Path;
use std::process::ExitCode;

use crate::cli::utils::{convert_file_to_ir, Format};
use crate::codegen;

/// Output naming arguments parsed from the CLI (Phase 14 task 1): the BASE
/// name (`--out`, or None for the spec-stem default), the per-artifact output
/// overrides, and the fsm-reference flag forms (mc/test only).
pub struct OutputArgs {
    /// The `--out` value: a BASE name, or "-" for stdout mode.
    pub base: Option<String>,
    pub out_fsm_code: Option<String>,
    pub out_fsm_header: Option<String>,
    pub out_mc_code: Option<String>,
    pub out_mc_header: Option<String>,
    pub out_test_code: Option<String>,
    pub out_fuzz_target: Option<String>,
    /// Rust `NAME[.rs]` / C form-1 `STEM`.
    pub fsm_file: Option<String>,
    /// C form 2 (both or neither - parser-enforced).
    pub fsm_file_header: Option<String>,
    pub fsm_file_source: Option<String>,
}

/// The CLI write loop (Phase 14 task 1): resolves every filled artifact slot
/// to its output path and writes it, applying take-or-emit to the fsm slots
/// referenced by mc/test.
///
/// Naming resolution (D14-3): BASE := the `--out` value when given, otherwise
/// the SPEC FILE STEM (file name without extension, no directory component);
/// default names derive from BASE per `ArtifactSlot::default_name`; explicit
/// per-artifact flags override the corresponding default. The fsm reference
/// (D14-2): explicit fsm-reference flags win over the BASE-derived name; C
/// form 1 expands STEM to `STEM.h` + `STEM.c`; Rust appends `.rs` when the
/// extension is omitted. A referenced fsm artifact present on disk is TAKEN
/// (never overwritten - a partial C pair is a usage error); absent, it is
/// EMITTED from the generated slots. A taken artifact whose line-1 header spec
/// hash differs from the current spec's hash warns (D14-4) but is still used.
/// Stdout mode (`--out -`) prints the single filled slot and is refused when
/// more than one slot would be written.
pub fn write_artifacts(
    backend_name: &str,
    command: codegen::CodegenCommandKind,
    input: &str,
    args: &OutputArgs,
    result: &codegen::CodegenOutput,
    spec_hash: &str,
) -> ExitCode {
    let is_c = backend_name == "c";
    let base = args.base.as_deref();

    // BASE := --out value | spec file stem (no directory component).
    let default_stem = Path::new(input)
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "spec".into());

    // Stdout mode: exactly one filled slot, nothing else. The refusal
    // diagnostic names the spec-stem files a bare invocation would have
    // written, so the user sees what stdout would have swallowed.
    if base == Some("-") {
        let filled = result.filled_slots();
        if filled.len() != 1 {
            let names = filled
                .iter()
                .map(|(s, _)| s.default_name(&default_stem, is_c))
                .collect::<Vec<_>>()
                .join(", ");
            eprintln!(
                "error: this command emits {} artifacts ({names}) that cannot all be written to stdout; use --out BASE or per-artifact flags",
                filled.len()
            );
            return ExitCode::FAILURE;
        }
        print!("{}", filled[0].1);
        return ExitCode::SUCCESS;
    }

    let base_name = base.map(|s| s.to_string()).unwrap_or(default_stem);

    // The slots this command OWNS (written fresh, overwritten) vs the fsm
    // slots it REFERENCES (take-or-emit).
    let owns_fsm = matches!(command, codegen::CodegenCommandKind::Production);

    // --- Own artifacts: explicit flag > BASE-derived default, written fresh. ---
    let own: Vec<(codegen::ArtifactSlot, Option<&str>)> = match command {
        codegen::CodegenCommandKind::Production => vec![
            (
                codegen::ArtifactSlot::FsmHeader,
                args.out_fsm_header.as_deref(),
            ),
            (codegen::ArtifactSlot::FsmCode, args.out_fsm_code.as_deref()),
        ],
        codegen::CodegenCommandKind::ModelChecker => vec![
            (
                codegen::ArtifactSlot::McHeader,
                args.out_mc_header.as_deref(),
            ),
            (codegen::ArtifactSlot::McCode, args.out_mc_code.as_deref()),
        ],
        codegen::CodegenCommandKind::UnitTests => vec![
            (
                codegen::ArtifactSlot::TestCode,
                args.out_test_code.as_deref(),
            ),
            (
                codegen::ArtifactSlot::FuzzTarget,
                args.out_fuzz_target.as_deref(),
            ),
        ],
    };
    for (slot, override_name) in own {
        let content = match result.slot_content(slot) {
            Some(c) => c,
            None => continue, // slot not filled by this backend/command
        };
        let name = override_name
            .map(|s| s.to_string())
            .unwrap_or_else(|| slot.default_name(&base_name, is_c));
        if !write_fresh(&name, content) {
            return ExitCode::FAILURE;
        }
    }

    // --- Referenced fsm artifact (mc/test): take-or-emit. ---
    if !owns_fsm {
        // Resolve the reference: form-2 pair > form-1/file stem > BASE-derived.
        let (header_path, source_path): (Option<String>, String) = if is_c {
            match (&args.fsm_file_header, &args.fsm_file_source, &args.fsm_file) {
                (Some(h), Some(s), None) => (Some(h.clone()), s.clone()),
                (_, _, Some(stem)) => (Some(format!("{stem}.h")), format!("{stem}.c")),
                _ => (
                    Some(format!("{base_name}-fsm.h")),
                    format!("{base_name}-fsm.c"),
                ),
            }
        } else {
            let name = match &args.fsm_file {
                Some(f) if f.ends_with(".rs") => f.clone(),
                Some(f) => format!("{f}.rs"),
                None => format!("{base_name}-fsm.rs"),
            };
            (None, name)
        };

        // Presence check: a partial C pair is a usage error (no silent
        // overwrite of the lone file).
        let source_present = Path::new(&source_path).is_file();
        let header_present = header_path
            .as_deref()
            .map(|p| Path::new(p).is_file())
            .unwrap_or(false);
        if is_c && source_present != header_present {
            eprintln!(
                "error: the referenced fsm pair is partial - only one of {:?} / {} exists; a lone file is never overwritten, generate the pair or provide both",
                header_path.as_deref().unwrap_or(&source_path),
                source_path
            );
            return ExitCode::FAILURE;
        }
        let taken = source_present && (!is_c || header_present);

        if taken {
            // D14-4: a taken artifact whose line-1 spec hash differs from the
            // current spec's hash is stale - warn, still reference it.
            if let Some(stale) = check_stale(&source_path, spec_hash) {
                eprintln!("warning: {stale}");
            }
        } else {
            // Emit the generated fsm slots (when this backend fills them).
            for slot in [
                codegen::ArtifactSlot::FsmHeader,
                codegen::ArtifactSlot::FsmCode,
            ] {
                let Some(content) = result.slot_content(slot) else {
                    continue;
                };
                let path = match slot {
                    codegen::ArtifactSlot::FsmHeader => header_path.clone().unwrap_or_default(),
                    _ => source_path.clone(),
                };
                if !write_fresh(&path, content) {
                    return ExitCode::FAILURE;
                }
            }
        }
    }

    ExitCode::SUCCESS
}

/// Write `content` to `path` (creating parent dirs), printing the conventional
/// `wrote <path>` line. Returns false on I/O failure.
fn write_fresh(path: &str, content: &str) -> bool {
    if let Some(parent) = Path::new(path).parent() {
        if !parent.as_os_str().is_empty() && std::fs::create_dir_all(parent).is_err() {
            return false;
        }
    }
    if std::fs::write(path, content).is_err() {
        return false;
    }
    println!("wrote {}", path);
    true
}

/// Parse the line-1 artifact header of a TAKEN fsm file and compare its spec
/// hash with the current spec's (D14-4). Returns a warning message on
/// mismatch; None when the hashes agree or the header is not in the documented
/// form (a hand-written file - nothing to compare).
fn check_stale(path: &str, current_hash: &str) -> Option<String> {
    let content = std::fs::read_to_string(path).ok()?;
    let line1 = content.lines().next()?;
    let start = line1.find("spec sha256:")? + "spec sha256:".len();
    let rest = &line1[start..];
    let end = rest.find('|')?;
    let taken_hash = rest[..end].trim();
    if taken_hash.is_empty() || taken_hash == current_hash {
        return None;
    }
    Some(format!(
        "referenced fsm file {} was generated from a different spec (its header says sha256:{taken_hash}, this spec is sha256:{current_hash}) - it is still used as referenced",
        path
    ))
}

pub fn run_cli(args: &[String]) -> ExitCode {
    match codegen::parse_codegen_args(args) {
        Err(errors) => {
            for e in &errors {
                eprintln!("error: {}", e);
            }
            ExitCode::FAILURE
        }
        Ok(codegen::CodegenRequest::List) => {
            println!("Available codegen backends:");
            for (name, desc) in codegen::list_backends() {
                println!("  {:<10} {}", name, desc);
            }
            ExitCode::SUCCESS
        }
        Ok(codegen::CodegenRequest::Help) => {
            println!("{}", codegen::common_help());
            ExitCode::SUCCESS
        }
        Ok(codegen::CodegenRequest::BackendHelp(name)) => match codegen::get_backend(&name) {
            Some(b) => {
                println!("{}", codegen::backend_help(b));
                ExitCode::SUCCESS
            }
            None => {
                eprintln!(
                    "error: unknown backend '{}'; see `rfs codegen --list`",
                    name
                );
                ExitCode::FAILURE
            }
        },
        Ok(codegen::CodegenRequest::Generate {
            backend_name,
            command,
            input,
            base,
            out_fsm_code,
            out_fsm_header,
            out_mc_code,
            out_mc_header,
            out_test_code,
            out_fuzz_target,
            fsm_file,
            fsm_file_header,
            fsm_file_source,
            assumptions,
            steps,
            no_std,
            with_property_monitors,
            mc_mode,
            mc_max_states,
            mc_nondet,
            mc_timeout_sec,
            mc_threads,
            mc_smt_bound,
            counterexample_format,
            with_protocol_fsm,
        }) => {
            let Some(backend) = codegen::get_backend(&backend_name) else {
                eprintln!(
                    "error: unknown backend '{}'; see `rfs codegen --list`",
                    backend_name
                );
                return ExitCode::FAILURE;
            };
            let ext = Path::new(&input)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("");
            let Some(fmt) = Format::from_extension(ext) else {
                eprintln!("error: input '{}' must have extension .rfs or .json", input);
                return ExitCode::FAILURE;
            };
            let ir = match convert_file_to_ir(Path::new(&input), fmt) {
                Ok(ir) => ir,
                Err(e) => {
                    eprintln!("error: {}", e);
                    return ExitCode::FAILURE;
                }
            };
            if ir.modules.is_empty() {
                eprintln!("error: no modules in {}", input);
                return ExitCode::FAILURE;
            }
            // The v1.0 spec hash (D-P6-3) of the whole-file canonical IR JSON -
            // stamped by `run` into every artifact AND used here for the
            // take-or-emit stale-reuse warning (Phase 14, D14-4).
            let spec_hash = match ir.to_json() {
                Ok(json) => crate::utils::hash::spec_hash16(&json.to_string()),
                Err(e) => {
                    eprintln!("error: {}", e);
                    return ExitCode::FAILURE;
                }
            };
            // Phase 14 task 2: stamp the resolved C fsm-header/source names so
            // the generated source #includes its own header by the correct
            // name and the include guard matches. Only C production emits a
            // pair; everything else stays None (standalone default).
            let is_c_prod =
                backend_name == "c" && command == codegen::CodegenCommandKind::Production;
            let base_name = base.clone().unwrap_or_else(|| {
                Path::new(&input)
                    .file_stem()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "spec".into())
            });
            let fsm_header_name = if is_c_prod {
                Some(
                    out_fsm_header
                        .clone()
                        .unwrap_or_else(|| format!("{base_name}-fsm.h")),
                )
            } else {
                None
            };
            let fsm_source_name = if is_c_prod {
                Some(
                    out_fsm_code
                        .clone()
                        .unwrap_or_else(|| format!("{base_name}-fsm.c")),
                )
            } else {
                None
            };
            let options = codegen::BackendOptions {
                assumptions,
                steps,
                no_std,
                with_property_monitors,
                mc_mode,
                mc_max_states,
                mc_nondet,
                mc_timeout_sec,
                mc_threads,
                mc_smt_bound,
                counterexample_format,
                with_protocol_fsm,
                spec_hash: String::new(),
                fsm_header_name,
                fsm_source_name,
            };
            match command.run(backend, &ir, &options) {
                Err(errors) => {
                    for e in &errors {
                        eprintln!("codegen error: {}", e);
                    }
                    ExitCode::FAILURE
                }
                Ok(result) => {
                    for w in &result.warnings {
                        eprintln!("warning: {}", w);
                    }
                    write_artifacts(
                        &backend_name,
                        command,
                        &input,
                        &OutputArgs {
                            base,
                            out_fsm_code,
                            out_fsm_header,
                            out_mc_code,
                            out_mc_header,
                            out_test_code,
                            out_fuzz_target,
                            fsm_file,
                            fsm_file_header,
                            fsm_file_source,
                        },
                        &result,
                        &spec_hash,
                    )
                }
            }
        }
    }
}
