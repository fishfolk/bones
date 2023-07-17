use std::ptr::NonNull;

use super::*;

/// A wrapper for a pointer, that also contains it's schema.
#[derive(Clone)]
pub struct SchemaPtr<'a> {
    ptr: Ptr<'a>,
    schema: Schema,
}

impl<'a> SchemaPtr<'a> {
    /// Cast this pointer to a reference to a type with a matching [`Schema`].
    ///
    /// # Panics
    ///
    /// Panics if the schema of the pointer does not match that of the type you are casting to.
    #[track_caller]
    pub fn cast<T: HasSchema>(&self) -> *const T {
        self.try_cast().expect(SchemaMismatchError::MSG)
    }

    /// Cast this pointer to a reference to a type with a matching [`Schema`].
    ///
    /// # Errors
    ///
    /// Errors if the schema of the pointer does not match that of the type you are casting to.
    pub fn try_cast<T: HasSchema>(&self) -> Result<&'a T, SchemaMismatchError> {
        if self.schema.represents(T::schema()) {
            // SAFE: the schemas have the same memory representation.
            Ok(unsafe { self.ptr.deref() })
        } else {
            Err(SchemaMismatchError)
        }
    }

    /// Create a new [`SchemaPtr`] from a reference to a type that implements [`HasSchema`].
    pub fn new<T: HasSchema>(v: &'a T) -> Self {
        let schema = T::schema().clone();
        Self {
            ptr: v.into(),
            schema,
        }
    }

    /// Create a new [`SchemaPtr`] from a raw pointer and it's schema.
    #[track_caller]
    pub fn from_ptr_schema(ptr: *const u8, schema: Schema) -> Self {
        Self {
            // SAFE: casting the `*const u8` to a `*mut u8` is dangerous but safe in this case
            // because we are passing the `NonNull` to a read-only `Ptr`. Unfortunately there's not
            // a read-only `NonNull` type to do that to instead.
            ptr: unsafe { Ptr::new(NonNull::new(ptr as *mut u8).expect("Ptr cannot be null")) },
            schema,
        }
    }

    /// Get the pointer.
    pub fn ptr(&self) -> Ptr<'a> {
        self.ptr
    }

    /// Get the [`Schema`] for the pointer.
    pub fn schema(&self) -> &Schema {
        &self.schema
    }
}

/// A wrapper for a pointer, that also contains it's schema.
pub struct SchemaPtrMut<'a> {
    ptr: PtrMut<'a>,
    schema: Schema,
}

impl<'a> SchemaPtrMut<'a> {
    /// Cast this pointer to a reference to a type with a matching [`Schema`].
    ///
    /// # Panics
    ///
    /// Panics if the schema of the pointer does not match that of the type you are casting to.
    #[track_caller]
    pub fn cast_mut<T: HasSchema + 'static>(self) -> &'a mut T {
        self.try_cast_mut().expect(SchemaMismatchError::MSG)
    }

    /// Cast this pointer to a mutable reference to a type with a matching [`Schema`].
    ///
    /// # Errors
    ///
    /// Errors if the schema of the pointer does not match that of the type you are casting to.
    pub fn try_cast_mut<T: HasSchema>(self) -> Result<&'a mut T, SchemaPtrMutMismatchError<'a>> {
        if self.schema.represents(T::schema()) {
            // SAFE: the schemas have the same memory representation.
            Ok(unsafe { self.ptr.deref_mut() })
        } else {
            Err(SchemaPtrMutMismatchError(self))
        }
    }

    /// Create a new [`SchemaPtr`] from a reference to a type that implements [`HasSchema`].
    pub fn new<T: HasSchema>(v: &'a mut T) -> Self {
        let schema = T::schema().clone();
        Self {
            ptr: v.into(),
            schema,
        }
    }

    /// Create a new [`SchemaPtr`] from a raw pointer and it's schema.
    ///
    /// # Safety
    /// - `inner` must point to valid value of whatever the pointee type is.
    /// - If the `A` type parameter is [`Aligned`] then `inner` must be sufficiently aligned for the
    ///   pointee type.
    /// - `inner` must have correct provenance to allow read and writes of the pointee type.
    /// - The lifetime `'a` must be constrained such that this [`PtrMut`] will stay valid and
    ///   nothing else can read or mutate the pointee while this [`PtrMut`] is live.
    pub unsafe fn from_ptr_schema(ptr: *mut u8, schema: Schema) -> Self {
        Self {
            ptr: PtrMut::new(NonNull::new(ptr as *mut u8).expect("Ptr cannot be null")),
            schema,
        }
    }

    /// Get the raw pointer.
    pub fn ptr(&self) -> &PtrMut<'a> {
        &self.ptr
    }

    /// Get the [`Schema`] for the pointer.
    pub fn schema(&self) -> &Schema {
        &self.schema
    }
}

/// A owning, type-erased [`Box`]-like container.
pub struct SchemaBox {
    ptr: OwningPtr<'static>,
    schema: Schema,
    layout: Layout,
    drop_fn: unsafe extern "C" fn(*mut u8),
    clone_fn: unsafe extern "C" fn(src: *const u8, dst: *mut u8),
}
unsafe impl Sync for SchemaBox {}
unsafe impl Send for SchemaBox {}
impl std::fmt::Debug for SchemaBox {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SchemaBox")
            .field("schema", &self.schema)
            .field("layout", &self.layout)
            .field("drop_fn", &self.drop_fn)
            .field("clone_fn", &self.clone_fn)
            .finish_non_exhaustive()
    }
}

impl Clone for SchemaBox {
    fn clone(&self) -> Self {
        // SAFE: the layout is not zero, or we could not create this box,
        // and the new pointer has the same layout as the source.
        let new_ptr = unsafe {
            let new_ptr = std::alloc::alloc(self.layout);
            (self.clone_fn)(self.ptr.as_ref().as_ptr(), new_ptr);
            OwningPtr::new(NonNull::new(new_ptr).expect("Allocation failed"))
        };
        Self {
            ptr: new_ptr,
            schema: self.schema.clone(),
            layout: self.layout,
            drop_fn: self.drop_fn,
            clone_fn: self.clone_fn,
        }
    }
}

impl Drop for SchemaBox {
    fn drop(&mut self) {
        unsafe {
            // Drop the type
            (self.drop_fn)(self.ptr.as_mut().as_ptr());
            // De-allocate the memory
            std::alloc::dealloc(self.ptr.as_mut().as_ptr(), self.layout)
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

    /// Create a new [`SchemaBox`].
    #[track_caller]
    pub fn new<T: HasSchema + Clone + Sync + Send>(v: T) -> Self {
        let schema = T::schema().clone();
        let layout = std::alloc::Layout::new::<T>();
        debug_assert_eq!(
            layout,
            schema.layout_info().layout,
            "BIG BUG: Schema layout doesn't match type layout!!"
        );
        assert!(
            layout.size() != 0,
            "Cannot allocate SchemaBox for zero-sized-type"
        );

        // SAFE: we check that the layout is non-zero, and
        // the pointer is allocated for the layout of type T.
        let ptr = unsafe {
            let ptr = std::alloc::alloc(layout);
            let mut ptr = OwningPtr::new(NonNull::new(ptr).expect("Allocation failed"));
            *ptr.as_mut().deref_mut() = v;
            ptr
        };

        Self {
            ptr,
            schema,
            layout,
            drop_fn: <T as RawFns>::raw_drop,
            clone_fn: <T as RawFns>::raw_clone,
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
    pub unsafe fn from_raw_parts(
        ptr: OwningPtr<'static>,
        schema: Schema,
        drop_fn: unsafe extern "C" fn(*mut u8),
        clone_fn: unsafe extern "C" fn(src: *const u8, dst: *mut u8),
    ) -> Self {
        Self {
            ptr,
            layout: schema.layout_info().layout,
            schema,
            drop_fn,
            clone_fn,
        }
    }

    /// Get the [`Schema`] for the pointer.
    pub fn schema(&self) -> &Schema {
        &self.schema
    }
}

use bones_utils::{OwningPtr, Ptr, PtrMut};
pub use walker::*;
mod walker {
    use std::marker::PhantomData;

    use super::*;

    /// Struct for walking the fields of a [`SchemaPtr`], [`SchemaPtrMut`], or [`SchemaBox`] and
    /// dynamically reading/writing fields.
    pub struct SchemaWalkerMut<'pointer, 'schema, 'parent> {
        ptr: PtrMut<'pointer>,
        schema: &'schema Schema,
        parent_schema: Option<&'schema Schema>,
        parent_lifetime: PhantomData<&'parent mut ()>,
    }
    impl<'a, 'b, 'c> std::fmt::Debug for SchemaWalkerMut<'a, 'b, 'c> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("SchemaWalkerMut")
                .field("schema", &self.schema)
                .field("parent_schema", &self.parent_schema)
                .field("parent_lt", &self.parent_lifetime)
                .finish_non_exhaustive()
        }
    }

    /// The index of a field in a struct in a [`SchemaWalkerMut`].
    pub enum FieldIdx<'a> {
        Name(&'a str),
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

    impl<'pointer, 'schema, 'parent> SchemaWalkerMut<'pointer, 'schema, 'parent> {
        /// Creates a new [`SchemaWalkerMut`] for the given pointer and schema.
        ///
        /// # Safety
        ///
        /// You must guarantee that the memory layout of the pointer matches the schema.
        pub unsafe fn from_ptr_schema(ptr: PtrMut<'pointer>, schema: &'schema Schema) -> Self {
            Self {
                ptr,
                schema,
                parent_schema: None,
                parent_lifetime: PhantomData,
            }
        }

        /// Extract the mutable pointer from the walker.
        pub fn into_ptr_mut(self) -> PtrMut<'pointer> {
            self.ptr
        }

        /// Get the [`Schema`].
        pub fn schema(&self) -> &'schema Schema {
            self.schema
        }

        /// Get the [`SchemaKind`].
        pub fn kind(&self) -> &'schema SchemaKind {
            &self.schema.kind
        }

        /// Get a walker for the given field.
        pub fn get_field<'a, 'b, I: Into<FieldIdx<'b>>>(
            &'a mut self,
            idx: I,
        ) -> Result<SchemaWalkerMut<'pointer, 'schema, 'a>, SchemaMismatchError> {
            let idx = idx.into();
            match &self.schema.kind {
                SchemaKind::Struct(s) => {
                    let Some(field) = (match idx {
                        FieldIdx::Name(n) => s.fields.iter().find(|x| x.name.as_deref() == Some(n)),
                        FieldIdx::Idx(i) => s.fields.get(i),
                    }) else { return Err(SchemaMismatchError) };

                    todo!()
                }
                SchemaKind::Vec(_) => Err(SchemaMismatchError),
                SchemaKind::Primitive(_) => Err(SchemaMismatchError),
            }
        }
    }
}

/// Error type when attempting to cast between types with mis-matched schemas.
#[derive(Debug)]
pub struct SchemaMismatchError;
impl SchemaMismatchError {
    pub const MSG: &str = "Invalid cast: the schemas of the casted types are not compatible.";
}

impl std::error::Error for SchemaMismatchError {}
impl std::fmt::Display for SchemaMismatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", Self::MSG)
    }
}

/// Error type when attempting to cast between types with mis-matched schemas.
pub struct SchemaPtrMutMismatchError<'a>(SchemaPtrMut<'a>);

impl<'a> std::fmt::Debug for SchemaPtrMutMismatchError<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SchemaMismatchError").finish()
    }
}
impl<'a> std::error::Error for SchemaPtrMutMismatchError<'a> {}
impl<'a> std::fmt::Display for SchemaPtrMutMismatchError<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", SchemaMismatchError::MSG)
    }
}
