//! Type layout schema, used to guide dynamic access to type data in scripts.

use std::alloc::Layout;

mod std_impls;

/// Trait implemented for types that have a [`Schema`].
///
/// May be derived.
pub trait HasSchema {
    /// Get this type's [`Schema`].
    fn schema() -> Schema;
}

/// A schema describes the data layout of a type, to enable dynamic access to the type's data
/// through a pointer.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum Schema {
    /// The type represents a struct.
    Struct(StructSchema),
    /// Type represents a Rust [`Vec`], where each item in the vec has the contained [`Schema`].
    ///
    /// Since the type is a Rust [`Vec`] interactions with it must happen through the Rust [`Vec`]
    /// methods.
    ///
    /// The scripting solution must facilitate a way for scripts to access data in the [`Vec`] if it
    /// is to be readable/modifyable from scripts.
    Vec(Box<Schema>),
    /// The type represents a primitive value.
    Primitive(Primitive),
}

impl Schema {
    /// Get the layout of the type represented by the schema.
    pub fn layout(&self) -> Layout {
        let mut layout: Option<Layout> = None;

        let extend_layout = |layout: &mut Option<Layout>, l| {
            if let Some(layout) = layout {
                let (new_layout, _offset) = layout.extend(l).unwrap();
                *layout = new_layout;
            } else {
                *layout = Some(l)
            }
        };

        match self {
            Schema::Struct(s) => {
                for field in &s.fields {
                    let field_layout = field.schema.layout();
                    extend_layout(&mut layout, field_layout);
                }
            }
            Schema::Vec(_) => extend_layout(&mut layout, Layout::new::<Vec<u8>>()),
            Schema::Primitive(p) => extend_layout(
                &mut layout,
                match p {
                    Primitive::Bool => Layout::new::<bool>(),
                    Primitive::U8 => Layout::new::<u8>(),
                    Primitive::U16 => Layout::new::<u16>(),
                    Primitive::U32 => Layout::new::<u32>(),
                    Primitive::U64 => Layout::new::<u64>(),
                    Primitive::U128 => Layout::new::<u128>(),
                    Primitive::I8 => Layout::new::<i8>(),
                    Primitive::I16 => Layout::new::<i16>(),
                    Primitive::I32 => Layout::new::<i32>(),
                    Primitive::I64 => Layout::new::<i64>(),
                    Primitive::I128 => Layout::new::<i128>(),
                    Primitive::F32 => Layout::new::<f32>(),
                    Primitive::F64 => Layout::new::<f64>(),
                    Primitive::String => Layout::new::<String>(),
                    Primitive::Pointer(_) => Layout::new::<*mut u8>(),
                    Primitive::OpaquePointer => Layout::new::<*mut u8>(),
                    Primitive::Opaque { size, align } => {
                        Layout::from_size_align(*size, *align).unwrap()
                    }
                },
            ),
        }

        layout.unwrap().pad_to_align()
    }
}

/// Deserialize able struct for schema files.
///
/// This struct is required because you can't use `serde(with = "..")` directly on the [`Schema`]
/// enum to make it use nested struct syntax instead of YAML tags for enum representation. Avoiding
/// tags is necessary because we use nested enums such as `vec: primitive: string`.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
pub struct SchemaFile {
    /// The schema defined in the file
    #[cfg_attr(
        feature = "serde",
        serde(with = "serde_yaml::with::singleton_map_recursive")
    )]
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub schema: Schema,
}

/// The schema for a struct.
#[derive(Debug, Clone)]
pub struct StructSchema {
    /// The fields in the struct, in the order they are defined.
    pub fields: Vec<StructField>,
}

#[cfg(feature = "serde")]
mod ser_de {
    use super::*;

    impl<'de> serde::Deserialize<'de> for StructSchema {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            deserializer.deserialize_any(StructSchemaVisitor)
        }
    }

    struct StructSchemaVisitor;
    impl<'de> serde::de::Visitor<'de> for StructSchemaVisitor {
        type Value = StructSchema;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(formatter, "a struct definition")
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::SeqAccess<'de>,
        {
            let mut struct_schema = StructSchema {
                fields: Vec::with_capacity(seq.size_hint().unwrap_or(0)),
            };
            while let Some(schema) = seq.next_element()? {
                struct_schema
                    .fields
                    .push(StructField { name: None, schema });
            }
            Ok(struct_schema)
        }

        fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::MapAccess<'de>,
        {
            let mut struct_schema = StructSchema {
                fields: Vec::with_capacity(map.size_hint().unwrap_or(0)),
            };
            while let Some((name, schema)) = map.next_entry()? {
                struct_schema.fields.push(StructField {
                    name: Some(name),
                    schema,
                });
            }
            Ok(struct_schema)
        }
    }
}

/// A field in a [`StructSchema`].
#[derive(Debug, Clone)]
pub struct StructField {
    /// The name of the field. Will be [`None`] if this is a field of a tuple struct.
    pub name: Option<String>,
    /// The schema of the field.
    pub schema: Schema,
}

/// The type of primitive. In the case of the number types, the size can be determined from the
/// [`Layout`] defined in the [`Schema`].
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
#[serde(rename_all = "snake_case")]
pub enum Primitive {
    /// A boolean.
    Bool,
    /// [`u8`]
    U8,
    /// [`u16`]
    U16,
    /// [`u32`]
    U32,
    /// [`u64`]
    U64,
    /// [`u128`]
    U128,
    /// [`i8`]
    I8,
    /// [`i16`]
    I16,
    /// [`i32`]
    I32,
    /// [`i64`]
    I64,
    /// [`i128`]
    I128,
    /// [`f32`]
    F32,
    /// [`f64`]
    F64,
    /// A Rust [`String`]. Must be manipulated with Rust string methods.
    String,
    /// A pointer to a type with the given schema.
    Pointer(Box<Schema>),
    /// A pointer to an opaque type that cannot be described by a schema.
    OpaquePointer,
    /// Opaque data that cannot described by a schema.
    Opaque {
        /// The size of the data.
        size: usize,
        /// The alignment of the data.
        align: usize,
    },
}
