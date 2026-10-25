//! Byte-parity sweep: compares ref_mc against the generated Rust MC binary for
//! every tractable valid example - exit-code + exact-stdout parity in both text
//! and JSON counterexample formats.
//!
//! Orchestration knobs are env vars.
//!
//! FAIL-FAST: the first failed sample sets a shared stop flag; every worker
//! kills its in-flight subprocess (rustc / mc binary) at the next checkpoint
//! (<=10ms poll) and exits; the test panics with all reported failures.
//!
//! PARALLELISM: N workers pull samples from a shared queue.
//! `RFS_SWEEP_PAR=<n>` - default: available CPUs / 2 (min 1).
//!
//! TIMEOUT: one deadline per sample covering ref_mc+codegen+compile+both runs.
//! `RFS_SWEEP_TIMEOUT_SECS=<s>` - default: 60. Default rationale (measured on
//! this host, 32 cores/125GB, 2026-10-24): healthy per-sample wall time
//! max=3.41s, median=0.86s over all 45 non-hanging examples -> 60s is a ~17x
//! margin. Thanks to fail-fast the worst-case suite wall time is one timeout +
//! kill overhead (~2 min), well under the 10-minute budget.
//!
//! FAILED-FIRST REORDERING: after a failing run, failed samples are written
//! (first failure first) to target/rfs_refmc_sweep/failed_order.txt and the
//! next run queues them at the front of the queue; a fully green run deletes
//! the file.
//!
//! `RFS_SWEEP_ONLY=name1,name2` - debug subset (basenames, e.g.
//! 45_elevator.rfs); does not touch the failed-order file.
//!
//! In-process phases (parse/validate/ref_mc/render/codegen) run in a watchdog
//! thread per sample, so a slow or stuck ref_mc is caught by the deadline;
//! the thread is leaked on timeout (it holds no shared resources).
//!
//! Usage:
//!   cargo test --test ref_mc_sweep_test -- --nocapture
//!   RFS_SWEEP_PAR=8 RFS_SWEEP_TIMEOUT_SECS=120 \
//!       cargo test --test ref_mc_sweep_test -- --nocapture

mod common;

use std::collections::{HashMap, VecDeque};
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

/// Default per-sample deadline in seconds. Rationale: see module docs.
const DEFAULT_TIMEOUT_SECS: u64 = 60;

/// Examples that are VALID specs (parse/validate/codegen all pass, and they
/// stay in examples/valid for those tests) but INTRACTABLE for exact MC on
/// this host (measured 2026-10-24):
///   * 19_guard_complex - 1 state x bit<8>^2 vars (65536 configs); the
///     referenced bit<8>^2 payload makes 65536 representative inputs per
///     config, so ref_mc needs ~4.3e9 transition steps (hours at ~400K/s)
///     and the generated MC's input materialization is OOM-killed (~2GB/s
///     growth). NOT a deadlock: ref_mc runs at 100% CPU; its --timeout-sec
///     now bounds it correctly (per-config deadline check in ref_mc::run).
///   * 39_endpoint_protocol - 5 states x bit<8> x bit<4> x bit<16> vars =
///     1.34e9 configurations; exact enumeration never completes in either
///     engine (ref_mc bounded by --timeout-sec, same as above).
///
/// The sweep skips them by default; RFS_SWEEP_ONLY still runs them on demand.
const INTRACTABLE_EXACT_MC: &[&str] = &["19_guard_complex.rfs", "39_endpoint_protocol.rfs"];

struct Job {
    name: String,
    path: PathBuf,
}

/// Result of the in-process phases for one sample.
struct Prep {
    text: String,
    json: String,
    exit: i32,
    src: String,
}

enum SampleErr {
    /// Genuine failure of this sample (recorded; triggers fail-fast).
    Fail(String),
    /// Per-sample deadline exceeded (recorded as a timeout failure).
    TimedOut,
    /// Another sample failed first; this one is abandoned without being recorded.
    Stopped,
}

/// All in-process phases for one sample. Runs in a watchdog thread so a
/// deadlock here cannot stall the suite beyond the per-sample deadline.
fn prep(path: PathBuf) -> Result<Prep, String> {
    let ir = rfs::parse_file(&path).map_err(|e| format!("parse error: {e:?}"))?;
    let module = ir
        .modules
        .first()
        .ok_or_else(|| "IR has no module".to_string())?;
    if let Err(e) = rfs::validator::validate_ir(&ir) {
        return Err(format!("valid example failed validation: {e:?}"));
    }
    let cfg = rfs::ref_mc::RefConfig::default();
    let res = rfs::ref_mc::run(module, &cfg).map_err(|e| format!("ref_mc error: {e}"))?;
    let text = rfs::ref_mc::render_text(&res, module);
    let json = rfs::ref_mc::to_json_report(&res, module).to_json_string();
    let exit = res.status.exit_code() as i32;
    let backend = rfs::codegen::get_backend("rust").expect("rust backend registered");
    let opts = rfs::codegen::BackendOptions::default();
    let out = (backend.generate_model_checker)(module, &opts)
        .map_err(|e| format!("codegen error: {e:?}"))?;
    Ok(Prep {
        text,
        json,
        exit,
        src: out.code,
    })
}

/// Wait for the watchdog result, honoring stop + deadline in 50ms ticks.
fn wait_watchdog(
    rx: mpsc::Receiver<Result<Prep, String>>,
    stop: &AtomicBool,
    deadline: Instant,
) -> Result<Prep, SampleErr> {
    loop {
        match rx.recv_timeout(Duration::from_millis(50)) {
            Ok(Ok(prep)) => return Ok(prep),
            Ok(Err(e)) => return Err(SampleErr::Fail(e)),
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return Err(SampleErr::Fail("prep watchdog thread died".into()));
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                if stop.load(Ordering::Relaxed) {
                    return Err(SampleErr::Stopped);
                }
                if Instant::now() >= deadline {
                    return Err(SampleErr::TimedOut);
                }
            }
        }
    }
}

/// Run a subprocess with stdout/stderr captured, bounded by `deadline`; the
/// child is killed on timeout or when `stop` is observed.
fn run_bounded(
    stop: &AtomicBool,
    deadline: Instant,
    mut cmd: Command,
) -> Result<Output, SampleErr> {
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = cmd
        .spawn()
        .map_err(|e| SampleErr::Fail(format!("spawn failed: {e}")))?;
    let stdout_pipe = child.stdout.take();
    let stderr_pipe = child.stderr.take();
    let out_buf = Arc::new(Mutex::new(Vec::new()));
    let err_buf = Arc::new(Mutex::new(Vec::new()));
    let (o, e) = (Arc::clone(&out_buf), Arc::clone(&err_buf));
    let t_out = thread::spawn(move || {
        if let Some(mut p) = stdout_pipe {
            let _ = io::copy(&mut p, &mut *o.lock().unwrap());
        }
    });
    let t_err = thread::spawn(move || {
        if let Some(mut p) = stderr_pipe {
            let _ = io::copy(&mut p, &mut *e.lock().unwrap());
        }
    });

    let (status, killed);
    loop {
        match child.try_wait() {
            Ok(Some(s)) => {
                status = s;
                killed = false;
                break;
            }
            Ok(None) => {}
            Err(err) => return Err(SampleErr::Fail(format!("wait failed: {err}"))),
        }
        if stop.load(Ordering::Relaxed) || Instant::now() >= deadline {
            let _ = child.kill();
            status = child.wait().expect("reap after kill");
            killed = true;
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }
    t_out.join().expect("stdout reader panicked");
    t_err.join().expect("stderr reader panicked");
    if killed {
        return Err(if stop.load(Ordering::Relaxed) {
            SampleErr::Stopped
        } else {
            SampleErr::TimedOut
        });
    }
    let stdout = out_buf.lock().unwrap().clone();
    let stderr = err_buf.lock().unwrap().clone();
    Ok(Output {
        status,
        stdout,
        stderr,
    })
}

/// Full per-sample pipeline under one deadline. Returns the ref exit code on OK.
fn run_sample(
    job: &Job,
    dir: &Path,
    rustc: &str,
    stop: &AtomicBool,
    timeout: Duration,
) -> Result<i32, SampleErr> {
    let deadline = Instant::now() + timeout;

    // 1) in-process phases under the watchdog.
    let path = job.path.clone();
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let _ = tx.send(prep(path));
    });
    let prep = wait_watchdog(rx, stop, deadline)?;
    if stop.load(Ordering::Relaxed) {
        return Err(SampleErr::Stopped);
    }

    // 2) write generated source.
    let src = dir.join(format!("{}.rs", job.name));
    std::fs::write(&src, &prep.src).map_err(|e| SampleErr::Fail(format!("write source: {e}")))?;

    // 3) compile (crate name must be rustc-legal: dots in .rfs names are not).
    let crate_name: String = job
        .name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let bin = dir.join(format!("{}.bin", job.name));
    let mut c = Command::new(rustc);
    c.arg("--edition")
        .arg("2021")
        .arg("-O")
        .arg("--crate-name")
        .arg(&crate_name)
        .arg(&src)
        .arg("-o")
        .arg(&bin);
    let cc = run_bounded(stop, deadline, c)?;
    if stop.load(Ordering::Relaxed) {
        return Err(SampleErr::Stopped);
    }
    if !cc.status.success() {
        return Err(SampleErr::Fail(format!(
            "rustc failed (exit {:?}): {}",
            cc.status.code(),
            String::from_utf8_lossy(&cc.stderr)
        )));
    }

    // 4) run the generated binary in both formats and compare with ref_mc.
    // Wire form of the JSON report is one line + trailing newline (the
    // generated engines print via println!; see backend_parity_test's
    // "stdout carries the trailing newline" contract), while to_json_string()
    // returns the bare document.
    let json_expected = format!("{}\n", prep.json);
    let cases: Vec<(&str, Vec<&str>, &String)> = vec![
        ("text", vec![], &prep.text),
        (
            "json",
            vec!["--counterexample-format", "json"],
            &json_expected,
        ),
    ];
    for (label, args, expected) in cases {
        let mut c = Command::new(&bin);
        c.args(args);
        let cr = run_bounded(stop, deadline, c)?;
        if stop.load(Ordering::Relaxed) {
            return Err(SampleErr::Stopped);
        }
        // Byte agreement modulo the informational peak_rss_kb field (the
        // generated binary self-reports actual RSS; ref_mc's rendering emits
        // 0 - task 6). zero_peak_rss is a no-op on text output.
        let gen = common::mc_json::zero_peak_rss(&String::from_utf8_lossy(&cr.stdout));
        let expected = common::mc_json::zero_peak_rss(expected);
        if cr.status.code() != Some(prep.exit) {
            return Err(SampleErr::Fail(format!(
                "[{label}] exit {:?} vs ref {}",
                cr.status.code(),
                prep.exit
            )));
        }
        if gen != expected {
            return Err(SampleErr::Fail(format!(
                "[{label}] {}",
                diff_snippet(&gen, &expected)
            )));
        }
    }
    Ok(prep.exit)
}

fn diff_snippet(gen: &str, exp: &str) -> String {
    let d = gen
        .as_bytes()
        .iter()
        .zip(exp.as_bytes())
        .position(|(a, b)| a != b)
        .unwrap_or_else(|| gen.len().min(exp.len()));
    let ga = gen.floor_char_boundary(d.saturating_sub(20));
    let gb = gen.ceil_char_boundary((d + 60).min(gen.len()));
    let ea = exp.floor_char_boundary(d.saturating_sub(20));
    let eb = exp.ceil_char_boundary((d + 60).min(exp.len()));
    format!(
        "stdout differs at byte {}:\ngen:  {:?}\nref:  {:?}",
        d,
        &gen[ga..gb],
        &exp[ea..eb]
    )
}

#[test]
fn ref_mc_vs_generated_rust_mc_sweep() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let dir = root.join("target").join("rfs_refmc_sweep");
    std::fs::create_dir_all(&dir).unwrap();

    // --- parameters ---
    let par: usize = match std::env::var("RFS_SWEEP_PAR") {
        Ok(v) => v
            .trim()
            .parse()
            .expect("RFS_SWEEP_PAR must be a positive integer"),
        Err(_) => std::thread::available_parallelism()
            .map(|n| (n.get() / 2).max(1))
            .unwrap_or(4),
    };
    let timeout: Duration = match std::env::var("RFS_SWEEP_TIMEOUT_SECS") {
        Ok(v) => Duration::from_secs(
            v.trim()
                .parse()
                .expect("RFS_SWEEP_TIMEOUT_SECS must be a positive integer"),
        ),
        Err(_) => Duration::from_secs(DEFAULT_TIMEOUT_SECS),
    };
    let only: Vec<String> = std::env::var("RFS_SWEEP_ONLY")
        .unwrap_or_default()
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    // --- jobs, failed-first ordered ---
    let files =
        rfs::collect_rfs_files(root.join("examples/valid")).expect("collect examples/valid");
    let mut by_name: HashMap<String, Job> = HashMap::new();
    let mut skipped_intractable = 0usize;
    for p in files {
        let name = p
            .file_name()
            .and_then(|s| s.to_str())
            .expect("utf8 file name")
            .to_string();
        if only.is_empty() || only.contains(&name) {
            if only.is_empty() && INTRACTABLE_EXACT_MC.contains(&name.as_str()) {
                skipped_intractable += 1; // intractable for exact MC (see const docs)
                continue;
            }
            by_name.insert(name.clone(), Job { name, path: p });
        }
    }
    assert!(
        !by_name.is_empty(),
        "no examples matched (RFS_SWEEP_ONLY={only:?})"
    );

    let order_file = dir.join("failed_order.txt");
    let prev_failed: Vec<String> = std::fs::read_to_string(&order_file)
        .map(|s| {
            s.lines()
                .map(|l| l.trim().to_string())
                .filter(|l| !l.is_empty())
                .collect()
        })
        .unwrap_or_default();

    let mut jobs: Vec<Job> = Vec::new();
    for name in &prev_failed {
        if let Some(j) = by_name.remove(name) {
            jobs.push(j);
        }
    }
    let mut rest: Vec<Job> = by_name.into_values().collect();
    rest.sort_by(|a, b| a.name.cmp(&b.name));
    jobs.append(&mut rest);

    eprintln!(
        "sweep: {} samples ({} intractable skipped) | par={par} | timeout={}s | failed-first: {}",
        jobs.len(),
        skipped_intractable,
        timeout.as_secs(),
        if prev_failed.is_empty() {
            "none".into()
        } else {
            prev_failed.join(",")
        }
    );

    // --- shared state ---
    let stop = Arc::new(AtomicBool::new(false));
    let queue = Arc::new(Mutex::new(VecDeque::from(
        (0..jobs.len()).collect::<Vec<usize>>(),
    )));
    let failures = Arc::new(Mutex::new(Vec::<(String, String)>::new()));
    let jobs = Arc::new(jobs);
    let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".into());

    let mut handles = Vec::new();
    for _w in 0..par.min(jobs.len()) {
        let (stop, queue, failures, jobs) = (
            Arc::clone(&stop),
            Arc::clone(&queue),
            Arc::clone(&failures),
            Arc::clone(&jobs),
        );
        let dir2 = dir.clone();
        let rustc2 = rustc.clone();
        handles.push(thread::spawn(move || loop {
            if stop.load(Ordering::Relaxed) {
                break;
            }
            let Some(i) = queue.lock().unwrap().pop_front() else {
                break;
            };
            let job = &jobs[i];
            match run_sample(job, &dir2, &rustc2, &stop, timeout) {
                Ok(exit) => eprintln!("{}: OK (exit {exit})", job.name),
                Err(SampleErr::Stopped) => break,
                Err(e) => {
                    let detail = match &e {
                        SampleErr::Fail(m) => m.clone(),
                        SampleErr::TimedOut => format!("timed out after {}s", timeout.as_secs()),
                        SampleErr::Stopped => unreachable!(),
                    };
                    eprintln!("{}: FAIL - {detail}", job.name);
                    failures.lock().unwrap().push((job.name.clone(), detail));
                    stop.store(true, Ordering::Relaxed);
                }
            }
        }));
    }
    for h in handles {
        h.join().expect("worker thread panicked");
    }

    // --- verdict ---
    let failed = failures.lock().unwrap().clone();
    if !failed.is_empty() {
        if only.is_empty() {
            let s: String = failed.iter().map(|(n, _)| format!("{n}\n")).collect();
            std::fs::write(&order_file, s).expect("write failed_order.txt");
        }
        panic!(
            "SWEEP FAILED: {} of {} samples failed (fail-fast stopped the rest):\n{}\nNext run will try the failed sample(s) first ({order}).",
            failed.len(), jobs.len(),
            failed.iter().map(|(n, d)| format!("  {n}: {d}")).collect::<Vec<_>>().join("\n"),
            order = if only.is_empty() { "order file updated" } else { "RFS_SWEEP_ONLY set, order file untouched" },
        );
    }
    if only.is_empty() {
        let _ = std::fs::remove_file(&order_file);
    }
    eprintln!("sweep: all {} samples OK", jobs.len());
}
