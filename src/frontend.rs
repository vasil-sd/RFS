//! Frontend CLI logic: RFS/JSON format conversion and validation dispatch.
//!
//! This module implements the reusable command-line logic behind `rfs parse`
//! and `rfs validate`:
//!
//! - `run_conversion`: load `.rfs` or `.json` files (file or directory tree),
//!   convert them through the IR, and write formatted output with the requested
//!   extension. Supports an optional `--target-dir`; without it, each output is
//!   written next to its input file. A collision guard refuses any conversion
//!   whose normalized absolute output path equals the input path.
//! - `run_validate`: parse + semantically validate `.rfs` or `.json` files.
//!   For directory inputs, failing files get a `<name>.rfs-validation.err`
//!   text file written next to them.

use crate::ir::Ir;
use crate::printer;
use crate::validator;
use std::path::{Component, Path, PathBuf};

/// Supported source/output formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Rfs,
    Json,
}

impl Format {
    pub fn name(self) -> &'static str {
        match self {
            Format::Rfs => "rfs",
            Format::Json => "json",
        }
    }

    /// File extension (without dot) for this format.
    pub fn ext(self) -> &'static str {
        self.name()
    }

    /// Map a file extension to a format, case-insensitively.
    pub fn from_extension(ext: &str) -> Option<Format> {
        match ext.to_ascii_lowercase().as_str() {
            "rfs" => Some(Format::Rfs),
            "json" => Some(Format::Json),
            _ => None,
        }
    }
}

impl std::str::FromStr for Format {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "rfs" => Ok(Format::Rfs),
            "json" => Ok(Format::Json),
            other => Err(format!(
                "unknown format '{}'; expected 'rfs' or 'json'",
                other
            )),
        }
    }
}

/// Lexically normalize a path to an absolute form, resolving `.` and `..`
/// components. Does not require the path to exist.
pub fn normalize_abs(path: &Path) -> PathBuf {
    let full = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir().unwrap_or_default().join(path)
    };
    let mut out = PathBuf::new();
    for comp in full.components() {
        match comp {
            Component::Prefix(p) => out.push(p.as_os_str()),
            Component::RootDir => out.push("/"),
            Component::CurDir => {}
            Component::ParentDir => {
                // pop() returns false at the root; ignore in that case.
                let _ = out.pop();
            }
            Component::Normal(c) => out.push(c),
        }
    }
    out
}

/// Recursively collect files with the given extension (case-insensitive) under
/// `root`, sorted by path.
pub fn collect_files_with_ext(root: &Path, ext: &str) -> Vec<PathBuf> {
    let mut out = Vec::new();
    collect_dir_with_ext(root, ext, &mut out);
    out.sort();
    out
}

fn collect_dir_with_ext(dir: &Path, ext: &str, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut subdirs = Vec::new();
    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_dir() {
            subdirs.push(p);
        } else if p
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_ascii_lowercase())
            == Some(ext.to_ascii_lowercase())
        {
            out.push(p);
        }
    }
    subdirs.sort();
    for d in subdirs {
        collect_dir_with_ext(&d, ext, out);
    }
}

/// Compute the input/output file pairs for a conversion request.
///
/// Returns all mapping errors (extension mismatches, missing paths, collision
/// guard violations) on `Err`; an empty error list is never returned with `Err`.
pub fn plan_conversion(
    input_format: Format,
    output_format: Format,
    path: &Path,
    target_dir: Option<&Path>,
) -> Result<Vec<(PathBuf, PathBuf)>, Vec<String>> {
    let mut errors = Vec::new();
    let mut pairs = Vec::new();

    if path.is_file() {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        match Format::from_extension(ext) {
            Some(f) if f == input_format => {}
            Some(f) => {
                errors.push(format!(
                    "input file '{}' has extension '.{}' but --input is '{}'; use --input {} or provide a .{} file",
                    path.display(),
                    f.ext(),
                    input_format.name(),
                    f.name(),
                    input_format.ext()
                ));
            }
            None => {
                errors.push(format!(
                    "input file '{}' must have extension .{} (matching --input {})",
                    path.display(),
                    input_format.ext(),
                    input_format.name()
                ));
            }
        }
        if errors.is_empty() {
            pairs.push((
                path.to_path_buf(),
                output_path_for(path, target_dir, input_format, output_format, Some(path)),
            ));
        }
    } else if path.is_dir() {
        let files = collect_files_with_ext(path, input_format.ext());
        if files.is_empty() {
            errors.push(format!(
                "no .{} files found under '{}'",
                input_format.ext(),
                path.display()
            ));
        }
        for f in files {
            pairs.push((
                f.clone(),
                output_path_for(&f, target_dir, input_format, output_format, Some(path)),
            ));
        }
    } else {
        errors.push(format!(
            "input path '{}' does not exist or is not a file or directory",
            path.display()
        ));
    }

    // Collision guard: never allow output to overwrite its own input.
    for (in_path, out_path) in &pairs {
        if normalize_abs(in_path) == normalize_abs(out_path) {
            errors.push(format!(
                "refusing to overwrite input file: output '{}' is identical to input '{}' (use --target-dir or convert to a different format)",
                out_path.display(),
                in_path.display()
            ));
        }
    }

    if errors.is_empty() {
        Ok(pairs)
    } else {
        Err(errors)
    }
}

/// Compute the output path for one input file.
fn output_path_for(
    input: &Path,
    target_dir: Option<&Path>,
    _input_format: Format,
    output_format: Format,
    scan_root: Option<&Path>,
) -> PathBuf {
    // Strip the original extension, append the new one: a.rfs -> a.json
    let stem = input
        .file_stem()
        .unwrap_or(input.file_name().unwrap_or_default());
    let mut new_name = stem.to_os_string();
    new_name.push(format!(".{}", output_format.ext()));

    match target_dir {
        None => input
            .parent()
            .map(|p| p.join(new_name.clone()))
            .unwrap_or_else(|| PathBuf::from(new_name)),
        Some(td) => {
            // Preserve the input directory structure relative to the scan root.
            let rel = scan_root
                .and_then(|root| input.strip_prefix(root).ok())
                .unwrap_or_else(|| Path::new(input.file_name().unwrap_or_default()));
            if rel.as_os_str().is_empty() {
                td.join(new_name)
            } else {
                let mut out = td.join(rel);
                out.set_file_name(new_name);
                out
            }
        }
    }
}

/// Load a file (RFS source or IR JSON) into an IR document.
pub fn convert_file_to_ir(path: &Path, format: Format) -> Result<Ir, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("failed to read {}: {}", path.display(), e))?;
    match format {
        Format::Rfs => crate::parse_source(&text).map_err(|e| e.message),
        Format::Json => {
            let value = crate::json_rw::parse(&text)
                .map_err(|e| format!("invalid RFS IR JSON in {}: {}", path.display(), e))?;
            Ir::from_json(&value)
                .map_err(|e| format!("invalid RFS IR JSON in {}: {}", path.display(), e))
        }
    }
}

/// Render an IR document as formatted output text for the target format.
pub fn render_ir(ir: &Ir, format: Format) -> Result<String, String> {
    match format {
        Format::Rfs => Ok(printer::print_ir(ir)),
        Format::Json => ir.to_json().map(|v| v.to_string_pretty() + "\n"),
    }
}

/// Run a full conversion request. Returns (written output paths, errors).
/// Planning errors (extension mismatches, collisions) abort before any file is
/// written; per-file conversion errors are collected without stopping the run.
pub fn run_conversion(
    input_format: Format,
    output_format: Format,
    path: &Path,
    target_dir: Option<&Path>,
) -> (Vec<PathBuf>, Vec<String>) {
    let pairs = match plan_conversion(input_format, output_format, path, target_dir) {
        Ok(p) => p,
        Err(errors) => return (Vec::new(), errors),
    };
    let mut written = Vec::new();
    let mut errors = Vec::new();
    for (in_path, out_path) in pairs {
        match convert_file_to_ir(&in_path, input_format)
            .and_then(|ir| render_ir(&ir, output_format))
        {
            Ok(content) => {
                if let Some(parent) = out_path.parent() {
                    if let Err(e) = std::fs::create_dir_all(parent) {
                        errors.push(format!(
                            "failed to create directory {}: {}",
                            parent.display(),
                            e
                        ));
                        continue;
                    }
                }
                match std::fs::write(&out_path, content) {
                    Ok(()) => written.push(out_path),
                    Err(e) => errors.push(format!("failed to write {}: {}", out_path.display(), e)),
                }
            }
            Err(e) => errors.push(format!("{}: {}", in_path.display(), e)),
        }
    }
    (written, errors)
}

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
            let value = crate::json_rw::parse(&text)
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
fn validate_json_doc(path: &Path, errors: &[StructuredError]) -> crate::json_rw::Json {
    crate::json_rw::Json::obj(vec![
        (
            "file".to_string(),
            crate::json_rw::Json::str(path.display().to_string()),
        ),
        (
            "ok".to_string(),
            crate::json_rw::Json::bool(errors.is_empty()),
        ),
        (
            "errors".to_string(),
            crate::json_rw::Json::arr(
                errors
                    .iter()
                    .map(|e| {
                        crate::json_rw::Json::obj(vec![
                            ("kind".to_string(), crate::json_rw::Json::str(e.kind)),
                            ("line".to_string(), crate::json_rw::Json::int(e.line as i64)),
                            ("col".to_string(), crate::json_rw::Json::int(e.col as i64)),
                            ("message".to_string(), crate::json_rw::Json::str(&e.message)),
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
        crate::json_rw::Json::arr(
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_parsing() {
        assert_eq!("rfs".parse::<Format>().unwrap(), Format::Rfs);
        assert_eq!("JSON".parse::<Format>().unwrap(), Format::Json);
        assert!("xml".parse::<Format>().is_err());
        assert_eq!(Format::from_extension("rfs"), Some(Format::Rfs));
        assert_eq!(Format::from_extension("Json"), Some(Format::Json));
        assert_eq!(Format::from_extension("txt"), None);
    }

    #[test]
    fn normalize_abs_resolves_dots() {
        let a = normalize_abs(Path::new("x/y/../z.rfs"));
        let b = normalize_abs(Path::new("x/z.rfs"));
        assert_eq!(a, b);
        assert!(a.is_absolute());
    }

    #[test]
    fn err_file_path_appends_suffix() {
        let p = err_file_path(Path::new("/tmp/dir/spec.rfs"));
        assert_eq!(p, PathBuf::from("/tmp/dir/spec.rfs.rfs-validation.err"));
        let p = err_file_path(Path::new("/tmp/dir/spec.json"));
        assert_eq!(p, PathBuf::from("/tmp/dir/spec.json.rfs-validation.err"));
    }
}
