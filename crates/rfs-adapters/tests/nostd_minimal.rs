//! no_std-minimal compile checks (Phase 5 task 11 exit criterion): the
//! core-only adapters must compile inside `#![no_std]` crates - both with a
//! hand-written mock FSM and against a REAL task-9 no_std generated FSM
//! (with the opt-in ProtocolFsm impl). Gated on rustc availability, like the
//! toolchain's other compile checks; generation runs unconditionally.

fn rustc_on_path() -> bool {
    std::env::var_os("PATH").map_or(false, |paths| {
        std::env::split_paths(&paths).any(|dir| dir.join("rustc").is_file())
    })
}

/// Locate the rfs-adapters rlib cargo built for this test run.
fn adapters_rlib() -> Option<std::path::PathBuf> {
    let target = std::env::var("CARGO_TARGET_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("../../target"));
    let deps = target.join("debug").join("deps");
    let entries = std::fs::read_dir(&deps).ok()?;
    for e in entries.flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        if name.starts_with("librfs_adapters-") && name.ends_with(".rlib") {
            return Some(e.path());
        }
    }
    None
}

fn compile_nostd_lib(src: &str, rlib: &std::path::Path, out: &std::path::Path) -> String {
    let harness = out.with_extension("rs");
    std::fs::write(&harness, src).unwrap();
    let meta = out.with_extension("rmeta");
    // -L dependency: the rlib's metadata references its own dependencies by
    // name (std is a sysroot crate; futures only under the `async` feature)
    // and rustc must be able to resolve them from the cargo deps dir.
    let res = std::process::Command::new("rustc")
        .args([
            "--edition",
            "2021",
            "--crate-type",
            "lib",
            "--emit=metadata",
        ])
        .arg("-o")
        .arg(&meta)
        .arg("-L")
        .arg(format!("dependency={}", rlib.parent().unwrap().display()))
        .arg("--extern")
        .arg(format!("rfs_adapters={}", rlib.display()))
        .arg(&harness)
        .output()
        .expect("spawn rustc");
    let stderr = String::from_utf8_lossy(&res.stderr).to_string();
    assert!(
        res.status.success(),
        "rustc failed on no_std harness:\n{stderr}"
    );
    stderr
}

/// Check 1: a `#![no_std]` crate using the minimal wrapper + callback driver
/// over an in-file mock FSM (core-only path, no generated code).
#[test]
fn minimal_and_callback_compile_under_no_std_mock() {
    let Some(rlib) = adapters_rlib() else {
        eprintln!("note: rfs-adapters rlib not found - no_std check skipped");
        return;
    };
    if !rustc_on_path() {
        eprintln!("note: rustc not on PATH - no_std check skipped");
        return;
    }
    // Unique per test: the two checks run on parallel threads and must not
    // share (or clean up) each other's directory.
    let dir = std::env::temp_dir().join(format!("rfs_adapters_nostd_{}_mock", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let src = r#"
#![no_std]
#[macro_use]
extern crate alloc;
use alloc::vec::Vec;
use rfs_adapters::callback::{drive, CallbackResult};
use rfs_adapters::minimal::Minimal;
use rfs_adapters::ProtocolFsm;

#[derive(Debug)]
struct NsMock { n: u32 }
#[derive(Debug)]
enum NsIn { Inc, Fail }
#[derive(Debug)]
enum NsOut { Tick(u32) }
#[derive(Debug)]
enum NsErr { Fail }

impl ProtocolFsm for NsMock {
    type Input = NsIn;
    type Output = NsOut;
    type Error = NsErr;
    fn new() -> Self { NsMock { n: 0 } }
    fn step(&mut self, input: NsIn) -> Result<Vec<NsOut>, NsErr> {
        match input {
            NsIn::Inc => { self.n += 1; Ok(vec![NsOut::Tick(self.n)]) }
            NsIn::Fail => Err(NsErr::Fail),
        }
    }
}

#[allow(dead_code)]
fn check() -> usize {
    let mut m = Minimal::<NsMock>::new();
    let _ = m.step(NsIn::Inc).unwrap();
    match drive(&mut m, |_| Some(NsIn::Inc), 3) {
        CallbackResult::StepLimit(k) => k,
        _ => 0,
    }
}
"#;
    compile_nostd_lib(src, &rlib, &dir.join("mock_check"));
    let _ = std::fs::remove_dir_all(&dir);
}

/// Check 2: the minimal wrapper compiles against a REAL task-9 no_std
/// generated FSM (generated here via the rfs toolchain with --no-std +
/// --with-protocol-fsm).
#[test]
fn minimal_compiles_against_nostd_generated_fsm() {
    // Generation runs unconditionally (pure library call).
    let spec = "module pfs {\n  enum State { A, B }\n  initial A\n  event Go\n  var n: bit<4> init 0\n  transition A on Go { next B; set n = n + 1; }\n  transition B on Go { next A; }\n}\n";
    let ir = rfs::parse_source(spec).expect("spec must parse");
    assert_eq!(ir.modules.len(), 1);
    let backend = rfs::codegen::get_backend("rust").expect("rust backend registered");
    let options = rfs::codegen::BackendOptions {
        no_std: true,
        with_protocol_fsm: true,
        ..Default::default()
    };
    let generated = rfs::codegen::CodegenCommandKind::Production
        .run(backend, &ir.modules[0], &options)
        .expect("no_std + protocol-fsm codegen must succeed")
        .code;
    assert!(generated.starts_with("// Generated by RFS Phase 1 codegen.\n#![no_std]\n"));
    assert!(generated.contains("impl ProtocolFsm for Fsm"));

    let Some(rlib) = adapters_rlib() else {
        eprintln!("note: rfs-adapters rlib not found - no_std check skipped");
        return;
    };
    if !rustc_on_path() {
        eprintln!("note: rustc not on PATH - no_std check skipped");
        return;
    }
    let dir = std::env::temp_dir().join(format!(
        "rfs_adapters_nostd_{}_generated",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let src = format!(
        "{}\n#[allow(dead_code)]\nfn minimal_check() {{\n    let m = rfs_adapters::minimal::Minimal::<Fsm>::new();\n}}\n",
        generated
    );
    compile_nostd_lib(&src, &rlib, &dir.join("generated_check"));
    let _ = std::fs::remove_dir_all(&dir);
}
