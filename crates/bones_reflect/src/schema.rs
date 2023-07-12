//! Type layout schema, used to guide dynamic access to type data in scripts.

use std::{alloc::Layout, any::TypeId, borrow::Cow};

use bones_utils::HashMap;
use serde::Deserialize;
use ulid::Ulid;

use crate::prelude::*;

mod ptr;
pub use ptr::*;

mod std_impls;

/// Trait implemented for types that have a [`Schema`].
///
/// # Safety
///
/// This trait is unsafe to implement manually because it makes claims about the memory layout of a
/// type that may be depended on in unsafe code, but it is safe to derive [`HasSchema`] on supported
/// types.
///
/// If implemented manually, you must ensure that the schema accurately describes the memory layout
/// of the type, or else accessing the type according to the schema would be unsound.
///
/// # TODO
///
/// TODO: We need a way to have schemas that don't allocate every time we call
/// [`HasSchema::schema()`]. Maybe we do that by requiring schema() to return a `&'static Schema`.
/// The issue is that means we have to memory leak all schemas essentially, which might be a problem
/// if a program generates a lot of schemas over time?
///
/// Maybe we could instead have a global schema registry and have `schema()` return a handle into
/// that registry ( maybe based on the schema hash? ). Then you could still delete schemas from the
/// registry. I haven't thought all the way through this.
pub unsafe trait HasSchema {
    /// Get this type's [`Schema`].
    fn schema() -> &'static Schema;

    /// Cast a reference of this type to a reference of another type with the same memory layout.
    ///
    /// # Panics
    ///
    /// Panics if the schema of `T` doesn't match the schema of `Self`.
    #[track_caller]
    fn cast<T: HasSchema>(&self) -> &T {
        self.try_cast().expect(SchemaMismatchError::MSG)
    }

    /// Cast a reference of this type to a reference of another type with the same memory layout.
    ///
    /// # Errors
    ///
    /// Errors if the schema of `T` doesn't match the schema of `Self`.
    fn try_cast<T: HasSchema>(&self) -> Result<&T, SchemaMismatchError> {
        let s1 = Self::schema();
        let s2 = T::schema();
        if s1.represents(s2) {
            // SAFE: the schemas have the same memory representation.
            unsafe { Ok(&*(self as *const Self as *const T)) }
        } else {
            Err(SchemaMismatchError)
        }
    }

    /// Cast a mutable reference of this type to a reference of another type with the same memory
    /// layout.
    ///
    /// # Panics
    ///
    /// Panics if the schema of `T` doesn't match the schema of `Self`.
    #[track_caller]
    fn cast_mut<T: HasSchema>(&mut self) -> &mut T {
        self.try_cast_mut().expect(SchemaMismatchError::MSG)
    }

    /// Cast a mutable reference of this type to a reference of another type with the same memory
    /// layout.
    ///
    /// # Errors
    ///
    /// Errors if the schema of `T` doesn't match the schema of `Self`.
    fn try_cast_mut<T: HasSchema>(&mut self) -> Result<&mut T, SchemaMismatchError> {
        let s1 = Self::schema();
        let s2 = T::schema();
        if s1.represents(s2) {
            // SAFE: the schemas have the same memory representation.
            unsafe { Ok(&mut *(self as *mut Self as *mut T)) }
        } else {
            Err(SchemaMismatchError)
        }
    }
}

/// Container for a schema that is nested in another schema.
#[derive(Debug, Clone)]
pub enum NestedSchema {
    /// The nested schema is a static reference to a schema.
    Static(&'static Schema),
    /// The nested schema is an owned box of a schema.
    Boxed(Box<Schema>),
}

impl From<&'static Schema> for NestedSchema {
    fn from(s: &'static Schema) -> Self {
        Self::Static(s)
    }
}
impl From<Schema> for NestedSchema {
    fn from(s: Schema) -> Self {
        Self::Boxed(Box::new(s))
    }
}
impl std::ops::Deref for NestedSchema {
    type Target = Schema;

    fn deref(&self) -> &Self::Target {
        match self {
            NestedSchema::Static(s) => s,
            NestedSchema::Boxed(s) => s,
        }
    }
}
impl<'de> Deserialize<'de> for NestedSchema {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = Schema::deserialize(deserializer)?;
        Ok(NestedSchema::Boxed(Box::new(s)))
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub struct Schema {
    #[serde(skip)]
    /// The Rust [`TypeId`] that this [`Schema`] was created from, if it was created from a Rust
    /// type.
    pub type_id: Option<TypeId>,
    /// The kind of schema.
    pub kind: SchemaKind,
    #[serde(skip)]
    /// Arbitrary type data assocated to the schema.
    ///
    /// The [`Ulid`] key is arbitrary, allows different types to add different kinds of data to the
    /// schema.
    pub type_data: HashMap<Ulid, SchemaBox>,
}

impl From<SchemaKind> for Schema {
    fn from(kind: SchemaKind) -> Self {
        Self {
            type_id: None,
            kind,
            type_data: Default::default(),
        }
    }
}

/// A schema describes the data layout of a type, to enable dynamic access to the type's data
/// through a pointer.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum SchemaKind {
    /// The type represents a struct.
    Struct(StructSchema),
    /// Type represents a Rust [`Vec`], where each item in the vec has the contained [`Schema`].
    ///
    /// Since the type is a Rust [`Vec`] interactions with it must happen through the Rust [`Vec`]
    /// methods.
    ///
    /// The scripting solution must facilitate a way for scripts to access data in the [`Vec`] if it
    /// is to be readable/modifyable from scripts.
    Vec(NestedSchema),
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

        match &self.kind {
            SchemaKind::Struct(s) => {
                for field in &s.fields {
                    let field_layout = field.schema.layout();
                    extend_layout(&mut layout, field_layout);
                }
            }
            SchemaKind::Vec(_) => extend_layout(&mut layout, Layout::new::<Vec<u8>>()),
            SchemaKind::Primitive(p) => extend_layout(
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
                    Primitive::Opaque { size, align } => {
                        Layout::from_size_align(*size, *align).unwrap()
                    }
                },
            ),
        }

        layout.unwrap().pad_to_align()
    }

    /// Recursively checks whether or not the schema contains any [`Opaque`][Primitive::Opaque] primitives.
    pub fn has_opaque(&self) -> bool {
        match &self.kind {
            SchemaKind::Struct(s) => s.fields.iter().any(|field| field.schema.has_opaque()),
            SchemaKind::Vec(v) => v.has_opaque(),
            SchemaKind::Primitive(p) => matches!(p, Primitive::Opaque { .. }),
        }
    }

    /// Returns whether or not this schema represents the same memory layout as the other schema,
    /// and you can safely cast a pointer to one to a pointer to the other.
    pub fn represents(&self, other: &Schema) -> bool {
        // If these have equal Rust type IDs, then they are the same.
        (self.type_id.is_some() && other.type_id.is_some() && self.type_id == other.type_id)
            // If the schemas don't have any opaque fields, and are equal to each-other, then they
            // have the same representation.
            || (!self.has_opaque() && !other.has_opaque() && {
                match (&self.kind, &other.kind) {
                    (SchemaKind::Struct(s1), SchemaKind::Struct(s2)) => {
                        s1.fields.len() == s2.fields.len() &&
                            s1.fields.iter().zip(s2.fields.iter())
                            .all(|(f1, f2)| f1.schema.represents(&f2.schema))
                    },
                    (SchemaKind::Vec(v1), SchemaKind::Vec(v2)) => v1.represents(v2),
                    (SchemaKind::Primitive(p1), SchemaKind::Primitive(p2)) => p1 == p2,
                    _ => false
                }
            })
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
    pub name: Option<Cow<'static, str>>,
    /// The schema of the field.
    pub schema: Schema,
}

/// The type of primitive. In the case of the number types, the size can be determined from the
/// [`Layout`] defined in the [`Schema`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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
    /// Opaque data that cannot described by a schema.
    Opaque {
        /// The size of the data.
        size: usize,
        /// The alignment of the data.
        align: usize,
    },
}
