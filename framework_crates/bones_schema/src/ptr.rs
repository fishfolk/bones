//! Schema-aware smart pointers.

use std::{
    alloc::handle_alloc_error,
    any::{type_name, TypeId},
    ffi::c_void,
    hash::Hash,
    iter::{Filter, Map},
    marker::PhantomData,
    mem::MaybeUninit,
    ptr::NonNull,
    str::Split,
    sync::OnceLock,
};

use bones_utils::prelude::*;
use parking_lot::RwLock;
use ustr::Ustr;

use crate::{
    prelude::*,
    raw_fns::{RawClone, RawDefault, RawDrop},
};

/// An untyped reference that knows the [`Schema`] of the pointee and that can be cast to a matching
/// type.
#[derive(Clone, Copy)]
pub struct SchemaRef<'pointer> {
    ptr: NonNull<c_void>,
    schema: &'static Schema,
    _phantom: PhantomData<&'pointer ()>,
}

impl<'pointer> SchemaRef<'pointer> {
    /// Unsafely cast this pointer to a specifc Rust type.
    /// # Safety
    /// All of the safety requirements of [`NonNull::as_ref()`] must be met.
    pub unsafe fn cast_unchecked<T>(&self) -> &T {
        self.ptr.cast::<T>().as_ref()
    }

    /// Unsafely cast this pointer to a specifc Rust type.
    /// # Safety
    /// All of the safety requirements of [`NonNull::as_ref()`] must be met.
    pub unsafe fn cast_into_unchecked<T>(self) -> &'pointer T {
        self.ptr.cast::<T>().as_ref()
    }

    /// Cast this pointer to a reference to a type with a matching [`Schema`].
    ///
    /// # Panics
    ///
    /// Panics if the schema of the pointer does not match that of the type you are casting to.
    #[track_caller]
    pub fn cast<T: HasSchema>(&self) -> &'pointer T {
        self.try_cast().expect(SchemaMismatchError::MSG)
    }

    /// Cast this pointer to a reference to a type with a matching [`Schema`].
    ///
    /// # Errors
    ///
    /// Errors if the schema of the pointer does not match that of the type you are casting to.
    pub fn try_cast<T: HasSchema>(&self) -> Result<&'pointer T, SchemaMismatchError> {
        if self.schema.represents(T::schema()) {
            // SOUND: the schemas have the same memory representation.
            Ok(unsafe { self.cast_into_unchecked() })
        } else {
            Err(SchemaMismatchError)
        }
    }

    /// Create a new [`SchemaRef`] from a reference to a type that implements [`HasSchema`].
    pub fn new<T: HasSchema>(v: &'pointer T) -> SchemaRef<'pointer> {
        let schema = T::schema();
        SchemaRef {
            // SOUND: The &T passed in cannot be null.
            ptr: unsafe { NonNull::new_unchecked(v as *const T as *mut c_void) },
            schema,
            _phantom: PhantomData,
        }
    }

    /// Create a new [`SchemaRef`] from a raw pointer and it's schema.
    ///
    /// # Safety
    /// - The pointee of `ptr` must be accurately described by the given `schema`.
    /// - `inner` must have correct provenance to allow read of the pointee type.
    /// - The pointer must be valid for the full lifetime of this [`SchemaRef`].
    #[track_caller]
    pub unsafe fn from_ptr_schema(ptr: *const c_void, schema: &'static Schema) -> Self {
        Self {
            ptr: NonNull::new_unchecked(ptr as *mut c_void),
            schema,
            _phantom: PhantomData,
        }
    }

    /// Get the pointer.
    pub fn as_ptr(&self) -> *const c_void {
        self.ptr.as_ptr()
    }

    /// Get the [`Schema`] for the pointer.
    pub fn schema(&self) -> &'static Schema {
        self.schema
    }

    /// Get the hash of this schema box, if supported.
    pub fn hash(&self) -> Option<u64> {
        self.schema
            .hash_fn
            .as_ref()
            .map(|hash_fn| unsafe { (hash_fn.get())(self.ptr.as_ptr()) })
    }

    /// Borrow the schema ref as a [`SchemaMap`] if it is one.
    pub fn as_map(&self) -> Option<&'pointer SchemaMap> {
        matches!(self.schema.kind, SchemaKind::Map { .. })
            // SOUND: Schema asserts this is a schema map
            .then(|| unsafe { self.cast_into_unchecked::<SchemaMap>() })
    }

    /// Borrow the schema ref as a [`SchemaVec`] if it is one.
    pub fn as_vec(&self) -> Option<&'pointer SchemaVec> {
        matches!(self.schema.kind, SchemaKind::Vec(_))
            // SOUND: Schema asserts this is a schema map
            .then(|| unsafe { self.cast_into_unchecked::<SchemaVec>() })
    }

    /// Borrow the schema ref as a [`SchemaBox`] if it is one.
    pub fn as_box(&self) -> Option<SchemaRef<'pointer>> {
        matches!(self.schema.kind, SchemaKind::Vec(_))
            // SOUND: Schema asserts this is a schema box
            .then(|| unsafe { self.cast_into_unchecked::<SchemaBox>().as_ref() })
    }

    /// Debug format the value stored in the schema box.
    ///
    /// This is used in the display and debug implementations.
    pub fn debug_format_value(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_fmt(format_args!("{:?}", self.access_borrowed()))
    }

    /// Get a helper to access the inner data.
    pub fn access(self) -> SchemaRefAccess<'pointer> {
        SchemaRefAccess::new(self)
    }

    /// Get a helper to access the inner without consuming this reference.
    fn access_borrowed(&self) -> SchemaRefAccess {
        SchemaRefAccess::new_borrowed(self)
    }

    /// Get the reference to a field.
    pub fn field<'a, I: Into<FieldIdx<'a>>>(self, field_idx: I) -> Option<SchemaRef<'pointer>> {
        Some(self.access().field(field_idx)?.into_schema_ref())
    }

    /// Get the field pointed to by the given path.
    pub fn field_path<'a, I: IntoIterator<Item = FieldIdx<'a>>>(self, path: I) -> Option<Self> {
        let mut current_field = self;
        for field_idx in path {
            current_field = current_field.field(field_idx)?;
        }
        Some(current_field)
    }

    /// Clone this schema ref into a new box.
    pub fn clone_into_box(&self) -> SchemaBox {
        let Some(clone_fn) = &self.schema.clone_fn else {
            panic!(
                "The schema for type `{}` does not allow cloning it.",
                self.schema.full_name
            );
        };
        unsafe {
            let b = SchemaBox::uninitialized(self.schema);
            (clone_fn.get())(self.ptr.as_ptr(), b.ptr.as_ptr());
            b
        }
    }
}

struct SchemaRefValueDebug<'a>(SchemaRef<'a>);
impl std::fmt::Debug for SchemaRefValueDebug<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.debug_format_value(f)
    }
}

impl std::fmt::Debug for SchemaRef<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("SchemaRef<'_>")
            .field(&SchemaRefValueDebug(*self))
            .finish()
    }
}
impl std::fmt::Display for SchemaRef<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        <SchemaRefValueDebug as std::fmt::Debug>::fmt(&SchemaRefValueDebug(*self), f)
    }
}

/// Helper for accessing the inner data of a schema ref at runtime.
#[derive(Clone, Copy)]
pub enum SchemaRefAccess<'a> {
    /// Access a struct.
    Struct(StructRefAccess<'a>),
    /// Access a vec.
    Vec(SchemaVecAccess<'a>),
    /// Access an enum.
    Enum(EnumRefAccess<'a>),
    /// Access a map.
    Map(SchemaMapAccess<'a>),
    /// Access a struct.
    Primitive(PrimitiveRef<'a>),
}

impl std::fmt::Debug for SchemaRefAccess<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SchemaRefAccess::Struct(s) => {
                let is_tuple = s.fields().any(|x| x.name.is_none());
                if is_tuple {
                    let mut builder = f.debug_tuple(&s.schema().name);
                    for field in s.fields() {
                        builder.field(&SchemaRefValueDebug(field.value));
                    }
                    builder.finish()
                } else {
                    let mut builder = f.debug_struct(&s.schema().name);
                    for field in s.fields() {
                        builder.field(field.name.unwrap(), &SchemaRefValueDebug(field.value));
                    }
                    builder.finish()
                }
            }
            SchemaRefAccess::Vec(v) => {
                let mut builder = f.debug_list();
                for item in v.iter() {
                    builder.entry(&SchemaRefValueDebug(item));
                }
                builder.finish()
            }
            SchemaRefAccess::Enum(e) => {
                f.write_fmt(format_args!("{:?}", SchemaRefValueDebug(e.value().0)))
            }
            SchemaRefAccess::Map(m) => {
                let mut builder = f.debug_map();
                for (key, value) in m.iter() {
                    builder.key(&SchemaRefValueDebug(key));
                    builder.value(&SchemaRefValueDebug(value));
                }
                builder.finish()
            }
            SchemaRefAccess::Primitive(p) => match p {
                PrimitiveRef::Bool(b) => f.write_fmt(format_args!("{b}")),
                PrimitiveRef::U8(n) => f.write_fmt(format_args!("{n}")),
                PrimitiveRef::U16(n) => f.write_fmt(format_args!("{n}")),
                PrimitiveRef::U32(n) => f.write_fmt(format_args!("{n}")),
                PrimitiveRef::U64(n) => f.write_fmt(format_args!("{n}")),
                PrimitiveRef::U128(n) => f.write_fmt(format_args!("{n}")),
                PrimitiveRef::I8(n) => f.write_fmt(format_args!("{n}")),
                PrimitiveRef::I16(n) => f.write_fmt(format_args!("{n}")),
                PrimitiveRef::I32(n) => f.write_fmt(format_args!("{n}")),
                PrimitiveRef::I64(n) => f.write_fmt(format_args!("{n}")),
                PrimitiveRef::I128(n) => f.write_fmt(format_args!("{n}")),
                PrimitiveRef::F32(n) => f.write_fmt(format_args!("{n}")),
                PrimitiveRef::F64(n) => f.write_fmt(format_args!("{n}")),
                PrimitiveRef::String(s) => f.write_fmt(format_args!("{s:?}")),
                PrimitiveRef::Opaque {
                    size,
                    align,
                    schema_ref,
                } => f
                    .debug_tuple(&schema_ref.schema.name)
                    .field(&Primitive::Opaque {
                        size: *size,
                        align: *align,
                    })
                    .finish(),
            },
        }
    }
}

impl<'ptr> SchemaRefAccess<'ptr> {
    /// Create a new [`SchemaRefAccess`] for the given [`SchemaRef`].
    ///
    /// This will create a new independent [`SchemaRefAccess`] that may be used even after
    /// the original [`SchemaRef`] is dropped ( but not beyond the safe lifetime of the
    /// original schema ref ).
    pub fn new(value: SchemaRef) -> SchemaRefAccess {
        match &value.schema.kind {
            SchemaKind::Struct(_) => SchemaRefAccess::Struct(StructRefAccess(value)),
            SchemaKind::Vec(_) => SchemaRefAccess::Vec(SchemaVecAccess {
                vec: value.as_vec().unwrap(),
                orig_ref: value,
            }),
            SchemaKind::Enum(_) => SchemaRefAccess::Enum(EnumRefAccess(value)),
            SchemaKind::Map { .. } => SchemaRefAccess::Map(SchemaMapAccess {
                map: value.as_map().unwrap(),
                orig_ref: value,
            }),
            SchemaKind::Box(_) => value.as_box().unwrap().access(),
            SchemaKind::Primitive(_) => SchemaRefAccess::Primitive(value.into()),
        }
    }

    /// Create a new [`SchemaRefAccess`] for the given [`SchemaRef`] that borrows the original
    /// [`SchemaRef`].
    ///
    /// This is subtly different from [`SchemaRefAccess::new()`] because it requires that it hold
    /// a borrow to the original schema ref it was created from. This is specifically useful becuse
    /// it lets you create a [`SchemaRefAccess`] from a refeence to a schema ref, which is required
    /// when accessing a schema ref that is behind an atomic resource borrow, for example.
    pub fn new_borrowed<'borrow>(value: &'borrow SchemaRef<'_>) -> SchemaRefAccess<'borrow> {
        match &value.schema.kind {
            SchemaKind::Struct(_) => SchemaRefAccess::Struct(StructRefAccess(*value)),
            SchemaKind::Vec(_) => SchemaRefAccess::Vec(SchemaVecAccess {
                vec: value.as_vec().unwrap(),
                orig_ref: *value,
            }),
            SchemaKind::Enum(_) => SchemaRefAccess::Enum(EnumRefAccess(*value)),
            SchemaKind::Map { .. } => SchemaRefAccess::Map(SchemaMapAccess {
                map: value.as_map().unwrap(),
                orig_ref: *value,
            }),
            SchemaKind::Box(_) => value.as_box().unwrap().access(),
            SchemaKind::Primitive(_) => SchemaRefAccess::Primitive((*value).into()),
        }
    }

    /// Get field with the given index.
    pub fn field<'a, I: Into<FieldIdx<'a>>>(self, field_idx: I) -> Option<Self> {
        let field_idx = field_idx.into();
        match self {
            SchemaRefAccess::Struct(s) => s.field(field_idx),
            SchemaRefAccess::Vec(_)
            | SchemaRefAccess::Enum(_)
            | SchemaRefAccess::Map(_)
            | SchemaRefAccess::Primitive(_) => None,
        }
    }

    /// Get the field pointed to by the given path.
    pub fn field_path<'a, I: IntoIterator<Item = FieldIdx<'a>>>(self, path: I) -> Option<Self> {
        let mut current_field = self;
        for field_idx in path {
            current_field = current_field.field(field_idx)?;
        }
        Some(current_field)
    }

    /// Borrow this [`SchemaRefMutAccess`] as a [`SchemaRefAccess`].
    pub fn into_schema_ref(self) -> SchemaRef<'ptr> {
        match self {
            SchemaRefAccess::Struct(s) => s.0,
            SchemaRefAccess::Vec(v) => v.into_schema_ref(),
            SchemaRefAccess::Enum(e) => e.0,
            SchemaRefAccess::Map(m) => m.into_schema_ref(),
            SchemaRefAccess::Primitive(p) => p.into_schema_ref(),
        }
    }
}

/// Access helper for a [`SchemaVec`].
#[derive(Deref, DerefMut, Clone, Copy)]
pub struct SchemaVecAccess<'a> {
    /// The schema vec borrow.
    #[deref]
    vec: &'a SchemaVec,
    orig_ref: SchemaRef<'a>,
}

impl<'a> SchemaVecAccess<'a> {
    /// Convert back to a [`SchemaRefMut`]
    pub fn into_schema_ref(self) -> SchemaRef<'a> {
        self.orig_ref
    }
}

/// Access helper for a [`SchemaMap`].
#[derive(Deref, DerefMut, Clone, Copy)]
pub struct SchemaMapAccess<'a> {
    /// The schema map borrow.
    #[deref]
    map: &'a SchemaMap,
    orig_ref: SchemaRef<'a>,
}

impl<'a> SchemaMapAccess<'a> {
    /// Convert back to a [`SchemaRefMut`]
    pub fn into_schema_ref(self) -> SchemaRef<'a> {
        self.orig_ref
    }
}

/// Helper for accessing the inner data of a schema ref at runtime.
#[derive(Clone, Copy)]
pub struct StructRefAccess<'a>(SchemaRef<'a>);

impl<'a> StructRefAccess<'a> {
    /// Get the struct's schema.
    pub fn schema(&self) -> &'static Schema {
        self.0.schema
    }

    /// Get the [`StructSchemaInfo`] for this struct.
    pub fn info(&self) -> &'static StructSchemaInfo {
        self.0.schema.kind.as_struct().unwrap()
    }

    /// Interate over the fields on the struct.
    pub fn fields(&self) -> StructRefFieldIter {
        StructRefFieldIter {
            ptr: self.0,
            field_idx: 0,
        }
    }

    /// Access a field, if it exists.
    pub fn field<'i, I: Into<FieldIdx<'i>>>(self, field_idx: I) -> Option<SchemaRefAccess<'a>> {
        let field_idx = field_idx.into();
        let field_idx = match field_idx {
            FieldIdx::Name(name) => self
                .info()
                .fields
                .iter()
                .position(|x| x.name.as_ref().map(|x| x.as_ref()) == Some(name))?,
            FieldIdx::Idx(idx) => idx,
        };
        let field_schema = self
            .0
            .schema
            .kind
            .as_struct()
            .unwrap()
            .fields
            .get(field_idx)
            .unwrap()
            .schema;
        let (_, field_offset) = self.0.schema.field_offsets().get(field_idx).unwrap();

        Some(unsafe {
            SchemaRef {
                ptr: NonNull::new_unchecked(self.0.as_ptr().add(*field_offset) as *mut c_void),
                schema: field_schema,
                _phantom: PhantomData,
            }
            .access()
        })
    }

    /// Convert to a [`SchemaRef`].
    pub fn as_schema_ref(&self) -> SchemaRef<'a> {
        self.0
    }
}

/// Iterator for [`StructRefAccess::fields()`].
pub struct StructRefFieldIter<'a> {
    ptr: SchemaRef<'a>,
    field_idx: usize,
}

/// A field returned by [`StructRefFieldIter`].
pub struct StructRefFieldIterField<'a> {
    /// The name of the field, if set.
    pub name: Option<&'static str>,
    /// The field's value.
    pub value: SchemaRef<'a>,
}

impl<'a> Iterator for StructRefFieldIter<'a> {
    type Item = StructRefFieldIterField<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let (name, _) = self.ptr.schema.field_offsets().get(self.field_idx)?;
        let ptr = self
            .ptr
            .access()
            .field(self.field_idx)
            .unwrap()
            .into_schema_ref();
        self.field_idx += 1;
        Some(StructRefFieldIterField {
            name: name.as_ref().map(|x| x.as_str()),
            value: ptr,
        })
    }
}

/// Helper for accessing the inner data of a schema ref at runtime.
#[derive(Clone, Copy)]
pub struct EnumRefAccess<'a>(pub SchemaRef<'a>);

impl<'a> EnumRefAccess<'a> {
    /// Get the enum's schema.
    pub fn schema(&self) -> &'static Schema {
        self.0.schema
    }

    /// Get the enum schema info.
    pub fn info(&self) -> &'static EnumSchemaInfo {
        let SchemaKind::Enum(info) = &self.0.schema.kind else {
            panic!("Not an enum");
        };
        info
    }

    /// Get the [`VariantInfo`] for the current variant.
    pub fn variant_info(&self) -> &'static VariantInfo {
        &self.info().variants[self.variant_idx() as usize]
    }

    /// Get the [`StructSchemaInfo`] for the current variant.
    pub fn variant_struct_info(&self) -> &'static StructSchemaInfo {
        self.variant_info().schema.kind.as_struct().unwrap()
    }

    /// Get the currently-selected variant index.
    pub fn variant_idx(&self) -> u32 {
        let info = self.info();
        match info.tag_type {
            EnumTagType::U8 => unsafe { self.0.as_ptr().cast::<u8>().read() as u32 },
            EnumTagType::U16 => unsafe { self.0.as_ptr().cast::<u16>().read() as u32 },
            EnumTagType::U32 => unsafe { self.0.as_ptr().cast::<u32>().read() },
        }
    }

    /// Get the name of the currently selected variant.
    pub fn variant_name(&self) -> &'static str {
        let info = self.info();
        let idx = self.variant_idx();
        info.variants[idx as usize].name.as_ref()
    }

    /// Get a reference to the enum's currently selected value.
    pub fn value(&self) -> StructRefAccess<'a> {
        let info = self.info();
        let variant_idx = self.variant_idx();
        let variant_info = &info.variants[variant_idx as usize];
        let schema = variant_info.schema;
        let value_offset = self.0.schema.field_offsets()[0].1;
        StructRefAccess(SchemaRef {
            ptr: unsafe { NonNull::new_unchecked(self.0.ptr.as_ptr().add(value_offset)) },
            schema,
            _phantom: PhantomData,
        })
    }
}

/// Helper for accessing the inner data of a schema ref at runtime.
#[derive(Clone, Copy, Debug)]
pub enum PrimitiveRef<'a> {
    /// A [`bool`]
    Bool(&'a bool),
    /// A [`u8`]
    U8(&'a u8),
    /// A [`u16`]
    U16(&'a u16),
    /// A [`u32`]
    U32(&'a u32),
    /// A [`u64`]
    U64(&'a u64),
    /// A [`u128`]
    U128(&'a u128),
    /// An [`i8`]
    I8(&'a i8),
    /// An [`i16`]
    I16(&'a i16),
    /// An [`i32`]
    I32(&'a i32),
    /// An [`i64`]
    I64(&'a i64),
    /// An [`i128`]
    I128(&'a i128),
    /// An [`f32`]
    F32(&'a f32),
    /// An [`f64`]
    F64(&'a f64),
    /// A [`String`]
    String(&'a String),
    /// An opaque type
    Opaque {
        /// The size of the opaque type.
        size: usize,
        /// The align of the opaque type.
        align: usize,
        /// The schema ref.
        schema_ref: SchemaRef<'a>,
    },
}

impl<'ptr> PrimitiveRef<'ptr> {
    fn into_schema_ref(self) -> SchemaRef<'ptr> {
        match self {
            PrimitiveRef::Bool(b) => SchemaRef::new(b),
            PrimitiveRef::U8(n) => SchemaRef::new(n),
            PrimitiveRef::U16(n) => SchemaRef::new(n),
            PrimitiveRef::U32(n) => SchemaRef::new(n),
            PrimitiveRef::U64(n) => SchemaRef::new(n),
            PrimitiveRef::U128(n) => SchemaRef::new(n),
            PrimitiveRef::I8(n) => SchemaRef::new(n),
            PrimitiveRef::I16(n) => SchemaRef::new(n),
            PrimitiveRef::I32(n) => SchemaRef::new(n),
            PrimitiveRef::I64(n) => SchemaRef::new(n),
            PrimitiveRef::I128(n) => SchemaRef::new(n),
            PrimitiveRef::F32(n) => SchemaRef::new(n),
            PrimitiveRef::F64(n) => SchemaRef::new(n),
            PrimitiveRef::String(s) => SchemaRef::new(s),
            PrimitiveRef::Opaque { schema_ref, .. } => schema_ref,
        }
    }
}

impl<'a> From<SchemaRef<'a>> for PrimitiveRef<'a> {
    fn from(value: SchemaRef<'a>) -> Self {
        match &value.schema.kind {
            SchemaKind::Primitive(p) => match p {
                Primitive::Bool => PrimitiveRef::Bool(value.cast()),
                Primitive::U8 => PrimitiveRef::U8(value.cast()),
                Primitive::U16 => PrimitiveRef::U16(value.cast()),
                Primitive::U32 => PrimitiveRef::U32(value.cast()),
                Primitive::U64 => PrimitiveRef::U64(value.cast()),
                Primitive::U128 => PrimitiveRef::U128(value.cast()),
                Primitive::I8 => PrimitiveRef::I8(value.cast()),
                Primitive::I16 => PrimitiveRef::I16(value.cast()),
                Primitive::I32 => PrimitiveRef::I32(value.cast()),
                Primitive::I64 => PrimitiveRef::I64(value.cast()),
                Primitive::I128 => PrimitiveRef::I128(value.cast()),
                Primitive::F32 => PrimitiveRef::F32(value.cast()),
                Primitive::F64 => PrimitiveRef::F64(value.cast()),
                Primitive::String => PrimitiveRef::String(value.cast()),
                Primitive::Opaque { size, align } => PrimitiveRef::Opaque {
                    size: *size,
                    align: *align,
                    schema_ref: value,
                },
            },
            _ => panic!("Schema mismatch"),
        }
    }
}

/// An untyped mutable reference that knows the [`Schema`] of the pointee and that can be cast to a matching
/// type.
pub struct SchemaRefMut<'pointer> {
    ptr: NonNull<c_void>,
    schema: &'static Schema,
    _phantom: PhantomData<&'pointer mut ()>,
}

impl<'pointer> std::fmt::Debug for SchemaRefMut<'pointer> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SchemaRefMut")
            // .field("ptr", &self.ptr)
            .field("schema", &self.schema)
            // .field("parent_lifetime", &self.parent_lifetime)
            .finish()
    }
}

impl<'pointer> SchemaRefMut<'pointer> {
    /// Cast this pointer to a mutable reference.
    /// # Safety
    /// You must uphold all safety requirements of [`NonNull::as_mut()`].
    pub unsafe fn cast_mut_unchecked<T>(&mut self) -> &mut T {
        self.ptr.cast::<T>().as_mut()
    }

    /// Cast this pointer to a mutable reference.
    /// # Safety
    /// You must uphold all safety requirements of [`NonNull::as_mut()`].
    pub unsafe fn cast_into_mut_unchecked<T>(self) -> &'pointer mut T {
        self.ptr.cast::<T>().as_mut()
    }

    /// Cast this pointer to a reference to a type with a matching [`Schema`].
    ///
    /// # Panics
    ///
    /// Panics if the schema of the pointer does not match that of the type you are casting to.
    #[track_caller]
    pub fn cast_mut<T: HasSchema + 'static>(&mut self) -> &mut T {
        self.try_cast_mut().expect(SchemaMismatchError::MSG)
    }

    /// Cast this pointer to a mutable reference to a type with a matching [`Schema`].
    ///
    /// # Errors
    /// Errors if the schema of the pointer does not match that of the type you are casting to.
    pub fn try_cast_mut<T: HasSchema>(&mut self) -> Result<&mut T, SchemaMismatchError> {
        if self.schema.represents(T::schema()) {
            // SOUND: this pointer has the same memory representation as T.
            unsafe { Ok(self.ptr.cast::<T>().as_mut()) }
        } else {
            Err(SchemaMismatchError)
        }
    }

    /// Cast this pointer to a mutable reference to a type with a matching [`Schema`]. This is
    /// different than `try_cast` because it consumes the [`SchemaRefMut`] and allows you to, for
    /// instance, pass it out of a mapping operation without keeping a reference to the old
    /// [`SchemaRefMut`].
    ///
    /// # Panics
    /// Panics if the schema of the pointer does not match that of the type you are casting to.
    #[inline]
    pub fn cast_into_mut<T: HasSchema>(self) -> &'pointer mut T {
        self.try_cast_into_mut().unwrap()
    }

    /// Cast this pointer to a mutable reference to a type with a matching [`Schema`]. This is
    /// different than `try_cast` because it consumes the [`SchemaRefMut`] and allows you to, for
    /// instance, pass it out of a mapping operation without keeping a reference to the old
    /// [`SchemaRefMut`].
    ///
    /// # Errors
    /// Errors if the schema of the pointer does not match that of the type you are casting to.
    pub fn try_cast_into_mut<T: HasSchema>(self) -> Result<&'pointer mut T, SchemaMismatchError> {
        if self.schema.represents(T::schema()) {
            // SOUND: We've checked that the pointer represents T
            Ok(unsafe { self.ptr.cast::<T>().as_mut() })
        } else {
            Err(SchemaMismatchError)
        }
    }

    /// Create a new [`SchemaRefMut`] from a reference to a type that implements [`HasSchema`].
    pub fn new<T: HasSchema>(v: &'pointer mut T) -> SchemaRefMut<'pointer> {
        let schema = T::schema();
        SchemaRefMut {
            // SOUND: the &mut T reference cannot be null.
            ptr: unsafe { NonNull::new_unchecked(v as *mut T as *mut c_void) },
            schema,
            _phantom: PhantomData,
        }
    }

    /// Create a new [`SchemaRefMut`] from a raw pointer and it's schema.
    ///
    /// # Safety
    /// - The pointee of `ptr` must be accurately described by the given `schema`.
    /// - `inner` must have correct provenance to allow reads and writes of the pointee type.
    /// - The pointer must be valid for the full lifetime of this [`SchemaRef`].
    pub unsafe fn from_ptr_schema(
        ptr: *mut c_void,
        schema: &'static Schema,
    ) -> SchemaRefMut<'pointer> {
        Self {
            ptr: NonNull::new_unchecked(ptr),
            schema,
            _phantom: PhantomData,
        }
    }

    /// Borrow the schema ref as a [`SchemaMap`] if it is one.
    pub fn into_map(self) -> Result<&'pointer mut SchemaMap, Self> {
        matches!(self.schema.kind, SchemaKind::Map { .. })
            // SOUND: Schema asserts this is a schema map
            .then(|| unsafe { &mut *(self.ptr.as_ptr() as *mut SchemaMap) })
            .ok_or(self)
    }

    /// Borrow the schema ref as a [`SchemaVec`] if it is one.
    pub fn into_vec(self) -> Result<&'pointer mut SchemaVec, Self> {
        matches!(self.schema.kind, SchemaKind::Vec(_))
            // SOUND: Schema asserts this is a schema map
            .then(|| unsafe { &mut *(self.ptr.as_ptr() as *mut SchemaVec) })
            .ok_or(self)
    }

    /// Borrow the schema ref as a [`SchemaBox`] if it is one.
    pub fn into_box(self) -> Result<SchemaRefMut<'pointer>, Self> {
        matches!(self.schema.kind, SchemaKind::Vec(_))
            // SOUND: Schema asserts this is a schema box
            .then(|| unsafe { (*(self.ptr.as_ptr() as *mut SchemaBox)).as_mut() })
            .ok_or(self)
    }

    /// Convert into an accessor for the inner data.
    pub fn into_access_mut(self) -> SchemaRefMutAccess<'pointer> {
        SchemaRefMutAccess::new(self)
    }

    /// Get a mutable access helper to this reference.
    pub fn access_mut(&mut self) -> SchemaRefMutAccess<'_> {
        SchemaRefMutAccess::new_borrowed(self)
    }

    /// Get the raw pointer
    pub fn as_ptr(&self) -> *mut c_void {
        self.ptr.as_ptr()
    }

    /// Get the [`Schema`] for the pointer.
    pub fn schema(&self) -> &'static Schema {
        self.schema
    }

    /// Get the hash of this schema box, if supported.
    pub fn hash(&self) -> Option<u64> {
        self.schema
            .hash_fn
            .as_ref()
            .map(|hash_fn| unsafe { (hash_fn.get())(self.ptr.as_ptr()) })
    }

    /// Borrow this [`SchemaRefMut`] as a [`SchemaRef`].
    pub fn as_ref(&self) -> SchemaRef<'_> {
        SchemaRef {
            ptr: self.ptr,
            schema: self.schema,
            _phantom: PhantomData,
        }
    }

    /// Convert a borrowed [`SchemaRefMut`] to an owned [`SchemaRefMut`] with a lifetime matching
    /// That of the borrow.
    pub fn reborrow(&mut self) -> SchemaRefMut<'_> {
        SchemaRefMut {
            ptr: self.ptr,
            schema: self.schema,
            _phantom: PhantomData,
        }
    }

    /// Get the reference to a field.
    pub fn field<'a, I: Into<FieldIdx<'a>>>(&mut self, field_idx: I) -> Option<SchemaRefMut> {
        Some(
            self.access_mut()
                .field(field_idx)
                .ok()?
                .into_schema_ref_mut(),
        )
    }

    /// Get the field pointed to by the given path.
    pub fn field_path<'a, I: IntoIterator<Item = FieldIdx<'a>>>(
        &mut self,
        path: I,
    ) -> Option<SchemaRefMut> {
        self.access_mut()
            .field_path(path)
            .map(|x| x.into_schema_ref_mut())
    }

    /// Get the field pointed to by the given path.
    pub fn into_field_path<'a, I: IntoIterator<Item = FieldIdx<'a>>>(
        self,
        path: I,
    ) -> Option<Self> {
        self.into_access_mut()
            .field_path(path)
            .map(|x| x.into_schema_ref_mut())
    }

    /// Get the reference to a field.
    pub fn into_field<'a, I: Into<FieldIdx<'a>>>(
        self,
        field_idx: I,
    ) -> Result<SchemaRefMut<'pointer>, Self> {
        self.into_access_mut()
            .field(field_idx)
            .map(|x| x.into_schema_ref_mut())
            .map_err(|access| access.into_schema_ref_mut())
    }

    /// Clone `other` and write it's data to `self`. Panics if this schema doesn't support cloning.
    pub fn write(&mut self, other: SchemaRef) -> Result<(), SchemaMismatchError> {
        if self.schema == other.schema {
            let clone_fn = self.schema.clone_fn.as_ref().unwrap_or_else(|| {
                panic!(
                    "Schema does not provide clone fn: {}",
                    self.schema.full_name
                )
            });
            // SOUND: we've verified the clone fn matches the schema of both values.
            unsafe { clone_fn.get()(other.as_ptr(), self.as_ptr()) }
            Ok(())
        } else {
            Err(SchemaMismatchError)
        }
    }
}

/// Access a schema
pub enum SchemaRefMutAccess<'a> {
    /// Access a struct.
    Struct(StructRefMutAccess<'a>),
    /// Access a vec.
    Vec(SchemaVecMutAccess<'a>),
    /// Access an enum.
    Enum(EnumRefMutAccess<'a>),
    /// Access a map.
    Map(SchemaMapMutAccess<'a>),
    /// Access a struct.
    Primitive(PrimitiveRefMut<'a>),
}

/// Mutable [`SchemaVec`] access helper.
#[derive(Deref, DerefMut)]
pub struct SchemaVecMutAccess<'a> {
    /// The schema vec borrow.
    #[deref]
    vec: &'a mut SchemaVec,
    /// The original pointer and schema to allow us to convert back to a [`SchemaRefMut`]
    /// WARNING: This pointer aliases with the `vec: &'a SchemaVec` reference and mut not be used
    /// until the borrow to the schema vec is dropped.
    orig_ptr: *mut c_void,
    orig_schema: &'static Schema,
}

impl<'a> SchemaVecMutAccess<'a> {
    /// Convert back to a [`SchemaRefMut`]
    pub fn as_mut(self) -> SchemaRefMut<'a> {
        // SOUND: we are taking ownership of self and dropping the reference that aliases,
        // so that we can return a valid [`SchemaRefMut`].
        unsafe { SchemaRefMut::from_ptr_schema(self.orig_ptr, self.orig_schema) }
    }
}

/// Mutable [`SchemaMap`] access helper.
#[derive(Deref, DerefMut)]
pub struct SchemaMapMutAccess<'a> {
    /// The schema map borrow.
    #[deref]
    map: &'a mut SchemaMap,
    /// The original pointer and schema to allow us to convert back to a [`SchemaRefMut`]
    /// WARNING: This pointer aliases with the `vec: &'a SchemaVec` reference and mut not be used
    /// until the borrow to the schema vec is dropped.
    orig_ptr: *mut c_void,
    orig_schema: &'static Schema,
}

impl<'a> SchemaMapMutAccess<'a> {
    /// Convert back to a [`SchemaRefMut`]
    pub fn into_schema_ref_mut(self) -> SchemaRefMut<'a> {
        // SOUND: we are taking ownership of self and dropping the reference that aliases,
        // so that we can return a valid [`SchemaRefMut`].
        unsafe { SchemaRefMut::from_ptr_schema(self.orig_ptr, self.orig_schema) }
    }
}

impl std::fmt::Debug for SchemaRefMutAccess<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_ref().fmt(f)
    }
}

impl<'pointer> SchemaRefMutAccess<'pointer> {
    /// Create a new [`SchemaRefAccess`] for the given [`SchemaRef`].
    ///
    /// This will create a new independent [`SchemaRefAccess`] that may be used even after
    /// the original [`SchemaRef`] is dropped ( but not beyond the safe lifetime of the
    /// original schema ref ).
    pub fn new(value: SchemaRefMut) -> SchemaRefMutAccess {
        match &value.schema.kind {
            SchemaKind::Struct(_) => SchemaRefMutAccess::Struct(StructRefMutAccess(value)),
            SchemaKind::Vec(_) => SchemaRefMutAccess::Vec(SchemaVecMutAccess {
                orig_ptr: value.as_ptr(),
                orig_schema: value.schema,
                vec: value.into_vec().unwrap(),
            }),
            SchemaKind::Enum(_) => SchemaRefMutAccess::Enum(EnumRefMutAccess(value)),
            SchemaKind::Map { .. } => SchemaRefMutAccess::Map(SchemaMapMutAccess {
                orig_ptr: value.as_ptr(),
                orig_schema: value.schema,
                map: value.into_map().unwrap(),
            }),
            SchemaKind::Box(_) => value.into_box().unwrap().into_access_mut(),
            SchemaKind::Primitive(_) => SchemaRefMutAccess::Primitive(value.into()),
        }
    }

    /// Create a new [`SchemaRefAccess`] for the given [`SchemaRef`] that borrows the original
    /// [`SchemaRef`].
    ///
    /// This is subtly different from [`SchemaRefAccess::new()`] because it requires that it hold
    /// a borrow to the original schema ref it was created from. This is specifically useful becuse
    /// it lets you create a [`SchemaRefAccess`] from a refeence to a schema ref, which is required
    /// when accessing a schema ref that is behind an atomic resource borrow, for example.
    pub fn new_borrowed<'borrow>(
        value: &'borrow mut SchemaRefMut<'_>,
    ) -> SchemaRefMutAccess<'borrow> {
        match &value.schema.kind {
            SchemaKind::Struct(_) => {
                SchemaRefMutAccess::Struct(StructRefMutAccess(value.reborrow()))
            }
            SchemaKind::Vec(_) => SchemaRefMutAccess::Vec(SchemaVecMutAccess {
                orig_ptr: value.as_ptr(),
                orig_schema: value.schema,
                vec: value.reborrow().into_vec().unwrap(),
            }),
            SchemaKind::Enum(_) => SchemaRefMutAccess::Enum(EnumRefMutAccess(value.reborrow())),
            SchemaKind::Map { .. } => SchemaRefMutAccess::Map(SchemaMapMutAccess {
                orig_ptr: value.as_ptr(),
                orig_schema: value.schema,
                map: value.reborrow().into_map().unwrap(),
            }),
            SchemaKind::Box(_) => value.reborrow().into_box().unwrap().into_access_mut(),
            SchemaKind::Primitive(_) => SchemaRefMutAccess::Primitive(value.reborrow().into()),
        }
    }

    /// Convert this to a [`SchemaRefMut`].
    pub fn into_schema_ref_mut(self) -> SchemaRefMut<'pointer> {
        match self {
            SchemaRefMutAccess::Struct(s) => s.0,
            SchemaRefMutAccess::Vec(v) => v.as_mut(),
            SchemaRefMutAccess::Enum(e) => e.0,
            SchemaRefMutAccess::Map(m) => m.into_schema_ref_mut(),
            SchemaRefMutAccess::Primitive(p) => p.into_schema_ref_mut(),
        }
    }

    /// Get field with the given index.
    pub fn field<'a, I: Into<FieldIdx<'a>>>(self, field_idx: I) -> Result<Self, Self> {
        let field_idx = field_idx.into();
        match self {
            SchemaRefMutAccess::Struct(s) => {
                s.into_field(field_idx).map_err(SchemaRefMutAccess::Struct)
            }
            other @ (SchemaRefMutAccess::Vec(_)
            | SchemaRefMutAccess::Enum(_)
            | SchemaRefMutAccess::Map(_)
            | SchemaRefMutAccess::Primitive(_)) => Err(other),
        }
    }

    /// Get the field pointed to by the given path.
    pub fn field_path<'a, I: IntoIterator<Item = FieldIdx<'a>>>(self, path: I) -> Option<Self> {
        let mut current_field = self;
        for field_idx in path {
            current_field = current_field.field(field_idx).ok()?;
        }
        Some(current_field)
    }

    /// Borrow this [`SchemaRefMutAccess`] as a [`SchemaRefAccess`].
    pub fn as_ref(&self) -> SchemaRefAccess {
        match self {
            SchemaRefMutAccess::Struct(s) => SchemaRefAccess::Struct(StructRefAccess(s.0.as_ref())),
            SchemaRefMutAccess::Vec(v) => SchemaRefAccess::Vec(SchemaVecAccess {
                vec: &*v.vec,
                // SOUND: We hold an exclusive borrow which we are allowed to downgrade to a read-only reference.
                orig_ref: unsafe {
                    SchemaRef::from_ptr_schema(
                        (&*v.vec) as *const SchemaVec as *const c_void,
                        v.orig_schema,
                    )
                },
            }),
            SchemaRefMutAccess::Enum(e) => SchemaRefAccess::Enum(EnumRefAccess(e.0.as_ref())),
            SchemaRefMutAccess::Map(m) => SchemaRefAccess::Map(SchemaMapAccess {
                map: &*m.map,
                // SOUND: We hold an exclusive borrow which we are allowed to downgrade to a read-only reference.
                orig_ref: unsafe {
                    SchemaRef::from_ptr_schema(
                        (&*m.map) as *const SchemaMap as *const c_void,
                        m.orig_schema,
                    )
                },
            }),
            SchemaRefMutAccess::Primitive(p) => SchemaRefAccess::Primitive(p.as_ref()),
        }
    }
}

/// Helper for accessing the inner data of a schema ref at runtime.
pub struct StructRefMutAccess<'a>(pub SchemaRefMut<'a>);

impl<'a> StructRefMutAccess<'a> {
    /// Get the struct's schema.
    pub fn schema(&self) -> &'static Schema {
        self.0.schema
    }

    /// Get the [`StructSchemaInfo`] for this struct.
    pub fn info(&self) -> &'static StructSchemaInfo {
        self.0.schema.kind.as_struct().unwrap()
    }

    /// Access a field, if it exists.
    pub fn into_field<'i, I: Into<FieldIdx<'i>>>(
        self,
        field_idx: I,
    ) -> Result<SchemaRefMutAccess<'a>, Self> {
        let field_idx = field_idx.into();
        let field_idx = match field_idx {
            FieldIdx::Name(name) => {
                if let Some(idx) = self
                    .info()
                    .fields
                    .iter()
                    .position(|x| x.name.as_ref().map(|x| x.as_ref()) == Some(name))
                {
                    idx
                } else {
                    return Err(self);
                }
            }
            FieldIdx::Idx(idx) => idx,
        };
        let field_schema = self
            .0
            .schema
            .kind
            .as_struct()
            .unwrap()
            .fields
            .get(field_idx)
            .unwrap()
            .schema;
        let (_, field_offset) = self.0.schema.field_offsets().get(field_idx).unwrap();

        Ok(unsafe {
            SchemaRefMut {
                ptr: NonNull::new_unchecked(self.0.as_ptr().add(*field_offset)),
                schema: field_schema,
                _phantom: PhantomData,
            }
            .into_access_mut()
        })
    }

    /// Iterate over fields in the struct.
    pub fn fields(&mut self) -> StructRefMutFieldIter<'_> {
        StructRefMutFieldIter {
            ptr: self.0.reborrow(),
            field_idx: 0,
        }
    }

    /// Consume to create an iterator over fields in the struct.
    pub fn into_fields(self) -> StructRefMutFieldIter<'a> {
        StructRefMutFieldIter {
            ptr: self.0,
            field_idx: 0,
        }
    }
}

/// Iterator for [`StructRefAccess::fields()`].
pub struct StructRefMutFieldIter<'a> {
    ptr: SchemaRefMut<'a>,
    field_idx: usize,
}

/// A field returned by [`StructRefFieldIter`].
pub struct StructRefMutFieldIterField<'a> {
    /// The name of the field, if set.
    pub name: Option<&'static str>,
    /// The field's value.
    pub value: SchemaRefMut<'a>,
}

impl<'a> Iterator for StructRefMutFieldIter<'a> {
    type Item = StructRefMutFieldIterField<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let field_schema = self
            .ptr
            .schema
            .kind
            .as_struct()
            .unwrap()
            .fields
            .get(self.field_idx)?
            .schema;
        let (name, field_offset) = self.ptr.schema.field_offsets().get(self.field_idx)?;
        self.field_idx += 1;

        Some(StructRefMutFieldIterField {
            name: name.as_ref().map(|x| x.as_str()),
            // SOUND: Return a new SchemaRefMut with the 'a lifetime. This is sound because we
            // don't return mutliple `SchemaRefMut`s to the same data.
            value: unsafe {
                SchemaRefMut {
                    ptr: NonNull::new_unchecked(self.ptr.as_ptr().add(*field_offset)),
                    schema: field_schema,
                    _phantom: PhantomData,
                }
            },
        })
    }
}

/// Helper for accessing the inner data of a schema ref at runtime.
pub struct EnumRefMutAccess<'a>(pub SchemaRefMut<'a>);

impl<'a> EnumRefMutAccess<'a> {
    /// Get the enum's schema.
    pub fn schema(&self) -> &'static Schema {
        self.0.schema
    }

    /// Get the enum schema info.
    pub fn info(&self) -> &'static EnumSchemaInfo {
        let SchemaKind::Enum(info) = &self.0.schema.kind else {
            panic!("Not an enum");
        };
        info
    }

    /// Get the currently-selected variant index.
    pub fn variant_idx(&self) -> u32 {
        let info = self.info();
        match info.tag_type {
            EnumTagType::U8 => unsafe { self.0.as_ptr().cast::<u8>().read() as u32 },
            EnumTagType::U16 => unsafe { self.0.as_ptr().cast::<u16>().read() as u32 },
            EnumTagType::U32 => unsafe { self.0.as_ptr().cast::<u32>().read() },
        }
    }

    /// Get the name of the currently selected variant.
    pub fn variant_name(&self) -> &'static str {
        let info = self.info();
        let idx = self.variant_idx();
        info.variants[idx as usize].name.as_ref()
    }

    /// Get a reference to the enum's currently selected value.
    pub fn value(&self) -> StructRefMutAccess<'a> {
        let info = self.info();
        let variant_idx = self.variant_idx();
        let variant_info = &info.variants[variant_idx as usize];
        let schema = variant_info.schema;
        let value_offset = self.0.schema.field_offsets()[0].1;
        StructRefMutAccess(SchemaRefMut {
            ptr: unsafe { NonNull::new_unchecked(self.0.ptr.as_ptr().add(value_offset)) },
            schema,
            _phantom: PhantomData,
        })
    }
}

/// Helper for accessing the inner data of a schema ref at runtime.
pub enum PrimitiveRefMut<'a> {
    /// A [`bool`]
    Bool(&'a mut bool),
    /// A [`u8`]
    U8(&'a mut u8),
    /// A [`u16`]
    U16(&'a mut u16),
    /// A [`u32`]
    U32(&'a mut u32),
    /// A [`u64`]
    U64(&'a mut u64),
    /// A [`u128`]
    U128(&'a mut u128),
    /// An [`i8`]
    I8(&'a mut i8),
    /// An [`i16`]
    I16(&'a mut i16),
    /// An [`i32`]
    I32(&'a mut i32),
    /// An [`i64`]
    I64(&'a mut i64),
    /// An [`i128`]
    I128(&'a mut i128),
    /// An [`f32`]
    F32(&'a mut f32),
    /// An [`f64`]
    F64(&'a mut f64),
    /// A [`String`]
    String(&'a mut String),
    /// An opaque type
    Opaque {
        /// The size of the opaque type.
        size: usize,
        /// The align of the opaque type.
        align: usize,
        /// The schema ref.
        schema_ref: SchemaRefMut<'a>,
    },
}

impl<'ptr> PrimitiveRefMut<'ptr> {
    /// Convert to an immutable [`PrimitiveRef`].
    pub fn as_ref(&self) -> PrimitiveRef {
        match self {
            PrimitiveRefMut::Bool(b) => PrimitiveRef::Bool(b),
            PrimitiveRefMut::U8(n) => PrimitiveRef::U8(n),
            PrimitiveRefMut::U16(n) => PrimitiveRef::U16(n),
            PrimitiveRefMut::U32(n) => PrimitiveRef::U32(n),
            PrimitiveRefMut::U64(n) => PrimitiveRef::U64(n),
            PrimitiveRefMut::U128(n) => PrimitiveRef::U128(n),
            PrimitiveRefMut::I8(n) => PrimitiveRef::I8(n),
            PrimitiveRefMut::I16(n) => PrimitiveRef::I16(n),
            PrimitiveRefMut::I32(n) => PrimitiveRef::I32(n),
            PrimitiveRefMut::I64(n) => PrimitiveRef::I64(n),
            PrimitiveRefMut::I128(n) => PrimitiveRef::I128(n),
            PrimitiveRefMut::F32(n) => PrimitiveRef::F32(n),
            PrimitiveRefMut::F64(n) => PrimitiveRef::F64(n),
            PrimitiveRefMut::String(n) => PrimitiveRef::String(n),
            PrimitiveRefMut::Opaque {
                size,
                align,
                schema_ref,
            } => PrimitiveRef::Opaque {
                size: *size,
                align: *align,
                schema_ref: schema_ref.as_ref(),
            },
        }
    }

    fn into_schema_ref_mut(self) -> SchemaRefMut<'ptr> {
        match self {
            PrimitiveRefMut::Bool(b) => SchemaRefMut::new(b),
            PrimitiveRefMut::U8(n) => SchemaRefMut::new(n),
            PrimitiveRefMut::U16(n) => SchemaRefMut::new(n),
            PrimitiveRefMut::U32(n) => SchemaRefMut::new(n),
            PrimitiveRefMut::U64(n) => SchemaRefMut::new(n),
            PrimitiveRefMut::U128(n) => SchemaRefMut::new(n),
            PrimitiveRefMut::I8(n) => SchemaRefMut::new(n),
            PrimitiveRefMut::I16(n) => SchemaRefMut::new(n),
            PrimitiveRefMut::I32(n) => SchemaRefMut::new(n),
            PrimitiveRefMut::I64(n) => SchemaRefMut::new(n),
            PrimitiveRefMut::I128(n) => SchemaRefMut::new(n),
            PrimitiveRefMut::F32(n) => SchemaRefMut::new(n),
            PrimitiveRefMut::F64(n) => SchemaRefMut::new(n),
            PrimitiveRefMut::String(s) => SchemaRefMut::new(s),
            PrimitiveRefMut::Opaque { schema_ref, .. } => schema_ref,
        }
    }
}

impl<'a> From<SchemaRefMut<'a>> for PrimitiveRefMut<'a> {
    fn from(value: SchemaRefMut<'a>) -> Self {
        match &value.schema.kind {
            SchemaKind::Primitive(p) => match p {
                Primitive::Bool => PrimitiveRefMut::Bool(value.cast_into_mut()),
                Primitive::U8 => PrimitiveRefMut::U8(value.cast_into_mut()),
                Primitive::U16 => PrimitiveRefMut::U16(value.cast_into_mut()),
                Primitive::U32 => PrimitiveRefMut::U32(value.cast_into_mut()),
                Primitive::U64 => PrimitiveRefMut::U64(value.cast_into_mut()),
                Primitive::U128 => PrimitiveRefMut::U128(value.cast_into_mut()),
                Primitive::I8 => PrimitiveRefMut::I8(value.cast_into_mut()),
                Primitive::I16 => PrimitiveRefMut::I16(value.cast_into_mut()),
                Primitive::I32 => PrimitiveRefMut::I32(value.cast_into_mut()),
                Primitive::I64 => PrimitiveRefMut::I64(value.cast_into_mut()),
                Primitive::I128 => PrimitiveRefMut::I128(value.cast_into_mut()),
                Primitive::F32 => PrimitiveRefMut::F32(value.cast_into_mut()),
                Primitive::F64 => PrimitiveRefMut::F64(value.cast_into_mut()),
                Primitive::String => PrimitiveRefMut::String(value.cast_into_mut()),
                Primitive::Opaque { size, align } => PrimitiveRefMut::Opaque {
                    size: *size,
                    align: *align,
                    schema_ref: value,
                },
            },
            _ => panic!("Schema mismatch"),
        }
    }
}

/// A owning, type-erased [`Box`]-like container for types with a [`Schema`].
pub struct SchemaBox {
    ptr: NonNull<c_void>,
    schema: &'static Schema,
}
impl Default for SchemaBox {
    fn default() -> Self {
        SchemaBox::new(())
    }
}
unsafe impl Sync for SchemaBox {}
unsafe impl Send for SchemaBox {}
impl std::fmt::Debug for SchemaBox {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SchemaBox")
            .field("schema", &self.schema.full_name)
            .field("value", &SchemaRefValueDebug(self.as_ref()))
            .finish_non_exhaustive()
    }
}
impl std::fmt::Display for SchemaBox {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_ref().fmt(f)
    }
}

impl Hash for SchemaBox {
    #[track_caller]
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let Some(hash_fn) = &self.schema.hash_fn else {
            panic!("Cannot hash schema box where schema doesn't provide hash_fn");
        };
        let hash = unsafe { (hash_fn.get())(self.ptr.as_ptr()) };
        state.write_u64(hash);
    }
}

impl PartialEq for SchemaBox {
    fn eq(&self, other: &Self) -> bool {
        if self.schema != other.schema {
            panic!("Cannot compare two `SchemaBox`s with different schemas.");
        }
        let Some(eq_fn) = &self.schema.eq_fn else {
            panic!("Cannot hash schema box where schema doesn't provide hash_fn.");
        };
        unsafe { (eq_fn.get())(self.ptr.as_ptr(), other.ptr.as_ptr()) }
    }
}
impl Eq for SchemaBox {}

impl Clone for SchemaBox {
    fn clone(&self) -> Self {
        let clone_fn = self.schema.clone_fn.as_ref().unwrap_or_else(|| {
            panic!(
                "The schema for this type does not allow cloning it.\nSchema: {:#?}",
                self.schema
            )
        });

        let layout = self.schema.layout();
        let new_ptr = if layout.size() == 0 {
            NonNull::<c_void>::dangling().as_ptr()
        } else {
            // SOUND: Non-zero size for layout
            unsafe { std::alloc::alloc(layout) as *mut c_void }
        };
        let new_ptr = unsafe {
            (clone_fn.get())(self.ptr.as_ptr(), new_ptr);
            NonNull::new(new_ptr).unwrap_or_else(|| handle_alloc_error(layout))
        };
        Self {
            ptr: new_ptr,
            schema: self.schema,
        }
    }
}

impl SchemaBox {
    /// Get a raw pointer to the box data.
    pub fn as_ptr(&self) -> *mut c_void {
        self.ptr.as_ptr()
    }

    /// Cast this box to it's inner type and return it.
    /// # Panics
    /// Panics if the schema of the box does not match that of the type you are casting to.
    #[track_caller]
    pub fn cast_into<T: HasSchema>(self) -> T {
        self.try_cast_into().unwrap()
    }

    /// Cast this box to it's inner type and return it.
    /// # Errors
    /// Errors if the schema of the box does not match that of the type you are casting to.
    pub fn try_cast_into<T: HasSchema>(self) -> Result<T, SchemaMismatchError> {
        if self.schema == T::schema() {
            // We've validated that the schema of the box matches T
            Ok(unsafe { self.cast_into_unchecked() })
        } else {
            Err(SchemaMismatchError)
        }
    }

    /// Unsafely convert this box into an owned T.
    /// # Safety
    /// - The schema of type T must equal that of this box.
    pub unsafe fn cast_into_unchecked<T: HasSchema>(self) -> T {
        // Allocate memory for T on the stack
        let mut ret = MaybeUninit::<T>::uninit();

        // Copy the data from the box into the stack.
        // SOUND: We've validated that the box has the same schema as T
        unsafe {
            (ret.as_mut_ptr() as *mut c_void)
                .copy_from_nonoverlapping(self.ptr.as_ptr(), self.schema.layout().size());
        }

        // De-allocate the box without running the destructor for the inner data.
        self.forget();

        // SOUND: we initialized the type above
        unsafe { ret.assume_init() }
    }

    /// Cast this box to a reference to a type with a representative [`Schema`].
    /// # Panics
    /// Panics if the schema of the box does not match that of the type you are casting to.
    #[track_caller]
    pub fn cast_ref<T: HasSchema>(&self) -> &T {
        self.try_cast_ref().expect(SchemaMismatchError::MSG)
    }

    /// Cast this box to a reference to a type with a representative [`Schema`].
    /// # Errors
    /// Errors if the schema of the box does not match that of the type you are casting to.
    pub fn try_cast_ref<T: HasSchema>(&self) -> Result<&T, SchemaMismatchError> {
        if self.schema.represents(T::schema()) {
            // SOUND: the schemas have the same memory representation.
            unsafe { Ok(self.ptr.cast::<T>().as_ref()) }
        } else {
            Err(SchemaMismatchError)
        }
    }

    /// Cast this box to a mutable reference to a type with a representing [`Schema`].
    /// # Panics
    /// Panics if the schema of the box does not match that of the type you are casting to.
    #[track_caller]
    pub fn cast_mut<T: HasSchema>(&mut self) -> &mut T {
        self.try_cast_mut().expect(SchemaMismatchError::MSG)
    }

    /// Cast this box to a mutable reference to a type with a representing [`Schema`].
    /// # Errors
    /// Errors if the schema of the box does not match that of the type you are casting to.
    pub fn try_cast_mut<T: HasSchema>(&mut self) -> Result<&mut T, SchemaMismatchError> {
        if self.schema.represents(T::schema()) {
            // SOUND: the schemas have the same memory representation.
            unsafe { Ok(self.ptr.cast::<T>().as_mut()) }
        } else {
            Err(SchemaMismatchError)
        }
    }

    /// Borrow this box as a [`SchemaRef`].
    pub fn as_ref(&self) -> SchemaRef<'_> {
        SchemaRef {
            ptr: self.ptr,
            schema: self.schema,
            _phantom: PhantomData,
        }
    }

    /// Borrow this box as a [`SchemaRefMut`].
    pub fn as_mut(&mut self) -> SchemaRefMut<'_> {
        SchemaRefMut {
            ptr: self.ptr,
            schema: self.schema,
            _phantom: PhantomData,
        }
    }

    /// Create a new [`SchemaBox`] from an owned type.
    #[track_caller]
    pub fn new<T: HasSchema + Sync + Send>(v: T) -> Self {
        let schema = T::schema();
        // SOUND: we initialize the box immediately after creation.
        unsafe {
            let b = SchemaBox::uninitialized(schema);
            (b.ptr.as_ptr() as *mut T).write(v);
            b
        }
    }

    /// Allocates a [`SchemaBox`] for the given [`Schema`], but **doesn't initialize the memory**.
    ///
    /// # Safety
    ///
    /// Accessing the data in an unitinialized [`SchemaBox`] is undefined behavior. It is up to the
    /// user to initialize the memory pointed at by the box after creating it.
    pub unsafe fn uninitialized(schema: &'static Schema) -> Self {
        let layout = schema.layout();

        let ptr = if layout.size() == 0 {
            NonNull::<c_void>::dangling().as_ptr()
        } else {
            // SOUND: Non-zero size for layout
            std::alloc::alloc(layout) as *mut c_void
        };
        // SOUND: The pointer is allocated for the layout matching the schema.
        let ptr = NonNull::new(ptr).unwrap_or_else(|| handle_alloc_error(layout));

        Self { ptr, schema }
    }

    /// Create a new [`SchemaBox`] for a type with a [`Schema`] that has a
    /// [`SchemaData::default_fn`].
    ///
    /// # Panics
    ///
    /// Panics if the passed in schema doesn't have a `default_fn`.
    #[track_caller]
    pub fn default(schema: &'static Schema) -> Self {
        let Some(default_fn) = &schema.default_fn else {
            panic!("Schema doesn't have `default_fn` to create default value with.");
        };

        unsafe {
            let b = SchemaBox::uninitialized(schema);
            (default_fn.get())(b.ptr.as_ptr());
            b
        }
    }

    /// Convert into an [`SBox`] if the schema of T matches.
    /// # Panics
    /// Panics if the schema of `T` doesn't match that of the box.
    pub fn into_sbox<T: HasSchema>(self) -> SBox<T> {
        self.try_into_sbox()
            .unwrap_or_else(|_| panic!("{:?}", SchemaMismatchError))
    }

    /// Convert into an [`SBox`] if the schema of T matches.
    /// # Errors
    /// Returns an error with the orignal [`SchemaBox`] if the schema of `T` doesn't match.
    pub fn try_into_sbox<T: HasSchema>(self) -> Result<SBox<T>, Self> {
        if self.schema == T::schema() {
            Ok(SBox {
                b: self,
                _phantom: PhantomData,
            })
        } else {
            Err(self)
        }
    }

    /// Get the [`Schema`] for the pointer.
    pub fn schema(&self) -> &'static Schema {
        self.schema
    }

    /// Create a new [`SchemaBox`] from raw parts.
    ///
    /// This is useful for creating a [`SchemaBox`] for data with a schema loaded at runtime and
    /// without a Rust type.
    ///
    /// # Safety
    ///
    /// - You must insure that the pointer is valid for the given schema.
    pub unsafe fn from_raw_parts(ptr: NonNull<c_void>, schema: &'static Schema) -> Self {
        Self { ptr, schema }
    }

    /// Deallocate the memory stored in the box, but don't run the destructor.
    pub fn forget(mut self) {
        unsafe {
            self.dealloc();
        }
        std::mem::forget(self);
    }

    /// Get the hash of this schema box, if supported.
    pub fn try_hash(&self) -> Option<u64> {
        self.schema
            .hash_fn
            .as_ref()
            .map(|hash_fn| unsafe { (hash_fn.get())(self.ptr.as_ptr()) })
    }

    /// Get the hash of this schema box.
    /// # Panics
    /// Panics if the schema doesn't implement hash.
    #[track_caller]
    pub fn hash(&self) -> u64 {
        self.try_hash().expect("Schema doesn't implement hash")
    }

    /// Deallocate the memory in the box.
    unsafe fn dealloc(&mut self) {
        if self.schema.layout().size() > 0 {
            std::alloc::dealloc(self.ptr.as_ptr() as *mut u8, self.schema.layout())
        }
    }

    /// Drop the inner type, without dealocating the box's memory.
    unsafe fn drop_inner(&mut self) {
        if let Some(drop_fn) = &self.schema.drop_fn {
            // Drop the type
            (drop_fn.get())(self.ptr.as_ptr());
        }
    }
}

unsafe impl HasSchema for SchemaBox {
    fn schema() -> &'static Schema {
        use crate::raw_fns::*;
        use std::alloc::Layout;
        static S: OnceLock<&'static Schema> = OnceLock::new();
        let layout = Layout::new::<Self>();
        S.get_or_init(|| {
            SCHEMA_REGISTRY.register(SchemaData {
                name: type_name::<Self>().into(),
                full_name: format!("{}::{}", module_path!(), type_name::<Self>()).into(),
                kind: SchemaKind::Primitive(Primitive::Opaque {
                    size: layout.size(),
                    align: layout.align(),
                }),
                type_id: Some(TypeId::of::<Self>()),
                clone_fn: Some(<Self as RawClone>::raw_clone_cb()),
                drop_fn: Some(<Self as RawDrop>::raw_drop_cb()),
                default_fn: None,
                hash_fn: Some(<Self as RawHash>::raw_hash_cb()),
                eq_fn: Some(<Self as RawEq>::raw_eq_cb()),
                type_data: default(),
            })
        })
    }
}

impl Drop for SchemaBox {
    fn drop(&mut self) {
        unsafe {
            self.drop_inner();
            self.dealloc();
        }
    }
}

/// A typed version of [`SchemaBox`].
///
/// This allows to use [`SBox<T>`] extremely similar to a [`Box<T>`] except that it can be converted
/// to and from a [`SchemaBox`] for compatibility with the schema ecosystem.
///
/// Also, compared to a [`SchemaBox`], it is more efficient to access, because it avoids extra
/// runtime checks for a matching schema after it has been created, and doesn't need to be cast to
/// `T` upon every access.
#[repr(transparent)]
pub struct SBox<T: HasSchema> {
    b: SchemaBox,
    _phantom: PhantomData<T>,
}
impl<T: HasSchema> Default for SBox<T> {
    #[track_caller]
    fn default() -> Self {
        let schema = T::schema();
        let Some(default_fn) = &schema.default_fn else {
            panic!("Schema doesn't implement default");
        };
        Self {
            // SOUND: we initialize the schema box immediately, and the schema asserts the default
            // fn is valid for the type.
            b: unsafe {
                let mut b = SchemaBox::uninitialized(schema);
                (default_fn.get())(b.as_mut().as_ptr());
                b
            },
            _phantom: Default::default(),
        }
    }
}
impl<T: HasSchema> Clone for SBox<T> {
    fn clone(&self) -> Self {
        Self {
            b: self.b.clone(),
            _phantom: self._phantom,
        }
    }
}
impl<T: HasSchema + std::fmt::Debug> std::fmt::Debug for SBox<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SBox").field("b", &self.b).finish()
    }
}
unsafe impl<T: HasSchema> HasSchema for SBox<T> {
    fn schema() -> &'static Schema {
        static S: OnceLock<RwLock<HashMap<TypeId, &'static Schema>>> = OnceLock::new();
        let schema = {
            S.get_or_init(default)
                .read()
                .get(&TypeId::of::<Self>())
                .copied()
        };
        schema.unwrap_or_else(|| {
            let schema = SCHEMA_REGISTRY.register(SchemaData {
                name: type_name::<Self>().into(),
                full_name: format!("{}::{}", module_path!(), type_name::<Self>()).into(),
                kind: SchemaKind::Box(T::schema()),
                type_id: Some(TypeId::of::<Self>()),
                clone_fn: Some(<Self as RawClone>::raw_clone_cb()),
                drop_fn: Some(<Self as RawDrop>::raw_drop_cb()),
                default_fn: Some(<Self as RawDefault>::raw_default_cb()),
                hash_fn: Some(unsafe {
                    Unsafe::new(Box::leak(Box::new(|a| SchemaVec::raw_hash(a))))
                }),
                eq_fn: Some(unsafe {
                    Unsafe::new(Box::leak(Box::new(|a, b| SchemaVec::raw_eq(a, b))))
                }),
                type_data: Default::default(),
            });

            S.get_or_init(default)
                .write()
                .insert(TypeId::of::<Self>(), schema);

            schema
        })
    }
}

impl<T: HasSchema> SBox<T> {
    /// Create a new [`SBox`].
    pub fn new(value: T) -> Self {
        SBox {
            b: SchemaBox::new(value),
            _phantom: PhantomData,
        }
    }

    /// Convert into a [`SchemaBox`]
    pub fn into_schema_box(self) -> SchemaBox {
        self.b
    }
}

impl<T: HasSchema> From<SBox<T>> for SchemaBox {
    fn from(value: SBox<T>) -> Self {
        value.b
    }
}

impl<T: HasSchema> TryFrom<SchemaBox> for SBox<T> {
    type Error = SchemaBox;
    fn try_from(value: SchemaBox) -> Result<Self, Self::Error> {
        value.try_into_sbox()
    }
}

impl<T: HasSchema> std::ops::Deref for SBox<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // SOUND: `SBox`s always contain their type `T`.
        unsafe { self.b.ptr.cast::<T>().as_ref() }
    }
}
impl<T: HasSchema> std::ops::DerefMut for SBox<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // SOUND: `SBox`s always contain their type `T`.
        unsafe { self.b.ptr.cast::<T>().as_mut() }
    }
}

/// The index of a field in a struct in a [`Schema`].
#[derive(Debug, Clone, Copy)]
pub enum FieldIdx<'a> {
    /// The name of a field.
    Name(&'a str),
    /// The index of a field. Works for tuple fields and named fields.
    Idx(usize),
}
impl From<usize> for FieldIdx<'static> {
    fn from(value: usize) -> Self {
        Self::Idx(value)
    }
}
impl<'a> From<&'a str> for FieldIdx<'a> {
    fn from(value: &'a str) -> Self {
        Self::Name(value)
    }
}
impl<'a> From<&'a String> for FieldIdx<'a> {
    fn from(value: &'a String) -> Self {
        Self::Name(value)
    }
}
impl<'a> std::fmt::Display for FieldIdx<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Idx(i) => write!(f, "{i}"),
            Self::Name(n) => write!(f, "{n}"),
        }
    }
}

/// A wrapper type that implements [`IntoIterator<Item = FieldIdx>`] for an inner string to make
/// it easier to use with [`SchemaRefAccess::field_path()`] and other field path methods.
pub struct FieldPath<T>(pub T);
impl<'a> IntoIterator for FieldPath<&'a str> {
    type Item = FieldIdx<'a>;
    type IntoIter = Map<Filter<Split<'a, char>, fn(&&str) -> bool>, fn(&str) -> FieldIdx>;

    fn into_iter(self) -> Self::IntoIter {
        fn flt(x: &&str) -> bool {
            !x.is_empty()
        }
        fn mp(x: &str) -> FieldIdx {
            x.parse::<usize>()
                .map(FieldIdx::Idx)
                .unwrap_or(FieldIdx::Name(x))
        }
        self.0.split('.').filter(flt as _).map(mp as _)
    }
}
impl IntoIterator for FieldPath<Ustr> {
    type Item = FieldIdx<'static>;
    type IntoIter = Map<Filter<Split<'static, char>, fn(&&str) -> bool>, fn(&str) -> FieldIdx>;

    fn into_iter(self) -> Self::IntoIter {
        fn flt(x: &&str) -> bool {
            !x.is_empty()
        }
        fn mp(x: &str) -> FieldIdx {
            x.parse::<usize>()
                .map(FieldIdx::Idx)
                .unwrap_or(FieldIdx::Name(x))
        }
        self.0.as_str().split('.').filter(flt as _).map(mp as _)
    }
}

/// Error type when attempting to cast between types with mis-matched schemas.
#[derive(Debug)]
pub struct SchemaMismatchError;
impl SchemaMismatchError {
    /// The display error message for this error type.
    pub const MSG: &'static str =
        "Invalid cast: the schemas of the casted types are not compatible.";
}
impl std::error::Error for SchemaMismatchError {}
impl std::fmt::Display for SchemaMismatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", Self::MSG)
    }
}

/// Error returned when a field is not found in a schema.
#[derive(Debug)]
pub struct SchemaFieldNotFoundError<'a> {
    idx: FieldIdx<'a>,
}
impl<'a> std::error::Error for SchemaFieldNotFoundError<'a> {}
impl<'a> std::fmt::Display for SchemaFieldNotFoundError<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Field not found in schema: {}", self.idx)
    }
}
