use std::rc::Rc;

use crate::prelude::*;

/// Read-only iterator over components matching a given bitset
pub type ComponentBitsetIterator<'a, T> =
    std::iter::Map<UntypedComponentBitsetIterator<'a>, for<'b> fn(SchemaRef<'b>) -> &'b T>;

/// Read-only iterator over components matching a given bitset.
/// Returns None for entities matching bitset but not in this ComponentStore.
pub type ComponentBitsetOptionalIterator<'a, T> = std::iter::Map<
    UntypedComponentOptionalBitsetIterator<'a>,
    for<'b> fn(Option<SchemaRef<'b>>) -> Option<&'b T>,
>;

/// Mutable iterator over components matching a given bitset
pub type ComponentBitsetIteratorMut<'a, T> = std::iter::Map<
    UntypedComponentBitsetIteratorMut<'a>,
    for<'b> fn(SchemaRefMut<'b>) -> &'b mut T,
>;

/// Mutable iterator over components matching a given bitset.
/// Returns None for entities matching bitset but not in this ComponentStore.
pub type ComponentBitsetOptionalIteratorMut<'a, T> = std::iter::Map<
    UntypedComponentOptionalBitsetIteratorMut<'a>,
    for<'b> fn(Option<SchemaRefMut<'b>>) -> Option<&'b mut T>,
>;

/// Iterates over components using a provided bitset. Each time the bitset has a 1 in index i, the
/// iterator will fetch data from the storage at index i and return it.
pub struct UntypedComponentBitsetIterator<'a> {
    pub(crate) current_id: usize,
    pub(crate) components: &'a UntypedComponentStore,
    pub(crate) bitset: Rc<BitSetVec>,
}

impl<'a> Iterator for UntypedComponentBitsetIterator<'a> {
    type Item = SchemaRef<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        let max_id = self.components.max_id;
        while self.current_id < max_id
            && !(self.bitset.bit_test(self.current_id)
                && self.components.bitset.bit_test(self.current_id))
        {
            self.current_id += 1;
        }
        let ret = if self.current_id < max_id {
            // SAFE: Here we are just getting a pointer, not doing anything unsafe with it.
            Some(unsafe {
                SchemaRef::from_ptr_schema(
                    self.components.storage.unchecked_idx(self.current_id),
                    self.components.schema,
                )
            })
        } else {
            None
        };
        self.current_id += 1;
        ret
    }
}

/// Iterate over component store returning `Option<SchemaRef<'a>>`,
/// filtered by bitset of iterator, but not bitset of own ComponentStore. Returns None on
/// bitset entries that do not have this Component.
pub struct UntypedComponentOptionalBitsetIterator<'a> {
    /// The component bitset iterator.
    pub inner: UntypedComponentBitsetIterator<'a>,
    /// The number of components (enabled bits in the `inner` iterator).
    pub components_count: usize,
    /// The number of enabled bits in the input bitset.
    pub query_count: usize,
    /// The number of enabled bits discovered in the query bitset. Iteration is complete when this
    /// reaches `query_count`.
    pub found: usize,
}

impl<'a> Iterator for UntypedComponentOptionalBitsetIterator<'a> {
    type Item = Option<SchemaRef<'a>>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.found >= self.query_count {
            // All enabled bits from the query bitset have been discovered. The rest is just zeros,
            // we are done iterating.
            return None;
        }

        if self.components_count == 0 {
            // The component store is empty, no need to test all of the bits.
            self.found += 1;
            return Some(None);
        }

        // We stop iterating at bitset length, not component store length, as we want to iterate over
        // whole bitset and return None for entities that don't have this optional component.
        let max_id = self.inner.bitset.bit_len();
        while self.inner.current_id < max_id && !self.inner.bitset.bit_test(self.inner.current_id) {
            self.inner.current_id += 1;
        }

        let ret = if self.inner.components.bitset.bit_test(self.inner.current_id) {
            self.found += 1;
            // SAFE: Here we are just getting a pointer, not doing anything unsafe with it.
            Some(Some(unsafe {
                SchemaRef::from_ptr_schema(
                    self.inner
                        .components
                        .storage
                        .unchecked_idx(self.inner.current_id),
                    self.inner.components.schema,
                )
            }))
        } else {
            // Component at current_id is not in store, however we are still iterating,
            // later ids in self.bitset may have components in store.
            self.found += 1;
            Some(None)
        };

        self.inner.current_id += 1;

        ret
    }
}

/// Iterate mutably over component store returning `Option<SchemaRef<'a>>`,
/// filtered by bitset of iterator, but not bitset of own ComponentStore. Returns None on
/// bitset entries that do not have this Component.
pub struct UntypedComponentOptionalBitsetIteratorMut<'a> {
    /// The component bitset iterator.
    pub inner: UntypedComponentBitsetIteratorMut<'a>,
    /// The number of components (enabled bits in the `inner` iterator).
    pub components_count: usize,
    /// The number of enabled bits in the input bitset.
    pub query_count: usize,
    /// The number of enabled bits discovered in the query bitset. Iteration is complete when this
    /// reaches `query_count`.
    pub found: usize,
}

impl<'a> Iterator for UntypedComponentOptionalBitsetIteratorMut<'a> {
    type Item = Option<SchemaRefMut<'a>>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.found >= self.query_count {
            // All enabled bits from the query bitset have been discovered. The rest is just zeros,
            // we are done iterating.
            return None;
        }

        if self.components_count == 0 {
            // The component store is empty, no need to test all of the bits.
            self.found += 1;
            return Some(None);
        }

        // We do not stop iterating at component store length, as we want to iterate over
        // whole bitset and return None for entities that don't have this optional component.
        let max_id = self.inner.bitset.bit_len();
        while self.inner.current_id < max_id && !self.inner.bitset.bit_test(self.inner.current_id) {
            self.inner.current_id += 1;
        }

        let ret = if self.inner.components.bitset.bit_test(self.inner.current_id) {
            self.found += 1;
            // SAFE: Here we are just getting a pointer, not doing anything unsafe with it.
            Some(Some(unsafe {
                SchemaRefMut::from_ptr_schema(
                    self.inner
                        .components
                        .storage
                        .unchecked_idx(self.inner.current_id),
                    self.inner.components.schema,
                )
            }))
        } else {
            // Component at current_id is not in store, however we are still iterating,
            // later ids in self.bitset may have components in store.
            self.found += 1;
            Some(None)
        };

        self.inner.current_id += 1;

        ret
    }
}

/// Iterates over components using a provided bitset. Each time the bitset has a 1 in index i, the
/// iterator will fetch data from the storage at index i.
pub struct UntypedComponentBitsetIteratorMut<'a> {
    pub(crate) current_id: usize,
    pub(crate) components: &'a mut UntypedComponentStore,
    pub(crate) bitset: Rc<BitSetVec>,
}

impl<'a> Iterator for UntypedComponentBitsetIteratorMut<'a> {
    type Item = SchemaRefMut<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        let max_id = self.components.max_id;
        while self.current_id < max_id
            && !(self.bitset.bit_test(self.current_id)
                && self.components.bitset.bit_test(self.current_id))
        {
            self.current_id += 1;
        }
        let ret = if self.current_id < max_id {
            // SAFE: We know that the index is within bounds, and we know that the pointer will be
            // valid for the new lifetime.
            Some(unsafe {
                SchemaRefMut::from_ptr_schema(
                    self.components.storage.unchecked_idx(self.current_id),
                    self.components.schema,
                )
            })
        } else {
            None
        };
        self.current_id += 1;
        ret
    }
}

#[cfg(test)]
mod tests {
    #![allow(non_snake_case)]

    use super::*;

    #[derive(Clone, HasSchema, Default)]
    struct A;

    #[derive(Clone, HasSchema, Default)]
    struct B;

    #[test]
    fn iter_with_empty_bitset() {
        let mut entities = Entities::default();
        let e = entities.create();
        let mut components = ComponentStore::<A>::default();

        components.insert(e, A);

        let bitset = Rc::new(BitSetVec::default());
        assert_eq!(components.iter_with_bitset(bitset.clone()).count(), 0);
        assert_eq!(components.iter_mut_with_bitset(bitset).count(), 0);
    }

    #[test]
    /// Test that iterating with optional components does not filter entities.
    fn iter_with_optional() {
        // Initialize two total entities, both with B, one with A.
        let mut entities = Entities::default();
        let e1 = entities.create();
        let e2 = entities.create();
        let mut components_a = ComponentStore::<A>::default();
        components_a.insert(e1, A);

        let mut components_b = ComponentStore::<B>::default();
        components_b.insert(e1, B);
        components_b.insert(e2, B);

        // Iterate over all entities, optionally retrieve A
        {
            let comp_a = Ref::new(&components_a);
            let mut count_a = 0;
            let mut count = 0;
            for (_, a) in entities.iter_with(&Optional(&comp_a)) {
                count += 1;
                if a.is_some() {
                    count_a += 1;
                }
            }
            assert_eq!(count_a, 1);
            assert_eq!(count, 2);
        }
        // Mutably Iterate over all entities, optionally retrieve A
        {
            let mut comp_a_mut = RefMut::new(&mut components_a);
            let mut count_a = 0;
            let mut count = 0;
            for (_, a) in entities.iter_with(&mut OptionalMut(&mut comp_a_mut)) {
                count += 1;
                if a.is_some() {
                    count_a += 1;
                }
            }
            assert_eq!(count_a, 1);
            assert_eq!(count, 2);
        }

        // Iterate over entities with B and optionaly retrieve A
        {
            let comp_a = Ref::new(&components_a);
            let comp_b = Ref::new(&components_b);
            let mut count_a = 0;
            let mut count = 0;
            for (_, (a, _b)) in entities.iter_with((&Optional(&comp_a), &comp_b)) {
                count += 1;
                if a.is_some() {
                    count_a += 1;
                }
            }
            assert_eq!(count_a, 1);
            assert_eq!(count, 2);
        }

        // Iterate over entities with A, and optionally retrieve B
        {
            let comp_a = Ref::new(&components_a);
            let comp_b = Ref::new(&components_b);
            let mut count = 0;
            for (_, (_a, b)) in entities.iter_with((&comp_a, &Optional(&comp_b))) {
                count += 1;
                assert!(b.is_some());
            }
            assert_eq!(count, 1);
        }

        // Make sure that entities with only optional components are still filtered by others,
        // and not included in query.
        //
        // Case: 4 entities, we query over A and Optionally C, where entities have comps: 0:[AB],1:[B],2:[C],3:[A]
        // Filtered by A, should iterate over entities 0 and 3. Verify that entitiy 2 with C is not included.
        {
            let e3 = entities.create();
            let e4 = entities.create();
            let mut components_c = ComponentStore::<A>::default();
            components_c.insert(e3, A);
            components_a.insert(e4, A);
            let comp_a = Ref::new(&components_a);
            let comp_c = Ref::new(&components_c);

            let mut count = 0;
            for (_, (_, c)) in entities.iter_with((&comp_a, &Optional(&comp_c))) {
                count += 1;
                // Should not iterate over entity with C, as it does not have A.
                assert!(c.is_none());
            }
            // Expected two entities with A
            assert_eq!(count, 2);
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, HasSchema, Default)]
    struct X(u32);

    #[derive(Debug, Clone, Copy, PartialEq, Eq, HasSchema, Default)]
    struct Y(u32);

    fn entity(index: u32) -> Entity {
        Entity::new(index, 0)
    }

    fn store<C: HasSchema>(entities: &[u32], ctor: fn(u32) -> C) -> ComponentStore<C> {
        let mut store = ComponentStore::default();
        for &i in entities {
            store.insert(entity(i), ctor(i));
        }
        store
    }

    fn bitset(enabled: &[usize]) -> BitSetVec {
        let mut bitset = BitSetVec::default();
        for &i in enabled {
            bitset.bit_set(i);
        }
        bitset
    }

    #[test]
    fn get_single_with_bitset__multiple_required() {
        {
            let bitset = bitset(&[]);
            let (store_x, store_y) = (store(&[], X), store(&[], Y));
            let query = (&Ref::new(&store_x), &Ref::new(&store_y));

            let result = query.get_single_with_bitset(Rc::new(bitset));

            assert_eq!(result, Err(QuerySingleError::NoEntities));
        }

        {
            let bitset = bitset(&[1]);
            let store_x = store(&[1], X);
            let store_y = store(&[1], Y);
            let query = (&Ref::new(&store_x), &Ref::new(&store_y));

            let result = query.get_single_with_bitset(Rc::new(bitset));

            assert_eq!(result, Ok((&X(1), &Y(1))));
        }

        {
            let bitset = bitset(&[1, 2]);
            let store_x = store(&[1, 2], X);
            let store_y = store(&[1, 2], Y);
            let query = (&Ref::new(&store_x), &Ref::new(&store_y));

            let result = query.get_single_with_bitset(Rc::new(bitset));

            assert_eq!(result, Err(QuerySingleError::MultipleEntities));
        }
    }
}
