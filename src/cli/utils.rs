//! Conversion/format utilities shared by the `rfs` CLI commands (the former
//! conversion half of `crate::cli::frontend`).
//!
//! - `Format`: the RFS/JSON source-output format enum + parsing.
//! - `run_conversion`: load `.rfs` or `.json` files (file or directory tree),
//!   convert them through the IR, and write formatted output with the requested
//!   extension. Supports an optional `--target-dir`; without it, each output is
//!   written next to its input file. A collision guard refuses any conversion
//!   whose normalized absolute output path equals the input path.
//! - File-collection + path helpers: `normalize_abs`, `collect_files_with_ext`,
//!   `plan_conversion`, `convert_file_to_ir`, `render_ir`.
//!
//! The validation half now lives in the `rfs validate` command plugin
//! (`crate::cli::commands::validate`); core semantic validation is in
//! `crate::lang::validator` (untouched).
//! Docs: dev:code-map-frontend

use crate::lang::ir::Ir;
use crate::lang::printer;
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
            let value = crate::json::json_rw::parse(&text)
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
}
