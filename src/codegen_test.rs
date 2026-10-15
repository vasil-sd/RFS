//! Target-language-agnostic planning for FSM unit-test generation.
//!
//! Shared by all codegen backends (see `CODEGEN.md`). A backend's `test`
//! command generates example runs of up to K steps:
//!
//! - **positive runs**: drive the FSM with representative inputs and expect
//!   progress (the sequence ends without errors);
//! - **negative runs**: look for an input that makes the FSM fail. When a
//!   statically derivable unexpected input exists (an input variant no
//!   transition handles), the generated test asserts that the failure is
//!   found; otherwise it records the attempt without asserting.
//!
//! This module decides *what* to test; backends render the actual test code.

use crate::codegen_fsm::FsmVariants;
use crate::ir::Module;

/// The plan for a generated unit-test suite.
#[derive(Debug, Clone)]
pub struct TestPlan {
    /// Maximum example-run length (K steps).
    pub steps: usize,
    /// Whether the positive run is expected to make at least one step
    /// (true when the module declares transitions).
    pub expect_progress: bool,
    /// Whether some input variant is statically not handled by any transition;
    /// when true, the negative test asserts that an unexpected-event failure
    /// is found.
    pub definite_unexpected: bool,
}

/// Build a test plan for a module given its flattened variants.
pub fn build_test_plan(variants: &FsmVariants, module: &Module, steps: usize) -> TestPlan {
    TestPlan {
        steps,
        expect_progress: !module.transitions.is_empty(),
        definite_unexpected: has_definite_unexpected(variants, module),
    }
}

/// True when at least one input variant is never matched by any transition
/// pattern. Such a variant's representative value is guaranteed to produce an
/// `UnexpectedEvent` error on some reachable state (in particular, the test
/// driver will find it while exploring).
pub fn has_definite_unexpected(variants: &FsmVariants, module: &Module) -> bool {
    if variants.inputs.is_empty() {
        return false;
    }
    let mut used = std::collections::HashSet::new();
    for t in &module.transitions {
        match &t.pattern.variant {
            None => {
                for v in variants
                    .inputs
                    .iter()
                    .filter(|v| v.group == t.pattern.group)
                {
                    used.insert(v.unique_name.clone());
                }
            }
            Some(crate::ir::PatternVariant::Unit { name })
            | Some(crate::ir::PatternVariant::Struct { name, .. }) => {
                if let Some(v) = variants.input_variant(&t.pattern.group, name) {
                    used.insert(v.unique_name.clone());
                }
            }
        }
    }
    variants
        .inputs
        .iter()
        .any(|v| !used.contains(&v.unique_name))
}

/// True when the module has inputs but no transitions at all: every input is
/// unexpected from the initial state (a deadlock-style negative case).
pub fn is_deadlocked_initial(module: &Module, variants: &FsmVariants) -> bool {
    !variants.inputs.is_empty() && module.transitions.is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codegen_fsm::build_variants;
    use crate::parse_source;

    fn variants(src: &str) -> (Module, FsmVariants) {
        let m = parse_source(src)
            .expect("parse")
            .modules
            .pop()
            .expect("module");
        (m.clone(), build_variants(&m))
    }

    #[test]
    fn plan_flags_deadlocked_module() {
        let (m, v) = variants("module m { enum State { A } initial A event Go }");
        let plan = build_test_plan(&v, &m, 4);
        assert!(plan.definite_unexpected);
        assert!(!plan.expect_progress);
        assert!(is_deadlocked_initial(&m, &v));
    }

    #[test]
    fn plan_flags_fully_covered_module() {
        let (m, v) = variants(
            "module m { enum State { A, B } initial A event Go transition A on Go { next B; } }",
        );
        let plan = build_test_plan(&v, &m, 4);
        assert!(!plan.definite_unexpected);
        assert!(plan.expect_progress);
        assert!(!is_deadlocked_initial(&m, &v));
    }

    #[test]
    fn partial_coverage_is_definite_unexpected() {
        let (m, v) = variants(
            "module m { enum State { A, B } initial A event Recv(Syn) event Recv(Ack(x: bit<4>)) transition A on Recv(Syn) { next B; } }",
        );
        let plan = build_test_plan(&v, &m, 4);
        assert!(plan.definite_unexpected); // Recv_Ack is never matched
    }
}
