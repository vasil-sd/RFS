//! C codegen backend: plugin registration and shared C-specific helpers.
//!
//! This is the plugin entry for the C target language (Phase 2). It registers
//! [`C_BACKEND`] in the codegen registry (`crate::codegen::BACKENDS`) and
//! provides the C-specific helpers shared by the three generator modules:
//!
//! - [`c_fsm`]: production FSM source generation;
//! - [`c_test`]: plain-assert unit-test file generation;
//! - [`c_mc`]: bounded model-checker executable generation (same runtime wire
//!   format as the Rust model checker).
//!
//! Architecture notes live in `backends/C.md`.
//! Docs: dev:backend-c-overview, llm:backend-c

use crate::codegen::{Backend, BackendCommandInfo};
use crate::ir::Type;

/// C keywords and common identifiers that generated names must avoid.
pub(crate) const C_KEYWORDS: &[&str] = &[
    "auto", "break", "case", "char", "const", "continue", "default", "do", "double", "else",
    "enum", "extern", "float", "for", "goto", "if", "inline", "int", "long", "register",
    "restrict", "return", "short", "signed", "sizeof", "static", "struct", "switch", "typedef",
    "union", "unsigned", "void", "volatile", "while", "bool", "true", "false", "main",
];

/// Mangling of an identifier for C: keywords and reserved words get a `c_`
/// prefix; everything else is kept as-is (RFS identifiers are already
/// snake_case-friendly).
pub(crate) fn c_ident(name: &str) -> String {
    if C_KEYWORDS.contains(&name) {
        format!("c_{}", name)
    } else {
        name.to_string()
    }
}

/// Mangling of a PATTERN BINDING identifier for C: in addition to keyword
/// mangling, bindings that collide with identifiers the generated transition
/// body uses unqualified are renamed to `<name>__rfsb` (and further suffixed
/// until unique among `used`). Without this, e.g. a field named `n` would
/// shadow mc_step_all's effect counter `int n`: effects were written to
/// `out[payload value]` (an out-of-bounds stack write for wide fields) and
/// the counter never incremented, so the C model checker silently missed
/// every successor through such a transition - a backend-parity violation
/// reachable from fully valid specs (Phase 6 task 6 finding).
pub(crate) fn c_binding_ident(
    name: &str,
    reserved: &[&str],
    used: &mut std::collections::HashSet<String>,
) -> String {
    let base = c_ident(name);
    let mut ident = if reserved.iter().any(|r| *r == base) {
        format!("{base}__rfsb")
    } else {
        base
    };
    while !used.insert(ident.clone()) {
        ident = format!("{ident}__rfsb");
    }
    ident
}

/// Fixed-width integer type for a `bit<N>` width. The C backend supports the
/// standard fixed widths 8/16/32/64; other widths are a codegen error (see
/// DESIGN.md section 19, decision D8).
pub(crate) fn bit_c_type(w: u32) -> Result<&'static str, String> {
    match w {
        0..=8 => Ok("uint8_t"),
        16 => Ok("uint16_t"),
        32 => Ok("uint32_t"),
        64 => Ok("uint64_t"),
        other => Err(format!(
            "C backend supports bit widths 8, 16, 32 and 64 (got bit<{other}>)"
        )),
    }
}

/// C type for an RFS type. Enum types map to `<Name>_t` typedefs emitted by
/// the FSM generator.
pub(crate) fn c_type(ty: &Type) -> Result<String, String> {
    match ty {
        Type::Bool => Ok("bool".into()),
        Type::Bit { width } => bit_c_type(*width).map(str::to_string),
        Type::Enum { name } => Ok(format!("{}_t", c_ident(name))),
    }
}

/// C expression for an integer literal of a given bit width (keeps the literal
/// in-range for the target type, e.g. `255` for `uint8_t`).
pub(crate) fn int_c_literal(value: u64, width: Option<u32>) -> String {
    match width {
        Some(w) if w < 64 => format!("({}u)", (value & ((1u64 << w) - 1))),
        _ => format!("({}ULL)", value),
    }
}

/// Mask expression for a bit width (used after bitwise ops).
pub(crate) fn mask_c_expr(width: u32) -> String {
    if width >= 64 {
        "0xFFFFFFFFFFFFFFFFULL".into()
    } else {
        format!("((1ULL << {}) - 1)", width)
    }
}

const PRODUCTION_INFO: BackendCommandInfo = BackendCommandInfo {
    name: "production",
    summary: "Production FSM source (states, inputs/outputs, vars, fsm_step)",
    usage: "rfs codegen c production <spec.rfs|spec.json> [--out FILE]",
    options: "--assumes all|none|label1,label2  active safety assumptions (default all)",
};

const TEST_INFO: BackendCommandInfo = BackendCommandInfo {
    name: "test",
    summary: "Generate C FSM code + plain-assert unit tests (module-level, per-transition, invariant, monitor-violation, random-trace) + separate fuzz_target.c",
    usage: "rfs codegen c test <spec.rfs|spec.json> --out FILE [--steps N] [--assumes all|none|l1,l2]",
    options: "--steps N  example-run length K (default: 4)\n      --assumes all|none|label1,label2  assumptions to activate (default: all)\n      --out FILE                    REQUIRED: write the test artifact to FILE; the libFuzzer skeleton is written next to it as fuzz_target.c (stdout mode is refused - no graceful degradation)",
};

const MC_INFO: BackendCommandInfo = BackendCommandInfo {
    name: "mc",
    summary: "Generate a model-checker C executable (exact reachable-state search by default; fuzz mode retained)",
    usage: "rfs codegen c mc <spec.rfs|spec.json> [--out FILE] [--assumes all|none|l1,l2] [--mode exact|fuzz] [--max-states N] [--nondet] [--timeout-sec N] [--threads N] [--counterexample-format json|text]",
    options: "--assumes all|none|label1,label2  assumptions active in the generated binary (default: all)\n      --mode exact|fuzz           model-checking mode (default: exact)\n      --max-states N              state limit for exact mode (default: 1000000)\n      --nondet                    every enabled transition is a successor (exact mode)\n      --timeout-sec N             wall-clock limit, 0 = off. Wall-clock ONLY - NOT\n                                  parity-guaranteed (the same spec can time out on one\n                                  machine and not another); never used by parity or\n                                  benchmark assertions\n      --threads N                 parallel level expansion for exact mode (default: 1).\n                                  Deterministic - results are byte-identical to\n                                  --threads 1 for any N.\n      --counterexample-format F   exact-mode counterexample report format: text\n                                  (default, the pinned human-readable report) or\n                                  json (a single machine-readable document,\n                                  byte-identical across backends). Exact mode\n                                  only - fuzz and smt-bounded keep their own\n                                  output.\n      --out FILE                  write generated source to FILE (default: stdout)\n      The generated binary accepts at runtime: --mode exact|fuzz, --max-states N,\n      --nondet, --timeout-sec N, --threads N, --fast (non-parity early abort),\n      --assumes ..., --counterexample-format json|text; fuzz mode additionally: --bound N,\n      --runs N, --seed N, --violate label:prob:percent%,... , --unexpected-prob P",
};

/// The registered C backend (see `CODEGEN.md` for the plugin contract).
pub static C_BACKEND: Backend = Backend {
    name: "c",
    description: "C backend: production FSM, plain-assert unit tests, bounded model checker",
    commands: &[PRODUCTION_INFO, TEST_INFO, MC_INFO],
    generate_production: crate::backends::c_fsm::generate_production,
    generate_unit_tests: crate::backends::c_test::generate_unit_tests,
    generate_model_checker: crate::backends::c_mc::generate_model_checker,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifier_mangling() {
        assert_eq!(c_ident("register"), "c_register");
        assert_eq!(c_ident("Ack"), "Ack");
        assert_eq!(c_ident("main"), "c_main");
    }

    #[test]
    fn type_mapping() {
        assert_eq!(c_type(&Type::Bool).unwrap(), "bool");
        assert_eq!(c_type(&Type::Bit { width: 8 }).unwrap(), "uint8_t");
        assert_eq!(c_type(&Type::Bit { width: 64 }).unwrap(), "uint64_t");
        assert!(c_type(&Type::Bit { width: 100 }).is_err());
        assert_eq!(
            c_type(&Type::Enum {
                name: "Mode".into()
            })
            .unwrap(),
            "Mode_t"
        );
    }

    #[test]
    fn backend_registration_shape() {
        assert_eq!(C_BACKEND.name, "c");
        let names: Vec<&str> = C_BACKEND.commands.iter().map(|c| c.name).collect();
        assert_eq!(names, vec!["production", "test", "mc"]);
    }

    #[test]
    fn literals_and_masks() {
        assert_eq!(int_c_literal(255, Some(8)), "(255u)");
        assert_eq!(int_c_literal(300, Some(8)), "(44u)");
        assert_eq!(int_c_literal(7, None), "(7ULL)");
        assert_eq!(mask_c_expr(8), "((1ULL << 8) - 1)");
    }
}
