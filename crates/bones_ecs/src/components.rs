//! ECS component storage.

use std::sync::Arc;

use crate::prelude::*;

mod iterator;
mod typed;
mod untyped;

pub use iterator::*;
pub use typed::*;
pub use untyped::*;

/// A collection of [`ComponentStore<T>`].
///
/// [`ComponentStores`] is used to in [`World`] to store all component types that have been
/// initialized for that world.
#[derive(Default)]
pub struct ComponentStores {
    pub(crate) components: HashMap<SchemaId, Arc<AtomicRefCell<UntypedComponentStore>>>,
}

// SOUND: all of the functions for ComponentStores requires that the types stored implement Sync +
// Send.
unsafe impl Sync for ComponentStores {}
unsafe impl Send for ComponentStores {}

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
        }
    }
}

impl ComponentStores {
    /// Initialize component storage for type `T`.
    pub fn init<T: HasSchema>(&mut self) {
        let schema = T::schema();
        self.components.entry(schema.id()).or_insert_with(|| {
            Arc::new(AtomicRefCell::new(UntypedComponentStore::for_type::<T>()))
        });
    }

    /// Get the components of a certain type
    ///
    /// # Panics
    ///
    /// Panics if the component type has not been initialized.
    pub fn get<T: HasSchema>(&self) -> AtomicComponentStore<T> {
        self.try_get::<T>().unwrap()
    }

    /// Get the components of a certain type
    pub fn try_get<T: HasSchema>(&self) -> Result<AtomicComponentStore<T>, EcsError> {
        let untyped = self.try_get_by_schema_id(T::schema().id())?;

        // Safe: We've made sure that the data initialized in the untyped components matches T
        unsafe { Ok(AtomicComponentStore::from_components_unsafe(untyped)) }
    }

    /// Get the untyped component storage by the component's [`SchemaId`].
    ///
    /// # Panics
    ///
    /// Panics if the component type has not been initialized.
    pub fn get_by_uuid(&self, id: SchemaId) -> Arc<AtomicRefCell<UntypedComponentStore>> {
        self.try_get_by_schema_id(id).unwrap()
    }

    /// Get the untyped component storage by the component's [`SchemaId`].
    pub fn try_get_by_schema_id(
        &self,
        id: SchemaId,
    ) -> Result<Arc<AtomicRefCell<UntypedComponentStore>>, EcsError> {
        self.components
            .get(&id)
            .cloned()
            .ok_or(EcsError::NotInitialized)
    }
}

#[cfg(test)]
mod test {
    use crate::prelude::*;

    #[derive(Clone, Copy, HasSchema, Default)]
    #[repr(C)]
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
