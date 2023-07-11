use super::*;

/// A wrapper for a pointer, that also contains it's schema.
#[derive(Clone, Debug)]
pub struct SchemaPtr {
    ptr: *const u8,
    schema: Schema,
}

impl SchemaPtr {
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
    pub fn try_cast<T: HasSchema>(&self) -> Result<*const T, SchemaMismatchError> {
        if self.schema.represents(&T::schema()) {
            // SAFE: the schemas have the same memory representation.
            Ok(self.ptr as *const T)
        } else {
            Err(SchemaMismatchError)
        }
    }

    /// Create a new [`SchemaPtr`] from a reference to a type that implements [`HasSchema`].
    pub fn new<T: HasSchema>(v: &T) -> Self {
        let ptr = v as *const T as *const u8;
        let schema = T::schema();
        Self { ptr, schema }
    }

    /// Create a new [`SchemaPtr`] from a raw pointer and it's schema.
    pub fn from_ptr_schema(ptr: *const u8, schema: Schema) -> Self {
        Self { ptr, schema }
    }

    /// Get the raw pointer.
    pub fn ptr(&self) -> *const u8 {
        self.ptr
    }

    /// Get the [`Schema`] for the pointer.
    pub fn schema(&self) -> &Schema {
        &self.schema
    }
}

/// A wrapper for a pointer, that also contains it's schema.
#[derive(Clone, Debug)]
pub struct SchemaPtrMut {
    ptr: *mut u8,
    schema: Schema,
}

impl SchemaPtrMut {
    /// Cast this pointer to a reference to a type with a matching [`Schema`].
    ///
    /// # Panics
    ///
    /// Panics if the schema of the pointer does not match that of the type you are casting to.
    #[track_caller]
    pub fn cast_mut<T: HasSchema>(&self) -> *mut T {
        self.try_cast_mut().expect(SchemaMismatchError::MSG)
    }

    /// Cast this pointer to a mutable reference to a type with a matching [`Schema`].
    ///
    /// # Errors
    ///
    /// Errors if the schema of the pointer does not match that of the type you are casting to.
    pub fn try_cast_mut<T: HasSchema>(&self) -> Result<*mut T, SchemaMismatchError> {
        if self.schema.represents(&T::schema()) {
            // SAFE: the schemas have the same memory representation.
            Ok(self.ptr as *mut T)
        } else {
            Err(SchemaMismatchError)
        }
    }

    /// Create a new [`SchemaPtr`] from a reference to a type that implements [`HasSchema`].
    pub fn new<T: HasSchema>(v: &mut T) -> Self {
        let ptr = v as *mut T as *mut u8;
        let schema = T::schema();
        Self { ptr, schema }
    }

    /// Create a new [`SchemaPtr`] from a raw pointer and it's schema.
    pub fn from_ptr_schema(ptr: *mut u8, schema: Schema) -> Self {
        Self { ptr, schema }
    }

    /// Get the raw pointer.
    pub fn ptr(&self) -> *mut u8 {
        self.ptr
    }

    /// Get the [`Schema`] for the pointer.
    pub fn schema(&self) -> &Schema {
        &self.schema
    }
}

/// A owning, type-erased [`Box`]-like container.
#[derive(Debug)]
pub struct SchemaBox {
    ptr: *mut u8,
    schema: Schema,
    layout: Layout,
    drop_fn: unsafe extern "C" fn(*mut u8),
    clone_fn: unsafe extern "C" fn(src: *const u8, dst: *mut u8),
}

impl Clone for SchemaBox {
    fn clone(&self) -> Self {
        // SAFE: the layout is not zero, or we could not create this box,
        // and the new pointer has the same layout as the source.
        let new_ptr = unsafe {
            let new_ptr = std::alloc::alloc(self.layout);
            (self.clone_fn)(self.ptr as *const u8, new_ptr);
            new_ptr
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
            (self.drop_fn)(self.ptr);
            // De-allocate the memory
            std::alloc::dealloc(self.ptr, self.layout)
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
        if self.schema.represents(&T::schema()) {
            // SAFE: the schemas have the same memory representation.
            unsafe { Ok(&*(self.ptr as *const T)) }
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
        if self.schema.represents(&T::schema()) {
            // SAFE: the schemas have the same memory representation.
            unsafe { Ok(&mut *(self.ptr as *mut T)) }
        } else {
            Err(SchemaMismatchError)
        }
    }

    /// Create a new [`SchemaBox`].
    #[track_caller]
    pub fn new<T: HasSchema + Clone>(v: T) -> Self {
        let schema = T::schema();
        let layout = std::alloc::Layout::new::<T>();
        debug_assert_eq!(
            layout,
            schema.layout(),
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
            (ptr as *mut T).write(v);
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
    /// You must insure that the pointer is valid for the given `layout`, `schem`, `drop_fn`, and
    /// `clone_fn`.
    pub unsafe fn from_raw_parts(
        ptr: *mut u8,
        schema: Schema,
        drop_fn: unsafe extern "C" fn(*mut u8),
        clone_fn: unsafe extern "C" fn(src: *const u8, dst: *mut u8),
    ) -> Self {
        Self {
            ptr,
            layout: schema.layout(),
            schema,
            drop_fn,
            clone_fn,
        }
    }

    /// Get the raw pointer.
    pub fn into_raw(s: Self) -> *mut u8 {
        s.ptr
    }

    /// Get the [`Schema`] for the pointer.
    pub fn schema(&self) -> &Schema {
        &self.schema
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
