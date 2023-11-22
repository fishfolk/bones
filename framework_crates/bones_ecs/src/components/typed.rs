use std::{marker::PhantomData, rc::Rc};

use crate::prelude::*;

use super::untyped::UntypedComponentStore;

/// A typed wrapper around [`UntypedComponentStore`].
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

    /// Get mutable references s to the component data for multiple entities at the same time.
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

    /// Iterates immutably over the components of this type where `bitset`
    /// indicates the indices of entities.
    /// Slower than `iter()` but allows joining between multiple component types.
    #[inline]
    pub fn iter_with_bitset(&self, bitset: Rc<BitSetVec>) -> ComponentBitsetIterator<T> {
        // SOUND: we know the schema matches.
        fn map<T>(r: SchemaRef) -> &T {
            unsafe { r.cast_into_unchecked() }
        }
        self.untyped.iter_with_bitset(bitset).map(map)
    }

    /// Iterates mutable over the components of this type where `bitset`
    /// indicates the indices of entities.
    /// Slower than `iter()` but allows joining between multiple component types.
    #[inline]
    pub fn iter_mut_with_bitset(&mut self, bitset: Rc<BitSetVec>) -> ComponentBitsetIteratorMut<T> {
        // SOUND: we know the schema matches.
        fn map<T>(r: SchemaRefMut<'_>) -> &mut T {
            unsafe { r.cast_into_mut_unchecked() }
        }

        self.untyped.iter_mut_with_bitset(bitset).map(map)
    }

    /// Read the bitset containing the list of entites with this component type on it.
    #[inline]
    pub fn bitset(&self) -> &BitSetVec {
        self.untyped.bitset()
    }

    /// Check whether or not this component store has data for the given entity.
    #[inline]
    pub fn contains(&self, entity: Entity) -> bool {
        self.bitset().contains(entity)
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;

    #[test]
    fn create_remove_components() {
        #[derive(Debug, Clone, PartialEq, Eq, HasSchema, Default)]
        #[repr(C)]
        struct A(String);

        let mut entities = Entities::default();
        let e1 = entities.create();
        let e2 = entities.create();

        let mut storage = ComponentStore::<A>::default();
        storage.insert(e1, A("hello".into()));
        storage.insert(e2, A("world".into()));
        assert!(storage.get(e1).is_some());
        storage.remove(e1);
        assert!(storage.get(e1).is_none());
        assert_eq!(
            storage.iter().cloned().collect::<Vec<_>>(),
            vec![A("world".into())]
        )
    }
}
