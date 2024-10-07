use std::ffi::c_void;

use bones_schema::alloc::TypeDatas;
use serde::Deserialize;
use ustr::ustr;

use crate::prelude::*;

/// A wrapper around a [`&'statid Schema`][Schema] that can be deserialized for use
/// in asset pack schema files.
pub struct PackSchema(pub &'static Schema);

#[derive(Deserialize)]
struct SchemaMeta {
    name: String,
    full_name: String,
    kind: SchemaKindMeta,
    #[serde(default)]
    asset_extension: Option<String>,
}

#[derive(Deserialize)]
enum SchemaKindMeta {
    Struct(StructMeta),
}

#[derive(Deserialize)]
struct StructMeta {
    #[serde(default)]
    fields: Vec<StructFieldMeta>,
}

#[derive(Deserialize)]
struct StructFieldMeta {
    #[serde(default)]
    name: Option<String>,
    schema: NestedSchema,
}

struct NestedSchema(&'static Schema);

impl<'de> Deserialize<'de> for NestedSchema {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let schema = deserializer.deserialize_any(NestedSchemaVisitor)?;
        Ok(NestedSchema(schema))
    }
}

struct NestedSchemaVisitor;
use serde::de::Error;
impl<'de> serde::de::Visitor<'de> for NestedSchemaVisitor {
    type Value = &'static Schema;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            formatter,
            "either an inline schema definition or a string name of the desired schema"
        )
    }

    fn visit_str<E>(self, name: &str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        for schema in SCHEMA_REGISTRY.schemas.iter() {
            if schema.name == name || schema.full_name == name {
                return Ok(schema);
            }
        }
        Err(E::custom(format!("Schema named `{name}` not found.")))
    }

    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(PackSchema::deserialize(deserializer)?.0)
    }
}

impl<'de> Deserialize<'de> for PackSchema {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let meta = SchemaMeta::deserialize(deserializer)?;

        let schema_kind = match meta.kind {
            SchemaKindMeta::Struct(info) => SchemaKind::Struct(StructSchemaInfo {
                fields: info
                    .fields
                    .into_iter()
                    .map(|field| StructFieldInfo {
                        name: field.name.as_deref().map(ustr),
                        schema: field.schema.0,
                    })
                    .collect(),
            }),
        };

        let name = ustr(&meta.name);
        let full_name = ustr(&meta.full_name);

        let schema_kind_ref = &*Box::leak(Box::new(schema_kind.clone()));
        let layout_info = &*Box::leak(Box::new(schema_kind_ref.compute_layout_info()));
        let default_fn = move |ptr: *mut c_void| {
            let schema_kind = schema_kind_ref;
            match schema_kind {
                SchemaKind::Struct(s) => {
                    for ((_, offset), info) in layout_info.field_offsets.iter().zip(&s.fields) {
                        if let Some(default_fn) = &info.schema.default_fn {
                            unsafe {
                                let field_ptr = ptr.add(*offset);
                                default_fn.get()(field_ptr);
                            }
                        } else {
                            panic!(
                                "Not all fields of `{full_name}` have a default implementation."
                            );
                        }
                    }
                }
                SchemaKind::Vec(_) => todo!(),
                SchemaKind::Enum(_) => todo!(),
                SchemaKind::Map { .. } => todo!(),
                SchemaKind::Box(_) => todo!(),
                SchemaKind::Primitive(_) => todo!(),
            }
        };
        let clone_fn = move |src: *const c_void, dst: *mut c_void| {
            let schema_kind = schema_kind_ref;
            match schema_kind {
                SchemaKind::Struct(s) => {
                    for ((_, offset), info) in layout_info.field_offsets.iter().zip(&s.fields) {
                        if let Some(clone_fn) = &info.schema.clone_fn {
                            unsafe {
                                let src_field = src.add(*offset);
                                let dst_field = dst.add(*offset);
                                clone_fn.get()(src_field, dst_field);
                            }
                        } else {
                            panic!("Not all fields of `{full_name}` have a clone implementation.");
                        }
                    }
                }
                SchemaKind::Vec(_) => todo!(),
                SchemaKind::Enum(_) => todo!(),
                SchemaKind::Map { .. } => todo!(),
                SchemaKind::Box(_) => todo!(),
                SchemaKind::Primitive(_) => todo!(),
            }
        };
        let drop_fn = move |ptr: *mut c_void| {
            let schema_kind = schema_kind_ref;
            match schema_kind {
                SchemaKind::Struct(s) => {
                    for ((_, offset), info) in layout_info.field_offsets.iter().zip(&s.fields) {
                        if let Some(drop_fn) = &info.schema.drop_fn {
                            unsafe {
                                let field_ptr = ptr.add(*offset);
                                drop_fn.get()(field_ptr);
                            }
                        }
                    }
                }
                SchemaKind::Vec(_) => todo!(),
                SchemaKind::Enum(_) => todo!(),
                SchemaKind::Map { .. } => todo!(),
                SchemaKind::Box(_) => todo!(),
                SchemaKind::Primitive(_) => todo!(),
            }
        };

        let type_data = TypeDatas::default();
        if let Some(ext) = meta.asset_extension {
            type_data
                .insert(AssetKind::Metadata { extension: ext })
                .unwrap();
        }

        let schema_data = SchemaData {
            name,
            full_name,
            kind: schema_kind,
            type_data,
            type_id: None,
            clone_fn: Some(unsafe { Unsafe::new(Box::leak(Box::new(clone_fn))) }),
            drop_fn: Some(unsafe { Unsafe::new(Box::leak(Box::new(drop_fn))) }),
            default_fn: Some(unsafe { Unsafe::new(Box::leak(Box::new(default_fn))) }),
            hash_fn: None,
            eq_fn: None,
        };

        let schema = SCHEMA_REGISTRY.register(schema_data);
        Ok(PackSchema(schema))
    }
}
