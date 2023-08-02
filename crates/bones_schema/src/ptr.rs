//! Schema-aware smart pointers.

use std::{
    alloc::{handle_alloc_error, Layout},
    marker::PhantomData,
    ptr::NonNull,
};

use crate::prelude::*;
use bones_utils::prelude::*;

/// An untyped reference that knows the [`Schema`] of the pointee and that can be cast to a matching
/// type.
#[derive(Clone)]
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
            // SAFE: the schemas have the same memory representation.
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
    /// - `inner` must point to valid value of whatever the pointee type is.
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
            // a read-only `NonNull` type to do that to instead.
            ptr: unsafe { Ptr::new(NonNull::new(ptr as *mut u8).expect("Ptr cannot be null")) },
            schema,
        }
    }

    /// Get a pointer to a field.
    ///
    /// # Panics
    ///
    /// Panics if the field doesn't exist in the schema.
    #[track_caller]
    pub fn field<'a, I: Into<FieldIdx<'a>>>(&self, idx: I) -> SchemaRef {
        self.get_field(idx).unwrap()
    }

    /// Get a pointer to a field.
    ///
    /// # Errors
    ///
    /// Errors if the field doesn't exist in the schema.
    pub fn get_field<'a, I: Into<FieldIdx<'a>>>(
        &self,
        idx: I,
    ) -> Result<SchemaRef, SchemaFieldNotFoundError<'a>> {
        let idx = idx.into();
        match &self.schema.kind {
            SchemaKind::Struct(s) => {
                let field_offsets = self.schema.field_offsets();
                let Some((idx, offset)) = field_offsets.iter().enumerate().find_map(|(i, (name, offset))| {
                        let matches = match idx {
                            FieldIdx::Idx(n) => n == i,
                            FieldIdx::Name(n) => name == &Some(n),
                        };
                        if matches {
                            Some((i, *offset))
                        } else {
                            None
                        }
                    }) else { return Err(SchemaFieldNotFoundError { idx }) };
                let field = &s.fields[idx];

                Ok(SchemaRef {
                    // SOUND: the schema certifies the soundness of the offset for the given field.
                    ptr: unsafe { self.ptr.byte_add(offset) },
                    schema: field.schema,
                })
            }
            SchemaKind::Vec(_) => Err(SchemaFieldNotFoundError { idx }),
            SchemaKind::Primitive(_) => Err(SchemaFieldNotFoundError { idx }),
        }
    }

    /// Get the pointer.
    pub fn ptr(&self) -> Ptr<'pointer> {
        self.ptr
    }

    /// Get the [`Schema`] for the pointer.
    pub fn schema(&self) -> &Schema {
        self.schema
    }
}

/// An untyped mutable reference that knows the [`Schema`] of the pointee and that can be cast to a matching
/// type.
pub struct SchemaRefMut<'pointer, 'parent> {
    ptr: PtrMut<'pointer>,
    schema: &'static Schema,
    /// This `'parent` lifetime is used to lock the borrow to the parent [`SchemaWalkerMut`] if
    /// this is a walker for a field of another walker.
    ///
    /// The top-level walker's `'parent` lifetime will be `'static`, meaning it doesn't have a
    /// parent.
    ///
    /// This allows us to prevent borrowing the parent walker, while one of the children walkers
    /// are potentially writing to the fields.
    ///
    /// In other words, this represents that the child schema walker may borrow mutably from
    /// it's parent schema walker.
    parent_lifetime: PhantomData<&'parent mut ()>,
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
    ///
    /// Errors if the schema of the pointer does not match that of the type you are casting to.
    pub fn try_cast_mut<T: HasSchema>(&mut self) -> Result<&mut T, SchemaMismatchError> {
        if self.schema.represents(T::schema()) {
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
            unsafe {
                let copied_ptr: PtrMut<'_, Aligned> =
                    PtrMut::new(NonNull::new_unchecked(self.ptr.as_ptr()));

                Ok(copied_ptr.deref_mut())
            }
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
    /// - `inner` must point to valid value of whatever the pointee type is.
    /// - If the `A` type parameter is [`Aligned`] then `inner` must be sufficiently aligned for the
    ///   pointee type.
    /// - `inner` must have correct provenance to allow read and writes of the pointee type.
    /// - The lifetime `'a` must be constrained such that this [`PtrMut`] will stay valid and
    ///   nothing else can read or mutate the pointee while this [`PtrMut`] is live.
    pub unsafe fn from_ptr_schema(
        ptr: *mut u8,
        schema: &'static Schema,
    ) -> SchemaRefMut<'pointer, 'parent> {
        Self {
            ptr: PtrMut::new(NonNull::new(ptr as *mut u8).expect("Ptr cannot be null")),
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

    /// Get a pointer to a field.
    ///
    /// # Errors
    ///
    /// Errors if the field doesn't exist in the schema.
    pub fn get_field<'this, 'b, I: Into<FieldIdx<'b>>>(
        &'this mut self,
        idx: I,
    ) -> Result<SchemaRefMut<'pointer, 'this>, SchemaMismatchError> {
        let idx = idx.into();
        match &self.schema.kind {
            SchemaKind::Struct(s) => {
                let field_offsets = self.schema.field_offsets();
                let Some((idx, offset)) = field_offsets.iter().enumerate().find_map(|(i, (name, offset))| {
                        let matches = match idx {
                            FieldIdx::Idx(n) => n == i,
                            FieldIdx::Name(n) => name == &Some(n),
                        };
                        if matches {
                            Some((i, *offset))
                        } else {
                            None
                        }
                    }) else { return Err(SchemaMismatchError) };
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
            SchemaKind::Vec(_) => Err(SchemaMismatchError),
            SchemaKind::Primitive(_) => Err(SchemaMismatchError),
        }
    }

    /// Get the raw pointer.
    pub fn ptr(&mut self) -> &PtrMut<'pointer> {
        &self.ptr
    }

    /// Get the [`Schema`] for the pointer.
    pub fn schema(&self) -> &Schema {
        self.schema
    }
}

/// A owning, type-erased [`Box`]-like container for types with a [`Schema`].
pub struct SchemaBox {
    ptr: OwningPtr<'static>,
    schema: &'static Schema,
    layout: Layout,
}
unsafe impl Sync for SchemaBox {}
unsafe impl Send for SchemaBox {}
impl std::fmt::Debug for SchemaBox {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SchemaBox")
            .field("schema", &self.schema)
            .field("layout", &self.layout)
            .finish_non_exhaustive()
    }
}

impl Clone for SchemaBox {
    fn clone(&self) -> Self {
        let clone_fn = self.schema.clone_fn.unwrap_or_else(|| {
            panic!(
                "The schema for this type does not allow cloning it.\nSchema: {:#?}",
                self.schema
            )
        });

        let new_ptr = if self.layout.size() == 0 {
            NonNull::<u8>::dangling().as_ptr()
        } else {
            // SAFE: Non-zero size for layout
            unsafe { std::alloc::alloc(self.layout) }
        };
        let new_ptr = unsafe {
            (clone_fn)(self.ptr.as_ref().as_ptr(), new_ptr);
            OwningPtr::new(NonNull::new(new_ptr).unwrap_or_else(|| handle_alloc_error(self.layout)))
        };
        Self {
            ptr: new_ptr,
            schema: self.schema,
            layout: self.layout,
        }
    }
}

impl Drop for SchemaBox {
    fn drop(&mut self) {
        unsafe {
            if let Some(drop_fn) = self.schema.drop_fn {
                // Drop the type
                (drop_fn)(self.ptr.as_mut().as_ptr());
            }

            // De-allocate the memory
            if self.layout.size() > 0 {
                std::alloc::dealloc(self.ptr.as_mut().as_ptr(), self.layout)
            }
        }
    }
}

impl SchemaBox {
    /// Cast this pointer to a reference to a type with a matching [`Schema`].
    ///
    /// # Panics
    ///
    /// Panics if the schema of the pointer does not match that of the type you are casting to.
    #[track_caller]
    pub fn cast<T: HasSchema>(&self) -> &T {
        self.try_cast().expect(SchemaMismatchError::MSG)
    }

    /// Cast this pointer to a reference to a type with a matching [`Schema`].
    ///
    /// # Errors
    ///
    /// Errors if the schema of the pointer does not match that of the type you are casting to.
    pub fn try_cast<T: HasSchema>(&self) -> Result<&T, SchemaMismatchError> {
        if self.schema.represents(T::schema()) {
            // SAFE: the schemas have the same memory representation.
            unsafe { Ok(self.ptr.as_ref().deref()) }
        } else {
            Err(SchemaMismatchError)
        }
    }

    /// Cast this pointer to a mutable reference to a type with a matching [`Schema`].
    ///
    /// # Panics
    ///
    /// Panics if the schema of the pointer does not match that of the type you are casting to.
    #[track_caller]
    pub fn cast_mut<T: HasSchema>(&mut self) -> &mut T {
        self.try_cast_mut().expect(SchemaMismatchError::MSG)
    }

    /// Cast this pointer to a mutable reference to a type with a matching [`Schema`].
    ///
    /// # Errors
    ///
    /// Errors if the schema of the pointer does not match that of the type you are casting to.
    pub fn try_cast_mut<T: HasSchema>(&mut self) -> Result<&mut T, SchemaMismatchError> {
        if self.schema.represents(T::schema()) {
            // SAFE: the schemas have the same memory representation.
            unsafe { Ok(self.ptr.as_mut().deref_mut()) }
        } else {
            Err(SchemaMismatchError)
        }
    }

    /// Borrow this as a [`SchemaRef`].
    pub fn as_ref(&self) -> SchemaRef<'_> {
        SchemaRef {
            ptr: self.ptr.as_ref(),
            schema: self.schema,
        }
    }

    /// Borrow this as a [`SchemaRefMut`].
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
        let layout = std::alloc::Layout::new::<T>();
        debug_assert_eq!(
            layout,
            schema.layout(),
            "BIG BUG: Schema layout doesn't match type layout!!"
        );

        let ptr = if layout.size() == 0 {
            NonNull::<u8>::dangling().as_ptr()
        } else {
            // SAFE: Non-zero size for layout
            unsafe { std::alloc::alloc(layout) }
        };
        // SAFE: The pointer is allocated for the layout of type T.
        let ptr = unsafe {
            (ptr as *mut T).write(v);
            OwningPtr::new(NonNull::new(ptr).unwrap_or_else(|| handle_alloc_error(layout)))
        };

        Self {
            ptr,
            schema,
            layout,
        }
    }

    /// Allocates a [`SchemaBox`] for the given [`Schema`], but **doesn't initialize the memory**.
    ///
    /// # Safety
    ///
    /// Accessing the data in an unitinialized [`SchemaBox`] is undefined behavior. It is up to the user to
    /// initialize the memory pointed at by the box after creating it.
    pub unsafe fn uninitialized(schema: &'static Schema) -> Self {
        let layout = schema.layout();

        let ptr = if layout.size() == 0 {
            NonNull::<u8>::dangling().as_ptr()
        } else {
            // SAFE: Non-zero size for layout
            unsafe { std::alloc::alloc(layout) }
        };
        // SOUND: The pointer is allocated for the layout matching the schema.
        let ptr = unsafe {
            OwningPtr::new(NonNull::new(ptr).unwrap_or_else(|| handle_alloc_error(layout)))
        };

        Self {
            ptr,
            schema,
            layout,
        }
    }

    /// Deallocate the memory stored in the box, but don't run the destructor.
    pub fn forget(self) {
        unsafe { std::alloc::dealloc(self.ptr.as_ptr(), self.layout) }
        std::mem::forget(self);
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

    /// Create a new [`SchemaBox`] from raw parts.
    ///
    /// This is useful for creating a [`SchemaBox`] for data with a schema loaded at runtime and
    /// without a Rust type.
    ///
    /// # Safety
    ///
    /// - You must insure that the pointer is valid for the given `layout`, `schem`, `drop_fn`, and
    /// `clone_fn`.
    pub unsafe fn from_raw_parts(ptr: OwningPtr<'static>, schema: &'static Schema) -> Self {
        Self {
            ptr,
            layout: schema.layout(),
            schema,
        }
    }

    /// Get the [`Schema`] for the pointer.
    pub fn schema(&self) -> &Schema {
        self.schema
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

/// Error type when attempting to cast between types with mis-matched schemas.
#[derive(Debug)]
pub struct SchemaMismatchError;
impl SchemaMismatchError {
    /// The display error message for this error type.
    pub const MSG: &str = "Invalid cast: the schemas of the casted types are not compatible.";
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
