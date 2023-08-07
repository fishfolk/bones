//
// 🚧 Under Construction 🚧
//

use std::{
    hash::{BuildHasher, Hasher},
    marker::PhantomData,
};

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
        assert!(
            key_schema.hash_fn.is_some() && key_schema.eq_fn.is_some(),
            "Key schema must implement hash and eq"
        );
        Self {
            map: HashMap::default(),
            key_schema,
            value_schema,
        }
    }

    /// Insert an item into themap.
    #[inline]
    #[track_caller]
    pub fn insert<K: HasSchema, V: HasSchema>(&mut self, key: K, value: V) -> Option<V> {
        self.try_insert(key, value).unwrap()
    }

    /// Insert an item into the map.
    pub fn try_insert<K: HasSchema, V: HasSchema>(
        &mut self,
        key: K,
        value: V,
    ) -> Result<Option<V>, SchemaMismatchError> {
        let key = SchemaBox::new(key);
        let value = SchemaBox::new(value);
        self.try_insert_box(key, value)
            // SOUND: We've try_insert_box won't succeed unless the schema's match, so we have already
            // checked that the value type matches.
            .map(|value| value.map(|x| unsafe { x.into_inner_unchecked() }))
    }

    /// Insert an item into the map.
    #[track_caller]
    #[inline]
    pub fn insert_box(&mut self, key: SchemaBox, value: SchemaBox) -> Option<SchemaBox> {
        self.try_insert_box(key, value).unwrap()
    }

    /// Insert an item into the map.
    pub fn try_insert_box(
        &mut self,
        key: SchemaBox,
        mut value: SchemaBox,
    ) -> Result<Option<SchemaBox>, SchemaMismatchError> {
        if key.schema() != self.key_schema || value.schema() != self.value_schema {
            Err(SchemaMismatchError)
        } else {
            let hash = {
                let mut hasher = self.map.hasher().build_hasher();
                hasher.write_u64(key.hash());
                hasher.finish()
            };
            let entry = self
                .map
                .raw_entry_mut()
                .from_hash(hash, |other| other == &key);
            let previous_value = match entry {
                bones_utils::hashbrown::hash_map::RawEntryMut::Occupied(mut occupied) => {
                    std::mem::swap(occupied.get_mut(), &mut value);
                    Some(value)
                }
                bones_utils::hashbrown::hash_map::RawEntryMut::Vacant(vacant) => {
                    vacant.insert(key, value);
                    None
                }
            };

            Ok(previous_value)
        }
    }

    /// Get a value out of the map for the given key.
    /// # Panics
    /// Panics if the schemas of the key and value don't match the schemas of the map.
    #[track_caller]
    #[inline]
    pub fn get<K: HasSchema, V: HasSchema>(&self, key: &K) -> Option<&V> {
        self.try_get(key).unwrap()
    }

    /// Get a value out of the map for the given key.
    /// # Errors
    /// Errors if the schemas of the key and value don't match the schemas of the map.
    #[track_caller]
    pub fn try_get<K: HasSchema, V: HasSchema>(
        &self,
        key: &K,
    ) -> Result<Option<&V>, SchemaMismatchError> {
        if K::schema() != self.key_schema || V::schema() != self.value_schema {
            Err(SchemaMismatchError)
        } else {
            let Some(hash_fn) = self.key_schema.hash_fn else {
                panic!("Key schema doesn't implement hash");
            };
            let Some(eq_fn) = self.key_schema.eq_fn else {
                panic!("Key schema doesn't implement eq");
            };
            let key_ref = key as *const K as *const u8;
            // SOUND: we know the hash function is valid for the schema
            let hash = unsafe { (hash_fn)(key_ref) };
            let hash = {
                let mut hasher = self.map.hasher().build_hasher();
                hasher.write_u64(hash);
                hasher.finish()
            };
            let value = self
                .map
                .raw_entry()
                .from_hash(hash, |key| {
                    let other_ref = key.as_ref().as_ptr();
                    // SOUND: we know the eq function is valid for the schema
                    unsafe { (eq_fn)(key_ref, other_ref) }
                })
                .map(|x| x.1)
                // SOUND: we know the schema box's schema matches the casted type.
                .map(|s_box| unsafe { s_box.as_ref().deref() });
            Ok(value)
        }
    }

    /// Get a value out of the map for the given key.
    /// # Panics
    /// Panics if the schemas of the key and value don't match the schemas of the map.
    #[track_caller]
    #[inline]
    pub fn get_mut<K: HasSchema, V: HasSchema>(&mut self, key: &K) -> Option<&mut V> {
        self.try_get_mut(key).unwrap()
    }

    /// Get a value out of the map for the given key.
    /// # Errors
    /// Errors if the schemas of the key and value don't match the schemas of the map.
    #[track_caller]
    pub fn try_get_mut<K: HasSchema, V: HasSchema>(
        &mut self,
        key: &K,
    ) -> Result<Option<&mut V>, SchemaMismatchError> {
        if K::schema() != self.key_schema || V::schema() != self.value_schema {
            Err(SchemaMismatchError)
        } else {
            let Some(hash_fn) = self.key_schema.hash_fn else {
                panic!("Key schema doesn't implement hash");
            };
            let Some(eq_fn) = self.key_schema.eq_fn else {
                panic!("Key schema doesn't implement eq");
            };
            let key_ref = key as *const K as *const u8;
            // SOUND: we know the hash function is valid for the schema
            let hash = unsafe { (hash_fn)(key_ref) };
            let hash = {
                let mut hasher = self.map.hasher().build_hasher();
                hasher.write_u64(hash);
                hasher.finish()
            };
            let entry = self.map.raw_entry_mut().from_hash(hash, |key| {
                let other_ref = key.as_ref().as_ptr();
                // SOUND: we know the eq function is valid for the schema
                unsafe { (eq_fn)(key_ref, other_ref) }
            });
            let value = match entry {
                bones_utils::hashbrown::hash_map::RawEntryMut::Occupied(entry) => {
                    Some(entry.into_mut())
                }
                bones_utils::hashbrown::hash_map::RawEntryMut::Vacant(_) => None,
            }
            // SOUND: we know the schema box's schema matches the casted type.
            .map(|x| unsafe { x.as_mut().deref_mut() });
            Ok(value)
        }
    }
}

/// Typed version of a [`SchemaMap`].
pub struct SMap<K: HasSchema, V: HasSchema> {
    _map: SchemaMap,
    _phantom: PhantomData<(K, V)>,
}
