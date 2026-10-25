//! Phase 6 task 4, action 4 (subtask 4.4): monitors vs hand-written reference
//! monitors (D-P6-6 family). For specs 43/44/45 - the property-rich benchmark
//! specs - this suite compares the GENERATED online property monitors against
//! two INDEPENDENT reference implementations of the documented semantics
//! (SEMANTICS.md 5.2-5.5):
//!
//! - a Rust reference written by hand in this file (plain functions over a
//!   parsed sample stream; no codegen involvement), and
//! - a C reference: per-spec hand-written bodies transcribed directly from the
//!   spec formulas + one generic engine, compiled per spec with `gcc -Wall`.
//!
//! Both references are driven over the same sample streams as the generated
//! monitors:
//!
//! 1. RANDOM traces: action 3's seeded input script drives the real production
//!    FSM (Rust driver); the committed trace is parsed back out of the driver
//!    stdout and fed to both references. The generated monitor's verdicts are
//!    that run's `M{idx}`/`FIRST` lines.
//! 2. CRAFTED sample scripts: raw `State|v0,v1,...` lines exercising failure
//!    paths random traces cannot reach - all three specs' monitorable
//!    properties are true invariants, so assumption-respecting random runs
//!    never confirm a violation. The crafted scripts cover boundary values,
//!    bit-width wraparound arithmetic, obligation deadlines (confirmed exactly
//!    at trigger+bound-1), re-trigger after clear, sticky failure, and
//!    same-sample multi-property confirmation (declaration-order tie-break).
//!    They drive SAMPLE-LEVEL drivers of the generated artifacts (Rust + C,
//!    which must also agree with each other) plus both references.
//!
//! Only the MONITORABLE subset is compared: `eventually`/`until` properties
//! are model-checking-only and production codegen skips them with a warning
//! (SEMANTICS 5/6), so they have no generated monitor to compare against.
//! Verdict contract (both references and all drivers): per sample k, `M{k} OK`
//! or `M{k} FAIL {label} step={k} trig={t|-1}` for the FIRST newly-confirmed
//! monitor at k (declaration order), plus a final `FIRST ...` line for the
//! earliest confirmation overall.

use std::path::PathBuf;
use std::process::Command;

mod common;
use common::prod_driver;

/// Pinned random-trace parameters (task file: "seeded random traces").
const TRACE_SEED: u64 = 90_717;
const TRACE_STEPS: usize = 200;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn scratch() -> PathBuf {
    root().join("target").join("rfs_monitor_ref")
}

// ---------------------------------------------------------------------------
// Sample model + script parsing/rendering (shared by the references and the
// generated sample-level drivers).

#[derive(Debug, Clone)]
struct Sample {
    state: String,
    /// Declaration-order raw var values (bool 0/1, bit<N> decimal, enum index).
    vars: Vec<u64>,
    /// Whether the LIVE FSM stepping evaluated this sample through the
    /// property monitors. ASSUME-stopped samples are recorded in the trace
    /// (and stepped by the probe's fresh monitors) but returned before the
    /// live prop check, so they never confirm a violation live.
    live: bool,
}

/// Parse `State|v0,v1,...` lines (bare `State` when the module has no vars).
fn parse_samples(text: &str) -> Vec<Sample> {
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        match line.split_once('|') {
            Some((state, rest)) => out.push(parse_sample_line(state, rest, true)),
            None => out.push(Sample {
                state: line.to_string(),
                vars: Vec::new(),
                live: true,
            }),
        }
    }
    out
}

fn parse_sample_line(state: &str, vars_s: &str, live: bool) -> Sample {
    let vars = if vars_s.is_empty() || vars_s == "-" {
        Vec::new()
    } else {
        vars_s
            .split(',')
            .map(|s| s.parse::<u64>().expect("var value"))
            .collect()
    };
    Sample {
        state: state.to_string(),
        vars,
        live,
    }
}

/// Render samples back to the script format (fed to the C reference).
fn render_samples(samples: &[Sample]) -> String {
    let mut out = String::new();
    for s in samples {
        if s.vars.is_empty() {
            out.push_str(&format!("{}\n", s.state));
        } else {
            let vs: Vec<String> = s.vars.iter().map(|v| v.to_string()).collect();
            out.push_str(&format!("{}|{}\n", s.state, vs.join(",")));
        }
    }
    out
}

/// Extract the monitor verdict block (M* lines + FIRST line) from a driver's
/// stdout. Sample-level drivers print ONLY this block; input-level drivers
/// mix it with S*/init lines.
fn verdict_block(out: &str) -> String {
    out.lines()
        .filter(|l| l.starts_with('M') || l.starts_with("FIRST"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Parse the recorded trace (sample 0 = init, sample k = step k's recorded
/// target config) out of an action-3 input-level driver stdout. Committed,
/// ASSUME-stopped and PROP-stopped steps all carry ` st=A->B vars=...`;
/// UNEXPECTED lines record nothing. ASSUME-stopped samples are marked
/// `live = false` (see [Sample::live]).
fn parse_trace_from_driver_output(out: &str) -> Vec<Sample> {
    let mut samples = Vec::new();
    for line in out.lines() {
        if let Some(rest) = line.strip_prefix("init st=") {
            let (st, vars_s) = rest.split_once(" vars=").expect("init line shape");
            samples.push(parse_sample_line(st, vars_s, true));
        } else if let Some(rest) = line.strip_prefix('S') {
            if !rest.starts_with(|c: char| c.is_ascii_digit()) {
                continue;
            }
            let (transition, vars_s) = match rest.split_once(" st=") {
                Some((_, r)) => match r.split_once(" vars=") {
                    Some((t, v)) => (t, v),
                    None => continue,
                },
                None => continue, // UNEXPECTED - no sample recorded
            };
            let live = !line.contains(" ASSUME ");
            // transition: "A->B" (or bare "B" defensively)
            let to = match transition.rsplit_once("->") {
                Some((_, t)) => t,
                None => transition,
            };
            samples.push(parse_sample_line(to, vars_s, live));
        }
    }
    samples
}

/// Convert an M-line (`M{k} FAIL {label} step={k} trig={t}`) into the
/// corresponding FIRST line; None for OK lines.
fn m_line_to_first(m_line: &str) -> Option<String> {
    let after_idx = m_line.split_once(' ')?.1;
    let after_fail = after_idx.split_once(' ')?.1;
    Some(format!("FIRST {after_fail}"))
}

// ---------------------------------------------------------------------------
// Hand-written Rust reference monitors (independent of the generated code).
// Generic engine per SEMANTICS 5.4/5.5: sticky failure, first confirmation
// wins, declaration order on same-sample ties; obligation stepping exactly as
// documented (trigger check before body check at each sample).

#[derive(Debug, Clone, Copy)]
struct RefViolation {
    label: &'static str,
    step: usize,
    /// Sample that started the failing obligation; -1 for per-sample checks.
    trig: i64,
}

enum RefProp {
    /// `always (body)` - a plain per-sample check.
    Always {
        label: &'static str,
        body: fn(&Sample) -> bool,
    },
    /// `always (trigger -> bounded_eventually[bound] body)` - reactive
    /// obligation; re-triggers while pending are ignored.
    ReactiveBoundedEventually {
        label: &'static str,
        trigger: fn(&Sample) -> bool,
        body: fn(&Sample) -> bool,
        bound: u64,
    },
}

/// Step all reference monitors over the sample stream. Returns (per-sample M
/// lines, first violation overall).
fn ref_run(props: &[RefProp], samples: &[Sample]) -> (Vec<String>, Option<RefViolation>) {
    let mut failed = vec![false; props.len()];
    let mut pending = vec![false; props.len()];
    let mut remaining = vec![0u64; props.len()];
    let mut trig_step = vec![-1i64; props.len()];
    let mut m_lines = Vec::with_capacity(samples.len());
    let mut first: Option<RefViolation> = None;

    for (k, s) in samples.iter().enumerate() {
        // First newly-confirmed monitor at this sample, declaration order.
        let mut newly: Option<(usize, RefViolation)> = None;
        for (pi, p) in props.iter().enumerate() {
            if failed[pi] {
                continue;
            }
            match *p {
                RefProp::Always { label, body } => {
                    if !body(s) {
                        failed[pi] = true;
                        if newly.is_none() {
                            newly = Some((
                                pi,
                                RefViolation {
                                    label,
                                    step: k,
                                    trig: -1,
                                },
                            ));
                        }
                    }
                }
                RefProp::ReactiveBoundedEventually {
                    label,
                    trigger,
                    body,
                    bound,
                } => {
                    let a = trigger(s);
                    let b = body(s);
                    if !pending[pi] && a {
                        pending[pi] = true;
                        remaining[pi] = bound;
                        trig_step[pi] = k as i64;
                    }
                    if pending[pi] {
                        if b {
                            pending[pi] = false;
                        } else {
                            remaining[pi] -= 1;
                            if remaining[pi] == 0 {
                                failed[pi] = true;
                                if newly.is_none() {
                                    newly = Some((
                                        pi,
                                        RefViolation {
                                            label,
                                            step: k,
                                            trig: trig_step[pi],
                                        },
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }
        match newly {
            Some((_, v)) => {
                m_lines.push(format!(
                    "M{} FAIL {} step={} trig={}",
                    k, v.label, v.step, v.trig
                ));
                if first.is_none() {
                    first = Some(v);
                }
            }
            None => m_lines.push(format!("M{} OK", k)),
        }
    }
    (m_lines, first)
}

fn block_of(m_lines: &[String], first: Option<RefViolation>) -> String {
    let mut b = m_lines.join("\n");
    match first {
        Some(v) => b.push_str(&format!(
            "\nFIRST {} step={} trig={}",
            v.label, v.step, v.trig
        )),
        None => b.push_str("\nFIRST none"),
    }
    b
}

// ---------------------------------------------------------------------------
// Per-spec definitions: the monitorable properties (declaration order) as
// hand-written Rust bodies, their C twins, and the crafted failure scenarios.
// Bodies are transcribed from the spec formulas, NOT from the generated sig
// functions (which use u128 masking gymnastics) - agreement on random traces
// is what validates the two against each other.

/// 43: `window_bound : always (next_seq - acked <= 4)` - bit<4> wraparound.
fn b43_window_bound(s: &Sample) -> bool {
    ((s.vars[0].wrapping_sub(s.vars[1])) & 0xF) <= 4
}

/// 44: `holder_no_request : always (req & (1 << holder) == 0)`.
fn b44_holder_no_request(s: &Sample) -> bool {
    s.vars[1] & (1u64 << s.vars[0]) == 0
}
/// 44: `req_granted_1_bounded : always (req & 2 != 0 -> bounded_eventually[25] (req & 2 == 0))`.
fn t44_req1_set(s: &Sample) -> bool {
    s.vars[1] & 2 != 0
}
fn b44_req1_clear(s: &Sample) -> bool {
    s.vars[1] & 2 == 0
}

/// 45: `idle_no_local_call : always (state == Idle -> up_calls & (1 << pos) == 0 && down_calls & (1 << pos) == 0)`.
fn b45_idle_no_local_call(s: &Sample) -> bool {
    s.state != "Idle"
        || (s.vars[1] & (1u64 << s.vars[0])) == 0 && (s.vars[2] & (1u64 << s.vars[0])) == 0
}

struct CTable {
    labels: &'static str,
    bodies: &'static str,
    triggers: &'static str,
    kinds: &'static str,
    bounds: &'static str,
}

struct SpecDef {
    file: &'static str,
    tag: &'static str,
    props: Vec<RefProp>,
    c_bodies: &'static str,
    c_table: CTable,
    crafted: Vec<(String, String)>,
}

fn spec_defs() -> Vec<SpecDef> {
    vec![
        SpecDef {
            file: "examples/valid/43_sliding_window.rfs",
            tag: "monref_43",
            props: vec![RefProp::Always {
                label: "window_bound",
                body: b43_window_bound,
            }],
            c_bodies: r#"static int b43_window_bound(const mr_sample_t* s) {
    return (((uint8_t)(s->vars[0] - s->vars[1])) & 0xF) <= 4;
}
"#,
            c_table: CTable {
                labels: r#""window_bound""#,
                bodies: "b43_window_bound",
                triggers: "0",
                kinds: "KIND_ALWAYS",
                bounds: "0",
            },
            crafted: vec![(
                "boundary_wraparound_and_sticky".to_string(),
                // s0: 15-15=0 OK; s1: (0-15) mod 16 = 1 <= 4 OK (a
                // non-wrapping reference would FAIL here); s2: 0-1 mod 16 =
                // 15 > 4 -> first confirmation; s3: recovery, sticky -> no new.
                "Active|15,15\nActive|0,15\nActive|0,1\nIdle|5,5\n".to_string(),
            )],
        },
        SpecDef {
            file: "examples/valid/44_token_ring.rfs",
            tag: "monref_44",
            props: vec![
                RefProp::Always {
                    label: "holder_no_request",
                    body: b44_holder_no_request,
                },
                RefProp::ReactiveBoundedEventually {
                    label: "req_granted_1_bounded",
                    trigger: t44_req1_set,
                    body: b44_req1_clear,
                    bound: 25,
                },
            ],
            c_bodies: r#"static int b44_holder_no_request(const mr_sample_t* s) {
    return (s->vars[1] & (1ULL << s->vars[0])) == 0;
}
static int t44_req1_set(const mr_sample_t* s) { return (s->vars[1] & 2) != 0; }
static int b44_req1_clear(const mr_sample_t* s) { return (s->vars[1] & 2) == 0; }
"#,
            c_table: CTable {
                labels: r#""holder_no_request", "req_granted_1_bounded""#,
                bodies: "b44_holder_no_request, b44_req1_clear",
                triggers: "0, t44_req1_set",
                kinds: "KIND_ALWAYS, KIND_OBLIGATION",
                bounds: "0, 25",
            },
            crafted: vec![
                (
                    // req bit 1 set from s0 (obligation pending, deadline
                    // s0+24 = s24); at s24 the holder bit is also set so
                    // holder_no_request confirms on the SAME sample - the M-line
                    // must name the earlier-declared property.
                    "obligation_deadline_tie_break".to_string(),
                    format!(
                        "{}\nRunning|1,2,5\nRunning|1,2,6\n",
                        vec!["Running|0,2,0"; 24].join("\n")
                    ),
                ),
                (
                    // Same countdown but holder stays clear: the obligation
                    // alone confirms at s24 with trig=0.
                    "obligation_deadline_alone".to_string(),
                    format!(
                        "{}\nRunning|0,2,5\nRunning|0,0,6\n",
                        vec!["Running|0,2,0"; 24].join("\n")
                    ),
                ),
                (
                    // Clear before the deadline (s1), re-trigger at s2: fresh
                    // countdown with trig=2, confirmed at s2+24 = s26.
                    "retrigger_after_clear".to_string(),
                    format!(
                        "Running|0,2,0\nRunning|0,0,1\n{}\n",
                        (0..25)
                            .map(|i| format!("Running|0,2,{}", i + 2))
                            .collect::<Vec<_>>()
                            .join("\n")
                    ),
                ),
                (
                    // holder_no_request fails at sample 0 itself.
                    "hnr_violation_at_init".to_string(),
                    "Running|3,8,0\nRunning|3,14,1\n".to_string(),
                ),
            ],
        },
        SpecDef {
            file: "examples/valid/45_elevator.rfs",
            tag: "monref_45",
            props: vec![RefProp::Always {
                label: "idle_no_local_call",
                body: b45_idle_no_local_call,
            }],
            c_bodies: r#"static int b45_idle_no_local_call(const mr_sample_t* s) {
    return strcmp(s->state, "Idle") != 0
        || ((s->vars[1] & (1ULL << s->vars[0])) == 0 && (s->vars[2] & (1ULL << s->vars[0])) == 0);
}
"#,
            c_table: CTable {
                labels: r#""idle_no_local_call""#,
                bodies: "b45_idle_no_local_call",
                triggers: "0",
                kinds: "KIND_ALWAYS",
                bounds: "0",
            },
            crafted: vec![(
                // s0 Up (antecedent false), s1 Idle with non-local calls,
                // s2 Down, s3 Idle with a LOCAL up-call -> first confirmation,
                // s4 recovery, sticky.
                "local_call_while_idle".to_string(),
                "Up|2,4,8\nIdle|1,4,8\nDown|0,1,16\nIdle|0,1,16\nIdle|0,0,0\n".to_string(),
            )],
        },
    ]
}

// ---------------------------------------------------------------------------
// C reference: per-spec hand-written bodies + one generic engine, compiled
// with `gcc -Wall` (warning-free, same bar as the generated artifacts).

fn c_reference_source(def: &SpecDef) -> String {
    format!(
        r#"/* Hand-written reference monitors for {tag} (Phase 6 task 4, action 4).
 * Independent implementation of SEMANTICS.md 5.2-5.5 over raw sample lines;
 * bodies transcribed directly from the spec's monitorable formulas. */
#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <string.h>

typedef struct {{ char state[128]; unsigned long long vars[64]; int nv; }} mr_sample_t;

{c_bodies}
enum {{ KIND_ALWAYS, KIND_OBLIGATION }};
#define NPROPS {nprops}
static const char* LABELS[NPROPS] = {{{labels}}};
static int (*BODIES[NPROPS])(const mr_sample_t*) = {{{bodies}}};
static int (*TRIGGERS[NPROPS])(const mr_sample_t*) = {{{triggers}}};
static const int KINDS[NPROPS] = {{{kinds}}};
static const long long BOUNDS[NPROPS] = {{{bounds}}};

static int failed[NPROPS];
static int pending[NPROPS];
static long long remaining[NPROPS];
static long long trig_step[NPROPS];

int main(void) {{
    char line[8192];
    int k = 0;
    int first_set = 0;
    const char* f_label = "";
    int f_step = 0;
    long long f_trig = -1;
    while (fgets(line, sizeof line, stdin)) {{
        size_t len = strlen(line);
        while (len > 0 && (line[len - 1] == '\n' || line[len - 1] == '\r')) line[--len] = 0;
        if (len == 0) continue;
        mr_sample_t s;
        memset(&s, 0, sizeof s);
        char* bar = strchr(line, '|');
        if (bar) *bar = 0;
        size_t slen = strlen(line);
        if (slen >= sizeof s.state) {{ fprintf(stderr, "state name too long\n"); abort(); }}
        memcpy(s.state, line, slen + 1);
        if (bar) {{
            char* p = bar + 1;
            char* tok = strtok(p, ",");
            while (tok && s.nv < 64) {{ s.vars[s.nv++] = strtoull(tok, 0, 10); tok = strtok(0, ","); }}
        }}
        int newly = -1;
        for (int pi = 0; pi < NPROPS; pi++) {{
            if (failed[pi]) continue;
            if (KINDS[pi] == KIND_ALWAYS) {{
                if (!BODIES[pi](&s)) {{ failed[pi] = 1; trig_step[pi] = -1; if (newly < 0) newly = pi; }}
            }} else {{
                int a = TRIGGERS[pi](&s);
                int b = BODIES[pi](&s);
                if (!pending[pi] && a) {{ pending[pi] = 1; remaining[pi] = BOUNDS[pi]; trig_step[pi] = k; }}
                if (pending[pi]) {{
                    if (b) {{ pending[pi] = 0; }}
                    else {{
                        remaining[pi]--;
                        if (remaining[pi] == 0) {{ failed[pi] = 1; if (newly < 0) newly = pi; }}
                    }}
                }}
            }}
        }}
        if (newly >= 0) {{
            printf("M%d FAIL %s step=%d trig=%lld\n", k, LABELS[newly], k, trig_step[newly]);
            if (!first_set) {{ first_set = 1; f_label = LABELS[newly]; f_step = k; f_trig = trig_step[newly]; }}
        }} else {{
            printf("M%d OK\n", k);
        }}
        k++;
    }}
    if (first_set) printf("FIRST %s step=%d trig=%lld\n", f_label, f_step, f_trig);
    else printf("FIRST none\n");
    return 0;
}}
"#,
        tag = def.tag,
        c_bodies = def.c_bodies,
        nprops = def.props.len(),
        labels = def.c_table.labels,
        bodies = def.c_table.bodies,
        triggers = def.c_table.triggers,
        kinds = def.c_table.kinds,
        bounds = def.c_table.bounds,
    )
}

/// Serializes C-reference compilation: both tests build the same per-spec
/// binary, and a concurrent gcc rewrite of a just-executed file yields
/// EACCES/EAGAIN races.
static REF_COMPILE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn run_c_reference(def: &SpecDef, script: &str) -> String {
    let dir = scratch().join(def.tag);
    std::fs::create_dir_all(&dir).expect("create ref dir");
    let src = dir.join(format!("{tag}_ref.c", tag = def.tag));
    let bin = dir.join(format!("{tag}_ref_c", tag = def.tag));
    let _g = REF_COMPILE_LOCK.lock().unwrap();
    {
        std::fs::write(&src, c_reference_source(def)).expect("write C reference");
        let out = Command::new(prod_driver::GCC)
            .arg("-Wall")
            .arg(&src)
            .arg("-o")
            .arg(&bin)
            .output()
            .expect("run gcc on C reference");
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            out.status.success(),
            "gcc failed on {}:\n{stderr}",
            src.display()
        );
        assert!(
            !stderr.lines().any(|l| l.contains("warning:")),
            "C reference has warnings (must be -Wall clean):\n{stderr}"
        );
        prod_driver::run_driver(&bin, script).1
    }
}

fn load_module(file: &str) -> rfs::Module {
    let spec = root().join(file);
    let src = std::fs::read_to_string(&spec).unwrap_or_else(|e| panic!("read {:?}: {e}", spec));
    let ir = rfs::parse_source(&src).unwrap_or_else(|e| panic!("{file} must parse: {e}"));
    ir.modules[0].clone()
}

// ---------------------------------------------------------------------------
// Tests.

#[test]
fn monitor_references_match_generated_random_traces() {
    for def in spec_defs() {
        let module = load_module(def.file);
        // 1) Generated monitors: action-3 input-level driver on the seeded script.
        let (rust_bin, _c_bin) =
            prod_driver::build_drivers(&format!("{tag}rand", tag = def.tag), &module, &scratch());
        let script = prod_driver::input_script(&module, TRACE_SEED, TRACE_STEPS);
        let (ec, out) = prod_driver::run_driver(&rust_bin, &script);
        assert_eq!(ec, 0, "[{}] random driver exited {ec}\n{out}", def.tag);

        // 2) Parse the recorded trace back out of the same run.
        let samples = parse_trace_from_driver_output(&out);
        assert!(
            samples.len() >= 5,
            "[{}] random trace degenerate ({} samples)",
            def.tag,
            samples.len()
        );

        // 3) Rust reference over the parsed trace. The probe's M-lines step
        // ALL recorded samples (incl. ASSUME phantoms); the FSM's sticky
        // FIRST record only sees live-stepped ones - compare each against its
        // generated counterpart.
        let gen_m: Vec<String> = out
            .lines()
            .filter(|l| l.starts_with('M'))
            .map(String::from)
            .collect();
        let gen_first = out
            .lines()
            .find(|l| l.starts_with("FIRST"))
            .expect("FIRST line")
            .to_string();
        let (m_rust, _first_probe) = ref_run(&def.props, &samples);
        assert_eq!(
            m_rust,
            gen_m,
            "[{}] random: Rust reference M-lines diverge from generated\n[ref]\n{}\n[gen]\n{}",
            def.tag,
            m_rust.join("\n"),
            gen_m.join("\n")
        );
        let first_live = m_rust
            .iter()
            .enumerate()
            .find(|(k, l)| samples[*k].live && l.contains(" FAIL "))
            .and_then(|(_, l)| m_line_to_first(l))
            .unwrap_or_else(|| "FIRST none".to_string());
        assert_eq!(
            gen_first, first_live,
            "[{}] random: FSM sticky FIRST diverges from reference live-first\n[gen] {gen_first}\n[ref] {first_live}",
            def.tag
        );

        // 4) C reference over the same trace (probe semantics).
        let c_ref = run_c_reference(&def, &render_samples(&samples));
        let c_m: Vec<String> = c_ref
            .lines()
            .filter(|l| l.starts_with('M'))
            .map(String::from)
            .collect();
        assert_eq!(
            c_m,
            gen_m,
            "[{}] random: C reference M-lines diverge from generated\n[c-ref]\n{}\n[gen]\n{}",
            def.tag,
            c_m.join("\n"),
            gen_m.join("\n")
        );
    }
}

#[test]
fn monitor_references_match_generated_crafted_scenarios() {
    for def in spec_defs() {
        let module = load_module(def.file);
        // Generated sample-level drivers (both backends), built once per spec.
        let (rs_bin, c_bin) = prod_driver::build_sample_drivers(
            &format!("{tag}craft", tag = def.tag),
            &module,
            &scratch(),
        );

        for (name, script) in &def.crafted {
            let gen_rs = verdict_block(&prod_driver::run_driver(&rs_bin, script).1);
            let gen_c = verdict_block(&prod_driver::run_driver(&c_bin, script).1);
            assert_eq!(
                gen_rs, gen_c,
                "[{}/{}] generated backends diverge on crafted samples\n[rs]\n{gen_rs}\n[c]\n{gen_c}",
                def.tag, name
            );

            // Rust reference.
            let samples = parse_samples(script);
            let (m_rust, first_rust) = ref_run(&def.props, &samples);
            let rust_ref = block_of(&m_rust, first_rust);
            assert_eq!(
                rust_ref, gen_rs,
                "[{}/{}] Rust reference diverges from generated monitors\n[ref]\n{rust_ref}\n[gen]\n{gen_rs}",
                def.tag, name
            );

            // C reference.
            let c_ref = verdict_block(&run_c_reference(&def, script));
            assert_eq!(
                c_ref, gen_rs,
                "[{}/{}] C reference diverges from generated monitors\n[c-ref]\n{c_ref}\n[gen]\n{gen_rs}",
                def.tag, name
            );
        }
    }
}
