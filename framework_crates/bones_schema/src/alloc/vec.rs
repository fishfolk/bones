use std::{
    any::{type_name, TypeId},
    ffi::c_void,
    fmt::Debug,
    iter::Iterator,
    marker::PhantomData,
    mem::MaybeUninit,
    sync::OnceLock,
};

use bones_utils::{default, fxhash::FxHasher, parking_lot::RwLock, HashMap};

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

impl std::fmt::Debug for SchemaVec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SchemaVec")
            .field("buffer", &"ResizableAlloc")
            .field("len", &self.len)
            .field("schema", &self.schema)
            .finish()
    }
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
    /// - The item must be a pointer to data with the same schema.
    /// - You must ensure the `item` pointer is not used after pusing.
    unsafe fn push_raw(&mut self, item: *mut c_void) {
        // Make room for more elements if necessary
        if self.len == self.buffer.capacity() {
            self.grow();
        }

        // Copy the item into the vec
        unsafe {
            self.buffer
                .unchecked_idx(self.len)
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
            self.push_raw(&mut item as *mut T as *mut c_void);
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
            self.push_raw(item.as_mut().as_ptr());
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
            unsafe { self.raw_pop() }.map(|ptr| unsafe {
                let mut b = SchemaBox::uninitialized(self.schema);
                b.as_mut()
                    .as_ptr()
                    .copy_from_nonoverlapping(ptr, self.buffer.layout().size());
                b
            })
        }
    }

    /// Pop an item off the vec.
    /// # Errors
    /// Errors if the schema of `T` doesn't match.
    pub fn try_pop<T: HasSchema>(&mut self) -> Result<Option<T>, SchemaMismatchError> {
        if self.schema != T::schema() {
            Err(SchemaMismatchError)
        } else {
            let ret = unsafe { self.raw_pop() }.map(|ptr| {
                let mut data = MaybeUninit::<T>::uninit();
                unsafe {
                    (data.as_mut_ptr() as *mut c_void)
                        .copy_from_nonoverlapping(ptr, self.buffer.layout().size());
                    data.assume_init()
                }
            });
            Ok(ret)
        }
    }

    /// # Safety
    /// The pointer may only be used immediately after calling raw_pop to read the data out of the
    /// popped item. Any further mutations to the vector may make the pointer invalid.
    unsafe fn raw_pop(&mut self) -> Option<*mut c_void> {
        if self.len == 0 {
            None
        } else {
            // Decrement our length
            self.len -= 1;

            // Return the pointer to the item that is being popped off.
            Some(unsafe { self.buffer.unchecked_idx(self.len) })
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
            unsafe { Some(SchemaRef::from_ptr_schema(ptr, self.schema)) }
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
            .map(|mut x| unsafe { x.try_cast_mut().map(|x| transmute_lifetime(x)) })
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
    pub fn get_ref_mut(&mut self, idx: usize) -> Option<SchemaRefMut<'_>> {
        if idx >= self.len {
            None
        } else {
            let ptr = unsafe { self.buffer.unchecked_idx(idx) };
            unsafe { Some(SchemaRefMut::from_ptr_schema(ptr, self.schema)) }
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
    pub fn schema(&self) -> &'static Schema {
        self.schema
    }

    /// Iterate over values in the vec
    pub fn iter(&self) -> SchemaVecIter {
        SchemaVecIter { vec: self, idx: 0 }
    }

    /// Iterate mutably over values in the vec
    pub fn iter_mut(&mut self) -> SchemaVecIterMut {
        SchemaVecIterMut { vec: self, idx: 0 }
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

    /// Get the hash of this [`SchemaVec`].
    /// # Panics
    /// Panics if the inner type doesn't implement hash.
    #[track_caller]
    pub fn hash(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let Some(hash_fn) = &self.schema.hash_fn else {
            panic!("Schema doesn't specify a hash_fn");
        };
        let mut hasher = FxHasher::default();
        for item_ptr in self.buffer.iter() {
            let item_hash = unsafe { (hash_fn.get())(item_ptr) };
            item_hash.hash(&mut hasher);
        }
        hasher.finish()
    }

    /// Raw version of the [`hash()`][Self::hash] function. Not meant for normal use.
    /// # Safety
    /// Pointer must be a valid pointer to a [`SchemaVec`].
    pub unsafe fn raw_hash(ptr: *const c_void) -> u64 {
        let this = unsafe { &*(ptr as *const Self) };
        this.hash()
    }

    /// Raw version of the [`eq()`][PartialEq::eq] function. Not meant for normal use.
    /// # Safety
    /// Pointers must be valid pointers to [`SchemaVec`]s.
    pub unsafe fn raw_eq(a: *const c_void, b: *const c_void) -> bool {
        let a = &*(a as *const Self);
        let b = &*(b as *const Self);
        a.eq(b)
    }

    /// Remove and return the element at position `index` within the vector,
    /// shifting all elements after it to the left.
    /// # Panics
    /// Panics if `index` is out of bounds.
    pub fn remove(&mut self, index: usize) -> SchemaBox {
        if index >= self.len {
            panic!("index out of bounds");
        }
        let item = unsafe {
            let ptr = self.buffer.unchecked_idx(index);
            let mut boxed = SchemaBox::uninitialized(self.schema);
            boxed
                .as_mut()
                .as_ptr()
                .copy_from_nonoverlapping(ptr, self.schema.layout().size());

            // Shift elements
            let to_move = self.len - index - 1;
            if to_move > 0 {
                std::ptr::copy(
                    self.buffer.unchecked_idx(index + 1),
                    self.buffer.unchecked_idx(index),
                    to_move * self.schema.layout().size(),
                );
            }

            self.len -= 1;
            boxed
        };
        item
    }

    /// Clears the vector, removing all values.
    pub fn clear(&mut self) {
        while self.pop_box().is_some() {}
    }

    /// Shortens the vector, keeping the first `len` elements and dropping the rest.
    ///
    /// If `len` is greater than the vector's current length, this has no effect.
    pub fn truncate(&mut self, len: usize) {
        while self.len > len {
            self.pop_box();
        }
    }
}

impl<T: HasSchema> FromIterator<T> for SVec<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut this = Self::default();
        for item in iter {
            this.push(item);
        }
        this
    }
}

impl<'a> IntoIterator for &'a SchemaVec {
    type Item = SchemaRef<'a>;
    type IntoIter = SchemaVecIter<'a>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
impl<'a> IntoIterator for &'a mut SchemaVec {
    type Item = SchemaRefMut<'a>;
    type IntoIter = SchemaVecIterMut<'a>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

/// Iterator over [`SchemaVec`].
pub struct SchemaVecIter<'a> {
    vec: &'a SchemaVec,
    idx: usize,
}

impl<'a> Iterator for SchemaVecIter<'a> {
    type Item = SchemaRef<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        let item = self.vec.get_ref(self.idx);
        if item.is_some() {
            self.idx += 1;
        }
        item
    }
}

/// Mutable iterator over [`SchemaVec`].
pub struct SchemaVecIterMut<'a> {
    vec: &'a mut SchemaVec,
    idx: usize,
}
impl<'a> Iterator for SchemaVecIterMut<'a> {
    type Item = SchemaRefMut<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let item = self
            .vec
            .get_ref_mut(self.idx)
            // SOUND: We are returning data with the lifetime of the SchemaVec, which is accurate
            // and sound as long as we don't return two mutable references to the same item.
            .map(|x| unsafe { SchemaRefMut::from_ptr_schema(x.as_ptr(), x.schema()) });
        if item.is_some() {
            self.idx += 1;
        }
        item
    }
}

impl Eq for SchemaVec {}
impl PartialEq for SchemaVec {
    #[track_caller]
    fn eq(&self, other: &Self) -> bool {
        if self.schema != other.schema {
            panic!("Cannot compare two `SchemaVec`s with different schemas.");
        }
        let Some(eq_fn) = &self.schema.eq_fn else {
            panic!("Schema doesn't have an eq_fn");
        };

        for i in 0..self.len {
            unsafe {
                let a = self.buffer.unchecked_idx(i);
                let b = self.buffer.unchecked_idx(i);
                if !(eq_fn.get())(a, b) {
                    return false;
                }
            }
        }
        true
    }
}

impl Clone for SchemaVec {
    fn clone(&self) -> Self {
        let Some(clone_fn) = &self.schema.clone_fn else {
            panic!("This type cannot be cloned");
        };
        let mut buffer_clone = ResizableAlloc::new(self.schema.layout());
        buffer_clone.resize(self.len).unwrap();

        // Clone each item in the vec
        for i in 0..self.len {
            // SOUND: we've check that the index is within bounds, and the schema asserts the
            // validity of the clone function.
            unsafe {
                let item = self.buffer.unchecked_idx(i);
                (clone_fn.get())(item, buffer_clone.unchecked_idx(i));
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
///
/// Additionally, accessing an [`SVec`] is more efficient than using a [`SchemaVec`] because it
/// avoids runtime schema checks after construction.
#[repr(transparent)]
#[derive(Eq, PartialEq)]
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
    pub fn push(&mut self, mut item: T) {
        // SOUND: We know that the schema matches, and we forget the item after pushing.
        unsafe {
            self.vec.push_raw(&mut item as *mut T as *mut c_void);
        }
        std::mem::forget(item);
    }

    /// Pop an item off of the vector.
    pub fn pop(&mut self) -> Option<T> {
        unsafe {
            self.vec.raw_pop().map(|ptr| {
                let mut ret = MaybeUninit::<T>::uninit();
                ret.as_mut_ptr().copy_from_nonoverlapping(ptr as *mut T, 1);
                ret.assume_init()
            })
        }
    }

    /// Get an item from the vec.
    pub fn get(&self, idx: usize) -> Option<&T> {
        // SOUND: We know that the pointer is to a type T
        self.vec
            .get_ref(idx)
            .map(|x| unsafe { x.cast_into_unchecked() })
    }

    /// Get an item from the vec.
    pub fn get_mut(&mut self, idx: usize) -> Option<&mut T> {
        // SOUND: We know that the pointer is to a type T
        self.vec
            .get_ref_mut(idx)
            .map(|x| unsafe { x.cast_into_mut_unchecked() })
    }

    /// Iterate over references to the items in the vec.
    pub fn iter(&self) -> SVecIter<T> {
        SVecIter {
            vec: self,
            idx: 0,
            end: self.len() as isize - 1,
        }
    }

    /// Iterate over mutable references to the items in the vec.
    pub fn iter_mut(&mut self) -> SVecIterMut<T> {
        SVecIterMut {
            idx: 0,
            end: self.len() as isize - 1,
            vec: self,
        }
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

    /// Get the hash of the [`SVec`].
    pub fn hash(&self) -> u64 {
        self.vec.hash()
    }

    /// Remove and return the element at position `index` within the vector,
    /// shifting all elements after it to the left.
    /// # Panics
    /// Panics if `index` is out of bounds.
    pub fn remove(&mut self, index: usize) -> T {
        let boxed = self.vec.remove(index);
        // SAFETY: We know that the SchemaBox contains a value of type T
        unsafe { boxed.cast_into_unchecked() }
    }

    /// Clears the vector, removing all values.
    pub fn clear(&mut self) {
        self.vec.clear();
    }

    /// Shortens the vector, keeping the first `len` elements and dropping the rest.
    ///
    /// If `len` is greater than the vector's current length, this has no effect.
    pub fn truncate(&mut self, len: usize) {
        self.vec.truncate(len);
    }

    /// Extends the vector with the contents of an iterator.
    pub fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        for item in iter {
            self.push(item);
        }
    }

    /// Retains only the elements specified by the predicate.
    pub fn retain<F>(&mut self, mut f: F)
    where
        F: FnMut(&T) -> bool,
    {
        let mut i = 0;
        while i < self.len() {
            if !f(&self[i]) {
                self.remove(i);
            } else {
                i += 1;
            }
        }
    }

    /// Retains only the elements specified by the predicate, passing a mutable reference to it.
    pub fn retain_mut<F>(&mut self, mut f: F)
    where
        F: FnMut(&mut T) -> bool,
    {
        let mut i = 0;
        while i < self.len() {
            if !f(self.get_mut(i).unwrap()) {
                self.remove(i);
            } else {
                i += 1;
            }
        }
    }

    /// Removes and returns the last element of the vector if the predicate returns true.
    pub fn pop_if<F>(&mut self, f: F) -> Option<T>
    where
        F: FnOnce(&T) -> bool,
    {
        if let Some(last) = self.last() {
            if f(last) {
                self.pop()
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Returns a reference to the first element of the vector, or None if it is empty.
    pub fn first(&self) -> Option<&T> {
        self.get(0)
    }

    /// Returns a mutable reference to the first element of the vector, or None if it is empty.
    pub fn first_mut(&mut self) -> Option<&mut T> {
        self.get_mut(0)
    }

    /// Returns a reference to the last element of the vector, or None if it is empty.
    pub fn last(&self) -> Option<&T> {
        self.get(self.len().wrapping_sub(1))
    }

    /// Returns a mutable reference to the last element of the vector, or None if it is empty.
    pub fn last_mut(&mut self) -> Option<&mut T> {
        let len = self.len();
        self.get_mut(len.wrapping_sub(1))
    }
}

/// Iterator over [`SVec`].
pub struct SVecIntoIter<T: HasSchema> {
    svec: SVec<T>,
    index: usize,
}

impl<T: HasSchema + Debug> std::fmt::Debug for SVec<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut l = f.debug_list();
        for item in self.iter() {
            l.entry(item);
        }
        l.finish()
    }
}

impl<T: HasSchema> From<SVec<T>> for Vec<T> {
    fn from(svec: SVec<T>) -> Self {
        let mut vec = Vec::with_capacity(svec.len());
        for item in svec {
            vec.push(item);
        }
        vec
    }
}

impl<T: HasSchema> From<Vec<T>> for SVec<T> {
    fn from(vec: Vec<T>) -> Self {
        let mut svec = SVec::new();
        for item in vec {
            svec.push(item);
        }
        svec
    }
}

// Implement IntoIterator for SVec<T>
impl<T: HasSchema> IntoIterator for SVec<T> {
    type Item = T;
    type IntoIter = SVecIntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        SVecIntoIter {
            svec: self,
            index: 0,
        }
    }
}

impl<T: HasSchema> Iterator for SVecIntoIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.svec.len() {
            let item = self.svec.remove(self.index);
            Some(item)
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.svec.len() - self.index;
        (remaining, Some(remaining))
    }
}

impl<T: HasSchema> Drop for SVecIntoIter<T> {
    fn drop(&mut self) {
        // Ensure all remaining elements are properly dropped
        for _ in self.by_ref() {}
    }
}

impl<T: HasSchema, const N: usize> From<[T; N]> for SVec<T> {
    fn from(arr: [T; N]) -> Self {
        arr.into_iter().collect()
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

impl<T: HasSchema> std::ops::Deref for SVec<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        // SOUND: we know that the schema matches T, and the internal buffer of a SchemaVec stores
        // the types contiguously in memory.
        unsafe { std::slice::from_raw_parts(self.vec.buffer.as_ptr() as *const T, self.len()) }
    }
}
impl<T: HasSchema> std::ops::DerefMut for SVec<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // SOUND: we know that the schema matches T, and the internal buffer of a SchemaVec stores
        // the types contiguously in memory.
        unsafe { std::slice::from_raw_parts_mut(self.vec.buffer.as_ptr() as *mut T, self.len()) }
    }
}

unsafe impl<T: HasSchema> HasSchema for SVec<T> {
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
                kind: SchemaKind::Vec(T::schema()),
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
    vec: &'a SVec<T>,
    idx: usize,
    end: isize,
}
impl<'a, T: HasSchema> Iterator for SVecIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.end < 0 {
            return None;
        }
        let item = (self.idx <= self.end as usize).then(|| self.vec.get(self.idx).unwrap());
        if item.is_some() {
            self.idx += 1;
        }
        item
    }
}
impl<'a, T: HasSchema> DoubleEndedIterator for SVecIter<'a, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.end < 0 {
            return None;
        }
        let item =
            (self.end as usize >= self.idx).then(|| self.vec.get(self.end as usize).unwrap());
        if item.is_some() {
            self.end -= 1;
        }
        item
    }
}

/// Iterator over items in an [`SVec`].
pub struct SVecIterMut<'a, T: HasSchema> {
    vec: &'a mut SVec<T>,
    idx: usize,
    end: isize,
}
impl<'a, T: HasSchema> Iterator for SVecIterMut<'a, T> {
    type Item = &'a mut T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.end < 0 {
            return None;
        }
        let item = (self.idx <= self.end as usize).then(|| self.vec.get_mut(self.idx).unwrap());
        if item.is_some() {
            self.idx += 1;
        }
        // SOUND: we are returning data with the lifetime of the vec which is valid and sound,
        // assuming we don't return two mutable references to the same item.
        item.map(|x| unsafe { transmute_lifetime(x) })
    }
}
impl<'a, T: HasSchema> DoubleEndedIterator for SVecIterMut<'a, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.end < 0 {
            return None;
        }
        let item =
            (self.end as usize >= self.idx).then(|| self.vec.get_mut(self.end as usize).unwrap());
        if item.is_some() {
            self.end -= 1;
        }
        // SOUND: we are returning data with the lifetime of the vec which is valid and sound,
        // assuming we don't return two mutable references to the same item.
        item.map(|x| unsafe { transmute_lifetime(x) })
    }
}

/// Helper to transmute a lifetime unsafely.
///
/// This is safer than just calling [`transmute`][std::mem::transmute] because it can only transmut
/// the lifetime, not the type of the reference.
unsafe fn transmute_lifetime<'b, T>(v: &mut T) -> &'b mut T {
    std::mem::transmute(v)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn double_ended() {
        let mut v = [1, 2, 3, 4, 5, 6].into_iter().collect::<SVec<_>>();

        let mut iter = v.iter();
        assert_eq!(iter.next_back(), Some(&6));
        assert_eq!(iter.next_back(), Some(&5));
        assert_eq!(iter.next(), Some(&1));
        assert_eq!(iter.next(), Some(&2));
        assert_eq!(iter.next_back(), Some(&4));
        assert_eq!(iter.next(), Some(&3));
        assert_eq!(iter.next_back(), None);
        assert_eq!(iter.next(), None);

        let mut iter = v.iter_mut();
        assert_eq!(iter.next_back(), Some(&mut 6));
        assert_eq!(iter.next_back(), Some(&mut 5));
        assert_eq!(iter.next(), Some(&mut 1));
        assert_eq!(iter.next(), Some(&mut 2));
        assert_eq!(iter.next_back(), Some(&mut 4));
        assert_eq!(iter.next(), Some(&mut 3));
        assert_eq!(iter.next_back(), None);
        assert_eq!(iter.next(), None);

        let v = [].into_iter().collect::<SVec<u8>>();
        let mut iter = v.iter();
        assert_eq!(iter.next(), None);
        assert_eq!(iter.next(), None);
        assert_eq!(iter.next_back(), None);
        let mut iter = v.iter();
        assert_eq!(iter.next_back(), None);
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_vec_and_svec_conversions() {
        // Test Vec to SVec conversion
        let vec = vec![1, 2, 3, 4, 5];
        let svec: SVec<i32> = vec.clone().into();
        assert_eq!(svec.len(), 5);
        // Test SVec to Vec conversion
        let vec_from_svec: Vec<i32> = svec.into();
        assert_eq!(vec, vec_from_svec);
        // Test direct array conversion to SVec
        let svec_direct: SVec<i32> = [11, 12, 13].into();
        assert_eq!(svec_direct.len(), 3);
        // Test SVec to Vec conversion for array-created SVec
        let vec_from_array_svec: Vec<i32> = svec_direct.into();
        assert_eq!(vec_from_array_svec, vec![11, 12, 13]);
    }

    #[test]
    fn test_remove() {
        let mut svec: SVec<i32> = vec![10, 20, 30, 40, 50].into();

        // Remove from the middle
        let removed = svec.remove(2);
        assert_eq!(removed, 30);
        assert_eq!(svec.len(), 4);
        assert_eq!(svec[0], 10);

        // Remove from the beginning
        let removed = svec.remove(0);
        assert_eq!(removed, 10);
        assert_eq!(svec.len(), 3);
        assert_eq!(svec[0], 20);

        // Remove from the end
        let removed = svec.remove(2);
        assert_eq!(removed, 50);
        assert_eq!(svec.len(), 2);
        assert_eq!(svec[1], 40);

        // Test removing the last element
        let removed = svec.remove(1);
        assert_eq!(removed, 40);
        assert_eq!(svec.len(), 1);
        assert_eq!(svec[0], 20);

        // Test removing the very last element
        let removed = svec.remove(0);
        assert_eq!(removed, 20);
        assert_eq!(svec.len(), 0);
    }

    #[test]
    fn test_svec_operations() {
        let mut vec: SVec<i32> = SVec::new();

        // Test push and len
        vec.push(1);
        vec.push(2);
        vec.push(3);
        assert_eq!(vec.len(), 3);

        // Test get
        assert_eq!(vec.get(1), Some(&2));
        assert_eq!(vec.get(3), None);

        // Test remove
        let removed = vec.remove(2);
        assert_eq!(removed, 3);
        assert_eq!(vec.len(), 2);

        // Test iteration
        let sum: i32 = vec.iter().copied().sum();
        assert_eq!(sum, 3); // 1 + 2

        // Test extend
        vec.extend(vec![5, 6]);
        assert_eq!(vec.len(), 4);
        assert_eq!(vec.get(2), Some(&5));
        assert_eq!(vec.get(3), Some(&6));

        // Test retain
        vec.retain(|&x| x % 2 == 0);
        assert_eq!(vec.len(), 2);
        assert_eq!(vec[0], 2);
        assert_eq!(vec[1], 6);

        // Test retain_mut
        vec.retain_mut(|x| {
            *x *= 2;
            true
        });
        assert_eq!(vec.len(), 2);
        assert_eq!(vec[0], 4);
        assert_eq!(vec[1], 12);

        // Test truncate
        vec.truncate(1);
        assert_eq!(vec.len(), 1);
        assert_eq!(vec[0], 4);
        assert_eq!(vec.get(1), None);

        // Prepare for further tests
        vec.extend(vec![7, 9, 11]);

        // Test first() and first_mut()
        assert_eq!(vec.first(), Some(&4));
        if let Some(first) = vec.first_mut() {
            *first = 1;
        }
        assert_eq!(vec[0], 1);

        // Test last() and last_mut()
        assert_eq!(vec.last(), Some(&11));
        if let Some(last) = vec.last_mut() {
            *last = 15;
        }
        assert_eq!(vec[3], 15);

        // Test pop_if()
        assert_eq!(vec.pop_if(|&x| x > 10), Some(15));
        assert_eq!(vec.len(), 3);
        assert_eq!(vec.pop_if(|&x| x < 0), None);
        assert_eq!(vec.len(), 3);

        // Test clear
        vec.clear();
        assert_eq!(vec.len(), 0);
        assert!(vec.is_empty());

        // Test pop on empty vector
        assert_eq!(vec.pop(), None);

        // Test Vec and SVec conversions
        let original_vec = vec![1, 2, 3, 4, 5];
        let svec: SVec<i32> = original_vec.clone().into();
        let vec_from_svec: Vec<i32> = svec.into();
        assert_eq!(original_vec, vec_from_svec);
    }

    #[test]
    fn miri_error_001() {
        let mut vec: SVec<i32> = SVec::new();
        vec.push(10);
        vec.pop();
    }
}
