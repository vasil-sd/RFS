use std::path::PathBuf;

fn main() {
    let project_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| project_root.join("all_rfs_training.jsonl"));

    match rfs::training::aggregate_training_sets(&project_root, &out) {
        Ok(count) => println!(
            "aggregated {} training examples into {}",
            count,
            out.display()
        ),
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    }
}
