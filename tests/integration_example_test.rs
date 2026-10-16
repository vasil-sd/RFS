//! Phase 2 integration example: a realistic endpoint-protocol spec is compiled
//! to production Rust, and a driver reads events from an mpsc channel, steps
//! the generated FSM, and asserts on the emitted outputs. This proves the
//! Phase 2 exit criteria (docs/roadmap/PHASE_2.md): "a realistic endpoint
//! protocol spec compiles to production Rust" and "generated code is
//! deterministic and testable".

use std::path::{Path, PathBuf};
use std::process::Command;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Generate production code for the endpoint-protocol example into a scratch
/// dir and return its path.
fn generate_fsm_source(out_dir: &Path) -> PathBuf {
    let spec_path = root().join("examples/valid/39_endpoint_protocol.rfs");
    let ir = rfs::parse_file(&spec_path).expect("endpoint protocol example must parse");
    assert_eq!(ir.modules.len(), 1);
    let backend = rfs::codegen::get_backend("rust").expect("rust backend registered");
    let options = rfs::codegen::BackendOptions {
        assumptions: rfs::codegen::AssumptionSelection::All,
        steps: 4,
    };
    let out = rfs::codegen::CodegenCommandKind::Production
        .run(backend, &ir.modules[0], &options)
        .expect("production codegen must succeed");
    std::fs::create_dir_all(out_dir).expect("create scratch dir");
    let path = out_dir.join("endpoint_fsm.rs");
    std::fs::write(&path, &out.code).expect("write generated source");
    path
}

/// The driver program: reads event names from an mpsc channel, maps them to
/// Input variants, steps the FSM, and prints one line per step:
/// `state outputs...` (or `ERROR`). A final `__END__` message stops it.
const DRIVER_SRC: &str = r#"
#[path = "endpoint_fsm.rs"]
mod endpoint_fsm;

use endpoint_fsm::*;
use std::sync::mpsc;

fn input_from(name: &str) -> Option<Input> {
    match name {
        "syn" => Some(Input::Recv_Syn { src_port: 4000 }),
        "data_ok" => Some(Input::Recv_Data { seq: 1, len: 3 }),
        "data_ok2" => Some(Input::Recv_Data { seq: 2, len: 5 }),
        "data_bad" => Some(Input::Recv_Data { seq: 9, len: 3 }),
        "fin" => Some(Input::Recv_Fin),
        "rst" => Some(Input::Recv_Rst),
        "tick" => Some(Input::Tick),
        _ => None,
    }
}

fn main() {
    let (tx, rx) = mpsc::channel::<String>();
    // Producer thread: the event channel. Scenario below.
    std::thread::spawn(move || {
        for name in [
            "syn",      // Idle -> Connecting, emits Send_SynAck
            "data_ok",  // Connecting -> Open (priority-10 arm), emits Send_Ack{ack_seq: 1}
            "data_ok2", // Open stays Open, echoes received seq: Send_Ack{ack_seq: 2}
            "fin",      // Open -> Closing, emits Send_FinAck
            "tick",     // Closing -> Closed (timeout), emits Alert_Code(9)
        ] {
            tx.send(name.to_string()).expect("consumer alive");
        }
    });
    let mut fsm = Fsm::new();
    for msg in rx {
        if msg == "__END__" {
            break;
        }
        let Some(input) = input_from(&msg) else {
            eprintln!("bad event name {msg}");
            std::process::exit(2);
        };
        match fsm.step(input) {
            Ok(outputs) => {
                let names: Vec<String> = outputs.iter().map(|o| format!("{o:?}")).collect();
                println!("{:?} [{}]", fsm.state, names.join(", "));
            }
            Err(e) => println!("ERROR {e:?}"),
        }
    }
    println!("final {:?}", fsm.state);
}
"#;

/// Run the scenario twice and compare: generated code must be deterministic.
#[test]
fn endpoint_protocol_channel_driver_is_deterministic() {
    let target = root().join("target").join("rfs_integration");
    let _ = std::fs::remove_dir_all(&target);
    let fsm_src = generate_fsm_source(&target);

    // Write the driver next to the generated FSM source so #[path] resolves.
    let driver_path = target.join("driver.rs");
    std::fs::write(&driver_path, DRIVER_SRC).expect("write driver");
    assert!(fsm_src.exists() && driver_path.exists());

    let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_string());
    let bin = target.join("endpoint_driver");
    let status = Command::new(&rustc)
        .args(["--edition", "2021", "-O"])
        .arg(&driver_path)
        .arg("-o")
        .arg(&bin)
        .output()
        .expect("run rustc on generated code + driver");
    assert!(
        status.status.success(),
        "generated endpoint FSM must compile with rustc:
{}",
        String::from_utf8_lossy(&status.stderr)
    );

    let run = || -> Vec<String> {
        let out = Command::new(&bin).output().expect("run generated driver");
        assert!(
            out.status.success(),
            "driver failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .map(str::to_string)
            .collect()
    };

    let first = run();
    let second = run();
    assert_eq!(first, second, "generated FSM must be deterministic");

    // Expected protocol behavior:
    //   syn      -> Connecting, emits Send_SynAck { src_port: 4000 }
    //   data_ok  -> Open (priority-10 exact-seq arm), emits Send_Ack { ack_seq: 1 }
    //   data_ok  -> Open, emits Send_Ack { ack_seq: 2 } (seq advanced)
    //   fin      -> Closing, emits Send_FinAck
    //   tick     -> Closed, emits Alert_Code { reason: 9 }
    assert!(
        first[0].contains("Connecting") && first[0].contains("Send_SynAck"),
        "step1: {:?}",
        first
    );
    assert!(
        first[1].contains("Open") && first[1].contains("ack_seq: 1"),
        "step2: {:?}",
        first
    );
    assert!(
        first[2].contains("Open") && first[2].contains("ack_seq: 2"),
        "step3: {:?}",
        first
    );
    assert!(
        first[3].contains("Closing") && first[3].contains("Send_FinAck"),
        "step4: {:?}",
        first
    );
    assert!(
        first[4].contains("Closed") && first[4].contains("reason: 9"),
        "step5: {:?}",
        first
    );
    assert_eq!(first[5], "final Closed");

    // Version header must survive into the generated artifact.
    let src = std::fs::read_to_string(&fsm_src).expect("read generated source");
    assert!(src.contains("pub const SPEC_VERSION: &str = ") && src.contains("1.0.0"));
}

#[test]
fn endpoint_protocol_on_error_rejects_unexpected_inputs() {
    // The on_error fallback must handle inputs no transition matches, e.g.
    // Rst while Connecting (declared neither in Connecting nor covered).
    let target = root().join("target").join("rfs_integration_onerr");
    let _ = std::fs::remove_dir_all(&target);
    generate_fsm_source(&target);

    let driver = r#"
#[path = "endpoint_fsm.rs"]
mod endpoint_fsm;
use endpoint_fsm::*;

fn main() {
    let mut fsm = Fsm::new();
    assert!(fsm.step(Input::Recv_Syn { src_port: 1 }).is_ok()); // -> Connecting
    // Rst is not handled in Connecting: on_error takes over.
    match fsm.step(Input::Recv_Rst) {
        Ok(outputs) => println!("ok state={:?} outputs={:?}", fsm.state, outputs),
        Err(e) => println!("err {:?}", e),
    }
}
"#;
    let driver_path = target.join("onerr_driver.rs");
    std::fs::write(&driver_path, driver).expect("write driver");
    let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_string());
    let bin = target.join("onerr_driver");
    let status = Command::new(&rustc)
        .args(["--edition", "2021"])
        .arg(&driver_path)
        .arg("-o")
        .arg(&bin)
        .output()
        .expect("run rustc");
    assert!(
        status.status.success(),
        "rustc failed: {}",
        String::from_utf8_lossy(&status.stderr)
    );
    let out = Command::new(&bin).output().expect("run driver");
    let stdout = String::from_utf8_lossy(&out.stdout);
    // on_error: next Closed, emit Alert_Code(255) - no StepError.
    assert!(
        stdout.contains("ok state=Closed"),
        "on_error must land in Closed: {stdout}"
    );
    assert!(
        stdout.contains("Alert_Code { reason: 255 }"),
        "on_error alert: {stdout}"
    );
}
