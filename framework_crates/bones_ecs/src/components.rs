//! ECS component storage.

use once_map::OnceMap;
use std::sync::Arc;

use crate::prelude::*;

mod iterator;
mod typed;
mod untyped;

pub use iterator::*;
pub use typed::*;
pub use untyped::*;

/// An atomic component store.
pub type AtomicComponentStore<T> = Arc<AtomicCell<ComponentStore<T>>>;
/// An untyped atomic component store.
pub type UntypedAtomicComponentStore = Arc<AtomicCell<UntypedComponentStore>>;

/// A collection of [`ComponentStore<T>`].
///
/// [`ComponentStores`] is used to in [`World`] to store all component types that have been
/// initialized for that world.
#[derive(Default)]
pub struct ComponentStores {
    pub(crate) components: OnceMap<SchemaId, UntypedAtomicComponentStore>,
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
                .read_only_view()
                .iter()
                // Be sure to clone the inner data of the components, so we don't just end up with
                // new `Arc`s pointing to the same data.
                .map(|(&k, v)| (k, Arc::new((**v).clone())))
                .collect(),
        }
    }
}

impl DesyncHash for ComponentStores {
    fn hash(&self, hasher: &mut dyn std::hash::Hasher) {
        for (_, component_store) in self.components.read_only_view().iter() {
            // Verify Schema for component store implement desync hash. If no hash_fn, while the
            // components will not impact hash, we should probably not hash the schema_id in this case.
            let component_store = component_store.as_ref().borrow();
            if component_store
                .schema()
                .type_data
                .get::<SchemaDesyncHash>()
                .is_some()
            {
                component_store.schema().full_name.hash(hasher);
                component_store.hash(hasher);
            }
        }
    }
}

impl ComponentStores {
    /// Get the components of a certain type
    pub fn get_cell<T: HasSchema>(&self) -> AtomicComponentStore<T> {
        let untyped = self.get_cell_by_schema(T::schema());

        // Safe: We know the schema matches, and `ComponentStore<T>` is repr(transparent) over
        // `UntypedComponentStore`.
        unsafe {
            std::mem::transmute::<
                Arc<AtomicCell<UntypedComponentStore>>,
                Arc<AtomicCell<ComponentStore<T>>>,
            >(untyped)
        }
    }

    /// Borrow a component store.
    /// # Errors
    /// Errors if the component store has not been initialized yet.
    pub fn get<T: HasSchema>(&self) -> &AtomicCell<ComponentStore<T>> {
        let schema = T::schema();
        let atomiccell = self.get_by_schema(schema);

        // SOUND: ComponentStore<T> is repr(transparent) over UntypedComponent store.
        unsafe {
            std::mem::transmute::<&AtomicCell<UntypedComponentStore>, &AtomicCell<ComponentStore<T>>>(
                atomiccell,
            )
        }
    }

    /// Get the untyped component storage by the component's [`SchemaId`].
    pub fn get_by_schema(&self, schema: &'static Schema) -> &AtomicCell<UntypedComponentStore> {
        self.components.insert(schema.id(), |_| {
            Arc::new(AtomicCell::new(UntypedComponentStore::new(schema)))
        })
    }

    /// Get the untyped component storage by the component's [`SchemaId`].
    pub fn get_cell_by_schema(
        &self,
        schema: &'static Schema,
    ) -> Arc<AtomicCell<UntypedComponentStore>> {
        self.components.map_insert(
            schema.id(),
            |_| Arc::new(AtomicCell::new(UntypedComponentStore::new(schema))),
            |_key, value| value.clone(),
        )
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
        World::new().run_system(
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
            (),
        );
    }

    #[test]
    #[should_panic = "must be unique"]
    fn borrow_many_overlapping_mut() {
        World::new().run_system(
            |mut entities: ResMut<Entities>, mut my_datas: CompMut<MyData>| {
                let ent1 = entities.create();
                let ent2 = entities.create();

                my_datas.insert(ent1, MyData(1));
                my_datas.insert(ent2, MyData(2));

                my_datas.get_many_mut([ent1, ent2, ent1]).unwrap_many();
            },
            (),
        )
    }
}
