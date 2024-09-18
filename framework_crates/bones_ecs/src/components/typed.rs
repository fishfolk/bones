use std::{marker::PhantomData, rc::Rc};

use crate::prelude::*;

use super::untyped::UntypedComponentStore;

/// A typed wrapper around [`UntypedComponentStore`].
#[derive(Clone)]
#[repr(transparent)]
pub struct ComponentStore<T: HasSchema> {
    untyped: UntypedComponentStore,
    _phantom: PhantomData<T>,
}

impl<T: HasSchema> Default for ComponentStore<T> {
    fn default() -> Self {
        Self {
            untyped: UntypedComponentStore::for_type::<T>(),
            _phantom: PhantomData,
        }
    }
}

impl<T: HasSchema> TryFrom<UntypedComponentStore> for ComponentStore<T> {
    type Error = SchemaMismatchError;

    fn try_from(untyped: UntypedComponentStore) -> Result<Self, Self::Error> {
        if untyped.schema == T::schema() {
            Ok(Self {
                untyped,
                _phantom: PhantomData,
            })
        } else {
            Err(SchemaMismatchError)
        }
    }
}

impl<T: HasSchema> ComponentStore<T> {
    /// Converts to the internal, untyped [`ComponentStore`].
    #[inline]
    pub fn into_untyped(self) -> UntypedComponentStore {
        self.untyped
    }

    /// Creates a [`ComponentStore`] from an [`UntypedComponentStore`].
    /// # Panics
    /// Panics if the schema doesn't match `T`.
    #[track_caller]
    pub fn from_untyped(untyped: UntypedComponentStore) -> Self {
        untyped.try_into().unwrap()
    }

    // TODO: Replace ComponentStore functions with non-validating ones.
    // Right now functions like `insert`, `get`, and `get_mut` use the checked and panicing versions
    // of the `untyped` functions. These functions do an extra check to see that the schema matches,
    // but we've already validated that in the construction of the `ComponentStore`, so we should
    // bypass the extra schema check for performance.

    /// Inserts a component for the given `Entity` index.
    /// Returns the previous component, if any.
    #[inline]
    pub fn insert(&mut self, entity: Entity, component: T) -> Option<T> {
        self.untyped.insert(entity, component)
    }

    /// Gets an immutable reference to the component of `Entity`.
    #[inline]
    pub fn get(&self, entity: Entity) -> Option<&T> {
        self.untyped.get(entity)
    }

    /// Gets a mutable reference to the component of `Entity`.
    #[inline]
    pub fn get_mut(&mut self, entity: Entity) -> Option<&mut T> {
        self.untyped.get_mut(entity)
    }

    /// Get a mutable reference to component if it exists.
    /// Otherwise inserts `T` generated by calling parameter: `f`.
    #[inline]
    pub fn get_mut_or_insert(&mut self, entity: Entity, f: impl FnOnce() -> T) -> &mut T {
        self.untyped.get_mut_or_insert(entity, f)
    }

    /// Get mutable references to the component data for multiple entities at the same time.
    ///
    /// # Panics
    ///
    /// This will panic if the same entity is specified multiple times. This is invalid because it
    /// would mean you would have two mutable references to the same component data at the same
    /// time.
    #[track_caller]
    pub fn get_many_mut<const N: usize>(&mut self, entities: [Entity; N]) -> [Option<&mut T>; N] {
        let mut result = self.untyped.get_many_ref_mut(entities);

        std::array::from_fn(move |i| {
            // SOUND: we know that the schema matches.
            result[i]
                .take()
                .map(|x| unsafe { x.cast_into_mut_unchecked() })
        })
    }

    /// Removes the component of `Entity`.
    /// Returns `Some(T)` if the entity did have the component.
    /// Returns `None` if the entity did not have the component.
    #[inline]
    pub fn remove(&mut self, entity: Entity) -> Option<T> {
        self.untyped.remove(entity)
    }

    /// Gets an immutable reference to the component if there is exactly one instance of it.
    #[inline]
    pub fn get_single_with_bitset(&self, bitset: Rc<BitSetVec>) -> Result<&T, QuerySingleError> {
        // SOUND: we know the schema matches.
        self.untyped
            .get_single_with_bitset(bitset)
            .map(|x| unsafe { x.cast_into_unchecked() })
    }

    /// Gets a mutable reference to the component if there is exactly one instance of it.
    #[inline]
    pub fn get_single_with_bitset_mut(
        &mut self,
        bitset: Rc<BitSetVec>,
    ) -> Result<&mut T, QuerySingleError> {
        // SOUND: we know the schema matches.
        self.untyped
            .get_single_with_bitset_mut(bitset)
            .map(|x| unsafe { x.cast_into_mut_unchecked() })
    }

    /// Iterates immutably over all components of this type.
    /// Very fast but doesn't allow joining with other component types.
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        // SOUND: we know the schema matches.
        self.untyped
            .iter()
            .map(|x| unsafe { x.cast_into_unchecked() })
    }

    /// Iterates mutably over all components of this type.
    /// Very fast but doesn't allow joining with other component types.
    #[inline]
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> {
        // SOUND: we know the schema matches.
        self.untyped
            .iter_mut()
            .map(|x| unsafe { x.cast_into_mut_unchecked() })
    }
}

/// This trait factors out functions for iterating with bitset over component store.
/// Separated from `impl ComponentStore` for usage in generic trait types that must
/// be able to create [`ComponentBitsetIterator`] and related types.
///
/// Automatically implemented for [`ComponentStore`].
pub trait ComponentIterBitset<'a, T: HasSchema> {
    /// Gets an immutable reference to the component if there is exactly one instance of it.
    fn get_single_with_bitset(&self, bitset: Rc<BitSetVec>) -> Result<&T, QuerySingleError>;

    /// Gets a mutable reference to the component if there is exactly one instance of it.
    fn get_single_mut_with_bitset(
        &mut self,
        bitset: Rc<BitSetVec>,
    ) -> Result<&mut T, QuerySingleError>;

    /// Iterates immutably over the components of this type where `bitset`
    /// indicates the indices of entities.
    /// Slower than `iter()` but allows joining between multiple component types.
    fn iter_with_bitset(&self, bitset: Rc<BitSetVec>) -> ComponentBitsetIterator<T>;

    /// Iterates immutably over the components of this type where `bitset`
    /// indicates the indices of entities.
    /// Slower than `iter()` but allows joining between multiple component types.
    fn iter_with_bitset_optional(
        &self,
        bitset: Rc<BitSetVec>,
    ) -> ComponentBitsetOptionalIterator<T>;

    /// Iterates mutable over the components of this type where `bitset`
    /// indicates the indices of entities.
    /// Slower than `iter()` but allows joining between multiple component types.
    fn iter_mut_with_bitset(&mut self, bitset: Rc<BitSetVec>) -> ComponentBitsetIteratorMut<T>;

    /// Iterates mutably over the components of this type where `bitset`
    /// indicates the indices of entities.
    /// Slower than `iter()` but allows joining between multiple component types.
    fn iter_mut_with_bitset_optional(
        &mut self,
        bitset: Rc<BitSetVec>,
    ) -> ComponentBitsetOptionalIteratorMut<T>;

    /// Get bitset of [`ComponentStore`] / implementor.
    fn bitset(&self) -> &BitSetVec;

    /// Check whether or not this component store has data for the given entity.
    fn contains(&self, entity: Entity) -> bool;

    /// Get [`ComponentStore`] for usage with generic types implementing [`ComponentIterBitset`].
    fn component_store(&self) -> &ComponentStore<T>;
}

impl<'a, T: HasSchema> ComponentIterBitset<'a, T> for ComponentStore<T> {
    /// Gets an immutable reference to the component if there is exactly one instance of it.
    fn get_single_with_bitset(&self, bitset: Rc<BitSetVec>) -> Result<&T, QuerySingleError> {
        // SOUND: we know the schema matches.
        fn map<T>(r: SchemaRef) -> &T {
            unsafe { r.cast_into_unchecked() }
        }
        self.untyped.get_single_with_bitset(bitset).map(map)
    }

    /// Gets a mutable reference to the component if there is exactly one instance of it.
    fn get_single_mut_with_bitset(
        &mut self,
        bitset: Rc<BitSetVec>,
    ) -> Result<&mut T, QuerySingleError> {
        // SOUND: we know the schema matches.
        fn map<T>(r: SchemaRefMut) -> &mut T {
            unsafe { r.cast_into_mut_unchecked() }
        }
        self.untyped.get_single_with_bitset_mut(bitset).map(map)
    }

    /// Iterates immutably over the components of this type where `bitset`
    /// indicates the indices of entities.
    /// Slower than `iter()` but allows joining between multiple component types.
    #[inline]
    fn iter_with_bitset(&self, bitset: Rc<BitSetVec>) -> ComponentBitsetIterator<T> {
        // SOUND: we know the schema matches.
        fn map<T>(r: SchemaRef) -> &T {
            unsafe { r.cast_into_unchecked() }
        }
        self.untyped.iter_with_bitset(bitset).map(map)
    }

    /// Iterates immutably over the components of this type where `bitset`
    /// indicates the indices of entities where iterator returns an Option.
    /// None is returned for entities in bitset when Component is not in [`ComponentStore`]
    #[inline]
    fn iter_with_bitset_optional(
        &self,
        bitset: Rc<BitSetVec>,
    ) -> ComponentBitsetOptionalIterator<T> {
        // SOUND: we know the schema matches.
        fn map<T>(r: Option<SchemaRef>) -> Option<&T> {
            r.map(|r| unsafe { r.cast_into_unchecked() })
        }
        self.untyped.iter_with_bitset_optional(bitset).map(map)
    }

    /// Iterates mutable over the components of this type where `bitset`
    /// indicates the indices of entities.
    /// Slower than `iter()` but allows joining between multiple component types.
    #[inline]
    fn iter_mut_with_bitset(&mut self, bitset: Rc<BitSetVec>) -> ComponentBitsetIteratorMut<T> {
        // SOUND: we know the schema matches.
        fn map<T>(r: SchemaRefMut) -> &mut T {
            unsafe { r.cast_into_mut_unchecked() }
        }

        self.untyped.iter_mut_with_bitset(bitset).map(map)
    }

    /// Iterates mutably over the components of this type where `bitset`
    /// indicates the indices of entities where iterator returns an Option.
    /// None is returned for entities in bitset when Component is not in [`ComponentStore`
    #[inline]
    fn iter_mut_with_bitset_optional(
        &mut self,
        bitset: Rc<BitSetVec>,
    ) -> ComponentBitsetOptionalIteratorMut<T> {
        // SOUND: we know the schema matches.
        fn map<T>(r: Option<SchemaRefMut>) -> Option<&mut T> {
            r.map(|r| unsafe { r.cast_into_mut_unchecked() })
        }
        self.untyped.iter_mut_with_bitset_optional(bitset).map(map)
    }

    /// Read the bitset containing the list of entites with this component type on it.
    #[inline]
    fn bitset(&self) -> &BitSetVec {
        self.untyped.bitset()
    }

    /// Check whether or not this component store has data for the given entity.
    #[inline]
    fn contains(&self, entity: Entity) -> bool {
        self.bitset().contains(entity)
    }

    //// Get [`ComponentStore`] for usage with generic types implementing [`ComponentIterBitset`].
    #[inline]
    fn component_store(&self) -> &ComponentStore<T> {
        self
    }
}

#[cfg(test)]
mod tests {
    use std::{iter, rc::Rc};

    use crate::prelude::*;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, HasSchema, Default)]
    #[repr(C)]
    struct A(u32);

    #[test]
    fn create_remove_components() {
        let mut entities = Entities::default();
        let e1 = entities.create();
        let e2 = entities.create();

        let mut storage = ComponentStore::<A>::default();
        storage.insert(e1, A(1));
        storage.insert(e2, A(2));
        assert!(storage.get(e1).is_some());
        storage.remove(e1);
        assert!(storage.get(e1).is_none());
        assert!(storage.iter().eq(iter::once(&A(2))));
    }

    #[test]
    fn get_mut_or_insert() {
        let mut entities = Entities::default();
        let e1 = entities.create();

        let mut storage = ComponentStore::<A>::default();
        {
            // Test that inserted component is correct
            let comp = storage.get_mut_or_insert(e1, || A(1));
            assert_eq!(comp.0, 1);

            // Mutate component
            comp.0 = 2;
        }

        // Should not insert the unexpected value but retrieve original mutated component.
        let comp = storage.get_mut_or_insert(e1, || A(u32::MAX));

        // Test that existing component is retrieved
        assert_eq!(comp.0, 2);
    }

    #[test]
    fn single_returns_none_when_empty() {
        let storage = ComponentStore::<A>::default();
        let bitset = Rc::new({
            let mut entities = Entities::default();
            entities.create();
            entities.bitset().clone()
        });

        let maybe_comp = storage.get_single_with_bitset(bitset);

        assert_eq!(maybe_comp, Err(QuerySingleError::NoEntities));
    }

    #[test]
    fn single_returns_some_single() {
        let mut storage = ComponentStore::<A>::default();
        let mut entities = Entities::default();

        // Create some dummies so that the target entity isn't 0
        (0..3).map(|_| entities.create()).count();

        let e = entities.create();
        let a = A(1);
        storage.insert(e, a);

        let bitset = Rc::new(entities.bitset().clone());

        let maybe_comp = storage.get_single_with_bitset(bitset);

        assert_eq!(maybe_comp, Ok(&a));
    }

    #[test]
    fn single_returns_none_when_more_than_1() {
        let mut entities = Entities::default();
        let mut storage = ComponentStore::<A>::default();

        (0..3)
            .map(|i| storage.insert(entities.create(), A(i)))
            .count();

        let bitset = Rc::new(entities.bitset().clone());

        let maybe_comp = storage.get_single_with_bitset(bitset);

        assert_eq!(maybe_comp, Err(QuerySingleError::MultipleEntities));
    }

    #[test]
    fn iter_with_bitset() {
        let mut entities = Entities::default();
        let mut storage = ComponentStore::<A>::default();

        {
            let bitset = Rc::new(entities.bitset().clone());

            let mut comp_iter = storage.iter_with_bitset(bitset.clone());
            assert_eq!(comp_iter.next(), None);

            let mut comp_mut_iter = storage.iter_mut_with_bitset(bitset);
            assert_eq!(comp_mut_iter.next(), None);
        }

        {
            let e = entities.create();
            let mut a = A(1);
            storage.insert(e, a);

            let bitset = Rc::new(entities.bitset().clone());

            let mut comp_iter = storage.iter_with_bitset(bitset.clone());
            assert_eq!(comp_iter.next(), Some(&a));

            let mut comp_mut_iter = storage.iter_mut_with_bitset(bitset);
            assert_eq!(comp_mut_iter.next(), Some(&mut a));

            entities.kill(e);
        }
    }

    #[test]
    fn iter_with_bitset_optional() {
        let mut entities = Entities::default();
        let mut storage = ComponentStore::<A>::default();

        {
            let bitset = Rc::new(entities.bitset().clone());

            let mut comp_iter = storage.iter_with_bitset_optional(bitset.clone());
            assert_eq!(comp_iter.next(), None);

            let mut comp_mut_iter = storage.iter_mut_with_bitset_optional(bitset);
            assert_eq!(comp_mut_iter.next(), None);
        }

        {
            let e = entities.create();
            let bitset = Rc::new(entities.bitset().clone());

            let mut comp_iter = storage.iter_with_bitset_optional(bitset.clone());
            assert_eq!(comp_iter.next(), Some(None));

            let mut comp_mut_iter = storage.iter_mut_with_bitset_optional(bitset);
            assert_eq!(comp_mut_iter.next(), Some(None));

            entities.kill(e);
        }

        {
            let e = entities.create();
            let mut a = A(0);
            storage.insert(e, a);
            let bitset = Rc::new(entities.bitset().clone());

            let mut comp_iter = storage.iter_with_bitset_optional(bitset.clone());
            assert_eq!(comp_iter.next(), Some(Some(&a)));

            let mut comp_mut_iter = storage.iter_mut_with_bitset_optional(bitset);
            assert_eq!(comp_mut_iter.next(), Some(Some(&mut a)));

            entities.kill(e);
        }
    }
}
