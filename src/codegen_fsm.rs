//! Target-language-agnostic helpers for production FSM code generation.
//!
//! This module contains the parts of FSM source generation that do not depend
//! on any target language and are therefore shared by all codegen backends
//! (see `CODEGEN.md`):
//!
//! - flattening grouped event/output constructors into a variant list with
//!   unique, collision-free names (`build_variants`);
//! - representative (default) field values used for candidate input
//!   generation (`representative_values`);
//! - safety/liveness formula analysis that determines which properties are in
//!   the translated subset (`contains_liveness`, `safety_formula_inner`).
//!
//! Backends render these structures into their own syntax; identifier
//! mangling, type mapping, and expression lowering stay in the backend.
//! Docs: dev:codegen-core-overview, llm:codegen-core

use crate::ir::*;

/// One flattened constructor variant of an event/output group.
#[derive(Debug, Clone)]
pub struct VariantInfo {
    /// Name of the owning group (event or output).
    pub group: String,
    /// Name of the variant within the group.
    pub variant: String,
    /// Pre-mangling base name: the variant name for single-variant groups,
    /// otherwise `group_variant`.
    pub base_name: String,
    /// Collision-free name derived from `base_name` (numeric suffixes are
    /// appended on duplicates). Not yet mangled by any target language.
    pub unique_name: String,
    /// Field names and types in declaration order (raw IR names; backends
    /// apply their own identifier rules when emitting).
    pub fields: Vec<(String, Type)>,
}

/// Flattened input/output variants of a module.
#[derive(Debug, Clone)]
pub struct FsmVariants {
    pub inputs: Vec<VariantInfo>,
    pub outputs: Vec<VariantInfo>,
}

impl FsmVariants {
    pub fn input_variant(&self, group: &str, variant: &str) -> Option<&VariantInfo> {
        self.inputs
            .iter()
            .find(|v| v.group == group && v.variant == variant)
    }
    pub fn output_variant(&self, group: &str, variant: &str) -> Option<&VariantInfo> {
        self.outputs
            .iter()
            .find(|v| v.group == group && v.variant == variant)
    }
}

/// Flatten the module's event and output groups into variant lists.
///
/// Naming scheme (shared by all backends): a group with a single variant whose
/// name equals the group name keeps that name; otherwise the base name is
/// `group_variant`. Duplicates get numeric suffixes (`Name`, `Name2`, ...).
pub fn build_variants(module: &Module) -> FsmVariants {
    let flatten = |groups: &[ConstructorGroup]| -> Vec<VariantInfo> {
        let mut out = Vec::new();
        let mut used: std::collections::HashSet<String> = std::collections::HashSet::new();
        for group in groups {
            for v in &group.variants {
                let (vname, fields) = match v {
                    ConstructorVariant::Unit { name } => (name.clone(), vec![]),
                    ConstructorVariant::Struct { name, fields } => (
                        name.clone(),
                        fields
                            .iter()
                            .map(|f| (f.name.clone(), f.ty.clone()))
                            .collect(),
                    ),
                };
                let base = if group.name == vname && group.variants.len() == 1 {
                    group.name.clone()
                } else {
                    format!("{}_{}", group.name, vname)
                };
                let mut unique = base.clone();
                let mut n = 2;
                while !used.insert(unique.clone()) {
                    unique = format!("{}{}", base, n);
                    n += 1;
                }
                out.push(VariantInfo {
                    group: group.name.clone(),
                    variant: vname,
                    base_name: base,
                    unique_name: unique,
                    fields,
                });
            }
        }
        out
    };
    FsmVariants {
        inputs: flatten(&module.events),
        outputs: flatten(&module.outputs),
    }
}

/// A representative (default) value for a constructor field.
#[derive(Debug, Clone, PartialEq)]
pub enum RepValue {
    /// `false` for bool fields.
    Bool(bool),
    /// Zero for `bit<N>` fields.
    Int(u64),
    /// First declared variant of the enum, as a full path (`[Enum, Variant]`).
    EnumVariant(Vec<String>),
    /// The enum type exists but declares no variants (unrepresentable).
    EnumEmpty(String),
    /// The enum type is not declared in the module (unrepresentable).
    EnumUnknown(String),
}

/// Representative field values for a variant: bool -> `false`, `bit<N>` -> `0`,
/// enum -> first declared variant. Candidate input generation (unit tests,
/// model checker) uses one representative value per variant.
pub fn representative_values(variant: &VariantInfo, module: &Module) -> Vec<(String, RepValue)> {
    variant
        .fields
        .iter()
        .map(|(f, ty)| {
            let val = match ty {
                Type::Bool => RepValue::Bool(false),
                Type::Bit { .. } => RepValue::Int(0),
                Type::Enum { name } => {
                    if let Some(e) = module.enums.iter().find(|e| e.name == *name) {
                        match e.variants.first() {
                            Some(first) => RepValue::EnumVariant(vec![name.clone(), first.clone()]),
                            None => RepValue::EnumEmpty(name.clone()),
                        }
                    } else {
                        RepValue::EnumUnknown(name.clone())
                    }
                }
            };
            (f.clone(), val)
        })
        .collect()
}

/// Whether a formula contains `bounded_eventually` (liveness in the Phase 1
/// translated subset).
pub fn contains_liveness(f: &Formula) -> bool {
    match f {
        Formula::BoundedEventually { .. } => true,
        Formula::Not { inner }
        | Formula::Always { inner }
        | Formula::Next { inner }
        | Formula::BoundedAlways { inner, .. } => contains_liveness(inner),
        Formula::And { left, right }
        | Formula::Or { left, right }
        | Formula::Implies { left, right } => contains_liveness(left) || contains_liveness(right),
        _ => false,
    }
}

/// The inner formula of a translatable safety property: `always (phi)` where
/// `phi` is liveness-free. Returns `None` for anything outside the subset.
pub fn safety_formula_inner(f: &Formula) -> Option<&Formula> {
    let Formula::Always { inner } = f else {
        return None;
    };
    if contains_liveness(inner) {
        return None;
    }
    Some(inner)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_source;

    fn module(src: &str) -> Module {
        parse_source(src)
            .expect("parse")
            .modules
            .pop()
            .expect("module")
    }

    #[test]
    fn variant_flattening_and_unique_names() {
        let m = module(
            "module m { enum State { A } initial A event Recv(Syn) event Recv(Ack(x: bit<4>)) output O output O(X(y: bool)) }",
        );
        let v = build_variants(&m);
        assert_eq!(v.inputs.len(), 2);
        assert_eq!(v.inputs[0].unique_name, "Recv_Syn");
        assert_eq!(v.inputs[1].unique_name, "Recv_Ack");
        assert_eq!(
            v.inputs[1].fields,
            vec![("x".to_string(), Type::Bit { width: 4 })]
        );
        // Single-variant group keeps the bare name.
        let m2 = module("module m { enum State { A } initial A event Tick }");
        let v2 = build_variants(&m2);
        assert_eq!(v2.inputs[0].unique_name, "Tick");
    }

    #[test]
    fn representative_values_cover_all_field_kinds() {
        let m = module(
            "module m { enum State { A } initial A enum Mode { M1, M2 } event E(V(b: bool, n: bit<8>, mode: Mode)) }",
        );
        let v = build_variants(&m);
        let vals = representative_values(&v.inputs[0], &m);
        assert_eq!(vals[0].1, RepValue::Bool(false));
        assert_eq!(vals[1].1, RepValue::Int(0));
        assert_eq!(
            vals[2].1,
            RepValue::EnumVariant(vec!["Mode".into(), "M1".into()])
        );
    }

    #[test]
    fn safety_subset_analysis() {
        let m = module("module m { enum State { A } initial A event E prop p : always (state == A) ; prop q : always (bounded_eventually[3] (state == A)) ; }");
        assert!(safety_formula_inner(&m.properties[0].formula).is_some());
        assert!(safety_formula_inner(&m.properties[1].formula).is_none());
        assert!(contains_liveness(&m.properties[1].formula));
    }
}
