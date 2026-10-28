//! C backend: production FSM source generation.
//!
//! Renders the module into a self-contained C translation unit: state/input/
//! output types, vars struct, trace recording, online property and assumption
//! monitors (rendered from the shared `MonitorPlan` of `crate::monitor` -
//! identical semantics to the Rust backend), assumption handling (function
//! pointer mirroring the Rust `AssumptionHandler`), and the priority-ordered
//! `fsm_step`. Simultaneous variable updates use temporary next variables,
//! exactly like the Rust backend (DESIGN.md section 19).
//! Docs: dev:backend-c-overview, llm:backend-c

use crate::backends::c::{c_binding_ident, c_ident, c_type, int_c_literal, mask_c_expr};
use crate::codegen::{self, BackendOptions, PropertyMonitorMode};
use crate::codegen_fsm::{self, FsmVariants, VariantInfo};
use crate::ir::*;
use crate::monitor::{self, AnchorRange, MonNode, MonitorPlan, SignalAtom, SignalExpr};
use crate::validator::{self, Env, ExprType};
use std::fmt::Write as _;

/// Entry point for the `production` command: production FSM code without
/// tests. Property monitors are emitted only with `--with-property-monitors`
/// (D-P3-6), gated behind `#ifdef RFS_PROPERTY_MONITORS`.
pub fn generate_production(
    module: &Module,
    options: &BackendOptions,
) -> Result<crate::codegen::CodegenOutput, Vec<String>> {
    let mode = if options.with_property_monitors {
        PropertyMonitorMode::Gated
    } else {
        PropertyMonitorMode::Off
    };
    generate_core(module, options, mode)
}

/// Core generator shared by all three commands (D-P3-6): `production` maps
/// the CLI flag to Off/Gated; `test` and `mc` always pass Always.
pub(crate) fn generate_core(
    module: &Module,
    options: &BackendOptions,
    prop_mode: PropertyMonitorMode,
) -> Result<crate::codegen::CodegenOutput, Vec<String>> {
    let mut warnings = Vec::new();
    let gen = Gen::new(module, options, prop_mode, &mut warnings)?;
    Ok(crate::codegen::CodegenOutput {
        code: gen.code,
        warnings,
        extra_files: Vec::new(),
    })
}

struct Gen<'a> {
    module: &'a Module,
    options: &'a BackendOptions,
    variants: FsmVariants,
    prop_mode: PropertyMonitorMode,
    /// Compiled assumption monitors (selected via `--assumes`), in declaration order.
    assume_plans: Vec<MonitorPlan>,
    /// Compiled property monitors, in declaration order.
    prop_plans: Vec<MonitorPlan>,
    active_assumes: Vec<String>,
    property_names: Vec<String>,
    code: String,
}

/// Render a per-sample signal expression against `s: const rfs_sample_t*`,
/// referencing pattern helpers named `{prefix}_in_<i>` / `{prefix}_out_<i>`.
/// Shared by the monitor renderer (prefix `mon_<m>`), the liveness sample
/// evaluator and the generated invariant tests (Phase 5 task 6).
pub(crate) fn render_signal_expr_c(
    module: &Module,
    sig: &SignalExpr,
    prefix: &str,
    input_pats: &[&EventPattern],
    output_pats: &[&OutputPattern],
) -> Result<String, String> {
    match sig {
        SignalExpr::Atom(SignalAtom::True) => Ok("(true)".into()),
        SignalExpr::Atom(SignalAtom::VarExpr(expr)) => {
            let env = Env::for_module(module);
            let mut ctx = Ctx::default();
            for v in &module.vars {
                ctx.names
                    .insert(v.name.clone(), format!("s->vars.{}", c_ident(&v.name)));
            }
            ctx.state_variants = module
                .enums
                .iter()
                .find(|e| e.name == "State")
                .map(|e| e.variants.iter().cloned().collect())
                .unwrap_or_default();
            ctx.names.insert("state".into(), "s->to_state".into());
            gen_expr_c(expr, &env, &ctx, module).map(|e| format!("({e})"))
        }
        SignalExpr::Atom(SignalAtom::InputIs(p)) => {
            let idx = input_pats
                .iter()
                .position(|x| *x == p)
                .ok_or("unresolvable input pattern")?;
            Ok(format!("{prefix}_in_{idx}(s)"))
        }
        SignalExpr::Atom(SignalAtom::OutputHas(p)) => {
            let idx = output_pats
                .iter()
                .position(|x| *x == p)
                .ok_or("unresolvable output pattern")?;
            Ok(format!("{prefix}_out_{idx}(s)"))
        }
        SignalExpr::Not(inner) => Ok(format!(
            "(!({}))",
            render_signal_expr_c(module, inner, prefix, input_pats, output_pats)?
        )),
        SignalExpr::And(l, r) => Ok(format!(
            "({}) && ({})",
            render_signal_expr_c(module, l, prefix, input_pats, output_pats)?,
            render_signal_expr_c(module, r, prefix, input_pats, output_pats)?
        )),
        SignalExpr::Or(l, r) => Ok(format!(
            "({}) || ({})",
            render_signal_expr_c(module, l, prefix, input_pats, output_pats)?,
            render_signal_expr_c(module, r, prefix, input_pats, output_pats)?
        )),
    }
}

/// Helper: does the sample input match this pattern? (`NULL` input - sample 0
/// - never matches.) Emits a `static inline bool {prefix}_in_<idx>` function.
pub(crate) fn input_is_sample_helper_c(
    prefix: &str,
    idx: usize,
    pattern: &EventPattern,
    variants: &FsmVariants,
) -> Option<String> {
    let cond = input_match_cond_c(variants, pattern)?;
    Some(format!(
        "static inline bool {prefix}_in_{idx}(const rfs_sample_t* s) {{ if (!s->input) return false; const input_t* inp = s->input; return {cond}; }}\n"
    ))
}

/// Helper: does the sample contain an output matching this pattern? Emits a
/// `static inline bool {prefix}_out_<idx>` function.
pub(crate) fn output_has_sample_helper_c(
    prefix: &str,
    idx: usize,
    pattern: &OutputPattern,
    variants: &FsmVariants,
) -> Option<String> {
    let cond = output_match_cond_c(variants, pattern)?;
    Some(format!(
        "static inline bool {prefix}_out_{idx}(const rfs_sample_t* s) {{ for (int i = 0; i < s->n_outputs; i++) {{ const output_t* o = &s->outputs[i]; if ({cond}) return true; }} return false; }}\n"
    ))
}

/// C condition for an input matching a pattern (over `inp`).
pub(crate) fn input_match_cond_c(variants: &FsmVariants, pattern: &EventPattern) -> Option<String> {
    match &pattern.variant {
        None => {
            let variants: Vec<&VariantInfo> = variants
                .inputs
                .iter()
                .filter(|v| v.group == pattern.group)
                .collect();
            if variants.is_empty() {
                return None;
            }
            let arms: Vec<String> = variants
                .iter()
                .map(|v| format!("inp->kind == INPUT_{}", v.unique_name.to_uppercase()))
                .collect();
            Some(arms.join(" || "))
        }
        Some(PatternVariant::Unit { name }) => {
            let v = variants.input_variant(&pattern.group, name)?;
            if !v.fields.is_empty() {
                return None;
            }
            Some(format!(
                "inp->kind == INPUT_{}",
                v.unique_name.to_uppercase()
            ))
        }
        Some(PatternVariant::Struct { name, fields }) => {
            let v = variants.input_variant(&pattern.group, name)?;
            if v.fields.len() != fields.len() {
                return None;
            }
            let pl = format!("inp->payload.input_{}", v.unique_name.to_lowercase());
            let mut guards: Vec<String> = vec![format!(
                "inp->kind == INPUT_{}",
                v.unique_name.to_uppercase()
            )];
            for (i, pf) in fields.iter().enumerate() {
                let fname = c_ident(&v.fields[i].0);
                match &pf.pattern {
                    PatternExpr::Int { value } => guards.push(format!(
                        "{}.{} == {}",
                        pl,
                        fname,
                        int_c_literal(*value, None)
                    )),
                    PatternExpr::Bool { value } => guards.push(format!(
                        "{}.{} == {}",
                        pl,
                        fname,
                        if *value { "true" } else { "false" }
                    )),
                    PatternExpr::EnumVariant { path } => {
                        let base = c_ident(&path[0]);
                        let rest = path[1..]
                            .iter()
                            .map(|x| c_ident(x).to_uppercase())
                            .collect::<Vec<_>>()
                            .join("_");
                        guards.push(format!(
                            "{}.{} == {}_{}",
                            pl,
                            fname,
                            base.to_uppercase(),
                            rest
                        ));
                    }
                    _ => {} // wildcard / bind: no constraint
                }
            }
            Some(guards.join(" && "))
        }
    }
}

/// C condition for an output matching a pattern (over `o`).
pub(crate) fn output_match_cond_c(
    variants: &FsmVariants,
    pattern: &OutputPattern,
) -> Option<String> {
    match &pattern.variant {
        None => {
            let variants: Vec<&VariantInfo> = variants
                .outputs
                .iter()
                .filter(|v| v.group == pattern.group)
                .collect();
            if variants.is_empty() {
                return None;
            }
            let arms: Vec<String> = variants
                .iter()
                .map(|v| format!("o->kind == OUTPUT_{}", v.unique_name.to_uppercase()))
                .collect();
            Some(arms.join(" || "))
        }
        Some(PatternVariant::Unit { name }) => {
            let v = variants.output_variant(&pattern.group, name)?;
            if !v.fields.is_empty() {
                return None;
            }
            Some(format!(
                "o->kind == OUTPUT_{}",
                v.unique_name.to_uppercase()
            ))
        }
        Some(PatternVariant::Struct { name, fields }) => {
            let v = variants.output_variant(&pattern.group, name)?;
            if v.fields.len() != fields.len() {
                return None;
            }
            let pl = format!("o->payload.output_{}", v.unique_name.to_lowercase());
            let mut guards: Vec<String> = vec![format!(
                "o->kind == OUTPUT_{}",
                v.unique_name.to_uppercase()
            )];
            for (i, pf) in fields.iter().enumerate() {
                let fname = c_ident(&v.fields[i].0);
                match &pf.pattern {
                    PatternExpr::Int { value } => guards.push(format!(
                        "{}.{} == {}",
                        pl,
                        fname,
                        int_c_literal(*value, None)
                    )),
                    PatternExpr::Bool { value } => guards.push(format!(
                        "{}.{} == {}",
                        pl,
                        fname,
                        if *value { "true" } else { "false" }
                    )),
                    PatternExpr::EnumVariant { path } => {
                        let base = c_ident(&path[0]);
                        let rest = path[1..]
                            .iter()
                            .map(|x| c_ident(x).to_uppercase())
                            .collect::<Vec<_>>()
                            .join("_");
                        guards.push(format!(
                            "{}.{} == {}_{}",
                            pl,
                            fname,
                            base.to_uppercase(),
                            rest
                        ));
                    }
                    _ => {}
                }
            }
            Some(guards.join(" && "))
        }
    }
}

impl<'a> Gen<'a> {
    fn new(
        module: &'a Module,
        options: &'a BackendOptions,
        prop_mode: PropertyMonitorMode,
        warnings: &mut Vec<String>,
    ) -> Result<Self, Vec<String>> {
        // Reject unsupported bit widths up front (decision D8).
        let mut bad = Vec::new();
        let mut check_ty = |ty: &Type| match ty {
            Type::Bit { width } => {
                if !matches!(width, 0..=8 | 16 | 32 | 64) {
                    bad.push(format!("bit<{}>", width));
                }
            }
            _ => {}
        };
        for v in &module.vars {
            check_ty(&v.ty);
        }
        for g in module.events.iter().chain(module.outputs.iter()) {
            for v in &g.variants {
                if let ConstructorVariant::Struct { fields, .. } = v {
                    for f in fields {
                        check_ty(&f.ty);
                    }
                }
            }
        }
        if !bad.is_empty() {
            return Err(vec![format!(
                "C backend supports bit widths 8/16/32/64; unsupported: {}",
                bad.join(", ")
            )]);
        }

        let mut g = Gen {
            module,
            options,
            variants: codegen_fsm::build_variants(module),
            prop_mode,
            assume_plans: Vec::new(),
            prop_plans: Vec::new(),
            property_names: Vec::new(),
            active_assumes: Vec::new(),
            code: String::new(),
        };
        g.compile_monitors(warnings);
        g.emit_header();
        g.emit_types();
        g.emit_monitors().map_err(|e| vec![e])?;
        g.emit_fsm_impl().map_err(|e| vec![e])?;
        Ok(g)
    }

    /// Whether property-monitor machinery is emitted at all (D-P3-6).
    fn has_prop(&self) -> bool {
        self.prop_mode != PropertyMonitorMode::Off
    }

    /// Wrap a chunk of generated C in the property-monitor guard. In Gated
    /// mode the code sits behind `#ifdef RFS_PROPERTY_MONITORS` (enabled by
    /// the `--with-property-monitors` header define or `-D` on the command
    /// line); Always/Off emit it bare.
    fn prop_wrap(&self, code: &str) -> String {
        match self.prop_mode {
            PropertyMonitorMode::Gated => {
                // The directive lines must stand alone: start on a fresh line
                // (call sites may be mid-line) and end one before the next.
                format!("\n#ifdef RFS_PROPERTY_MONITORS\n{code}\n#endif\n")
            }
            _ => code.to_string(),
        }
    }

    /// Property-monitor-only generated C: present in Gated (behind the
    /// `#ifdef`) and Always modes, absent in Off.
    fn prop_emit(&self, code: &str) -> String {
        if self.has_prop() {
            self.prop_wrap(code)
        } else {
            String::new()
        }
    }

    fn emit_header(&mut self) {
        // Line 1 is the v1.0 unified artifact header (Phase 6, D-P6-3): same
        // text as the Rust backend's line 1 (`//` is a valid C comment), so
        // artifacts are recognizable and drift is detectable in both.
        let _ = writeln!(
            self.code,
            "// Generated by RFS v{} | spec sha256:{} | module {}\n\
             #include <stdint.h>\n#include <stdbool.h>\n#include <string.h>\n#include <stdio.h>\n#include <stdlib.h>",
            env!("CARGO_PKG_VERSION"),
            self.options.spec_hash,
            self.module.name
        );
        // --no-std (Phase 5 task 9): accepted for CLI symmetry with the Rust
        // backend; plain C is already dependency-free, so it is a no-op here.
        if self.options.no_std {
            let _ = writeln!(
                self.code,
                "// --no-std accepted: the C backend is already dependency-free; the flag has no effect on this output."
            );
        }
        if let Some(v) = &self.module.version {
            let _ = writeln!(self.code, "#define RFS_SPEC_VERSION \"{}\"", v);
        }
        // --with-property-monitors (D-P3-6): enable the #ifdef-guarded
        // property-monitor code in this translation unit.
        if self.prop_mode == PropertyMonitorMode::Gated {
            let _ = writeln!(self.code, "#define RFS_PROPERTY_MONITORS 1");
        }
    }

    fn emit_types(&mut self) {
        let m = self.module;
        // Non-State enums first (vars/fields may reference them).
        for e in &m.enums {
            if e.name == "State" {
                continue;
            }
            let et = format!("{}_t", c_ident(&e.name));
            let _ = writeln!(self.code, "typedef enum {{");
            for (i, v) in e.variants.iter().enumerate() {
                let sep = if i + 1 < e.variants.len() { "," } else { "" };
                let _ = writeln!(
                    self.code,
                    "    {}_{}{}",
                    c_ident(&e.name).to_uppercase(),
                    c_ident(v).to_uppercase(),
                    sep
                );
            }
            let _ = writeln!(self.code, "}} {};", et);
        }
        // State enum.
        if let Some(se) = m.enums.iter().find(|e| e.name == "State") {
            let _ = writeln!(self.code, "typedef enum {{");
            for (i, v) in se.variants.iter().enumerate() {
                let sep = if i + 1 < se.variants.len() { "," } else { "" };
                let _ = writeln!(self.code, "    STATE_{}{}", c_ident(v).to_uppercase(), sep);
            }
            let _ = writeln!(self.code, "}} state_t;");
        } else {
            let _ = writeln!(self.code, "typedef int state_t; // no State enum declared");
        }

        // Vars struct.
        if m.vars.is_empty() {
            let _ = writeln!(self.code, "typedef struct {{ int _unused; }} vars_t;");
        } else {
            let _ = writeln!(self.code, "typedef struct {{");
            for v in &m.vars {
                let ty = c_type(&v.ty).unwrap_or_else(|_| "uint64_t".into());
                let _ = writeln!(self.code, "    {} {};", ty, c_ident(&v.name));
            }
            let _ = writeln!(self.code, "}} vars_t;");
        }

        // Input/output payloads + tagged unions (clone to avoid borrow clash).
        let inputs = self.variants.inputs.clone();
        let outputs = self.variants.outputs.clone();
        self.emit_tagged("input", &inputs);
        self.emit_tagged("output", &outputs);

        let max_outputs = m
            .transitions
            .iter()
            .map(|t| t.emits.len())
            .chain(m.on_error.iter().map(|o| o.emits.len()))
            .max()
            .unwrap_or(0)
            .max(1);
        let _ = writeln!(self.code, "#define MAX_OUTPUTS {}", max_outputs);

        // Trace.
        let _ = writeln!(
            self.code,
            "typedef struct {{ input_t input; output_t outputs[MAX_OUTPUTS]; int n_outputs; state_t from; state_t to; vars_t vars; }} trace_step_t;\n\
             typedef struct {{ state_t initial_state; vars_t initial_vars; trace_step_t* steps; int len; int cap; }} rfs_trace_t;"
        );

        // Status / policies / handler (mirrors the Rust StepError + AssumptionHandler).
        let _ = writeln!(
            self.code,
            "typedef enum {{ RFS_OK, RFS_UNEXPECTED_EVENT, RFS_ASSUMPTION_VIOLATION, RFS_PROPERTY_VIOLATION }} rfs_status_t;\n\
             typedef struct {{ rfs_status_t status; output_t outputs[MAX_OUTPUTS]; int n_outputs; char assume_label[64]; char prop_label[64]; int prop_step; int prop_trigger_step; }} step_result_t;\n\
             static inline step_result_t rfs_make_result(rfs_status_t status) {{ step_result_t r; memset(&r, 0, sizeof(r)); r.status = status; return r; }}\n\
             typedef enum {{ RFS_CONTINUE, RFS_STOP, RFS_PANIC }} rfs_policy_t;\n\
             typedef rfs_policy_t (*rfs_assumption_handler)(void* user_data, const char* label);\n\
             static rfs_policy_t rfs_panic_handler(void* user_data, const char* label) {{ (void)user_data; fprintf(stderr, \"RFS assumption violated: %s\\n\", label); abort(); }}"
        );

        // Trace helpers.
        let _ = writeln!(
            self.code,
            "static inline state_t trace_state(const rfs_trace_t* t, int pos) {{ return pos == 0 ? t->initial_state : t->steps[pos - 1].to; }}\n\
             static inline vars_t trace_vars(const rfs_trace_t* t, int pos) {{ return pos == 0 ? t->initial_vars : t->steps[pos - 1].vars; }}\n\
             static void trace_push(rfs_trace_t* t, const trace_step_t* s) {{ if (t->len == t->cap) {{ t->cap = t->cap == 0 ? 8 : t->cap * 2; t->steps = (trace_step_t*)realloc(t->steps, (size_t)t->cap * sizeof(trace_step_t)); }} t->steps[t->len++] = *s; }}"
        );
    }

    /// Emit a tagged-union type for inputs or outputs: payload structs, kind
    /// enum, and the union wrapper.
    fn emit_tagged(&mut self, kw: &str, variants: &[VariantInfo]) {
        let _ = writeln!(self.code, "// {} constructors", kw);
        for v in variants {
            if !v.fields.is_empty() {
                let _ = writeln!(self.code, "typedef struct {{");
                for (fname, fty) in &v.fields {
                    let ty = c_type(fty).unwrap_or_else(|_| "uint64_t".into());
                    let _ = writeln!(self.code, "    {} {};", ty, c_ident(fname));
                }
                let _ = writeln!(
                    self.code,
                    "}} {}_{}_payload_t;",
                    kw,
                    v.unique_name.to_lowercase()
                );
            }
        }
        if variants.is_empty() {
            let _ = writeln!(
                self.code,
                "typedef enum {{ {}_NONE }} {}_kind_t;",
                kw.to_uppercase(),
                kw
            );
            let _ = writeln!(
                self.code,
                "typedef struct {{ int kind; union {{ int _none; }} payload; }} {}_t;",
                kw
            );
            return;
        }
        let _ = writeln!(self.code, "typedef enum {{");
        for (i, v) in variants.iter().enumerate() {
            let sep = if i + 1 < variants.len() { "," } else { "" };
            let _ = writeln!(
                self.code,
                "    {}_{}{}",
                kw.to_uppercase(),
                v.unique_name.to_uppercase(),
                sep
            );
        }
        let _ = writeln!(self.code, "}} {}_kind_t;", kw);
        let _ = writeln!(self.code, "typedef struct {{ int kind; union {{");
        for v in variants {
            if v.fields.is_empty() {
                let _ = writeln!(
                    self.code,
                    "        int {}_{};",
                    kw,
                    v.unique_name.to_lowercase()
                );
            } else {
                let _ = writeln!(
                    self.code,
                    "        {}_{}_payload_t {}_{};",
                    kw,
                    v.unique_name.to_lowercase(),
                    kw,
                    v.unique_name.to_lowercase()
                );
            }
        }
        let _ = writeln!(self.code, "    }} payload; }} {}_t;", kw);
    }
}

// ---------------------------------------------------------------------------
// Expression lowering (C)
// ---------------------------------------------------------------------------

/// Name context for expression lowering.
#[derive(Default)]
struct Ctx {
    names: std::collections::HashMap<String, String>,
    state_variants: std::collections::HashSet<String>,
}

/// True iff `ident` occurs in `s` as a whole C identifier (word boundaries).
fn c_contains_ident(s: &str, ident: &str) -> bool {
    let b = s.as_bytes();
    let mut start = 0usize;
    while let Some(rel) = s[start..].find(ident) {
        let p = start + rel;
        let after = p + ident.len();
        let before_ok = p == 0 || !(b[p - 1].is_ascii_alphanumeric() || b[p - 1] == b'_');
        let after_ok = after >= b.len() || !(b[after].is_ascii_alphanumeric() || b[after] == b'_');
        if before_ok && after_ok {
            return true;
        }
        start = p + 1;
    }
    false
}

fn gen_expr_c(e: &Expr, env: &Env, ctx: &Ctx, module: &Module) -> Result<String, String> {
    let ty = validator::type_expr(e, env)?;
    let width = match &ty {
        ExprType::Bit(w) => Some(*w),
        _ => None,
    };
    let ctype = |t: &ExprType| -> String {
        match t {
            ExprType::Bool => "bool".into(),
            ExprType::Bit(w) => crate::backends::c::bit_c_type(*w)
                .unwrap_or("uint64_t")
                .into(),
            ExprType::IntLit => "uint64_t".into(),
            ExprType::Enum(name) => format!("{}_t", c_ident(name)),
        }
    };
    match e {
        Expr::Int { value } => Ok(int_c_literal(*value, width)),
        Expr::Bool { value } => Ok(if *value {
            "true".into()
        } else {
            "false".into()
        }),
        Expr::Path { path } => {
            if path.len() == 1 {
                let name = &path[0];
                if let Some(expr) = ctx.names.get(name) {
                    return Ok(expr.clone());
                }
                if ctx.state_variants.contains(name) {
                    return Ok(format!("STATE_{}", c_ident(name).to_uppercase()));
                }
                Err(format!("unknown identifier {}", name))
            } else {
                // Enum variant path: EnumName_VARIANTNAME
                let base = c_ident(&path[0]);
                let rest = path[1..]
                    .iter()
                    .map(|x| c_ident(x).to_uppercase())
                    .collect::<Vec<_>>()
                    .join("_");
                Ok(format!("{}_{}", base.to_uppercase(), rest))
            }
        }
        Expr::Unary { op, expr } => {
            let inner = gen_expr_c(expr, env, ctx, module)?;
            match op {
                UnaryOp::Not => Ok(format!("(!({}))", inner)),
                UnaryOp::BitNot => Ok(format!("(({})((~({}))))", ctype(&ty), inner)),
                UnaryOp::Neg => Ok(format!("(({})(0ULL - ({})))", ctype(&ty), inner)),
            }
        }
        Expr::Binary { op, left, right } => {
            let l = gen_expr_c(left, env, ctx, module)?;
            let r = gen_expr_c(right, env, ctx, module)?;
            match op {
                BinOp::Plus | BinOp::Minus | BinOp::Star => {
                    let sym = match op {
                        BinOp::Plus => "+",
                        BinOp::Minus => "-",
                        _ => "*",
                    };
                    // Phase 6 task 2a: multiplication computes in a
                    // uint64_t-wide intermediate so bit<16> products above
                    // INT_MAX are not signed overflow (UB); the low w bits are
                    // exactly RFS's modular result. `+`/`-` stay as-is (no
                    // overflow possible after promotion for width <= 16;
                    // unsigned wrap is defined for 32/64) - minimal byte churn.
                    let body = match (op, width) {
                        (BinOp::Star, Some(_)) => {
                            format!("((uint64_t)({}) * (uint64_t)({}))", l, r)
                        }
                        _ => format!("({}) {} ({})", l, sym, r),
                    };
                    if let Some(w) = width {
                        // bit<N> arithmetic is modular at N bits (parity with
                        // the Rust backend's wrapping ops): C types are wider
                        // than sub-byte RFS widths, so mask after the cast.
                        Ok(format!(
                            "({})(({}) & ({}))",
                            ctype(&ty),
                            body,
                            mask_c_expr(w)
                        ))
                    } else {
                        Ok(format!("({})", body))
                    }
                }
                BinOp::Eq | BinOp::NotEq | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
                    let sym = match op {
                        BinOp::Eq => "==",
                        BinOp::NotEq => "!=",
                        BinOp::Lt => "<",
                        BinOp::Le => "<=",
                        BinOp::Gt => ">",
                        _ => ">=",
                    };
                    Ok(format!("({} {} {})", l, sym, r))
                }
                BinOp::LogicAnd | BinOp::LogicOr => {
                    let sym = if matches!(op, BinOp::LogicAnd) {
                        "&&"
                    } else {
                        "||"
                    };
                    Ok(format!("({} {} {})", l, sym, r))
                }
                BinOp::BitAnd | BinOp::BitOr | BinOp::BitXor => {
                    let sym = match op {
                        BinOp::BitAnd => "&",
                        BinOp::BitOr => "|",
                        _ => "^",
                    };
                    let body = format!("({}) {} ({})", l, sym, r);
                    if width.is_some() {
                        Ok(format!(
                            "({})(({}) & ({}))",
                            ctype(&ty),
                            body,
                            mask_c_expr(width.unwrap())
                        ))
                    } else {
                        Ok(format!("({})", body))
                    }
                }
                BinOp::ShiftLeft | BinOp::ShiftRight => {
                    let sym = if matches!(op, BinOp::ShiftLeft) {
                        "<<"
                    } else {
                        ">>"
                    };
                    match width {
                        Some(w) => {
                            // Phase 6 task 2a: RFS shifts are (l op r) mod 2^w
                            // with `amount >= w -> 0` (docs/SEMANTICS.md
                            // section 2.2). The old rendering `(l) op
                            // (uint64_t)(r)` relied on C integer promotion and
                            // was UNDEFINED when the variable amount reached
                            // the promoted LHS width (>= 32 for bit<8..32>,
                            // >= 64 for bit<64>) - e.g. gcc wraps the count
                            // mod 32, diverging from Rust. The operands are
                            // widened to uint64_t (wide enough for every
                            // C-legal width 8/16/32/64) and a runtime guard
                            // implements `amount >= w -> 0`; one uniform form
                            // for literal and variable amounts, matching the
                            // Rust backend's `if amount >= width { 0 } else
                            // { ... }`.
                            Ok(format!(
                                "({})((((uint64_t)({})) < {} ? ((uint64_t)({}) {} (uint64_t)({})) : ((uint64_t)0)) & ({}))",
                                ctype(&ty),
                                r,
                                w,
                                l,
                                sym,
                                r,
                                mask_c_expr(w)
                            ))
                        }
                        None => Ok(format!("({} {} (uint64_t)({}))", l, sym, r)),
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Formula translation (C) - same translated safety subset as the Rust backend
// ---------------------------------------------------------------------------

impl<'a> Gen<'a> {
    // ------------------------------------------------------------------
    // Online monitors (Phase 3): rendered from the shared MonitorPlan
    // (`crate::monitor`) with identical semantics to the Rust backend.
    // ------------------------------------------------------------------

    /// Compile all property/assumption formulas into monitor plans. Formulas
    /// outside the monitor subset (R1-R4) are skipped with a warning - the
    /// same text in every backend (D-P3-5).
    fn compile_monitors(&mut self, warnings: &mut Vec<String>) {
        for p in &self.module.properties {
            match monitor::compile_formula(&p.name, false, &p.formula) {
                Ok(plan) => {
                    self.property_names.push(p.name.clone());
                    self.prop_plans.push(plan);
                }
                Err(_) => warnings.push(format!(
                    "property '{}' is outside the monitor subset; skipped",
                    p.name
                )),
            }
        }
        for a in &self.module.assumptions {
            let selected = matches!(self.options.assumptions, codegen::AssumptionSelection::All)
                || matches!(&self.options.assumptions, codegen::AssumptionSelection::Labels(l) if l.iter().any(|x| x == &a.name));
            if !selected {
                continue;
            }
            match monitor::compile_formula(&a.name, true, &a.formula) {
                Ok(plan) => {
                    self.active_assumes.push(a.name.clone());
                    self.assume_plans.push(plan);
                }
                Err(_) => warnings.push(format!(
                    "assume '{}' is outside the monitor subset; skipped",
                    a.name
                )),
            }
        }
    }

    fn emit_monitors(&mut self) -> Result<(), String> {
        // One sample of the trace - the unit the online monitors evaluate.
        // `input` is NULL for sample 0 (the initial configuration, so
        // `input_is` is false there).
        let _ = writeln!(
            self.code,
            "typedef struct {{ const input_t* input; const output_t* outputs; int n_outputs; state_t from_state; state_t to_state; vars_t vars; }} rfs_sample_t;"
        );
        // Per-label assumption monitor structs (declaration order).
        for plan in &self.assume_plans {
            self.code.push_str(&self.render_monitor(plan)?);
        }
        // Property-monitor machinery: violation record type, per-label
        // structs, and the aggregate - gated together in Gated mode so a
        // build without the debug switch carries nothing (D-P3-6).
        if self.has_prop() {
            let mut prop = String::new();
            let _ = writeln!(
                prop,
                "/// A property violation confirmed by the online monitors. `step` is the\n/// sample where the failure was confirmed; `trigger_step` is the sample that\n/// started the failing obligation (-1 for plain per-sample checks).\ntypedef struct {{ const char* label; int step; int trigger_step; }} rfs_property_violation_t;"
            );
            for plan in &self.prop_plans {
                prop.push_str(&self.render_monitor(plan)?);
            }
            prop.push_str(&self.render_prop_monitors());
            self.code.push_str(&self.prop_wrap(&prop));
        }
        // Aggregate assumption monitors (always on; the handler path enforces).
        if !self.assume_plans.is_empty() {
            self.code.push_str(&self.render_assume_monitors());
        } else {
            // Empty aggregate keeps the fsm_t shape and call sites uniform.
            let _ = writeln!(
                self.code,
                "typedef struct {{ int _unused; }} assume_monitors_t;\nstatic inline void assume_monitors_new(assume_monitors_t* m) {{ (void)m; }}\nstatic inline const char* assume_monitors_step(assume_monitors_t* m, const rfs_sample_t* s, int idx, const char** active_assumes, int n_active) {{ (void)m; (void)s; (void)idx; (void)active_assumes; (void)n_active; return 0; }}"
            );
        }
        // Label constants (kept for MC/test runtimes and user code; D-P3-4).
        let props: Vec<String> = if self.property_names.is_empty() {
            vec!["0".into()]
        } else {
            self.property_names
                .iter()
                .map(|s| format!("\"{}\"", s))
                .collect()
        };
        let assumes: Vec<String> = if self.active_assumes.is_empty() {
            vec!["0".into()]
        } else {
            self.active_assumes
                .iter()
                .map(|s| format!("\"{}\"", s))
                .collect()
        };
        // `RFS_UNUSED`: the label tables exist for MC/test runtimes and user
        // code (D-P3-4); a production TU without them referenced must not
        // warn under -Wall.
        let _ = writeln!(
            self.code,
            "#if defined(__GNUC__) || defined(__clang__)\n#define RFS_UNUSED __attribute__((unused))\n#else\n#define RFS_UNUSED\n#endif"
        );
        let _ = writeln!(
            self.code,
            "static const char* PROPERTY_LABELS[] RFS_UNUSED = {{{}}};",
            props.join(", ")
        );
        let _ = writeln!(
            self.code,
            "static const int N_PROPERTY_LABELS RFS_UNUSED = {};",
            self.property_names.len()
        );
        let _ = writeln!(
            self.code,
            "static const char* ASSUME_LABELS[] RFS_UNUSED = {{{}}};",
            assumes.join(", ")
        );
        let _ = writeln!(
            self.code,
            "static const int N_ASSUME_LABELS RFS_UNUSED = {};",
            self.active_assumes.len()
        );
        Ok(())
    }

    /// Render one monitor struct from a compiled plan. Nodes are numbered in
    /// DFS pre-order over the plan tree; the step body evaluates them in
    /// post-order (children before parents), mirroring the reference
    /// interpreter in `crate::monitor`.
    fn render_monitor(&self, plan: &MonitorPlan) -> Result<String, String> {
        let m = crate::backends::rust::snake(&plan.label);
        // Distinct InputIs/OutputHas patterns (first-seen DFS order) get one
        // helper function each.
        let mut input_pats: Vec<&EventPattern> = Vec::new();
        let mut output_pats: Vec<&OutputPattern> = Vec::new();
        collect_signal_patterns(&plan.root, &mut input_pats, &mut output_pats);

        let mut fields: Vec<String> = Vec::new();
        let mut inits: Vec<String> = Vec::new();
        let mut sigs: Vec<String> = Vec::new();
        let mut blocks: Vec<String> = Vec::new();
        let mut next_id = 0usize;
        self.render_node(
            &plan.root,
            &mut next_id,
            &mut fields,
            &mut inits,
            &mut sigs,
            &mut blocks,
            &m,
            &input_pats,
            &output_pats,
        )?;

        let kind = if plan.is_assumption {
            "assumption"
        } else {
            "property"
        };
        let mut out = String::new();
        for (i, p) in input_pats.iter().enumerate() {
            match self.input_is_sample_helper_c(&m, i, p) {
                Some(h) => out.push_str(&h),
                None => {
                    return Err(format!(
                        "monitor '{}': unresolvable input pattern",
                        plan.label
                    ))
                }
            }
        }
        for (i, p) in output_pats.iter().enumerate() {
            match self.output_has_sample_helper_c(&m, i, p) {
                Some(h) => out.push_str(&h),
                None => {
                    return Err(format!(
                        "monitor '{}': unresolvable output pattern",
                        plan.label
                    ))
                }
            }
        }
        let sig_fns: Vec<String> = sigs
            .iter()
            .enumerate()
            .map(|(i, e)| {
                format!(
                    "static inline bool mon_{m}_sig_{i}(const rfs_sample_t* s) {{ return {e}; }}\n"
                )
            })
            .collect();
        let _ = writeln!(
            out,
            "/// Online monitor for {kind} '{label}' (compiled from the RFS formula).\ntypedef struct {{\n{fields}\n}} mon_{m}_t;\nstatic inline void mon_{m}_new(mon_{m}_t* m) {{ {inits} }}\n{sigs}/// Step the monitor on sample `idx`. Returns true when the failure is\n/// newly confirmed at this sample.\nstatic inline bool mon_{m}_step(mon_{m}_t* m, const rfs_sample_t* s, int idx) {{\n    bool before = m->n0_failed;\n{blocks}\n    return m->n0_failed && !before;\n}}\n/// The sample that started the currently failing obligation (-1 for plain\n/// per-sample check failures).\nstatic inline int mon_{m}_trigger_step(const mon_{m}_t* m) {{ return m->n0_trigger_step; }}\n",
            label = plan.label,
            fields = fields.join("\n"),
            inits = inits.join(" "),
            sigs = sig_fns.join(""),
            blocks = blocks.join("\n"),
        );
        Ok(out)
    }

    fn render_node(
        &self,
        node: &MonNode,
        next_id: &mut usize,
        fields: &mut Vec<String>,
        inits: &mut Vec<String>,
        sigs: &mut Vec<String>,
        blocks: &mut Vec<String>,
        m: &str,
        input_pats: &[&EventPattern],
        output_pats: &[&OutputPattern],
    ) -> Result<(), String> {
        let id = *next_id;
        *next_id += 1;
        match node {
            MonNode::RangeCheck { expr, range } => {
                let sig_expr = self.gen_signal_c(expr, m, input_pats, output_pats)?;
                sigs.push(sig_expr.clone());
                fields.push(format!("    bool n{id}_failed;"));
                fields.push(format!("    int n{id}_trigger_step;"));
                fields.push(format!("    bool n{id}_now;"));
                inits.push(format!(
                    "m->n{id}_failed = false; m->n{id}_trigger_step = -1; m->n{id}_now = false;"
                ));
                let cond = Self::range_cond_c(range);
                // Per-sample check: `now` is recomputed every sample (the
                // child may recover); `failed` is sticky for reporting.
                blocks.push(format!(
                    "    m->n{id}_now = ({cond}) && !({sig_expr});\n    if (m->n{id}_now) {{ m->n{id}_failed = true; }}"
                ));
            }
            MonNode::Obligation {
                bound,
                trigger,
                body,
                range,
            } => {
                let tsig_expr = self.gen_signal_c(trigger, m, input_pats, output_pats)?;
                sigs.push(tsig_expr.clone());
                let bsig_expr = self.gen_signal_c(body, m, input_pats, output_pats)?;
                sigs.push(bsig_expr.clone());
                fields.push(format!("    bool n{id}_failed;"));
                fields.push(format!("    int n{id}_trigger_step;"));
                inits.push(format!(
                    "m->n{id}_failed = false; m->n{id}_trigger_step = -1;"
                ));
                if *bound >= 1 {
                    fields.push(format!("    bool n{id}_pending;"));
                    fields.push(format!("    long long n{id}_remaining;"));
                    inits.push(format!("m->n{id}_pending = false; m->n{id}_remaining = 0;"));
                }
                fields.push(format!("    bool n{id}_now;"));
                inits.push(format!("m->n{id}_now = false;"));
                let cond = Self::range_cond_c(range);
                if *bound == 0 {
                    // be[0]: the body must hold at the trigger sample itself.
                    blocks.push(format!(
                        "    if (!m->n{id}_failed && {cond} && ({tsig_expr}) && !({bsig_expr})) {{ m->n{id}_failed = true; m->n{id}_trigger_step = idx; }}\n    m->n{id}_now = m->n{id}_failed;"
                    ));
                } else {
                    // D-P3-3 pending model (the Rust backend renders).
                    blocks.push(format!(
                        "    if (!m->n{id}_failed) {{\n        bool a = {cond} && ({tsig_expr});\n        bool b = ({bsig_expr});\n        if (!m->n{id}_pending && a) {{ m->n{id}_pending = true; m->n{id}_remaining = {bound}; m->n{id}_trigger_step = idx; }}\n        if (m->n{id}_pending) {{\n            if (b) {{ m->n{id}_pending = false; }}\n            else {{ m->n{id}_remaining -= 1; if (m->n{id}_remaining == 0) {{ m->n{id}_failed = true; }} }}\n        }}\n    }}\n    // Obligation failures are terminal: once confirmed, the node is\n    // failing at every later sample.\n    m->n{id}_now = m->n{id}_failed;"
                    ));
                }
            }
            MonNode::DelayedCheck {
                antecedent,
                consequent,
                range,
            } => {
                let asig_expr = self.gen_signal_c(antecedent, m, input_pats, output_pats)?;
                sigs.push(asig_expr.clone());
                let csig_expr = self.gen_signal_c(consequent, m, input_pats, output_pats)?;
                sigs.push(csig_expr.clone());
                fields.push(format!("    bool n{id}_failed;"));
                fields.push(format!("    int n{id}_trigger_step;"));
                fields.push(format!("    bool n{id}_prev_antecedent;"));
                fields.push(format!("    bool n{id}_now;"));
                inits.push(format!(
                    "m->n{id}_failed = false; m->n{id}_trigger_step = -1; m->n{id}_prev_antecedent = false; m->n{id}_now = false;"
                ));
                let cond = Self::range_cond_c(range);
                // Delayed check: the antecedent held at the PREVIOUS sample, so
                // the consequent must hold now. Failure is terminal; there is
                // no trigger sample to report (trigger_step stays -1).
                blocks.push(format!(
                    "    if (!m->n{id}_failed && {cond} && m->n{id}_prev_antecedent && !({csig_expr})) {{ m->n{id}_failed = true; }}\n    m->n{id}_prev_antecedent = ({asig_expr});\n    // DelayedCheck failures are terminal: once confirmed, the node is\n    // failing at every later sample.\n    m->n{id}_now = m->n{id}_failed;"
                ));
            }
            MonNode::And(children) | MonNode::Or(children) => {
                let mut child_ids: Vec<usize> = Vec::new();
                for c in children {
                    child_ids.push(*next_id);
                    self.render_node(
                        c,
                        next_id,
                        fields,
                        inits,
                        sigs,
                        blocks,
                        m,
                        input_pats,
                        output_pats,
                    )?;
                }
                fields.push(format!("    bool n{id}_failed;"));
                fields.push(format!("    int n{id}_trigger_step;"));
                fields.push(format!("    bool n{id}_now;"));
                inits.push(format!(
                    "m->n{id}_failed = false; m->n{id}_trigger_step = -1; m->n{id}_now = false;"
                ));
                // Per-sample composition over the children's CURRENT failure
                // status (a disjunction fails only while ALL children fail at
                // this sample; a conjunction while ANY child fails).
                let join = if matches!(node, MonNode::And(_)) {
                    " || "
                } else {
                    " && "
                };
                let any_all = child_ids
                    .iter()
                    .map(|c| format!("m->n{c}_now"))
                    .collect::<Vec<_>>()
                    .join(join);
                let mins: Vec<String> = child_ids
                    .iter()
                    .map(|c| {
                        format!(
                            "        if (m->n{c}_now && m->n{c}_trigger_step >= 0) {{ if (ts < 0 || m->n{c}_trigger_step < ts) ts = m->n{c}_trigger_step; }}"
                        )
                    })
                    .collect();
                blocks.push(format!(
                    "    m->n{id}_now = ({any_all});\n    if (m->n{id}_now && !m->n{id}_failed) {{\n        m->n{id}_failed = true;\n        int ts = -1;\n{mins}\n        m->n{id}_trigger_step = ts;\n    }}",
                    mins = mins.join("\n")
                ));
            }
        }
        Ok(())
    }

    /// The `idx` condition for an anchor range ("true" when unbounded from 0).
    fn range_cond_c(range: &AnchorRange) -> String {
        if range.from == 0 && range.to.is_none() {
            return "true".into();
        }
        let mut c = format!("((long long)(idx) >= {})", range.from);
        if let Some(t) = range.to {
            c.push_str(&format!(" && ((long long)(idx) <= {})", t));
        }
        c
    }

    /// Render a per-sample signal expression against `s: const rfs_sample_t*`
    /// (monitor helpers named `mon_{m}_in_<i>` / `mon_{m}_out_<i>`).
    fn gen_signal_c(
        &self,
        sig: &SignalExpr,
        m: &str,
        input_pats: &[&EventPattern],
        output_pats: &[&OutputPattern],
    ) -> Result<String, String> {
        render_signal_expr_c(
            self.module,
            sig,
            &format!("mon_{m}"),
            input_pats,
            output_pats,
        )
    }

    /// Helper: does the sample input match this pattern? (`NULL` input -
    /// sample 0 - never matches.)
    fn input_is_sample_helper_c(
        &self,
        m: &str,
        idx: usize,
        pattern: &EventPattern,
    ) -> Option<String> {
        input_is_sample_helper_c(&format!("mon_{m}"), idx, pattern, &self.variants)
    }

    /// Helper: does the sample contain an output matching this pattern?
    fn output_has_sample_helper_c(
        &self,
        m: &str,
        idx: usize,
        pattern: &OutputPattern,
    ) -> Option<String> {
        output_has_sample_helper_c(&format!("mon_{m}"), idx, pattern, &self.variants)
    }

    /// Emit the `assume_monitors` aggregate (always present on `fsm_t`).
    fn render_assume_monitors(&self) -> String {
        let fields: Vec<String> = self
            .assume_plans
            .iter()
            .map(|p| {
                let s = crate::backends::rust::snake(&p.label);
                format!("mon_{s}_t mon_{s};")
            })
            .collect();
        let inits: Vec<String> = self
            .assume_plans
            .iter()
            .map(|p| {
                let s = crate::backends::rust::snake(&p.label);
                format!("mon_{s}_new(&m->mon_{s});")
            })
            .collect();
        let steps: Vec<String> = self.assume_plans.iter().map(|p| {
            let s = crate::backends::rust::snake(&p.label);
            format!(
                "    if (mon_{s}_step(&m->mon_{s}, s, idx) && !r) {{ for (int i = 0; i < n_active; i++) {{ if (strcmp(active_assumes[i], \"{}\") == 0) {{ r = \"{}\"; break; }} }} }}",
                p.label, p.label
            )
        }).collect();
        format!(
            "/// Aggregate state of all assumption monitors (stepped online in `rfs_commit`).\ntypedef struct {{ {fields} }} assume_monitors_t;\nstatic inline void assume_monitors_new(assume_monitors_t* m) {{ {inits} }}\n/// Step all assumption monitors on sample `idx`. Returns the label of a\n/// newly failed ACTIVE assumption (first in declaration order), or NULL.\nstatic inline const char* assume_monitors_step(assume_monitors_t* m, const rfs_sample_t* s, int idx, const char** active_assumes, int n_active) {{\n    const char* r = 0;\n{steps}\n    return r;\n}}\n",
            fields = fields.join(" "),
            inits = inits.join(" "),
            steps = steps.join("\n"),
        )
    }

    /// Emit the `prop_monitors` aggregate (only when property monitors are on).
    fn render_prop_monitors(&self) -> String {
        if self.prop_plans.is_empty() {
            return "typedef struct { int _unused; } prop_monitors_t;\nstatic inline void prop_monitors_new(prop_monitors_t* m) { (void)m; }\nstatic inline int prop_monitors_step(prop_monitors_t* m, const rfs_sample_t* s, int idx, rfs_property_violation_t* out) { (void)m; (void)s; (void)idx; (void)out; return 0; }\n".into();
        }
        let fields: Vec<String> = self
            .prop_plans
            .iter()
            .map(|p| {
                let s = crate::backends::rust::snake(&p.label);
                format!("mon_{s}_t mon_{s};")
            })
            .collect();
        let inits: Vec<String> = self
            .prop_plans
            .iter()
            .map(|p| {
                let s = crate::backends::rust::snake(&p.label);
                format!("mon_{s}_new(&m->mon_{s});")
            })
            .collect();
        let steps: Vec<String> = self.prop_plans.iter().map(|p| {
            let s = crate::backends::rust::snake(&p.label);
            format!(
                "    if (mon_{s}_step(&m->mon_{s}, s, idx) && !found) {{ out->label = \"{}\"; out->step = idx; out->trigger_step = mon_{s}_trigger_step(&m->mon_{s}); found = 1; }}",
                p.label
            )
        }).collect();
        format!(
            "/// Aggregate state of all property monitors (stepped online in `rfs_commit`).\ntypedef struct {{ {fields} }} prop_monitors_t;\nstatic inline void prop_monitors_new(prop_monitors_t* m) {{ {inits} }}\n/// Step all property monitors on sample `idx`. Returns 1 and fills `out`\n/// with the first newly failed monitor's violation (declaration order).\nstatic inline int prop_monitors_step(prop_monitors_t* m, const rfs_sample_t* s, int idx, rfs_property_violation_t* out) {{\n    int found = 0;\n{steps}\n    return found;\n}}\n",
            fields = fields.join(" "),
            inits = inits.join(" "),
            steps = steps.join("\n"),
        )
    }
}

impl<'a> Gen<'a> {
    /// Emit the fsm struct, constructor, commit_step, and priority-ordered step.
    fn emit_fsm_impl(&mut self) -> Result<(), String> {
        let m = self.module;
        let init_state = m.initial_state.clone().unwrap_or_else(|| "__None__".into());

        let prop_fields = if self.has_prop() {
            let f = "prop_monitors_t prop_monitors;\n    char prop_label[64];\n    int prop_step;\n    int prop_trigger_step;";
            match self.prop_mode {
                PropertyMonitorMode::Gated => {
                    format!("\n#ifdef RFS_PROPERTY_MONITORS\n    {f}\n#endif\n")
                }
                _ => format!(" {f}"),
            }
        } else {
            String::new()
        };
        let _ = writeln!(
            self.code,
            "typedef struct {{ state_t state; vars_t vars; rfs_trace_t trace; const char** active_assumes; int n_active_assumes; assume_monitors_t assume_monitors;{prop_fields} }} fsm_t;"
        );

        // Initial vars literal.
        let init_vars = if m.vars.is_empty() {
            "(vars_t){0}".to_string()
        } else {
            let env = Env::for_module(m);
            let mut ctx = Ctx::default();
            for v in &m.vars {
                ctx.names
                    .insert(v.name.clone(), format!("vars.{}", c_ident(&v.name)));
            }
            // Build via a temporary struct initializer using the init expressions.
            let fields: Vec<String> = m
                .vars
                .iter()
                .map(|v| {
                    gen_expr_c(&v.init, &env, &ctx, m)
                        .map(|e| format!(".{} = {}", c_ident(&v.name), e))
                        .or(Ok(format!(".{} = 0", c_ident(&v.name))))
                })
                .collect::<Result<Vec<String>, String>>()?;
            format!("(vars_t){{ {} }}", fields.join(", "))
        };

        let prop_init = self.prop_emit(
            "    prop_monitors_new(&f->prop_monitors);\n\
             \n    f->prop_label[0] = 0; f->prop_step = -1; f->prop_trigger_step = -1;",
        );
        let prop_sample0 = self.prop_emit(
            "    rfs_property_violation_t pv;\n\
             \n    if (prop_monitors_step(&f->prop_monitors, &sample, 0, &pv)) {\n\
             \n        strcpy(f->prop_label, pv.label); f->prop_step = pv.step; f->prop_trigger_step = pv.trigger_step;\n\
             \n    }",
        );
        let _ = writeln!(
            self.code,
            "void fsm_new(fsm_t* f) {{\n    f->state = STATE_{};\n    f->vars = {};\n    f->trace.initial_state = f->state; f->trace.initial_vars = f->vars; f->trace.steps = 0; f->trace.len = 0; f->trace.cap = 0;\n    f->n_active_assumes = N_ASSUME_LABELS;\n    f->active_assumes = (const char**)calloc((size_t)f->n_active_assumes, sizeof(char*));\n    for (int i = 0; i < f->n_active_assumes; i++) f->active_assumes[i] = ASSUME_LABELS[i];\n    assume_monitors_new(&f->assume_monitors);{prop_init}\n    // Sample 0 (the initial configuration): step the monitors so countdowns\n    // and one-shot obligations start correctly. An assumption violated at\n    // sample 0 aborts, as with the previous post-hoc check.\n    rfs_sample_t sample = {{ 0, 0, 0, f->state, f->state, f->vars }};\n    const char* al = assume_monitors_step(&f->assume_monitors, &sample, 0, f->active_assumes, f->n_active_assumes);\n    if (al) {{ fprintf(stderr, \"RFS assumption violated: %s\\n\", al); abort(); }}{prop_sample0}\n}}",
            c_ident(&init_state).to_uppercase(),
            init_vars
        );

        // Explicit lifecycle end (Phase 5 task 16 close-out fix): frees the two
        // per-fsm heap allocations (trace buffer + active_assumes array) so a
        // long-lived consumer - notably the libFuzzer target, which calls
        // fsm_new/fsm_step/fsm_free once per input - does not accumulate RSS
        // until libFuzzer's out-of-memory limit trips. Parity with Rust, where
        // the per-call Fsm is dropped at scope end. Short-lived consumers (the
        // unit-test and MC executables) may simply exit without calling it.
        let _ = writeln!(
            self.code,
            "void fsm_free(fsm_t* f) {{ free(f->trace.steps); f->trace.steps = 0; f->trace.len = 0; f->trace.cap = 0; free(f->active_assumes); f->active_assumes = 0; }}"
        );

        // rfs_commit: push the trace step, then step the online monitors -
        // assumption enforcement via the handler path and a sticky property
        // violation record (D-P3-4: post-hoc checks removed).
        let prop_check = self.prop_emit(
            "    rfs_property_violation_t pv;\n\
             \n    if (prop_monitors_step(&f->prop_monitors, &sample, idx, &pv) && f->prop_label[0] == 0) {\n\
             \n        strcpy(f->prop_label, pv.label); f->prop_step = pv.step; f->prop_trigger_step = pv.trigger_step;\n\
             \n        step_result_t r = rfs_make_result(RFS_PROPERTY_VIOLATION);\n\
             \n        strncpy(r.prop_label, pv.label, sizeof(r.prop_label) - 1); r.prop_label[sizeof(r.prop_label) - 1] = 0;\n\
             \n        r.prop_step = pv.step; r.prop_trigger_step = pv.trigger_step;\n\
             \n        return r;\n\
             \n    }",
        );
        let _ = writeln!(
            self.code,
            "static RFS_UNUSED step_result_t rfs_commit(fsm_t* f, const input_t* in, const output_t* outs, int n_out, state_t next_state, const vars_t* new_vars, rfs_assumption_handler handler, void* user) {{\n    trace_step_t s; s.input = *in; for (int i = 0; i < n_out && i < MAX_OUTPUTS; i++) s.outputs[i] = outs[i]; s.n_outputs = n_out; s.from = f->state; s.to = next_state; s.vars = *new_vars;\n    trace_push(&f->trace, &s);\n    int idx = f->trace.len;\n    rfs_sample_t sample = {{ in, outs, n_out, s.from, s.to, s.vars }};\n    const char* al = assume_monitors_step(&f->assume_monitors, &sample, idx, f->active_assumes, f->n_active_assumes);\n    if (al) {{\n        rfs_policy_t pol = handler(user, al);\n        if (pol == RFS_STOP) {{ step_result_t r = rfs_make_result(RFS_ASSUMPTION_VIOLATION); strncpy(r.assume_label, al, sizeof(r.assume_label) - 1); r.assume_label[sizeof(r.assume_label) - 1] = 0; return r; }}\n        if (pol == RFS_PANIC) {{ fprintf(stderr, \"RFS assumption violated: %s\\n\", al); abort(); }}\n    }}{prop_check}\n    f->state = next_state; f->vars = *new_vars;\n    step_result_t r = rfs_make_result(RFS_OK); for (int i = 0; i < n_out && i < MAX_OUTPUTS; i++) r.outputs[i] = outs[i]; r.n_outputs = n_out; return r;\n}}"
        );

        // step(): grouped by state, priority-ordered within a state (Phase 2).
        let _ = writeln!(
            self.code,
            "step_result_t fsm_step(fsm_t* f, input_t in, rfs_assumption_handler handler, void* user) {{\n    if (handler == 0) handler = rfs_panic_handler;"
        );

        let mut state_order: Vec<&str> = Vec::new();
        let mut by_state: std::collections::HashMap<&str, Vec<usize>> =
            std::collections::HashMap::new();
        for (idx, t) in m.transitions.iter().enumerate() {
            if !by_state.contains_key(t.state.as_str()) {
                state_order.push(t.state.as_str());
            }
            by_state.entry(t.state.as_str()).or_default().push(idx);
        }
        for state in &state_order {
            let mut idxs: Vec<usize> = by_state.get(state).unwrap().clone();
            idxs.sort_by_key(|&k| std::cmp::Reverse(m.transitions[k].priority.unwrap_or(0)));
            let _ = writeln!(
                self.code,
                "    if (f->state == STATE_{}) {{",
                c_ident(state).to_uppercase()
            );
            for idx in idxs {
                self.code
                    .push_str(&self.transition_block_c(&m.transitions[idx])?);
            }
            let _ = writeln!(self.code, "    }}");
        }

        if let Some(oe) = &m.on_error {
            // on_error fallback (Phase 2): no state check, no bindings.
            let mut ctx = Ctx::default();
            for v in &m.vars {
                ctx.names
                    .insert(v.name.clone(), format!("f->vars.{}", c_ident(&v.name)));
            }
            let env = Env::for_module(m);
            let mut outs: Vec<String> = Vec::new();
            for o in &oe.emits {
                outs.push(self.output_expr_c(o, &env, &ctx)?);
            }
            let _ = writeln!(
                self.code,
                "    // on_error fallback: no transition matched the input"
            );
            let _ = writeln!(self.code, "    {{");
            let _ = writeln!(
                self.code,
                "        output_t outs[MAX_OUTPUTS] = {{0}}; int n_out = {};",
                outs.len()
            );
            for (i, o) in outs.iter().enumerate() {
                let _ = writeln!(self.code, "        outs[{}] = {};", i, o);
            }
            let _ = writeln!(
                self.code,
                "        return rfs_commit(f, &in, outs, n_out, STATE_{}, &f->vars, handler, user);",
                c_ident(&oe.next_state).to_uppercase()
            );
            let _ = writeln!(self.code, "    }}");
        } else {
            let _ = writeln!(
                self.code,
                "    {{ return rfs_make_result(RFS_UNEXPECTED_EVENT); }}"
            );
        }
        let _ = writeln!(self.code, "}}");
        Ok(())
    }

    /// Render one transition as nested if-blocks: state (emitted by caller) ->
    /// pattern match (bindings become local variables) -> guard -> actions.
    fn transition_block_c(&self, t: &Transition) -> Result<String, String> {
        let m = self.module;
        let mut out = String::new();

        // Pattern: kind check + bindings as locals + field constraints.
        match &t.pattern.variant {
            None => {
                let variants: Vec<&VariantInfo> = self
                    .variants
                    .inputs
                    .iter()
                    .filter(|v| v.group == t.pattern.group)
                    .collect();
                if variants.is_empty() {
                    return Err(format!("unknown event group '{}'", t.pattern.group));
                }
                let arms: Vec<String> = variants
                    .iter()
                    .map(|v| format!("in.kind == INPUT_{}", v.unique_name.to_uppercase()))
                    .collect();
                let _ = writeln!(out, "        if ({}) {{", arms.join(" || "));
            }
            Some(PatternVariant::Unit { name }) => {
                let v = self
                    .variants
                    .input_variant(&t.pattern.group, name)
                    .ok_or_else(|| {
                        format!(
                            "unknown event variant '{}' in group '{}'",
                            name, t.pattern.group
                        )
                    })?;
                if !v.fields.is_empty() {
                    return Err("unit pattern on struct variant".into());
                }
                let _ = writeln!(
                    out,
                    "        if (in.kind == INPUT_{}) {{",
                    v.unique_name.to_uppercase()
                );
            }
            Some(PatternVariant::Struct { name, fields }) => {
                let v = self
                    .variants
                    .input_variant(&t.pattern.group, name)
                    .ok_or_else(|| {
                        format!(
                            "unknown event variant '{}' in group '{}'",
                            name, t.pattern.group
                        )
                    })?;
                if v.fields.len() != fields.len() {
                    return Err("pattern field count mismatch".into());
                }
                let pl = format!("in.payload.input_{}", v.unique_name.to_lowercase());
                let mut conds: Vec<String> =
                    vec![format!("in.kind == INPUT_{}", v.unique_name.to_uppercase())];
                for (i, pf) in fields.iter().enumerate() {
                    let fname = c_ident(&v.fields[i].0);
                    match &pf.pattern {
                        PatternExpr::Int { value } => conds.push(format!(
                            "{}.{} == {}",
                            pl,
                            fname,
                            int_c_literal(*value, None)
                        )),
                        PatternExpr::Bool { value } => conds.push(format!(
                            "{}.{} == {}",
                            pl,
                            fname,
                            if *value { "true" } else { "false" }
                        )),
                        PatternExpr::EnumVariant { path } => {
                            let base = c_ident(&path[0]);
                            let rest = path[1..]
                                .iter()
                                .map(|x| c_ident(x).to_uppercase())
                                .collect::<Vec<_>>()
                                .join("_");
                            conds.push(format!(
                                "{}.{} == {}_{}",
                                pl,
                                fname,
                                base.to_uppercase(),
                                rest
                            ));
                        }
                        _ => {}
                    }
                }
                let _ = writeln!(out, "        if ({}) {{", conds.join(" && "));
            }
        }

        // Bindings as local variables (in declared field order).
        let mut env = Env::for_module(m);
        let mut ctx = Ctx::default();
        for v in &m.vars {
            ctx.names
                .insert(v.name.clone(), format!("f->vars.{}", c_ident(&v.name)));
        }
        ctx.state_variants = m
            .enums
            .iter()
            .find(|e| e.name == "State")
            .map(|e| e.variants.iter().cloned().collect())
            .unwrap_or_default();
        ctx.names.insert("state".into(), "f->state".into());

        let mut bound_names: Vec<String> = Vec::new();
        if let Some(PatternVariant::Struct { name, fields, .. }) = &t.pattern.variant {
            let v = self
                .variants
                .input_variant(&t.pattern.group, name)
                .ok_or("pattern variant lookup")?;
            let pl = format!("in.payload.input_{}", v.unique_name.to_lowercase());
            // Reserved: identifiers this body uses unqualified inside the
            // binding scope (the commit call's `f` / `in` / `handler` /
            // `user`). A same-named binding would shadow them (see
            // c_binding_ident).
            let reserved: &[&str] = &["f", "in", "handler", "user"];
            let mut used_idents: std::collections::HashSet<String> =
                std::collections::HashSet::new();
            for (i, pf) in fields.iter().enumerate() {
                if let PatternExpr::Bind { name } = &pf.pattern {
                    let fname = c_ident(&v.fields[i].0);
                    let ty = c_type(&v.fields[i].1).map_err(|e| e.to_string())?;
                    let ident = c_binding_ident(name, reserved, &mut used_idents);
                    let _ = writeln!(out, "            {} {} = {}.{};", ty, ident, pl, fname);
                    bound_names.push(ident.clone());
                    env.bindings.insert(
                        name.clone(),
                        crate::validator::type_to_expr_type(&v.fields[i].1),
                    );
                    env.binding_idents.insert(name.clone(), ident);
                }
            }
        }
        for name in env.bindings.keys() {
            let ident = env
                .binding_idents
                .get(name)
                .cloned()
                .unwrap_or_else(|| c_ident(name));
            ctx.names.insert(name.clone(), ident);
        }

        // Pre-render every expression that may use the bindings, so a binding
        // that no guard/output/update references gets a `(void)` cast (the
        // local would otherwise trip gcc -Wall's unused-variable warning).
        let guard_str: Option<String> = t
            .guard
            .as_ref()
            .map(|g| gen_expr_c(g, &env, &ctx, m))
            .transpose()?;
        let outs: Vec<String> = t
            .emits
            .iter()
            .map(|o| self.output_expr_c(o, &env, &ctx))
            .collect::<Result<_, String>>()?;
        let assign_vals: Vec<String> = t
            .assignments
            .iter()
            .map(|a| gen_expr_c(&a.value, &env, &ctx, m))
            .collect::<Result<_, String>>()?;
        for n in &bound_names {
            let used = guard_str
                .as_deref()
                .map(|s| c_contains_ident(s, n))
                .unwrap_or(false)
                || outs.iter().any(|s| c_contains_ident(s, n))
                || assign_vals.iter().any(|s| c_contains_ident(s, n));
            if !used {
                let _ = writeln!(out, "            (void){n};");
            }
        }

        // Guard.
        if let Some(gexpr) = &guard_str {
            let _ = writeln!(out, "            if ({}) {{", gexpr);
        }

        // Actions: outputs, next state, simultaneous var updates via temps.
        let _ = writeln!(
            out,
            "            output_t outs[MAX_OUTPUTS] = {{0}}; int n_out = {};",
            outs.len()
        );
        for (i, o) in outs.iter().enumerate() {
            let _ = writeln!(out, "            outs[{}] = {};", i, o);
        }

        if m.vars.is_empty() {
            let _ = writeln!(
                out,
                "            return rfs_commit(f, &in, outs, n_out, STATE_{}, &f->vars, handler, user);",
                c_ident(&t.next_state).to_uppercase()
            );
        } else {
            // Simultaneous update: start from current vars, evaluate all RHS
            // into temps (old values only), then assign.
            let _ = writeln!(out, "            vars_t tmp_vars = f->vars;");
            for (a, val) in t.assignments.iter().zip(assign_vals.iter()) {
                let ty = c_type(
                    &m.vars
                        .iter()
                        .find(|v| v.name == a.var)
                        .map(|v| &v.ty)
                        .unwrap_or(&Type::Bool),
                )
                .map_err(|e| e.to_string())?;
                let _ = writeln!(out, "            {} tmp_{} = {};", ty, c_ident(&a.var), val);
            }
            for a in &t.assignments {
                let _ = writeln!(
                    out,
                    "            tmp_vars.{} = tmp_{};",
                    c_ident(&a.var),
                    c_ident(&a.var)
                );
            }
            let _ = writeln!(
                out,
                "            return rfs_commit(f, &in, outs, n_out, STATE_{}, &tmp_vars, handler, user);",
                c_ident(&t.next_state).to_uppercase()
            );
        }

        if t.guard.is_some() {
            let _ = writeln!(out, "            }}");
        }
        let _ = writeln!(out, "        }}");
        Ok(out)
    }

    /// Render an output expression as a C initializer for `output_t`.
    fn output_expr_c(&self, o: &OutputExpr, env: &Env, ctx: &Ctx) -> Result<String, String> {
        let variant = match &o.variant {
            Some(v) => v.clone(),
            None => {
                let g = self
                    .module
                    .outputs
                    .iter()
                    .find(|g| g.name == o.group)
                    .ok_or("unknown output group")?;
                if g.variants.len() != 1 {
                    return Err("bare output ambiguous".into());
                }
                match &g.variants[0] {
                    ConstructorVariant::Unit { name } => name.clone(),
                    ConstructorVariant::Struct { name, .. } => name.clone(),
                }
            }
        };
        let v = self
            .variants
            .output_variant(&o.group, &variant)
            .ok_or("unknown output variant")?;
        if v.fields.is_empty() {
            return Ok(format!(
                "(output_t){{ .kind = OUTPUT_{} }}",
                v.unique_name.to_uppercase()
            ));
        }
        if v.fields.len() != o.args.len() {
            return Err(format!(
                "output {}({}) expects {} args, got {}",
                o.group,
                variant,
                v.fields.len(),
                o.args.len()
            ));
        }
        let pl = format!("payload.output_{}", v.unique_name.to_lowercase());
        let mut fields = vec![format!(".kind = OUTPUT_{}", v.unique_name.to_uppercase())];
        for (i, a) in o.args.iter().enumerate() {
            let val = gen_expr_c(a, env, ctx, self.module)?;
            fields.push(format!(".{}.{} = {}", pl, c_ident(&v.fields[i].0), val));
        }
        Ok(format!("(output_t){{ {} }}", fields.join(", ")))
    }
}

// ---------------------------------------------------------------------------
// Monitor-plan signal collection (shared by the C monitor rendering)
// ---------------------------------------------------------------------------

/// Collect distinct InputIs/OutputHas patterns from a plan tree, in first-
/// seen DFS order (used to number the per-monitor helper functions).
fn collect_signal_patterns<'p>(
    node: &'p MonNode,
    input_pats: &mut Vec<&'p EventPattern>,
    output_pats: &mut Vec<&'p OutputPattern>,
) {
    fn push<'p>(
        sig: &'p SignalExpr,
        input_pats: &mut Vec<&'p EventPattern>,
        output_pats: &mut Vec<&'p OutputPattern>,
    ) {
        let mut stack = vec![sig];
        while let Some(s) = stack.pop() {
            if let SignalExpr::Atom(SignalAtom::InputIs(p)) = s {
                if !input_pats.iter().any(|x| *x == p) {
                    input_pats.push(p);
                }
            } else if let SignalExpr::Atom(SignalAtom::OutputHas(p)) = s {
                if !output_pats.iter().any(|x| *x == p) {
                    output_pats.push(p);
                }
            }
            match s {
                SignalExpr::Not(inner) => stack.push(inner),
                SignalExpr::And(l, r) | SignalExpr::Or(l, r) => {
                    stack.push(r);
                    stack.push(l);
                }
                _ => {}
            }
        }
    }
    match node {
        MonNode::DelayedCheck {
            antecedent,
            consequent,
            ..
        } => {
            push(antecedent, input_pats, output_pats);
            push(consequent, input_pats, output_pats);
        }
        MonNode::RangeCheck { expr, .. } => push(expr, input_pats, output_pats),
        MonNode::Obligation { trigger, body, .. } => {
            push(trigger, input_pats, output_pats);
            push(body, input_pats, output_pats);
        }
        MonNode::And(children) | MonNode::Or(children) => {
            for c in children {
                collect_signal_patterns(c, input_pats, output_pats);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Exact MC runtime support (Phase 4, D-P4-11)
// ---------------------------------------------------------------------------

/// Render the `mc_step_all` function used by the exact C MC runtime: for
/// `(state, input)`, the effects of EVERY enabled transition in firing order
/// (deterministic mode uses only the first effect; nondet mode explores all).
/// The on_error fallback is appended when no transition matched. Rendered from
/// the same IR with the same helpers as `fsm_step`, so both paths agree by
/// construction.
pub(crate) fn render_mc_step_all_c(module: &Module) -> Result<String, String> {
    let options = BackendOptions::default();
    let mut warnings = Vec::new();
    let gen = Gen::new(module, &options, PropertyMonitorMode::Off, &mut warnings)
        .map_err(|e| e.join("; "))?;
    gen.render_mc_step_all_body()
}

/// Collect the distinct input/output patterns of a signal in first-appearance
/// order (for helper emission + `gen_signal_c` index resolution).
pub(crate) fn collect_liv_signal_patterns<'a>(
    sig: &'a crate::monitor::SignalExpr,
) -> (
    Vec<&'a crate::ir::EventPattern>,
    Vec<&'a crate::ir::OutputPattern>,
) {
    let mut input_pats: Vec<&'a crate::ir::EventPattern> = Vec::new();
    let mut output_pats: Vec<&'a crate::ir::OutputPattern> = Vec::new();
    fn walk<'a>(
        sig: &'a crate::monitor::SignalExpr,
        input_pats: &mut Vec<&'a crate::ir::EventPattern>,
        output_pats: &mut Vec<&'a crate::ir::OutputPattern>,
    ) {
        match sig {
            crate::monitor::SignalExpr::Atom(crate::monitor::SignalAtom::InputIs(p)) => {
                if !input_pats.contains(&p) {
                    input_pats.push(p);
                }
            }
            crate::monitor::SignalExpr::Atom(crate::monitor::SignalAtom::OutputHas(p)) => {
                if !output_pats.contains(&p) {
                    output_pats.push(p);
                }
            }
            crate::monitor::SignalExpr::Not(inner) => walk(inner, input_pats, output_pats),
            crate::monitor::SignalExpr::And(l, r) | crate::monitor::SignalExpr::Or(l, r) => {
                walk(l, input_pats, output_pats);
                walk(r, input_pats, output_pats);
            }
            _ => {}
        }
    }
    walk(sig, &mut input_pats, &mut output_pats);
    (input_pats, output_pats)
}

/// Render a per-sample boolean evaluator for a temporal-operator-free formula
/// (Phase 4 task 7 liveness leaves; C mirror of rust_fsm's
/// `render_sample_formula_evaluator`). Emits unique input/output pattern
/// helpers plus `static int {name}(const rfs_sample_t* s)` returning 0/1.
pub(crate) fn render_liv_sample_evaluator_c(
    module: &Module,
    name: &str,
    formula: &Formula,
) -> Result<String, String> {
    let signal = crate::monitor::formula_to_signal(formula)
        .ok_or_else(|| "liveness subformula contains temporal operators".to_string())?;
    let options = BackendOptions::default();
    let mut warnings = Vec::new();
    let gen = Gen::new(module, &options, PropertyMonitorMode::Off, &mut warnings)
        .map_err(|e| e.join("; "))?;
    let (input_pats, output_pats) = collect_liv_signal_patterns(&signal);
    let mut out = String::new();
    for (i, p) in input_pats.iter().enumerate() {
        out.push_str(
            &gen.input_is_sample_helper_c(name, i, p)
                .ok_or("unresolvable input pattern")?,
        );
    }
    for (i, p) in output_pats.iter().enumerate() {
        out.push_str(
            &gen.output_has_sample_helper_c(name, i, p)
                .ok_or("unresolvable output pattern")?,
        );
    }
    let expr = gen.gen_signal_c(&signal, name, &input_pats, &output_pats)?;
    out.push_str(&format!(
        "static int {name}(const rfs_sample_t* s) {{ return {expr}; }}\n"
    ));
    Ok(out)
}

impl<'a> Gen<'a> {
    /// Pattern condition for `mc_step_all` (mirrors the pattern part of
    /// `transition_block_c`; `in` is a local parameter there).
    fn mc_pattern_cond(&self, t: &Transition) -> Result<String, String> {
        match &t.pattern.variant {
            None => {
                let variants: Vec<&VariantInfo> = self
                    .variants
                    .inputs
                    .iter()
                    .filter(|v| v.group == t.pattern.group)
                    .collect();
                if variants.is_empty() {
                    return Err(format!("unknown event group '{}'", t.pattern.group));
                }
                let arms: Vec<String> = variants
                    .iter()
                    .map(|v| format!("in.kind == INPUT_{}", v.unique_name.to_uppercase()))
                    .collect();
                Ok(arms.join(" || "))
            }
            Some(PatternVariant::Unit { name }) => {
                let v = self
                    .variants
                    .input_variant(&t.pattern.group, name)
                    .ok_or_else(|| {
                        format!(
                            "unknown event variant '{}' in group '{}'",
                            name, t.pattern.group
                        )
                    })?;
                if !v.fields.is_empty() {
                    return Err("unit pattern on struct variant".into());
                }
                Ok(format!("in.kind == INPUT_{}", v.unique_name.to_uppercase()))
            }
            Some(PatternVariant::Struct { name, fields }) => {
                let v = self
                    .variants
                    .input_variant(&t.pattern.group, name)
                    .ok_or_else(|| {
                        format!(
                            "unknown event variant '{}' in group '{}'",
                            name, t.pattern.group
                        )
                    })?;
                if v.fields.len() != fields.len() {
                    return Err("pattern field count mismatch".into());
                }
                let pl = format!("in.payload.input_{}", v.unique_name.to_lowercase());
                let mut conds: Vec<String> =
                    vec![format!("in.kind == INPUT_{}", v.unique_name.to_uppercase())];
                for (i, pf) in fields.iter().enumerate() {
                    let fname = c_ident(&v.fields[i].0);
                    match &pf.pattern {
                        PatternExpr::Int { value } => conds.push(format!(
                            "{}.{} == {}",
                            pl,
                            fname,
                            int_c_literal(*value, None)
                        )),
                        PatternExpr::Bool { value } => conds.push(format!(
                            "{}.{} == {}",
                            pl,
                            fname,
                            if *value { "true" } else { "false" }
                        )),
                        PatternExpr::EnumVariant { path } => {
                            let base = c_ident(&path[0]);
                            let rest = path[1..]
                                .iter()
                                .map(|x| c_ident(x).to_uppercase())
                                .collect::<Vec<_>>()
                                .join("_");
                            conds.push(format!(
                                "{}.{} == {}_{}",
                                pl,
                                fname,
                                base.to_uppercase(),
                                rest
                            ));
                        }
                        _ => {}
                    }
                }
                Ok(conds.join(" && "))
            }
        }
    }

    /// Render the full `mc_step_all` C function (see [`render_mc_step_all_c`]).
    fn render_mc_step_all_body(&self) -> Result<String, String> {
        let m = self.module;
        let mut out = String::new();
        out.push_str(
            "typedef struct { state_t next_state; vars_t next_vars; output_t outputs[MAX_OUTPUTS]; int n_outputs; int transition_id; } mc_effect_t;\n",
        );
        out.push_str("static int mc_step_all(state_t state, const vars_t* vars, input_t in, mc_effect_t* out) {\n    int n = 0;\n");

        // Per-state firing order (identical to emit_fsm_impl).
        let mut state_order: Vec<&str> = Vec::new();
        let mut by_state: std::collections::HashMap<&str, Vec<usize>> =
            std::collections::HashMap::new();
        for (idx, t) in m.transitions.iter().enumerate() {
            if !by_state.contains_key(t.state.as_str()) {
                state_order.push(t.state.as_str());
            }
            by_state.entry(t.state.as_str()).or_default().push(idx);
        }
        for st in &state_order {
            let mut idxs: Vec<usize> = by_state.get(st).unwrap().clone();
            idxs.sort_by_key(|&k| std::cmp::Reverse(m.transitions[k].priority.unwrap_or(0)));
            out.push_str(&format!(
                "    if (state == STATE_{}) {{\n",
                c_ident(st).to_uppercase()
            ));
            for idx in idxs {
                let t = &m.transitions[idx];
                let cond = self.mc_pattern_cond(t)?;
                out.push_str(&format!("        if ({}) {{\n", cond));

                // Bindings as locals (same order/semantics as transition_block_c).
                let mut env = Env::for_module(m);
                let mut ctx = Ctx::default();
                for v in &m.vars {
                    ctx.names
                        .insert(v.name.clone(), format!("vars->{}", c_ident(&v.name)));
                }
                ctx.state_variants = m
                    .enums
                    .iter()
                    .find(|e| e.name == "State")
                    .map(|e| e.variants.iter().cloned().collect())
                    .unwrap_or_default();
                ctx.names.insert("state".into(), "state".into());
                let mut bound_names: Vec<String> = Vec::new();
                if let Some(PatternVariant::Struct { name, fields, .. }) = &t.pattern.variant {
                    let v = self
                        .variants
                        .input_variant(&t.pattern.group, name)
                        .ok_or("pattern variant lookup")?;
                    let pl = format!("in.payload.input_{}", v.unique_name.to_lowercase());
                    // Reserved: identifiers mc_step_all uses unqualified inside
                    // the binding scope - the guard's `state` / `vars`, and the
                    // effect body's array `out` + counter `n`. A same-named
                    // binding would shadow them (see c_binding_ident).
                    let reserved: &[&str] = &["state", "vars", "out", "n"];
                    let mut used_idents: std::collections::HashSet<String> =
                        std::collections::HashSet::new();
                    for (i, pf) in fields.iter().enumerate() {
                        if let PatternExpr::Bind { name } = &pf.pattern {
                            let fname = c_ident(&v.fields[i].0);
                            let ty = c_type(&v.fields[i].1).map_err(|e| e.to_string())?;
                            let ident = c_binding_ident(name, reserved, &mut used_idents);
                            out.push_str(&format!(
                                "            {} {} = {}.{};\n",
                                ty, ident, pl, fname
                            ));
                            bound_names.push(ident.clone());
                            env.bindings.insert(
                                name.clone(),
                                crate::validator::type_to_expr_type(&v.fields[i].1),
                            );
                            env.binding_idents.insert(name.clone(), ident);
                        }
                    }
                }
                for name in env.bindings.keys() {
                    let ident = env
                        .binding_idents
                        .get(name)
                        .cloned()
                        .unwrap_or_else(|| c_ident(name));
                    ctx.names.insert(name.clone(), ident);
                }

                // Pre-render every expression that may use the bindings, so a
                // binding no guard/output/update references gets a `(void)`
                // cast (otherwise gcc -Wall warns about the unused local).
                let guard_str: Option<String> = t
                    .guard
                    .as_ref()
                    .map(|g| gen_expr_c(g, &env, &ctx, m))
                    .transpose()?;
                let outs: Vec<String> = t
                    .emits
                    .iter()
                    .map(|o| self.output_expr_c(o, &env, &ctx))
                    .collect::<Result<_, _>>()?;
                let assign_vals: Vec<String> = t
                    .assignments
                    .iter()
                    .map(|a| gen_expr_c(&a.value, &env, &ctx, m))
                    .collect::<Result<_, _>>()?;
                for n in &bound_names {
                    let used = guard_str
                        .as_deref()
                        .map(|s| c_contains_ident(s, n))
                        .unwrap_or(false)
                        || outs.iter().any(|s| c_contains_ident(s, n))
                        || assign_vals.iter().any(|s| c_contains_ident(s, n));
                    if !used {
                        out.push_str(&format!("            (void){n};\n"));
                    }
                }

                // Guard.
                if let Some(gexpr) = &guard_str {
                    out.push_str(&format!("            if ({}) {{\n", gexpr));
                }

                // Effect body (no commit: collect into `out[n]`).
                out.push_str("            {\n");
                out.push_str("                output_t outs[MAX_OUTPUTS];\n");
                out.push_str(&format!("                int n_out = {};\n", outs.len()));
                for (i, o) in outs.iter().enumerate() {
                    out.push_str(&format!("                outs[{}] = {};\n", i, o));
                }
                out.push_str(&format!(
                    "                state_t next_state = STATE_{};\n",
                    c_ident(&t.next_state).to_uppercase()
                ));
                if m.vars.is_empty() {
                    out.push_str("                vars_t new_vars = *vars;\n");
                } else {
                    out.push_str("                vars_t new_vars = *vars;\n");
                    for (a, val) in t.assignments.iter().zip(assign_vals.iter()) {
                        let ty = c_type(
                            m.vars
                                .iter()
                                .find(|v| v.name == a.var)
                                .map(|v| &v.ty)
                                .unwrap_or(&Type::Bool),
                        )
                        .map_err(|e| e.to_string())?;
                        out.push_str(&format!(
                            "                {} tmp_{} = {};\n",
                            ty,
                            c_ident(&a.var),
                            val
                        ));
                    }
                    for a in &t.assignments {
                        out.push_str(&format!(
                            "                new_vars.{} = tmp_{};\n",
                            c_ident(&a.var),
                            c_ident(&a.var)
                        ));
                    }
                }
                out.push_str("                out[n].next_state = next_state;\n");
                out.push_str("                out[n].next_vars = new_vars;\n");
                out.push_str(
                    "                for (int i = 0; i < n_out && i < MAX_OUTPUTS; i++) out[n].outputs[i] = outs[i];\n",
                );
                out.push_str("                out[n].n_outputs = n_out;\n");
                out.push_str(&format!(
                    "                out[n].transition_id = {};\n",
                    idx
                ));
                out.push_str("                n++;\n");
                out.push_str("            }\n");
                if t.guard.is_some() {
                    out.push_str("            }\n");
                }
                out.push_str("        }\n");
            }
            out.push_str("    }\n");
        }

        // on_error fallback: fires only when no transition matched the input.
        if let Some(oe) = &m.on_error {
            let env = Env::for_module(m);
            let mut ctx = Ctx::default();
            for v in &m.vars {
                ctx.names
                    .insert(v.name.clone(), format!("vars->{}", c_ident(&v.name)));
            }
            ctx.state_variants = m
                .enums
                .iter()
                .find(|e| e.name == "State")
                .map(|e| e.variants.iter().cloned().collect())
                .unwrap_or_default();
            ctx.names.insert("state".into(), "state".into());
            let outs: Vec<String> = oe
                .emits
                .iter()
                .map(|o| self.output_expr_c(o, &env, &ctx))
                .collect::<Result<_, _>>()?;
            out.push_str("    if (n == 0) {\n");
            out.push_str("        output_t outs[MAX_OUTPUTS];\n");
            out.push_str(&format!("        int n_out = {};\n", outs.len()));
            for (i, o) in outs.iter().enumerate() {
                out.push_str(&format!("        outs[{}] = {};\n", i, o));
            }
            out.push_str(&format!(
                "        out[n].next_state = STATE_{};\n",
                c_ident(&oe.next_state).to_uppercase()
            ));
            out.push_str("        out[n].next_vars = *vars;\n");
            out.push_str(
                "        for (int i = 0; i < n_out && i < MAX_OUTPUTS; i++) out[n].outputs[i] = outs[i];\n",
            );
            out.push_str("        out[n].n_outputs = n_out;\n");
            out.push_str("        out[n].transition_id = -1; // on_error fallback\n");
            out.push_str("        n++;\n");
            out.push_str("    }\n");
        }

        if m.transitions.is_empty() && m.on_error.is_none() {
            out.push_str("    (void)state; (void)vars; (void)in; (void)out;\n");
        }
        out.push_str("    return n;\n}\n");
        Ok(out)
    }
}
