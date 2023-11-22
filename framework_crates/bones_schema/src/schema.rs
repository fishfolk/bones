//! [`Schema`], [`HasSchema`], [`SchemaData`], and related types.

use std::{alloc::Layout, any::TypeId, borrow::Cow, ffi::c_void};

use bones_utils::Ustr;

use crate::{alloc::TypeDatas, prelude::*};

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
pub unsafe trait HasSchema: Sync + Send + 'static {
    /// Get this type's [`Schema`].
    #[must_use = "If you are only calling schema() for it's side-effect of registering the
        schema, it is more clear to use register_schema() instead."]
    fn schema() -> &'static Schema;

    /// Register this schema with the global schema registry.
    ///
    /// This is automatically done by the framework in many cases, whenever
    /// [`schema()`][HasSchema::schema] is called, but it may be necessary sometimes to
    /// manually register it.
    fn register_schema() {
        let _ = Self::schema();
    }

    /// Cast a reference of this type to a reference of another type with the same memory layout.
    ///
    /// # Panics
    ///
    /// Panics if the schema of `T` doesn't match the schema of `Self`.
    #[track_caller]
    fn cast<T: HasSchema>(this: &Self) -> &T {
        HasSchema::try_cast(this).expect(SchemaMismatchError::MSG)
    }

    /// Cast a reference of this type to a reference of another type with the same memory layout.
    ///
    /// # Errors
    ///
    /// Errors if the schema of `T` doesn't match the schema of `Self`.
    fn try_cast<T: HasSchema>(this: &Self) -> Result<&T, SchemaMismatchError> {
        let s1 = Self::schema();
        let s2 = T::schema();
        if s1.represents(s2) {
            // SOUND: the schemas have the same memory representation.
            unsafe { Ok(&*(this as *const Self as *const T)) }
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
    fn cast_mut<T: HasSchema>(this: &mut Self) -> &mut T {
        HasSchema::try_cast_mut(this).expect(SchemaMismatchError::MSG)
    }

    /// Cast a mutable reference of this type to a reference of another type with the same memory
    /// layout.
    ///
    /// # Errors
    ///
    /// Errors if the schema of `T` doesn't match the schema of `Self`.
    fn try_cast_mut<T: HasSchema>(this: &mut Self) -> Result<&mut T, SchemaMismatchError> {
        let s1 = Self::schema();
        let s2 = T::schema();
        if s1.represents(s2) {
            // SOUND: the schemas have the same memory representation.
            unsafe { Ok(&mut *(this as *mut Self as *mut T)) }
        } else {
            Err(SchemaMismatchError)
        }
    }

    /// Converts a reference of `T` to a [`SchemaRef`]
    fn as_schema_ref(&self) -> SchemaRef
    where
        Self: Sized,
    {
        SchemaRef::new(self)
    }

    /// Converts a reference of `T` to a [`SchemaRefMut`]
    fn as_schema_mut(&mut self) -> SchemaRefMut
    where
        Self: Sized,
    {
        SchemaRefMut::new(self)
    }
}

// Export the `Schema` type so it appears in this module. It is defined in the registry module so
// that the registry is the only module that is allowed to construct `Schema`s.
#[doc(inline)]
pub use crate::registry::Schema;

impl Schema {
    /// Returns whether or not this schema represents the same memory layout as the other schema,
    /// and you can safely cast a pointer to one to a pointer to the other.
    pub fn represents(&self, other: &Schema) -> bool {
        // If these have equal type/schema ids.
        self == other
        // If the schemas don't have any opaque fields, and are equal to each-other, then they
        // have the same representation.
        || (!self.kind.has_opaque() && !other.kind.has_opaque() && {
            match (&self.kind, &other.kind) {
                (SchemaKind::Struct(s1), SchemaKind::Struct(s2)) => {
                    s1.fields.len() == s2.fields.len() &&
                        s1.fields.iter().zip(s2.fields.iter())
                        .all(|(f1, f2)| f1.schema.represents(f2.schema))
                },
                (SchemaKind::Vec(v1), SchemaKind::Vec(v2)) => v1.represents(v2),
                (SchemaKind::Primitive(p1), SchemaKind::Primitive(p2)) => p1 == p2,
                (SchemaKind::Enum(e1), SchemaKind::Enum(e2)) => e1 == e2,
                _ => false
            }
        })
    }
}

/// Schema information describing the memory layout of a type.
#[derive(Clone)]
pub struct SchemaData {
    /// The short name of the type.
    ///
    /// **Note:** Currently bones isn't very standardized as far as name generation for Rust or
    /// other language type names, and this is mostly for diagnostics. This may change in the future
    /// but for now there are no guarantees.
    pub name: Ustr,
    /// The full name of the type, including any module specifiers.
    ///
    /// **Note:** Currently bones isn't very standardized as far as name generation for Rust or
    /// other language type names, and this is mostly for diagnostics. This may change in the future
    /// but for now there are no guarantees.
    pub full_name: Ustr,
    /// The kind of schema.
    pub kind: SchemaKind,
    /// Container for storing [`Schema`] type datas.
    ///
    /// "Type data" is extra data that is stored in a type's [`Schema`] that may be used for any number
    /// of different purposes.
    ///
    /// Each type data is a type that implements [`HasSchema`] and usually describes something about the
    /// type that has the schema. For instance, a type data could be added to a struct that can be used
    /// to serialize/deserialize that type.
    ///
    /// If a type data also implements [`FromType`] it can be derived for types that it implements
    /// [`FromType`] for:
    ///
    /// ```rust
    /// # use bones_schema::prelude::*;
    /// #[derive(HasSchema, Default, Clone)]
    /// struct SomeTypeData;
    ///
    /// impl<T> FromType<T> for SomeTypeData {
    ///     fn from_type() -> Self {
    ///         SomeTypeData
    ///     }
    /// }
    ///
    /// #[derive(HasSchema, Default, Clone)]
    /// #[derive_type_data(SomeTypeData)]
    /// struct MyData;
    /// ```
    pub type_data: TypeDatas,

    // NOTE: The fields below could be implemented as type datas, and it would be nicely elegant to
    // do so, but for performance reasons, we put them right in the [`Schema`] struct because
    // they're use is so common. If profiling does not reveal any performance issues with using them
    // as type datas, we may want to remove these fields in favor of the type data.
    /// The Rust [`TypeId`] that this [`Schema`] was created from, if it was created from a Rust
    /// type.
    pub type_id: Option<TypeId>,
    /// The function pointer that may be used to clone data with this schema.
    pub clone_fn:
        Option<Unsafe<&'static (dyn Fn(*const c_void, *mut c_void) + Sync + Send + 'static)>>,
    /// The function pointer that may be used to drop data with this schema.
    pub drop_fn: Option<Unsafe<&'static (dyn Fn(*mut c_void) + Sync + Send + 'static)>>,
    /// The function pointer that may be used to write a default value to a pointer.
    pub default_fn: Option<Unsafe<&'static (dyn Fn(*mut c_void) + Sync + Send + 'static)>>,
    /// The function pointer that may be used to hash the value.
    pub hash_fn: Option<Unsafe<&'static (dyn Fn(*const c_void) -> u64 + Sync + Send + 'static)>>,
    /// The function pointer that may be used to compare two values for equality. Note that this is
    /// total equality, not partial equality.
    pub eq_fn: Option<
        Unsafe<&'static (dyn Fn(*const c_void, *const c_void) -> bool + Sync + Send + 'static)>,
    >,
}

impl std::fmt::Debug for SchemaData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SchemaData")
            .field("name", &self.name)
            .field("full_name", &self.full_name)
            .field("kind", &self.kind)
            .field("type_data", &self.type_data)
            .field("type_id", &self.type_id)
            .finish_non_exhaustive()
    }
}

/// A wrapper struct that marks it unsafe to both create and access the inner value.
#[derive(Clone, Debug)]
pub struct Unsafe<T>(T);

impl<T> Unsafe<T> {
    /// Create a new [`Unsafe`] contianing the `value`.
    /// # Safety
    /// The safety invariants are dependent on the inner type.
    pub unsafe fn new(value: T) -> Self {
        Self(value)
    }

    /// Unsafely get the inner value.
    /// # Safety
    /// The safety invariants are dependent on the inner type.
    pub unsafe fn get(&self) -> &T {
        &self.0
    }
}

/// A schema describes the data layout of a type, to enable dynamic access to the type's data
/// through a pointer.
#[derive(Debug, Clone)]
pub enum SchemaKind {
    /// The type represents a struct.
    Struct(StructSchemaInfo),
    /// Type represents a [`SchemaVec`], where each item in the vec has the contained [`Schema`].
    ///
    /// The scripting solution must facilitate a way for scripts to access data in the [`Vec`] if it
    /// is to be readable/modifyable from scripts.
    Vec(&'static Schema),
    /// Type represents an enum, which in the C layout is called a tagged union.
    Enum(EnumSchemaInfo),
    /// Type represents a [`SchemaMap`].
    Map {
        /// The schema of the key type.
        key: &'static Schema,
        /// The schema of the value type.
        value: &'static Schema,
    },
    /// The represents a [`SchemaBox`] with given type inside.
    Box(&'static Schema),
    /// The type represents a primitive value.
    Primitive(Primitive),
}

impl SchemaKind {
    /// Get the primitive, if this is a primitive.
    pub fn as_primitive(&self) -> Option<&Primitive> {
        if let Self::Primitive(p) = self {
            Some(p)
        } else {
            None
        }
    }
    /// Get the struct, if this is a struct.
    pub fn as_struct(&self) -> Option<&StructSchemaInfo> {
        if let Self::Struct(s) = self {
            Some(s)
        } else {
            None
        }
    }
    /// Get the enum, if this is a enum.
    pub fn as_enum(&self) -> Option<&EnumSchemaInfo> {
        if let Self::Enum(e) = self {
            Some(e)
        } else {
            None
        }
    }
    /// Get the schema of the items in the vector, if this is a vector.
    pub fn as_vec(&self) -> Option<&'static Schema> {
        if let Self::Vec(v) = self {
            Some(v)
        } else {
            None
        }
    }
}

/// Layout information computed for [`SchemaData`].
#[derive(Debug, Clone)]
pub struct SchemaLayoutInfo<'a> {
    /// The layout of the type.
    pub layout: Layout,
    /// If this is a struct, then the field offsets will contain the byte offset from the front of
    /// the struct to each field in the order the fields are declared.
    ///
    /// If this is an enum, there will be exactly one entry specifying the offset from the beginning
    /// of the struct to the enum value.
    pub field_offsets: Vec<(Option<&'a str>, usize)>,
}

/// Schema data for a struct.
#[derive(Debug, Clone)]
pub struct StructSchemaInfo {
    /// The fields in the struct, in the order they are defined.
    pub fields: Vec<StructFieldInfo>,
}

/// Schema data for an enum.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnumSchemaInfo {
    /// The layout of the enum tag.
    pub tag_type: EnumTagType,
    /// Info for the enum variants.
    pub variants: Vec<VariantInfo>,
}

/// A type for an enum tag for [`EnumSchemaInfo`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnumTagType {
    /// A [`u8`].
    U8,
    /// A [`u16`].
    U16,
    /// A [`u32`].
    U32,
}

impl EnumTagType {
    /// Get the memory layout of the enum tag.
    pub fn layout(&self) -> Layout {
        match self {
            EnumTagType::U8 => Layout::new::<u8>(),
            EnumTagType::U16 => Layout::new::<u16>(),
            EnumTagType::U32 => Layout::new::<u32>(),
        }
    }
}

/// Information about an enum variant for [`EnumSchemaInfo`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariantInfo {
    /// The name of the enum variant.
    pub name: Cow<'static, str>,
    /// The schema of this variant.
    pub schema: &'static Schema,
}

/// A field in a [`StructSchemaInfo`].
#[derive(Debug, Clone)]
pub struct StructFieldInfo {
    /// The name of the field. Will be [`None`] if this is a field of a tuple struct.
    pub name: Option<Ustr>,
    /// The schema of the field.
    pub schema: &'static Schema,
    // TODO: Investigate adding attribute info to `StructFieldInfo`.
    // It could be very useful if the derive macro could capture custom attribute data for struct
    // fields and put it into the schema data. This would allow type data implementations that
    // implement `FromType` to have access to custom attributes that could be used to modify various
    // behavior, without requiring a new macro.
}

/// A type of primitive.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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

/// Trait implemented for types that can produce an instance of themselves from a Rust type.
///
/// This is useful for [type datas][`SchemaData::type_data`], which may be derived for a type if the type data
/// implements [`FromType`] for type that is deriving it.
pub trait FromType<T> {
    /// Return the data for the type.
    fn from_type() -> Self;
}

impl SchemaKind {
    /// Calculate the layout of the type represented by the schema.
    ///
    /// Usually you don't need to call this and should use the static, cached layout and field
    /// offsets from [`Schema::layout()`] and [`Schema::field_offsets()`].
    pub fn compute_layout_info(&self) -> SchemaLayoutInfo<'_> {
        let mut layout: Option<Layout> = None;
        let mut field_offsets = Vec::new();
        let mut offset;

        let extend_layout = |layout: &mut Option<Layout>, l| {
            if let Some(layout) = layout {
                let (new_layout, offset) = layout.extend(l).unwrap();
                *layout = new_layout;
                offset
            } else {
                *layout = Some(l);
                0
            }
        };

        match &self {
            SchemaKind::Struct(s) => {
                for field in &s.fields {
                    let field_layout_info = field.schema.kind.compute_layout_info();
                    offset = extend_layout(&mut layout, field_layout_info.layout);
                    field_offsets.push((field.name.as_deref(), offset));
                }
            }
            SchemaKind::Vec(_) => {
                extend_layout(&mut layout, Layout::new::<SchemaVec>());
            }
            SchemaKind::Box(_) => {
                extend_layout(&mut layout, Layout::new::<SchemaBox>());
            }
            SchemaKind::Map { .. } => {
                extend_layout(&mut layout, Layout::new::<SchemaMap>());
            }
            SchemaKind::Enum(e) => {
                let enum_tag_layout = e.tag_type.layout();
                extend_layout(&mut layout, enum_tag_layout);

                let mut enum_value_layout = Layout::from_size_align(0, 1).unwrap();
                // The enum layout is the greatest of it's variants sizes and aligments
                for var in &e.variants {
                    let var_layout = var.schema.layout();
                    if var_layout.size() > enum_value_layout.size() {
                        enum_value_layout =
                            Layout::from_size_align(var_layout.size(), enum_value_layout.align())
                                .unwrap();
                    }
                    if var_layout.align() > enum_value_layout.align() {
                        enum_value_layout =
                            Layout::from_size_align(enum_value_layout.size(), var_layout.align())
                                .unwrap();
                    }
                }
                let offset = extend_layout(&mut layout, enum_value_layout);
                // Enums have a single field offset that is the offset of the value part of the enum
                // comming after the discriminant.
                field_offsets.push((None, offset));
            }
            SchemaKind::Primitive(p) => {
                extend_layout(
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
                );
            }
        }

        SchemaLayoutInfo {
            layout: layout
                // Handle ZST
                .unwrap_or_else(|| Layout::from_size_align(0, 1).unwrap())
                .pad_to_align(),
            field_offsets,
        }
    }

    /// Recursively checks whether or not the schema contains any [`Opaque`][Primitive::Opaque] primitives.
    pub fn has_opaque(&self) -> bool {
        match &self {
            SchemaKind::Struct(s) => s.fields.iter().any(|field| field.schema.kind.has_opaque()),
            SchemaKind::Vec(v) => v.kind.has_opaque(),
            SchemaKind::Box(b) => b.schema().kind.has_opaque(),
            SchemaKind::Enum(e) => e.variants.iter().any(|var| var.schema.kind.has_opaque()),
            SchemaKind::Map { key, value } => {
                key.schema().kind.has_opaque() || value.schema().kind.has_opaque()
            }
            SchemaKind::Primitive(p) => matches!(p, Primitive::Opaque { .. }),
        }
    }
}
