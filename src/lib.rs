pub mod backends;
pub mod codegen;
pub mod codegen_fsm;
pub mod codegen_mc;
pub mod codegen_test;
pub mod error;
pub mod frontend;
pub mod ir;
pub mod mc_plan;
pub mod monitor;
pub mod parser;
pub mod printer;
pub mod training;
pub mod validator;

pub use error::{ParseError, Span};
pub use frontend::{
    collect_files_with_ext, convert_file_to_ir, err_file_path, normalize_abs, plan_conversion,
    render_ir, run_conversion, run_validate, validate_one, validate_one_with_warnings,
    write_err_file, FailedFile, Format, ValidateOneResult, ValidateResult,
};
pub use ir::*;
pub use parser::{parse_file, parse_source};
pub use printer::print_ir;

use std::fs;
use std::io::{Error as IoError, ErrorKind};
use std::path::{Path, PathBuf};

/// Collect `.rfs` files from a file or directory. Directories are traversed recursively.
pub fn collect_rfs_files<P: AsRef<Path>>(root: P) -> Result<Vec<PathBuf>, IoError> {
    let root = root.as_ref();
    let mut out = Vec::new();

    if root.is_file() {
        if root.extension().and_then(|e| e.to_str()) == Some("rfs") {
            out.push(root.to_path_buf());
        }
        return Ok(out);
    }

    if root.is_dir() {
        collect_rfs_dir(root, &mut out)?;
        out.sort();
        return Ok(out);
    }

    Err(IoError::new(
        ErrorKind::NotFound,
        format!("{} is not a file or directory", root.display()),
    ))
}

fn collect_rfs_dir(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), IoError> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_rfs_dir(&path, out)?;
        } else if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("rfs") {
            out.push(path);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collect_rfs_files_accepts_single_file() {
        let dir = std::env::temp_dir().join(format!("rfs_collect_test_{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let file = dir.join("example.rfs");
        fs::write(&file, "module m { enum State { A } initial A }").unwrap();

        let files = collect_rfs_files(&file).unwrap();
        assert_eq!(files, vec![file.clone()]);

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn parse_source_smoke() {
        let ir = parse_source("module m { enum State { A } initial A }").unwrap();
        assert_eq!(ir.modules.len(), 1);
    }
}
