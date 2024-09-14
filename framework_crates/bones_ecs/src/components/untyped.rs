use crate::prelude::*;

use bones_schema::alloc::ResizableAlloc;
use std::{
    ffi::c_void,
    mem::MaybeUninit,
    ptr::{self},
    rc::Rc,
};

/// Holds components of a given type indexed by `Entity`.
///
/// We do not check if the given entity is alive here, this should be done using `Entities`.
pub struct UntypedComponentStore {
    pub(crate) bitset: BitSetVec,
    pub(crate) storage: ResizableAlloc,
    pub(crate) max_id: usize,
    pub(crate) schema: &'static Schema,
}

unsafe impl Sync for UntypedComponentStore {}
unsafe impl Send for UntypedComponentStore {}

impl Clone for UntypedComponentStore {
    fn clone(&self) -> Self {
        let new_storage = self.storage.clone();

        for i in 0..self.max_id {
            if self.bitset.bit_test(i) {
                // SAFE: constructing an UntypedComponent store is unsafe, and the user affirms that
                // clone_fn will not do anything unsound.
                //
                // - And our previous pointer is a valid pointer to component data
                // - And our new pointer is a writable pointer with the same layout
                unsafe {
                    let prev_ptr = self.storage.unchecked_idx(i);
                    let new_ptr = new_storage.unchecked_idx(i);
                    (self
                        .schema
                        .clone_fn
                        .as_ref()
                        .expect("Cannot clone component")
                        .get())(prev_ptr, new_ptr);
                }
            }
        }

        Self {
            bitset: self.bitset.clone(),
            storage: new_storage,
            max_id: self.max_id,
            schema: self.schema,
        }
    }
}

impl Drop for UntypedComponentStore {
    fn drop(&mut self) {
        if let Some(drop_fn) = &self.schema.drop_fn {
            for i in 0..self.storage.capacity() {
                if self.bitset.bit_test(i) {
                    // SAFE: constructing an UntypedComponent store is unsafe, and the user affirms
                    // that drop_fn will not do anything unsound.
                    //
                    // And our pointer is valid.
                    unsafe {
                        let ptr = self.storage.unchecked_idx(i);
                        drop_fn.get()(ptr);
                    }
                }
            }
        }
    }
}

impl UntypedComponentStore {
    /// Create a arbitrary [`UntypedComponentStore`].
    ///
    /// In Rust, you will usually not use [`UntypedComponentStore`] and will use the statically
    /// typed [`ComponentStore<T>`] instead.
    pub fn new(schema: &'static Schema) -> Self {
        Self {
            bitset: create_bitset(),
            storage: ResizableAlloc::new(schema.layout()),
            max_id: 0,
            schema,
        }
    }

    /// Create an [`UntypedComponentStore`] that is valid for the given type `T`.
    pub fn for_type<T: HasSchema>() -> Self {
        Self {
            bitset: create_bitset(),
            storage: ResizableAlloc::new(T::schema().layout()),
            max_id: 0,
            schema: T::schema(),
        }
    }

    /// Get the schema of the components stored.
    pub fn schema(&self) -> &'static Schema {
        self.schema
    }

    /// Insert component data for the given entity and get the previous component data if present.
    /// # Panics
    /// Panics if the schema of `T` doesn't match the store.
    #[inline]
    #[track_caller]
    pub fn insert_box(&mut self, entity: Entity, data: SchemaBox) -> Option<SchemaBox> {
        self.try_insert_box(entity, data).unwrap()
    }

    /// Insert component data for the given entity and get the previous component data if present.
    /// # Errors
    /// Errors if the schema of `T` doesn't match the store.
    pub fn try_insert_box(
        &mut self,
        entity: Entity,
        data: SchemaBox,
    ) -> Result<Option<SchemaBox>, SchemaMismatchError> {
        if self.schema != data.schema() {
            Err(SchemaMismatchError)
        } else {
            let ptr = data.as_ptr();
            // SOUND: we validated schema matches
            let already_had_component = unsafe { self.insert_raw(entity, ptr) };
            if already_had_component {
                // Previous component data will be written to data pointer
                Ok(Some(data))
            } else {
                // Don't run the data's destructor, it has been moved into the storage.
                std::mem::forget(data);
                Ok(None)
            }
        }
    }

    /// Insert component data for the given entity and get the previous component data if present.
    /// # Panics
    /// Panics if the schema of `T` doesn't match the store.
    #[inline]
    #[track_caller]
    pub fn insert<T: HasSchema>(&mut self, entity: Entity, data: T) -> Option<T> {
        self.try_insert(entity, data).unwrap()
    }

    /// Insert component data for the given entity and get the previous component data if present.
    /// # Errors
    /// Errors if the schema of `T` doesn't match the store.
    pub fn try_insert<T: HasSchema>(
        &mut self,
        entity: Entity,
        mut data: T,
    ) -> Result<Option<T>, SchemaMismatchError> {
        if self.schema != T::schema() {
            Err(SchemaMismatchError)
        } else {
            let ptr = &mut data as *mut T as *mut c_void;
            // SOUND: we validated schema matches
            let already_had_component = unsafe { self.insert_raw(entity, ptr) };
            if already_had_component {
                // Previous component data will be written to data pointer
                Ok(Some(data))
            } else {
                // Don't run the data's destructor, it has been moved into the storage.
                std::mem::forget(data);
                Ok(None)
            }
        }
    }

    /// Returns true if the entity already had a component of this type.
    ///
    /// If true is returned, the previous value of the pointer will be written to `data`.
    ///
    /// # Safety
    /// - The data must be a pointer to memory with the same schema.
    /// - If `false` is returned you must ensure the `data` pointer is not used after pushing.
    pub unsafe fn insert_raw(&mut self, entity: Entity, data: *mut c_void) -> bool {
        let index = entity.index() as usize;
        let size = self.schema.layout().size();

        // If the component already exists on the entity
        if self.bitset.bit_test(entity.index() as usize) {
            let ptr = self.storage.unchecked_idx(index);

            // Swap the data with the data already there
            ptr::swap_nonoverlapping(ptr, data, size);

            // There was already a component of this type
            true

        // If the component does not already exist for this entity.
        } else {
            // Update our maximum enitity id.
            self.max_id = self.max_id.max(index + 1);

            // Make sure we have enough memory allocated for storage.
            self.allocate_enough(index);

            // Set the bit indicating that this entity has this component data stored.
            self.bitset.bit_set(index);

            // Copy the data from the data pointer into our storage
            self.storage
                .unchecked_idx(index)
                .copy_from_nonoverlapping(data, size);

            // There was not already a component of this type
            false
        }
    }

    /// Ensures that we have the storage filled at least until the `until` variable.
    ///
    /// Usually, set this to `entity.index`.
    fn allocate_enough(&mut self, until: usize) {
        if self.storage.capacity() <= until {
            self.storage
                // TODO: Determine a better policy for resizing and pre-allocating component storage.
                // Right now we double the size of the storage every time we run out. It seems like we
                // might be able to come up with a smarter policy. On top of that we should
                // be able to create a type data for components ( see
                // `bones_framework::metadata_asset()` for example ) that lets you customize the resize
                // and also pre-allocation strategy for the component. Right now we don't pre-allocate
                // any memory, but that could be useful for components that know there will be a lot of
                // them, such as bullets.
                .resize((until + 1) * 2)
                .unwrap();
        }
    }

    /// Get a reference to the component storage for the given [`Entity`].
    /// # Panics
    /// Panics if the schema of `T` doesn't match.
    #[track_caller]
    #[inline]
    pub fn get<T: HasSchema>(&self, entity: Entity) -> Option<&T> {
        self.try_get(entity).unwrap()
    }

    /// Get a reference to the component storage for the given [`Entity`].
    /// # Errors
    /// Errors if the schema of `T` doesn't match.
    pub fn try_get<T: HasSchema>(&self, entity: Entity) -> Result<Option<&T>, SchemaMismatchError> {
        self.get_ref(entity).map(|x| x.try_cast()).transpose()
    }

    /// Get a [`SchemaRef`] to the component for the given [`Entity`] if the entity has this
    /// component.
    #[inline]
    pub fn get_ref(&self, entity: Entity) -> Option<SchemaRef> {
        let idx = entity.index() as usize;
        self.get_idx(idx)
    }

    fn get_idx(&self, idx: usize) -> Option<SchemaRef> {
        if self.bitset.bit_test(idx) {
            // SOUND: we ensure that there is allocated storge for entities that have their bit set.
            let ptr = unsafe { self.storage.unchecked_idx(idx) };
            // SOUND: we know that the pointer has our schema.
            Some(unsafe { SchemaRef::from_ptr_schema(ptr, self.schema) })
        } else {
            None
        }
    }

    /// Get a mutable reference to the component storage for the given [`Entity`].
    /// # Panics
    /// Panics if the schema of `T` doesn't match.
    #[track_caller]
    #[inline]
    pub fn get_mut<T: HasSchema>(&mut self, entity: Entity) -> Option<&mut T> {
        self.try_get_mut(entity).unwrap()
    }

    /// Get a mutable reference to the component storage for the given [`Entity`].
    /// # Errors
    /// Errors if the schema of `T` doesn't match.
    pub fn try_get_mut<T: HasSchema>(
        &mut self,
        entity: Entity,
    ) -> Result<Option<&mut T>, SchemaMismatchError> {
        self.get_ref_mut(entity)
            .map(|x| x.try_cast_into_mut())
            .transpose()
    }

    /// Get a mutable reference to component storage for the given [`Entity`]
    /// if it exists. Otherwise inserts `T` generated by calling parameter: `f`.
    #[inline]
    pub fn get_mut_or_insert<T: HasSchema>(
        &mut self,
        entity: Entity,
        f: impl FnOnce() -> T,
    ) -> &mut T {
        if !self.bitset.bit_test(entity.index() as usize) {
            self.insert(entity, f());
        }
        self.get_mut(entity).unwrap()
    }

    /// Get a [`SchemaRefMut`] to the component for the given [`Entity`]
    #[inline]
    pub fn get_ref_mut<'a>(&mut self, entity: Entity) -> Option<SchemaRefMut<'a>> {
        let idx = entity.index() as usize;
        self.get_idx_mut(idx)
    }

    fn get_idx_mut<'a>(&mut self, idx: usize) -> Option<SchemaRefMut<'a>> {
        if self.bitset.bit_test(idx) {
            // SOUND: we ensure that there is allocated storage for entities that have their bit
            // set.
            let ptr = unsafe { self.storage.unchecked_idx(idx) };
            // SOUND: we know that the pointer has our schema.
            Some(unsafe { SchemaRefMut::from_ptr_schema(ptr, self.schema) })
        } else {
            None
        }
    }

    /// Get mutable references s to the component data for multiple entities at the same time.
    ///
    /// # Panics
    ///
    /// This will panic if the same entity is specified multiple times. This is invalid because it
    /// would mean you would have two mutable references to the same component data at the same
    /// time.
    ///
    /// This will also panic if there is a schema mismatch.
    #[inline]
    #[track_caller]
    pub fn get_many_mut<const N: usize, T: HasSchema>(
        &mut self,
        entities: [Entity; N],
    ) -> [Option<&mut T>; N] {
        self.try_get_many_mut(entities).unwrap()
    }

    /// Get mutable references s to the component data for multiple entities at the same time.
    ///
    /// # Panics
    ///
    /// This will panic if the same entity is specified multiple times. This is invalid because it
    /// would mean you would have two mutable references to the same component data at the same
    /// time.
    ///
    /// # Errors
    ///
    /// This will error if there is a schema mismatch.
    pub fn try_get_many_mut<const N: usize, T: HasSchema>(
        &mut self,
        entities: [Entity; N],
    ) -> Result<[Option<&mut T>; N], SchemaMismatchError> {
        if self.schema != T::schema() {
            Err(SchemaMismatchError)
        } else {
            let mut refs = self.get_many_ref_mut(entities);
            let refs = std::array::from_fn(|i| {
                let r = refs[i].take();
                // SOUND: we've validated the schema matches.
                r.map(|r| unsafe { r.cast_into_mut_unchecked() })
            });

            Ok(refs)
        }
    }

    /// Get [`SchemaRefMut`]s to the component data for multiple entities at the same time.
    ///
    /// # Panics
    ///
    /// This will panic if the same entity is specified multiple times. This is invalid because it
    /// would mean you would have two mutable references to the same component data at the same
    /// time.
    pub fn get_many_ref_mut<const N: usize>(
        &mut self,
        entities: [Entity; N],
    ) -> [Option<SchemaRefMut>; N] {
        // Sort a copy of the passed in entities list.
        let mut sorted = entities;
        sorted.sort_unstable();
        // Detect duplicates.
        //
        // Since we have sorted the slice, any duplicates will be adjacent to each-other, and we
        // only have to make sure that for every item in the slice, the one after it is not the same
        // as it.
        for i in 0..(N - 1) {
            if sorted[i] == sorted[i + 1] {
                panic!("All entities passed to `get_multiple_mut()` must be unique.");
            }
        }

        std::array::from_fn(|i| {
            let index = entities[i].index() as usize;

            if self.bitset.bit_test(index) {
                // SOUND: we've already validated that the contents of storage is valid for type T.
                // The new lifetime is sound because we validate that all of these borrows don't
                // overlap and their lifetimes are that of the &mut self borrow.
                unsafe {
                    let ptr = self.storage.unchecked_idx(index);
                    Some(SchemaRefMut::from_ptr_schema(ptr, self.schema))
                }
            } else {
                None
            }
        })
    }

    /// Remove the component data for the entity if it exists.
    /// # Errors
    /// Errors if the schema doesn't match.
    #[inline]
    #[track_caller]
    pub fn remove<T: HasSchema>(&mut self, entity: Entity) -> Option<T> {
        self.try_remove(entity).unwrap()
    }

    /// Remove the component data for the entity if it exists.
    /// # Errors
    /// Errors if the schema doesn't match.
    pub fn try_remove<T: HasSchema>(
        &mut self,
        entity: Entity,
    ) -> Result<Option<T>, SchemaMismatchError> {
        if self.schema != T::schema() {
            Err(SchemaMismatchError)
        } else if self.bitset.contains(entity) {
            let mut data = MaybeUninit::<T>::uninit();
            // SOUND: the data doesn't overlap the storage.
            unsafe { self.remove_raw(entity, Some(data.as_mut_ptr() as *mut c_void)) };

            // SOUND: we've initialized the data.
            Ok(Some(unsafe { data.assume_init() }))
        } else {
            // SOUND: we don't use the out pointer.
            unsafe { self.remove_raw(entity, None) };
            Ok(None)
        }
    }

    /// Remove the component data for the entity if it exists.
    pub fn remove_box(&mut self, entity: Entity) -> Option<SchemaBox> {
        if self.bitset.contains(entity) {
            // SOUND: we will immediately initialize the schema box with data matching the schema.
            let b = unsafe { SchemaBox::uninitialized(self.schema) };
            // SOUND: the box data doesn't overlap the storage.
            unsafe { self.remove_raw(entity, Some(b.as_ptr())) };
            Some(b)
        } else {
            // SOUND: we don't use the out pointer.
            unsafe { self.remove_raw(entity, None) };
            None
        }
    }

    /// If there is a previous value, `true` will be returned.
    ///
    /// If `out` is set and true is returned, the previous value will be written to it.
    ///
    /// # Safety
    ///
    /// If set, the `out` pointer, must not overlap the internal component storage.
    pub unsafe fn remove_raw(&mut self, entity: Entity, out: Option<*mut c_void>) -> bool {
        let index = entity.index() as usize;
        let size = self.schema.layout().size();

        if self.bitset.bit_test(index) {
            self.bitset.bit_reset(index);

            let ptr = self.storage.unchecked_idx(index);

            if let Some(out) = out {
                // SAFE: user asserts `out` is non-overlapping
                out.copy_from_nonoverlapping(ptr, size);
            } else if let Some(drop_fn) = &self.schema.drop_fn {
                // SAFE: construcing `UntypedComponentStore` asserts the soundess of the drop_fn
                //
                // And ptr is a valid pointer to the component type.
                drop_fn.get()(ptr);
            }

            // Found previous component
            true
        } else {
            // No previous component
            false
        }
    }

    /// Get a reference to the component store if there is exactly one instance of the component.
    pub fn get_single_with_bitset(
        &self,
        bitset: Rc<BitSetVec>,
    ) -> Result<SchemaRef, QuerySingleError> {
        let len = self.bitset().bit_len();
        let mut iter = (0..len).filter(|&i| bitset.bit_test(i) && self.bitset().bit_test(i));
        let i = iter.next().ok_or(QuerySingleError::NoEntities)?;
        if iter.next().is_some() {
            return Err(QuerySingleError::MultipleEntities);
        }
        // TODO: add unchecked variant to avoid redundant validation
        self.get_idx(i).ok_or(QuerySingleError::NoEntities)
    }

    /// Get a mutable reference to the component store if there is exactly one instance of the
    /// component.
    pub fn get_single_with_bitset_mut(
        &mut self,
        bitset: Rc<BitSetVec>,
    ) -> Result<SchemaRefMut, QuerySingleError> {
        let len = self.bitset().bit_len();
        let mut iter = (0..len).filter(|&i| bitset.bit_test(i) && self.bitset().bit_test(i));
        let i = iter.next().ok_or(QuerySingleError::NoEntities)?;
        if iter.next().is_some() {
            return Err(QuerySingleError::MultipleEntities);
        }
        // TODO: add unchecked variant to avoid redundant validation
        self.get_idx_mut(i).ok_or(QuerySingleError::NoEntities)
    }

    /// Iterates immutably over all components of this type.
    ///
    /// Very fast but doesn't allow joining with other component types.
    pub fn iter(&self) -> UntypedComponentStoreIter<'_> {
        UntypedComponentStoreIter {
            store: self,
            idx: 0,
        }
    }

    /// Iterates mutably over all components of this type.
    ///
    /// Very fast but doesn't allow joining with other component types.
    pub fn iter_mut(&mut self) -> UntypedComponentStoreIterMut<'_> {
        UntypedComponentStoreIterMut {
            store: self,
            idx: 0,
        }
    }

    /// Iterates immutably over the components of this type where `bitset` indicates the indices of
    /// entities.
    ///
    /// Slower than `iter()` but allows joining between multiple component types.
    pub fn iter_with_bitset(&self, bitset: Rc<BitSetVec>) -> UntypedComponentBitsetIterator {
        UntypedComponentBitsetIterator {
            current_id: 0,
            components: self,
            bitset,
        }
    }

    /// Iterates immutably over the components of this type where `bitset` indicates the indices of
    /// entities. Iterator provides Option, returning None if there is no component for entity in bitset.
    pub fn iter_with_bitset_optional(
        &self,
        bitset: Rc<BitSetVec>,
    ) -> UntypedComponentOptionalBitsetIterator {
        UntypedComponentOptionalBitsetIterator(UntypedComponentBitsetIterator {
            current_id: 0,
            components: self,
            bitset,
        })
    }

    /// Iterates mutable over the components of this type where `bitset` indicates the indices of
    /// entities.
    ///
    /// Slower than `iter()` but allows joining between multiple component types.
    pub fn iter_mut_with_bitset(
        &mut self,
        bitset: Rc<BitSetVec>,
    ) -> UntypedComponentBitsetIteratorMut {
        UntypedComponentBitsetIteratorMut {
            current_id: 0,
            components: self,
            bitset,
        }
    }

    /// Iterates mutably over the components of this type where `bitset` indicates the indices of
    /// entities. Iterator provides Option, returning None if there is no component for entity in bitset.
    pub fn iter_mut_with_bitset_optional(
        &mut self,
        bitset: Rc<BitSetVec>,
    ) -> UntypedComponentOptionalBitsetIteratorMut {
        UntypedComponentOptionalBitsetIteratorMut(UntypedComponentBitsetIteratorMut {
            current_id: 0,
            components: self,
            bitset,
        })
    }

    /// Returns the bitset indicating which entity indices have a component associated to them.
    ///
    /// Useful to build conditions between multiple `Components`' bitsets.
    ///
    /// For example, take two bitsets from two different `Components` types. Then,
    /// bitset1.clone().bit_and(bitset2); And finally, you can use bitset1 in `iter_with_bitset` and
    /// `iter_mut_with_bitset`. This will iterate over the components of the entity only for
    /// entities that have both components.
    pub fn bitset(&self) -> &BitSetVec {
        &self.bitset
    }

    /// Convert into a typed [`ComponentStore`].
    /// # Panics
    /// Panics if the schema doesn't match.
    #[inline]
    #[track_caller]
    pub fn into_typed<T: HasSchema>(self) -> ComponentStore<T> {
        self.try_into().unwrap()
    }
}

/// Mutable iterator over pointers in an untyped component store.
pub struct UntypedComponentStoreIter<'a> {
    store: &'a UntypedComponentStore,
    idx: usize,
}
impl<'a> Iterator for UntypedComponentStoreIter<'a> {
    type Item = SchemaRef<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.idx < self.store.max_id {
                if let Some(ptr) = self.store.get_idx(self.idx) {
                    self.idx += 1;
                    break Some(ptr);
                }
                self.idx += 1;
            } else {
                break None;
            }
        }
    }
}

/// Mutable iterator over pointers in an untyped component store.
pub struct UntypedComponentStoreIterMut<'a> {
    store: &'a mut UntypedComponentStore,
    idx: usize,
}
impl<'a> Iterator for UntypedComponentStoreIterMut<'a> {
    type Item = SchemaRefMut<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.idx < self.store.max_id {
                if let Some(ptr) = self.store.get_idx_mut(self.idx) {
                    self.idx += 1;
                    // Re-create the ref to extend the lifetime.
                    // SOUND: We know the pointer will be valid for the lifetime of the store.
                    break Some(unsafe {
                        SchemaRefMut::from_ptr_schema(ptr.as_ptr(), ptr.schema())
                    });
                }
                self.idx += 1;
            } else {
                break None;
            }
        }
    }
}
