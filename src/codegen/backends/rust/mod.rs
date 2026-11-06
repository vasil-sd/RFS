//! Rust codegen backend: plugin registration and shared Rust-specific helpers.
//!
//! This is the plugin entry for the Rust target language. It registers the
//! [`RUST_BACKEND`] in the codegen registry (`crate::codegen::BACKENDS`) and
//! provides the Rust-specific helpers shared by the three generator modules:
//!
//! - [`rust_fsm`]: production FSM source generation;
//! - [`rust_test`]: unit-test template generation (and the aggregate test crate);
//! - [`rust_mc`]: bounded model-checker executable generation.
//!
//! Architecture notes live in `codegen/backends/RUST.md`.
//! Docs: dev:backend-rust-overview, dev:backend-rust-plugin, llm:backend-rust

pub mod fsm;
pub mod mc;
pub mod test;

use crate::codegen::ident::{rust_binding_ident, rust_ident, snake};
use crate::codegen::{Backend, BackendCommandInfo};

/// Rust integer type for a `bit<N>` width.
pub(crate) fn bit_rust_type(w: u32) -> &'static str {
    match w {
        0..=8 => "u8",
        9..=16 => "u16",
        17..=32 => "u32",
        33..=64 => "u64",
        _ => "u128",
    }
}

/// Rust type for an RFS type.
pub(crate) fn rust_type(ty: &crate::lang::ir::Type) -> String {
    use crate::lang::ir::Type;
    match ty {
        Type::Bool => "bool".into(),
        Type::Bit { width } => bit_rust_type(*width).into(),
        Type::Enum { name } => rust_ident(name),
    }
}

/// Bit mask expression for a width (used to wrap arithmetic results).
pub(crate) fn mask_expr(w: u32) -> String {
    if w >= 128 {
        "u128::MAX".into()
    } else {
        format!("((1u128 << {}) - 1u128)", w)
    }
}

/// Render a representative input value for a variant as Rust source (see
/// `crate::codegen::fsm::representative_values` for the value selection rules).
pub(crate) fn representative_input_expr(
    v: &crate::codegen::fsm::VariantInfo,
    module: &crate::lang::ir::Module,
) -> String {
    use crate::codegen::fsm::{representative_values, RepValue};
    if v.fields.is_empty() {
        return format!("Input::{}", rust_ident(&v.unique_name));
    }
    let fields: Vec<String> = representative_values(v, module)
        .into_iter()
        .map(|(f, val)| {
            let val = match val {
                RepValue::Bool(b) => b.to_string(),
                RepValue::Int(i) => i.to_string(),
                RepValue::EnumVariant(path) => path
                    .iter()
                    .map(|s| rust_ident(s))
                    .collect::<Vec<_>>()
                    .join("::"),
                RepValue::EnumEmpty(name) => format!("{}::__Empty__", rust_ident(&name)),
                RepValue::EnumUnknown(name) => format!("{}::__Unknown__", rust_ident(&name)),
            };
            format!("{}: {}", rust_ident(&f), val)
        })
        .collect();
    format!(
        "Input::{} {{ {} }}",
        rust_ident(&v.unique_name),
        fields.join(", ")
    )
}

/// Command: generate production Rust FSM code.
pub static PRODUCTION_INFO: BackendCommandInfo = BackendCommandInfo {
    name: "production",
    summary: "Generate production Rust FSM code (Fsm::step, trace recording, assumption handlers)",
    usage: "rfs codegen rust production <spec.rfs|spec.json> [--out FILE] [--assumes all|none|l1,l2] [--with-protocol-fsm]",
    options: "--assumes all|none|label1,label2  assumptions to activate (default: all)\n      --with-protocol-fsm         emit `impl ProtocolFsm for Fsm` (opt-in; links the artifact to the rfs-adapters crate - it must then be a dependency of the consuming crate)\n      --out FILE                    write generated source to FILE (default: stdout)",
};

/// Command: generate Rust FSM code plus a unit-test module + a separate
/// libfuzzer_sys fuzz-target skeleton file.
pub static TEST_INFO: BackendCommandInfo = BackendCommandInfo {
    name: "test",
    summary: "Generate Rust FSM code + unit tests (module-level, per-transition, invariant, monitor-violation, random-trace) + separate fuzz_target.rs",
    usage: "rfs codegen rust test <spec.rfs|spec.json> --out FILE [--steps N] [--assumes all|none|l1,l2] [--with-protocol-fsm]",
    options: "--steps N  example-run length K (default: 4)\n      --assumes all|none|label1,label2  assumptions to activate (default: all)\n      --with-protocol-fsm         emit `impl ProtocolFsm for Fsm` (opt-in; links the artifact to the rfs-adapters crate - it must then be a dependency of the consuming crate)\n      --out FILE                    REQUIRED: write the test artifact to FILE; the libfuzzer_sys fuzz skeleton is written next to it as fuzz_target.rs (stdout mode is refused - no graceful degradation)",
};

/// Command: generate a bounded model-checker Rust executable.
pub static MC_INFO: BackendCommandInfo = BackendCommandInfo {
    name: "mc",
    summary: "Generate a model-checker Rust executable (exact reachable-state search by default; fuzz mode retained)",
    usage: "rfs codegen rust mc <spec.rfs|spec.json> [--out FILE] [--assumes all|none|l1,l2] [--mode exact|fuzz] [--max-states N] [--nondet] [--timeout-sec N] [--threads N] [--counterexample-format json|text]",
    options: "--assumes all|none|label1,label2  assumptions active in the generated binary (default: all)\n      --mode exact|fuzz           model-checking mode (default: exact)\n      --max-states N              state limit for exact mode (default: 1000000)\n      --nondet                    every enabled transition is a successor (exact mode)\n      --timeout-sec N             wall-clock limit, 0 = off. Wall-clock ONLY - NOT\n                                  parity-guaranteed (the same spec can time out on one\n                                  machine and not another); never used by parity or\n                                  benchmark assertions\n      --threads N                 parallel level expansion for exact mode (default: 1).\n                                  Deterministic - results are byte-identical to\n                                  --threads 1 for any N.\n      --counterexample-format F   exact-mode counterexample report format: text\n                                  (default, the pinned human-readable report) or\n                                  json (a single machine-readable document,\n                                  byte-identical across backends). Exact mode\n                                  only - fuzz and smt-bounded keep their own\n                                  output.\n      --out FILE                  write generated source to FILE (default: stdout)\n      The generated binary accepts at runtime: --mode exact|fuzz, --max-states N,\n      --nondet, --timeout-sec N, --threads N, --fast (non-parity early abort),\n      --assumes ..., --counterexample-format json|text; fuzz mode additionally: --bound N,\n      --runs N, --seed N, --violate label:prob:percent%,... , --unexpected-prob P",
};

/// The Rust backend plugin entry, registered in `crate::codegen::BACKENDS`.
pub static RUST_BACKEND: Backend = Backend {
    name: "rust",
    description: "Rust backend: production FSM, unit tests, bounded model checker",
    commands: &[PRODUCTION_INFO, TEST_INFO, MC_INFO],
    generate_production: crate::codegen::backends::rust::fsm::generate_production,
    generate_unit_tests: crate::codegen::backends::rust::test::generate_unit_tests,
    generate_model_checker: crate::codegen::backends::rust::mc::generate_model_checker,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifier_mangling() {
        assert_eq!(rust_ident("type"), "r#type");
        assert_eq!(rust_ident("State"), "State");
        assert_eq!(snake("Recv Ack"), "recv_ack");
        assert_eq!(snake("2fast"), "_2fast");
    }

    #[test]
    fn type_mapping() {
        use crate::lang::ir::Type;
        assert_eq!(rust_type(&Type::Bool), "bool");
        assert_eq!(rust_type(&Type::Bit { width: 8 }), "u8");
        assert_eq!(rust_type(&Type::Bit { width: 64 }), "u64");
        assert_eq!(
            rust_type(&Type::Enum {
                name: "Mode".into()
            }),
            "Mode"
        );
    }

    #[test]
    fn backend_registration_shape() {
        assert_eq!(RUST_BACKEND.name, "rust");
        assert_eq!(RUST_BACKEND.commands.len(), 3);
        let names: Vec<&str> = RUST_BACKEND.commands.iter().map(|c| c.name).collect();
        assert_eq!(names, vec!["production", "test", "mc"]);
    }
}
