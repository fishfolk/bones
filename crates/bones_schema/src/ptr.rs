//! Schema-aware smart pointers.

use std::{
    alloc::{handle_alloc_error, Layout},
    hash::Hash,
    marker::PhantomData,
    mem::MaybeUninit,
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
                let Some((idx, offset)) = field_offsets.iter().enumerate().find_map(|(i, (name, offset))| {
                        let matches = match idx {
                            FieldIdx::Idx(n) => n == i,
                            FieldIdx::Name(n) => name.as_deref() == Some(n),
                        };
                        if matches {
                            Some((i, *offset))
                        } else {
                            None
                        }
                    }) else { return not_found };
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
            SchemaKind::Vec(_) | SchemaKind::Primitive(_) | SchemaKind::Map { .. } => not_found,
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
    pub fn schema(&self) -> &Schema {
        self.schema
    }

    /// Get the hash of this schema box, if supported.
    pub fn hash(&self) -> Option<u64> {
        self.schema
            .hash_fn
            .map(|hash_fn| unsafe { (hash_fn)(self.ptr.as_ptr()) })
    }
}

/// An untyped mutable reference that knows the [`Schema`] of the pointee and that can be cast to a matching
/// type.
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
            ptr: PtrMut::new(NonNull::new_unchecked(ptr as *mut u8)),
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
    pub fn get_field<'this, 'idx, I: Into<FieldIdx<'idx>>>(
        &'this mut self,
        idx: I,
    ) -> Result<SchemaRefMut<'pointer, 'this>, SchemaFieldNotFoundError<'idx>> {
        let idx = idx.into();
        let not_found = Err(SchemaFieldNotFoundError { idx });
        match &self.schema.kind {
            SchemaKind::Struct(s) => {
                let field_offsets = self.schema.field_offsets();
                let Some((idx, offset)) = field_offsets.iter().enumerate().find_map(|(i, (name, offset))| {
                        let matches = match idx {
                            FieldIdx::Idx(n) => n == i,
                            FieldIdx::Name(n) => name.as_deref() == Some(n),
                        };
                        if matches {
                            Some((i, *offset))
                        } else {
                            None
                        }
                    }) else { return not_found };
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
            SchemaKind::Map { .. } | SchemaKind::Vec(_) | SchemaKind::Primitive(_) => not_found,
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
    pub fn schema(&self) -> &Schema {
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

        let new_ptr = if self.layout.size() == 0 {
            NonNull::<u8>::dangling().as_ptr()
        } else {
            // SOUND: Non-zero size for layout
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

        Self {
            ptr,
            schema,
            layout,
        }
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
    pub fn schema(&self) -> &Schema {
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
        Self {
            ptr,
            layout: schema.layout(),
            schema,
        }
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
            std::alloc::dealloc(self.ptr.as_ptr(), self.layout)
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
impl<T: HasSchema + Default> Default for SBox<T> {
    fn default() -> Self {
        Self {
            b: SchemaBox::new(T::default()),
            _phantom: Default::default(),
        }
    }
}
impl<T: HasSchema + std::fmt::Debug> std::fmt::Debug for SBox<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SBox").field("b", &self.b).finish()
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
