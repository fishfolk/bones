//! ECS component storage.

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
    pub(crate) components: HashMap<SchemaId, UntypedAtomicComponentStore>,
}

/// An error returned when trying to access an uninitialized component.
#[derive(Debug)]
pub struct NotInitialized;

impl std::fmt::Display for NotInitialized {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Component not initialized")
    }
}
impl std::error::Error for NotInitialized {}

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
        self.components
            .entry(schema.id())
            .or_insert_with(|| Arc::new(AtomicCell::new(UntypedComponentStore::for_type::<T>())));
    }

    /// Get the components of a certain type
    pub fn get_cell<T: HasSchema>(&self) -> Result<AtomicComponentStore<T>, NotInitialized> {
        let untyped = self.get_cell_by_schema_id(T::schema().id())?;

        // Safe: We know the schema matches, and `ComponentStore<T>` is repr(transparent) over
        // `UntypedComponentStore`.
        unsafe {
            Ok(std::mem::transmute::<
                Arc<AtomicCell<UntypedComponentStore>>,
                Arc<AtomicCell<ComponentStore<T>>>,
            >(untyped))
        }
    }

    /// Borrow a component store.
    /// # Errors
    /// Errors if the component store has not been initialized yet.
    pub fn get<T: HasSchema>(&self) -> Result<Ref<ComponentStore<T>>, NotInitialized> {
        let id = T::schema().id();
        let atomicref = self.components.get(&id).ok_or(NotInitialized)?.borrow();

        // SOUND: ComponentStore<T> is repr(transparent) over UntypedComponent store.
        let atomicref = Ref::map(atomicref, |x| unsafe {
            std::mem::transmute::<&UntypedComponentStore, &ComponentStore<T>>(x)
        });

        Ok(atomicref)
    }

    /// Borrow a component store.
    /// # Errors
    /// Errors if the component store has not been initialized yet.
    pub fn get_mut<T: HasSchema>(&self) -> Result<RefMut<ComponentStore<T>>, NotInitialized> {
        let id = T::schema().id();
        let atomicref = self.components.get(&id).ok_or(NotInitialized)?.borrow_mut();

        // SOUND: ComponentStore<T> is repr(transparent) over UntypedComponent store.
        let atomicref = RefMut::map(atomicref, |x| unsafe {
            std::mem::transmute::<&mut UntypedComponentStore, &mut ComponentStore<T>>(x)
        });

        Ok(atomicref)
    }

    /// Get the untyped component storage by the component's [`SchemaId`].
    pub fn get_cell_by_schema_id(
        &self,
        id: SchemaId,
    ) -> Result<Arc<AtomicCell<UntypedComponentStore>>, NotInitialized> {
        self.components.get(&id).cloned().ok_or(NotInitialized)
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
