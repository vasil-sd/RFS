//! Shared JSON-Schema (draft-07) emitter + markdown renderer + sync walker
//! for the `rfs info format` JSON entries (Phase 7 task 6, D-P7-8).
//!
//! Each JSON format entry (spec-json, ce-trace, mc-report, validate-json,
//! info-json) builds a [`SchemaDoc`] against the code's own key constants /
//! writers, and this module renders it as:
//! - draft-07 JSON Schema - pretty JSON produced through `json_rw` (project
//!   rule: all JSON goes through json_rw), with `additionalProperties: false`
//!   at every object level so CI validators reject undocumented keys (the
//!   exception is [`FieldType::FreeObj`] - objects whose KEYS are data, e.g.
//!   variable/field-name maps in counterexample traces);
//! - a markdown field-table document (the `md` output), optionally followed
//!   by free-form extra sections ([`SchemaDoc::extra_sections`]) for format
//!   aspects that have no schema (e.g. the text trace format).
//!
//! [`walk_check`] is the D-P7-9 sync guarantee: it verifies that a real
//! generated document matches the model - every required key present, no
//! undocumented key at any object level, types and enum constraints agree.
//! Docs: dev:ir-json-rw-overview, dev:json-schema, llm:ir-json-rw

use crate::json_rw::Json;

/// One field of an object (or one tagged-union variant's extra fields).
#[derive(Debug, Clone)]
pub struct Field {
    pub name: String,
    pub ty: FieldType,
    /// `true` = the key is always present in writer output.
    pub required: bool,
    /// One-line description (JSON-Schema "description" + md table).
    pub note: Option<String>,
}

impl Field {
    pub fn req(name: &str, ty: FieldType, note: Option<&str>) -> Self {
        Field {
            name: name.into(),
            ty,
            required: true,
            note: note.map(str::to_string),
        }
    }

    pub fn opt(name: &str, ty: FieldType, note: Option<&str>) -> Self {
        Field {
            name: name.into(),
            ty,
            required: false,
            note: note.map(str::to_string),
        }
    }
}

/// The value model of one JSON field. Owns its data (built at runtime by the
/// format entries; no const constraints needed).
#[derive(Debug, Clone)]
pub enum FieldType {
    Str,
    Int,
    Bool,
    Null,
    /// String restricted to a fixed set of values.
    EnumStr(Vec<String>),
    /// Object with named fields.
    Obj(Vec<Field>),
    /// Homogeneous array.
    Arr(Box<FieldType>),
    /// `$ref` to a named definition in the same document.
    Ref(String),
    /// Internally-tagged union: object with a `"kind"` string discriminator;
    /// each variant lists its fields beyond `kind`.
    TaggedUnion(Vec<TaggedVariant>),
    /// Inner type or JSON null - the key is still present (e.g. `guard`).
    Nullable(Box<FieldType>),
    /// Object with ARBITRARY string keys; every value must conform to the
    /// inner type (JSON Schema: `additionalProperties` = the inner schema,
    /// no fixed `properties`). For name->value maps whose names come from
    /// the spec (variable names, field names) - e.g. ce-trace `vars`.
    FreeObj(Box<FieldType>),
    /// Plain type union: the value must conform to at least one alternative
    /// (JSON Schema: `anyOf`). Not a tagged union - no discriminator.
    Union(Vec<FieldType>),
}

impl FieldType {
    pub fn str() -> Self {
        FieldType::Str
    }
    pub fn int() -> Self {
        FieldType::Int
    }
    pub fn bool() -> Self {
        FieldType::Bool
    }
    pub fn null() -> Self {
        FieldType::Null
    }
    pub fn enum_str(vals: &[&str]) -> Self {
        FieldType::EnumStr(vals.iter().map(|s| s.to_string()).collect())
    }
    pub fn obj(fields: Vec<Field>) -> Self {
        FieldType::Obj(fields)
    }
    pub fn arr(item: FieldType) -> Self {
        FieldType::Arr(Box::new(item))
    }
    pub fn r#ref(name: &str) -> Self {
        FieldType::Ref(name.to_string())
    }
    pub fn tagged_union(variants: Vec<TaggedVariant>) -> Self {
        FieldType::TaggedUnion(variants)
    }
    pub fn nullable(inner: FieldType) -> Self {
        FieldType::Nullable(Box::new(inner))
    }
    pub fn free_obj(value: FieldType) -> Self {
        FieldType::FreeObj(Box::new(value))
    }
    pub fn union(alts: Vec<FieldType>) -> Self {
        FieldType::Union(alts)
    }
}

/// One variant of an internally-tagged union.
#[derive(Debug, Clone)]
pub struct TaggedVariant {
    pub kind: String,
    /// Fields beyond the `"kind"` discriminator itself.
    pub fields: Vec<Field>,
}

impl TaggedVariant {
    pub fn new(kind: &str, fields: Vec<Field>) -> Self {
        TaggedVariant {
            kind: kind.to_string(),
            fields,
        }
    }
}

/// One documented JSON document: root value model + named sub-schemas.
pub struct SchemaDoc {
    pub title: String,
    pub description: String,
    pub root: FieldType,
    /// Named sub-schemas (recursion + reuse). Order = md section order.
    pub definitions: Vec<(String, FieldType)>,
    /// Free-form markdown sections appended after the definition sections in
    /// the `md` output (heading, body) - for format aspects without a schema
    /// (e.g. the ce-trace text report format). Never emitted into the schema.
    pub extra_sections: Vec<(String, String)>,
}

// ---------------------------------------------------------------------------
// JSON-Schema emission (draft-07)
// ---------------------------------------------------------------------------

/// Emit one value model as its JSON-Schema fragment. Public so format
/// entries can cross-check shared sub-models (e.g. mc-report's
/// `counterexample` definition against the ce-trace root - task 8).
pub fn emit_type(ty: &FieldType) -> Json {
    match ty {
        FieldType::Str => Json::obj(vec![("type".into(), Json::str("string"))]),
        FieldType::Int => Json::obj(vec![("type".into(), Json::str("integer"))]),
        FieldType::Bool => Json::obj(vec![("type".into(), Json::str("boolean"))]),
        FieldType::Null => Json::obj(vec![("type".into(), Json::str("null"))]),
        FieldType::EnumStr(vals) => Json::obj(vec![
            ("type".into(), Json::str("string")),
            (
                "enum".into(),
                Json::arr(vals.iter().map(|v| Json::str(v.clone())).collect()),
            ),
        ]),
        FieldType::Obj(fields) => emit_object(fields),
        FieldType::Arr(item) => Json::obj(vec![
            ("type".into(), Json::str("array")),
            ("items".into(), emit_type(item)),
        ]),
        FieldType::Ref(name) => Json::obj(vec![(
            "$ref".into(),
            Json::str(format!("#/definitions/{name}")),
        )]),
        FieldType::TaggedUnion(variants) => {
            let alts = variants
                .iter()
                .map(|v| {
                    let mut fields = vec![Field::req(
                        "kind",
                        FieldType::enum_str(&[v.kind.as_str()]),
                        None,
                    )];
                    fields.extend(v.fields.iter().cloned());
                    emit_object(&fields)
                })
                .collect();
            Json::obj(vec![("oneOf".into(), Json::arr(alts))])
        }
        FieldType::Nullable(inner) => match inner.as_ref() {
            // Simple types merge into a type array; $ref/oneOf cannot sit
            // beside other keywords in draft-07, so those need anyOf.
            FieldType::Str | FieldType::Int | FieldType::Bool => {
                let t = match inner.as_ref() {
                    FieldType::Str => "string",
                    FieldType::Int => "integer",
                    _ => "boolean",
                };
                Json::obj(vec![(
                    "type".into(),
                    Json::arr(vec![Json::str(t), Json::str("null")]),
                )])
            }
            _ => Json::obj(vec![(
                "anyOf".into(),
                Json::arr(vec![emit_type(inner), emit_type(&FieldType::Null)]),
            )]),
        },
        FieldType::FreeObj(value) => Json::obj(vec![
            ("type".into(), Json::str("object")),
            ("additionalProperties".into(), emit_type(value)),
        ]),
        FieldType::Union(alts) => Json::obj(vec![(
            "anyOf".into(),
            Json::arr(alts.iter().map(emit_type).collect()),
        )]),
    }
}

fn emit_object(fields: &[Field]) -> Json {
    let mut props = Vec::new();
    let mut required = Vec::new();
    for f in fields {
        // Merge the field's schema fragment into the property object (a
        // simple type is `{"type": "..."}`, a ref is `{"$ref": ...}`). In
        // draft-07 siblings of `$ref` are ignored, so no description there.
        let frag = emit_type(&f.ty);
        let is_ref = matches!(&frag, Json::Obj(p) if p.iter().any(|(k, _)| k == "$ref"));
        let mut o = Vec::new();
        if let Json::Obj(pairs) = frag {
            o.extend(pairs);
        }
        if !is_ref {
            if let Some(n) = &f.note {
                o.push(("description".into(), Json::str(n.clone())));
            }
        }
        props.push((f.name.clone(), Json::obj(o)));
        if f.required {
            required.push(f.name.clone());
        }
    }
    let mut o = vec![
        ("type".into(), Json::str("object")),
        ("properties".into(), Json::obj(props)),
    ];
    if !required.is_empty() {
        o.push((
            "required".into(),
            Json::arr(required.into_iter().map(Json::str).collect()),
        ));
    }
    o.push(("additionalProperties".into(), Json::bool(false)));
    Json::obj(o)
}

/// The full draft-07 schema document as a `Json` value.
pub fn emit_schema(doc: &SchemaDoc) -> Json {
    let mut o = vec![
        (
            "$schema".into(),
            Json::str("http://json-schema.org/draft-07/schema#"),
        ),
        ("title".into(), Json::str(doc.title.clone())),
        ("description".into(), Json::str(doc.description.clone())),
    ];
    match &doc.root {
        // A plain object root inlines its properties at the top level
        // (the standard shape for document schemas).
        FieldType::Obj(fields) => {
            let inner = emit_object(fields);
            if let Json::Obj(pairs) = inner {
                o.extend(pairs);
            }
        }
        other => o.push(("allOf".into(), Json::arr(vec![emit_type(other)]))),
    }
    let defs = doc
        .definitions
        .iter()
        .map(|(name, ty)| (name.clone(), emit_type(ty)))
        .collect();
    o.push(("definitions".into(), Json::obj(defs)));
    Json::obj(o)
}

/// The schema as pretty JSON text (project rule: through json_rw).
pub fn emit_schema_pretty(doc: &SchemaDoc) -> String {
    emit_schema(doc).to_string_pretty() + "\n"
}

// ---------------------------------------------------------------------------
// Markdown rendering (shared by all JSON format entries)
// ---------------------------------------------------------------------------

/// Render one field type for the md table's Type column. `[name]` marks a
/// reference to the `## name` section below.
fn type_md(ty: &FieldType) -> String {
    match ty {
        FieldType::Str => "string".into(),
        FieldType::Int => "integer".into(),
        FieldType::Bool => "boolean".into(),
        FieldType::Null => "null".into(),
        FieldType::EnumStr(vals) => format!("string (one of: {})", vals.join(", ")),
        FieldType::Obj(_) => "object".into(),
        FieldType::Arr(item) => format!("array of {}", type_md(item)),
        FieldType::Ref(name) => format!("[{name}]"),
        FieldType::TaggedUnion(_) => "tagged union (see its section)".into(),
        FieldType::Nullable(inner) => format!("{} or null", type_md(inner)),
        FieldType::FreeObj(value) => {
            format!("object (arbitrary keys; values: {})", type_md(value))
        }
        FieldType::Union(alts) => alts.iter().map(type_md).collect::<Vec<_>>().join(" | "),
    }
}

fn field_table(fields: &[Field]) -> String {
    let mut out = String::from("| Field | Type | Required | Description |\n");
    out.push_str("|---|---|---|---|\n");
    for f in fields {
        out.push_str(&format!(
            "| `{}` | {} | {} | {} |\n",
            f.name,
            type_md(&f.ty),
            if f.required {
                "yes"
            } else {
                "no (key omitted)"
            },
            f.note.as_deref().unwrap_or("")
        ));
    }
    out
}

/// The kind table of an internally-tagged union (one row per variant).
fn kind_table(variants: &[TaggedVariant]) -> String {
    let mut out = String::from("| kind | Fields (beyond `kind`) |\n");
    out.push_str("|---|---|\n");
    for v in variants {
        let fields: Vec<String> = v
            .fields
            .iter()
            .map(|f| {
                format!(
                    "`{}`: {}{}",
                    f.name,
                    type_md(&f.ty),
                    if f.required { "" } else { " (optional)" }
                )
            })
            .collect();
        out.push_str(&format!(
            "| `{}` | {} |\n",
            v.kind,
            if fields.is_empty() {
                "-".into()
            } else {
                fields.join(", ")
            }
        ));
    }
    out
}

/// The markdown document: intro + one section per root/definition.
pub fn render_md(doc: &SchemaDoc) -> String {
    let mut out = String::new();
    out.push_str(&format!("# {}\n\n", doc.title));
    out.push_str(&doc.description);
    out.push_str(
        "\n\n`[name]` in the Type column refers to the sub-schema defined in the `## name` section below. `or null` means the key is present with a JSON null value; `no (key omitted)` means the key is absent entirely.\n",
    );

    match &doc.root {
        FieldType::Obj(fields) => {
            out.push_str("\n## document\n\n");
            out.push_str(&field_table(fields));
        }
        // A tagged-union root documents its variants in the document section
        // itself (there is no separate `## name` section for the root).
        FieldType::TaggedUnion(variants) => {
            out.push_str("\n## document\n\n");
            out.push_str(&kind_table(variants));
        }
        other => {
            out.push_str(&format!("\n## document\n\nType: {}\n", type_md(other)));
        }
    }

    for (name, ty) in &doc.definitions {
        out.push_str(&format!("\n## {name}\n\n"));
        match ty {
            FieldType::Obj(fields) => out.push_str(&field_table(fields)),
            FieldType::TaggedUnion(variants) => out.push_str(&kind_table(variants)),
            other => out.push_str(&format!("Type: {}\n", type_md(other))),
        }
    }

    for (heading, body) in &doc.extra_sections {
        out.push_str(&format!("\n## {heading}\n\n{body}"));
    }
    out
}

// ---------------------------------------------------------------------------
// Sync walker (D-P7-9)
// ---------------------------------------------------------------------------

fn json_type_name(v: &Json) -> &'static str {
    match v {
        Json::Null => "null",
        Json::Bool(_) => "boolean",
        Json::Int(_) => "integer",
        Json::Str(_) => "string",
        Json::Arr(_) => "array",
        Json::Obj(_) => "object",
    }
}

fn check_object(
    fields: &[Field],
    pairs: &[(String, Json)],
    path: &str,
    defs: &std::collections::HashMap<&str, &FieldType>,
    errs: &mut Vec<String>,
) {
    let present: std::collections::HashSet<&str> = pairs.iter().map(|(k, _)| k.as_str()).collect();
    for f in fields {
        if f.required && !present.contains(f.name.as_str()) {
            errs.push(format!("missing required key '{}' at {path}", f.name));
        }
    }
    for (k, val) in pairs {
        match fields.iter().find(|f| f.name == *k) {
            Some(f) => check_type(&f.ty, val, &format!("{path}.{k}"), defs, errs),
            None => errs.push(format!("undocumented key '{k}' at {path}")),
        }
    }
}

fn check_type(
    ty: &FieldType,
    v: &Json,
    path: &str,
    defs: &std::collections::HashMap<&str, &FieldType>,
    errs: &mut Vec<String>,
) {
    match ty {
        FieldType::Ref(name) => match defs.get(name.as_str()) {
            Some(t) => check_type(t, v, path, defs, errs),
            None => errs.push(format!("undefined definition '{name}' at {path}")),
        },
        FieldType::Nullable(inner) => {
            if !matches!(v, Json::Null) {
                check_type(inner, v, path, defs, errs);
            }
        }
        FieldType::Str => {
            if !matches!(v, Json::Str(_)) {
                errs.push(format!(
                    "type mismatch at {path}: expected string, got {}",
                    json_type_name(v)
                ));
            }
        }
        FieldType::Int => {
            if !matches!(v, Json::Int(_)) {
                errs.push(format!(
                    "type mismatch at {path}: expected integer, got {}",
                    json_type_name(v)
                ));
            }
        }
        FieldType::Bool => {
            if !matches!(v, Json::Bool(_)) {
                errs.push(format!(
                    "type mismatch at {path}: expected boolean, got {}",
                    json_type_name(v)
                ));
            }
        }
        FieldType::Null => {
            if !matches!(v, Json::Null) {
                errs.push(format!(
                    "type mismatch at {path}: expected null, got {}",
                    json_type_name(v)
                ));
            }
        }
        FieldType::EnumStr(vals) => match v {
            Json::Str(s) => {
                if !vals.iter().any(|x| x == s) {
                    errs.push(format!(
                        "value '{s}' at {path} not in enum {{ {} }}",
                        vals.join(", ")
                    ));
                }
            }
            _ => errs.push(format!(
                "type mismatch at {path}: expected string, got {}",
                json_type_name(v)
            )),
        },
        FieldType::Arr(item) => match v {
            Json::Arr(items) => {
                for (i, it) in items.iter().enumerate() {
                    check_type(item, it, &format!("{path}[{i}]"), defs, errs);
                }
            }
            _ => errs.push(format!(
                "type mismatch at {path}: expected array, got {}",
                json_type_name(v)
            )),
        },
        FieldType::Obj(fields) => match v {
            Json::Obj(pairs) => check_object(fields, pairs, path, defs, errs),
            _ => errs.push(format!(
                "type mismatch at {path}: expected object, got {}",
                json_type_name(v)
            )),
        },
        FieldType::TaggedUnion(variants) => match v {
            Json::Obj(pairs) => {
                let kind = pairs
                    .iter()
                    .find(|(k, _)| k == "kind")
                    .and_then(|(_, val)| val.as_str());
                match kind {
                    Some(k) => match variants.iter().find(|x| x.kind == k) {
                        Some(variant) => {
                            let mut fields = vec![Field::req(
                                "kind",
                                FieldType::enum_str(&[variant.kind.as_str()]),
                                None,
                            )];
                            fields.extend(variant.fields.iter().cloned());
                            check_object(&fields, pairs, path, defs, errs);
                        }
                        None => errs.push(format!(
                            "unknown kind '{k}' at {path} (expected one of: {})",
                            variants
                                .iter()
                                .map(|x| x.kind.as_str())
                                .collect::<Vec<_>>()
                                .join(", ")
                        )),
                    },
                    None => errs.push(format!("missing or non-string 'kind' at {path}")),
                }
            }
            _ => errs.push(format!(
                "type mismatch at {path}: expected object, got {}",
                json_type_name(v)
            )),
        },
        FieldType::FreeObj(value) => match v {
            Json::Obj(pairs) => {
                for (k, val) in pairs {
                    check_type(value, val, &format!("{path}.{k}"), defs, errs);
                }
            }
            _ => errs.push(format!(
                "type mismatch at {path}: expected object, got {}",
                json_type_name(v)
            )),
        },
        FieldType::Union(alts) => {
            // Conforms iff at least one alternative passes cleanly; report
            // the shortest error list otherwise.
            let mut best: Option<Vec<String>> = None;
            for alt in alts {
                let mut e = Vec::new();
                check_type(alt, v, path, defs, &mut e);
                if e.is_empty() {
                    return;
                }
                if best.as_ref().is_none_or(|b| e.len() < b.len()) {
                    best = Some(e);
                }
            }
            if let Some(e) = best {
                errs.extend(e);
            }
        }
    }
}

/// Verify one generated document against the model (D-P7-9): every required
/// key present, no undocumented key at any object level, types and enum
/// constraints agree. Returns all violations (empty = in sync).
pub fn walk_check(doc: &SchemaDoc, value: &Json) -> Result<(), Vec<String>> {
    let defs: std::collections::HashMap<&str, &FieldType> = doc
        .definitions
        .iter()
        .map(|(n, t)| (n.as_str(), t))
        .collect();
    let mut errs = Vec::new();
    check_type(&doc.root, value, "$", &defs, &mut errs);
    if errs.is_empty() {
        Ok(())
    } else {
        Err(errs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tiny_doc() -> SchemaDoc {
        SchemaDoc {
            title: "tiny".into(),
            description: "tiny doc".into(),
            root: FieldType::obj(vec![
                Field::req("a", FieldType::str(), None),
                Field::opt("b", FieldType::int(), None),
                Field::req("c", FieldType::nullable(FieldType::r#ref("thing")), None),
            ]),
            extra_sections: vec![],
            definitions: vec![(
                "thing".into(),
                FieldType::tagged_union(vec![TaggedVariant::new(
                    "x",
                    vec![Field::req("v", FieldType::enum_str(&["p", "q"]), None)],
                )]),
            )],
        }
    }

    #[test]
    fn schema_emits_draft07_with_refs_and_oneof() {
        let doc = tiny_doc();
        let s = emit_schema_pretty(&doc);
        // Round-trips through json_rw (project rule).
        let j = crate::json_rw::parse(&s).expect("schema must be valid JSON");
        assert_eq!(
            j.get("$schema").and_then(|v| v.as_str()),
            Some("http://json-schema.org/draft-07/schema#")
        );
        assert_eq!(j.get("type").and_then(|v| v.as_str()), Some("object"));
        assert_eq!(
            j.get("additionalProperties").and_then(|v| v.as_bool()),
            Some(false)
        );
        let c = j.get("properties").unwrap().get("c").cloned().unwrap();
        // nullable ref -> anyOf with $ref + null
        let anyof = c.get("anyOf").and_then(|v| v.as_arr()).cloned().unwrap();
        assert_eq!(anyof.len(), 2);
        assert!(anyof[0].get("$ref").is_some());
        let thing = j.get("definitions").unwrap().get("thing").cloned().unwrap();
        assert!(thing.get("oneOf").is_some());
    }

    #[test]
    fn walk_check_accepts_conforming_and_rejects_drift() {
        let doc = tiny_doc();
        // Conforming.
        let ok = crate::json_rw::parse(r#"{"a": "x", "c": {"kind": "x", "v": "p"}}"#).unwrap();
        assert!(walk_check(&doc, &ok).is_ok());
        // Missing required key.
        let missing = crate::json_rw::parse(r#"{"c": null}"#).unwrap();
        let errs = walk_check(&doc, &missing).unwrap_err();
        assert!(
            errs.iter().any(|e| e.contains("missing required key 'a'")),
            "{errs:?}"
        );
        // Undocumented key.
        let extra = crate::json_rw::parse(r#"{"a": "x", "c": null, "zzz": 1}"#).unwrap();
        let errs = walk_check(&doc, &extra).unwrap_err();
        assert!(
            errs.iter().any(|e| e.contains("undocumented key 'zzz'")),
            "{errs:?}"
        );
        // Bad enum value.
        let badenum =
            crate::json_rw::parse(r#"{"a": "x", "c": {"kind": "x", "v": "nope"}}"#).unwrap();
        let errs = walk_check(&doc, &badenum).unwrap_err();
        assert!(errs.iter().any(|e| e.contains("not in enum")), "{errs:?}");
        // Unknown kind.
        let badkind = crate::json_rw::parse(r#"{"a": "x", "c": {"kind": "zzz"}}"#).unwrap();
        let errs = walk_check(&doc, &badkind).unwrap_err();
        assert!(
            errs.iter().any(|e| e.contains("unknown kind 'zzz'")),
            "{errs:?}"
        );
    }

    #[test]
    fn md_renders_all_sections() {
        let doc = tiny_doc();
        let md = render_md(&doc);
        assert!(md.starts_with("# tiny\n"), "{md}");
        assert!(md.contains("## document"), "{md}");
        assert!(md.contains("## thing"), "{md}");
        assert!(md.contains("`a` | string | yes"), "{md}");
        assert!(md.contains("[thing] or null"), "{md}");
    }

    #[test]
    fn free_obj_and_union_emit_and_walk() {
        let doc = SchemaDoc {
            title: "free".into(),
            description: "d".into(),
            root: FieldType::obj(vec![Field::req(
                "m",
                FieldType::free_obj(FieldType::union(vec![FieldType::bool(), FieldType::int()])),
                None,
            )]),
            definitions: vec![],
            extra_sections: vec![("extra".into(), "body text".into())],
        };
        let s = emit_schema_pretty(&doc);
        let j = crate::json_rw::parse(&s).expect("valid JSON");
        let m = j.get("properties").unwrap().get("m").cloned().unwrap();
        assert_eq!(m.get("type").and_then(|v| v.as_str()), Some("object"));
        // Free keys: additionalProperties carries the value schema (anyOf),
        // and there is NO properties/required list.
        let ap = m.get("additionalProperties").cloned().unwrap();
        assert!(ap.get("anyOf").is_some(), "{ap:?}");
        assert!(m.get("properties").is_none(), "{m:?}");

        // md: the extra section is appended after the definitions.
        let md = render_md(&doc);
        assert!(md.contains("\n## extra\n\nbody text"), "{md}");
        assert!(
            md.contains("object (arbitrary keys; values: boolean | integer)"),
            "{md}"
        );

        // walk: arbitrary keys accepted, value types enforced.
        let ok = crate::json_rw::parse(r#"{"m": {"a": true, "b": 3}}"#).unwrap();
        assert!(walk_check(&doc, &ok).is_ok());
        let bad = crate::json_rw::parse(r#"{"m": {"a": "s"}}"#).unwrap();
        let errs = walk_check(&doc, &bad).unwrap_err();
        assert!(
            errs.iter().any(|e| e.contains("type mismatch at $.m.a")),
            "{errs:?}"
        );
    }

    #[test]
    fn tagged_union_root_renders_kind_table_in_document_section() {
        let doc = SchemaDoc {
            title: "rooted".into(),
            description: "d".into(),
            root: FieldType::tagged_union(vec![TaggedVariant::new(
                "x",
                vec![Field::req("v", FieldType::int(), None)],
            )]),
            definitions: vec![],
            extra_sections: vec![],
        };
        let md = render_md(&doc);
        assert!(md.contains("## document"), "{md}");
        assert!(md.contains("| kind | Fields (beyond `kind`) |"), "{md}");
        assert!(md.contains("| `x` | `v`: integer |"), "{md}");
    }
}
