use std::rc::Rc;

use crate::prelude::*;

/// Read-only iterator over components matching a given bitset
pub type ComponentBitsetIterator<'a, T> =
    std::iter::Map<UntypedComponentBitsetIterator<'a>, for<'b> fn(SchemaRef<'b>) -> &'b T>;

/// Mutable iterator over components matching a given bitset
pub type ComponentBitsetIteratorMut<'a, T> = std::iter::Map<
    UntypedComponentBitsetIteratorMut<'a>,
    for<'b> fn(SchemaRefMut<'b, 'b>) -> &'b mut T,
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
        while !(self.bitset.bit_test(self.current_id)
            && self.components.bitset.bit_test(self.current_id))
            && self.current_id <= max_id
        {
            self.current_id += 1;
        }
        let ret = if self.current_id <= max_id {
            // SAFE: Here we are just getting a pointer, not doing anything unsafe with it.
            Some(unsafe {
                SchemaRef::from_ptr_schema(
                    self.components
                        .storage
                        .unchecked_idx(self.current_id)
                        .as_ptr(),
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

/// Iterates over components using a provided bitset. Each time the bitset has a 1 in index i, the
/// iterator will fetch data from the storage at index i.
pub struct UntypedComponentBitsetIteratorMut<'a> {
    pub(crate) current_id: usize,
    pub(crate) components: &'a mut UntypedComponentStore,
    pub(crate) bitset: Rc<BitSetVec>,
}

impl<'a> Iterator for UntypedComponentBitsetIteratorMut<'a> {
    type Item = SchemaRefMut<'a, 'a>;
    fn next(&mut self) -> Option<Self::Item> {
        let max_id = self.components.max_id;
        while !(self.bitset.bit_test(self.current_id)
            && self.components.bitset.bit_test(self.current_id))
            && self.current_id <= max_id
        {
            self.current_id += 1;
        }
        let ret = if self.current_id <= max_id {
            // SAFE: We know that the index is within bounds, and we know that the pointer will be
            // valid for the new lifetime.
            Some(unsafe {
                SchemaRefMut::from_ptr_schema(
                    self.components
                        .storage
                        .unchecked_idx_mut(self.current_id)
                        .as_ptr(),
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
mod test {
    use super::*;

    #[derive(Clone, HasSchema, Default)]
    struct A;

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
}
