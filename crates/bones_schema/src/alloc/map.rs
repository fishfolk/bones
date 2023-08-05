//
// 🚧 Under Construction 🚧
//

use std::marker::PhantomData;

use bones_utils::HashMap;

use crate::prelude::*;

/// Untyped schema-aware "HashMap".
pub struct SchemaMap {
    map: HashMap<SchemaBox, SchemaBox>,
    key_schema: &'static Schema,
    value_schema: &'static Schema,
}

impl SchemaMap {
    /// Create a new map, with the given key and value schemas.
    pub fn new(key_schema: &'static Schema, value_schema: &'static Schema) -> Self {
        Self {
            map: HashMap::default(),
            key_schema,
            value_schema,
        }
    }

    /// Insert an item into themap.
    #[inline]
    #[track_caller]
    pub fn insert<K: HasSchema, V: HasSchema>(&mut self, key: K, value: V) {
        self.try_insert(key, value).unwrap()
    }

    /// Insert an item into the map.
    pub fn try_insert<K: HasSchema, V: HasSchema>(
        &mut self,
        key: K,
        value: V,
    ) -> Result<(), SchemaMismatchError> {
        if K::schema() != self.key_schema || V::schema() != self.value_schema {
            Err(SchemaMismatchError)
        } else {
            self.map.insert(SchemaBox::new(key), SchemaBox::new(value));
            Ok(())
        }
    }

    // pub fn try_get<K: HasSchema, V: HasSchema>(
    //     &self,
    //     key: &K,
    // ) -> Result<Option<&V>, SchemaMismatchError> {
    //     if K::schema() != self.key_schema || V::schema() != self.value_schema {
    //         Err(SchemaMismatchError)
    //     } else {
    //         Ok(self.map.get())
    //     }
    // }
}

/// Typed version of a [`SchemaMap`].
pub struct SMap<K: HasSchema, V: HasSchema> {
    _map: SchemaMap,
    _phantom: PhantomData<(K, V)>,
}

#[cfg(test)]
mod test {
    #[test]
    fn sanity_check() {

    }
}
