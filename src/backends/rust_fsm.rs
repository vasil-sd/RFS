//! Rust backend: production FSM source generation.
//!
//! Generates the standalone Rust source for one RFS module: type declarations
//! (enums, `Vars`, `Input`, `Output`), the trace/sample runtime, online
//! property and assumption monitors (rendered from the shared `MonitorPlan`
//! of `crate::monitor` - identical semantics to the C backend), and the
//! `Fsm` implementation with its transition blocks. See `backends/RUST.md`
//! for the generated-code anatomy and design decisions.

use crate::backends::rust::{
    bit_rust_type, mask_expr, representative_input_expr, rust_ident, rust_type, snake,
};
use crate::codegen::{AssumptionSelection, BackendOptions, CodegenOutput, PropertyMonitorMode};
use crate::codegen_fsm::{self, FsmVariants, VariantInfo};
use crate::ir::*;
use crate::monitor::{self, AnchorRange, MonNode, MonitorPlan, SignalAtom, SignalExpr};
use crate::validator::{self, Env, ExprType};
use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;

/// Entry point for the `production` command: production FSM code without
/// tests. Property monitors are emitted only with `--with-property-monitors`
/// (D-P3-6), gated behind `#[cfg(debug_assertions)]`.
pub fn generate_production(
    module: &Module,
    options: &BackendOptions,
) -> Result<CodegenOutput, Vec<String>> {
    let mode = if options.with_property_monitors {
        PropertyMonitorMode::Gated
    } else {
        PropertyMonitorMode::Off
    };
    generate_core(module, options, false, mode)
}

/// Core generator shared by all three commands: validates the module, then
/// emits the Rust FSM source. When `with_tests` is set, a unit-test module
/// is appended (see `crate::backends::rust_test`). `prop_mode` selects the
/// property-monitor activation (D-P3-6): production maps the CLI flag to
/// Off/Gated; test and mc always pass Always.
pub(crate) fn generate_core(
    module: &Module,
    options: &BackendOptions,
    with_tests: bool,
    prop_mode: PropertyMonitorMode,
) -> Result<CodegenOutput, Vec<String>> {
    let errors = validator::validate_module(module);
    if !errors.is_empty() {
        return Err(errors.into_iter().map(|e| e.message).collect());
    }
    let mut warnings = Vec::new();
    let gen = Gen::new(module, options, &mut warnings, with_tests, prop_mode)?;
    Ok(CodegenOutput {
        code: gen.code,
        warnings,
        extra_files: Vec::new(),
    })
}

struct Gen<'a> {
    module: &'a Module,
    options: &'a BackendOptions,
    code: String,
    variants: FsmVariants,
    prop_mode: PropertyMonitorMode,
    /// Compiled assumption monitors (selected via `--assumes`), in declaration order.
    assume_plans: Vec<MonitorPlan>,
    /// Compiled property monitors, in declaration order.
    prop_plans: Vec<MonitorPlan>,
    active_assumes: Vec<String>,
    property_names: Vec<String>,
    /// Whether any transition (or the on_error block) emits an output payload,
    /// i.e. whether the generated body contains the `vec![...]` macro. The
    /// no_std header imports the macro only in that case, so the file stays
    /// warning-free without widening the `#![allow(...)]` list.
    uses_vec_macro: bool,
}

impl<'a> Gen<'a> {
    fn new(
        module: &'a Module,
        options: &'a BackendOptions,
        warnings: &mut Vec<String>,
        with_tests: bool,
        prop_mode: PropertyMonitorMode,
    ) -> Result<Self, Vec<String>> {
        let mut g = Self {
            module,
            options,
            code: String::new(),
            variants: codegen_fsm::build_variants(module),
            prop_mode,
            assume_plans: vec![],
            prop_plans: vec![],
            active_assumes: vec![],
            property_names: vec![],
            uses_vec_macro: module.transitions.iter().any(|t| !t.emits.is_empty())
                || module
                    .on_error
                    .as_ref()
                    .is_some_and(|oe| !oe.emits.is_empty()),
        };
        g.compile_monitors(warnings);
        g.emit_prelude();
        g.emit_enums();
        g.emit_vars();
        g.emit_input_output();
        g.emit_renderers();
        g.emit_trace_and_handlers();
        g.emit_monitors().map_err(|e| vec![e])?;
        g.emit_fsm_impl().map_err(|e| vec![e])?;
        if g.options.with_protocol_fsm {
            g.emit_protocol_fsm_impl();
        }
        if with_tests {
            let tests = crate::backends::rust_test::emit_test_module(
                g.module,
                &g.variants,
                g.options.steps,
            )?;
            g.code.push_str(&tests);
        }
        Ok(g)
    }

    /// The opt-in `ProtocolFsm` impl (Phase 5 task 11, `--with-protocol-fsm`):
    /// links this artifact to the `rfs-adapters` crate - the one documented
    /// opt-in that connects generated code to an external crate. The inherent
    /// `Fsm` methods are called explicitly so the trait impl forwards instead
    /// of recursing (inherent methods shadow trait methods in path syntax).
    fn emit_protocol_fsm_impl(&mut self) {
        self.code.push_str(
            r#"
// ProtocolFsm adapter surface (opt-in via --with-protocol-fsm, Phase 5
// task 11): this is the one documented opt-in that links a generated
// artifact to an external crate (rfs-adapters).
use rfs_adapters::ProtocolFsm;

impl ProtocolFsm for Fsm {
    type Input = Input;
    type Output = Output;
    type Error = StepError;

    fn new() -> Self { Fsm::new() }
    fn step(&mut self, input: Self::Input) -> Result<Vec<Self::Output>, Self::Error> {
        Fsm::step(self, input)
    }
}
"#,
        );
    }

    /// Whether property-monitor machinery is emitted at all (D-P3-6).
    fn has_prop(&self) -> bool {
        self.prop_mode != PropertyMonitorMode::Off
    }

    /// The `#[cfg(...)]` prefix for gated property-monitor items ("" when the
    /// monitors are unconditional or absent).
    fn prop_cfg(&self) -> &'static str {
        match self.prop_mode {
            PropertyMonitorMode::Gated => "#[cfg(debug_assertions)] ",
            _ => "",
        }
    }

    fn input_variant(&self, group: &str, variant: &str) -> Option<&VariantInfo> {
        self.variants.input_variant(group, variant)
    }
    fn output_variant(&self, group: &str, variant: &str) -> Option<&VariantInfo> {
        self.variants.output_variant(group, variant)
    }

    fn emit_prelude(&mut self) {
        // Line 1 is the v1.0 unified artifact header (Phase 6, D-P6-3):
        // toolchain version + spec hash (SHA-256 of the canonical IR JSON of
        // the whole input file, first 16 hex chars) + module name - identical
        // text in both backends so artifacts are recognizable and drift is
        // detectable. Line 2 onward is unchanged.
        let _ = writeln!(
            self.code,
            "// Generated by RFS v{} | spec sha256:{} | module {}",
            env!("CARGO_PKG_VERSION"),
            self.options.spec_hash,
            self.module.name
        );
        if self.options.no_std {
            // no_std production codegen (Phase 5 task 9): core + alloc only.
            // The body below is BYTE-IDENTICAL to the std variant - only this
            // header changes. alloc provides Vec/String/ToString; panic! comes
            // from core, so the error paths need no modification. `use
            // alloc::vec;` (the vec! macro, exported at the alloc crate root)
            // is emitted only when some transition emits outputs.
            let _ = writeln!(self.code, "#![no_std]");
        }
        let _ = writeln!(self.code, "#![allow(dead_code, unused_variables, unused_mut, unused_parens, irrefutable_let_patterns, non_camel_case_types, clippy::all)]");
        if self.options.no_std {
            let _ = writeln!(self.code, "extern crate alloc;");
            let _ = writeln!(self.code, "use alloc::string::{{String, ToString}};");
            let _ = writeln!(self.code, "use alloc::vec::Vec;");
            // The report renderers (rfs_render_*) always use format!.
            let _ = writeln!(self.code, "use alloc::format;");
            if self.uses_vec_macro {
                let _ = writeln!(self.code, "use alloc::vec;");
            }
        }
    }
    fn emit_enums(&mut self) {
        for e in &self.module.enums {
            let name = rust_ident(&e.name);
            let _ = writeln!(self.code, "#[derive(Debug, Clone, Copy, PartialEq, Eq)]");
            let _ = writeln!(self.code, "pub enum {} {{", name);
            for v in &e.variants {
                let _ = writeln!(self.code, "    {},", rust_ident(v));
            }
            let _ = writeln!(self.code, "}}");
        }
    }
    fn emit_vars(&mut self) {
        let _ = writeln!(self.code, "#[derive(Debug, Clone, Copy, PartialEq, Eq)]");
        if self.module.vars.is_empty() {
            let _ = writeln!(self.code, "pub struct Vars;");
        } else {
            let _ = writeln!(self.code, "pub struct Vars {{");
            for v in &self.module.vars {
                let _ = writeln!(
                    self.code,
                    "    pub {}: {},",
                    rust_ident(&v.name),
                    rust_type(&v.ty)
                );
            }
            let _ = writeln!(self.code, "}}");
        }
    }
    fn emit_input_output(&mut self) {
        for (name, variants) in [
            ("Input", &self.variants.inputs),
            ("Output", &self.variants.outputs),
        ] {
            let _ = writeln!(self.code, "#[derive(Debug, Clone, Copy, PartialEq, Eq)]");
            if variants.is_empty() {
                let _ = writeln!(self.code, "pub enum {} {{}}", name);
            } else {
                let _ = writeln!(self.code, "pub enum {} {{", name);
                for v in variants {
                    if v.fields.is_empty() {
                        let _ = writeln!(self.code, "    {},", rust_ident(&v.unique_name));
                    } else {
                        let fs: Vec<String> = v
                            .fields
                            .iter()
                            .map(|(f, ty)| format!("{}: {}", rust_ident(f), rust_type(ty)))
                            .collect();
                        let _ = writeln!(
                            self.code,
                            "    {} {{ {} }},",
                            rust_ident(&v.unique_name),
                            fs.join(", ")
                        );
                    }
                }
                let _ = writeln!(self.code, "}}");
            }
        }
    }

    /// Report value renderers - byte-identical to ref_mc's text/JSON value
    /// forms (enum -> `Name::Variant`; Vars -> `Vars { f: v, ... }` / `Vars`;
    /// Input/Output -> `Input::V [payload]`). Used by the exact-mode report
    /// printers and JSON renderers of the generated Rust artifacts.
    fn emit_renderers(&mut self) {
        // Declaration order - deterministic artifact bytes.
        let list = enum_renderer_names(self.module);
        let enum_fn: std::collections::HashMap<String, String> = list.iter().cloned().collect();
        for (e, fn_name) in &list {
            let name = rust_ident(e);
            self.code.push_str(&format!(
                "fn {fn_name}(v: &{name}) -> String {{ format!(\"{name}::{{v:?}}\") }}\n"
            ));
        }
        for (tname, list) in [
            ("Input", &self.variants.inputs),
            ("Output", &self.variants.outputs),
        ] {
            let fn_name = if tname == "Input" {
                "rfs_render_input"
            } else {
                "rfs_render_output"
            };
            if list.is_empty() {
                self.code.push_str(&format!(
                    "fn {fn_name}(v: &{tname}) -> String {{ match *v {{}} }}\n"
                ));
                continue;
            }
            let arms: Vec<String> = list
                .iter()
                .map(|vi| {
                    let ident = rust_ident(&vi.unique_name);
                    if vi.fields.is_empty() {
                        format!("{tname}::{ident} => \"{tname}::{ident}\".to_string(),")
                    } else {
                        let bindings: Vec<String> =
                            vi.fields.iter().map(|(f, _)| rust_ident(f)).collect();
                        let slots: Vec<String> = vi
                            .fields
                            .iter()
                            .map(|(f, _)| format!("{}: {{}}", rust_ident(f)))
                            .collect();
                        let args: Vec<String> = vi
                            .fields
                            .iter()
                            .map(|(f, ty)| renderer_arg_rust(ty, &rust_ident(f), &enum_fn))
                            .collect();
                        format!(
                            "{tname}::{ident} {{ {} }} => format!(\"{tname}::{ident} {{{{ {} }}}}\", {}),",
                            bindings.join(", "),
                            slots.join(", "),
                            args.join(", ")
                        )
                    }
                })
                .collect();
            self.code.push_str(&format!(
                "fn {fn_name}(v: &{tname}) -> String {{ match *v {{\n{}\n}} }}\n",
                arms.join("\n")
            ));
        }
        if self.module.vars.is_empty() {
            self.code.push_str(
                "fn rfs_render_vars(v: &Vars) -> String { let _ = v; \"Vars\".to_string() }\n",
            );
        } else {
            let slots: Vec<String> = self
                .module
                .vars
                .iter()
                .map(|v| format!("{}: {{}}", rust_ident(&v.name)))
                .collect();
            let args: Vec<String> = self
                .module
                .vars
                .iter()
                .map(|v| renderer_arg_rust(&v.ty, &format!("v.{}", rust_ident(&v.name)), &enum_fn))
                .collect();
            self.code.push_str(&format!(
                "fn rfs_render_vars(v: &Vars) -> String {{ format!(\"Vars {{{{ {} }}}}\", {}) }}\n",
                slots.join(", "),
                args.join(", ")
            ));
        }
    }

    fn emit_trace_and_handlers(&mut self) {
        let pv_variant = if self.has_prop() {
            format!(
                ", {}PropertyViolation {{ label: String, step: usize }}",
                self.prop_cfg()
            )
        } else {
            String::new()
        };
        let _ = writeln!(
            self.code,
            r#"#[derive(Debug, Clone)]
pub struct TraceStep {{ pub input: Input, pub outputs: Vec<Output>, pub from_state: State, pub to_state: State, pub vars: Vars }}
#[derive(Debug, Clone)]
pub struct Trace {{ pub initial_state: State, pub initial_vars: Vars, pub steps: Vec<TraceStep> }}
/// One sample of the trace - the unit the online monitors evaluate. Values
/// are post-step; sample 0 is the initial configuration (no input/outputs,
/// so `input_is`/`output_has` are false there).
#[derive(Debug, Clone)]
pub struct Sample {{ pub input: Option<Input>, pub outputs: Vec<Output>, pub from_state: State, pub to_state: State, pub vars: Vars }}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssumptionPolicy {{ Continue, Stop, Panic }}
pub trait AssumptionHandler {{ fn on_violation(&mut self, label: &str) -> AssumptionPolicy; }}
pub struct PanicAssumptionHandler;
impl AssumptionHandler for PanicAssumptionHandler {{ fn on_violation(&mut self, label: &str) -> AssumptionPolicy {{ panic!("RFS assumption violated: {{}}", label); }} }}
pub struct StopAssumptionHandler;
impl AssumptionHandler for StopAssumptionHandler {{ fn on_violation(&mut self, _label: &str) -> AssumptionPolicy {{ AssumptionPolicy::Stop }} }}
pub struct ContinueAssumptionHandler;
impl AssumptionHandler for ContinueAssumptionHandler {{ fn on_violation(&mut self, _label: &str) -> AssumptionPolicy {{ AssumptionPolicy::Continue }} }}
#[derive(Debug, Clone, PartialEq)]
pub enum StepError {{ UnexpectedEvent, AssumptionViolation {{ label: String }}{pv_variant} }}
"#
        );
    }

    // ------------------------------------------------------------------
    // Online monitors (Phase 3): rendered from the shared MonitorPlan
    // (`crate::monitor`) with identical semantics to the C backend.
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
            let selected = matches!(&self.options.assumptions, AssumptionSelection::All)
                || matches!(&self.options.assumptions, AssumptionSelection::Labels(l) if l.iter().any(|x| x == &a.name));
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
        // Per-label monitor structs (assumptions first, then properties;
        // declaration order within each group). Property monitors are gated
        // with the aggregate in Gated mode so release builds carry nothing.
        for plan in &self.assume_plans {
            self.code.push_str(&self.render_monitor(plan, "")?);
        }
        let prop_struct_cfg = match self.prop_mode {
            PropertyMonitorMode::Gated => "#[cfg(debug_assertions)]\n",
            _ => "",
        };
        for plan in &self.prop_plans {
            self.code
                .push_str(&self.render_monitor(plan, prop_struct_cfg)?);
        }
        if self.has_prop() {
            let cfg = self.prop_cfg();
            let _ = writeln!(
                self.code,
                "{cfg}/// A property violation confirmed by the online monitors. `step` is the\n{cfg}/// sample where the failure was confirmed; `trigger_step` is the sample that\n{cfg}/// started the failing obligation (None for plain per-sample checks).\n{cfg}#[derive(Debug, Clone)]\n{cfg}pub struct PropertyViolation {{ pub label: String, pub step: usize, pub trigger_step: Option<usize> }}"
            );
            self.emit_prop_monitors();
        }
        self.emit_assume_monitors();
        // Label constants (kept for MC/test runtimes and user code; D-P3-4).
        let props: Vec<&str> = self.property_names.iter().map(|s| s.as_str()).collect();
        let assumes: Vec<&str> = self.active_assumes.iter().map(|s| s.as_str()).collect();
        let _ = writeln!(
            self.code,
            "pub const PROPERTY_LABELS: &[&str] = &{:?};",
            props
        );
        let _ = writeln!(
            self.code,
            "pub const ASSUME_LABELS: &[&str] = &{:?};",
            assumes
        );
        if let Some(v) = &self.module.version {
            let _ = writeln!(
                self.code,
                "/// Spec version header from the RFS module declaration.\npub const SPEC_VERSION: &str = \"{v}\";"
            );
        }
        let inputs: Vec<String> = self
            .variants
            .inputs
            .iter()
            .map(|v| representative_input_expr(v, self.module))
            .collect();
        if inputs.is_empty() {
            let _ = writeln!(self.code, "pub const ALL_INPUTS: &[Input] = &[];");
        } else {
            let _ = writeln!(
                self.code,
                "pub const ALL_INPUTS: &[Input] = &[{}];",
                inputs.join(", ")
            );
        }
        Ok(())
    }

    /// Render one monitor struct from a compiled plan. Nodes are numbered in
    /// DFS pre-order over the plan tree; the step body evaluates them in
    /// post-order (children before parents), mirroring the reference
    /// interpreter in `crate::monitor`.
    fn render_monitor(&self, plan: &MonitorPlan, cfg: &str) -> Result<String, String> {
        let m = snake(&plan.label);
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
        )?;

        let kind = if plan.is_assumption {
            "assumption"
        } else {
            "property"
        };
        let sig_fns: Vec<String> = sigs
            .iter()
            .enumerate()
            .map(|(i, e)| format!("    fn sig_{}(s: &Sample) -> bool {{ {} }}", i, e))
            .collect();
        Ok(format!(
            "{cfg}/// Online monitor for {kind} '{label}' (compiled from the RFS formula).\n{cfg}#[derive(Debug, Clone)]\n{cfg}pub struct Mon_{m} {{\n{fields}\n}}\n{cfg}impl Mon_{m} {{\n{sigs}\n    pub fn new() -> Self {{ Self {{ {inits} }} }}\n    /// Step the monitor on sample `idx`. Returns true when the failure is\n    /// newly confirmed at this sample.\n    pub fn step(&mut self, s: &Sample, idx: usize) -> bool {{\n        let before = self.n0_failed;\n{blocks}\n        self.n0_failed && !before\n    }}\n    /// The sample that started the currently failing obligation (None for\n    /// plain per-sample check failures).\n    pub fn trigger_step(&self) -> Option<usize> {{ (self.n0_trigger_step >= 0).then(|| self.n0_trigger_step as usize) }}\n}}\n",
            label = plan.label,
            fields = fields.iter().map(|f| format!("    {f}")).collect::<Vec<_>>().join(",\n"),
            inits = inits.join(", "),
            sigs = sig_fns.join("\n"),
            blocks = blocks.join("\n"),
        ))
    }

    fn render_node(
        &self,
        node: &MonNode,
        next_id: &mut usize,
        fields: &mut Vec<String>,
        inits: &mut Vec<String>,
        sigs: &mut Vec<String>,
        blocks: &mut Vec<String>,
    ) -> Result<(), String> {
        let id = *next_id;
        *next_id += 1;
        match node {
            MonNode::RangeCheck { expr, range } => {
                let sig = sigs.len();
                sigs.push(self.gen_signal(expr)?);
                fields.push(format!("n{id}_failed: bool"));
                fields.push(format!("n{id}_trigger_step: i32"));
                fields.push(format!("n{id}_now: bool"));
                inits.push(format!("n{id}_failed: false"));
                inits.push(format!("n{id}_trigger_step: -1"));
                inits.push(format!("n{id}_now: false"));
                let cond = Self::range_cond(range);
                // Per-sample check: `now` is recomputed every sample (the
                // child may recover); `failed` is sticky for reporting.
                blocks.push(format!(
                    "        self.n{id}_now = {cond} && !Self::sig_{sig}(s);\n        if self.n{id}_now {{ self.n{id}_failed = true; }}"
                ));
            }
            MonNode::Obligation {
                bound,
                trigger,
                body,
                range,
            } => {
                let tsig = sigs.len();
                sigs.push(self.gen_signal(trigger)?);
                let bsig = sigs.len();
                sigs.push(self.gen_signal(body)?);
                fields.push(format!("n{id}_failed: bool"));
                fields.push(format!("n{id}_trigger_step: i32"));
                inits.push(format!("n{id}_failed: false"));
                inits.push(format!("n{id}_trigger_step: -1"));
                if *bound >= 1 {
                    fields.push(format!("n{id}_pending: bool"));
                    fields.push(format!("n{id}_remaining: u64"));
                    inits.push(format!("n{id}_pending: false"));
                    inits.push(format!("n{id}_remaining: 0"));
                }
                fields.push(format!("n{id}_now: bool"));
                inits.push(format!("n{id}_now: false"));
                let cond = Self::range_cond(range);
                if *bound == 0 {
                    // be[0]: the body must hold at the trigger sample itself.
                    blocks.push(format!(
                        "        if !self.n{id}_failed && {cond} && Self::sig_{tsig}(s) && !Self::sig_{bsig}(s) {{ self.n{id}_failed = true; self.n{id}_trigger_step = idx as i32; }}\n        self.n{id}_now = self.n{id}_failed;"
                    ));
                } else {
                    // D-P3-3 pending model (the algorithm backends render).
                    blocks.push(format!(
                        "        if !self.n{id}_failed {{\n            let a = {cond} && Self::sig_{tsig}(s);\n            let b = Self::sig_{bsig}(s);\n            if !self.n{id}_pending && a {{ self.n{id}_pending = true; self.n{id}_remaining = {bound}; self.n{id}_trigger_step = idx as i32; }}\n            if self.n{id}_pending {{\n                if b {{ self.n{id}_pending = false; }}\n                else {{ self.n{id}_remaining -= 1; if self.n{id}_remaining == 0 {{ self.n{id}_failed = true; }} }}\n            }}\n        }}\n        // Obligation failures are terminal: once confirmed, the node is\n        // failing at every later sample.\n        self.n{id}_now = self.n{id}_failed;"
                    ));
                }
            }
            MonNode::DelayedCheck {
                antecedent,
                consequent,
                range,
            } => {
                let asig = sigs.len();
                sigs.push(self.gen_signal(antecedent)?);
                let csig = sigs.len();
                sigs.push(self.gen_signal(consequent)?);
                fields.push(format!("n{id}_failed: bool"));
                fields.push(format!("n{id}_trigger_step: i32"));
                fields.push(format!("n{id}_prev_antecedent: bool"));
                fields.push(format!("n{id}_now: bool"));
                inits.push(format!("n{id}_failed: false"));
                inits.push(format!("n{id}_trigger_step: -1"));
                inits.push(format!("n{id}_prev_antecedent: false"));
                inits.push(format!("n{id}_now: false"));
                let cond = Self::range_cond(range);
                // Delayed check: the antecedent held at the PREVIOUS sample, so
                // the consequent must hold now. Failure is terminal; there is
                // no trigger sample to report (trigger_step stays -1).
                blocks.push(format!(
                    "        if !self.n{id}_failed && {cond} && self.n{id}_prev_antecedent && !Self::sig_{csig}(s) {{ self.n{id}_failed = true; }}\n        self.n{id}_prev_antecedent = Self::sig_{asig}(s);\n        // DelayedCheck failures are terminal: once confirmed, the node is\n        // failing at every later sample.\n        self.n{id}_now = self.n{id}_failed;"
                ));
            }
            MonNode::And(children) | MonNode::Or(children) => {
                let mut child_ids: Vec<usize> = Vec::new();
                for c in children {
                    child_ids.push(*next_id);
                    self.render_node(c, next_id, fields, inits, sigs, blocks)?;
                }
                fields.push(format!("n{id}_failed: bool"));
                fields.push(format!("n{id}_trigger_step: i32"));
                fields.push(format!("n{id}_now: bool"));
                inits.push(format!("n{id}_failed: false"));
                inits.push(format!("n{id}_trigger_step: -1"));
                inits.push(format!("n{id}_now: false"));
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
                    .map(|c| format!("self.n{c}_now"))
                    .collect::<Vec<_>>()
                    .join(join);
                let mins: Vec<String> = child_ids
                    .iter()
                    .map(|c| {
                        format!(
                            "            if self.n{c}_now && self.n{c}_trigger_step >= 0 {{ ts = ts.min(self.n{c}_trigger_step); }}"
                        )
                    })
                    .collect();
                blocks.push(format!(
                    "        self.n{id}_now = ({any_all});\n        if self.n{id}_now && !self.n{id}_failed {{\n            self.n{id}_failed = true;\n            let mut ts = i32::MAX;\n{mins}\n            self.n{id}_trigger_step = if ts == i32::MAX {{ -1 }} else {{ ts }};\n        }}",
                    mins = mins.join("\n")
                ));
            }
        }
        Ok(())
    }

    /// The `idx` condition for an anchor range ("true" when unbounded from 0).
    fn range_cond(range: &AnchorRange) -> String {
        if range.from == 0 && range.to.is_none() {
            return "true".into();
        }
        let mut c = format!("idx as u64 >= {}", range.from);
        if let Some(t) = range.to {
            c.push_str(&format!(" && idx as u64 <= {}", t));
        }
        c
    }

    /// Render a per-sample signal expression against `s: &Sample`.
    fn gen_signal(&self, sig: &SignalExpr) -> Result<String, String> {
        render_signal_expr(self.module, &self.variants, sig)
    }

    /// Emit the `AssumeMonitors` aggregate (always present, even when empty).
    fn emit_assume_monitors(&mut self) {
        let fields: Vec<String> = self
            .assume_plans
            .iter()
            .map(|p| format!("pub mon_{}: Mon_{}", snake(&p.label), snake(&p.label)))
            .collect();
        let inits: Vec<String> = self
            .assume_plans
            .iter()
            .map(|p| {
                let s = snake(&p.label);
                format!("mon_{s}: Mon_{s}::new()")
            })
            .collect();
        let steps: Vec<String> = self.assume_plans.iter().map(|p| {
            let s = snake(&p.label);
            format!(
                "        if self.mon_{s}.step(s, idx) && r.is_none() && active_assumes.iter().any(|a| a == \"{}\") {{ r = Some(\"{}\"); }}",
                p.label, p.label
            )
        }).collect();
        let _ = writeln!(
            self.code,
            "/// Aggregate state of all assumption monitors (stepped online in `commit_step`)."
        );
        let _ = writeln!(self.code, "#[derive(Debug, Clone)]");
        let _ = writeln!(
            self.code,
            "pub struct AssumeMonitors {{ {} }}",
            fields.join(", ")
        );
        let _ = writeln!(self.code, "impl AssumeMonitors {{");
        let _ = writeln!(
            self.code,
            "    pub fn new() -> Self {{ Self {{ {} }} }}",
            inits.join(", ")
        );
        let _ = writeln!(
            self.code,
            "    /// Step all assumption monitors on sample `idx`. Returns the label of a"
        );
        let _ = writeln!(
            self.code,
            "    /// newly failed ACTIVE assumption (first in declaration order), if any."
        );
        let _ = writeln!(self.code, "    pub fn step(&mut self, s: &Sample, idx: usize, active_assumes: &[String]) -> Option<&'static str> {{");
        let _ = writeln!(self.code, "        let mut r: Option<&'static str> = None;");
        for line in steps {
            let _ = writeln!(self.code, "{}", line);
        }
        let _ = writeln!(self.code, "        r");
        let _ = writeln!(self.code, "    }}");
        let _ = writeln!(self.code, "}}");
    }

    /// Emit the `PropMonitors` aggregate (only when property monitors are on).
    fn emit_prop_monitors(&mut self) {
        let cfg = match self.prop_mode {
            PropertyMonitorMode::Gated => "#[cfg(debug_assertions)]\n",
            _ => "",
        };
        let fields: Vec<String> = self
            .prop_plans
            .iter()
            .map(|p| format!("pub mon_{}: Mon_{}", snake(&p.label), snake(&p.label)))
            .collect();
        let inits: Vec<String> = self
            .prop_plans
            .iter()
            .map(|p| {
                let s = snake(&p.label);
                format!("mon_{s}: Mon_{s}::new()")
            })
            .collect();
        let steps: Vec<String> = self.prop_plans.iter().map(|p| {
            let s = snake(&p.label);
            format!(
                "        if self.mon_{s}.step(s, idx) && r.is_none() {{ r = Some(PropertyViolation {{ label: \"{}\".into(), step: idx, trigger_step: self.mon_{s}.trigger_step() }}); }}",
                p.label
            )
        }).collect();
        let _ = writeln!(
            self.code,
            "{cfg}/// Aggregate state of all property monitors (stepped online in `commit_step`)."
        );
        let _ = writeln!(self.code, "{cfg}#[derive(Debug, Clone)]");
        let _ = writeln!(
            self.code,
            "{cfg}pub struct PropMonitors {{ {} }}",
            fields.join(", ")
        );
        let _ = writeln!(self.code, "{cfg}impl PropMonitors {{");
        let _ = writeln!(
            self.code,
            "    pub fn new() -> Self {{ Self {{ {} }} }}",
            inits.join(", ")
        );
        let _ = writeln!(
            self.code,
            "    /// Step all property monitors on sample `idx`. Returns the first newly"
        );
        let _ = writeln!(
            self.code,
            "    /// failed monitor's violation (declaration order), if any."
        );
        let _ = writeln!(
            self.code,
            "    pub fn step(&mut self, s: &Sample, idx: usize) -> Option<PropertyViolation> {{"
        );
        let _ = writeln!(
            self.code,
            "        let mut r: Option<PropertyViolation> = None;"
        );
        for line in steps {
            let _ = writeln!(self.code, "{}", line);
        }
        let _ = writeln!(self.code, "        r");
        let _ = writeln!(self.code, "    }}");
        let _ = writeln!(self.code, "}}");
    }

    fn initial_vars_expr(&self) -> Result<String, String> {
        if self.module.vars.is_empty() {
            return Ok("Vars".into());
        }
        let env = Env::for_module(self.module);
        let mut ctx = ExprCtx::new(self.module);
        ctx.names.insert("state".into(), "State::__Init__".into()); // not used in valid init
        let mut fields = Vec::new();
        for v in &self.module.vars {
            ctx.names
                .insert(v.name.clone(), format!("self.vars.{}", rust_ident(&v.name)));
            let expr = gen_expr(&v.init, &env, &ctx)?;
            fields.push(format!("{}: {}", rust_ident(&v.name), expr));
        }
        Ok(format!("Vars {{ {} }}", fields.join(", ")))
    }

    fn emit_fsm_impl(&mut self) -> Result<(), String> {
        let init_state = self
            .module
            .initial_state
            .clone()
            .unwrap_or_else(|| "__None__".into());
        let init_vars = self.initial_vars_expr().unwrap_or_else(|_| "Vars".into());
        let cfg = self.prop_cfg();
        let prop_fields = if self.has_prop() {
            format!(
                "\n    {cfg}pub prop_monitors: PropMonitors,\n    {cfg}pub property_violation: Option<PropertyViolation>"
            )
        } else {
            String::new()
        };
        let prop_init = if self.has_prop() {
            format!(" {cfg}prop_monitors: PropMonitors::new(), {cfg}property_violation: None")
        } else {
            String::new()
        };
        let prop_init_check = if self.has_prop() {
            format!(
                "{cfg}if let Some(pv) = fsm.prop_monitors.step(&sample, 0) {{\n            fsm.property_violation = Some(pv);\n        }}"
            )
        } else {
            String::new()
        };
        let prop_check = if self.has_prop() {
            format!(
                "{cfg}if let Some(pv) = self.prop_monitors.step(&sample, idx) {{\n            if self.property_violation.is_none() {{\n                self.property_violation = Some(pv.clone());\n                return Err(StepError::PropertyViolation {{ label: pv.label.clone(), step: pv.step }});\n            }}\n        }}"
            )
        } else {
            String::new()
        };
        let _ = writeln!(
            self.code,
            r#"#[derive(Debug, Clone)]
pub struct Fsm {{
    pub state: State,
    pub vars: Vars,
    pub trace: Trace,
    pub active_assumes: Vec<String>,
    pub assume_monitors: AssumeMonitors,{prop_fields}
}}
impl Fsm {{
    pub fn new() -> Self {{
        let state = State::{};
        let vars = {};
        let trace = Trace {{ initial_state: state, initial_vars: vars, steps: Vec::new() }};
        let active_assumes = ASSUME_LABELS.iter().map(|s| s.to_string()).collect();
        let mut fsm = Self {{ state, vars, trace, active_assumes, assume_monitors: AssumeMonitors::new(),{prop_init} }};
        // Sample 0 (the initial configuration): step the monitors so countdowns
        // and one-shot obligations start correctly. An assumption violated at
        // sample 0 panics, as with the previous post-hoc check.
        let sample = Sample {{ input: None, outputs: Vec::new(), from_state: fsm.state, to_state: fsm.state, vars: fsm.vars.clone() }};
        if let Some(label) = fsm.assume_monitors.step(&sample, 0, &fsm.active_assumes) {{
            panic!("RFS assumption violated: {{}}", label);
        }}
{prop_init_check}
        fsm
    }}
    pub fn step(&mut self, input: Input) -> Result<Vec<Output>, StepError> {{
        let mut handler = PanicAssumptionHandler;
        self.step_with_handler(input, &mut handler)
    }}
    pub fn step_with_handler<H: AssumptionHandler>(&mut self, input: Input, handler: &mut H) -> Result<Vec<Output>, StepError> {{
"#,
            rust_ident(&init_state),
            init_vars
        );
        // Phase 2: transitions are grouped by state (first-appearance order);
        // within a state they are checked in descending effective priority
        // (`priority N;`, default 0). Cross-state order is irrelevant because
        // only the active state can match.
        let mut state_order: Vec<&str> = Vec::new();
        let mut by_state: std::collections::HashMap<&str, Vec<usize>> =
            std::collections::HashMap::new();
        for (idx, t) in self.module.transitions.iter().enumerate() {
            if !by_state.contains_key(t.state.as_str()) {
                state_order.push(t.state.as_str());
            }
            by_state.entry(t.state.as_str()).or_default().push(idx);
        }
        for state in &state_order {
            let mut idxs: Vec<usize> = by_state.get(state).unwrap().clone();
            idxs.sort_by_key(|&k| {
                std::cmp::Reverse(self.module.transitions[k].priority.unwrap_or(0))
            });
            for idx in idxs {
                let t = &self.module.transitions[idx];
                let block = self
                    .transition_block(t, idx)
                    .map_err(|e| format!("transition #{}: {}", idx, e))?;
                self.code.push_str(&block);
            }
        }
        if self.module.on_error.is_some() {
            self.code.push_str(&self.on_error_block()?);
        } else {
            let _ = writeln!(self.code, "        Err(StepError::UnexpectedEvent)\n    }}");
        }
        let _ = writeln!(
            self.code,
            r#"    fn commit_step<H: AssumptionHandler>(&mut self, input: Input, outputs: Vec<Output>, next_state: State, new_vars: Vars, handler: &mut H) -> Result<Vec<Output>, StepError> {{
        let step = TraceStep {{ input, outputs: outputs.clone(), from_state: self.state, to_state: next_state, vars: new_vars }};
        let sample = Sample {{ input: Some(step.input.clone()), outputs: step.outputs.clone(), from_state: step.from_state, to_state: step.to_state, vars: step.vars.clone() }};
        self.trace.steps.push(step);
        let idx = self.trace.steps.len();
        if let Some(label) = self.assume_monitors.step(&sample, idx, &self.active_assumes) {{
            match handler.on_violation(label) {{
                AssumptionPolicy::Continue => {{}},
                AssumptionPolicy::Stop => return Err(StepError::AssumptionViolation {{ label: label.to_string() }}),
                AssumptionPolicy::Panic => panic!("RFS assumption violated: {{}}", label),
            }}
        }}
{prop_check}
        self.state = sample.to_state;
        self.vars = sample.vars;
        Ok(outputs)
    }}
}}"#
        );
        // group match helpers
        for group in &self.module.events {
            let variants: Vec<&VariantInfo> = self
                .variants
                .inputs
                .iter()
                .filter(|v| v.group == group.name)
                .collect();
            if variants.len() <= 1 {
                continue;
            }
            let matches: Vec<String> = variants
                .iter()
                .map(|v| {
                    if v.fields.is_empty() {
                        format!("Input::{}", rust_ident(&v.unique_name))
                    } else {
                        format!("Input::{} {{ .. }}", rust_ident(&v.unique_name))
                    }
                })
                .collect();
            let _ = writeln!(self.code, "impl Fsm {{ pub fn input_matches_group_{}(&self, input: Input) -> bool {{ matches!(input, {}) }} }}", snake(&group.name), matches.join(" | "));
        }
        Ok(())
    }

    fn transition_block(&self, t: &Transition, _idx: usize) -> Result<String, String> {
        let mut out = String::new();
        let _ = writeln!(
            out,
            "        if self.state == State::{} {{",
            rust_ident(&t.state)
        );
        let (pattern_code, env) = self.pattern_block_open(t)?;
        out.push_str(&pattern_code);

        let mut ctx = ExprCtx::new(self.module);
        ctx.names.insert("state".into(), "self.state".into());
        for v in &self.module.vars {
            ctx.names
                .insert(v.name.clone(), format!("self.vars.{}", rust_ident(&v.name)));
        }
        for name in env.bindings.keys() {
            ctx.names.insert(name.clone(), name.clone());
        }

        let mut guards: Vec<String> = Vec::new();
        if let Some(g) = &t.guard {
            guards.push(gen_expr(g, &env, &ctx)?);
        }
        let guard_cond = if guards.is_empty() {
            "true".into()
        } else {
            format!("({})", guards.join(" && "))
        };
        let _ = writeln!(out, "            if {} {{", guard_cond);

        let mut outputs: Vec<String> = Vec::new();
        for o in &t.emits {
            outputs.push(self.output_expr(o, &env, &ctx)?);
        }
        let outputs_expr: String = if outputs.is_empty() {
            "Vec::new()".into()
        } else {
            format!("vec![{}]", outputs.join(", "))
        };

        let _ = writeln!(out, "                let outputs = {};", outputs_expr);
        let _ = writeln!(
            out,
            "                let next_state = State::{};",
            rust_ident(&t.next_state)
        );
        if self.module.vars.is_empty() {
            let _ = writeln!(out, "                let new_vars = Vars;");
        } else {
            let _ = writeln!(out, "                let new_vars = {{");
            for a in &t.assignments {
                let _ = writeln!(
                    out,
                    "                    let tmp_{} = {};",
                    snake(&a.var),
                    gen_expr(&a.value, &env, &ctx)?
                );
            }
            let _ = writeln!(out, "                    let mut new_vars = self.vars;");
            for a in &t.assignments {
                let _ = writeln!(
                    out,
                    "                    new_vars.{} = tmp_{};",
                    rust_ident(&a.var),
                    snake(&a.var)
                );
            }
            let _ = writeln!(out, "                    new_vars");
            let _ = writeln!(out, "                }};");
        }
        let _ = writeln!(out, "                return self.commit_step(input, outputs, next_state, new_vars, handler);");
        let _ = writeln!(out, "            }}");
        out.push_str(&self.pattern_block_close(t)?);
        let _ = writeln!(out, "        }}");
        Ok(out)
    }

    /// Phase 2: fallback block executed when no transition matched the input
    /// (the module declared an `on_error` block). No state check and no
    /// pattern bindings; variable values are carried over unchanged.
    fn on_error_block(&self) -> Result<String, String> {
        let oe = self.module.on_error.as_ref().ok_or("no on_error block")?;
        let mut out = String::new();
        let env = Env::for_module(self.module);
        let mut ctx = ExprCtx::new(self.module);
        ctx.names.insert("state".into(), "self.state".into());
        for v in &self.module.vars {
            ctx.names
                .insert(v.name.clone(), format!("self.vars.{}", rust_ident(&v.name)));
        }
        let mut outputs: Vec<String> = Vec::new();
        for o in &oe.emits {
            outputs.push(self.output_expr(o, &env, &ctx)?);
        }
        let outputs_expr: String = if outputs.is_empty() {
            "Vec::new()".into()
        } else {
            format!("vec![{}]", outputs.join(", "))
        };
        let _ = writeln!(
            out,
            "        // on_error fallback: no transition matched the input"
        );
        let _ = writeln!(out, "        {{");
        let _ = writeln!(out, "            let outputs = {};", outputs_expr);
        let _ = writeln!(
            out,
            "            let next_state = State::{};",
            rust_ident(&oe.next_state)
        );
        let _ = writeln!(out, "            let new_vars = self.vars.clone();");
        let _ = writeln!(
            out,
            "            return self.commit_step(input, outputs, next_state, new_vars, handler);"
        );
        let _ = writeln!(out, "        }}\n    }}");
        Ok(out)
    }

    fn pattern_block_open(&self, t: &Transition) -> Result<(String, Env), String> {
        let mut code = String::new();
        let mut env = Env::for_module(self.module);
        match &t.pattern.variant {
            None => {
                let group = self
                    .module
                    .events
                    .iter()
                    .find(|g| g.name == t.pattern.group)
                    .ok_or("unknown event group")?;
                if group.variants.len() <= 1 {
                    let v = self
                        .variants
                        .inputs
                        .iter()
                        .find(|v| v.group == t.pattern.group)
                        .ok_or("no input variant")?;
                    if v.fields.is_empty() {
                        let _ = writeln!(
                            code,
                            "            if let Input::{} = input {{",
                            rust_ident(&v.unique_name)
                        );
                    } else {
                        let binds: Vec<String> =
                            (0..v.fields.len()).map(|i| format!("_f{}", i)).collect();
                        let pat_fields: Vec<String> = v
                            .fields
                            .iter()
                            .zip(binds.iter())
                            .map(|((df, _), b)| {
                                let field = rust_ident(df);
                                if field == *b {
                                    field
                                } else {
                                    format!("{}: {}", field, b)
                                }
                            })
                            .collect();
                        let _ = writeln!(
                            code,
                            "            if let Input::{} {{ {} }} = input {{",
                            rust_ident(&v.unique_name),
                            pat_fields.join(", ")
                        );
                    }
                } else {
                    let _ = writeln!(
                        code,
                        "            if self.input_matches_group_{}(input) {{",
                        snake(&t.pattern.group)
                    );
                }
            }
            Some(PatternVariant::Unit { name }) => {
                let v = self
                    .input_variant(&t.pattern.group, name)
                    .ok_or("unknown unit variant")?;
                if !v.fields.is_empty() {
                    return Err("unit pattern for struct".into());
                }
                let _ = writeln!(
                    code,
                    "            if let Input::{} = input {{",
                    rust_ident(&v.unique_name)
                );
            }
            Some(PatternVariant::Struct { name, fields }) => {
                let v = self
                    .input_variant(&t.pattern.group, name)
                    .ok_or("unknown struct variant")?;
                if v.fields.len() != fields.len() {
                    return Err("field count mismatch".into());
                }
                let mut binds = Vec::new();
                for (i, pf) in fields.iter().enumerate() {
                    let b = match &pf.name {
                        Some(n) => rust_ident(n),
                        None => format!("_f{}", i),
                    };
                    binds.push(b.clone());
                    if let PatternExpr::Bind { name } = &pf.pattern {
                        env.bindings
                            .insert(name.clone(), validator::type_to_expr_type(&v.fields[i].1));
                    }
                }
                let pat_fields: Vec<String> = v
                    .fields
                    .iter()
                    .zip(binds.iter())
                    .map(|((df, _), b)| {
                        let field = rust_ident(df);
                        if field == *b {
                            field
                        } else {
                            format!("{}: {}", field, b)
                        }
                    })
                    .collect();
                let _ = writeln!(
                    code,
                    "            if let Input::{} {{ {} }} = input {{",
                    rust_ident(&v.unique_name),
                    pat_fields.join(", ")
                );
                let mut literal_guards: Vec<String> = Vec::new();
                for ((pf, b), (_, _ty)) in fields.iter().zip(binds.iter()).zip(v.fields.iter()) {
                    match &pf.pattern {
                        PatternExpr::Int { value } => {
                            literal_guards.push(format!("{} == {}", b, value))
                        }
                        PatternExpr::Bool { value } => {
                            literal_guards.push(format!("{} == {}", b, value))
                        }
                        PatternExpr::EnumVariant { path } => literal_guards.push(format!(
                            "{} == {}",
                            b,
                            path.iter()
                                .map(|x| rust_ident(x))
                                .collect::<Vec<_>>()
                                .join("::")
                        )),
                        _ => {}
                    }
                }
                if !literal_guards.is_empty() {
                    let _ = writeln!(
                        code,
                        "                if {} {{",
                        literal_guards.join(" && ")
                    );
                }
            }
        }
        Ok((code, env))
    }

    fn pattern_block_close(&self, t: &Transition) -> Result<String, String> {
        let mut code = String::new();
        if let Some(PatternVariant::Struct { fields, .. }) = &t.pattern.variant {
            let has_literal = fields.iter().any(|pf| {
                !matches!(
                    &pf.pattern,
                    PatternExpr::Wildcard | PatternExpr::Bind { .. }
                )
            });
            if has_literal {
                code.push_str("                }\n");
            }
        }
        code.push_str("            }\n");
        Ok(code)
    }

    fn output_expr(&self, o: &OutputExpr, env: &Env, ctx: &ExprCtx) -> Result<String, String> {
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
            .output_variant(&o.group, &variant)
            .ok_or("unknown output variant")?;
        if v.fields.is_empty() {
            return Ok(format!("Output::{}", rust_ident(&v.unique_name)));
        }
        if v.fields.len() != o.args.len() {
            return Err("output arg count mismatch".into());
        }
        let mut fields = Vec::new();
        for ((fname, fty), arg) in v.fields.iter().zip(o.args.iter()) {
            let mut ctx2 = ctx.clone();
            ctx2.expected = Some(validator::type_to_expr_type(fty));
            fields.push(format!(
                "{}: {}",
                rust_ident(fname),
                gen_expr(arg, env, &ctx2)?
            ));
        }
        Ok(format!(
            "Output::{} {{ {} }}",
            rust_ident(&v.unique_name),
            fields.join(", ")
        ))
    }
}

/// Expression context for per-sample signals: `state` is the post-step state
/// and variables read from the sample.
fn signal_expr_ctx(module: &Module) -> ExprCtx {
    let mut ctx = ExprCtx::new(module);
    ctx.names.insert("state".into(), "s.to_state".into());
    for v in &module.vars {
        ctx.names
            .insert(v.name.clone(), format!("s.vars.{}", rust_ident(&v.name)));
    }
    ctx
}

/// Render a per-sample signal expression as Rust code against `s: &Sample`
/// (shared by the monitor rendering and the invariant re-checks of the test
/// module - see `crate::backends::rust_test`).
pub(crate) fn render_signal_expr(
    module: &Module,
    variants: &FsmVariants,
    sig: &SignalExpr,
) -> Result<String, String> {
    match sig {
        SignalExpr::Atom(SignalAtom::True) => Ok("true".into()),
        SignalExpr::Atom(SignalAtom::VarExpr(expr)) => {
            let env = Env::for_module(module);
            let mut ctx = signal_expr_ctx(module);
            ctx.expected = Some(ExprType::Bool);
            gen_expr(expr, &env, &ctx)
        }
        SignalExpr::Atom(SignalAtom::InputIs(p)) => {
            Ok(input_is_sample_expr(variants, p).ok_or("unresolvable input pattern")?)
        }
        SignalExpr::Atom(SignalAtom::OutputHas(p)) => {
            Ok(output_has_sample_expr(variants, p).ok_or("unresolvable output pattern")?)
        }
        SignalExpr::Not(inner) => Ok(format!(
            "!({})",
            render_signal_expr(module, variants, inner)?
        )),
        SignalExpr::And(l, r) => Ok(format!(
            "({}) && ({})",
            render_signal_expr(module, variants, l)?,
            render_signal_expr(module, variants, r)?
        )),
        SignalExpr::Or(l, r) => Ok(format!(
            "({}) || ({})",
            render_signal_expr(module, variants, l)?,
            render_signal_expr(module, variants, r)?
        )),
    }
}

fn input_is_sample_expr(variants: &FsmVariants, pattern: &EventPattern) -> Option<String> {
    match &pattern.variant {
        None => {
            let group_variants: Vec<&VariantInfo> = variants
                .inputs
                .iter()
                .filter(|v| v.group == pattern.group)
                .collect();
            if group_variants.is_empty() {
                return None;
            }
            let arms: Vec<String> = group_variants
                .iter()
                .map(|v| {
                    if v.fields.is_empty() {
                        format!("Input::{}", rust_ident(&v.unique_name))
                    } else {
                        format!("Input::{} {{ .. }}", rust_ident(&v.unique_name))
                    }
                })
                .collect();
            let arms = arms
                .iter()
                .map(|a| format!("Some({})", a))
                .collect::<Vec<_>>()
                .join(" | ");
            Some(format!("matches!(s.input.as_ref(), {})", arms))
        }
        Some(PatternVariant::Unit { name }) => {
            let v = variants.input_variant(&pattern.group, name)?;
            if !v.fields.is_empty() {
                return None;
            }
            Some(format!(
                "matches!(s.input.as_ref(), Some(Input::{name}))",
                name = rust_ident(&v.unique_name)
            ))
        }
        Some(PatternVariant::Struct { name, fields }) => {
            let v = variants.input_variant(&pattern.group, name)?;
            if v.fields.len() != fields.len() {
                return None;
            }
            // Bind each variant field by its real name and guard on the
            // pattern constraints (a `matches!` + external guard would not
            // compile: pattern bindings do not escape the match).
            let binds: Vec<String> = v.fields.iter().map(|(f, _)| rust_ident(f)).collect();
            let guards: Vec<String> = fields
                .iter()
                .enumerate()
                .filter_map(|(i, pf)| {
                    let b = &binds[i];
                    match &pf.pattern {
                        PatternExpr::Int { value } => Some(format!("*{} == {}", b, value)),
                        PatternExpr::Bool { value } => Some(format!("*{} == {}", b, value)),
                        PatternExpr::EnumVariant { path } => Some(format!(
                            "*{} == {}",
                            b,
                            path.iter()
                                .map(|x| rust_ident(x))
                                .collect::<Vec<_>>()
                                .join("::")
                        )),
                        _ => None,
                    }
                })
                .collect();
            let guard = if guards.is_empty() {
                "true".to_string()
            } else {
                guards.join(" && ")
            };
            Some(format!(
                "match s.input.as_ref() {{ Some(Input::{} {{ {} }}) => {}, _ => false }}",
                rust_ident(&v.unique_name),
                binds.join(", "),
                guard
            ))
        }
    }
}

fn output_has_sample_expr(variants: &FsmVariants, pattern: &OutputPattern) -> Option<String> {
    let inner = match &pattern.variant {
        None => {
            let group_variants: Vec<&VariantInfo> = variants
                .outputs
                .iter()
                .filter(|v| v.group == pattern.group)
                .collect();
            if group_variants.is_empty() {
                return None;
            }
            let arms: Vec<String> = group_variants
                .iter()
                .map(|v| {
                    if v.fields.is_empty() {
                        format!("Output::{}", rust_ident(&v.unique_name))
                    } else {
                        format!("Output::{} {{ .. }}", rust_ident(&v.unique_name))
                    }
                })
                .collect();
            format!("matches!(o, {})", arms.join(" | "))
        }
        Some(PatternVariant::Unit { name }) => {
            let v = variants.output_variant(&pattern.group, name)?;
            if !v.fields.is_empty() {
                return None;
            }
            format!("matches!(o, Output::{})", rust_ident(&v.unique_name))
        }
        Some(PatternVariant::Struct { name, .. }) => {
            let v = variants.output_variant(&pattern.group, name)?;
            format!(
                "matches!(o, Output::{} {{ .. }})",
                rust_ident(&v.unique_name)
            )
        }
    };
    Some(format!("s.outputs.iter().any(|o| {{ {} }})", inner))
}

struct ExprCtx {
    names: HashMap<String, String>,
    state_variants: HashSet<String>,
    expected: Option<ExprType>,
}
impl Clone for ExprCtx {
    fn clone(&self) -> Self {
        Self {
            names: self.names.clone(),
            state_variants: self.state_variants.clone(),
            expected: self.expected.clone(),
        }
    }
}
impl ExprCtx {
    fn new(module: &Module) -> Self {
        let state_variants = module
            .enums
            .iter()
            .find(|e| e.name == "State")
            .map(|e| e.variants.iter().cloned().collect())
            .unwrap_or_default();
        Self {
            names: HashMap::new(),
            state_variants,
            expected: None,
        }
    }
}

fn gen_expr(e: &Expr, env: &Env, ctx: &ExprCtx) -> Result<String, String> {
    let ty = validator::type_expr(e, env)?;
    match e {
        Expr::Int { value } => Ok(value.to_string()),
        Expr::Bool { value } => Ok(value.to_string()),
        Expr::Path { path } => {
            if path.len() == 1 {
                let name = &path[0];
                if let Some(expr) = ctx.names.get(name) {
                    return Ok(expr.clone());
                }
                if ctx.state_variants.contains(name) {
                    return Ok(format!("State::{}", rust_ident(name)));
                }
                Err(format!("unknown identifier {}", name))
            } else {
                Ok(path
                    .iter()
                    .map(|x| rust_ident(x))
                    .collect::<Vec<_>>()
                    .join("::"))
            }
        }
        Expr::Unary { op, expr } => {
            let inner = gen_expr(expr, env, ctx)?;
            match op {
                UnaryOp::Not => Ok(format!("!({})", inner)),
                UnaryOp::BitNot => {
                    let w = width_from_type(&ty, ctx.expected.as_ref())?;
                    Ok(format!(
                        "{{ let x = {}; ((! (x as u128)) & {}) as {} }}",
                        inner,
                        mask_expr(w),
                        bit_rust_type(w)
                    ))
                }
                UnaryOp::Neg => {
                    let w = width_from_type(&ty, ctx.expected.as_ref())?;
                    Ok(format!(
                        "{{ let x = {}; ((0u128.wrapping_sub(x as u128)) & {}) as {} }}",
                        inner,
                        mask_expr(w),
                        bit_rust_type(w)
                    ))
                }
            }
        }
        Expr::Binary { op, left, right } => {
            let lty = validator::type_expr(left, env)?;
            let _rty = validator::type_expr(right, env)?;
            let l = gen_expr(left, env, ctx)?;
            let r = gen_expr(right, env, ctx)?;
            match op {
                BinOp::LogicAnd => Ok(format!("({} && {})", l, r)),
                BinOp::LogicOr => Ok(format!("({} || {})", l, r)),
                BinOp::Eq => Ok(format!("({} == {})", l, r)),
                BinOp::NotEq => Ok(format!("({} != {})", l, r)),
                BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
                    let cmp = match op {
                        BinOp::Lt => "<",
                        BinOp::Le => "<=",
                        BinOp::Gt => ">",
                        _ => ">=",
                    };
                    Ok(format!("(({} as u128) {} ({} as u128))", l, cmp, r))
                }
                BinOp::Plus | BinOp::Minus | BinOp::Star => {
                    let w = width_from_type(&ty, ctx.expected.as_ref())?;
                    let m = match op {
                        BinOp::Plus => "wrapping_add",
                        BinOp::Minus => "wrapping_sub",
                        _ => "wrapping_mul",
                    };
                    Ok(format!(
                        "{{ let a = {}; let b = {}; (((a as u128).{}(b as u128)) & {}) as {} }}",
                        l,
                        r,
                        m,
                        mask_expr(w),
                        bit_rust_type(w)
                    ))
                }
                BinOp::ShiftLeft | BinOp::ShiftRight => {
                    let w = width_from_type(&lty, ctx.expected.as_ref())?;
                    let sh = if matches!(op, BinOp::ShiftLeft) {
                        "<<"
                    } else {
                        ">>"
                    };
                    Ok(format!("{{ let a = {}; let b = {}; (if (b as u32) >= {} {{ 0u128 }} else {{ ((a as u128) {} (b as u32)) & {} }}) as {} }}", l, r, w, sh, mask_expr(w), bit_rust_type(w)))
                }
                BinOp::BitAnd | BinOp::BitOr | BinOp::BitXor => {
                    if matches!(ty, ExprType::Bool) {
                        let opstr = match op {
                            BinOp::BitAnd => "&",
                            BinOp::BitOr => "|",
                            _ => "^",
                        };
                        return Ok(format!("({} {} {})", l, opstr, r));
                    }
                    let w = width_from_type(&ty, ctx.expected.as_ref())?;
                    let opstr = match op {
                        BinOp::BitAnd => "&",
                        BinOp::BitOr => "|",
                        _ => "^",
                    };
                    Ok(format!(
                        "{{ let a = {}; let b = {}; (((a as u128) {} (b as u128)) & {}) as {} }}",
                        l,
                        r,
                        opstr,
                        mask_expr(w),
                        bit_rust_type(w)
                    ))
                }
            }
        }
    }
}

fn width_from_type(ty: &ExprType, expected: Option<&ExprType>) -> Result<u32, String> {
    if let Some(ExprType::Bit(w)) = expected {
        return Ok(*w);
    }
    match ty {
        ExprType::Bit(w) => Ok(*w),
        ExprType::IntLit => Ok(8),
        _ => Err("expected bitvector".into()),
    }
}

/// Render the `mc_step_all` function used by the exact MC runtime (Phase 4,
/// D-P4-11): for `(state, input)`, the effects of EVERY enabled transition in
/// firing order (deterministic mode uses only the first effect; nondet mode
/// explores all). The on_error fallback is appended when no transition
/// matched. Rendered from the same IR with the same helpers as `Fsm::step`,
/// so both paths agree by construction.
pub(crate) fn render_mc_step_all(module: &Module) -> Result<String, String> {
    let options = BackendOptions::default();
    let variants = codegen_fsm::build_variants(module);
    let gen = Gen {
        module,
        options: &options,
        code: String::new(),
        variants,
        prop_mode: PropertyMonitorMode::Off,
        assume_plans: vec![],
        prop_plans: vec![],
        active_assumes: vec![],
        property_names: vec![],
        uses_vec_macro: false, // expression-only helpers: prelude/outputs never rendered
    };

    let mut out = String::new();
    // Group-match helpers for multi-variant groups referenced by group-level
    // patterns (the core renders these as `self.input_matches_group_*`
    // methods; here they are free functions).
    let mut groups_needed: HashSet<String> = HashSet::new();
    for t in &module.transitions {
        if t.pattern.variant.is_none() {
            let g = module
                .events
                .iter()
                .find(|g| g.name == t.pattern.group)
                .ok_or("unknown event group")?;
            if g.variants.len() > 1 {
                groups_needed.insert(g.name.clone());
            }
        }
    }
    for gname in &groups_needed {
        let arms: Vec<String> = gen
            .variants
            .inputs
            .iter()
            .filter(|v| v.group == *gname)
            .map(|v| {
                if v.fields.is_empty() {
                    format!("Input::{}", rust_ident(&v.unique_name))
                } else {
                    format!("Input::{} {{ .. }}", rust_ident(&v.unique_name))
                }
            })
            .collect();
        out.push_str(&format!(
            "fn mc_input_matches_group_{}(input: Input) -> bool {{ matches!(input, {}) }}\n",
            snake(gname),
            arms.join(" | ")
        ));
    }

    out.push_str(
        "#[derive(Debug, Clone)]\npub struct McEffect { pub next_state: State, pub next_vars: Vars, pub outputs: Vec<Output>, pub transition_id: usize }\n",
    );
    out.push_str("pub fn mc_step_all(state: State, vars: &Vars, input: Input) -> Vec<McEffect> {\n    let mut out = Vec::new();\n");

    // Per-state firing order (identical to emit_fsm_impl).
    let mut state_order: Vec<&str> = Vec::new();
    let mut by_state: HashMap<&str, Vec<usize>> = HashMap::new();
    for (idx, t) in module.transitions.iter().enumerate() {
        if !by_state.contains_key(t.state.as_str()) {
            state_order.push(t.state.as_str());
        }
        by_state.entry(t.state.as_str()).or_default().push(idx);
    }
    for st in &state_order {
        let mut idxs: Vec<usize> = by_state.get(st).unwrap().clone();
        idxs.sort_by_key(|&k| std::cmp::Reverse(module.transitions[k].priority.unwrap_or(0)));
        for idx in idxs {
            let t = &module.transitions[idx];
            out.push_str(&format!(
                "    if state == State::{} {{\n",
                rust_ident(&t.state)
            ));
            let (pattern_code, env) = gen.pattern_block_open(t)?;
            let pattern_code =
                pattern_code.replace("self.input_matches_group_", "mc_input_matches_group_");
            out.push_str(&pattern_code);

            let mut ctx = ExprCtx::new(module);
            ctx.names.insert("state".into(), "state".into());
            for v in &module.vars {
                ctx.names
                    .insert(v.name.clone(), format!("vars.{}", rust_ident(&v.name)));
            }
            for name in env.bindings.keys() {
                ctx.names.insert(name.clone(), name.clone());
            }
            let guard_cond = match &t.guard {
                Some(g) => format!("({})", gen_expr(g, &env, &ctx)?),
                None => "true".into(),
            };
            out.push_str(&format!("    if {} {{\n", guard_cond));

            let outputs: Vec<String> = t
                .emits
                .iter()
                .map(|o| gen.output_expr(o, &env, &ctx))
                .collect::<Result<_, _>>()?;
            let outputs_expr = if outputs.is_empty() {
                "Vec::new()".into()
            } else {
                format!("vec![{}]", outputs.join(", "))
            };
            out.push_str(&format!("        let outputs = {};\n", outputs_expr));
            out.push_str(&format!(
                "        let next_state = State::{};\n",
                rust_ident(&t.next_state)
            ));

            if module.vars.is_empty() {
                out.push_str("        let new_vars = Vars;\n");
            } else {
                out.push_str("        let new_vars = {\n");
                for a in &t.assignments {
                    out.push_str(&format!(
                        "            let tmp_{} = {};\n",
                        snake(&a.var),
                        gen_expr(&a.value, &env, &ctx)?
                    ));
                }
                out.push_str("            let mut new_vars = vars.clone();\n");
                for a in &t.assignments {
                    out.push_str(&format!(
                        "            new_vars.{} = tmp_{};\n",
                        rust_ident(&a.var),
                        snake(&a.var)
                    ));
                }
                out.push_str("            new_vars\n        };\n");
            }
            out.push_str(&format!(
                "        out.push(McEffect {{ next_state, next_vars: new_vars, outputs, transition_id: {} }});\n",
                idx
            ));
            out.push_str("    }\n");
            out.push_str(&gen.pattern_block_close(t)?);
            out.push_str("}\n");
        }
    }

    // on_error fallback: fires only when no transition matched the input.
    if let Some(oe) = &module.on_error {
        let env = Env::for_module(module);
        let mut ctx = ExprCtx::new(module);
        ctx.names.insert("state".into(), "state".into());
        for v in &module.vars {
            ctx.names
                .insert(v.name.clone(), format!("vars.{}", rust_ident(&v.name)));
        }
        let outputs: Vec<String> = oe
            .emits
            .iter()
            .map(|o| gen.output_expr(o, &env, &ctx))
            .collect::<Result<_, _>>()?;
        let outputs_expr = if outputs.is_empty() {
            "Vec::new()".into()
        } else {
            format!("vec![{}]", outputs.join(", "))
        };
        out.push_str(&format!(
            "    if out.is_empty() {{\n        let outputs = {};\n        out.push(McEffect {{ next_state: State::{}, next_vars: vars.clone(), outputs, transition_id: usize::MAX }});\n    }}\n",
            outputs_expr,
            rust_ident(&oe.next_state)
        ));
    }
    out.push_str("    out\n}\n");
    Ok(out)
}

/// Render a per-sample evaluator for a boolean-over-atoms formula (Phase 4
/// task 6, liveness engine): `fn <name>(s: &Sample) -> bool { <expr> }` using
/// the SAME signal rendering as monitors (`state` = post-step state, vars from
/// the sample, `input_is`/`output_has` pattern matches), so obligation bodies
/// and safety leaves agree with monitor semantics by construction.
pub(crate) fn render_sample_formula_evaluator(
    module: &Module,
    name: &str,
    formula: &Formula,
) -> Result<String, String> {
    let signal = crate::monitor::formula_to_signal(formula)
        .ok_or_else(|| "liveness subformula contains temporal operators".to_string())?;
    let options = BackendOptions::default();
    let variants = codegen_fsm::build_variants(module);
    let gen = Gen {
        module,
        options: &options,
        code: String::new(),
        variants,
        prop_mode: PropertyMonitorMode::Off,
        assume_plans: vec![],
        prop_plans: vec![],
        active_assumes: vec![],
        property_names: vec![],
        uses_vec_macro: false, // expression-only helpers: prelude/outputs never rendered
    };
    let expr = gen.gen_signal(&signal)?;
    Ok(format!(
        "pub fn {}(s: &Sample) -> bool {{ {} }}\n",
        name, expr
    ))
}

/// Map every module enum to its report-renderer function name, in declaration
/// order. `State` is a language-reserved type name and gets the fixed name the
/// static report template references; other enums get indexed names. Both the
/// Rust emitter (rust_fsm) and the JSON renderer (rust_mc) must use this same
/// map, so it lives here.
pub(crate) fn enum_renderer_names(module: &Module) -> Vec<(String, String)> {
    module
        .enums
        .iter()
        .enumerate()
        .map(|(i, e)| {
            let fn_name = if e.name == "State" {
                "rfs_render_state".to_string()
            } else {
                format!("rfs_render_e{i}")
            };
            (e.name.clone(), fn_name)
        })
        .collect()
}

/// Format-argument expression rendering `expr` of type `ty` in ref_mc's value
/// form: bool/bit print directly; enums go through their renderer.
fn renderer_arg_rust(
    ty: &crate::ir::Type,
    expr: &str,
    enum_fn: &std::collections::HashMap<String, String>,
) -> String {
    use crate::ir::Type;
    match ty {
        Type::Bool | Type::Bit { .. } => expr.to_string(),
        Type::Enum { name } => format!("{}(&{})", enum_fn[name], expr),
    }
}
