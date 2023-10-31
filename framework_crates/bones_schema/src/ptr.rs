//! Schema-aware smart pointers.

use std::{
    alloc::handle_alloc_error,
    any::{type_name, TypeId},
    hash::Hash,
    iter::{Filter, Map},
    marker::PhantomData,
    mem::MaybeUninit,
    ptr::NonNull,
    str::Split,
    sync::OnceLock,
};

use crate::{
    prelude::*,
    raw_fns::{RawClone, RawDefault, RawDrop},
};
use bones_utils::{parking_lot::RwLock, prelude::*};

/// An untyped reference that knows the [`Schema`] of the pointee and that can be cast to a matching
/// type.
#[derive(Clone, Copy)]
pub struct SchemaRef<'pointer> {
    ptr: Ptr<'pointer>,
    schema: &'static Schema,
}

impl<'pointer> SchemaRef<'pointer> {
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
            Ok(unsafe { self.ptr.deref() })
        } else {
            Err(SchemaMismatchError)
        }
    }

    /// Create a new [`SchemaRef`] from a reference to a type that implements [`HasSchema`].
    pub fn new<T: HasSchema>(v: &'pointer T) -> SchemaRef<'pointer> {
        let schema = T::schema();
        SchemaRef {
            ptr: v.into(),
            schema,
        }
    }

    /// Create a new [`SchemaRef`] from a raw pointer and it's schema.
    ///
    /// # Safety
    /// - `ptr` must point to valid value of whatever the pointee type is.
    /// - `ptr` must not be null.
    /// - If the `A` type parameter is [`Aligned`] then `inner` must be sufficiently aligned for the
    ///   pointee type.
    /// - `inner` must have correct provenance to allow read and writes of the pointee type.
    /// - The lifetime `'a` must be constrained such that this [`PtrMut`] will stay valid and
    ///   nothing else can read or mutate the pointee while this [`PtrMut`] is live.
    #[track_caller]
    pub unsafe fn from_ptr_schema(ptr: *const u8, schema: &'static Schema) -> Self {
        Self {
            // SOUND: casting the `*const u8` to a `*mut u8` is dangerous but sound in this case
            // because we are passing the `NonNull` to a read-only `Ptr`. Unfortunately there's not
            // a read-only `NonNull` type to do that to instead. Also, the user verifies that the
            // pointer is non-null.
            ptr: unsafe { Ptr::new(NonNull::new_unchecked(ptr as *mut u8)) },
            schema,
        }
    }

    /// Get a pointer to a field.
    ///
    /// # Panics
    ///
    /// Panics if the field doesn't exist in the schema.
    #[track_caller]
    pub fn field<'a, I: Into<FieldIdx<'a>>>(&self, idx: I) -> SchemaRef<'pointer> {
        self.get_field(idx).unwrap()
    }

    /// Get a nested field from the box.
    ///
    /// # Panics
    ///
    /// Panics if the field doesn't exist in the schema.
    #[track_caller]
    pub fn field_path<'a, I: IntoIterator<Item = FieldIdx<'a>>>(
        self,
        path: I,
    ) -> SchemaRef<'pointer> {
        self.get_field_path(path).unwrap()
    }

    /// Get a nested field from the box.
    ///
    /// # Errors
    ///
    /// Errors if the field doesn't exist in the schema.
    pub fn get_field_path<'a, I: IntoIterator<Item = FieldIdx<'a>>>(
        self,
        path: I,
    ) -> Result<SchemaRef<'pointer>, SchemaFieldNotFoundError<'a>> {
        let mut schemaref = self;
        for item in path {
            schemaref = schemaref.get_field(item)?;
        }
        Ok(schemaref)
    }

    /// Get a pointer to a field.
    ///
    /// # Errors
    ///
    /// Errors if the field doesn't exist in the schema.
    pub fn get_field<'a, I: Into<FieldIdx<'a>>>(
        &self,
        idx: I,
    ) -> Result<SchemaRef<'pointer>, SchemaFieldNotFoundError<'a>> {
        let idx = idx.into();
        let not_found = Err(SchemaFieldNotFoundError { idx });
        match &self.schema.kind {
            SchemaKind::Struct(s) => {
                let field_offsets = self.schema.field_offsets();
                let Some((idx, offset)) =
                    field_offsets
                        .iter()
                        .enumerate()
                        .find_map(|(i, (name, offset))| {
                            let matches = match idx {
                                FieldIdx::Idx(n) => n == i,
                                FieldIdx::Name(n) => name.as_deref() == Some(n),
                            };
                            if matches {
                                Some((i, *offset))
                            } else {
                                None
                            }
                        })
                else {
                    return not_found;
                };
                let field = &s.fields[idx];

                Ok(SchemaRef {
                    // SOUND: the schema certifies the soundness of the offset for the given field.
                    ptr: unsafe { self.ptr.byte_add(offset) },
                    schema: field.schema,
                })
            }
            SchemaKind::Box(_) => {
                // SOUND: schema asserts that type is box
                let the_box = unsafe { self.ptr.deref::<SchemaBox>() };
                the_box.get_field(idx)
            }
            SchemaKind::Vec(_)
            | SchemaKind::Primitive(_)
            | SchemaKind::Map { .. }
            | SchemaKind::Enum(_) => not_found,
        }
    }

    /// Get the pointer.
    pub fn as_ptr(&self) -> *const u8 {
        self.ptr.as_ptr()
    }

    /// # Safety
    /// Assert that the pointer is valid for type T, and that the lifetime is valid.
    pub unsafe fn deref<T>(&self) -> &'pointer T {
        self.ptr.deref()
    }

    /// Get the [`Schema`] for the pointer.
    pub fn schema(&self) -> &'static Schema {
        self.schema
    }

    /// Get the hash of this schema box, if supported.
    pub fn hash(&self) -> Option<u64> {
        self.schema
            .hash_fn
            .map(|hash_fn| unsafe { (hash_fn)(self.ptr.as_ptr()) })
    }

    /// Borrow the schema ref as a [`SchemaMap`] if it is one.
    pub fn as_map(&self) -> Option<&'pointer SchemaMap> {
        matches!(self.schema.kind, SchemaKind::Map { .. })
            // SOUND: Schema asserts this is a schema map
            .then_some(unsafe { self.ptr.deref::<SchemaMap>() })
    }

    /// Borrow the schema ref as a [`SchemaVec`] if it is one.
    pub fn as_vec(&self) -> Option<&'pointer SchemaVec> {
        matches!(self.schema.kind, SchemaKind::Vec(_))
            // SOUND: Schema asserts this is a schema map
            .then_some(unsafe { self.ptr.deref::<SchemaVec>() })
    }

    /// Borrow the schema ref as a [`SchemaBox`] if it is one.
    pub fn as_box(&self) -> Option<SchemaRef<'pointer>> {
        matches!(self.schema.kind, SchemaKind::Vec(_))
            // SOUND: Schema asserts this is a schema box
            .then_some(unsafe { self.ptr.deref::<SchemaBox>().as_ref() })
    }

    /// Get a helper to access the inner data at runtime.
    pub fn access(&self) -> SchemaRefAccess<'pointer> {
        (*self).into()
    }

    /// Debug format the value stored in the schema box.
    ///
    /// This is used in the display and debug implementations.
    pub fn debug_format_value(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self.access() {
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
                PrimitiveRef::Opaque { size, align } => f
                    .debug_struct("Opaque")
                    .field("size", &size)
                    .field("align", &align)
                    .finish(),
            },
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
pub enum SchemaRefAccess<'a> {
    /// Access a struct.
    Struct(StructRefAccess<'a>),
    /// Access a vec.
    Vec(&'a SchemaVec),
    /// Access an enum.
    Enum(EnumRefAccess<'a>),
    /// Access a map.
    Map(&'a SchemaMap),
    /// Access a struct.
    Primitive(PrimitiveRef<'a>),
}

impl<'a> From<SchemaRef<'a>> for SchemaRefAccess<'a> {
    fn from(value: SchemaRef<'a>) -> Self {
        match &value.schema.kind {
            SchemaKind::Struct(_) => SchemaRefAccess::Struct(StructRefAccess(value)),
            SchemaKind::Vec(_) => SchemaRefAccess::Vec(value.as_vec().unwrap()),
            SchemaKind::Enum(_) => SchemaRefAccess::Enum(EnumRefAccess(value)),
            SchemaKind::Map { .. } => SchemaRefAccess::Map(value.as_map().unwrap()),
            SchemaKind::Box(_) => value.as_box().unwrap().access(),
            SchemaKind::Primitive(_) => SchemaRefAccess::Primitive(value.into()),
        }
    }
}

/// Helper for accessing the inner data of a schema ref at runtime.
pub struct StructRefAccess<'a>(pub SchemaRef<'a>);

impl<'a> StructRefAccess<'a> {
    /// Get the struct's schema.
    pub fn schema(&self) -> &'static Schema {
        self.0.schema
    }

    /// Iterate over fields in the struct.
    pub fn fields(&self) -> StructRefFieldIter<'a> {
        StructRefFieldIter {
            ptr: self.0,
            field_idx: 0,
        }
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
        let ptr = self.ptr.field(self.field_idx);
        self.field_idx += 1;
        Some(StructRefFieldIterField {
            name: name.as_ref().map(|x| x.as_str()),
            value: ptr,
        })
    }
}

/// Helper for accessing the inner data of a schema ref at runtime.
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
            ptr: unsafe { self.0.ptr.byte_add(value_offset) },
            schema,
        })
    }
}

/// Helper for accessing the inner data of a schema ref at runtime.
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
    /// An [`f23`]
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
    },
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
                },
            },
            _ => panic!("Schema mismatch"),
        }
    }
}

/// An untyped mutable reference that knows the [`Schema`] of the pointee and that can be cast to a matching
/// type.
// TODO: Re-evaluate whether or not it is necessary to have two lifetimes for `SchemaRefMut`.
// I believe the current implementation is sound, and the extra liftime is not a huge annoyance, but
// it would be good to simplify if possible. See the comment below on `parent_lifetime` for a
// description of the purpose of the second lifetime. We need to maintain the effect of the
// lifetime, but we might be able to do that by using the '`pointer` lifetime to represent the
// `'parent` lifetime when necessary. **Note:** This is a little more advanced rust than "normal".
// This is not a beginner issue.
pub struct SchemaRefMut<'pointer, 'parent> {
    ptr: PtrMut<'pointer>,
    schema: &'static Schema,
    /// This `'parent` lifetime is used to lock the borrow to the parent [`SchemaRefMut`] if this
    /// was created by borrowing the field of another [`SchemaRefMut`].
    ///
    /// A top-level [`SchemaRefMut`] that doesn't borrow from another one will have the 'parent
    /// lifetime equal to the 'pointer lifetime.
    ///
    /// This allows us to prevent borrowing the [`SchemaRefMut`], while one of the children
    /// [`SchemaRefMut`]s are potentially writing to the fields.
    ///
    /// In other words, this represents that the child [`SchemaRefMut`] may borrow mutably from it's
    /// parent schema walker.
    parent_lifetime: PhantomData<&'parent mut ()>,
}

impl<'pointer, 'parent> std::fmt::Debug for SchemaRefMut<'pointer, 'parent> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SchemaRefMut")
            // .field("ptr", &self.ptr)
            .field("schema", &self.schema)
            // .field("parent_lifetime", &self.parent_lifetime)
            .finish()
    }
}

impl<'pointer, 'parent> SchemaRefMut<'pointer, 'parent> {
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
            // SOUND: here we clone our mutable pointer and dereference it. This is dangerous, but
            // sound in this case because we don't use our pointer at the same time as it, and we
            // make sure that we lock ourselves with a mutable borrow until the user drops the
            // reference that we gave them.
            unsafe {
                let copied_ptr: PtrMut<'_, Aligned> =
                    PtrMut::new(NonNull::new_unchecked(self.ptr.as_ptr()));

                Ok(copied_ptr.deref_mut())
            }
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
            // We've checked that the pointer represents T
            Ok(unsafe { self.ptr.deref_mut() })
        } else {
            Err(SchemaMismatchError)
        }
    }

    /// Create a new [`SchemaRefMut`] from a reference to a type that implements [`HasSchema`].
    pub fn new<T: HasSchema>(v: &'pointer mut T) -> SchemaRefMut<'pointer, 'static> {
        let schema = T::schema();
        SchemaRefMut {
            ptr: v.into(),
            schema,
            parent_lifetime: PhantomData,
        }
    }

    /// Create a new [`SchemaRefMut`] from a raw pointer and it's schema.
    ///
    /// # Safety
    /// - `ptr` must point to valid value of whatever the pointee type is.
    /// - `ptr` must not be null.
    /// - If the `A` type parameter is [`Aligned`] then `inner` must be sufficiently aligned for the
    ///   pointee type.
    /// - `ptr` must have correct provenance to allow read and writes of the pointee type.
    /// - The lifetime `'a` must be constrained such that this [`PtrMut`] will stay valid and
    ///   nothing else can read or mutate the pointee while this [`PtrMut`] is live.
    pub unsafe fn from_ptr_schema(
        ptr: *mut u8,
        schema: &'static Schema,
    ) -> SchemaRefMut<'pointer, 'parent> {
        Self {
            ptr: PtrMut::new(NonNull::new_unchecked(ptr)),
            schema,
            parent_lifetime: PhantomData,
        }
    }

    /// Get a pointer to a field.
    ///
    /// # Panics
    ///
    /// Panics if the field doesn't exist in the schema.
    #[track_caller]
    pub fn field<'this, 'b, I: Into<FieldIdx<'b>>>(
        &'this mut self,
        idx: I,
    ) -> SchemaRefMut<'pointer, 'this> {
        self.get_field(idx).unwrap()
    }

    /// Get a nested field from the box.
    ///
    /// # Panics
    ///
    /// Panics if the field doesn't exist in the schema.
    #[track_caller]
    pub fn into_field_path<'a, I: IntoIterator<Item = FieldIdx<'a>>>(
        self,
        path: I,
    ) -> SchemaRefMut<'pointer, 'parent> {
        self.try_into_field_path(path).unwrap()
    }

    /// Get a nested field from the box.
    ///
    /// # Errors
    ///
    /// Errors if the field doesn't exist in the schema.
    pub fn try_into_field_path<'a, I: IntoIterator<Item = FieldIdx<'a>>>(
        self,
        path: I,
    ) -> Result<SchemaRefMut<'pointer, 'parent>, Self> {
        let mut schemaref = self;
        for item in path {
            schemaref = schemaref.try_into_field(item)?;
        }
        Ok(schemaref)
    }

    /// Get a nested field from the box.
    ///
    /// # Panics
    ///
    /// Panics if the field doesn't exist in the schema.
    #[track_caller]
    pub fn get_field_path<'this, 'a, I: IntoIterator<Item = FieldIdx<'a>>>(
        &'this mut self,
        path: I,
    ) -> SchemaRefMut<'pointer, 'this> {
        self.try_get_field_path(path).unwrap()
    }

    /// Get a nested field from the box.
    ///
    /// # Errors
    ///
    /// Errors if the field doesn't exist in the schema.
    pub fn try_get_field_path<'this, 'a, I: IntoIterator<Item = FieldIdx<'a>>>(
        &'this mut self,
        path: I,
    ) -> Result<SchemaRefMut<'pointer, 'this>, SchemaFieldNotFoundError<'a>> {
        let mut schemaref = Self {
            // SOUND: we are cloning our mutable reference here, but we are returning only one
            // of them, and it contains the 'this lifetime that indicates a borrow of this one,
            // preventing both from being used at the same time.
            ptr: unsafe { PtrMut::new(NonNull::new_unchecked(self.ptr.as_ptr())) },
            schema: self.schema,
            parent_lifetime: PhantomData,
        };
        for item in path {
            schemaref = schemaref
                .try_into_field(item)
                .map_err(|_| SchemaFieldNotFoundError { idx: item })?;
        }
        Ok(schemaref)
    }

    /// Get a pointer to a field.
    ///
    /// # Errors
    ///
    /// Errors if the field doesn't exist in the schema.
    pub fn get_field<'this, 'idx, I: Into<FieldIdx<'idx>>>(
        &'this mut self,
        idx: I,
    ) -> Result<SchemaRefMut<'pointer, 'this>, SchemaFieldNotFoundError<'idx>> {
        let idx = idx.into();
        let not_found = Err(SchemaFieldNotFoundError { idx });
        match &self.schema.kind {
            SchemaKind::Struct(s) => {
                let field_offsets = self.schema.field_offsets();
                let Some((idx, offset)) =
                    field_offsets
                        .iter()
                        .enumerate()
                        .find_map(|(i, (name, offset))| {
                            let matches = match idx {
                                FieldIdx::Idx(n) => n == i,
                                FieldIdx::Name(n) => name.as_deref() == Some(n),
                            };
                            if matches {
                                Some((i, *offset))
                            } else {
                                None
                            }
                        })
                else {
                    return not_found;
                };
                let field = &s.fields[idx];

                // SOUND: here we clone our mutable pointer, and then offset it according to the
                // field. This is dangerous, but sound because we make sure that this
                // `get_field` method returns a `SchemaWalkerMut` with a virtual mutable borrow
                // to this one.
                //
                // This means that Rust will not let anybody use this `SchemaWalkerMut`, until
                // the other one is dropped. That means all we have to do is not use our
                // `self.ptr` while the `offset_ptr` exists.
                //
                // Additionally, the `new_unchecked` is sound because our pointer cannot be null
                // because it comes out of a `PtrMut`.
                let offset_ptr = unsafe {
                    PtrMut::new(NonNull::new_unchecked(self.ptr.as_ptr())).byte_add(offset)
                };

                Ok(SchemaRefMut {
                    ptr: offset_ptr,
                    schema: field.schema,
                    parent_lifetime: PhantomData,
                })
            }
            SchemaKind::Box(_) => {
                // SOUND: schema asserts that type is box
                let the_box = unsafe { &mut *(self.ptr.as_ptr() as *mut SchemaBox) };
                the_box.get_field_mut(idx)
            }
            SchemaKind::Map { .. }
            | SchemaKind::Vec(_)
            | SchemaKind::Primitive(_)
            | SchemaKind::Enum(_) => not_found,
        }
    }

    /// Convert this ref into a ref to one of it's fields.
    ///
    /// This is useful because it consumes self and avoids keeping a reference to it's parent
    /// [`SchemaRefMut`].
    /// # Panics
    /// Panics if the field does not exist.
    #[inline]
    #[track_caller]
    pub fn into_field<'idx, I: Into<FieldIdx<'idx>>>(
        self,
        idx: I,
    ) -> SchemaRefMut<'pointer, 'parent> {
        self.try_into_field(idx).unwrap()
    }

    /// Convert this ref into a ref to one of it's fields.
    ///
    /// This is useful because it consumes self and avoids keeping a reference to it's parent
    /// [`SchemaRefMut`].
    /// # Errors
    /// Errors if the field does not exist.
    pub fn try_into_field<'idx, I: Into<FieldIdx<'idx>>>(
        mut self,
        idx: I,
    ) -> Result<SchemaRefMut<'pointer, 'parent>, Self> {
        match self.get_field(idx) {
            Ok(r) => Ok(SchemaRefMut {
                ptr: r.ptr,
                schema: r.schema,
                parent_lifetime: PhantomData,
            }),
            Err(_) => Err(self),
        }
    }

    /// Get the raw pointer
    pub fn as_ptr(&self) -> *mut u8 {
        self.ptr.as_ptr()
    }

    /// # Safety
    /// You assert that the pointer points to a valid instance of T with the given lifetime.
    pub unsafe fn deref_mut<T>(self) -> &'pointer mut T {
        self.ptr.deref_mut()
    }

    /// Get the [`Schema`] for the pointer.
    pub fn schema(&self) -> &'static Schema {
        self.schema
    }

    /// Get the hash of this schema box, if supported.
    pub fn hash(&self) -> Option<u64> {
        self.schema
            .hash_fn
            .map(|hash_fn| unsafe { (hash_fn)(self.ptr.as_ptr()) })
    }
}

/// A owning, type-erased [`Box`]-like container for types with a [`Schema`].
pub struct SchemaBox {
    ptr: OwningPtr<'static>,
    schema: &'static Schema,
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
        let Some(hash_fn) = self.schema.hash_fn else {
            panic!("Cannot hash schema box where schema doesn't provide hash_fn");
        };
        let hash = unsafe { (hash_fn)(self.ptr.as_ptr()) };
        state.write_u64(hash);
    }
}

impl PartialEq for SchemaBox {
    fn eq(&self, other: &Self) -> bool {
        if self.schema != other.schema {
            panic!("Cannot compare two `SchemaBox`s with different schemas.");
        }
        let Some(eq_fn) = self.schema.eq_fn else {
            panic!("Cannot hash schema box where schema doesn't provide hash_fn.");
        };
        unsafe { (eq_fn)(self.ptr.as_ptr(), other.ptr.as_ptr()) }
    }
}
impl Eq for SchemaBox {}

impl Clone for SchemaBox {
    fn clone(&self) -> Self {
        let clone_fn = self.schema.clone_fn.unwrap_or_else(|| {
            panic!(
                "The schema for this type does not allow cloning it.\nSchema: {:#?}",
                self.schema
            )
        });

        let layout = self.schema.layout();
        let new_ptr = if layout.size() == 0 {
            NonNull::<u8>::dangling().as_ptr()
        } else {
            // SOUND: Non-zero size for layout
            unsafe { std::alloc::alloc(layout) }
        };
        let new_ptr = unsafe {
            (clone_fn)(self.ptr.as_ref().as_ptr(), new_ptr);
            OwningPtr::new(NonNull::new(new_ptr).unwrap_or_else(|| handle_alloc_error(layout)))
        };
        Self {
            ptr: new_ptr,
            schema: self.schema,
        }
    }
}

impl SchemaBox {
    /// Cast this box to it's inner type and return it.
    /// # Panics
    /// Panics if the schema of the box does not match that of the type you are casting to.
    #[track_caller]
    pub fn into_inner<T: HasSchema>(self) -> T {
        self.try_into_inner().unwrap()
    }

    /// Cast this box to it's inner type and return it.
    /// # Errors
    /// Errors if the schema of the box does not match that of the type you are casting to.
    pub fn try_into_inner<T: HasSchema>(self) -> Result<T, SchemaMismatchError> {
        if self.schema == T::schema() {
            // We've validated that the schema of the box matches T
            Ok(unsafe { self.into_inner_unchecked() })
        } else {
            Err(SchemaMismatchError)
        }
    }

    /// Unsafely convert this box into an owned T.
    /// # Safety
    /// - The schema of type T must equal that of this box.
    pub unsafe fn into_inner_unchecked<T: HasSchema>(self) -> T {
        // Allocate memory for T on the stack
        let mut ret = MaybeUninit::<T>::uninit();

        // Copy the data from the box into the stack.
        // SOUND: We've validated that the box has the same schema as T
        unsafe {
            (ret.as_mut_ptr() as *mut u8)
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
            unsafe { Ok(self.ptr.as_ref().deref()) }
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
            unsafe { Ok(self.ptr.as_mut().deref_mut()) }
        } else {
            Err(SchemaMismatchError)
        }
    }

    /// Borrow this box as a [`SchemaRef`].
    pub fn as_ref(&self) -> SchemaRef<'_> {
        SchemaRef {
            ptr: self.ptr.as_ref(),
            schema: self.schema,
        }
    }

    /// Borrow this box as a [`SchemaRefMut`].
    pub fn as_mut(&mut self) -> SchemaRefMut<'_, '_> {
        SchemaRefMut {
            ptr: self.ptr.as_mut(),
            schema: self.schema,
            parent_lifetime: PhantomData,
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
            NonNull::<u8>::dangling().as_ptr()
        } else {
            // SOUND: Non-zero size for layout
            unsafe { std::alloc::alloc(layout) }
        };
        // SOUND: The pointer is allocated for the layout matching the schema.
        let ptr = unsafe {
            OwningPtr::new(NonNull::new(ptr).unwrap_or_else(|| handle_alloc_error(layout)))
        };

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
        let Some(default_fn) = schema.default_fn else {
            panic!("Schema doesn't have `default_fn` to create default value with.");
        };

        unsafe {
            let b = SchemaBox::uninitialized(schema);
            (default_fn)(b.ptr.as_ptr());
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
    pub unsafe fn from_raw_parts(ptr: OwningPtr<'static>, schema: &'static Schema) -> Self {
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
            .map(|hash_fn| unsafe { (hash_fn)(self.ptr.as_ptr()) })
    }

    /// Get the hash of this schema box.
    /// # Panics
    /// Panics if the schema doesn't implement hash.
    #[track_caller]
    pub fn hash(&self) -> u64 {
        self.try_hash().expect("Schema doesn't implement hash")
    }

    /// Get a ref to the field with the given name/index, and panic if it doesn't exist.
    #[inline]
    #[track_caller]
    pub fn field<'idx, I: Into<FieldIdx<'idx>>>(&self, idx: I) -> SchemaRef {
        self.get_field(idx).unwrap()
    }

    /// Get a reference to the field with the given name/index, if it exists.
    pub fn get_field<'idx, 'ptr, I: Into<FieldIdx<'idx>>>(
        &'ptr self,
        idx: I,
    ) -> Result<SchemaRef<'ptr>, SchemaFieldNotFoundError<'idx>> {
        self.as_ref().get_field(idx)
    }

    /// Get a ref to the field with the given name/index, and panic if it doesn't exist.
    #[inline]
    #[track_caller]
    pub fn field_mut<'idx, I: Into<FieldIdx<'idx>>>(&mut self, idx: I) -> SchemaRefMut {
        self.get_field_mut(idx).unwrap()
    }

    /// Get a mutable reference to the field with the given name/index, if it exists.
    pub fn get_field_mut<'idx, 'ptr, I: Into<FieldIdx<'idx>>>(
        &'ptr mut self,
        idx: I,
    ) -> Result<SchemaRefMut<'ptr, 'ptr>, SchemaFieldNotFoundError<'idx>> {
        let idx = idx.into();
        match self.as_mut().try_into_field(idx) {
            Ok(r) => Ok(r),
            Err(_) => Err(SchemaFieldNotFoundError { idx }),
        }
    }

    /// Deallocate the memory in the box.
    unsafe fn dealloc(&mut self) {
        if self.schema.layout().size() > 0 {
            std::alloc::dealloc(self.ptr.as_ptr(), self.schema.layout())
        }
    }

    /// Drop the inner type, without dealocating the box's memory.
    unsafe fn drop_inner(&mut self) {
        if let Some(drop_fn) = self.schema.drop_fn {
            // Drop the type
            (drop_fn)(self.ptr.as_mut().as_ptr());
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
                clone_fn: Some(<Self as RawClone>::raw_clone),
                drop_fn: Some(<Self as RawDrop>::raw_drop),
                default_fn: None,
                hash_fn: Some(<Self as RawHash>::raw_hash),
                eq_fn: Some(<Self as RawEq>::raw_eq),
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
        let Some(default_fn) = schema.default_fn else {
            panic!("Schema doesn't implement default");
        };
        Self {
            // SOUND: we initialize the schema box immediately, and the schema asserts the default
            // fn is valid for the type.
            b: unsafe {
                let mut b = SchemaBox::uninitialized(schema);
                (default_fn)(b.as_mut().as_ptr());
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
                clone_fn: Some(<Self as RawClone>::raw_clone),
                drop_fn: Some(<Self as RawDrop>::raw_drop),
                default_fn: Some(<Self as RawDefault>::raw_default),
                hash_fn: Some(SchemaVec::raw_hash),
                eq_fn: Some(SchemaVec::raw_eq),
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
        unsafe { self.b.ptr.as_ref().deref() }
    }
}
impl<T: HasSchema> std::ops::DerefMut for SBox<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.b.ptr.as_mut().deref_mut() }
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
/// it easier to use with [`SchemaRef::get_field_path()`] and other field path methods.
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
