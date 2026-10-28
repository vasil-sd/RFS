//! sync_handler demo binary: run one ARQ session over std mpsc channels and
//! print what the receiver observed. Exit 0 on success, 1 on protocol error.
//! Docs: dev:workspace-crates-overview, llm:workspace-crates
fn main() {
    match sync_handler::run() {
        Ok(r) => {
            println!(
                "sync_handler: ARQ session complete (FSM on its own thread, std mpsc channels)"
            );
            println!("  ticks issued:             {}", r.ticks);
            println!("  frames sent (in order):   {:?}", r.frames_sent);
            println!("  acks received (in order): {:?}", r.acks_received);
            println!("  out-of-band acks dropped: {}", r.out_of_band_acks_dropped);
            println!("  max window observed:      {} (bound 4)", r.max_window);
        }
        Err(e) => {
            eprintln!("sync_handler FAILED: {e}");
            std::process::exit(1);
        }
    }
}
