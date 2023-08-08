use std::{
    any::TypeId,
    hash::{BuildHasher, Hasher},
    marker::PhantomData,
    sync::OnceLock,
};

use bones_utils::HashMap;

use crate::{
    prelude::*,
    raw_fns::{RawClone, RawDefault, RawDrop},
};

/// Untyped schema-aware "HashMap".
#[derive(Clone, Debug)]
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

    /// Get the schema for the map keys.
    pub fn key_schema(&self) -> &'static Schema {
        self.key_schema
    }

    /// Get the schema for the map values.
    pub fn value_schema(&self) -> &'static Schema {
        self.value_schema
    }

    /// Insert an item into the map.
    /// # Panics
    /// Panics if the key or value schemas do not match the map's.
    #[inline]
    #[track_caller]
    pub fn insert<K: HasSchema, V: HasSchema>(&mut self, key: K, value: V) -> Option<V> {
        self.try_insert(key, value).unwrap()
    }

    /// Insert an item into the map.
    /// # Errors
    /// Errors if the key or value schemas do not match the map's.
    pub fn try_insert<K: HasSchema, V: HasSchema>(
        &mut self,
        key: K,
        value: V,
    ) -> Result<Option<V>, SchemaMismatchError> {
        let key = SchemaBox::new(key);
        let value = SchemaBox::new(value);
        self.try_insert_box(key, value)
            // SOUND: try_insert_box won't succeed unless the schema's match, so we have already
            // checked that the value schema matches.
            .map(|value| value.map(|x| unsafe { x.into_inner_unchecked() }))
    }

    /// Insert an untyped item into the map.
    /// # Panics
    /// Panics if the key or value schemas do not match the map's.
    #[track_caller]
    #[inline]
    pub fn insert_box(&mut self, key: SchemaBox, value: SchemaBox) -> Option<SchemaBox> {
        self.try_insert_box(key, value).unwrap()
    }

    /// Insert an untyped item into the map.
    /// # Errors
    /// Errors if the key or value schemas do not match the map's.
    pub fn try_insert_box(
        &mut self,
        key: SchemaBox,
        value: SchemaBox,
    ) -> Result<Option<SchemaBox>, SchemaMismatchError> {
        if key.schema() != self.key_schema || value.schema() != self.value_schema {
            Err(SchemaMismatchError)
        } else {
            // SOUnD: we've checked that the schemas are matching.
            let previous_value = unsafe { self.insert_box_unchecked(key, value) };
            Ok(previous_value)
        }
    }

    /// # Safety
    /// Key schema and value schema must match the map's.
    pub unsafe fn insert_box_unchecked(
        &mut self,
        key: SchemaBox,
        mut value: SchemaBox,
    ) -> Option<SchemaBox> {
        let hash = {
            let mut hasher = self.map.hasher().build_hasher();
            hasher.write_u64(key.hash());
            hasher.finish()
        };
        let entry = self
            .map
            .raw_entry_mut()
            .from_hash(hash, |other| other == &key);
        // Return the previous value.
        match entry {
            bones_utils::hashbrown::hash_map::RawEntryMut::Occupied(mut occupied) => {
                std::mem::swap(occupied.get_mut(), &mut value);
                Some(value)
            }
            bones_utils::hashbrown::hash_map::RawEntryMut::Vacant(vacant) => {
                vacant.insert(key, value);
                None
            }
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
            // SOUND: we've veriried that they key schema matches the map's
            let value = unsafe { self.get_ref_unchecked(SchemaRef::new(key)) }
                // SOUND: we've verified that the value schema maches the map's
                .map(|x| unsafe { x.deref() });

            Ok(value)
        }
    }

    /// Get an untyped reference to an item in the map.
    /// # Panics
    /// Panics if the schema of the key doesn't match.
    #[inline]
    #[track_caller]
    pub fn get_ref(&self, key: SchemaRef) -> Option<SchemaRef> {
        self.try_get_ref(key).unwrap()
    }

    /// Get an untyped reference to an item in the map.
    /// # Errors
    /// Errors if the schema of the key doesn't match.
    pub fn try_get_ref(&self, key: SchemaRef) -> Result<Option<SchemaRef>, SchemaMismatchError> {
        if key.schema() != self.key_schema {
            Err(SchemaMismatchError)
        } else {
            // SOUND: we've check the key schema matches.
            Ok(unsafe { self.get_ref_unchecked(key) })
        }
    }

    /// # Safety
    /// The key's schema must match this map's key schema.
    pub unsafe fn get_ref_unchecked(&self, key: SchemaRef) -> Option<SchemaRef> {
        let Some(hash_fn) = self.key_schema.hash_fn else {
                panic!("Key schema doesn't implement hash");
            };
        let Some(eq_fn) = self.key_schema.eq_fn else {
                panic!("Key schema doesn't implement eq");
            };
        let key_ptr = key.as_ptr();
        // SOUND: caller asserts the key schema matches
        let raw_hash = unsafe { (hash_fn)(key_ptr) };
        let hash = {
            let mut hasher = self.map.hasher().build_hasher();
            hasher.write_u64(raw_hash);
            hasher.finish()
        };
        self.map
            .raw_entry()
            .from_hash(hash, |key| {
                let other_ptr = key.as_ref().as_ptr();
                // SOUND: caller asserts the key schema matches.
                unsafe { (eq_fn)(key_ptr, other_ptr) }
            })
            .map(|x| x.1.as_ref())
    }

    /// Get an untyped reference to an item in the map.
    /// # Panics
    /// Panics if the schema of the key doesn't match.
    #[inline]
    #[track_caller]
    pub fn get_ref_mut(&mut self, key: SchemaRef) -> Option<SchemaRefMut> {
        self.try_get_ref_mut(key).unwrap()
    }

    /// Get an untyped reference to an item in the map.
    /// # Errors
    /// Errors if the schema of the key doesn't match.
    pub fn try_get_ref_mut(
        &mut self,
        key: SchemaRef,
    ) -> Result<Option<SchemaRefMut>, SchemaMismatchError> {
        if key.schema() != self.key_schema {
            Err(SchemaMismatchError)
        } else {
            // SOUND: we've checked that the key schema matches.
            Ok(unsafe { self.get_ref_unchecked_mut(key) })
        }
    }

    /// # Safety
    /// The key's schema must match this map's key schema.
    pub unsafe fn get_ref_unchecked_mut(&mut self, key: SchemaRef) -> Option<SchemaRefMut> {
        let Some(hash_fn) = self.key_schema.hash_fn else {
            panic!("Key schema doesn't implement hash");
        };
        let Some(eq_fn) = self.key_schema.eq_fn else {
            panic!("Key schema doesn't implement eq");
        };
        let key_ptr = key.as_ptr();
        // SOUND: caller asserts the key schema matches
        let raw_hash = unsafe { (hash_fn)(key_ptr) };
        let hash = {
            let mut hasher = self.map.hasher().build_hasher();
            hasher.write_u64(raw_hash);
            hasher.finish()
        };
        let entry = self.map.raw_entry_mut().from_hash(hash, |key| {
            let other_ptr = key.as_ref().as_ptr();
            // SOUND: caller asserts the key schema matches.
            unsafe { (eq_fn)(key_ptr, other_ptr) }
        });
        match entry {
            bones_utils::hashbrown::hash_map::RawEntryMut::Occupied(entry) => {
                Some(entry.into_mut())
            }
            bones_utils::hashbrown::hash_map::RawEntryMut::Vacant(_) => None,
        }
        .map(|x| x.as_mut())
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
            // SOUND: we've checked that the key schema matches.
            let value = unsafe { self.get_ref_unchecked_mut(SchemaRef::new(key)) }
                // SOUND: we've checked that the value schema matches.
                .map(|x| unsafe { x.deref_mut() });
            Ok(value)
        }
    }

    /// Remove an item.
    /// # Panics
    /// Panics if the schemas of the key and value don't match the map.
    #[inline]
    #[track_caller]
    pub fn remove<K: HasSchema, V: HasSchema>(&mut self, key: &K) -> Option<V> {
        self.try_remove(key).unwrap()
    }

    /// Remove an item.
    /// # Errors
    /// Errors if the schemas of the key and value don't match the map.
    pub fn try_remove<K: HasSchema, V: HasSchema>(
        &mut self,
        key: &K,
    ) -> Result<Option<V>, SchemaMismatchError> {
        if K::schema() != self.key_schema || V::schema() != self.value_schema {
            Err(SchemaMismatchError)
        } else {
            // SOUND: we've checked that the key schema matches.
            let value = unsafe { self.remove_unchecked(SchemaRef::new(key)) }
                // SOUND: we've checked that the value schema matches.
                .map(|x| unsafe { x.into_inner_unchecked() });
            Ok(value)
        }
    }

    /// Untypededly remove an item.
    /// # Panics
    /// Panics if the key schema does not match the map's.
    #[inline]
    #[track_caller]
    pub fn remove_box(&mut self, key: SchemaRef) -> Option<SchemaBox> {
        self.try_remove_box(key).unwrap()
    }

    /// Untypededly remove an item.
    /// # Errors
    /// Errors if the key schema does not match the map's.
    pub fn try_remove_box(
        &mut self,
        key: SchemaRef,
    ) -> Result<Option<SchemaBox>, SchemaMismatchError> {
        if key.schema() != self.key_schema {
            Err(SchemaMismatchError)
        } else {
            // SOUND: we've checked the key schema matches.
            Ok(unsafe { self.remove_unchecked(key) })
        }
    }

    /// # Safety
    /// The key schema must match the map's.
    pub unsafe fn remove_unchecked(&mut self, key: SchemaRef) -> Option<SchemaBox> {
        let Some(hash_fn) = self.key_schema.hash_fn else {
                panic!("Key schema doesn't implement hash");
            };
        let Some(eq_fn) = self.key_schema.eq_fn else {
                panic!("Key schema doesn't implement eq");
            };
        let key_ptr = key.as_ptr();
        // SOUND: caller asserts the key schema matches
        let hash = unsafe { (hash_fn)(key_ptr) };
        let hash = {
            let mut hasher = self.map.hasher().build_hasher();
            hasher.write_u64(hash);
            hasher.finish()
        };
        let entry = self.map.raw_entry_mut().from_hash(hash, |key| {
            let other_ptr = key.as_ref().as_ptr();
            // SOUND: caller asserts the key schema matches
            unsafe { (eq_fn)(key_ptr, other_ptr) }
        });
        match entry {
            bones_utils::hashbrown::hash_map::RawEntryMut::Occupied(entry) => Some(entry.remove()),
            bones_utils::hashbrown::hash_map::RawEntryMut::Vacant(_) => None,
        }
    }

    /// Convert into a typed [`SMap`].
    /// # Panics
    /// Panics if the schemas of K and V don't match the [`SchemaMap`].
    #[inline]
    #[track_caller]
    pub fn into_smap<K: HasSchema, V: HasSchema>(self) -> SMap<K, V> {
        self.try_into_smap().unwrap()
    }

    /// Convert into a typed [`SMap`].
    /// # Errors
    /// Errors if the schemas of K and V don't match the [`SchemaMap`].
    pub fn try_into_smap<K: HasSchema, V: HasSchema>(
        self,
    ) -> Result<SMap<K, V>, SchemaMismatchError> {
        if K::schema() == self.key_schema && V::schema() == self.value_schema {
            Ok(SMap {
                map: self,
                _phantom: PhantomData,
            })
        } else {
            Err(SchemaMismatchError)
        }
    }
}

/// Typed version of a [`SchemaMap`].
///
/// This works essentially like a [`HashMap`], but is compatible with the schema ecosystem.
///
/// It is also slightly more efficient to access an [`SMap`] compared to a [`SchemaMap`] because it
/// doesn't need to do a runtime schema check every time the map is accessed.
#[derive(Debug)]
pub struct SMap<K: HasSchema, V: HasSchema> {
    map: SchemaMap,
    _phantom: PhantomData<(K, V)>,
}
impl<K: HasSchema, V: HasSchema> Clone for SMap<K, V> {
    fn clone(&self) -> Self {
        Self {
            map: self.map.clone(),
            _phantom: self._phantom,
        }
    }
}
impl<K: HasSchema, V: HasSchema> Default for SMap<K, V> {
    fn default() -> Self {
        Self {
            map: SchemaMap::new(K::schema(), V::schema()),
            _phantom: Default::default(),
        }
    }
}
unsafe impl<K: HasSchema, V: HasSchema> HasSchema for SMap<K, V> {
    fn schema() -> &'static Schema {
        static S: OnceLock<&'static Schema> = OnceLock::new();
        S.get_or_init(|| {
            SCHEMA_REGISTRY.register(SchemaData {
                kind: SchemaKind::Map {
                    key: K::schema(),
                    value: V::schema(),
                },
                type_id: Some(TypeId::of::<Self>()),
                clone_fn: Some(<Self as RawClone>::raw_clone),
                drop_fn: Some(<Self as RawDrop>::raw_drop),
                default_fn: Some(<Self as RawDefault>::raw_default),
                hash_fn: Some(SchemaVec::raw_hash),
                eq_fn: Some(SchemaVec::raw_eq),
                type_data: Default::default(),
            })
        })
    }
}

impl<K: HasSchema, V: HasSchema> SMap<K, V> {
    /// Initialize the [`SMap`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert an entry into the map, returning the previous value, if it exists.
    pub fn insert(&mut self, k: K, v: V) -> Option<V> {
        // SOUND: schemas are checked to match when SMap is constructed.
        unsafe {
            self.map
                .insert_box_unchecked(SchemaBox::new(k), SchemaBox::new(v))
                .map(|x| x.into_inner_unchecked())
        }
    }

    /// Get a reference to an item in the map.
    pub fn get(&self, key: &K) -> Option<&V> {
        // SOUND: schemas are checked to match when SMap is constructed.
        unsafe {
            self.map
                .get_ref_unchecked(SchemaRef::new(key))
                .map(|x| x.deref())
        }
    }

    /// Get a mutable reference to an item in the map.
    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        // SOUND: schemas are checked to match when SMap is constructed.
        unsafe {
            self.map
                .get_ref_unchecked_mut(SchemaRef::new(key))
                .map(|x| x.deref_mut())
        }
    }

    /// Remove an item from the map.
    pub fn remove(&mut self, key: &K) -> Option<V> {
        // SOUND: schemas are checked to match when SMap is constructed.
        unsafe {
            self.map
                .remove_unchecked(SchemaRef::new(key))
                .map(|x| x.into_inner_unchecked())
        }
    }

    /// Convert into an untyped [`SchemaMap`].
    pub fn into_schema_map(self) -> SchemaMap {
        self.map
    }
}
