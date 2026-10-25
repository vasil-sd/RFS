use std::path::PathBuf;

fn main() {
    let project_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    match rfs::training::extract_all_snippets(&project_root) {
        Ok(results) => {
            let mut valid = 0;
            let mut invalid = 0;
            for (category, counts) in results {
                println!(
                    "{}: {} valid snippet(s), {} invalid snippet(s)",
                    category
                        .file_name()
                        .map(|s| s.to_string_lossy())
                        .unwrap_or_default(),
                    counts.valid,
                    counts.invalid
                );
                valid += counts.valid;
                invalid += counts.invalid;
            }
            println!(
                "total: {} valid snippet(s), {} invalid snippet(s)",
                valid, invalid
            );
        }
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    }
}
