//! `rfs validate` command plugin: the CLI validation logic (moved from the old
//! `crate::cli::frontend` validation half) + the `run_cli` entry point.
//!
//! Core semantic validation lives in `crate::lang::validator` (untouched); this
//! module is the command-line layer around it - parse + validate files, write a
//! `.rfs-validation.err` next to each failing file in directory mode, and render
//! structured JSON diagnostics for editor integrations. The conversion/format
//! helpers it relies on (`Format`, `convert_file_to_ir`, `collect_files_with_ext`)
//! are in `crate::cli::utils`.
//! Docs: dev:cli-commands-frontend

use crate::cli::utils::{collect_files_with_ext, convert_file_to_ir, Format};
use crate::lang::ir::Ir;
use crate::lang::validator;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use crate::cli::args::{take_format, usage};

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

/// One file that failed validation.
#[derive(Debug)]
pub struct FailedFile {
    pub path: PathBuf,
    /// Parse or semantic error messages.
    pub errors: Vec<String>,
    /// The `.rfs-validation.err` file written for this failure (directory
    /// mode only; `None` in file mode or if writing the err file failed).
    pub err_file: Option<PathBuf>,
}

/// Result of a validation run.
#[derive(Debug)]
pub struct ValidateResult {
    pub ok_files: Vec<PathBuf>,
    /// Warnings per validated file (Phase 2 non-fatal diagnostics); files
    /// that failed validation have no warnings.
    pub warnings: Vec<(PathBuf, Vec<String>)>,
    /// Failing files with their error messages (parse or semantic).
    pub failed: Vec<FailedFile>,
    /// IO errors from writing `.rfs-validation.err` files.
    pub write_errors: Vec<String>,
    /// True when the input was a directory (drives `.rfs-validation.err` output).
    pub is_directory: bool,
}

/// Parse + validate one file. Returns semantic/parse error messages on failure.
pub fn validate_one(path: &Path, format: Format) -> Result<(), Vec<String>> {
    let r = validate_one_with_warnings(path, format)?;
    if r.errors.is_empty() {
        Ok(())
    } else {
        Err(r.errors)
    }
}

/// Parse + validate one file, returning fatal errors and non-fatal warnings.
pub fn validate_one_with_warnings(
    path: &Path,
    format: Format,
) -> Result<ValidateOneResult, Vec<String>> {
    let ir = convert_file_to_ir(path, format).map_err(|e| vec![e])?;
    match validator::validate_ir(&ir) {
        Ok(()) => {
            let mut warnings = Vec::new();
            for m in &ir.modules {
                warnings.extend(validator::module_warnings(m));
            }
            Ok(ValidateOneResult {
                errors: Vec::new(),
                warnings,
            })
        }
        Err(errors) => Ok(ValidateOneResult {
            errors: errors.into_iter().map(|e| e.message).collect(),
            warnings: Vec::new(),
        }),
    }
}

/// Errors (fatal) and warnings (non-fatal) for one validated file.
#[derive(Debug)]
pub struct ValidateOneResult {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

/// A structured diagnostic for `rfs validate --json` (Phase 5 task 13) - the
/// shape editor integrations consume.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructuredError {
    /// "parse" or "semantic".
    pub kind: &'static str,
    /// 1-based line of the error; 0 when unknown (semantic errors carry no
    /// span - the validator reports messages only).
    pub line: usize,
    /// 1-based column of the error; 0 when unknown.
    pub col: usize,
    pub message: String,
}

/// Parse + validate one file with STRUCTURED diagnostics (`validate --json`).
/// Ok holds the structured errors (empty on success); Err holds CLI-level
/// misuse messages (bad extension, IO) - same contract as the text path.
pub fn validate_one_structured(
    path: &Path,
    format: Format,
) -> Result<Vec<StructuredError>, Vec<String>> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| vec![format!("failed to read {}: {}", path.display(), e)])?;
    let ir = match format {
        Format::Rfs => match crate::parse_source(&text) {
            Ok(ir) => ir,
            Err(e) => {
                let (line, col) = e.line_col();
                return Ok(vec![StructuredError {
                    kind: "parse",
                    line,
                    col,
                    message: e.message,
                }]);
            }
        },
        Format::Json => {
            let value = crate::json::json_rw::parse(&text)
                .map_err(|e| vec![format!("invalid RFS IR JSON in {}: {}", path.display(), e)])?;
            Ir::from_json(&value)
                .map_err(|e| vec![format!("invalid RFS IR JSON in {}: {}", path.display(), e)])?
        }
    };
    match validator::validate_ir(&ir) {
        Ok(()) => Ok(Vec::new()),
        Err(errors) => Ok(errors
            .into_iter()
            .map(|e| StructuredError {
                kind: "semantic",
                line: 0,
                col: 0,
                message: e.message,
            })
            .collect()),
    }
}

/// One `validate --json` document (pinned shape - Phase 5 task 13):
/// `{ "file": ..., "ok": ..., "errors": [ {kind,line,col,message} ] }`.
fn validate_json_doc(path: &Path, errors: &[StructuredError]) -> crate::json::json_rw::Json {
    crate::json::json_rw::Json::obj(vec![
        (
            "file".to_string(),
            crate::json::json_rw::Json::str(path.display().to_string()),
        ),
        (
            "ok".to_string(),
            crate::json::json_rw::Json::bool(errors.is_empty()),
        ),
        (
            "errors".to_string(),
            crate::json::json_rw::Json::arr(
                errors
                    .iter()
                    .map(|e| {
                        crate::json::json_rw::Json::obj(vec![
                            ("kind".to_string(), crate::json::json_rw::Json::str(e.kind)),
                            (
                                "line".to_string(),
                                crate::json::json_rw::Json::int(e.line as i64),
                            ),
                            (
                                "col".to_string(),
                                crate::json::json_rw::Json::int(e.col as i64),
                            ),
                            (
                                "message".to_string(),
                                crate::json::json_rw::Json::str(&e.message),
                            ),
                        ])
                    })
                    .collect(),
            ),
        ),
    ])
}

/// Run STRUCTURED validation over a file or directory (`validate --json`).
/// Mirrors `run_validate`'s traversal/extension contract (file: extension
/// selects the format, `--input` must agree; directory: `--input` required,
/// files collected by extension) but reports only FAILING files with
/// structured errors and writes no `.rfs-validation.err` side files. Err
/// holds CLI-level misuse messages.
pub fn run_validate_structured(
    input_format: Option<Format>,
    path: &Path,
) -> Result<Vec<(PathBuf, Vec<StructuredError>)>, Vec<String>> {
    if path.is_file() {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        let fmt = Format::from_extension(ext).ok_or_else(|| {
            vec![format!(
                "file '{}' must have extension .rfs or .json",
                path.display()
            )]
        })?;
        if let Some(req) = input_format {
            if req != fmt {
                return Err(vec![format!(
                    "--input {} conflicts with file extension '.{}' for '{}'",
                    req.name(),
                    fmt.ext(),
                    path.display()
                )]);
            }
        }
        let errors = validate_one_structured(path, fmt)?;
        if errors.is_empty() {
            Ok(Vec::new())
        } else {
            Ok(vec![(path.to_path_buf(), errors)])
        }
    } else if path.is_dir() {
        let fmt = input_format.ok_or_else(|| {
            vec![format!(
                "directory input requires --input rfs or --input json (got directory '{}')",
                path.display()
            )]
        })?;
        let files = collect_files_with_ext(path, fmt.ext());
        if files.is_empty() {
            return Err(vec![format!(
                "no .{} files found under '{}'",
                fmt.ext(),
                path.display()
            )]);
        }
        let mut failed = Vec::new();
        for f in files {
            match validate_one_structured(&f, fmt) {
                Ok(errors) if !errors.is_empty() => failed.push((f, errors)),
                Ok(_) => {}
                Err(errors) => failed.push((
                    f,
                    errors
                        .into_iter()
                        .map(|m| StructuredError {
                            kind: "semantic",
                            line: 0,
                            col: 0,
                            message: m,
                        })
                        .collect(),
                )),
            }
        }
        Ok(failed)
    } else {
        Err(vec![format!(
            "input path '{}' does not exist or is not a file or directory",
            path.display()
        )])
    }
}

/// Render the `validate --json` stdout: one document for the input file
/// (file mode - `ok: true` with empty errors on success) or an array of one
/// document per FAILING file (directory mode). Pretty-printed like
/// `rfs info --json`.
pub fn render_validate_json(
    input_path: &Path,
    is_directory: bool,
    failed: &[(PathBuf, Vec<StructuredError>)],
) -> String {
    let doc = if is_directory {
        crate::json::json_rw::Json::arr(
            failed
                .iter()
                .map(|(p, e)| validate_json_doc(p, e))
                .collect(),
        )
    } else {
        let errors = failed.first().map(|(_, e)| e.as_slice()).unwrap_or(&[]);
        validate_json_doc(input_path, errors)
    };
    doc.to_string_pretty() + "\n"
}

/// Run validation over a file or directory. `Err` holds CLI-level misuse
/// messages (bad extension, missing --input for directories, ...); `Ok` holds
/// per-file results.
pub fn run_validate(
    input_format: Option<Format>,
    path: &Path,
) -> Result<ValidateResult, Vec<String>> {
    if path.is_file() {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        let fmt = Format::from_extension(ext).ok_or_else(|| {
            vec![format!(
                "file '{}' must have extension .rfs or .json",
                path.display()
            )]
        })?;
        if let Some(req) = input_format {
            if req != fmt {
                return Err(vec![format!(
                    "--input {} conflicts with file extension '.{}' for '{}'",
                    req.name(),
                    fmt.ext(),
                    path.display()
                )]);
            }
        }
        match validate_one_with_warnings(path, fmt) {
            Ok(r) => {
                let warnings = if r.warnings.is_empty() {
                    Vec::new()
                } else {
                    vec![(path.to_path_buf(), r.warnings)]
                };
                if r.errors.is_empty() {
                    Ok(ValidateResult {
                        ok_files: vec![path.to_path_buf()],
                        warnings,
                        failed: Vec::new(),
                        write_errors: Vec::new(),
                        is_directory: false,
                    })
                } else {
                    Ok(ValidateResult {
                        ok_files: Vec::new(),
                        warnings: Vec::new(),
                        failed: vec![FailedFile {
                            path: path.to_path_buf(),
                            errors: r.errors,
                            err_file: None,
                        }],
                        write_errors: Vec::new(),
                        is_directory: false,
                    })
                }
            }
            Err(errors) => Ok(ValidateResult {
                ok_files: Vec::new(),
                warnings: Vec::new(),
                failed: vec![FailedFile {
                    path: path.to_path_buf(),
                    errors,
                    err_file: None,
                }],
                write_errors: Vec::new(),
                is_directory: false,
            }),
        }
    } else if path.is_dir() {
        let fmt = input_format.ok_or_else(|| {
            vec![format!(
                "directory input requires --input rfs or --input json (got directory '{}')",
                path.display()
            )]
        })?;
        let files = collect_files_with_ext(path, fmt.ext());
        if files.is_empty() {
            return Err(vec![format!(
                "no .{} files found under '{}'",
                fmt.ext(),
                path.display()
            )]);
        }
        let mut ok_files = Vec::new();
        let mut failed = Vec::new();
        let mut warnings: Vec<(PathBuf, Vec<String>)> = Vec::new();
        for f in files {
            match validate_one_with_warnings(&f, fmt) {
                Ok(r) if r.errors.is_empty() => {
                    ok_files.push(f.clone());
                    if !r.warnings.is_empty() {
                        warnings.push((f, r.warnings));
                    }
                }
                Ok(r) => failed.push(FailedFile {
                    path: f,
                    errors: r.errors,
                    err_file: None,
                }),
                Err(errors) => failed.push(FailedFile {
                    path: f,
                    errors,
                    err_file: None,
                }),
            }
        }
        // Write a `.rfs-validation.err` file next to each failing source.
        let mut write_errors = Vec::new();
        for ff in &mut failed {
            match write_err_file(&ff.path, &ff.errors) {
                Ok(p) => ff.err_file = Some(p),
                Err(e) => write_errors.push(format!(
                    "failed to write validation error file for {}: {}",
                    ff.path.display(),
                    e
                )),
            }
        }
        Ok(ValidateResult {
            ok_files,
            warnings,
            failed,
            write_errors,
            is_directory: true,
        })
    } else {
        Err(vec![format!(
            "input path '{}' does not exist or is not a file or directory",
            path.display()
        )])
    }
}

/// The `.rfs-validation.err` path for a failing source file: the full source
/// file name with `.rfs-validation.err` appended, next to the source.
pub fn err_file_path(source: &Path) -> PathBuf {
    let name = source.file_name().unwrap_or_default();
    let mut suffix = std::ffi::OsString::from(name);
    suffix.push(".rfs-validation.err");
    match source.parent() {
        Some(p) => p.join(suffix),
        None => PathBuf::from(suffix),
    }
}

/// Write validation/parse error messages for a failing source file next to it.
pub fn write_err_file(source: &Path, errors: &[String]) -> std::io::Result<PathBuf> {
    let mut content = format!("RFS validation errors for {}\n", source.display());
    for e in errors {
        content.push_str(&format!("- {}\n", e));
    }
    let p = err_file_path(source);
    std::fs::write(&p, content)?;
    Ok(p)
}
/// `rfs validate` CLI entry point (moved from `main.rs` `cmd_validate`).
pub fn run_cli(args: &[String]) -> ExitCode {
    let mut input_format: Option<Format> = None;
    let mut as_json = false;
    let mut path: Option<String> = None;

    let mut i = 0;
    while i < args.len() {
        let arg = args[i].clone();
        if arg == "--help" || arg == "-h" {
            println!("{}", usage());
            return ExitCode::SUCCESS;
        }
        match arg.as_str() {
            "--" => {}
            "--input" => match take_format(args, &mut i, "--input") {
                Ok(f) => input_format = f,
                Err(e) => {
                    eprintln!("error: {}", e);
                    return ExitCode::FAILURE;
                }
            },
            "--json" => as_json = true,
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
        eprintln!(
            "error: rfs validate requires a file or directory\n{}",
            usage()
        );
        return ExitCode::FAILURE;
    };

    if as_json {
        // Structured diagnostics for editor integrations (Phase 5 task 13):
        // the pinned JSON document on stdout; exit codes unchanged (0/1).
        match run_validate_structured(input_format, Path::new(&path)) {
            Err(misuse) => {
                for m in &misuse {
                    eprintln!("error: {}", m);
                }
                ExitCode::FAILURE
            }
            Ok(failed) => {
                let p = Path::new(&path);
                // `print!` (not println): the rendered document already ends
                // with a newline (codegen convention).
                print!("{}", render_validate_json(p, p.is_dir(), &failed));
                if failed.is_empty() {
                    ExitCode::SUCCESS
                } else {
                    ExitCode::FAILURE
                }
            }
        }
    } else {
        match run_validate(input_format, Path::new(&path)) {
            Err(misuse) => {
                for m in &misuse {
                    eprintln!("error: {}", m);
                }
                ExitCode::FAILURE
            }
            Ok(result) => {
                let mut failed = 0usize;
                for f in &result.ok_files {
                    println!("valid {}", f.display());
                }
                for (f, warns) in &result.warnings {
                    for msg in warns {
                        eprintln!("warning: {}: {}", f.display(), msg);
                    }
                }
                for ff in &result.failed {
                    failed += 1;
                    if let Some(err_path) = &ff.err_file {
                        println!(
                            "invalid {} ({} error(s), written to {})",
                            ff.path.display(),
                            ff.errors.len(),
                            err_path.display()
                        );
                    } else {
                        // File mode, or the err file could not be written:
                        // report on stderr.
                        for msg in &ff.errors {
                            eprintln!("{}: {}", ff.path.display(), msg);
                        }
                    }
                }
                for w in &result.write_errors {
                    eprintln!("error: {}", w);
                }
                if failed > 0 || !result.write_errors.is_empty() {
                    ExitCode::FAILURE
                } else {
                    ExitCode::SUCCESS
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn err_file_path_appends_suffix() {
        let p = err_file_path(Path::new("/tmp/dir/spec.rfs"));
        assert_eq!(p, PathBuf::from("/tmp/dir/spec.rfs.rfs-validation.err"));
        let p = err_file_path(Path::new("/tmp/dir/spec.json"));
        assert_eq!(p, PathBuf::from("/tmp/dir/spec.json.rfs-validation.err"));
    }
}
