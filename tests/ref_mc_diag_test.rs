//! Diagnostic companion to ref_mc_sweep_test: a non-fail-fast pass over all
//! tractable valid examples that ENUMERATES every ref_mc-vs-generated-mc
//! mismatch in one shot (the sweep itself stops at the first failure).
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

/// Intractable for exact MC (see ref_mc_sweep_test.rs for the full
/// measurements): hours of ref_mc work / OOM on 19_guard_complex, 1.34e9
/// configs on 39_endpoint_protocol. Neither is a deadlock - both engines run
/// at 100% CPU; ref_mc's --timeout-sec bounds them correctly.
const INTRACTABLE_EXACT_MC: &[&str] = &["19_guard_complex.rfs", "39_endpoint_protocol.rfs"];

struct Prep {
    text: String,
    json: String,
    exit: i32,
    src: String,
}

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

/// Bounded subprocess run (no stop flag; hard deadline only).
fn run_bounded(deadline: Instant, mut cmd: Command) -> Result<Output, String> {
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = cmd.spawn().map_err(|e| format!("spawn failed: {e}"))?;
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
            Err(err) => return Err(format!("wait failed: {err}")),
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            status = child.wait().expect("reap after kill");
            killed = true;
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }
    t_out.join().expect("reader panicked");
    t_err.join().expect("reader panicked");
    if killed {
        return Err("subprocess timed out".into());
    }
    let stdout = out_buf.lock().unwrap().clone();
    let stderr = err_buf.lock().unwrap().clone();
    Ok(Output {
        status,
        stdout,
        stderr,
    })
}

fn diff_snippet(gen: &str, exp: &str) -> String {
    let d = gen
        .as_bytes()
        .iter()
        .zip(exp.as_bytes())
        .position(|(a, b)| a != b)
        .unwrap_or_else(|| gen.len().min(exp.len()));
    let ga = gen.floor_char_boundary(d.saturating_sub(30));
    let gb = gen.ceil_char_boundary((d + 80).min(gen.len()));
    let ea = exp.floor_char_boundary(d.saturating_sub(30));
    let eb = exp.ceil_char_boundary((d + 80).min(exp.len()));
    format!(
        "differs at byte {d}:\ngen: {:?}\nref: {:?}",
        &gen[ga..gb],
        &exp[ea..eb]
    )
}

fn check_sample(job_name: &str, job_path: &Path, dir: &Path, rustc: &str) -> (String, String) {
    // returns (name, "OK" | "HANG" | "MISMATCH: ..." | "ERROR: ...")
    let deadline = Instant::now() + Duration::from_secs(15);
    let path = job_path.to_path_buf();
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let _ = tx.send(prep(path));
    });
    let prep = match rx.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
        Ok(Ok(p)) => p,
        Ok(Err(e)) => return (job_name.into(), format!("ERROR: {e}")),
        Err(_) => return (job_name.into(), "HANG (in-process phase > 15s)".into()),
    };
    let src = dir.join(format!("{job_name}.rs"));
    if std::fs::write(&src, &prep.src).is_err() {
        return (job_name.into(), "ERROR: write".into());
    }
    let crate_name: String = job_name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let bin = dir.join(format!("{job_name}.bin"));
    let mut c = Command::new(rustc);
    c.arg("--edition")
        .arg("2021")
        .arg("-O")
        .arg("--crate-name")
        .arg(&crate_name)
        .arg(&src)
        .arg("-o")
        .arg(&bin);
    let cc = match run_bounded(deadline, c) {
        Ok(o) => o,
        Err(e) => return (job_name.into(), format!("ERROR: {e}")),
    };
    if !cc.status.success() {
        return (
            job_name.into(),
            format!(
                "COMPILE_FAIL: {}",
                String::from_utf8_lossy(&cc.stderr)
                    .lines()
                    .take(3)
                    .collect::<Vec<_>>()
                    .join(" | ")
            ),
        );
    }
    let json_expected = format!("{}\n", prep.json);
    for (label, args, expected) in [
        ("text", vec![], prep.text.as_str()),
        (
            "json",
            vec!["--counterexample-format", "json"],
            json_expected.as_str(),
        ),
    ] {
        let mut c = Command::new(&bin);
        c.args(args);
        let cr = match run_bounded(deadline, c) {
            Ok(o) => o,
            Err(e) => return (job_name.into(), format!("ERROR: {e}")),
        };
        let gen = String::from_utf8_lossy(&cr.stdout).into_owned();
        if cr.status.code() != Some(prep.exit) {
            return (
                job_name.into(),
                format!(
                    "MISMATCH [{label}] exit {:?} vs ref {}",
                    cr.status.code(),
                    prep.exit
                ),
            );
        }
        if gen != expected {
            return (
                job_name.into(),
                format!("MISMATCH [{label}] {}", diff_snippet(&gen, expected)),
            );
        }
    }
    (job_name.into(), "OK".into())
}

#[test]
fn enumerate_all_mismatches() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let dir = root.join("target").join("rfs_diag");
    std::fs::create_dir_all(&dir).unwrap();
    let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".into());
    let files = rfs::collect_rfs_files(root.join("examples/valid")).expect("collect");
    let only: Vec<String> = std::env::var("RFS_DIAG_ONLY")
        .unwrap_or_default()
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    let jobs: Vec<(String, PathBuf)> = files
        .into_iter()
        .map(|p| (p.file_name().unwrap().to_string_lossy().to_string(), p))
        .filter(|(n, _)| {
            only.contains(n) || (only.is_empty() && !INTRACTABLE_EXACT_MC.contains(&n.as_str()))
        })
        .collect();
    eprintln!("diag: {} samples", jobs.len());

    let jobs = Arc::new(jobs);
    let next = Arc::new(AtomicUsize::new(0));
    let results = Arc::new(Mutex::new(Vec::<(String, String)>::new()));
    let mut handles = Vec::new();
    for _w in 0..8 {
        let (next, results, jobs, dir2, rustc2) = (
            Arc::clone(&next),
            Arc::clone(&results),
            Arc::clone(&jobs),
            dir.clone(),
            rustc.clone(),
        );
        handles.push(thread::spawn(move || loop {
            let i = next.fetch_add(1, Ordering::Relaxed);
            if i >= jobs.len() {
                break;
            }
            results
                .lock()
                .unwrap()
                .push(check_sample(&jobs[i].0, &jobs[i].1, &dir2, &rustc2));
        }));
    }
    for h in handles {
        h.join().expect("worker panicked");
    }

    let mut res: Vec<(String, String)> = results.lock().unwrap().clone();
    res.sort_by(|a, b| a.0.cmp(&b.0));
    let mut ok = 0;
    for (name, status) in &res {
        if status == "OK" {
            ok += 1;
        } else {
            eprintln!("{name}: {status}");
        }
    }
    eprintln!("diag: {ok}/{} OK", res.len());
}
