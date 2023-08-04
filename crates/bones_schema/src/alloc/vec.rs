use std::{any::TypeId, marker::PhantomData, mem::MaybeUninit, sync::OnceLock};

use crate::{prelude::*, raw_fns::*};

use super::ResizableAlloc;

/// A type-erased [`Vec`]-like collection that for items with the same [`Schema`].
pub struct SchemaVec {
    /// The allocation for stored items.
    buffer: ResizableAlloc,
    /// The number of items actually stored in the vec.
    len: usize,
    /// The schema of the items stored in the vec.
    schema: &'static Schema,
}

// SOUND: the SchemaVec may only contain `HasSchema` types which are required to be `Sync + Send`.
unsafe impl Sync for SchemaVec {}
unsafe impl Send for SchemaVec {}

impl SchemaVec {
    /// Initialize an empty [`SchemaVec`] for items with the given schema.
    pub fn new(schema: &'static Schema) -> Self {
        Self {
            buffer: ResizableAlloc::new(schema.layout()),
            len: 0,
            schema,
        }
    }

    /// Grow the backing buffer to fit more elements.
    fn grow(&mut self) {
        let cap = self.buffer.capacity();
        if cap == 0 {
            self.buffer.resize(1).unwrap();
        } else {
            self.buffer.resize(cap * 2).unwrap();
        }
    }

    /// Push an item unsafely to the vector.
    /// # Safety
    /// - The item be a pointer to data with the same schema.
    /// - You must ensure the `item` pointer is not used after pusing.
    unsafe fn push_raw(&mut self, item: *mut u8) {
        // Make room for more elements if necessary
        if self.len == self.buffer.capacity() {
            self.grow();
        }

        // Copy the item into the vec
        unsafe {
            self.buffer
                .unchecked_idx_mut(self.len)
                .as_ptr()
                .copy_from_nonoverlapping(item, self.buffer.layout().size());
        }

        // Extend the length. This cannot overflow because we will run out of memory before we
        // exhaust `usize`.
        self.len += 1;
    }

    /// Push an item to the vec.
    /// # Errors
    /// Errors if the schema of `T` doesn't match the vec.
    pub fn try_push<T: HasSchema>(&mut self, mut item: T) -> Result<(), SchemaMismatchError> {
        // Ensure matching schema
        if self.schema != T::schema() {
            return Err(SchemaMismatchError);
        }

        unsafe {
            self.push_raw(&mut item as *mut T as *mut u8);
            std::mem::forget(item);
        }

        Ok(())
    }

    /// Push an item to the vec.
    /// # Panics
    /// Panics if the schema of `T` doesn't match the vec.
    #[inline]
    #[track_caller]
    pub fn push<T: HasSchema>(&mut self, item: T) {
        self.try_push(item).unwrap()
    }

    /// Push the item into the end of the vector.
    pub fn try_push_box(&mut self, mut item: SchemaBox) -> Result<(), SchemaMismatchError> {
        // Ensure matching schema
        if self.schema != item.schema() {
            return Err(SchemaMismatchError);
        }

        // We validated matching schemas.
        unsafe {
            self.push_raw(item.as_mut().ptr().as_ptr());
        }

        // Don't run the item's destructor, it's the responsibility of the vec
        item.forget();

        Ok(())
    }

    /// Push the item into the end of the vector.
    #[track_caller]
    #[inline]
    pub fn push_box(&mut self, item: SchemaBox) {
        self.try_push_box(item).unwrap()
    }

    /// Pop the last item off of the end of the vector.
    pub fn pop_box(&mut self) -> Option<SchemaBox> {
        if self.len == 0 {
            None
        } else {
            // Decrement our length
            self.len -= 1;

            // SOUND: we make sure that we initialize the schema box immediately after creating it.
            unsafe {
                // Allocate memory for the box
                let mut b = SchemaBox::uninitialized(self.schema);
                // Copy the last item in our vec to the box
                b.as_mut().ptr().as_ptr().copy_from_nonoverlapping(
                    self.buffer.unchecked_idx_mut(self.len).as_ptr(),
                    self.buffer.layout().size(),
                );

                Some(b)
            }
        }
    }

    /// Pop an item off the vec.
    /// # Errors
    /// Errors if the schema of `T` doesn't match.
    pub fn try_pop<T: HasSchema>(&mut self) -> Result<Option<T>, SchemaMismatchError> {
        if self.schema != T::schema() {
            return Err(SchemaMismatchError);
        }

        if self.len == 0 {
            Ok(None)
        } else {
            // Decrement our length
            self.len -= 1;

            // Allocate space on the stack for the item
            let mut data = MaybeUninit::uninit();
            unsafe {
                // Copy the data from the vec to the stack
                (data.as_mut_ptr() as *mut u8).copy_from_nonoverlapping(
                    self.buffer.unchecked_idx_mut(self.len).as_ptr(),
                    self.buffer.layout().size(),
                )
            }

            // SOUND: we've initialized the data
            Ok(Some(unsafe { data.assume_init() }))
        }
    }

    /// Pop an item off the vec.
    /// # Panics
    /// Panics if the schema of `T` doesn't match.
    #[inline]
    #[track_caller]
    pub fn pop<T: HasSchema>(&mut self) -> Option<T> {
        self.try_pop().unwrap()
    }

    /// Get an item in the vec.
    /// # Errors
    /// Errors if the schema doesn't match.
    pub fn try_get<T: HasSchema>(&self, idx: usize) -> Result<Option<&T>, SchemaMismatchError> {
        self.get_ref(idx).map(|x| x.try_cast()).transpose()
    }

    /// Get an item in the vec.
    /// # Panics
    /// Panics if the schema doesn't match.
    #[inline]
    #[track_caller]
    pub fn get<T: HasSchema>(&self, idx: usize) -> Option<&T> {
        self.try_get(idx).unwrap()
    }

    /// Get the item with the given index.
    pub fn get_ref(&self, idx: usize) -> Option<SchemaRef<'_>> {
        if idx >= self.len {
            None
        } else {
            let ptr = unsafe { self.buffer.unchecked_idx(idx) };

            unsafe { Some(SchemaRef::from_ptr_schema(ptr.as_ptr(), self.schema)) }
        }
    }

    /// Get an item in the vec.
    /// # Errors
    /// Errors if the schema doesn't match.
    pub fn try_get_mut<T: HasSchema>(
        &mut self,
        idx: usize,
    ) -> Result<Option<&mut T>, SchemaMismatchError> {
        self.get_ref_mut(idx)
            // SOUND: We are extending the lifetime of the cast to the lifetime of our borrow of
            // `&mut self`, which is valid.
            .map(|mut x| unsafe { x.try_cast_mut().map(|x| transmute_lt(x)) })
            .transpose()
    }

    /// Get an item in the vec.
    /// # Panics
    /// Panics if the schema doesn't match.
    #[inline]
    #[track_caller]
    pub fn get_mut<T: HasSchema>(&mut self, idx: usize) -> Option<&mut T> {
        self.try_get_mut(idx).unwrap()
    }

    /// Get an item with the given index.
    pub fn get_ref_mut(&mut self, idx: usize) -> Option<SchemaRefMut<'_, '_>> {
        if idx >= self.len {
            None
        } else {
            let ptr = unsafe { self.buffer.unchecked_idx(idx) };

            unsafe { Some(SchemaRefMut::from_ptr_schema(ptr.as_ptr(), self.schema)) }
        }
    }

    /// Get the number of items in the vector.
    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns `true` if the vector has zero items in it.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get the capacity of the backing buffer.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.buffer.capacity()
    }

    /// Get the schema of items in this [`SchemaVec`].
    #[inline]
    pub fn schema(&self) -> &Schema {
        self.schema
    }

    /// Convert into a typed [`SVec`].
    /// # Panics
    /// Panics if the schema of `T` doesn't match this [`SchemaVec`]'s schema.
    #[track_caller]
    pub fn into_svec<T: HasSchema>(self) -> SVec<T> {
        self.try_into_svec().unwrap()
    }

    /// Try to convert into a typed [`SVec`].
    /// # Errors
    /// Errors if the schema of `T` doesn't match this [`SchemaVec`]'s schema.
    pub fn try_into_svec<T: HasSchema>(self) -> Result<SVec<T>, SchemaMismatchError> {
        if T::schema() == self.schema {
            Ok(SVec {
                vec: self,
                _phantom: PhantomData,
            })
        } else {
            Err(SchemaMismatchError)
        }
    }
}

impl Clone for SchemaVec {
    fn clone(&self) -> Self {
        let Some(clone_fn) = self.schema.clone_fn else {
            panic!("This type cannot be cloned");
        };
        let mut buffer_clone = ResizableAlloc::new(self.schema.layout());
        buffer_clone.resize(self.len).unwrap();

        // Clone each item in the vec
        for i in 0..self.len {
            // SOUND: we've check that the index is within bounds.
            let item = unsafe { self.buffer.unchecked_idx(i).as_ptr() as *const u8 };

            unsafe {
                (clone_fn)(item, buffer_clone.unchecked_idx_mut(i).as_ptr());
            }
        }

        SchemaVec {
            buffer: buffer_clone,
            len: self.len,
            schema: self.schema,
        }
    }
}

impl Drop for SchemaVec {
    fn drop(&mut self) {
        for _ in 0..self.len {
            drop(self.pop_box().unwrap());
        }
    }
}

/// A typed version of a [`SchemaVec`].
///
/// This type exists as an alternative to [`Vec`] that properly implements [`HasSchema`].
#[repr(transparent)]
pub struct SVec<T: HasSchema> {
    vec: SchemaVec,
    _phantom: PhantomData<T>,
}

impl<T: HasSchema> SVec<T> {
    /// Create a new, empty [`SVec`].
    pub fn new() -> Self {
        Self {
            vec: SchemaVec::new(T::schema()),
            _phantom: PhantomData,
        }
    }

    /// Push an item onto the vector.
    pub fn push(&mut self, item: T) {
        self.vec.push(item)
    }

    /// Pop an item off of the vector.
    pub fn pop(&mut self) -> Option<T> {
        self.vec.pop()
    }

    /// Get an item from the vec.
    pub fn get(&self, idx: usize) -> Option<&T> {
        self.vec.get(idx)
    }

    /// Get an item from the vec.
    pub fn get_mut(&mut self, idx: usize) -> Option<&mut T> {
        self.vec.get_mut(idx)
    }

    /// Iterate over references to the items in the vec.
    pub fn iter(&self) -> SVecIter<T> {
        SVecIter { v: self, idx: 0 }
    }

    /// Iterate over mutable references to the items in the vec.
    pub fn iter_mut(&mut self) -> SVecIterMut<T> {
        SVecIterMut { v: self, idx: 0 }
    }

    /// Get the length of the vector.
    #[inline]
    pub fn len(&self) -> usize {
        self.vec.len()
    }

    /// Returns `true` if there are no items in the vector.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.vec.is_empty()
    }

    /// Get the capacity of the vec.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.vec.capacity()
    }

    /// Convert to an untyped [`SchemaVec`].
    #[inline]
    pub fn into_schema_vec(self) -> SchemaVec {
        self.vec
    }
}

impl<T: HasSchema> std::ops::Index<usize> for SVec<T> {
    type Output = T;

    #[track_caller]
    fn index(&self, idx: usize) -> &Self::Output {
        self.get(idx).unwrap()
    }
}

impl<T: HasSchema> std::ops::IndexMut<usize> for SVec<T> {
    #[track_caller]
    fn index_mut(&mut self, idx: usize) -> &mut Self::Output {
        self.get_mut(idx).unwrap()
    }
}

unsafe impl<T: HasSchema> HasSchema for SVec<T> {
    fn schema() -> &'static Schema {
        static S: OnceLock<&'static Schema> = OnceLock::new();
        S.get_or_init(|| {
            SCHEMA_REGISTRY.register(SchemaData {
                kind: SchemaKind::Vec(T::schema()),
                type_id: Some(TypeId::of::<Self>()),
                clone_fn: Some(<Self as RawClone>::raw_clone),
                drop_fn: Some(<Self as RawDrop>::raw_drop),
                default_fn: Some(<Self as RawDefault>::raw_default),
                type_data: Default::default(),
            })
        })
    }
}

impl<T: HasSchema> Default for SVec<T> {
    fn default() -> Self {
        Self {
            vec: SchemaVec::new(T::schema()),
            _phantom: Default::default(),
        }
    }
}

impl<T: HasSchema> Clone for SVec<T> {
    fn clone(&self) -> Self {
        Self {
            vec: self.vec.clone(),
            _phantom: self._phantom,
        }
    }
}

impl<'a, T: HasSchema> IntoIterator for &'a SVec<T> {
    type Item = &'a T;
    type IntoIter = SVecIter<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
impl<'a, T: HasSchema> IntoIterator for &'a mut SVec<T> {
    type Item = &'a mut T;
    type IntoIter = SVecIterMut<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

/// Iterator over items in an [`SVec`].
pub struct SVecIter<'a, T: HasSchema> {
    v: &'a SVec<T>,
    idx: usize,
}
impl<'a, T: HasSchema> Iterator for SVecIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.idx >= self.v.len() {
            None
        } else {
            let r = &self.v[self.idx];
            self.idx += 1;
            Some(r)
        }
    }
}

/// Iterator over items in an [`SVec`].
pub struct SVecIterMut<'a, T: HasSchema> {
    v: &'a mut SVec<T>,
    idx: usize,
}
impl<'a, T: HasSchema> Iterator for SVecIterMut<'a, T> {
    type Item = &'a mut T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.idx >= self.v.len() {
            None
        } else {
            let r = &mut self.v[self.idx];
            self.idx += 1;
            // SOUND: we know that the data is valid for the borrow of the SVecIterMut, and the
            // iterator will never return two references to the same item.
            Some(unsafe { transmute_lt(r) })
        }
    }
}

/// Helper to transmute a lifetime unsafely.
unsafe fn transmute_lt<'b, T>(v: &mut T) -> &'b mut T {
    std::mem::transmute(v)
}
