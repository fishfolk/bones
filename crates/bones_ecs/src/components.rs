//! ECS component storage.

use std::{any::TypeId, sync::Arc};

use crate::prelude::*;

mod iterator;
mod typed;
mod untyped;

pub use iterator::*;
pub use typed::*;
pub use untyped::*;

/// Makes sure that the component type `T` matches the component type previously registered with
/// the same UUID.
fn validate_type_uuid_match<T: TypeUlid + 'static>(
    type_ids: &UlidMap<TypeId>,
) -> Result<(), EcsError> {
    if type_ids.get(&T::ULID).ok_or(EcsError::NotInitialized)? != &TypeId::of::<T>() {
        Err(EcsError::TypeUlidCollision)
    } else {
        Ok(())
    }
}

/// A collection of [`ComponentStore<T>`].
///
/// [`ComponentStores`] is used to in [`World`] to store all component types that have been
/// initialized for that world.
#[derive(Default)]
pub struct ComponentStores {
    pub(crate) components: UlidMap<Arc<AtomicRefCell<UntypedComponentStore>>>,
    type_ids: UlidMap<TypeId>,
}

impl Clone for ComponentStores {
    fn clone(&self) -> Self {
        Self {
            components: self
                .components
                .iter()
                // Be sure to clone the inner data of the components, so we don't just end up with
                // new `Arc`s pointing to the same data.
                .map(|(&k, v)| (k, Arc::new((**v).clone())))
                .collect(),
            type_ids: self.type_ids.clone(),
        }
    }
}

impl ComponentStores {
    /// Initialize component storage for type `T`.
    pub fn init<T: Clone + TypeUlid + Send + Sync + 'static>(&mut self) {
        self.try_init::<T>().unwrap();
    }

    /// Initialize component storage for type `T`.
    pub fn try_init<T: Clone + TypeUlid + Send + Sync + 'static>(
        &mut self,
    ) -> Result<(), EcsError> {
        match self.components.entry(T::ULID) {
            std::collections::hash_map::Entry::Occupied(_) => {
                validate_type_uuid_match::<T>(&self.type_ids)
            }
            std::collections::hash_map::Entry::Vacant(entry) => {
                entry.insert(Arc::new(AtomicRefCell::new(
                    UntypedComponentStore::for_type::<T>(),
                )));
                self.type_ids.insert(T::ULID, TypeId::of::<T>());

                Ok(())
            }
        }
    }

    /// Get the components of a certain type
    ///
    /// # Panics
    ///
    /// Panics if the component type has not been initialized.
    pub fn get<T: Clone + TypeUlid + Send + Sync + 'static>(&self) -> AtomicComponentStore<T> {
        self.try_get::<T>().unwrap()
    }

    /// Get the components of a certain type
    pub fn try_get<T: Clone + TypeUlid + Send + Sync + 'static>(
        &self,
    ) -> Result<AtomicComponentStore<T>, EcsError> {
        validate_type_uuid_match::<T>(&self.type_ids)?;
        let untyped = self.try_get_by_uuid(T::ULID)?;

        // Safe: We've made sure that the data initialized in the untyped components matches T
        unsafe { Ok(AtomicComponentStore::from_components_unsafe(untyped)) }
    }

    /// Get the untyped component storage by the component's UUID
    ///
    /// # Panics
    ///
    /// Panics if the component type has not been initialized.
    pub fn get_by_uuid(&self, uuid: Ulid) -> Arc<AtomicRefCell<UntypedComponentStore>> {
        self.try_get_by_uuid(uuid).unwrap()
    }

    /// Get the untyped component storage by the component's UUID
    pub fn try_get_by_uuid(
        &self,
        uuid: Ulid,
    ) -> Result<Arc<AtomicRefCell<UntypedComponentStore>>, EcsError> {
        self.components
            .get(&uuid)
            .cloned()
            .ok_or(EcsError::NotInitialized)
    }
}

#[cfg(test)]
mod test {
    use crate::prelude::*;

    #[derive(Clone, Copy, TypeUlid)]
    #[ulid = "01GQNWZKBT0SN37QKJQNKPF5RR"]
    struct MyData(pub i32);

    #[test]
    fn borrow_many_mut() {
        World::new()
            .run_system(
                |mut entities: ResMut<Entities>, mut my_datas: CompMut<MyData>| {
                    let ent1 = entities.create();
                    let ent2 = entities.create();

                    my_datas.insert(ent1, MyData(7));
                    my_datas.insert(ent2, MyData(8));

                    {
                        let [data2, data1] = my_datas.get_many_mut([ent2, ent1]).unwrap_many();

                        data1.0 = 0;
                        data2.0 = 1;
                    }

                    assert_eq!(my_datas.get(ent1).unwrap().0, 0);
                    assert_eq!(my_datas.get(ent2).unwrap().0, 1);
                },
            )
            .unwrap();
    }

    #[test]
    #[should_panic = "must be unique"]
    fn borrow_many_overlapping_mut() {
        World::new()
            .run_system(
                |mut entities: ResMut<Entities>, mut my_datas: CompMut<MyData>| {
                    let ent1 = entities.create();
                    let ent2 = entities.create();

                    my_datas.insert(ent1, MyData(1));
                    my_datas.insert(ent2, MyData(2));

                    my_datas.get_many_mut([ent1, ent2, ent1]).unwrap_many();
                },
            )
            .unwrap();
    }
}
