//! async_handler demo binary: run one ARQ session over the futures stream
//! adapter and print what the receiver observed. Exit 0 on success, 1 on
//! protocol error.
fn main() {
    match async_handler::run() {
        Ok(r) => {
            println!("async_handler: ARQ session complete (FSM as a futures Stream)");
            println!("  ticks issued:             {}", r.ticks);
            println!("  frames sent (in order):   {:?}", r.frames_sent);
            println!("  acks received (in order): {:?}", r.acks_received);
            println!("  out-of-band acks dropped: {}", r.out_of_band_acks_dropped);
            println!("  max window observed:      {} (bound 4)", r.max_window);
        }
        Err(e) => {
            eprintln!("async_handler FAILED: {e}");
            std::process::exit(1);
        }
    }
}
