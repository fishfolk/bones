use crate::prelude::*;

use bones_schema::alloc::ResizableAlloc;
use std::{
    alloc::Layout,
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

impl Clone for UntypedComponentStore {
    fn clone(&self) -> Self {
        let size = self.schema.layout().size();
        let mut new_storage = self.storage.clone();

        for i in 0..self.max_id {
            if self.bitset.bit_test(i) {
                // SAFE: constructing an UntypedComponent store is unsafe, and the user affirms that
                // clone_fn will not do anything unsound.
                //
                // - And our previous pointer is a valid pointer to component data
                // - And our new pointer is a writable pointer with the same layout
                unsafe {
                    let prev_ptr = self.storage.ptr().byte_add(i * size);
                    let new_ptr = new_storage.ptr_mut().byte_add(i * size);
                    (self.schema.clone_fn.expect("Cannot clone component"))(
                        prev_ptr.as_ptr(),
                        new_ptr.as_ptr(),
                    );
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
        if let Some(drop_fn) = self.schema.drop_fn {
            for i in 0..self.storage.capacity() {
                if self.bitset.bit_test(i) {
                    // SAFE: constructing an UntypedComponent store is unsafe, and the user affirms
                    // that drop_fn will not do anything unsound.
                    //
                    // And our pointer is valid.
                    unsafe {
                        let ptr = self.storage.unchecked_idx_mut(i);
                        drop_fn(ptr.as_ptr());
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
    ///
    /// # Safety
    ///
    /// The `clone_fn` and `drop_fn`, if specified, must not do anything unsound, when given valid
    /// pointers to clone or drop.
    pub unsafe fn new(schema: &'static Schema) -> Self {
        let layout = schema.layout();
        Self {
            bitset: create_bitset(),
            // Approximation of a good default.
            storage: ResizableAlloc::with_capacity(layout, BITSET_SIZE >> 4).unwrap(),
            max_id: 0,
            schema,
        }
    }

    /// Create an [`UntypedComponentStore`] that is valid for the given type `T`.
    pub fn for_type<T: HasSchema>() -> Self {
        let layout = Layout::new::<T>();
        Self {
            bitset: create_bitset(),
            // Approximation of a good default.
            storage: ResizableAlloc::with_capacity(layout, BITSET_SIZE >> 4).unwrap(),
            max_id: 0,
            schema: T::schema(),
        }
    }

    /// Get the schema of the components stored.
    pub fn schema(&self) -> &'static Schema {
        self.schema
    }

    /// Returns true if the entity already had a component of this type.
    ///
    /// If true is returned, the previous value of the pointer will be written to `data`.
    ///
    /// # Safety
    ///
    /// - The data pointer must be valid for reading and writing objects with the layout that the
    /// [`UntypedComponentStore`] was created with.
    /// - The data pointer must not overlap with the [`UntypedComponentStore`]'s internal storage.
    pub unsafe fn insert(&mut self, entity: Entity, data: *mut u8) -> bool {
        let index = entity.index() as usize;
        let size = self.schema.layout().size();

        // If the component already exists on the entity
        if self.bitset.bit_test(entity.index() as usize) {
            let ptr = self.storage.unchecked_idx_mut(index);

            // Swap the data with the data already there
            ptr::swap_nonoverlapping(ptr.as_ptr(), data, size);

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
                .unchecked_idx_mut(index)
                .as_ptr()
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
                .resize(self.storage.capacity().max(1) * 2)
                .unwrap();
        }
    }

    /// Get a read-only pointer to the component for the given [`Entity`] if the entity has this
    /// component.
    pub fn get(&self, entity: Entity) -> Option<Ptr<'_>> {
        let idx = entity.index() as usize;
        self.get_idx(idx)
    }

    fn get_idx(&self, idx: usize) -> Option<Ptr<'_>> {
        if self.bitset.bit_test(idx) {
            // SOUND: we ensure that there is allocated storge for entities that have their bit set.
            Some(unsafe { self.storage.unchecked_idx(idx) })
        } else {
            None
        }
    }

    /// Get a mutable pointer to the component for the given [`Entity`]
    pub fn get_mut(&mut self, entity: Entity) -> Option<PtrMut<'_>> {
        let idx = entity.index() as usize;
        self.get_idx_mut(idx)
    }

    fn get_idx_mut(&mut self, idx: usize) -> Option<PtrMut<'_>> {
        if self.bitset.bit_test(idx) {
            // SAFE: we've already validated that the contents of storage is valid for type T.
            unsafe {
                let ptr = self.storage.unchecked_idx_mut(idx);
                Some(ptr)
            }
        } else {
            None
        }
    }

    /// Get mutable pointers to the component data for multiple entities at the same time.
    ///
    /// # Panics
    ///
    /// This will panic if the same entity is specified multiple times. This is invalid because it
    /// would mean you would have two mutable references to the same component data at the same
    /// time.
    pub fn get_many_mut<const N: usize>(
        &mut self,
        entities: [Entity; N],
    ) -> [Option<PtrMut<'_>>; N] {
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
                // SAFE: we've already validated that the contents of storage is valid for type T.
                // The transmut is safe because we validate that all of these borrows don't overlap
                // and their lifetimes are that of the &mut self borrow.
                unsafe {
                    let ptr = self.storage.unchecked_idx_mut(index).transmute_lifetime();
                    Some(ptr)
                }
            } else {
                None
            }
        })
    }

    /// If there is a previous value, `true` will be returned.
    ///
    /// If `out` is set and true is returned, the previous value will be written to it.
    ///
    /// # Safety
    ///
    /// If set, the `out` pointer, must not overlap the internal component storage.
    pub unsafe fn remove(&mut self, entity: Entity, out: Option<*mut u8>) -> bool {
        let index = entity.index() as usize;
        let size = self.schema.layout().size();

        if self.bitset.bit_test(index) {
            self.bitset.bit_reset(index);

            let ptr = self.storage.unchecked_idx_mut(index).as_ptr();

            if let Some(out) = out {
                // SAFE: user asserts `out` is non-overlapping
                out.copy_from_nonoverlapping(ptr, size);
            } else if let Some(drop_fn) = self.schema.drop_fn {
                // SAFE: construcing `UntypedComponentStore` asserts the soundess of the drop_fn
                //
                // And ptr is a valid pointer to the component type.
                drop_fn(ptr);
            }

            // Found previous component
            true
        } else {
            // No previous component
            false
        }
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
}

/// Mutable iterator over pointers in an untyped component store.
pub struct UntypedComponentStoreIter<'a> {
    store: &'a UntypedComponentStore,
    idx: usize,
}
impl<'a> Iterator for UntypedComponentStoreIter<'a> {
    type Item = Ptr<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.idx < self.store.max_id {
                if let Some(ptr) = self.store.get_idx(self.idx) {
                    self.idx += 1;
                    break Some(ptr);
                } else {
                    self.idx += 1;
                }
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
    type Item = PtrMut<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.idx < self.store.max_id {
                if let Some(ptr) = self.store.get_idx_mut(self.idx) {
                    self.idx += 1;
                    // SOUND: We know the pointer will be valid for the lifetime of the store.
                    break Some(unsafe { ptr.transmute_lifetime() });
                } else {
                    self.idx += 1;
                }
            } else {
                break None;
            }
        }
    }
}
