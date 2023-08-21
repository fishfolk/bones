use bones_utils::HashMap;

use crate::prelude::*;

/// A `TypeMap`-like structure that can store items that implement [`HasSchema`].
#[derive(Clone, Debug, Default)]
pub struct SchemaTypeMap(HashMap<SchemaId, SchemaBox>);

impl SchemaTypeMap {
    /// Get data out of the store.
    #[track_caller]
    pub fn get<T: HasSchema>(&self) -> Option<&T> {
        let schema = T::schema();
        self.0.get(&schema.id()).map(|x| x.cast_ref())
    }

    /// Insert data into the store
    pub fn insert<T: HasSchema>(&mut self, data: T) {
        self.0.insert(T::schema().id(), SchemaBox::new(data));
    }

    /// Remove data from the store.
    pub fn remove<T: HasSchema>(&mut self) -> Option<T> {
        self.0.remove(&T::schema().id()).map(|x| x.into_inner())
    }
}
