//! Type layout schema, used to guide dynamic access to type data in scripts.

use std::alloc::Layout;

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
    fn schema() -> Schema;

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
        if s1.represents(&s2) {
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
        if s1.represents(&s2) {
            // SAFE: the schemas have the same memory representation.
            unsafe { Ok(&mut *(self as *mut Self as *mut T)) }
        } else {
            Err(SchemaMismatchError)
        }
    }
}

/// A schema describes the data layout of a type, to enable dynamic access to the type's data
/// through a pointer.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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
                    Primitive::Opaque { id: _, size, align } => {
                        Layout::from_size_align(*size, *align).unwrap()
                    }
                },
            ),
        }

        layout.unwrap().pad_to_align()
    }

    /// Returns whether or not the schema contains an [`Opaque`][Primitive::Opaque] primitive
    /// anywhere in the nested schema.
    ///
    /// This is important because a cast between two types with the same [`Schema`] is safe, as long
    /// as the schema doesn't contain any `Opaque` primitives.
    ///
    /// If you have two equal schemas that have an `Opaque` primitive, they may or may not have the
    /// same memory layout, and casting to a Rust struct with that schema may not be sound, if that
    /// opaque field is ever accessed.
    pub fn represents(&self, other: &Schema) -> bool {
        match (self, other) {
            (Schema::Struct(s1), Schema::Struct(s2)) => {
                s1.fields.len() == s2.fields.len()
                    && s1
                        .fields
                        .iter()
                        .zip(s2.fields.iter())
                        .all(|(f1, f2)| f1.schema.represents(&f2.schema))
            }
            (Schema::Vec(v1), Schema::Vec(v2)) => v1.represents(v2),
            (Schema::Primitive(p1), Schema::Primitive(p2)) => match (p1, p2) {
                (
                    Primitive::Opaque {
                        id: id1,
                        size: size1,
                        align: align1,
                    },
                    Primitive::Opaque {
                        id: id2,
                        size: size2,
                        align: align2,
                    },
                ) => {
                    // If there is no opaque ID, then we cannot know if these schemas represent the
                    // same memory layout.
                    if id1.is_none() || id2.is_none() {
                        false
                    } else {
                        id1 == id2 && size1 == size2 && align1 == align2
                    }
                }
                (p1, p2) => p1 == p2,
            },
            _ => false,
        }
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
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StructField {
    /// The name of the field. Will be [`None`] if this is a field of a tuple struct.
    pub name: Option<String>,
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
        /// An optional unique ID for the opaque type.
        ///
        /// If this is specified, then it will be used when checking the compatibility of two opaque
        /// types in [`Schema::represents()`]. Two opaque types, both with a set ID, and where the
        /// IDs are equal to each-other, will be considered to have exactly the same memory layout
        /// and can be safely cast from one to the other.
        ///
        /// If this is [`None`] then `represents()` will always return `false`, because we cannot be
        /// sure that the two opaque types actually represent the same exact type layout.
        #[cfg_attr(feature = "serde", serde(default))]
        id: Option<Ulid>,
        /// The size of the data.
        size: usize,
        /// The alignment of the data.
        align: usize,
    },
}
